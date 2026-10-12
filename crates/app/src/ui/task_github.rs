//! GitHub issues in the window: the chip of the header, the panel that asks
//! before sync is turned on, the `GH #n` of a card and the GitHub group of a
//! card's details.
//!
//! Turning sync on writes to the person's repository (it makes the
//! `status:*` and `priority:*` labels there), so it is asked first: the
//! panel names the repository `gh` found for the folder, says what will be
//! done and what never will, and waits for **Turn on**. An issue is opened
//! only by **Create GitHub issue** on one card. The window never talks to
//! GitHub itself: every call is an engine operation (`Op::EnableTaskSync`,
//! `Op::SyncTasks`, `Op::CreateIssue`, `Op::UnlinkTask`).

use std::collections::HashMap;

use gpui_kit::prelude::*;
use gpui_kit::{div, Context, Div, FontWeight, SharedString, Stateful};
use leon_core::{MemoryScope, SyncConfig, Task as Card};
use leon_remote::RepoInfo;

use super::shell::Shell;
use super::updates_view::pill;
use super::widgets::mono;
use crate::engine::Op;
use crate::theme::{metrics, px, Palette};

/// How often the open board syncs by itself, in the ticks of the board's
/// refresh (1.5 s each): every 90 seconds.
pub const SYNC_EVERY_TICKS: u32 = 60;

/// The panel that asks before sync is turned on, or says how it stands.
pub struct SyncPanel {
    /// What was found for the folder: the repository, or why there is none.
    /// `None` while it is being asked.
    pub found: Option<Result<RepoInfo, String>>,
}

/// What the window knows of syncing, read from the store with the board.
#[derive(Default)]
pub struct SyncState {
    /// How the project is synced, when it is.
    pub config: Option<SyncConfig>,
    /// The issue of each linked task: its number and address.
    pub links: HashMap<String, (i64, String)>,
}

impl Shell {
    /// Whether the board's project is synced with GitHub.
    pub(super) fn github_on(&self) -> bool {
        self.tasks_ui.sync.config.is_some()
    }

    /// Reads how the project is synced and which tasks are linked. Called
    /// with the board, so what is shown is the store's.
    pub(super) fn tasks_load_sync(&mut self, scope: &MemoryScope) {
        let store = self.engine.store().clone();
        let mut state = SyncState::default();
        if matches!(scope, MemoryScope::Project(_)) {
            if let Ok(Some(config)) = store.sync_config(scope) {
                if let Ok(links) = store.task_links(&config.provider, &config.repo) {
                    for link in links {
                        if let Some(task) = link.task_id {
                            state.links.insert(task, (link.number, link.url));
                        }
                    }
                }
                state.config = Some(config);
            }
        }
        self.tasks_ui.sync = state;
    }

    /// Syncs now, when sync is on.
    pub(super) fn sync_now(&mut self, cx: &mut Context<Self>) {
        if let (Some(project), true) = (self.tasks_ui.project.clone(), self.github_on()) {
            self.engine.submit(Op::SyncTasks(project));
        }
        cx.notify();
    }

    /// Opens the panel: the question when sync is off, the state when on.
    pub(super) fn open_sync_panel(&mut self, cx: &mut Context<Self>) {
        let Some(project) = self.tasks_ui.project.clone() else {
            self.engine.report(
                crate::engine::StatusKind::Info,
                "GitHub sync belongs to a project of this computer: choose one in the header.",
            );
            return;
        };
        self.tasks_ui.create = None;
        self.tasks_ui.detail = None;
        if self.github_on() {
            self.tasks_ui.sync_panel = Some(SyncPanel { found: None });
            cx.notify();
            return;
        }
        self.tasks_ui.sync_panel = Some(SyncPanel { found: None });
        let asked = self.engine.github_repository(project);
        cx.spawn(async move |this, cx| {
            let found = match asked.await {
                Ok(found) => found,
                Err(_) => Err("The question was not answered.".to_owned()),
            };
            this.update(cx, |this, cx| {
                if let Some(panel) = this.tasks_ui.sync_panel.as_mut() {
                    panel.found = Some(found);
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    pub(super) fn close_sync_panel(&mut self, cx: &mut Context<Self>) {
        self.tasks_ui.sync_panel = None;
        cx.notify();
    }

    pub(super) fn turn_on_sync(&mut self, cx: &mut Context<Self>) {
        let repo = self
            .tasks_ui
            .sync_panel
            .as_ref()
            .and_then(|panel| panel.found.as_ref())
            .and_then(|found| found.as_ref().ok())
            .map(|repo| repo.name.clone());
        if let (Some(project), Some(repo)) = (self.tasks_ui.project.clone(), repo) {
            self.engine.submit(Op::EnableTaskSync { project, repo });
        }
        self.close_sync_panel(cx);
    }

    fn turn_off_sync(&mut self, cx: &mut Context<Self>) {
        if let Some(project) = self.tasks_ui.project.clone() {
            self.engine.submit(Op::DisableTaskSync(project));
        }
        self.close_sync_panel(cx);
    }

    /// The chip of the header: how the project stands with GitHub.
    pub(super) fn github_chip(&self, c: &Palette, cx: &mut Context<Self>) -> Option<Stateful<Div>> {
        self.tasks_ui.project.as_ref()?;
        let (label, tone) = match &self.tasks_ui.sync.config {
            None => ("GITHUB · OFF".to_owned(), c.text_muted),
            Some(config) if config.last_error.is_some() => {
                (format!("GITHUB · {} · FAILED", config.repo), c.warning)
            }
            Some(config) => {
                let when = config
                    .last_ok_at
                    .map(|at| {
                        let seconds = (self.now().timestamp_millis() - at).max(0) / 1000;
                        leon_usage::ago(seconds)
                    })
                    .unwrap_or_else(|| "not yet".to_owned());
                (format!("GITHUB · {} · {when}", config.repo), c.success)
            }
        };
        let c = *c;
        Some(
            mono(label)
                .id(SharedString::from("tasks-github"))
                .debug_selector(|| "tasks-github".into())
                .flex_none()
                .px(px(8.))
                .py(px(3.))
                .rounded(metrics::RADIUS())
                .border_1()
                .border_color(c.elevated_border)
                .bg(c.surface)
                .text_color(tone)
                .cursor_pointer()
                .on_click(cx.listener(|this, _, _, cx| this.open_sync_panel(cx))),
        )
    }

    /// The panel over the board's right edge.
    pub(super) fn render_sync_panel(
        &self,
        c: &Palette,
        cx: &mut Context<Self>,
    ) -> Option<Stateful<Div>> {
        let panel = self.tasks_ui.sync_panel.as_ref()?;
        let c = *c;
        let width = px(420.).min(self.viewport.width * 0.36).max(px(320.));
        let line = |text: String| {
            div()
                .text_size(metrics::TEXT_SMALL())
                .text_color(c.text_muted)
                .child(text)
        };
        let caption = |text: &str| {
            mono(text.to_owned())
                .text_size(metrics::TEXT_SMALL())
                .text_color(c.text_faint)
        };
        let mut body = div()
            .flex_1()
            .min_h_0()
            .px_4()
            .py_3()
            .flex()
            .flex_col()
            .gap_3();
        let mut buttons = div().flex().flex_wrap().gap(px(6.));
        match (&self.tasks_ui.sync.config, &panel.found) {
            (Some(config), _) => {
                body = body
                    .child(caption("REPOSITORY"))
                    .child(
                        div()
                            .debug_selector(|| "github-repo".into())
                            .text_color(c.text)
                            .child(config.repo.clone()),
                    )
                    .child(caption("LAST SYNC"))
                    .child(line(match (config.last_ok_at, &config.last_error) {
                        (_, Some(why)) => format!("It failed: {why}"),
                        (Some(at), None) => {
                            let seconds = (self.now().timestamp_millis() - at).max(0) / 1000;
                            leon_usage::ago(seconds)
                        }
                        (None, None) => "It has not run yet.".to_owned(),
                    }))
                    .child(line(
                        "It syncs when the board opens and every minute and a half while it \
                         is open. Issues are opened only from a card."
                            .to_owned(),
                    ));
                buttons = buttons
                    .child(
                        pill("github-sync-now", "Sync now  s", &c)
                            .border_color(c.signal)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.sync_now(cx);
                                this.close_sync_panel(cx);
                            })),
                    )
                    .child(
                        pill("github-off", "Turn off", &c)
                            .on_click(cx.listener(|this, _, _, cx| this.turn_off_sync(cx))),
                    );
            }
            (None, None) => {
                body = body.child(line(
                    "Looking for the repository of this folder\u{2026}".to_owned(),
                ));
            }
            (None, Some(Err(why))) => {
                body = body.child(
                    div()
                        .debug_selector(|| "github-problem".into())
                        .text_color(c.warning)
                        .child(why.clone()),
                );
            }
            (None, Some(Ok(repo))) => {
                body = body
                    .child(caption("REPOSITORY"))
                    .child(
                        div()
                            .debug_selector(|| "github-repo".into())
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(c.text)
                            .child(repo.name.clone()),
                    )
                    .child(line(repo.url.clone()))
                    .child(caption("TURNING IT ON WILL"))
                    .child(line(
                        "\u{2022} make the labels status:backlog, status:todo, status:doing, \
                         status:review and priority:low to priority:urgent in that repository"
                            .to_owned(),
                    ))
                    .child(line(
                        "\u{2022} read its issues: open ones become tasks, and a task and its \
                         issue are kept equal (title, description, column, priority, labels)"
                            .to_owned(),
                    ))
                    .child(caption("IT WILL NEVER"))
                    .child(line(
                        "\u{2022} open an issue by itself: only Create GitHub issue on a card does"
                            .to_owned(),
                    ))
                    .child(line("\u{2022} delete an issue".to_owned()));
                buttons = buttons.child(
                    pill("github-on", "Turn on  \u{23ce}", &c)
                        .border_color(c.signal)
                        .on_click(cx.listener(|this, _, _, cx| this.turn_on_sync(cx))),
                );
            }
        }
        buttons = buttons.child(
            pill("github-close", "Close  esc", &c)
                .on_click(cx.listener(|this, _, _, cx| this.close_sync_panel(cx))),
        );
        Some(
            div()
                .id("tasks-github-panel")
                .debug_selector(|| "tasks-github-panel".into())
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
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(c.text)
                        .child("GitHub issues"),
                )
                .child(body)
                .child(
                    div()
                        .flex_none()
                        .px_4()
                        .py(px(10.))
                        .border_t_1()
                        .border_color(c.border)
                        .child(buttons),
                ),
        )
    }

    /// The GitHub group of a card's details.
    pub(super) fn github_group(&self, card: &Card, c: &Palette, cx: &mut Context<Self>) -> Div {
        let c = *c;
        let link = self.tasks_ui.sync.links.get(&card.id).cloned();
        let repo = self
            .tasks_ui
            .sync
            .config
            .as_ref()
            .map(|config| config.repo.clone());
        match (link, repo) {
            (Some((number, url)), repo) => {
                let id = card.id.clone();
                div()
                    .debug_selector(|| "task-github-linked".into())
                    .flex()
                    .flex_col()
                    .gap(px(6.))
                    .child(div().text_color(c.text).child(format!(
                        "#{number}{}",
                        repo.map(|repo| format!(" on {repo}")).unwrap_or_default()
                    )))
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .gap(px(6.))
                            .child(pill("task-github-open", "Open on GitHub", &c).on_click(
                                cx.listener(move |this, _, _, cx| {
                                    (this.options.open_url)(cx, &url);
                                }),
                            ))
                            .child(
                                pill("task-github-unlink", "Unlink", &c).on_click(cx.listener(
                                    move |this, _, _, cx| {
                                        this.engine.submit(Op::UnlinkTask { id: id.clone() });
                                        cx.notify();
                                    },
                                )),
                            ),
                    )
            }
            (None, Some(repo)) => {
                let id = card.id.clone();
                div()
                    .debug_selector(|| "task-github-unlinked".into())
                    .flex()
                    .flex_col()
                    .gap(px(6.))
                    .child(
                        div()
                            .text_size(metrics::TEXT_SMALL())
                            .text_color(c.text_muted)
                            .child(format!("No issue yet. Creating one opens it in {repo}.")),
                    )
                    .child(
                        pill("task-github-create", "Create GitHub issue", &c).on_click(
                            cx.listener(move |this, _, _, cx| {
                                this.engine.submit(Op::CreateIssue { id: id.clone() });
                                cx.notify();
                            }),
                        ),
                    )
            }
            (None, None) => div()
                .debug_selector(|| "task-github-off".into())
                .text_size(metrics::TEXT_SMALL())
                .text_color(c.text_muted)
                .child("Turn on GitHub sync in the header to link tasks with issues."),
        }
    }
}
