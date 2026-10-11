//! Starting work on a task, in the window: the key on a card asks which agent
//! codes it, the engine makes a worktree, a session starts in it with the task
//! as its prompt, and the card moves to doing with the branch it was made on.
//! A worktree git refuses leaves the card where it was, and a task that has a
//! worktree opens it. The terminals are the scripted computer of `tests.rs`;
//! git is a scripted runner.

use super::live::{open_live, real_worktree_in, screen};
use super::*;
use crate::address::worktree_path;
use crate::task_board::Which;
use crate::ui::tasks_view::REFRESH;
use leon_core::{MemoryScope, NewTask, TaskStatus};
use std::time::Duration;

fn listing(path: &str, made: &[&str]) -> String {
    let mut text = format!(
        "worktree {path}\nHEAD 1111111111111111111111111111111111111111\nbranch refs/heads/trunk\n"
    );
    for branch in made {
        text.push_str(&format!(
            "worktree {}\nHEAD 2222222222222222222222222222222222222222\nbranch refs/heads/{branch}\n",
            worktree_path(path, branch)
        ));
    }
    text
}

fn wait_until(
    h: &Harness,
    cx: &mut TestAppContext,
    what: &str,
    condition: impl Fn(&Harness, &mut TestAppContext) -> bool,
) {
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    while std::time::Instant::now() < deadline {
        cx.executor().advance_clock(Duration::from_millis(200));
        h.settle(cx);
        if condition(h, cx) {
            return;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    panic!("timed out waiting for {what}: {}", h.status());
}

/// The board of the project "real" is open and holds one task: returns its
/// id and short id.
fn board_with_a_task(
    h: &Harness,
    title: &str,
    body: &str,
    cx: &mut TestAppContext,
) -> (String, String) {
    h.press_chord("cmd-shift-i", "ctrl-shift-alt-i", cx);
    assert_eq!(h.main_kind(cx), "tasks");
    let root = match h.shell(cx, |s| s.tasks_ui.which.clone()) {
        Which::Project(root) => root,
        Which::Global => panic!("the board of the project was expected"),
    };
    let mut new = NewTask::titled(MemoryScope::project(root), title);
    new.body = body.to_owned();
    let task = h.store.add_task(&new, 1).unwrap().into_task();
    cx.executor().advance_clock(REFRESH * 2);
    h.settle(cx);
    assert_eq!(h.shell(cx, |s| s.tasks_ui.board.len()), 1);
    (task.id.clone(), task.short_id().to_owned())
}

#[gpui_kit::test]
fn a_task_gets_a_worktree_and_an_agent_and_moves_to_doing(cx: &mut TestAppContext) {
    let h = open_live(cx);
    let (_dir, path) = real_worktree_in(&h, cx, tempfile::tempdir().unwrap());
    let (id, short) = board_with_a_task(&h, "Fix the login", "Users land back on the form.", cx);
    let branch = format!("fix-the-login-{short}");
    std::fs::create_dir_all(worktree_path(&path, &branch)).unwrap();
    // What git is asked: the branches it has, the worktree, its listing.
    h.runner.queue(Output::ok("trunk"));
    h.runner.queue(Output::ok(String::new()));
    for _ in 0..4 {
        h.runner.queue(Output::ok(listing(&path, &[&branch])));
    }

    h.press("w", cx);
    assert_eq!(h.shell(cx, |s| s.overlay), Overlay::Palette);
    h.type_text("claude", cx);
    h.press("enter", cx);

    wait_until(&h, cx, "the agent in its worktree", |h, cx| {
        screen(h, cx, 1).contains("FAKE-CLAUDE")
    });
    // The terminal wraps the long line that was typed: read it unwrapped.
    let shown = screen(&h, cx, 1).replace('\n', "");
    assert!(
        shown.contains("Work on this task: Fix the login"),
        "{shown}"
    );
    assert!(shown.contains("Users land back on the form."), "{shown}");
    assert!(
        shown.contains(&format!("task move {short} review")),
        "it is told how to close the card: {shown}"
    );
    let adds: Vec<String> = h
        .runner
        .calls()
        .iter()
        .filter(|call| {
            call.args.iter().any(|arg| arg == "worktree") && call.args.contains(&"add".to_owned())
        })
        .map(|call| call.args.join(" "))
        .collect();
    assert_eq!(adds.len(), 1, "{adds:?}");
    assert!(adds[0].contains(&format!("-b {branch}")), "{adds:?}");
    // The card is written after the worktree exists.
    wait_until(&h, cx, "the card to be moved", |h, _| {
        h.store.task(&id).unwrap().status == TaskStatus::Doing
    });
    let task = h.store.task(&id).unwrap();
    assert_eq!(task.branch.as_deref(), Some(branch.as_str()));
    assert_eq!(task.assignee.as_deref(), Some("claude"));
    assert!(h.status().contains("Created worktree"), "{}", h.status());
}

#[gpui_kit::test]
fn a_worktree_git_refuses_leaves_the_card_where_it_was(cx: &mut TestAppContext) {
    let h = open_live(cx);
    let (_dir, _path) = real_worktree_in(&h, cx, tempfile::tempdir().unwrap());
    let (id, _short) = board_with_a_task(&h, "Break nothing", "", cx);
    h.runner.queue(Output::ok("trunk"));
    h.runner.queue(Output::failed(
        128,
        "fatal: cannot lock ref 'refs/heads/break-nothing'",
    ));

    h.press("w", cx);
    h.type_text("claude", cx);
    h.press("enter", cx);

    wait_until(&h, cx, "the refusal", |h, _| {
        h.status().contains("Could not make a worktree")
    });
    assert!(h.status().contains("cannot lock ref"), "{}", h.status());
    let task = h.store.task(&id).unwrap();
    assert_eq!((task.status, task.branch), (TaskStatus::Todo, None));
}

#[gpui_kit::test]
fn only_a_worktree_starts_a_shell_in_it(cx: &mut TestAppContext) {
    let h = open_live(cx);
    let (_dir, path) = real_worktree_in(&h, cx, tempfile::tempdir().unwrap());
    let (id, short) = board_with_a_task(&h, "Look around", "", cx);
    let branch = format!("look-around-{short}");
    let added = worktree_path(&path, &branch);
    std::fs::create_dir_all(&added).unwrap();
    h.runner.queue(Output::ok("trunk"));
    h.runner.queue(Output::ok(String::new()));
    for _ in 0..4 {
        h.runner.queue(Output::ok(listing(&path, &[&branch])));
    }

    h.press("w", cx);
    h.type_text("only a worktree", cx);
    h.press("enter", cx);

    wait_until(&h, cx, "its shell", |h, cx| {
        screen(h, cx, 1).contains(&format!("FAKE-SHELL in {added}"))
    });
    wait_until(&h, cx, "the card to be moved", |h, _| {
        h.store.task(&id).unwrap().status == TaskStatus::Doing
    });
    // No agent: nobody is assigned.
    let task = h.store.task(&id).unwrap();
    assert_eq!(task.assignee, None);
    assert_eq!(task.branch.as_deref(), Some(branch.as_str()));
}

#[gpui_kit::test]
fn a_task_that_has_a_worktree_opens_it_instead_of_making_another(cx: &mut TestAppContext) {
    let h = open_live(cx);
    let (_dir, _path) = real_worktree_in(&h, cx, tempfile::tempdir().unwrap());
    let (id, _short) = board_with_a_task(&h, "Already started", "", cx);
    // The worktree the project has, by its branch.
    let patch = leon_core::TaskPatch {
        branch: Some(Some("trunk".to_owned())),
        ..leon_core::TaskPatch::default()
    };
    h.store.update_task(&id, &patch, 2).unwrap();
    cx.executor().advance_clock(REFRESH * 2);
    h.settle(cx);
    assert!(h.shows_dynamic("tasks-branch-1-0".to_owned(), cx));
    let before = h.runner.calls().len();

    h.press("w", cx);

    assert_eq!(h.main_kind(cx), "worktree:trunk");
    assert_eq!(h.shell(cx, |s| s.overlay), Overlay::None);
    assert_eq!(
        h.runner.calls().len(),
        before,
        "nothing was asked of git: no second worktree"
    );
}

#[gpui_kit::test]
fn the_global_board_cannot_start_work_and_says_why(cx: &mut TestAppContext) {
    let h = open_live(cx);
    let (_dir, _path) = real_worktree_in(&h, cx, tempfile::tempdir().unwrap());
    h.press_chord("cmd-shift-i", "ctrl-shift-alt-i", cx);
    h.press("g", cx);
    assert_eq!(h.shell(cx, |s| s.tasks_ui.which.clone()), Which::Global);
    h.store
        .add_task(&NewTask::titled(MemoryScope::Global, "Renew the domain"), 1)
        .unwrap();
    cx.executor().advance_clock(REFRESH * 2);
    h.settle(cx);
    h.press("w", cx);
    h.type_text("only a worktree", cx);
    h.press("enter", cx);
    cx.executor().advance_clock(Duration::from_secs(1));
    h.settle(cx);
    assert!(h.status().contains("no project"), "{}", h.status());
}

// ----- the link between a task and its worktree ----------------------------------------------

#[gpui_kit::test]
fn a_task_with_a_worktree_shows_it_and_that_it_is_active(cx: &mut TestAppContext) {
    use crate::ui::activity::Activity;
    use crate::ui::task_detail::Work;
    let h = open_live(cx);
    let (_dir, path) = real_worktree_in(&h, cx, tempfile::tempdir().unwrap());
    let (id, short) = board_with_a_task(&h, "Fix the login", "", cx);
    let branch = format!("fix-the-login-{short}");
    std::fs::create_dir_all(worktree_path(&path, &branch)).unwrap();
    h.runner.queue(Output::ok("trunk"));
    h.runner.queue(Output::ok(String::new()));
    for _ in 0..6 {
        h.runner.queue(Output::ok(listing(&path, &[&branch])));
    }
    h.press("w", cx);
    h.type_text("claude", cx);
    h.press("enter", cx);
    wait_until(&h, cx, "the agent in its worktree", |h, cx| {
        screen(h, cx, 1).contains("FAKE-CLAUDE")
    });
    wait_until(&h, cx, "the card to be linked", |h, _| {
        h.store.task(&id).unwrap().branch.is_some()
    });

    // The board again: the card says where its work is and that it is active.
    h.press_chord("cmd-shift-i", "ctrl-shift-alt-i", cx);
    assert_eq!(h.main_kind(cx), "tasks");
    cx.executor().advance_clock(REFRESH * 2);
    h.settle(cx);
    let card = h.store.task(&id).unwrap();
    let work = h.shell(cx, |s| s.work_of(&card));
    match work {
        Work::Here {
            branch: linked,
            activity,
            sessions,
            ..
        } => {
            assert_eq!(linked, branch);
            assert_ne!(activity, Activity::Off, "a session runs in it");
            assert_eq!(sessions, 1);
        }
        other => panic!("its worktree was expected: {other:?}"),
    }
    // It is in the doing column, with the work row on the card.
    assert!(h.shows_dynamic("tasks-branch-2-0".to_owned(), cx));
    assert!(h.shows_dynamic("tasks-work-2-0".to_owned(), cx));

    // The details say it too, with the actions of a worktree that exists.
    h.press("e", cx);
    assert!(h.shows("task-work-here", cx));
    assert!(h.shows("task-work-state", cx));
    assert!(h.shows("task-action-session", cx));
    assert!(h.shows("task-action-start", cx));
}

#[gpui_kit::test]
fn open_session_goes_to_the_terminal_of_the_worktree(cx: &mut TestAppContext) {
    let h = open_live(cx);
    let (_dir, _path) = real_worktree_in(&h, cx, tempfile::tempdir().unwrap());
    let (id, _short) = board_with_a_task(&h, "Already started", "", cx);
    // The worktree the project has, with a shell open in it.
    let patch = leon_core::TaskPatch {
        branch: Some(Some("trunk".to_owned())),
        ..leon_core::TaskPatch::default()
    };
    h.store.update_task(&id, &patch, 2).unwrap();
    h.press("ctrl-shift-t", cx);
    wait_until(&h, cx, "a shell", |h, cx| {
        screen(h, cx, 1).contains("FAKE-SHELL")
    });
    h.press_chord("cmd-shift-i", "ctrl-shift-alt-i", cx);
    cx.executor().advance_clock(REFRESH * 2);
    h.settle(cx);
    h.press("e", cx);
    assert!(h.shows("task-action-session", cx));
    h.mouse_on(
        "task-action-session".to_owned(),
        gpui_kit::MouseButton::Left,
        cx,
    );
    assert_ne!(h.main_kind(cx), "tasks");
}

#[gpui_kit::test]
fn a_branch_without_a_worktree_says_it_is_gone_and_offers_to_create_it_again(
    cx: &mut TestAppContext,
) {
    use crate::ui::task_detail::Work;
    let h = open_live(cx);
    let (_dir, _path) = real_worktree_in(&h, cx, tempfile::tempdir().unwrap());
    let (id, _short) = board_with_a_task(&h, "Was started", "", cx);
    let patch = leon_core::TaskPatch {
        branch: Some(Some("a-branch-nobody-has".to_owned())),
        ..leon_core::TaskPatch::default()
    };
    h.store.update_task(&id, &patch, 2).unwrap();
    cx.executor().advance_clock(REFRESH * 2);
    h.settle(cx);
    let card = h.store.task(&id).unwrap();
    assert_eq!(
        h.shell(cx, |s| s.work_of(&card)),
        Work::Gone("a-branch-nobody-has".to_owned())
    );
    assert!(h.shows_dynamic("tasks-branch-1-0".to_owned(), cx));
    h.press("e", cx);
    assert!(h.shows("task-work-gone", cx));
    assert!(!h.shows("task-action-session", cx));
}
