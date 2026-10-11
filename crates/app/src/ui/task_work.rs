//! Creating the worktree of a task, in the window: a worktree and a session
//! to code it.
//!
//! What the branch is called and what the agent is told are decided by
//! [`crate::task_work`]. This file carries it out the way "One prompt,
//! several agents" does (`fanout.rs`), for one task and one agent: the engine
//! makes the worktree, the window waits for it to be listed in the local
//! store, and then the session starts in it, behind the project's `setup`
//! command when its `leon.toml` has one. **Only when the worktree exists** the
//! task is written: its branch, the column `doing` and, when an agent codes
//! it and nobody is on it, that agent as the assignee. A worktree that git
//! refuses leaves the card where it was and says why.
//!
//! A task that already has a worktree is not given a second one: the key opens
//! the worktree it has.

use std::time::Duration;

use gpui_kit::{Context, Window};
use leon_core::{AgentId, ProjectId, Task as Card};

use super::scripts;
use super::shell::{Pane, Shell};
use crate::engine::{MachineState, Op, StatusKind};
use crate::format;
use crate::launch::{self, Launch};
use crate::{address, fanout, task_work};

/// How long to wait for a worktree that was made to be listed.
const LISTED_FOR: Duration = Duration::from_secs(15);

/// How often the list is looked at while waiting.
const LISTED_EVERY: Duration = Duration::from_millis(200);

/// How the agent's terminal reaches the program.
const CLI: &str = "\"$LEON_BIN\"";

impl Shell {
    /// `w` or Enter on a card: opens the worktree the task has, or asks which
    /// agent codes it.
    pub(super) fn tasks_start(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(task) = self.tasks_ui.board.selected().cloned() else {
            return;
        };
        if let (Some(project), Some(branch)) = (self.tasks_ui.project.clone(), &task.branch) {
            if self.open_task_worktree(&project, branch, cx) {
                return;
            }
            // The worktree is gone: the work can be started again.
        }
        self.tasks_ui.return_from_palette = true;
        self.begin_flow(crate::keys::Command::StartTask, window, cx);
    }

    /// Opens the first session that runs in the worktree of the task under
    /// the cursor.
    pub(super) fn tasks_open_session(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(task) = self.tasks_ui.board.selected().cloned() else {
            return;
        };
        let Some(project) = self.tasks_ui.project.clone() else {
            return;
        };
        let worktree = task.branch.as_deref().and_then(|branch| {
            self.snapshot.project(&project).and_then(|entry| {
                entry
                    .worktrees
                    .iter()
                    .find(|worktree| worktree.branch.as_deref() == Some(branch))
                    .map(|worktree| worktree.id.clone())
            })
        });
        let session = worktree.and_then(|id| {
            self.placement
                .live_of_worktree(&id)
                .first()
                .map(|place| self.placement.live[*place].id)
        });
        match session {
            Some(id) => {
                self.tasks_ui.refresher_stop();
                self.open_live(id, window, cx);
                self.home.nav = None;
            }
            None => self
                .engine
                .report(StatusKind::Info, "No session is open in its worktree."),
        }
    }

    /// Selects the worktree of `branch` in the tree and shows it. `false`
    /// when the project has none by that name.
    fn open_task_worktree(
        &mut self,
        project: &ProjectId,
        branch: &str,
        cx: &mut Context<Self>,
    ) -> bool {
        let found = self.snapshot.project(project).and_then(|entry| {
            entry
                .worktrees
                .iter()
                .find(|worktree| worktree.branch.as_deref() == Some(branch))
                .map(|worktree| worktree.id.clone())
        });
        let Some(worktree) = found else {
            return false;
        };
        self.tasks_ui.refresher_stop();
        self.open_worktree(project, &worktree, cx);
        self.pane = Pane::Sidebar;
        self.home.nav = None;
        true
    }

    /// Makes a worktree for the task under the cursor and starts a session in
    /// it: `agent` with the task as its prompt, or only a shell.
    pub(super) fn start_task_work(
        &mut self,
        agent: Option<AgentId>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(task) = self.tasks_ui.board.selected().cloned() else {
            self.engine
                .report(StatusKind::Info, "Select a task on the board first.");
            return;
        };
        let Some(project) = self.tasks_ui.project.clone() else {
            self.engine.report(
                StatusKind::Error,
                "Work starts from the board of a project of this computer: this board has no \
                 project to make a worktree in.",
            );
            return;
        };
        // The branches git has are asked first, so a name it has is numbered.
        let listed = self.engine.base_refs(project.clone());
        self.fanout_tasks
            .push(cx.spawn_in(window, async move |this, cx| {
                let refs = listed.await.ok().flatten().unwrap_or_default();
                this.update_in(cx, |this, window, cx| {
                    this.make_task_worktree(project, task, agent, &refs, window, cx);
                })
                .ok();
            }));
    }

    fn make_task_worktree(
        &mut self,
        project: ProjectId,
        task: Card,
        agent: Option<AgentId>,
        refs: &[String],
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let fail = |this: &Self, why: String| this.engine.report(StatusKind::Error, why);
        let Some(entry) = self.snapshot.project(&project) else {
            return fail(self, "That project is not known.".to_owned());
        };
        let (machine_id, root) = (entry.project.machine_id.clone(), entry.project.root.clone());
        if !machine_id.is_local() {
            return fail(
                self,
                "Work starts from a task on this computer: that project is on another machine."
                    .to_owned(),
            );
        }
        let Some(machine) = self.snapshot.machine(&machine_id).cloned() else {
            return fail(self, "That machine is not known.".to_owned());
        };
        let taken = fanout::taken_branches(
            entry
                .worktrees
                .iter()
                .filter_map(|worktree| worktree.branch.clone()),
            refs,
        );
        let branch = task_work::branch_name(&task.title, task.short_id(), &taken);
        if let Err(why) = address::validate_branch(&branch) {
            return fail(self, why.to_owned());
        }
        let prefs = Self::launch_prefs(cx);
        let launch = match agent {
            Some(agent) => {
                let text = task_work::prompt(&task.title, &task.body, task.short_id(), CLI);
                let prompt = match launch::clean_prompt(&text) {
                    Ok(prompt) => prompt,
                    Err(why) => return fail(self, why),
                };
                let launch = Launch::Prompted {
                    kind: agent,
                    prompt,
                    account: super::terminals::default_account(agent, cx),
                };
                let report = match self.engine.machine_state(&machine_id) {
                    MachineState::Online(Some(report)) => Some(report),
                    _ => None,
                };
                if let Err(why) = launch::plan_with(
                    &machine,
                    report.as_ref(),
                    &root,
                    &launch,
                    &self.engine.ssh(),
                    &*self.options.system,
                    &prefs,
                ) {
                    let name = format::agent_name(agent);
                    return fail(
                        self,
                        format!("{name}: {}.", why.to_string().trim_end_matches('.')),
                    );
                }
                launch
            }
            None => Launch::Shell,
        };
        let made = self
            .engine
            .add_worktrees(project.clone(), vec![branch.clone()], None);
        let engine = self.engine.clone();
        // Somebody who was given the task keeps it.
        let assignee = match (&task.assignee, agent) {
            (None, Some(agent)) => Some(agent.as_str().to_owned()),
            _ => None,
        };
        let (id, short) = (task.id.clone(), task.short_id().to_owned());
        self.fanout_tasks
            .push(cx.spawn_in(window, async move |this, cx| {
                // The closure cannot hold the context while the loop below
                // needs it, so telling is a macro.
                macro_rules! report {
                    ($why:expr) => {{
                        let why: String = $why;
                        this.update(cx, |this, _| this.engine.report(StatusKind::Error, why))
                            .ok();
                    }};
                }
                match made.await.unwrap_or_default().into_iter().next() {
                    Some(Ok(_)) => {}
                    Some(Err(why)) => {
                        report!(format!("Could not make a worktree for {short}: {why}"));
                        return;
                    }
                    None => {
                        report!("The work did not finish.".to_owned());
                        return;
                    }
                }
                // The window reads the local store: the session starts where
                // the store says the worktree is.
                let mut waited = Duration::ZERO;
                let path = loop {
                    let found = this.read_with(cx, |this, _| {
                        this.snapshot.project(&project).and_then(|entry| {
                            entry
                                .worktrees
                                .iter()
                                .find(|worktree| {
                                    worktree.branch.as_deref() == Some(branch.as_str())
                                })
                                .map(|worktree| worktree.path.clone())
                        })
                    });
                    match found {
                        Ok(Some(path)) => break Some(path),
                        Ok(None) if waited < LISTED_FOR => {
                            cx.background_executor().timer(LISTED_EVERY).await;
                            waited += LISTED_EVERY;
                        }
                        _ => break None,
                    }
                };
                let Some(path) = path else {
                    report!(format!(
                        "The worktree of {short} was made but is not in the list yet."
                    ));
                    return;
                };
                // The project's file is read now: the setup it names is the
                // one that runs.
                let state = engine
                    .read_project_file(project.clone())
                    .await
                    .ok()
                    .and_then(Result::ok);
                this.update_in(cx, |this, window, cx| {
                    this.engine.submit(Op::StartTask {
                        id,
                        branch: branch.clone(),
                        assignee,
                        notice: scripts::setup_notice(&state),
                    });
                    this.start_worktree_sessions(
                        &project,
                        &machine_id,
                        vec![(path, launch)],
                        state,
                        window,
                        cx,
                    );
                })
                .ok();
            }));
    }
}
