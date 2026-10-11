//! The board of tasks: the window's part.
//!
//! An overlay over the whole window that shows the tasks of the project the
//! window is on (or the global board) in columns, the way `leon task board`
//! prints them, kept in step with the agents that write to it from other
//! processes. Everything it decides is in `task_board` (the keys, the cursor,
//! what a key that carries a card asks for, which board is shown); this file
//! reads the store, hands the plans to the engine and draws.
//!
//! **Reading.** The board is read from the store when it opens, whenever the
//! engine says something changed, and every [`REFRESH`] while it is open: an
//! agent's `leon task` or MCP call runs in another process, and the
//! window's store is never told about what another process wrote.
//!
//! **Writing.** Never here: a moved card, a new task and a new priority are
//! engine operations (`Op::MoveTask`, `Op::AddTask`, `Op::UpdateTask`), as
//! every write of the window is. The title of a new task is asked in the
//! palette, as every question is, and the board is back when it is answered.

use std::time::Duration;

use gpui_kit::prelude::*;
use gpui_kit::{div, Context, Div, Stateful, Task, Window};
use leon_core::{store::memory::normalized, MemoryScope, TaskPatch, TaskStatus};

use super::shell::{Overlay, Shell};
use super::updates_view::pill;
use super::widgets::{mono, section_label};
use crate::engine::Op;
use crate::keys::Command;
use crate::task_board::{self, Action, Board, Which, COLUMNS, PER_COLUMN};
use crate::theme::{metrics, px, Palette};

/// How often the open board reads the store again.
pub const REFRESH: Duration = Duration::from_millis(1500);

/// What the window keeps of the board while it is open.
pub struct TasksUi {
    /// The cards and the cursor.
    pub board: Board,
    /// The board that is shown.
    pub which: Which,
    /// Why it is not the one that may be expected, when it is not.
    pub note: Option<&'static str>,
    /// The person switched to the global board.
    pub want_global: bool,
    /// A question asked from the board ends back in it.
    pub return_from_palette: bool,
    /// The title of a task that was just asked for: the cursor goes to it
    /// when it shows up.
    pub follow_title: Option<String>,
    /// The store could not be read.
    pub error: Option<String>,
    /// The loop that reads again while the board is open.
    refresher: Option<Task<()>>,
}

impl Default for TasksUi {
    fn default() -> Self {
        Self {
            board: Board::new(),
            which: Which::Global,
            note: None,
            want_global: false,
            return_from_palette: false,
            follow_title: None,
            error: None,
            refresher: None,
        }
    }
}

impl Shell {
    /// Opens the board, or closes it when it is open.
    pub(super) fn toggle_tasks(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.overlay == Overlay::Tasks {
            self.close_overlay(window, cx);
            return;
        }
        self.close_overlay(window, cx);
        self.overlay = Overlay::Tasks;
        self.open_tasks(cx);
        self.focus.focus(window, cx);
        cx.notify();
    }

    /// Picks the board, reads it and starts keeping it fresh.
    pub(super) fn open_tasks(&mut self, cx: &mut Context<Self>) {
        self.tasks_ui.error = None;
        let (which, note) = self.task_board_now();
        self.tasks_ui.which = which;
        self.tasks_ui.note = note;
        self.tasks_reload();
        self.tasks_ui.refresher = Some(cx.spawn(async move |this, cx| loop {
            cx.background_executor().timer(REFRESH).await;
            let open = this
                .update(cx, |this, cx| {
                    if this.overlay != Overlay::Tasks {
                        return false;
                    }
                    if this.tasks_reload() {
                        cx.notify();
                    }
                    true
                })
                .unwrap_or(false);
            if !open {
                return;
            }
        }));
    }

    /// Stops keeping the board fresh: it was closed.
    pub(super) fn close_tasks(&mut self) {
        self.tasks_ui.refresher = None;
        self.tasks_ui.follow_title = None;
    }

    /// The board for what the window is on, and the reason when it is not the
    /// project's.
    fn task_board_now(&self) -> (Which, Option<&'static str>) {
        let here = self.here();
        let local = here.as_ref().is_none_or(|here| here.machine.is_local());
        let root = here.as_ref().and_then(|here| match &here.project {
            Some(id) => self
                .snapshot
                .project(id)
                .map(|entry| entry.project.root.clone()),
            None => self
                .engine
                .store()
                .memory_root(&here.machine, &here.cwd)
                .ok()
                .flatten(),
        });
        task_board::choose(local, root, self.tasks_ui.want_global)
    }

    fn tasks_scope(&self) -> MemoryScope {
        match &self.tasks_ui.which {
            Which::Project(root) => MemoryScope::project(root.clone()),
            Which::Global => MemoryScope::Global,
        }
    }

    /// Reads the board again. `true` when what is shown changed.
    pub(super) fn tasks_reload(&mut self) -> bool {
        let scope = self.tasks_scope();
        match crate::tasks::load_board(self.engine.store(), &scope, PER_COLUMN) {
            Ok(tasks) => {
                if let Some(title) = &self.tasks_ui.follow_title {
                    let wanted = normalized(title);
                    if let Some(task) = tasks.iter().find(|task| normalized(&task.title) == wanted)
                    {
                        self.tasks_ui.board.follow(task.id.clone());
                        self.tasks_ui.follow_title = None;
                    }
                }
                let before = self.tasks_ui.board.clone();
                self.tasks_ui.board.set_tasks(tasks);
                let had_error = self.tasks_ui.error.take().is_some();
                before != self.tasks_ui.board || had_error
            }
            Err(error) => {
                let said = crate::tasks::sentence(&error);
                let changed = self.tasks_ui.error.as_deref() != Some(said.as_str());
                self.tasks_ui.error = Some(said);
                changed
            }
        }
    }

    /// The questions of the palette come back to the board.
    pub(super) fn return_to_tasks(&mut self, cx: &mut Context<Self>) {
        self.overlay = Overlay::Tasks;
        self.open_tasks(cx);
    }

    /// Creates a task with the answer of the palette: in the column the
    /// cursor is in while the board is open, else in the one of tasks to do.
    pub(super) fn new_task(&mut self, title: &str, cx: &mut Context<Self>) {
        let open = self.overlay == Overlay::Tasks;
        let (which, status) = if open {
            (self.tasks_ui.which.clone(), self.tasks_ui.board.status())
        } else {
            (self.task_board_now().0, TaskStatus::Todo)
        };
        let root = match which {
            Which::Project(root) => Some(root),
            Which::Global => None,
        };
        if open {
            self.tasks_ui.follow_title = Some(title.to_owned());
        }
        self.engine.submit(Op::AddTask {
            root,
            title: title.to_owned(),
            status,
        });
        cx.notify();
    }

    /// A key while the board is up. `true` when it was the board's.
    pub(super) fn tasks_key(
        &mut self,
        stroke: &gpui_kit::Keystroke,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let m = &stroke.modifiers;
        let other = m.platform || m.control || m.alt;
        let Some(action) = task_board::action_of(stroke.key.as_str(), m.shift, other) else {
            return false;
        };
        match action {
            Action::Left | Action::Right | Action::Up | Action::Down => {
                self.tasks_ui.board.step(action);
            }
            Action::CardLeft
            | Action::CardRight
            | Action::CardUp
            | Action::CardDown
            | Action::Done => {
                if let Some(plan) = self.tasks_ui.board.plan(action) {
                    self.engine.submit(Op::MoveTask {
                        id: plan.id,
                        status: plan.status,
                        before: plan.before,
                    });
                }
            }
            Action::Priority => {
                if let Some(card) = self.tasks_ui.board.selected() {
                    let patch = TaskPatch {
                        priority: Some(task_board::next_priority(card.priority)),
                        ..TaskPatch::default()
                    };
                    self.engine.submit(Op::UpdateTask {
                        id: card.id.clone(),
                        patch,
                    });
                }
            }
            Action::New => {
                self.tasks_ui.return_from_palette = true;
                self.begin_flow(Command::NewTask, window, cx);
            }
            Action::Scope => self.tasks_switch_board(),
        }
        cx.notify();
        true
    }

    /// Switches between the project's board and the global one.
    pub(super) fn tasks_switch_board(&mut self) {
        self.tasks_ui.want_global = !self.tasks_ui.want_global;
        let (which, note) = self.task_board_now();
        self.tasks_ui.which = which;
        self.tasks_ui.note = note;
        self.tasks_reload();
    }

    pub(super) fn render_tasks(&self, colours: &Palette, cx: &mut Context<Self>) -> Stateful<Div> {
        let ui = &self.tasks_ui;
        let (cursor_column, cursor_row) = ui.board.cursor();
        let title = match &ui.which {
            Which::Project(root) => std::path::Path::new(root)
                .file_name()
                .map_or_else(|| root.clone(), |name| name.to_string_lossy().into_owned()),
            Which::Global => "Global board".to_owned(),
        };
        let switch_label = if matches!(ui.which, Which::Global) && !ui.want_global {
            None
        } else if ui.want_global {
            Some("Project board  g")
        } else {
            Some("Global board  g")
        };
        let mut header = div()
            .flex_none()
            .h(metrics::HEADER_HEIGHT())
            .px_4()
            .border_b_1()
            .border_color(colours.border)
            .flex()
            .items_center()
            .gap_3()
            .child(section_label("Tasks", colours))
            .child(
                div()
                    .debug_selector(|| "tasks-title".into())
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_color(colours.text_muted)
                    .child(format!("{title} · {} tasks", ui.board.len())),
            );
        if let Some(label) = switch_label {
            header = header.child(pill("tasks-switch", label, colours).on_click(cx.listener(
                |this, _, _, cx| {
                    this.tasks_switch_board();
                    cx.notify();
                },
            )));
        }
        header = header.child(
            pill("tasks-new", "New task  n", colours).on_click(cx.listener(
                |this, _, window, cx| {
                    this.tasks_ui.return_from_palette = true;
                    this.begin_flow(Command::NewTask, window, cx);
                },
            )),
        );

        let hover = colours.surface_2;
        let mut columns = div()
            .id("tasks-columns")
            .debug_selector(|| "tasks-columns".into())
            .flex_1()
            .min_h_0()
            .flex()
            .gap_2()
            .p_3();
        for (index, status) in COLUMNS.into_iter().enumerate() {
            let cards = &ui.board.columns()[index];
            let here = index == cursor_column;
            let mut list = div()
                .id(("tasks-cards", index))
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .flex()
                .flex_col()
                .gap(px(6.));
            for (row, task) in cards.iter().enumerate() {
                let on = here && row == cursor_row;
                let mut meta: Vec<String> = vec![task.short_id().to_owned()];
                if task.priority != leon_core::TaskPriority::None {
                    meta.push(task.priority.as_str().to_owned());
                }
                if let Some(assignee) = &task.assignee {
                    meta.push(format!("@{assignee}"));
                }
                meta.extend(task.labels.iter().map(|label| format!("#{label}")));
                list = list.child(
                    div()
                        .id(("tasks-card", index * 10_000 + row))
                        .debug_selector(move || format!("tasks-card-{index}-{row}"))
                        .flex_none()
                        .px(px(8.))
                        .py(px(6.))
                        .rounded(metrics::RADIUS())
                        .border_1()
                        .border_color(if on {
                            colours.signal
                        } else {
                            colours.elevated_border
                        })
                        .bg(if on {
                            colours.surface_2
                        } else {
                            colours.surface
                        })
                        .cursor_pointer()
                        .hover(move |style| style.bg(hover))
                        .flex()
                        .flex_col()
                        .gap(px(2.))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.tasks_ui.board.select(index, row);
                            cx.notify();
                        }))
                        .child(
                            div()
                                .text_size(metrics::TEXT_BODY())
                                .text_color(colours.text)
                                .child(task.title.clone()),
                        )
                        .child(
                            div()
                                .text_size(metrics::TEXT_SMALL())
                                .text_color(colours.text_muted)
                                .child(meta.join("  ")),
                        ),
                );
            }
            if cards.is_empty() {
                list = list.child(
                    div()
                        .debug_selector(move || format!("tasks-empty-{index}"))
                        .px(px(8.))
                        .text_size(metrics::TEXT_SMALL())
                        .text_color(colours.text_faint)
                        .child("Nothing here"),
                );
            }
            columns = columns.child(
                div()
                    .id(("tasks-column", index))
                    .debug_selector(move || format!("tasks-column-{index}"))
                    .flex_1()
                    .min_w_0()
                    .min_h_0()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        mono(format!(
                            "{} {}",
                            status.as_str().to_uppercase(),
                            cards.len()
                        ))
                        .flex_none()
                        .px(px(4.))
                        .pb(px(4.))
                        .border_b_1()
                        .border_color(if here { colours.signal } else { colours.border })
                        .text_color(if here {
                            colours.text
                        } else {
                            colours.text_muted
                        }),
                    )
                    .child(list),
            );
        }

        let mut footer = div()
            .flex_none()
            .px_4()
            .py(px(8.))
            .border_t_1()
            .border_color(colours.border)
            .text_size(metrics::TEXT_SMALL())
            .text_color(colours.text_muted)
            .flex()
            .flex_col()
            .gap(px(2.));
        if let Some(error) = &ui.error {
            footer = footer.child(
                div()
                    .debug_selector(|| "tasks-error".into())
                    .text_color(colours.text)
                    .child(format!("Could not read the board: {error}")),
            );
        }
        if ui.board.is_empty() && ui.error.is_none() {
            footer = footer.child(
                div()
                    .debug_selector(|| "tasks-hint".into())
                    .child("No tasks yet. Press n to add one; agents add theirs from their terminal or MCP tools."),
            );
        }
        if let Some(note) = ui.note {
            footer = footer.child(div().debug_selector(|| "tasks-note".into()).child(note));
        }
        footer = footer.child(div().child(
            "Arrows or h j k l move · Shift with them carries the card · n new · \
                 d done · p priority · g global board · Esc closes",
        ));

        self.card("tasks", colours)
            .w(self.viewport.width - px(64.))
            .max_w(px(1500.))
            .h(self.viewport.height - px(96.))
            .overflow_hidden()
            .flex()
            .flex_col()
            .child(header)
            .child(columns)
            .child(footer)
    }
}
