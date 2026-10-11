//! What the store keeps of syncing tasks with a tracker: which projects are
//! synced and with which repository, and the link between a task and an
//! issue.
//!
//! A [`TaskLink`] holds, beside the issue's number and address, a
//! [`SyncBase`]: what the task and the issue said when they were last made
//! equal. The next sync compares each side with that base, so that a field
//! changed on one side only goes to the other, and a field changed on both is
//! decided by which was changed last. A link whose `task_id` is `None` is a
//! **tombstone**: the task was deleted or unlinked here, and the issue is
//! remembered so that it is not imported again.
//!
//! The store only keeps what it is told; talking to the tracker is the
//! engine's.

use rusqlite::{params, OptionalExtension as _, Row};

use super::memory::MemoryScope;
use super::tasks::{TaskPriority, TaskStatus};
use super::Store;
use crate::change::StoreChange;
use crate::error::Result;

/// The provider of GitHub issues.
pub const GITHUB: &str = "github";

/// What a task and its issue said when they were last made equal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncBase {
    /// The title, as the store keeps one.
    pub title: String,
    /// The body, as the store keeps one.
    pub body: String,
    /// The column.
    pub status: TaskStatus,
    /// The priority.
    pub priority: TaskPriority,
    /// The labels, lower case, other than the tracker's own markers.
    pub labels: Vec<String>,
}

/// An issue and the task it is linked to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskLink {
    /// Which tracker.
    pub provider: String,
    /// Which repository, as `owner/name`.
    pub repo: String,
    /// The issue's number.
    pub number: i64,
    /// Its address.
    pub url: String,
    /// The task, or `None` for a tombstone.
    pub task_id: Option<String>,
    /// What both said at the last sync.
    pub base: SyncBase,
    /// The issue's `updatedAt` as the tracker wrote it, at the last sync.
    pub remote_updated_at: String,
    /// The task's body is a cut of the issue's: never write it back.
    pub body_cut: bool,
    /// When the last sync of this link was, in milliseconds.
    pub synced_at: i64,
}

/// A project that is synced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncConfig {
    /// The project's root, as it was written.
    pub root: String,
    /// Which tracker.
    pub provider: String,
    /// The repository the person confirmed, as `owner/name`.
    pub repo: String,
    /// When sync was turned on.
    pub enabled_at: i64,
    /// When the last run started.
    pub last_run_at: Option<i64>,
    /// When the last run ended well.
    pub last_ok_at: Option<i64>,
    /// Why the last run did not.
    pub last_error: Option<String>,
}

fn join(labels: &[String]) -> String {
    labels.join("\n")
}

fn split(text: &str) -> Vec<String> {
    text.split('\n')
        .filter(|label| !label.is_empty())
        .map(str::to_owned)
        .collect()
}

const LINK_COLUMNS: &str = "provider, repo, number, url, task_id, base_title, base_body, \
                            base_status, base_priority, base_labels, remote_updated_at, \
                            body_cut, synced_at";

fn link_from_row(row: &Row<'_>) -> rusqlite::Result<TaskLink> {
    let status: String = row.get(7)?;
    let labels: String = row.get(9)?;
    Ok(TaskLink {
        provider: row.get(0)?,
        repo: row.get(1)?,
        number: row.get(2)?,
        url: row.get(3)?,
        task_id: row.get(4)?,
        base: SyncBase {
            title: row.get(5)?,
            body: row.get(6)?,
            status: TaskStatus::parse(&status).map_err(|_| super::bad_tag(7, &status))?,
            priority: TaskPriority::from_level(row.get(8)?),
            labels: split(&labels),
        },
        remote_updated_at: row.get(10)?,
        body_cut: row.get::<_, i64>(11)? != 0,
        synced_at: row.get(12)?,
    })
}

fn config_from_row(row: &Row<'_>) -> rusqlite::Result<SyncConfig> {
    Ok(SyncConfig {
        root: row.get(0)?,
        provider: row.get(1)?,
        repo: row.get(2)?,
        enabled_at: row.get(3)?,
        last_run_at: row.get(4)?,
        last_ok_at: row.get(5)?,
        last_error: row.get(6)?,
    })
}

impl Store {
    /// Turns sync on for the project of `scope` with `repo`, at `at`. The
    /// repository the person confirmed is kept: every later call to the
    /// tracker names it, whatever the folder's remotes say.
    pub fn enable_sync(
        &self,
        scope: &MemoryScope,
        provider: &str,
        repo: &str,
        at: i64,
    ) -> Result<SyncConfig> {
        scope.check()?;
        let key = scope.key();
        let root = scope.root().unwrap_or_default().to_owned();
        self.write(StoreChange::Tasks, |tx| {
            tx.execute(
                "INSERT INTO task_sync (scope, root, provider, repo, enabled_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT (scope) DO UPDATE SET
                    provider = ?3, repo = ?4, enabled_at = ?5,
                    last_run_at = NULL, last_ok_at = NULL, last_error = NULL",
                params![key, root, provider, repo, at],
            )?;
            Ok(SyncConfig {
                root,
                provider: provider.to_owned(),
                repo: repo.to_owned(),
                enabled_at: at,
                last_run_at: None,
                last_ok_at: None,
                last_error: None,
            })
        })
    }

    /// Turns sync off for a project. The links stay, so that turning it on
    /// again does not import every issue a second time.
    pub fn disable_sync(&self, scope: &MemoryScope) -> Result<()> {
        let key = scope.key();
        self.write(StoreChange::Tasks, |tx| {
            tx.execute("DELETE FROM task_sync WHERE scope = ?1", [&key])?;
            Ok(())
        })
    }

    /// How a project is synced, when it is.
    pub fn sync_config(&self, scope: &MemoryScope) -> Result<Option<SyncConfig>> {
        let key = scope.key();
        self.read(|connection| {
            Ok(connection
                .query_row(
                    "SELECT root, provider, repo, enabled_at, last_run_at, last_ok_at, last_error
                     FROM task_sync WHERE scope = ?1",
                    [&key],
                    config_from_row,
                )
                .optional()?)
        })
    }

    /// Notes that a run started at `at` and how it ended: `error` is why it
    /// did not end well.
    pub fn record_sync_run(&self, scope: &MemoryScope, at: i64, error: Option<&str>) -> Result<()> {
        let key = scope.key();
        self.write(StoreChange::Tasks, |tx| {
            tx.execute(
                "UPDATE task_sync SET last_run_at = ?2,
                        last_ok_at = CASE WHEN ?3 IS NULL THEN ?2 ELSE last_ok_at END,
                        last_error = ?3
                 WHERE scope = ?1",
                params![key, at, error],
            )?;
            Ok(())
        })
    }

    /// Every link of a repository, the tombstones included.
    pub fn task_links(&self, provider: &str, repo: &str) -> Result<Vec<TaskLink>> {
        self.read(|connection| {
            let mut statement = connection.prepare_cached(&format!(
                "SELECT {LINK_COLUMNS} FROM task_link
                 WHERE provider = ?1 AND repo = ?2 ORDER BY number"
            ))?;
            let found = statement
                .query_map(params![provider, repo], link_from_row)?
                .collect::<rusqlite::Result<_>>()?;
            Ok(found)
        })
    }

    /// The link of a task, when it has one.
    pub fn task_link(&self, task_id: &str) -> Result<Option<TaskLink>> {
        self.read(|connection| {
            Ok(connection
                .query_row(
                    &format!("SELECT {LINK_COLUMNS} FROM task_link WHERE task_id = ?1"),
                    [task_id],
                    link_from_row,
                )
                .optional()?)
        })
    }

    /// Writes a link: a new one, or the new base of one that exists.
    pub fn put_task_link(&self, link: &TaskLink) -> Result<()> {
        self.write(StoreChange::Tasks, |tx| {
            tx.execute(
                "INSERT INTO task_link (provider, repo, number, url, task_id, base_title,
                                        base_body, base_status, base_priority, base_labels,
                                        remote_updated_at, body_cut, synced_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
                 ON CONFLICT (provider, repo, number) DO UPDATE SET
                    url = ?4, task_id = ?5, base_title = ?6, base_body = ?7, base_status = ?8,
                    base_priority = ?9, base_labels = ?10, remote_updated_at = ?11,
                    body_cut = ?12, synced_at = ?13",
                params![
                    link.provider,
                    link.repo,
                    link.number,
                    link.url,
                    link.task_id,
                    link.base.title,
                    link.base.body,
                    link.base.status.as_str(),
                    link.base.priority.level(),
                    join(&link.base.labels),
                    link.remote_updated_at,
                    i64::from(link.body_cut),
                    link.synced_at,
                ],
            )?;
            Ok(())
        })
    }

    /// Detaches a task from its issue. The issue is remembered as a
    /// tombstone and is not imported again; nothing happens to it remotely.
    /// `false` when the task had no link.
    pub fn unlink_task(&self, task_id: &str) -> Result<bool> {
        self.write(StoreChange::Tasks, |tx| {
            let changed = tx.execute(
                "UPDATE task_link SET task_id = NULL WHERE task_id = ?1",
                [task_id],
            )?;
            Ok(changed > 0)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::tasks::{NewTask, TaskPatch};

    fn api() -> MemoryScope {
        MemoryScope::project("/srv/api")
    }

    fn link(task_id: Option<&str>, number: i64) -> TaskLink {
        TaskLink {
            provider: GITHUB.into(),
            repo: "zavudev/api".into(),
            number,
            url: format!("https://github.com/zavudev/api/issues/{number}"),
            task_id: task_id.map(str::to_owned),
            base: SyncBase {
                title: "Fix it".into(),
                body: "Details".into(),
                status: TaskStatus::Doing,
                priority: TaskPriority::High,
                labels: vec!["bug".into(), "needs review".into()],
            },
            remote_updated_at: "2026-10-10T10:00:00Z".into(),
            body_cut: false,
            synced_at: 5,
        }
    }

    #[test]
    fn sync_is_turned_on_with_a_repository_and_off_again() {
        let store = Store::open_in_memory().unwrap();
        assert_eq!(store.sync_config(&api()).unwrap(), None);
        let on = store
            .enable_sync(&api(), GITHUB, "zavudev/api", 10)
            .unwrap();
        assert_eq!(on.repo, "zavudev/api");
        store.record_sync_run(&api(), 20, None).unwrap();
        let config = store.sync_config(&api()).unwrap().unwrap();
        assert_eq!(
            (config.last_run_at, config.last_ok_at),
            (Some(20), Some(20))
        );
        // A run that failed keeps when the last good one was.
        store
            .record_sync_run(&api(), 30, Some("gh is not signed in"))
            .unwrap();
        let config = store.sync_config(&api()).unwrap().unwrap();
        assert_eq!(config.last_run_at, Some(30));
        assert_eq!(config.last_ok_at, Some(20));
        assert_eq!(config.last_error.as_deref(), Some("gh is not signed in"));
        // Another project is not synced.
        assert_eq!(
            store
                .sync_config(&MemoryScope::project("/srv/web"))
                .unwrap(),
            None
        );
        store.disable_sync(&api()).unwrap();
        assert_eq!(store.sync_config(&api()).unwrap(), None);
    }

    #[test]
    fn a_link_is_written_read_back_and_replaced() {
        let store = Store::open_in_memory().unwrap();
        let task = store
            .add_task(&NewTask::titled(api(), "Fix it"), 1)
            .unwrap()
            .into_task();
        let mut made = link(Some(&task.id), 7);
        store.put_task_link(&made).unwrap();
        assert_eq!(store.task_link(&task.id).unwrap(), Some(made.clone()));
        assert_eq!(
            store.task_links(GITHUB, "zavudev/api").unwrap(),
            vec![made.clone()]
        );
        // The same issue again is the same row, with the new base.
        made.base.title = "Fix it for good".into();
        made.body_cut = true;
        store.put_task_link(&made).unwrap();
        let all = store.task_links(GITHUB, "zavudev/api").unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].base.title, "Fix it for good");
        assert!(all[0].body_cut);
        // Another repository's issue 7 is another link.
        let mut other = link(None, 7);
        other.repo = "zavudev/web".into();
        store.put_task_link(&other).unwrap();
        assert_eq!(store.task_links(GITHUB, "zavudev/api").unwrap().len(), 1);
    }

    #[test]
    fn deleting_or_unlinking_a_task_leaves_a_tombstone_for_its_issue() {
        let store = Store::open_in_memory().unwrap();
        let first = store
            .add_task(&NewTask::titled(api(), "First"), 1)
            .unwrap()
            .into_task();
        let second = store
            .add_task(&NewTask::titled(api(), "Second"), 2)
            .unwrap()
            .into_task();
        store.put_task_link(&link(Some(&first.id), 1)).unwrap();
        store.put_task_link(&link(Some(&second.id), 2)).unwrap();
        store.delete_task(&first.id).unwrap();
        assert!(store.unlink_task(&second.id).unwrap());
        assert!(
            !store.unlink_task(&second.id).unwrap(),
            "nothing left to unlink"
        );
        let all = store.task_links(GITHUB, "zavudev/api").unwrap();
        assert_eq!(
            all.iter()
                .map(|l| (l.number, l.task_id.clone()))
                .collect::<Vec<_>>(),
            [(1, None), (2, None)]
        );
        assert_eq!(store.task_link(&second.id).unwrap(), None);
    }

    #[test]
    fn an_imported_task_is_cut_to_what_a_task_holds_and_is_never_a_duplicate() {
        let store = Store::open_in_memory().unwrap();
        let mut new = NewTask::titled(api(), "Same title");
        new.body = "x".repeat(crate::store::tasks::MAX_BODY_CHARS + 50);
        new.labels = (0..15).map(|n| format!("L{n}")).collect();
        let (first, cut) = store.import_task(&new, 1).unwrap();
        assert!(cut);
        assert_eq!(
            first.body.chars().count(),
            crate::store::tasks::MAX_BODY_CHARS
        );
        assert_eq!(first.labels.len(), crate::store::tasks::MAX_LABELS);
        assert_eq!(first.labels[0], "l0");
        // The same title is another task, not the first one answered again.
        let (second, cut) = store
            .import_task(&NewTask::titled(api(), "Same title"), 2)
            .unwrap();
        assert!(!cut);
        assert_ne!(first.id, second.id);
        // A title longer than a task's is cut, and an empty one is named.
        let long = "w".repeat(crate::store::tasks::MAX_TITLE_CHARS + 30);
        let (third, _) = store.import_task(&NewTask::titled(api(), long), 3).unwrap();
        assert_eq!(
            third.title.chars().count(),
            crate::store::tasks::MAX_TITLE_CHARS
        );
        let (fourth, _) = store.import_task(&NewTask::titled(api(), "  "), 4).unwrap();
        assert_eq!(fourth.title, "(untitled)");
    }

    #[test]
    fn a_write_that_expects_the_task_as_it_was_is_skipped_when_it_changed() {
        let store = Store::open_in_memory().unwrap();
        let task = store
            .add_task(&NewTask::titled(api(), "Fix it"), 100)
            .unwrap()
            .into_task();
        let patch = TaskPatch {
            title: Some("From the tracker".into()),
            ..TaskPatch::default()
        };
        // Somebody edited it after the sync read it.
        let edit = TaskPatch {
            priority: Some(TaskPriority::Urgent),
            ..TaskPatch::default()
        };
        store.update_task(&task.id, &edit, 200).unwrap();
        assert_eq!(
            store
                .update_task_if(&task.id, Some(100), &patch, 300)
                .unwrap(),
            None
        );
        assert_eq!(store.task(&task.id).unwrap().title, "Fix it");
        let applied = store
            .update_task_if(&task.id, Some(200), &patch, 300)
            .unwrap()
            .unwrap();
        assert_eq!(applied.title, "From the tracker");
    }
}
