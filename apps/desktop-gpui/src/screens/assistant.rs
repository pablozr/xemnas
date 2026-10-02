//! Xemnas, the assistant: the mascot that sits at the foot of the sidebar
//! and the panel it opens.
//!
//! The mascot is a pre-rendered toon figure (`assets/mascot`, rendered
//! offline from a signed-distance model) so it reads as 3D without a 3D
//! renderer in the app; `mascot.rs` plays its frames. It floats, blinks and
//! glances, every move computed from the clock at the display's own rate and
//! only while it lasts; with reduced motion it only blinks. The panel offers what the app can actually do now: it reports
//! the real queue and takes the person to the right screen. Conversation with
//! a model is not wired yet, and the panel says so instead of faking it.

use std::sync::{Arc, LazyLock};
use std::time::{Duration, Instant};

use gpui::prelude::*;
use gpui::{
    div, img, px, AnyElement, Context, Div, EventEmitter, Image, ImageFormat, Render, Role, Window,
};

use super::mascot::{self, Pose};

use crate::ui::controls::{focus_ring, icon_action};
use crate::ui::icons::{icon, IconName};
use crate::ui::motion::{menu_out, panel_in};
use crate::ui::patterns::section_label;
use crate::ui::popup::{reap, Popup};
use crate::ui::theme::{text_style, Theme};
use crate::ui::tokens::{RadiusScale, SpacingScale, TypeScale};
use crate::ui::tooltip::tooltip;

macro_rules! sprite {
    ($file:literal) => {
        LazyLock::new(|| {
            Arc::new(Image::from_bytes(
                ImageFormat::Png,
                include_bytes!(concat!("../../assets/mascot/", $file)).to_vec(),
            ))
        })
    };
}

static BLINK: LazyLock<Arc<Image>> = sprite!("blink.png");
static GLOW: LazyLock<Arc<Image>> = sprite!("glow.png");

/// The mascot at rest, eyes lit: for the opening mark and empty states.
pub fn portrait() -> Arc<Image> {
    GLOW.clone()
}

/// The mascot with its eyes closed: for "nothing to do here".
pub fn resting() -> Arc<Image> {
    BLINK.clone()
}

/// Side of the mascot on the sidebar.
const DOCK_SIZE: f32 = 72.0;
/// Side of the portrait in the panel header.
const PORTRAIT_SIZE: f32 = 44.0;
/// Width and height of the panel.
pub const PANEL_WIDTH: f32 = 400.0;
const PANEL_HEIGHT: f32 = 540.0;
/// One breath of the docked mascot: rise and settle.
const BREATH: Duration = Duration::from_millis(2200);
/// Height of a breath, in pixels.
const BREATH_LIFT: f32 = 3.0;
/// How long the eyes take to light or dim.
const GLOW_FADE: Duration = Duration::from_millis(160);
/// Quiet time between one thing the mascot does and the next.
const REST: Duration = Duration::from_millis(2600);

/// Where the assistant can take the person.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssistantRoute {
    /// The review queue.
    Review,
    /// The project overview.
    Overview,
    /// The map's timeline.
    Timeline,
    /// Context › Test a task.
    TestTask,
    /// The map's suggestions.
    Suggestions,
}

impl AssistantRoute {
    /// The route string the shell understands (`destination:view`).
    pub fn route(self) -> &'static str {
        match self {
            Self::Review => "review",
            Self::Overview => "overview",
            Self::Timeline => "map:timeline",
            Self::TestTask => "context:test",
            Self::Suggestions => "map:suggestions",
        }
    }
}

/// Emitted when the person picks a place in the panel.
pub struct AssistantGo(pub AssistantRoute);

/// One thing the mascot does, played from its start by the clock.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Gesture {
    Blink,
    Look(usize),
}

impl Gesture {
    fn duration(self) -> Duration {
        match self {
            Self::Blink => Duration::from_millis(190),
            Self::Look(_) => Duration::from_millis(1300),
        }
    }

    /// The frame `t` (0 to 1) into the gesture.
    fn frame(self, t: f32) -> usize {
        match self {
            // Shut and open again: 0, 1 .. 4 .. 1, 0 closed steps.
            Self::Blink => {
                let closed = 1.0 - (2.0 * t - 1.0).abs();
                match (closed * mascot::BLINK_STEPS as f32).round() as usize {
                    0 => mascot::REST,
                    step => mascot::BLINK_FIRST + step - 1,
                }
            }
            // Turn, hold, turn back, each leg eased so it starts and ends
            // slowly.
            Self::Look(to) => {
                let turn = if t < 0.2 {
                    smooth(t / 0.2)
                } else if t < 0.65 {
                    1.0
                } else {
                    1.0 - smooth((t - 0.65) / 0.35)
                };
                let rest = mascot::REST as f32;
                (rest + (to as f32 - rest) * turn).round() as usize
            }
        }
    }
}

/// Ease in and out over 0 to 1.
fn smooth(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// What the assistant knows to say: only real numbers from the shell.
#[derive(Clone, Default, PartialEq, Eq)]
pub struct Briefing {
    /// Name of the selected project.
    pub project: Option<String>,
    /// Candidates waiting for review in it.
    pub pending: Option<usize>,
}

/// The mascot and its panel; the shell places both.
pub struct AssistantScreen {
    /// The panel, held mounted while it leaves.
    panel: Popup<()>,
    hovered: bool,
    /// What it is doing now with the head or eyes, and when it began.
    gesture: Option<(Gesture, Instant)>,
    /// When the current breath began.
    breath: Option<Instant>,
    /// How lit the eyes were when they last changed, where they are headed
    /// and when that began.
    glow_from: f32,
    glow_to: bool,
    glow_at: Instant,
    briefing: Briefing,
    focus: gpui::FocusHandle,
}

impl EventEmitter<AssistantGo> for AssistantScreen {}

impl AssistantScreen {
    /// Starts the idle routine: every few seconds a blink, a breath or a
    /// glance, in turn.
    pub fn new(cx: &mut Context<Self>) -> Self {
        cx.spawn(async move |this, cx| {
            let mut round = 0_usize;
            loop {
                cx.background_executor().timer(REST).await;
                let still = match this.update(cx, |_, cx| cx.reduce_motion()) {
                    Ok(still) => still,
                    Err(_) => break,
                };
                let gesture = match round % 6 {
                    0 | 3 => Some(Gesture::Blink),
                    2 if !still => Some(Gesture::Look(mascot::LOOK_LEFT)),
                    5 if !still => Some(Gesture::Look(mascot::LOOK_RIGHT)),
                    _ => None,
                };
                let breathe = !still && matches!(round % 6, 1 | 4);
                let wait = gesture.map_or(if breathe { BREATH } else { Duration::ZERO }, |g| {
                    g.duration()
                });
                let started = this.update(cx, |screen, cx| {
                    let now = Instant::now();
                    if let Some(gesture) = gesture {
                        screen.gesture = Some((gesture, now));
                    }
                    if breathe {
                        screen.breath = Some(now);
                    }
                    cx.notify();
                });
                if started.is_err() {
                    break;
                }
                cx.background_executor().timer(wait).await;
                round = round.wrapping_add(1);
            }
        })
        .detach();
        Self {
            panel: Popup::default(),
            hovered: false,
            gesture: None,
            breath: None,
            glow_from: 0.0,
            glow_to: false,
            glow_at: Instant::now(),
            briefing: Briefing::default(),
            focus: cx.focus_handle().tab_stop(true),
        }
    }

    /// Whether the panel is open.
    pub fn is_open(&self) -> bool {
        self.panel.is_open()
    }

    /// What the shell knows now: project and queue.
    pub fn set_briefing(&mut self, briefing: Briefing, cx: &mut Context<Self>) {
        if self.briefing != briefing {
            self.briefing = briefing;
            cx.notify();
        }
    }

    /// Opens or closes the panel.
    pub fn toggle(&mut self, cx: &mut Context<Self>) {
        if self.panel.is_open() {
            self.close(cx);
        } else {
            self.panel.open(());
            cx.emit(AssistantToggled);
            cx.notify();
        }
    }

    /// Closes the panel.
    pub fn close(&mut self, cx: &mut Context<Self>) {
        if self.panel.begin_close() {
            reap(cx, |assistant: &mut Self| &mut assistant.panel);
            cx.emit(AssistantToggled);
            cx.notify();
        }
    }

    /// What the mascot looks like now, computed from the clock: the frame of
    /// the head, how high it floats and how lit the eyes are.
    fn pose(&mut self, cx: &mut Context<Self>) -> Pose {
        let now = Instant::now();
        let still = cx.reduce_motion();
        let mut animating = false;

        let lit = self.hovered || self.panel.is_open();
        let level = |from: f32, to: bool, at: Instant| {
            let target = if to { 1.0 } else { 0.0 };
            if still {
                return (target, false);
            }
            let t = now.duration_since(at).as_secs_f32() / GLOW_FADE.as_secs_f32();
            (from + (target - from) * smooth(t), t < 1.0)
        };
        if lit != self.glow_to {
            self.glow_from = level(self.glow_from, self.glow_to, self.glow_at).0;
            self.glow_to = lit;
            self.glow_at = now;
        }
        let (glow, fading) = level(self.glow_from, self.glow_to, self.glow_at);
        animating |= fading;

        let mut frame = mascot::REST;
        if let Some((gesture, began)) = self.gesture {
            let t = now.duration_since(began).as_secs_f32() / gesture.duration().as_secs_f32();
            if t < 1.0 {
                frame = gesture.frame(t);
                animating = true;
            } else {
                self.gesture = None;
            }
        }

        let mut lift = 0.0;
        if let (Some(began), false) = (self.breath, still) {
            let t = now.duration_since(began).as_secs_f32() / BREATH.as_secs_f32();
            if t < 1.0 {
                // A raised cosine: rises and settles with no velocity at either
                // end, so there is no start or stop to see.
                lift = BREATH_LIFT * (0.5 - 0.5 * (t * std::f32::consts::TAU).cos());
                animating = true;
            } else {
                self.breath = None;
            }
        }
        Pose {
            frame,
            lift,
            glow,
            animating,
        }
    }

    /// The one line the mascot says beside itself: the queue, when there is
    /// one; otherwise its name.
    fn whisper(&self) -> String {
        match self.briefing.pending {
            Some(0) | None => "Xemnas".to_owned(),
            Some(count) => format!("{count} para revisar"),
        }
    }

    /// The mascot docked at the foot of the sidebar.
    fn render_dock(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let theme = Theme::current(cx);
        let colors = theme.colors;
        // Every move is the clock choosing a frame and a sub-pixel lift
        // (`mascot.rs`); between moves nothing animates and nothing redraws.
        let pose = self.pose(cx);
        let figure = mascot::figure(DOCK_SIZE, pose);
        let pending = self.briefing.pending.unwrap_or(0) > 0;
        div()
            .id("assistant-dock")
            .flex()
            .items_center()
            .gap(px(SpacingScale::S2))
            .mx(px(SpacingScale::S2))
            .mb(px(SpacingScale::S1))
            .px(px(SpacingScale::S2))
            .rounded(RadiusScale.surface())
            .cursor_pointer()
            .hover(move |style| style.bg(colors.glass_fill_low()))
            .when(self.panel.is_open(), |dock| {
                dock.bg(colors.glass_fill_medium())
            })
            .role(Role::Button)
            .aria_label(if self.panel.is_open() {
                "Fechar o assistente Xemnas"
            } else {
                "Abrir o assistente Xemnas"
            })
            .track_focus(&self.focus)
            .focus_visible(focus_ring(&theme))
            .tooltip(tooltip("Xemnas, o assistente do projeto", None))
            .on_hover(cx.listener(|this, hovered: &bool, _, cx| {
                this.hovered = *hovered;
                cx.notify();
            }))
            .on_click(cx.listener(|this, _, _, cx| this.toggle(cx)))
            .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, _, cx| {
                match event.keystroke.key.as_str() {
                    "enter" | "space" => {
                        this.toggle(cx);
                        cx.stop_propagation();
                    }
                    "escape" if this.panel.is_open() => {
                        this.close(cx);
                        cx.stop_propagation();
                    }
                    _ => {}
                }
            }))
            .child(figure)
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .flex()
                    .flex_col()
                    .gap(px(2.0))
                    .child(
                        text_style(div(), TypeScale::META)
                            .font_family(Theme::font_mono())
                            .text_color(colors.organization_silver())
                            .child("Nº I"),
                    )
                    .child(
                        text_style(div(), TypeScale::LABEL)
                            .truncate()
                            .text_color(if pending {
                                colors.text_primary()
                            } else {
                                colors.text_secondary()
                            })
                            .child(self.whisper()),
                    ),
            )
            .when(pending, |dock| {
                dock.child(
                    div()
                        .size(px(6.0))
                        .flex_none()
                        .rounded_full()
                        .bg(colors.mascot_ember()),
                )
            })
            .into_any_element()
    }

    /// The panel, anchored by the shell beside the sidebar.
    pub fn render_panel(&mut self, cx: &mut Context<Self>) -> Option<AnyElement> {
        self.panel.get()?;
        let exit = self.panel.exit_progress();
        let theme = Theme::current(cx);
        let colors = theme.colors;
        let greeting = match (&self.briefing.project, self.briefing.pending) {
            (None, _) => "Escolha um projeto na lateral e eu mostro o que ele guarda.".to_owned(),
            (Some(project), Some(0)) => {
                format!("Nada esperando revisão em {project}. As decisões estão em ordem.")
            }
            (Some(project), Some(1)) => {
                format!("Há 1 candidato esperando a sua revisão em {project}.")
            }
            (Some(project), Some(count)) => {
                format!("Há {count} candidatos esperando a sua revisão em {project}.")
            }
            (Some(project), None) => format!("Estou acompanhando {project}."),
        };
        let has_project = self.briefing.project.is_some();
        let pending = self.briefing.pending.unwrap_or(0);
        let routes: Vec<(AssistantRoute, IconName, String, &'static str)> = vec![
            (
                AssistantRoute::Review,
                IconName::Inbox,
                if pending > 0 {
                    format!("Revisar os {pending} candidatos")
                } else {
                    "Abrir a Revisão".to_owned()
                },
                "Confirmar, ajustar ou rejeitar o que foi capturado",
            ),
            (
                AssistantRoute::Overview,
                IconName::Compass,
                "Ler a Visão do projeto".to_owned(),
                "O resumo e os principais fluxos, com fontes",
            ),
            (
                AssistantRoute::Timeline,
                IconName::Clock,
                "Ver o que mudou".to_owned(),
                "A linha do tempo das decisões e regras",
            ),
            (
                AssistantRoute::Suggestions,
                IconName::Lightbulb,
                "Ver as sugestões do mapa".to_owned(),
                "Relações, contexto e vínculos à espera",
            ),
            (
                AssistantRoute::TestTask,
                IconName::Flask,
                "Testar o contexto de uma tarefa".to_owned(),
                "O que o agente receberia para um pedido",
            ),
        ];
        let mut list = div().flex().flex_col();
        for (index, (route, glyph, title, body)) in routes.into_iter().enumerate() {
            list = list.child(
                div()
                    .id(("assistant-route", index))
                    .flex()
                    .items_center()
                    .gap(px(SpacingScale::S3))
                    .px(px(SpacingScale::S3))
                    .py(px(SpacingScale::S2))
                    .rounded(theme.radius.control())
                    .cursor_pointer()
                    .hover(move |style| style.bg(colors.glass_fill_medium()))
                    .active(move |style| style.bg(colors.glass_fill_strong()))
                    .role(Role::Button)
                    .aria_label(title.clone())
                    .focus_visible(focus_ring(&theme))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        // A leaving panel takes no clicks.
                        if !this.panel.is_open() {
                            return;
                        }
                        this.close(cx);
                        cx.emit(AssistantGo(route));
                    }))
                    .child(icon(glyph, 16.0, colors.text_muted()))
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.0))
                            .flex()
                            .flex_col()
                            .gap(px(2.0))
                            .child(text_style(div(), TypeScale::ROW_TITLE).child(title))
                            .child(
                                text_style(div(), TypeScale::META)
                                    .text_color(colors.text_muted())
                                    .child(body),
                            ),
                    )
                    .child(icon(IconName::ChevronRight, 14.0, colors.text_disabled())),
            );
        }
        let close = icon_action(&theme, "assistant-close", "Fechar o assistente")
            .tooltip(tooltip("Fechar", Some("Esc")))
            .on_click(cx.listener(|this, _, _, cx| this.close(cx)))
            .child(icon(IconName::Close, 14.0, colors.text_secondary()));
        let panel = div()
            .id("assistant-panel")
            .w(px(PANEL_WIDTH))
            .h(px(PANEL_HEIGHT))
            .flex()
            .flex_col()
            .rounded(RadiusScale.dialog())
            .border_1()
            .border_color(colors.glass_border_card())
            .bg(colors.floating())
            .shadow(crate::ui::material::elevation(
                &theme,
                crate::ui::material::Elevation::Dialog,
            ))
            .overflow_hidden()
            .role(Role::Dialog)
            .aria_label("Assistente Xemnas")
            .occlude()
            .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, _, cx| {
                if event.keystroke.key == "escape" {
                    this.close(cx);
                    cx.stop_propagation();
                }
            }))
            // Header: the portrait, the name and the number.
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(SpacingScale::S3))
                    .px(px(SpacingScale::S4))
                    .pt(px(SpacingScale::S4))
                    .pb(px(SpacingScale::S3))
                    .child(img(GLOW.clone()).size(px(PORTRAIT_SIZE)).flex_none())
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.0))
                            .flex()
                            .flex_col()
                            .child(
                                text_style(div(), TypeScale::HEADING_3)
                                    .font_family(Theme::font_display())
                                    .child("Xemnas"),
                            )
                            .child(
                                text_style(div(), TypeScale::META)
                                    .font_family(Theme::font_mono())
                                    .text_color(colors.organization_silver())
                                    .child("Nº I · assistente do projeto"),
                            ),
                    )
                    .child(close),
            )
            .child(chain_rule(&theme))
            .child(
                div()
                    .id("assistant-body")
                    .flex_1()
                    .min_h(px(0.0))
                    .overflow_y_scroll()
                    .flex()
                    .flex_col()
                    .gap(px(SpacingScale::S4))
                    .p(px(SpacingScale::S4))
                    // What it says, in a bubble from the portrait side.
                    .child(
                        text_style(div(), TypeScale::BODY_SMALL)
                            .px(px(SpacingScale::S3))
                            .py(px(SpacingScale::S2))
                            .rounded(RadiusScale.surface())
                            .bg(colors.glass_fill_medium())
                            .text_color(colors.text_primary())
                            .child(greeting),
                    )
                    .when(has_project, |body| {
                        body.child(
                            div()
                                .flex()
                                .flex_col()
                                .gap(px(SpacingScale::S2))
                                .child(
                                    div()
                                        .px(px(SpacingScale::S3))
                                        .child(section_label(&theme, "Posso levar você a")),
                                )
                                .child(list),
                        )
                    }),
            )
            .child(
                text_style(div(), TypeScale::META)
                    .flex_none()
                    .px(px(SpacingScale::S4))
                    .py(px(SpacingScale::S3))
                    .border_t_1()
                    .border_color(colors.hairline_divider())
                    .text_color(colors.text_muted())
                    .child(
                        "Por enquanto eu guio pelo app. Perguntas em linguagem natural \
                         chegam quando o assistente for ligado ao provedor de IA.",
                    ),
            );
        Some(match exit {
            None => panel_in("assistant-panel-in", panel).into_any_element(),
            // Sinks back toward the mascot as it fades.
            Some(t) => menu_out("assistant-panel-out", 16.0, t, panel).into_any_element(),
        })
    }
}

/// A thin silver chain under the panel header: alternating flat and upright
/// links, the one ornament of the Organization in the interface.
pub fn chain_rule(theme: &Theme) -> Div {
    let silver = theme.colors.organization_silver();
    let mut chain = div().flex().items_center().justify_center().h(px(10.0));
    for index in 0..CHAIN_LINKS {
        // Flat links seen from the side, upright ones from the edge; each
        // overlaps the last so they read as interlocked.
        let link = if index % 2 == 0 {
            div().w(px(11.0)).h(px(6.0)).rounded(px(3.0))
        } else {
            div().w(px(4.0)).h(px(8.0)).rounded(px(2.0))
        };
        chain = chain.child(
            link.flex_none()
                .when(index > 0, |link| link.ml(px(-2.0)))
                .border_1()
                .border_color(silver)
                .opacity(if index == 0 || index == CHAIN_LINKS - 1 {
                    0.35
                } else {
                    0.7
                }),
        );
    }
    chain
}

/// Links in the chain rule: a short length, centred, not a dotted line.
const CHAIN_LINKS: usize = 15;

/// Emitted when the panel opens or closes: the shell, which draws the
/// panel, repaints.
pub struct AssistantToggled;

impl EventEmitter<AssistantToggled> for AssistantScreen {}

impl Render for AssistantScreen {
    /// The dock, as a view of its own: a blink or a breath repaints only
    /// this, not the screens beside it. The panel is drawn by the shell.
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.render_dock(cx)
    }
}
