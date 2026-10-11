//! The board of tasks in the window: the chord, the keys that carry a card,
//! the question that creates one, and the reading of what another process
//! wrote.

use super::*;
use crate::task_board::Which;
use crate::ui::tasks_view::REFRESH;
use leon_core::{MemoryScope, NewTask, TaskStatus};

/// The window with the keyboard on the project "api": its board is shown.
fn on_api(cx: &mut TestAppContext) -> Harness {
    let h = open(cx, ScriptedRunner::new());
    let api = project_named(&h, "api");
    put_cursor(&h, cx, NodeId::Project(api));
    h
}

fn api_scope(h: &Harness, cx: &mut TestAppContext) -> MemoryScope {
    match h.shell(cx, |s| s.tasks_ui.which.clone()) {
        Which::Project(root) => MemoryScope::project(root),
        Which::Global => MemoryScope::Global,
    }
}

fn open_board(h: &Harness, cx: &mut TestAppContext) {
    h.press_chord("cmd-shift-i", "ctrl-shift-alt-i", cx);
    assert_eq!(h.shell(cx, |s| s.overlay), Overlay::Tasks);
}

/// What `leon task add` does from a terminal, on the board that is open:
/// a write the window's own engine knows nothing about, and the board
/// reading it at its next turn.
fn written_elsewhere(h: &Harness, title: &str, at: i64, cx: &mut TestAppContext) -> String {
    let scope = api_scope(h, cx);
    let id = h
        .store
        .add_task(&NewTask::titled(scope, title), at)
        .unwrap()
        .into_task()
        .id;
    cx.executor().advance_clock(REFRESH * 2);
    h.settle(cx);
    id
}

fn status_of(h: &Harness, id: &str) -> TaskStatus {
    h.store.task(id).unwrap().status
}

#[gpui_kit::test]
fn the_chord_opens_the_board_of_the_project_and_escape_closes_it(cx: &mut TestAppContext) {
    let h = on_api(cx);
    open_board(&h, cx);
    assert!(h.shows("tasks", cx));
    for column in 0..5 {
        assert!(
            h.shows_dynamic(format!("tasks-column-{column}"), cx),
            "{column}"
        );
    }
    assert!(matches!(
        h.shell(cx, |s| s.tasks_ui.which.clone()),
        Which::Project(_)
    ));
    h.press("escape", cx);
    assert_eq!(h.shell(cx, |s| s.overlay), Overlay::None);
    // The same chord closes it.
    open_board(&h, cx);
    h.press_chord("cmd-shift-i", "ctrl-shift-alt-i", cx);
    assert_eq!(h.shell(cx, |s| s.overlay), Overlay::None);
}

#[gpui_kit::test]
fn the_palette_offers_the_board_and_a_new_task(cx: &mut TestAppContext) {
    let h = on_api(cx);
    h.press_chord("cmd-shift-p", "ctrl-shift-p", cx);
    h.set_palette_text(">task", cx);
    let titles = h.palette_titles(cx);
    assert!(
        titles.iter().any(|t| t == "Show the task board"),
        "{titles:?}"
    );
    assert!(titles.iter().any(|t| t == "New task\u{2026}"), "{titles:?}");
}

#[gpui_kit::test]
fn without_a_project_the_global_board_is_shown_and_says_why(cx: &mut TestAppContext) {
    let h = open(cx, ScriptedRunner::new());
    open_board(&h, cx);
    assert_eq!(h.shell(cx, |s| s.tasks_ui.which.clone()), Which::Global);
    assert!(h.shows("tasks-note", cx));
}

#[gpui_kit::test]
fn a_task_written_by_another_process_appears_when_the_board_reads_again(cx: &mut TestAppContext) {
    let h = on_api(cx);
    open_board(&h, cx);
    assert!(h.shell(cx, |s| s.tasks_ui.board.is_empty()));
    // What `leon task add` does from a terminal: a write the window's own
    // engine knows nothing about.
    let scope = api_scope(&h, cx);
    h.store
        .add_task(&NewTask::titled(scope, "Cover the retry path"), 1)
        .unwrap();
    cx.executor().advance_clock(REFRESH * 2);
    h.settle(cx);
    assert_eq!(h.shell(cx, |s| s.tasks_ui.board.len()), 1);
    assert!(h.shows_dynamic("tasks-card-1-0".to_owned(), cx));
}

#[gpui_kit::test]
fn the_keys_carry_a_card_between_columns_and_finish_it(cx: &mut TestAppContext) {
    let h = on_api(cx);
    open_board(&h, cx);
    let id = written_elsewhere(&h, "Ship the board", 1, cx);
    assert_eq!(h.shell(cx, |s| s.tasks_ui.board.cursor()), (1, 0));
    h.press("shift-right", cx);
    cx.executor().advance_clock(REFRESH * 2);
    h.settle(cx);
    assert_eq!(status_of(&h, &id), TaskStatus::Doing);
    assert!(h.status().contains("Moved"), "{}", h.status());
    // The cursor followed the card to its new column.
    assert_eq!(h.shell(cx, |s| s.tasks_ui.board.cursor()), (2, 0));
    h.press("d", cx);
    cx.executor().advance_clock(REFRESH * 2);
    h.settle(cx);
    assert_eq!(status_of(&h, &id), TaskStatus::Done);
    assert!(h.store.task(&id).unwrap().closed_at.is_some());
}

#[gpui_kit::test]
fn a_card_moves_up_and_down_inside_its_column(cx: &mut TestAppContext) {
    let h = on_api(cx);
    open_board(&h, cx);
    for (n, title) in ["A", "B", "C"].into_iter().enumerate() {
        written_elsewhere(&h, title, n as i64, cx);
    }
    let order = |h: &Harness, cx: &mut TestAppContext| -> Vec<String> {
        h.shell(cx, |s| {
            s.tasks_ui.board.columns()[1]
                .iter()
                .map(|task| task.title.clone())
                .collect()
        })
    };
    assert_eq!(order(&h, cx), ["A", "B", "C"]);
    h.press("shift-down", cx);
    cx.executor().advance_clock(REFRESH * 2);
    h.settle(cx);
    assert_eq!(order(&h, cx), ["B", "A", "C"]);
    h.press("shift-down", cx);
    cx.executor().advance_clock(REFRESH * 2);
    h.settle(cx);
    assert_eq!(order(&h, cx), ["B", "C", "A"]);
    h.press("shift-up", cx);
    cx.executor().advance_clock(REFRESH * 2);
    h.settle(cx);
    assert_eq!(order(&h, cx), ["B", "A", "C"]);
}

#[gpui_kit::test]
fn p_raises_the_priority_of_the_card(cx: &mut TestAppContext) {
    let h = on_api(cx);
    open_board(&h, cx);
    let id = written_elsewhere(&h, "Tune the query", 1, cx);
    h.press("p", cx);
    assert_eq!(
        h.store.task(&id).unwrap().priority,
        leon_core::TaskPriority::Low
    );
    assert!(h.status().contains("Priority"), "{}", h.status());
}

#[gpui_kit::test]
fn n_asks_for_a_title_in_the_palette_and_comes_back_to_the_board(cx: &mut TestAppContext) {
    let h = on_api(cx);
    open_board(&h, cx);
    // The cursor is in the doing column: the new task starts there.
    h.press("right", cx);
    h.press("n", cx);
    assert_eq!(h.shell(cx, |s| s.overlay), Overlay::Palette);
    h.type_text("Write the migration guide", cx);
    h.press("enter", cx);
    assert_eq!(h.shell(cx, |s| s.overlay), Overlay::Tasks);
    cx.executor().advance_clock(REFRESH * 2);
    h.settle(cx);
    let scope = api_scope(&h, cx);
    let tasks = h
        .store
        .tasks(&scope, &leon_core::TaskFilter::default(), 10)
        .unwrap();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].title, "Write the migration guide");
    assert_eq!(tasks[0].status, TaskStatus::Doing);
    // The cursor is on the card that was just made.
    assert_eq!(
        h.shell(cx, |s| s.tasks_ui.board.selected().map(|t| t.title.clone())),
        Some("Write the migration guide".to_owned())
    );
}

#[gpui_kit::test]
fn g_switches_between_the_project_board_and_the_global_one(cx: &mut TestAppContext) {
    let h = on_api(cx);
    open_board(&h, cx);
    written_elsewhere(&h, "Only for api", 1, cx);
    h.store
        .add_task(&NewTask::titled(MemoryScope::Global, "For everyone"), 2)
        .unwrap();
    let titles = |h: &Harness, cx: &mut TestAppContext| -> Vec<String> {
        h.shell(cx, |s| {
            s.tasks_ui
                .board
                .columns()
                .iter()
                .flatten()
                .map(|task| task.title.clone())
                .collect()
        })
    };
    assert_eq!(titles(&h, cx), ["Only for api"]);
    h.press("g", cx);
    assert_eq!(titles(&h, cx), ["For everyone"]);
    assert_eq!(h.shell(cx, |s| s.tasks_ui.which.clone()), Which::Global);
    h.press("g", cx);
    assert_eq!(titles(&h, cx), ["Only for api"]);
}

#[gpui_kit::test]
fn the_boards_keys_are_not_taken_while_another_overlay_is_up(cx: &mut TestAppContext) {
    let h = on_api(cx);
    h.press_chord("cmd-shift-p", "ctrl-shift-p", cx);
    assert_eq!(h.shell(cx, |s| s.overlay), Overlay::Palette);
    h.type_text("n", cx);
    assert_eq!(h.shell(cx, |s| s.overlay), Overlay::Palette);
}
