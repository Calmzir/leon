//! The engine's half of syncing tasks with the issues of a GitHub repository.
//!
//! What is to be done is decided by [`crate::task_sync::plan`], a pure
//! function. This file carries it out, in an order that does no damage when
//! something fails or something else writes at the same time:
//!
//! * the repository is the one the person **confirmed** when sync was turned
//!   on, named on every call (`-R`): the folder's remotes are never asked
//!   again, so a fork cannot sync with its upstream by mistake;
//! * the writes to GitHub go one at a time, and the link of a pair is written
//!   as soon as that pair is done, so a run that stops half way leaves every
//!   finished pair finished;
//! * the write to a task is made only if the task is as the plan read it
//!   (`update_task_if`): an edit made meanwhile is kept and the pair is tried
//!   again at the next run;
//! * a failure of `gh` (not installed, not signed in, offline) stops the run
//!   with one sentence the person can act on;
//! * one run at a time: a second request waits for the first.
//!
//! Nothing here makes an issue unless a card was asked to
//! ([`Engine::create_issue_for`]), and nothing deletes one.

use leon_core::{MemoryScope, NewTask, ProjectId, StoreError, TaskLink, TaskStatus, GITHUB};
use leon_remote::{issue_number_of_url, Github, GithubError, RepoInfo};
use tokio::task::JoinHandle;

use super::{Engine, EngineError, SharedRunner};
use crate::task_sync::{self, Input, Step};

/// The most issues one run reads.
const ISSUE_LIMIT: usize = 500;

/// The most tasks one run reads of a project.
const TASK_LIMIT: usize = 500;

/// What a run did, for the line the person reads.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Summary {
    /// Issues that became tasks.
    pub imported: usize,
    /// Tasks written to their issue.
    pub pushed: usize,
    /// Tasks changed from their issue.
    pub pulled: usize,
    /// Pairs left for the next run because the task changed meanwhile.
    pub postponed: usize,
    /// Pairs the store refused a change of, and why.
    pub refused: Vec<String>,
}

impl Summary {
    /// The line the status bar shows.
    pub fn line(&self, repo: &str) -> String {
        let mut parts = Vec::new();
        for (count, what) in [
            (self.imported, "imported"),
            (self.pushed, "pushed"),
            (self.pulled, "pulled"),
        ] {
            if count > 0 {
                parts.push(format!("{count} {what}"));
            }
        }
        let mut line = if parts.is_empty() {
            format!("GitHub {repo}: everything is in sync")
        } else {
            format!("GitHub {repo}: {}", parts.join(", "))
        };
        if self.postponed > 0 {
            line.push_str(&format!(
                "; {} changed meanwhile and will be tried again",
                self.postponed
            ));
        }
        for why in &self.refused {
            line.push_str(&format!("; {why}"));
        }
        line.push('.');
        line
    }
}

/// What a failure of `gh` says to the person.
fn said(error: &GithubError) -> String {
    match error {
        GithubError::Run(error) => format!("Could not run the GitHub CLI: {error}"),
        GithubError::Failed { stderr, .. } => {
            let lower = stderr.to_lowercase();
            if lower.contains("auth login")
                || lower.contains("not logged")
                || lower.contains("authentication")
            {
                "GitHub says you are not signed in: run `gh auth login` in a terminal.".to_owned()
            } else if lower.contains("could not resolve") || lower.contains("connect") {
                "GitHub could not be reached: check the connection.".to_owned()
            } else {
                format!("GitHub said: {stderr}")
            }
        }
    }
}

fn no_gh() -> EngineError {
    EngineError::Invalid(
        "The GitHub CLI (gh) is not installed here: install it and run `gh auth login`.".to_owned(),
    )
}

fn github_error(error: GithubError) -> EngineError {
    EngineError::Invalid(said(&error))
}

/// A repository name that is `owner/name` and nothing a shell could take
/// another way.
fn valid_repo(name: &str) -> bool {
    let plain = |part: &str| {
        !part.is_empty()
            && part
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
    };
    name.split_once('/')
        .is_some_and(|(owner, repo)| plain(owner) && plain(repo))
}

impl Engine {
    /// Asks which GitHub repository the folder of a project is, for the
    /// person to confirm before sync is turned on.
    pub fn github_repository(&self, project: ProjectId) -> JoinHandle<Result<RepoInfo, String>> {
        let engine = self.clone();
        self.inner.handle.spawn(async move {
            let project = engine
                .inner
                .store
                .project(&project)
                .map_err(|error| error.to_string())?;
            if !project.machine_id.is_local() {
                return Err(
                    "Syncing tasks works for projects of this computer: this one is on another \
                     machine."
                        .to_owned(),
                );
            }
            let machine = engine
                .inner
                .store
                .machine(&project.machine_id)
                .map_err(|error| error.to_string())?;
            let runner = SharedRunner(engine.inner.runner.clone());
            let ssh = engine.ssh();
            match Github::new(&runner, &machine, &ssh)
                .repository(&project.root)
                .await
            {
                Ok(Some(repo)) => Ok(repo),
                Ok(None) => Err(no_gh().to_string()),
                Err(error) => Err(said(&error)),
            }
        })
    }

    /// Turns sync on for a project with the repository the person
    /// confirmed: makes the marker labels the repository needs, remembers
    /// the repository, and syncs once.
    pub(super) async fn enable_task_sync(
        &self,
        project: &ProjectId,
        repo: &str,
    ) -> Result<String, EngineError> {
        if !valid_repo(repo) {
            return Err(EngineError::Invalid(format!(
                "{repo:?} is not a repository (owner/name)."
            )));
        }
        let project = self.inner.store.project(project)?;
        if !project.machine_id.is_local() {
            return Err(EngineError::Invalid(
                "Syncing tasks works for projects of this computer.".to_owned(),
            ));
        }
        let machine = self.inner.store.machine(&project.machine_id)?;
        let runner = SharedRunner(self.inner.runner.clone());
        let ssh = self.ssh();
        let github = Github::new(&runner, &machine, &ssh);
        for (name, colour, description) in task_sync::marker_labels() {
            let made = github
                .ensure_label(&project.root, repo, &name, colour, &description)
                .await
                .map_err(github_error)?;
            if !made {
                return Err(no_gh());
            }
        }
        let scope = MemoryScope::project(project.root.clone());
        self.inner
            .store
            .enable_sync(&scope, GITHUB, repo, crate::memory::now_millis())?;
        self.sync_tasks(&project.id).await
    }

    /// Turns sync off for a project. The links stay.
    pub(super) fn disable_task_sync(&self, project: &ProjectId) -> Result<String, EngineError> {
        let project = self.inner.store.project(project)?;
        let scope = MemoryScope::project(project.root.clone());
        self.inner.store.disable_sync(&scope)?;
        Ok(format!(
            "Stopped syncing the tasks of {} with GitHub. Nothing was changed there.",
            project.name
        ))
    }

    /// Syncs the tasks of a project with its repository.
    pub(super) async fn sync_tasks(&self, project: &ProjectId) -> Result<String, EngineError> {
        // One run at a time.
        let _running = self.inner.task_sync_lock.lock().await;
        let project = self.inner.store.project(project)?;
        if !project.machine_id.is_local() {
            return Err(EngineError::Invalid(
                "Syncing tasks works for projects of this computer.".to_owned(),
            ));
        }
        let scope = MemoryScope::project(project.root.clone());
        let Some(config) = self.inner.store.sync_config(&scope)? else {
            return Err(EngineError::Invalid(
                "Syncing with GitHub is not turned on for this project.".to_owned(),
            ));
        };
        let machine = self.inner.store.machine(&project.machine_id)?;
        let runner = SharedRunner(self.inner.runner.clone());
        let ssh = self.ssh();
        let github = Github::new(&runner, &machine, &ssh);
        let outcome = self
            .run_task_sync(&github, &project.root, &scope, &config.repo)
            .await;
        let at = crate::memory::now_millis();
        match &outcome {
            Ok(_) => self.inner.store.record_sync_run(&scope, at, None)?,
            Err(error) => {
                self.inner
                    .store
                    .record_sync_run(&scope, at, Some(&error.to_string()))?;
            }
        }
        outcome.map(|summary| summary.line(&config.repo))
    }

    async fn run_task_sync<R: leon_remote::Runner>(
        &self,
        github: &Github<'_, R>,
        root: &str,
        scope: &MemoryScope,
        repo: &str,
    ) -> Result<Summary, EngineError> {
        let store = &self.inner.store;
        let issues = github
            .issues(root, repo, ISSUE_LIMIT)
            .await
            .map_err(github_error)?
            .ok_or_else(no_gh)?;
        let tasks = crate::tasks::load_board(store, scope, TASK_LIMIT)?;
        let links = store.task_links(GITHUB, repo)?;
        let now = crate::memory::now_millis();
        let steps = task_sync::plan(&Input {
            repo,
            tasks: &tasks,
            links: &links,
            issues: &issues,
            now,
        });
        let mut summary = Summary::default();
        for step in steps {
            match step {
                Step::Import { issue } => {
                    let said = task_sync::said_by(&issue, None);
                    let mut new = NewTask::titled(scope.clone(), said.title.clone());
                    new.body = said.body.clone();
                    new.status = Some(said.status);
                    new.priority = Some(said.priority);
                    new.labels = said.labels.iter().cloned().collect();
                    new.assignee = issue.assignee.clone();
                    new.agent = Some("github".to_owned());
                    let (task, cut) = store.import_task(&new, now)?;
                    store.put_task_link(&TaskLink {
                        provider: GITHUB.to_owned(),
                        repo: repo.to_owned(),
                        number: issue.number,
                        url: issue.url.clone(),
                        task_id: Some(task.id),
                        base: task_sync::base_of(&said),
                        remote_updated_at: issue.updated_at.clone(),
                        body_cut: cut,
                        synced_at: now,
                    })?;
                    summary.imported += 1;
                }
                Step::Sync {
                    number,
                    task_id,
                    expected,
                    local,
                    remote,
                    link,
                } => {
                    if let Some(change) = &remote {
                        for label in &change.edit.add_labels {
                            // A marker the repository lost is made again.
                            if let Some((name, colour, description)) = task_sync::marker_labels()
                                .into_iter()
                                .find(|(name, _, _)| name == label)
                            {
                                github
                                    .ensure_label(root, repo, &name, colour, &description)
                                    .await
                                    .map_err(github_error)?;
                            }
                        }
                        if !change.edit.is_empty() {
                            github
                                .edit_issue(root, repo, number, &change.edit)
                                .await
                                .map_err(github_error)?
                                .then_some(())
                                .ok_or_else(no_gh)?;
                        }
                        if let Some((closed, not_planned)) = change.close {
                            github
                                .set_issue_closed(root, repo, number, closed, not_planned)
                                .await
                                .map_err(github_error)?
                                .then_some(())
                                .ok_or_else(no_gh)?;
                        }
                        summary.pushed += 1;
                    }
                    if let Some(patch) = &local {
                        match store.update_task_if(&task_id, Some(expected), patch, now) {
                            Ok(Some(_)) => summary.pulled += 1,
                            Ok(None) => {
                                // Changed since the plan read it: kept, and
                                // the pair is tried again next time.
                                summary.postponed += 1;
                                continue;
                            }
                            Err(StoreError::NotFound(_)) => continue,
                            Err(StoreError::Invalid(why)) => {
                                summary.refused.push(format!(
                                    "#{number} could not be read into its task: {why}"
                                ));
                                continue;
                            }
                            Err(error) => return Err(error.into()),
                        }
                    }
                    store.put_task_link(&link)?;
                }
            }
        }
        Ok(summary)
    }

    /// Opens an issue for a task: the one thing in sync that makes an issue,
    /// and only for the card it was asked of. The task is linked to it.
    pub(super) async fn create_issue_for(&self, id: &str) -> Result<String, EngineError> {
        let _running = self.inner.task_sync_lock.lock().await;
        let store = &self.inner.store;
        let task = store.task(id)?;
        let MemoryScope::Project(root) = &task.scope else {
            return Err(EngineError::Invalid(
                "Only the tasks of a project can be sent to GitHub: this one is on the global \
                 board."
                    .to_owned(),
            ));
        };
        let Some(config) = store.sync_config(&task.scope)? else {
            return Err(EngineError::Invalid(
                "Turn on GitHub sync for this project first: its repository is confirmed there."
                    .to_owned(),
            ));
        };
        if store.task_link(&task.id)?.is_some() {
            return Err(EngineError::Invalid(
                "That task has an issue already.".to_owned(),
            ));
        }
        let project = store
            .projects(Some(&leon_core::MachineId::local()))?
            .into_iter()
            .find(|project| leon_core::path::key(&project.root) == leon_core::path::key(root))
            .ok_or_else(|| EngineError::Invalid("That project is not known.".to_owned()))?;
        let machine = store.machine(&project.machine_id)?;
        let runner = SharedRunner(self.inner.runner.clone());
        let ssh = self.ssh();
        let github = Github::new(&runner, &machine, &ssh);

        let said = task_sync::said_by_task(&task);
        let mut labels: Vec<String> = Vec::new();
        labels.extend(task_sync::status_label(task.status));
        labels.extend(task_sync::priority_label(task.priority));
        labels.extend(said.labels.iter().cloned());
        let url = github
            .create_issue(
                &project.root,
                &config.repo,
                &said.title,
                &said.body,
                &labels,
            )
            .await
            .map_err(github_error)?
            .ok_or_else(no_gh)?;
        let number = issue_number_of_url(&url).ok_or_else(|| {
            EngineError::Invalid(format!("GitHub answered {url:?}, which is not an issue."))
        })?;
        // Linked at once: whatever else fails, the issue is not made twice.
        store.put_task_link(&TaskLink {
            provider: GITHUB.to_owned(),
            repo: config.repo.clone(),
            number,
            url: url.clone(),
            task_id: Some(task.id.clone()),
            base: task_sync::base_of(&said),
            remote_updated_at: String::new(),
            body_cut: false,
            synced_at: crate::memory::now_millis(),
        })?;
        if task.status.is_closed() {
            github
                .set_issue_closed(
                    &project.root,
                    &config.repo,
                    number,
                    true,
                    task.status == TaskStatus::Canceled,
                )
                .await
                .map_err(github_error)?;
        }
        Ok(format!(
            "Opened issue #{number} for {}: {url}",
            task.short_id()
        ))
    }

    /// Detaches a task from its issue. The issue is left as it is.
    pub(super) fn unlink_task_issue(&self, id: &str) -> Result<String, EngineError> {
        let task = self.inner.store.task(id)?;
        if self.inner.store.unlink_task(&task.id)? {
            Ok(format!(
                "Unlinked {} from its issue. The issue was not changed.",
                task.short_id()
            ))
        } else {
            Err(EngineError::Invalid("That task has no issue.".to_owned()))
        }
    }
}
