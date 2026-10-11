//! The board of tasks in the window: how it opens, the keys that carry a
//! card, the drag with the pointer, the question that creates a task, and the
//! reading of what another process wrote.

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

fn board_scope(h: &Harness, cx: &mut TestAppContext) -> MemoryScope {
    match h.shell(cx, |s| s.tasks_ui.which.clone()) {
        Which::Project(root) => MemoryScope::project(root),
        Which::Global => MemoryScope::Global,
    }
}

fn open_board(h: &Harness, cx: &mut TestAppContext) {
    h.press_chord("cmd-shift-i", "ctrl-shift-alt-i", cx);
    assert_eq!(h.main_kind(cx), "tasks");
}

/// What `leon task add` does from a terminal, on the board that is open:
/// a write the window's own engine knows nothing about, and the board
/// reading it at its next turn.
fn written_elsewhere(h: &Harness, title: &str, at: i64, cx: &mut TestAppContext) -> String {
    let scope = board_scope(h, cx);
    let id = h
        .store
        .add_task(&NewTask::titled(scope, title), at)
        .unwrap()
        .into_task()
        .id;
    wait_for_the_board(h, cx);
    id
}

fn wait_for_the_board(h: &Harness, cx: &mut TestAppContext) {
    cx.executor().advance_clock(REFRESH * 2);
    h.settle(cx);
}

fn status_of(h: &Harness, id: &str) -> TaskStatus {
    h.store.task(id).unwrap().status
}

fn titles(h: &Harness, column: usize, cx: &mut TestAppContext) -> Vec<String> {
    h.shell(cx, |s| {
        s.tasks_ui.board.columns()[column]
            .iter()
            .map(|task| task.title.clone())
            .collect()
    })
}

/// Drags the card drawn at `from` to a point of `to`, `fraction` of the way
/// down it, and lets go there.
fn drag_card(h: &Harness, from: &str, to: &str, fraction: f32, cx: &mut TestAppContext) {
    let from = h
        .bounds_of(from.to_owned(), cx)
        .unwrap_or_else(|| panic!("{from} is not drawn"));
    let to = h
        .bounds_of(to.to_owned(), cx)
        .unwrap_or_else(|| panic!("{to} is not drawn"));
    let point = gpui_kit::point(to.center().x, to.origin.y + to.size.height * fraction);
    cx.update_window(h.window.into(), |_, window, cx| {
        window.drag(from.center(), point, cx);
    })
    .unwrap();
    h.settle(cx);
    wait_for_the_board(h, cx);
}

#[gpui_kit::test]
fn the_chord_opens_the_board_of_the_project_and_escape_puts_back_what_was_there(
    cx: &mut TestAppContext,
) {
    let h = on_api(cx);
    let before = h.main_kind(cx);
    open_board(&h, cx);
    assert!(h.shows("tasks", cx));
    // An empty board is the lion and what to do; the columns come with the
    // first task.
    assert!(h.shows("tasks-hint", cx));
    written_elsewhere(&h, "First", 1, cx);
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
    // The sidebar stays beside it.
    assert!(h.shows("sidebar-nav", cx));
    h.press("escape", cx);
    assert_eq!(h.main_kind(cx), before);
    // The same chord closes it.
    open_board(&h, cx);
    h.press_chord("cmd-shift-i", "ctrl-shift-alt-i", cx);
    assert_eq!(h.main_kind(cx), before);
}

#[gpui_kit::test]
fn the_row_of_the_sidebar_opens_and_closes_the_board(cx: &mut TestAppContext) {
    let h = open(cx, ScriptedRunner::new());
    h.mouse_on("sidebar-tasks".to_owned(), gpui_kit::MouseButton::Left, cx);
    assert_eq!(h.main_kind(cx), "tasks");
    h.mouse_on("sidebar-tasks".to_owned(), gpui_kit::MouseButton::Left, cx);
    assert_ne!(h.main_kind(cx), "tasks");
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
fn from_the_home_the_first_project_of_this_computer_is_shown(cx: &mut TestAppContext) {
    let h = open(cx, ScriptedRunner::new());
    open_board(&h, cx);
    assert!(matches!(
        h.shell(cx, |s| s.tasks_ui.which.clone()),
        Which::Project(_)
    ));
    // `g` goes to the global board and back.
    h.press("g", cx);
    assert_eq!(h.shell(cx, |s| s.tasks_ui.which.clone()), Which::Global);
    h.press("g", cx);
    assert!(matches!(
        h.shell(cx, |s| s.tasks_ui.which.clone()),
        Which::Project(_)
    ));
}

#[gpui_kit::test]
fn the_project_chips_switch_the_board(cx: &mut TestAppContext) {
    let h = on_api(cx);
    let web_root = h
        .store
        .projects(Some(&MachineId::local()))
        .unwrap()
        .into_iter()
        .find(|project| project.name == "web")
        .unwrap()
        .root;
    h.store
        .add_task(
            &NewTask::titled(MemoryScope::project(web_root), "Only for web"),
            1,
        )
        .unwrap();
    open_board(&h, cx);
    assert!(h.shell(cx, |s| s.tasks_ui.board.is_empty()));
    // The chips are the projects of this computer by name, then Global.
    let chips = h.shell(cx, |s| s.board_projects().len());
    assert!(chips >= 2);
    let web = (0..chips)
        .find(|index| {
            h.shell(cx, |s| {
                s.board_projects()
                    .get(*index)
                    .is_some_and(|(_, name, _)| name == "web")
            })
        })
        .unwrap();
    h.mouse_on(
        format!("tasks-project-{web}"),
        gpui_kit::MouseButton::Left,
        cx,
    );
    assert_eq!(titles(&h, 1, cx), ["Only for web"]);
    h.mouse_on("tasks-global".to_owned(), gpui_kit::MouseButton::Left, cx);
    assert_eq!(h.shell(cx, |s| s.tasks_ui.which.clone()), Which::Global);
}

#[gpui_kit::test]
fn a_task_written_by_another_process_appears_when_the_board_reads_again(cx: &mut TestAppContext) {
    let h = on_api(cx);
    open_board(&h, cx);
    assert!(h.shell(cx, |s| s.tasks_ui.board.is_empty()));
    assert!(h.shows("tasks-hint", cx));
    written_elsewhere(&h, "Cover the retry path", 1, cx);
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
    wait_for_the_board(&h, cx);
    assert_eq!(status_of(&h, &id), TaskStatus::Doing);
    assert!(h.status().contains("Moved"), "{}", h.status());
    // The cursor followed the card to its new column.
    assert_eq!(h.shell(cx, |s| s.tasks_ui.board.cursor()), (2, 0));
    h.press("d", cx);
    wait_for_the_board(&h, cx);
    assert_eq!(status_of(&h, &id), TaskStatus::Done);
    assert!(h.store.task(&id).unwrap().closed_at.is_some());
}

#[gpui_kit::test]
fn a_card_moves_up_and_down_inside_its_column_by_key(cx: &mut TestAppContext) {
    let h = on_api(cx);
    open_board(&h, cx);
    for (n, title) in ["A", "B", "C"].into_iter().enumerate() {
        written_elsewhere(&h, title, n as i64, cx);
    }
    assert_eq!(titles(&h, 1, cx), ["A", "B", "C"]);
    h.press("shift-down", cx);
    wait_for_the_board(&h, cx);
    assert_eq!(titles(&h, 1, cx), ["B", "A", "C"]);
    h.press("shift-down", cx);
    wait_for_the_board(&h, cx);
    assert_eq!(titles(&h, 1, cx), ["B", "C", "A"]);
    h.press("shift-up", cx);
    wait_for_the_board(&h, cx);
    assert_eq!(titles(&h, 1, cx), ["B", "A", "C"]);
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
    assert_eq!(h.main_kind(cx), "tasks");
    assert_eq!(h.shell(cx, |s| s.pane), Pane::Main);
    wait_for_the_board(&h, cx);
    let scope = board_scope(&h, cx);
    let tasks = h
        .store
        .tasks(&scope, &leon_core::TaskFilter::default(), 10)
        .unwrap();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].title, "Write the migration guide");
    assert_eq!(tasks[0].status, TaskStatus::Doing);
    // The cursor is on the card that was just made, and the keys work again.
    assert_eq!(
        h.shell(cx, |s| s.tasks_ui.board.selected().map(|t| t.title.clone())),
        Some("Write the migration guide".to_owned())
    );
    h.press("left", cx);
    assert_eq!(h.shell(cx, |s| s.tasks_ui.board.cursor().0), 1);
}

#[gpui_kit::test]
fn a_card_dragged_to_another_column_changes_its_status(cx: &mut TestAppContext) {
    let h = on_api(cx);
    open_board(&h, cx);
    let id = written_elsewhere(&h, "Fix the redirect", 1, cx);
    // The doing column is empty: its whole body takes the drop.
    drag_card(&h, "tasks-card-1-0", "tasks-tail-2", 0.5, cx);
    assert_eq!(status_of(&h, &id), TaskStatus::Doing);
    assert_eq!(titles(&h, 2, cx), ["Fix the redirect"]);
    assert!(h.status().contains("Moved"), "{}", h.status());
}

#[gpui_kit::test]
fn a_card_dragged_above_another_goes_above_it(cx: &mut TestAppContext) {
    let h = on_api(cx);
    open_board(&h, cx);
    for (n, title) in ["A", "B", "C"].into_iter().enumerate() {
        written_elsewhere(&h, title, n as i64, cx);
    }
    // C onto the upper half of A: first.
    drag_card(&h, "tasks-card-1-2", "tasks-card-1-0", 0.2, cx);
    assert_eq!(titles(&h, 1, cx), ["C", "A", "B"]);
    // C onto the lower half of B (now the last): the end.
    drag_card(&h, "tasks-card-1-0", "tasks-card-1-2", 0.8, cx);
    assert_eq!(titles(&h, 1, cx), ["A", "B", "C"]);
}

#[gpui_kit::test]
fn a_card_let_go_outside_the_columns_stays_where_it_was(cx: &mut TestAppContext) {
    let h = on_api(cx);
    open_board(&h, cx);
    let id = written_elsewhere(&h, "Stay put", 1, cx);
    let from = h.bounds_of("tasks-card-1-0".to_owned(), cx).unwrap();
    // Over the footer of the board: no column there.
    let footer = h.bounds_of("tasks-detail".to_owned(), cx).unwrap();
    cx.update_window(h.window.into(), |_, window, cx| {
        window.drag(from.center(), footer.center(), cx);
    })
    .unwrap();
    wait_for_the_board(&h, cx);
    assert_eq!(status_of(&h, &id), TaskStatus::Todo);
    assert_eq!(titles(&h, 1, cx), ["Stay put"]);
}

#[gpui_kit::test]
fn the_boards_keys_are_not_taken_while_another_overlay_is_up(cx: &mut TestAppContext) {
    let h = on_api(cx);
    open_board(&h, cx);
    h.press_chord("cmd-shift-p", "ctrl-shift-p", cx);
    assert_eq!(h.shell(cx, |s| s.overlay), Overlay::Palette);
    h.type_text("n", cx);
    assert_eq!(h.shell(cx, |s| s.overlay), Overlay::Palette);
}
