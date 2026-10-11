//! Making a new task: the same panel as the details of a card, over the right
//! edge of the board, with the title field already holding the keyboard.
//!
//! Every way of asking for a task (`n`, a **New task** button, the **+** of a
//! column, the palette's **New task…** while the board is open) lands here
//! instead of in a question of the palette: the title, the description, the
//! column, the priority, who it is for and the labels are all there to fill in
//! before the task exists. The column it starts in is the one it was asked
//! from.
//!
//! **Keys.** The panel is typing: `Enter` in the title or the assignee, and
//! `Cmd+Enter` or `Ctrl+Enter` anywhere, create the task; `Enter` in the
//! labels adds one; `Esc` throws it away. Nothing is written until it is
//! created, and a title that would be empty is refused where it is typed.
//! The task is written by the engine (`Op::AddTask`) and the board's cursor
//! goes to it when it shows up.

use gpui_kit::component::input::{InputState, Textarea, TextareaState};
use gpui_kit::prelude::*;
use gpui_kit::{
    div, Context, Div, Entity, Focusable, FontWeight, Hsla, SharedString, Stateful, Window,
};
use leon_core::{TaskPriority, TaskStatus};

use super::shell::{Pane, Shell};
use super::updates_view::pill;
use super::widgets::{mono, text_input};
use crate::engine::Op;
use crate::icons::{icon, IconName};
use crate::task_board::{self, Which, COLUMNS};
use crate::theme::{metrics, px, Palette};

/// The widest the panel is, in pixels.
const WIDEST: f32 = 420.;

/// The narrowest the panel is, in pixels.
const NARROWEST: f32 = 320.;

/// What the panel holds while a task is being made.
pub struct CreateUi {
    /// The title field.
    pub title: Entity<InputState>,
    /// The description field.
    pub body: Entity<TextareaState>,
    /// The assignee field.
    pub assignee: Entity<InputState>,
    /// The field a label is typed in.
    pub label: Entity<InputState>,
    /// The column it starts in.
    pub status: TaskStatus,
    /// How much it matters.
    pub priority: TaskPriority,
    /// The labels added so far.
    pub labels: Vec<String>,
    /// Why the last attempt was refused.
    pub error: Option<String>,
}

fn status_colour(status: TaskStatus, c: &Palette) -> Hsla {
    match status {
        TaskStatus::Backlog => c.text_muted,
        TaskStatus::Todo => c.info,
        TaskStatus::Doing => c.success,
        TaskStatus::Review => c.warning,
        TaskStatus::Done | TaskStatus::Canceled => c.text_faint,
    }
}

fn priority_colour(priority: TaskPriority, c: &Palette) -> Hsla {
    match priority {
        TaskPriority::Urgent => c.error,
        TaskPriority::High => c.warning,
        TaskPriority::Medium => c.info,
        TaskPriority::Low | TaskPriority::None => c.text_faint,
    }
}

impl Shell {
    /// Whether the panel for a new task is open.
    pub(super) fn create_open(&self) -> bool {
        self.tasks_ui.create.is_some()
    }

    /// Whether a field of the panel has the keyboard.
    pub(super) fn create_typing(&self, window: &Window, cx: &gpui_kit::App) -> bool {
        self.tasks_ui.create.as_ref().is_some_and(|create| {
            create.title.read(cx).focus_handle(cx).is_focused(window)
                || create.body.read(cx).focus_handle(cx).is_focused(window)
                || create.assignee.read(cx).focus_handle(cx).is_focused(window)
                || create.label.read(cx).focus_handle(cx).is_focused(window)
        })
    }

    /// Opens the panel for a new task, the title field holding the keyboard.
    /// `status` is the column it starts in: the one the cursor is in when
    /// none is given. A card's details that are open are saved first.
    pub(super) fn open_create(
        &mut self,
        status: Option<TaskStatus>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.detail_open() && !self.save_detail(cx) {
            // A title that is wrong keeps its card.
            cx.notify();
            return;
        }
        self.tasks_ui.detail = None;
        let status = status.unwrap_or_else(|| self.tasks_ui.board.status());
        let title = cx.new(|cx| InputState::new(window, cx).placeholder("What is to be done"));
        let body = cx.new(|cx| {
            TextareaState::new(window, cx)
                .placeholder("What it is about: why, where, how to know it is done")
        });
        let assignee = cx.new(|cx| InputState::new(window, cx).placeholder("A person or an agent"));
        let label = cx.new(|cx| InputState::new(window, cx).placeholder("Add a label, Enter"));
        // The keyboard is in the title before anything is clicked.
        let handle = title.read(cx).focus_handle(cx);
        handle.focus(window, cx);
        self.tasks_ui.create = Some(CreateUi {
            title,
            body,
            assignee,
            label,
            status,
            priority: TaskPriority::None,
            labels: Vec::new(),
            error: None,
        });
        self.pane = Pane::Main;
        cx.notify();
    }

    /// Throws away what was typed and closes the panel.
    pub(super) fn close_create(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.tasks_ui.create = None;
        self.pane = Pane::Main;
        self.focus.focus(window, cx);
        cx.notify();
    }

    /// Makes the task from what was typed, when it can be. The panel stays
    /// open, with the reason, when it cannot.
    pub(super) fn submit_create(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(create) = self.tasks_ui.create.as_mut() else {
            return;
        };
        let title = task_board::clean_line(create.title.read(cx).value().as_ref());
        if title.is_empty() {
            create.error = Some("A task needs a title.".to_owned());
            cx.notify();
            return;
        }
        let body = task_board::clean_body(create.body.read(cx).value().as_ref());
        let assignee = task_board::clean_line(create.assignee.read(cx).value().as_ref());
        // A label typed and not yet added is not lost.
        let mut labels = create.labels.clone();
        if let Some(all) = task_board::with_label(&labels, create.label.read(cx).value().as_ref()) {
            labels = all;
        }
        let (status, priority) = (create.status, create.priority);
        let root = match &self.tasks_ui.which {
            Which::Project(root) => Some(root.clone()),
            Which::Global => None,
        };
        self.tasks_ui.follow_title = Some(title.clone());
        self.engine.submit(Op::AddTask {
            root,
            title,
            status,
            body,
            priority: (priority != TaskPriority::None).then_some(priority),
            assignee: Some(assignee).filter(|name| !name.is_empty()),
            labels,
        });
        self.close_create(window, cx);
    }

    /// A key while the panel is open. `true` when it was the panel's.
    pub(super) fn create_key(
        &mut self,
        stroke: &gpui_kit::Keystroke,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let m = &stroke.modifiers;
        if stroke.key == "enter" && (m.platform || m.control) && !m.alt {
            self.submit_create(window, cx);
            return true;
        }
        if stroke.key != "enter" || m.platform || m.control || m.alt || m.shift {
            return false;
        }
        let Some(create) = self.tasks_ui.create.as_ref() else {
            return false;
        };
        let focused = |field: &Entity<InputState>, cx: &gpui_kit::App| {
            field.read(cx).focus_handle(cx).is_focused(window)
        };
        let (title, assignee, label) = (
            focused(&create.title, cx),
            focused(&create.assignee, cx),
            focused(&create.label, cx),
        );
        if label {
            let typed = create.label.read(cx).value().to_string();
            let field = create.label.clone();
            if let Some(all) = task_board::with_label(&create.labels, &typed) {
                if let Some(create) = self.tasks_ui.create.as_mut() {
                    create.labels = all;
                }
            }
            field.update(cx, |field, cx| field.set_value("", window, cx));
            cx.notify();
            return true;
        }
        if title || assignee {
            self.submit_create(window, cx);
            return true;
        }
        // Enter in the description is a new line.
        false
    }

    fn create_remove_label(&mut self, name: &str) {
        if let Some(create) = self.tasks_ui.create.as_mut() {
            create.labels.retain(|label| label != name);
        }
    }

    /// The panel, over the right edge of the board.
    pub(super) fn render_create(
        &self,
        c: &Palette,
        cx: &mut Context<Self>,
    ) -> Option<Stateful<Div>> {
        let create = self.tasks_ui.create.as_ref()?;
        let c = *c;
        let width = px(WIDEST)
            .min(self.viewport.width * 0.36)
            .max(px(NARROWEST));
        let body_height = if self.viewport.height < px(900.) {
            90.
        } else {
            150.
        };
        let field = |content: gpui_kit::AnyElement| {
            div()
                .px(px(8.))
                .py(px(4.))
                .rounded(metrics::RADIUS())
                .border_1()
                .border_color(c.elevated_border)
                .bg(c.background)
                .child(content)
        };
        let group = |title: &str, content: Div| {
            div()
                .flex()
                .flex_col()
                .gap(px(6.))
                .child(
                    mono(title.to_owned())
                        .text_size(metrics::TEXT_SMALL())
                        .text_color(c.text_faint),
                )
                .child(content)
        };
        let chip = |id: String, label: String, on: bool, tone: Hsla| {
            let selector = id.clone();
            mono(label)
                .id(SharedString::from(id))
                .debug_selector(move || selector.clone())
                .flex_none()
                .px(px(8.))
                .py(px(3.))
                .rounded(metrics::RADIUS())
                .border_1()
                .border_color(if on { tone } else { c.elevated_border })
                .bg(if on { c.surface_2 } else { c.surface })
                .text_color(if on { c.text } else { c.text_muted })
                .cursor_pointer()
        };

        // The column it starts in.
        let mut status = div().flex().flex_wrap().gap(px(6.));
        for option in COLUMNS {
            status = status.child(
                chip(
                    format!("new-status-{}", option.as_str()),
                    option.as_str().to_uppercase(),
                    create.status == option,
                    status_colour(option, &c),
                )
                .on_click(cx.listener(move |this, _, _, cx| {
                    if let Some(create) = this.tasks_ui.create.as_mut() {
                        create.status = option;
                    }
                    cx.notify();
                })),
            );
        }
        let mut priority = div().flex().flex_wrap().gap(px(6.));
        for option in TaskPriority::ALL {
            priority = priority.child(
                chip(
                    format!("new-priority-{}", option.as_str()),
                    option.as_str().to_uppercase(),
                    create.priority == option,
                    priority_colour(option, &c),
                )
                .on_click(cx.listener(move |this, _, _, cx| {
                    if let Some(create) = this.tasks_ui.create.as_mut() {
                        create.priority = option;
                    }
                    cx.notify();
                })),
            );
        }
        let mut labels = div().flex().flex_wrap().gap(px(6.));
        for (index, name) in create.labels.iter().enumerate() {
            let remove = name.clone();
            labels = labels.child(
                div()
                    .id(("new-label", index))
                    .debug_selector(move || format!("new-label-{index}"))
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap(px(5.))
                    .px(px(7.))
                    .py(px(2.))
                    .rounded(metrics::RADIUS())
                    .bg(c.surface_2)
                    .child(
                        mono(name.clone())
                            .text_size(metrics::TEXT_SMALL())
                            .text_color(c.text),
                    )
                    .child(
                        div()
                            .id(("new-label-x", index))
                            .debug_selector(move || format!("new-label-remove-{index}"))
                            .cursor_pointer()
                            .child(icon(IconName::X, px(10.), c.text_muted))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.create_remove_label(&remove);
                                cx.notify();
                            })),
                    ),
            );
        }

        let content = div()
            .id("tasks-create-body")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .px_4()
            .py_3()
            .flex()
            .flex_col()
            .gap_3()
            .child(group(
                "TITLE",
                div()
                    .flex()
                    .flex_col()
                    .gap(px(4.))
                    .child(field(text_input(&create.title).into_any_element()))
                    .children(create.error.as_ref().map(|why| {
                        div()
                            .debug_selector(|| "new-task-error".into())
                            .text_size(metrics::TEXT_SMALL())
                            .text_color(c.error)
                            .child(why.clone())
                    })),
            ))
            .child(group(
                "DESCRIPTION",
                field(
                    Textarea::new(&create.body)
                        .appearance(false)
                        .bordered(false)
                        .h(px(body_height))
                        .into_any_element(),
                ),
            ))
            .child(group("COLUMN", status))
            .child(group("PRIORITY", priority))
            .child(group(
                "ASSIGNED TO",
                field(text_input(&create.assignee).into_any_element()),
            ))
            .child(group(
                "LABELS",
                div()
                    .flex()
                    .flex_col()
                    .gap(px(6.))
                    .child(labels)
                    .child(field(text_input(&create.label).into_any_element())),
            ));

        let footer = div()
            .flex_none()
            .px_4()
            .py(px(10.))
            .border_t_1()
            .border_color(c.border)
            .flex()
            .items_center()
            .gap(px(8.))
            .child(
                pill("new-task-create", "Create task  ⌘⏎", &c)
                    .border_color(c.signal)
                    .on_click(cx.listener(|this, _, window, cx| this.submit_create(window, cx))),
            )
            .child(
                pill("new-task-cancel", "Cancel  esc", &c)
                    .on_click(cx.listener(|this, _, window, cx| this.close_create(window, cx))),
            );

        Some(
            div()
                .id("tasks-create")
                .debug_selector(|| "tasks-create".into())
                // Over the board, against its right edge, as tall as it.
                .absolute()
                .top_0()
                .right_0()
                .bottom_0()
                .w(width)
                .flex()
                .flex_col()
                .border_l_1()
                .border_color(c.elevated_border)
                .bg(c.surface)
                .shadow_md()
                .occlude()
                .on_click(|_, _, cx| cx.stop_propagation())
                .child(
                    div()
                        .flex_none()
                        .px_4()
                        .py(px(10.))
                        .border_b_1()
                        .border_color(c.border)
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .child(
                            div()
                                .flex_none()
                                .size(px(8.))
                                .bg(status_colour(create.status, &c)),
                        )
                        .child(
                            div()
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(c.text)
                                .child("New task"),
                        )
                        .child(div().flex_1())
                        .child(pill("new-task-close", "Close  esc", &c).on_click(
                            cx.listener(|this, _, window, cx| this.close_create(window, cx)),
                        )),
                )
                .child(content)
                .child(footer),
        )
    }
}
