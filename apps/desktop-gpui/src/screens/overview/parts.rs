//! The architecture of the project as something to open, not to scan.
//!
//! The page holds a short grid of cards, one per part (a top-level component
//! of the map), six at first. A card opens a modal with what the part calls,
//! what calls it, the flows that pass through it and its weight; "Ver
//! diagrama" opens the explorer: one flow at a time drawn as a diagram (a
//! flow is sparse, the whole system is not), or the whole system as a matrix
//! of who calls whom (a matrix stays readable where node-link drawings tangle:
//! no lines, no crossings). The model is `application::architecture`.

use application::architecture::{Architecture, Neighbour, StepRef};
use application::overview::OverviewFlow;
use gpui::prelude::*;
use gpui::{deferred, div, px, AnyElement, Div, Role, SharedString, Size, Stateful};

use super::{OpenEntity, OverviewScreen};
use crate::screens::format::{clipped, plural, roman};
use crate::ui::controls::{action_button, focus_ring, icon_action, ButtonKind};
use crate::ui::icons::{icon, IconName};
use crate::ui::material::{elevation, Elevation};
use crate::ui::motion::panel_in;
use crate::ui::patterns::{count_chip, meter, section_header, segment_label, segmented, share};
use crate::ui::theme::{text_style, Theme};
use crate::ui::tokens::{tint, RadiusScale, SpacingScale, TypeScale};

/// Cards shown before "Mostrar as outras".
const SHOWN_AT_FIRST: usize = 6;
/// Cards per row.
const CARDS_PER_ROW: usize = 3;
/// Height of a card: every card of a grid is the same, whatever it holds.
const CARD_HEIGHT: f32 = 112.0;
/// Width of the explorer and of a part's modal.
const EXPLORER_WIDTH: f32 = 840.0;
const PART_WIDTH: f32 = 620.0;
/// Space above the content area the modal does not cover (tabs and header)
/// plus the margin it keeps from the window's edge.
const MODAL_MARGIN: f32 = 150.0;
/// Width of the matrix's row labels and of one cell.
const MATRIX_LABEL: f32 = 156.0;
const MATRIX_CELL: f32 = 40.0;
/// Longest flow title on a chip.
const FLOW_CHIP_CHARS: usize = 30;

/// What the modal layer shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Modal {
    /// One part, by position in the architecture.
    Part(usize),
    /// The diagram and the matrix.
    Explorer,
}

/// How the explorer draws the architecture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum View {
    /// One flow at a time.
    Diagram,
    /// Everyone against everyone.
    Matrix,
}

/// The explorer's state, kept while a part's modal is open over it so
/// "Voltar" lands where the person was.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Explorer {
    pub view: View,
    pub flow: usize,
    /// The matrix cell (from, to) under the pointer or last pressed.
    pub cell: Option<(usize, usize)>,
}

impl Default for Explorer {
    fn default() -> Self {
        Self {
            view: View::Diagram,
            flow: 0,
            cell: None,
        }
    }
}

impl OverviewScreen {
    /// Opens a part's modal; from the explorer or another part, "Voltar"
    /// returns there.
    pub(super) fn open_part(&mut self, at: usize, cx: &mut gpui::Context<Self>) {
        self.back = match self.modal {
            Some(Modal::Part(current)) if current != at => Some(Modal::Part(current)),
            Some(Modal::Explorer) => Some(Modal::Explorer),
            _ => None,
        };
        self.modal = Some(Modal::Part(at));
        self.focus_modal = true;
        self.hover_box = None;
        cx.notify();
    }

    /// Opens the part whose entity is `entity_id` (a click on a drawn box,
    /// whose position is that of the narrowed architecture).
    pub(super) fn open_part_by_id(&mut self, entity_id: &str, cx: &mut gpui::Context<Self>) {
        let at = self
            .view
            .as_ref()
            .and_then(|view| view.overview.architecture.position(entity_id));
        if let Some(at) = at {
            self.open_part(at, cx);
        }
    }

    /// Opens the explorer, on `flow` when given.
    pub(super) fn open_explorer(&mut self, flow: Option<usize>, cx: &mut gpui::Context<Self>) {
        if let Some(flow) = flow {
            self.explorer.flow = flow;
            self.explorer.view = View::Diagram;
        }
        self.modal = Some(Modal::Explorer);
        self.back = None;
        self.focus_modal = true;
        cx.notify();
    }

    /// Opens a modal by its demo route name (see `open_demo_modal`).
    pub(super) fn open_by_name(&mut self, name: &str) {
        let number = |prefix: &str| {
            name.strip_prefix(prefix)
                .and_then(|n| n.parse::<usize>().ok())
        };
        if name == "matrix" {
            self.explorer.view = View::Matrix;
            self.modal = Some(Modal::Explorer);
        } else if let Some(flow) = number("explorer") {
            self.explorer.flow = flow;
            self.modal = Some(Modal::Explorer);
        } else if name == "explorer" {
            self.modal = Some(Modal::Explorer);
        } else if let Some(at) = number("part") {
            self.modal = Some(Modal::Part(at));
        }
        self.focus_modal = self.modal.is_some();
    }

    pub(super) fn close_modal(&mut self, cx: &mut gpui::Context<Self>) {
        if self.modal.take().is_some() {
            self.back = None;
            self.hover_step = None;
            cx.notify();
        }
    }

    fn go_back(&mut self, cx: &mut gpui::Context<Self>) {
        self.modal = self.back.take();
        self.focus_modal = true;
        cx.notify();
    }

    /// The parts as a grid of cards, six at first.
    pub(super) fn parts_section(
        &mut self,
        theme: &Theme,
        architecture: &Architecture,
        cx: &mut gpui::Context<Self>,
    ) -> Div {
        let colors = theme.colors;
        let total = architecture.containers.len();
        let heaviest = architecture
            .containers
            .iter()
            .map(|container| container.decisions)
            .max()
            .unwrap_or(0);
        let shown = if self.parts_expanded {
            total
        } else {
            total.min(SHOWN_AT_FIRST)
        };
        let mut grid = div().flex().flex_col().gap(px(SpacingScale::S3));
        for start in (0..shown).step_by(CARDS_PER_ROW) {
            let mut row = div().flex().gap(px(SpacingScale::S3));
            for at in start..start + CARDS_PER_ROW {
                row = row.child(if at < shown {
                    self.part_card(theme, architecture, at, heaviest, cx)
                        .into_any_element()
                } else {
                    div().flex_1().min_w(px(0.0)).into_any_element()
                });
            }
            grid = grid.child(row);
        }
        let flows_exist = !architecture.interactions.is_empty();
        let diagram = flows_exist.then(|| {
            let button = action_button(theme, "overview-open-explorer", ButtonKind::Ghost, true)
                .aria_label("Ver diagrama")
                .child(icon(IconName::Blocks, 14.0, colors.text_secondary()))
                .child("Ver diagrama");
            self.pressable(
                button,
                "overview-open-explorer",
                |this, cx| this.open_explorer(None, cx),
                cx,
            )
        });
        let toggle = (total > SHOWN_AT_FIRST).then(|| {
            let label = if self.parts_expanded {
                "Mostrar menos".to_owned()
            } else {
                format!("Mostrar as outras {}", total - SHOWN_AT_FIRST)
            };
            let button = action_button(theme, "overview-parts-toggle", ButtonKind::Ghost, true)
                .aria_label(label.clone())
                .child(icon(
                    if self.parts_expanded {
                        IconName::ChevronUp
                    } else {
                        IconName::ChevronDown
                    },
                    14.0,
                    colors.text_secondary(),
                ))
                .child(label);
            div().flex().justify_center().child(self.pressable(
                button,
                "overview-parts-toggle",
                |this, cx| {
                    this.parts_expanded = !this.parts_expanded;
                    cx.notify();
                },
                cx,
            ))
        });
        div()
            .flex()
            .flex_col()
            .gap(px(SpacingScale::S3))
            .child(
                section_header(theme, "Arquitetura")
                    .child(count_chip(theme, total.to_string()))
                    .children(diagram),
            )
            .child(
                text_style(div(), TypeScale::BODY_SMALL)
                    .text_color(colors.text_muted())
                    .child(
                        "Cada parte é um componente do Mapa. Abra uma para ver o que ela chama, \
                         quem a chama e os fluxos que passam por ela.",
                    ),
            )
            .child(grid)
            .children(toggle)
            .when(architecture.hidden > 0, |column| {
                column.child(
                    text_style(div(), TypeScale::META)
                        .text_color(colors.text_muted())
                        .child(format!(
                            "Mostra as {total} partes mais ligadas aos fluxos; {} ficam no Mapa.",
                            architecture.hidden
                        )),
                )
            })
    }

    /// One part: its name, what it is, what it is made with and how much it
    /// weighs and connects.
    fn part_card(
        &mut self,
        theme: &Theme,
        architecture: &Architecture,
        at: usize,
        heaviest: usize,
        cx: &mut gpui::Context<Self>,
    ) -> Stateful<Div> {
        let colors = theme.colors;
        let container = &architecture.containers[at];
        let calls = architecture.calls(at).len();
        let called = architecture.called_by(at).len();
        let label = format!(
            "{}: {}. {}, chama {}, chamada por {}. Abrir detalhes.",
            container.name,
            if container.role.is_empty() {
                "sem descrição"
            } else {
                container.role.as_str()
            },
            plural(container.decisions, "decisão", "decisões"),
            calls,
            called
        );
        let id = format!("overview-part-{at}");
        let card = div()
            .id(SharedString::from(id.clone()))
            .flex_1()
            .min_w(px(0.0))
            .h(px(CARD_HEIGHT))
            .p(px(SpacingScale::S3))
            .flex()
            .flex_col()
            .justify_between()
            .rounded(RadiusScale.surface())
            .border_1()
            .border_color(colors.glass_border_card())
            .bg(colors.glass_fill_card())
            .cursor_pointer()
            .hover(move |style| style.border_color(colors.glass_border_card_hover()))
            .role(Role::Button)
            .aria_label(label)
            .focus_visible(focus_ring(theme))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(2.0))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(SpacingScale::S2))
                            .child(icon(IconName::Component, 14.0, colors.text_muted()))
                            .child(
                                text_style(div(), TypeScale::ROW_TITLE)
                                    .flex_1()
                                    .min_w(px(0.0))
                                    .truncate()
                                    .text_color(colors.text_primary())
                                    .child(container.name.clone()),
                            )
                            .when(container.conflicts > 0, |row| {
                                row.child(
                                    div()
                                        .size(px(7.0))
                                        .flex_none()
                                        .rounded_full()
                                        .bg(colors.status_warning()),
                                )
                            }),
                    )
                    .child(
                        text_style(div(), TypeScale::META)
                            .h(px(16.0))
                            .truncate()
                            .text_color(colors.text_muted())
                            .child(container.role.clone()),
                    )
                    .child(
                        text_style(div(), TypeScale::META)
                            .h(px(16.0))
                            .truncate()
                            .font_family(Theme::font_mono())
                            .text_color(colors.graph_technology())
                            .child(container.technologies.join(" · ")),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(SpacingScale::S2))
                    .child(meter(
                        theme,
                        share(container.decisions, heaviest),
                        colors.accent_default(),
                    ))
                    .child(
                        text_style(div(), TypeScale::META)
                            .flex()
                            .justify_between()
                            .text_color(colors.text_muted())
                            .child(plural(container.decisions, "decisão", "decisões"))
                            .child(format!("→ {calls}  ← {called}")),
                    ),
            );
        self.pressable(card, &id, move |this, cx| this.open_part(at, cx), cx)
    }

    /// The modal layer, when one is open.
    pub(super) fn render_modal(
        &mut self,
        theme: &Theme,
        flows: &[OverviewFlow],
        architecture: &Architecture,
        window: Size<gpui::Pixels>,
        cx: &mut gpui::Context<Self>,
    ) -> Option<AnyElement> {
        let modal = self.modal?;
        let max_height = (f32::from(window.height) - MODAL_MARGIN).max(320.0);
        match modal {
            Modal::Part(at) if at < architecture.containers.len() => {
                Some(self.part_modal(theme, flows, architecture, at, max_height, cx))
            }
            Modal::Explorer if !architecture.is_empty() => {
                Some(self.explorer_modal(theme, flows, architecture, max_height, cx))
            }
            _ => {
                self.modal = None;
                None
            }
        }
    }

    /// The frame every modal shares: scrim, panel, header with the close
    /// button, a scrolling body and an optional footer. Escape or a click on
    /// the scrim closes it.
    #[allow(clippy::too_many_arguments)]
    fn modal_shell(
        &mut self,
        theme: &Theme,
        key: &str,
        title: Div,
        middle: Option<AnyElement>,
        subheader: Option<AnyElement>,
        width: f32,
        max_height: f32,
        body: impl IntoElement,
        footer: Option<Div>,
        cx: &mut gpui::Context<Self>,
    ) -> AnyElement {
        let colors = theme.colors;
        let focus = self.focus_for("overview-modal", cx);
        let close = self.pressable(
            icon_action(theme, "overview-modal-close", "Fechar").child(icon(
                IconName::Close,
                14.0,
                colors.text_secondary(),
            )),
            "overview-modal-close",
            |this, cx| this.close_modal(cx),
            cx,
        );
        let header = div()
            .flex_none()
            .flex()
            .items_center()
            .gap(px(SpacingScale::S4))
            .px(px(SpacingScale::S5))
            .py(px(SpacingScale::S4))
            .border_b_1()
            .border_color(colors.hairline_divider())
            .child(title.flex_1().min_w(px(0.0)))
            .children(middle)
            .child(close);
        let panel = div()
            .id("overview-modal")
            .track_focus(&focus)
            .w(px(width))
            .max_w(gpui::relative(0.96))
            .max_h(px(max_height))
            .flex()
            .flex_col()
            .overflow_hidden()
            .rounded(theme.radius.dialog())
            .border_1()
            .border_color(colors.hairline_divider())
            .bg(colors.floating())
            .shadow(elevation(theme, Elevation::Dialog))
            .occlude()
            .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, _, cx| {
                if event.keystroke.key == "escape" {
                    this.close_modal(cx);
                    cx.stop_propagation();
                }
            }))
            .child(header)
            .children(subheader)
            .child(
                div()
                    .id(SharedString::from(format!("overview-modal-body-{key}")))
                    .flex_1()
                    .min_h(px(0.0))
                    .overflow_y_scroll()
                    .child(body),
            )
            .children(footer);
        deferred(
            div()
                .id("overview-modal-scrim")
                .absolute()
                .inset_0()
                .flex()
                .items_center()
                .justify_center()
                .bg(colors.scrim())
                .occlude()
                .on_mouse_down(
                    gpui::MouseButton::Left,
                    cx.listener(|this, _, _, cx| this.close_modal(cx)),
                )
                .child(panel_in(
                    SharedString::from(format!("overview-modal-in-{key}")),
                    panel,
                )),
        )
        .with_priority(3)
        .into_any_element()
    }

    /// A part: what it is, who it talks to, the flows through it and its
    /// weight. One primary action: open it in the Mapa.
    fn part_modal(
        &mut self,
        theme: &Theme,
        flows: &[OverviewFlow],
        architecture: &Architecture,
        at: usize,
        max_height: f32,
        cx: &mut gpui::Context<Self>,
    ) -> AnyElement {
        let colors = theme.colors;
        let container = architecture.containers[at].clone();
        let heaviest = architecture
            .containers
            .iter()
            .map(|container| container.decisions)
            .max()
            .unwrap_or(0);
        let title = div()
            .flex()
            .flex_col()
            .gap(px(2.0))
            .child(text_style(div(), TypeScale::HEADING_2).child(container.name.clone()))
            .child(
                text_style(div(), TypeScale::BODY_SMALL)
                    .text_color(colors.text_secondary())
                    .child(if container.role.is_empty() {
                        "Sem descrição no Mapa.".to_owned()
                    } else {
                        container.role.clone()
                    }),
            );

        let relation = |this: &mut Self,
                        label: &'static str,
                        empty: &'static str,
                        side: &str,
                        neighbours: Vec<Neighbour>,
                        cx: &mut gpui::Context<Self>| {
            let mut list = div().flex().flex_col().gap(px(SpacingScale::S1));
            if neighbours.is_empty() {
                list = list.child(
                    text_style(div(), TypeScale::BODY_SMALL)
                        .py(px(SpacingScale::S2))
                        .text_color(colors.text_muted())
                        .child(empty),
                );
            }
            for (order, neighbour) in neighbours.iter().enumerate() {
                let other = &architecture.containers[neighbour.index];
                let to = neighbour.index;
                let row_id = format!("overview-{side}-{at}-{order}");
                let row = div()
                    .id(SharedString::from(row_id.clone()))
                    .flex()
                    .items_center()
                    .gap(px(SpacingScale::S2))
                    .px(px(SpacingScale::S3))
                    .py(px(SpacingScale::S2))
                    .rounded(theme.radius.control())
                    .border_1()
                    .border_color(colors.hairline_divider())
                    .cursor_pointer()
                    .hover(move |style| style.bg(colors.glass_fill_medium()))
                    .role(Role::Button)
                    .aria_label(format!(
                        "{}: {}. Abrir.",
                        other.name,
                        plural(neighbour.steps.len(), "passo", "passos")
                    ))
                    .focus_visible(focus_ring(theme))
                    .child(
                        text_style(div(), TypeScale::BODY_SMALL)
                            .flex_1()
                            .min_w(px(0.0))
                            .truncate()
                            .text_color(colors.text_primary())
                            .child(other.name.clone()),
                    )
                    .children(neighbour.via().map(|via| {
                        text_style(div(), TypeScale::META)
                            .flex_none()
                            .font_family(Theme::font_mono())
                            .text_color(colors.graph_technology())
                            .child(clipped(via, 16))
                    }))
                    .child(
                        text_style(div(), TypeScale::META)
                            .flex_none()
                            .text_color(colors.text_muted())
                            .child(neighbour.steps.len().to_string()),
                    );
                list = list.child(this.pressable(
                    row,
                    &row_id,
                    move |this, cx| this.open_part(to, cx),
                    cx,
                ));
            }
            div()
                .flex_1()
                .min_w(px(0.0))
                .flex()
                .flex_col()
                .gap(px(SpacingScale::S2))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(SpacingScale::S2))
                        .child(crate::ui::patterns::section_label(theme, label))
                        .when(!neighbours.is_empty(), |header| {
                            header.child(count_chip(theme, neighbours.len().to_string()))
                        }),
                )
                .child(list)
        };
        let called_by = relation(
            self,
            "Quem chama",
            "Nenhuma parte a chama.",
            "by",
            architecture.called_by(at),
            cx,
        );
        let calls = relation(
            self,
            "Quem ela chama",
            "Ela não chama nenhuma parte.",
            "to",
            architecture.calls(at),
            cx,
        );

        // The flows that pass through it.
        let mut passing = div().flex().flex_wrap().gap(px(SpacingScale::S2));
        for flow_at in container.flows.iter().copied() {
            let Some(flow) = flows.get(flow_at) else {
                continue;
            };
            let chip_id = format!("overview-part-flow-{at}-{flow_at}");
            let chip = div()
                .id(SharedString::from(chip_id.clone()))
                .flex()
                .items_center()
                .gap(px(SpacingScale::S2))
                .px(px(SpacingScale::S3))
                .py(px(SpacingScale::S1))
                .rounded_full()
                .border_1()
                .border_color(colors.hairline_divider())
                .cursor_pointer()
                .hover(move |style| style.bg(colors.glass_fill_medium()))
                .role(Role::Button)
                .aria_label(format!("Ver o fluxo {} no diagrama", flow.title))
                .focus_visible(focus_ring(theme))
                .child(
                    text_style(div(), TypeScale::META)
                        .font_family(Theme::font_mono())
                        .text_color(colors.text_muted())
                        .child(roman(flow_at + 1)),
                )
                .child(
                    text_style(div(), TypeScale::BODY_SMALL)
                        .text_color(colors.text_secondary())
                        .child(clipped(&flow.title, 36)),
                );
            passing = passing.child(self.pressable(
                chip,
                &chip_id,
                move |this, cx| this.open_explorer(Some(flow_at), cx),
                cx,
            ));
        }

        // Weight, as shapes: decisions against the heaviest part, conflicts.
        let measure = |label: &'static str, value: String, fraction: f32, color| {
            div()
                .flex()
                .items_center()
                .gap(px(SpacingScale::S3))
                .child(
                    text_style(div(), TypeScale::BODY_SMALL)
                        .w(px(130.0))
                        .flex_none()
                        .text_color(colors.text_secondary())
                        .child(label),
                )
                .child(div().flex_1().child(meter(theme, fraction, color)))
                .child(
                    text_style(div(), TypeScale::META)
                        .w(px(28.0))
                        .flex_none()
                        .text_color(colors.text_primary())
                        .child(value),
                )
        };
        let numbers = div()
            .flex()
            .flex_col()
            .gap(px(SpacingScale::S2))
            .child(measure(
                "Decisões em vigor",
                container.decisions.to_string(),
                share(container.decisions, heaviest),
                colors.accent_default(),
            ))
            .child(measure(
                "Partes dentro dela",
                container.parts.to_string(),
                share(
                    container.parts,
                    architecture
                        .containers
                        .iter()
                        .map(|other| other.parts)
                        .max()
                        .unwrap_or(0),
                ),
                colors.text_secondary(),
            ))
            .when(container.conflicts > 0, |column| {
                column.child(measure(
                    "Em conflito",
                    container.conflicts.to_string(),
                    share(container.conflicts, container.decisions.max(1)),
                    colors.status_warning(),
                ))
            });

        let technologies = (!container.technologies.is_empty()).then(|| {
            div().flex().flex_wrap().gap(px(SpacingScale::S2)).children(
                container.technologies.iter().map(|technology| {
                    text_style(div(), TypeScale::META)
                        .px(px(SpacingScale::S2))
                        .py(px(2.0))
                        .rounded(px(4.0))
                        .bg(tint(colors.graph_technology(), 0.12))
                        .font_family(Theme::font_mono())
                        .text_color(colors.graph_technology())
                        .child(technology.clone())
                }),
            )
        });
        let labelled = |label: &'static str, content: Div| {
            div()
                .flex()
                .flex_col()
                .gap(px(SpacingScale::S2))
                .child(crate::ui::patterns::section_label(theme, label))
                .child(content)
        };
        let body = div()
            .px(px(SpacingScale::S5))
            .py(px(SpacingScale::S4))
            .flex()
            .flex_col()
            .gap(px(SpacingScale::S6))
            .children(technologies)
            .child(
                div()
                    .flex()
                    .gap(px(SpacingScale::S5))
                    .child(called_by)
                    .child(calls),
            )
            .when(!container.flows.is_empty(), |column| {
                column.child(labelled("Fluxos que passam por ela", passing))
            })
            .child(labelled("Em números", numbers));

        let entity_id = container.entity_id.clone();
        let open_map = self.pressable(
            action_button(theme, "overview-part-map", ButtonKind::Primary, true)
                .aria_label("Abrir no Mapa")
                .child("Abrir no Mapa"),
            "overview-part-map",
            move |this, cx| {
                this.close_modal(cx);
                cx.emit(OpenEntity(entity_id.clone()));
            },
            cx,
        );
        let first_flow = container.flows.first().copied();
        let see_diagram = first_flow.map(|flow| {
            self.pressable(
                action_button(theme, "overview-part-diagram", ButtonKind::Secondary, true)
                    .aria_label("Ver no diagrama")
                    .child("Ver no diagrama"),
                "overview-part-diagram",
                move |this, cx| this.open_explorer(Some(flow), cx),
                cx,
            )
        });
        let back = self.back.map(|_| {
            self.pressable(
                action_button(theme, "overview-part-back", ButtonKind::Ghost, true)
                    .aria_label("Voltar")
                    .child(icon(IconName::ArrowLeft, 14.0, colors.text_secondary()))
                    .child("Voltar"),
                "overview-part-back",
                |this, cx| this.go_back(cx),
                cx,
            )
        });
        let footer = div()
            .flex_none()
            .flex()
            .items_center()
            .gap(px(SpacingScale::S2))
            .px(px(SpacingScale::S5))
            .py(px(SpacingScale::S3))
            .border_t_1()
            .border_color(colors.hairline_divider())
            .children(back)
            .child(div().flex_1())
            .children(see_diagram)
            .child(open_map);
        self.modal_shell(
            theme,
            &format!("part-{at}"),
            title,
            None,
            None,
            PART_WIDTH,
            max_height,
            body,
            Some(footer),
            cx,
        )
    }

    /// The explorer: the diagram of one flow, or the matrix of the system.
    fn explorer_modal(
        &mut self,
        theme: &Theme,
        flows: &[OverviewFlow],
        architecture: &Architecture,
        max_height: f32,
        cx: &mut gpui::Context<Self>,
    ) -> AnyElement {
        let colors = theme.colors;
        let flow = self.explorer.flow.min(flows.len().saturating_sub(1));
        self.explorer.flow = flow;
        let view = self.explorer.view;
        let links: usize = architecture
            .interactions
            .iter()
            .map(|interaction| interaction.steps.len())
            .sum();
        let title = div()
            .flex()
            .flex_col()
            .gap(px(2.0))
            .child(text_style(div(), TypeScale::HEADING_2).child("Arquitetura"))
            .child(
                text_style(div(), TypeScale::BODY_SMALL)
                    .text_color(colors.text_muted())
                    .child(format!(
                        "{} · {} entre elas",
                        plural(architecture.containers.len(), "parte", "partes"),
                        plural(links, "passo de fluxo", "passos de fluxo")
                    )),
            );
        let mut switch = segmented(theme)
            .id("overview-view-switch")
            .role(Role::RadioGroup);
        for (target, id, label) in [
            (View::Diagram, "overview-view-diagram", "Diagrama"),
            (View::Matrix, "overview-view-matrix", "Matriz"),
        ] {
            switch = switch.child(self.pressable(
                segment_label(theme, id, label, view == target),
                id,
                move |this, cx| {
                    this.explorer.view = target;
                    this.hover_step = None;
                    cx.notify();
                },
                cx,
            ));
        }

        let (subheader, body): (Option<AnyElement>, AnyElement) = match view {
            View::Diagram => {
                let mut chips = div()
                    .flex()
                    .flex_wrap()
                    .gap(px(SpacingScale::S2))
                    .px(px(SpacingScale::S5))
                    .py(px(SpacingScale::S3))
                    .border_b_1()
                    .border_color(colors.hairline_divider());
                for (at, item) in flows.iter().enumerate() {
                    let selected = at == flow;
                    let id = format!("overview-flow-chip-{at}");
                    let chip = text_style(div(), TypeScale::LABEL)
                        .id(SharedString::from(id.clone()))
                        .flex()
                        .items_center()
                        .gap(px(SpacingScale::S2))
                        .h(px(26.0))
                        .px(px(SpacingScale::S3))
                        .rounded_full()
                        .border_1()
                        .border_color(if selected {
                            colors.accent_default().alpha(0.6)
                        } else {
                            colors.hairline_divider()
                        })
                        .when(selected, |chip| chip.bg(colors.glass_fill_medium()))
                        .when(!selected, |chip| {
                            chip.hover(move |style| style.bg(colors.glass_fill_low()))
                        })
                        .text_color(if selected {
                            colors.text_primary()
                        } else {
                            colors.text_secondary()
                        })
                        .cursor_pointer()
                        .role(Role::RadioButton)
                        .aria_label(format!("Fluxo: {}", item.title))
                        .focus_visible(focus_ring(theme))
                        .child(
                            text_style(div(), TypeScale::META)
                                .font_family(Theme::font_mono())
                                .text_color(colors.text_muted())
                                .child(roman(at + 1)),
                        )
                        .child(clipped(&item.title, FLOW_CHIP_CHARS));
                    chips = chips.child(self.pressable(
                        chip,
                        &id,
                        move |this, cx| {
                            this.explorer.flow = at;
                            this.hover_step = None;
                            cx.notify();
                        },
                        cx,
                    ));
                }
                (
                    Some(chips.into_any_element()),
                    self.diagram_view(theme, flows, architecture, flow, cx)
                        .into_any_element(),
                )
            }
            View::Matrix => (
                None,
                self.matrix_view(theme, flows, architecture, cx)
                    .into_any_element(),
            ),
        };
        self.modal_shell(
            theme,
            &format!("explorer-{}", if view == View::Diagram { "d" } else { "m" }),
            title,
            Some(switch.into_any_element()),
            subheader,
            EXPLORER_WIDTH,
            max_height,
            body,
            None,
            cx,
        )
    }

    /// One flow drawn, and its steps beside it (as rows, so the diagram has a
    /// text equivalent and a step can light its arrow).
    fn diagram_view(
        &mut self,
        theme: &Theme,
        flows: &[OverviewFlow],
        architecture: &Architecture,
        flow: usize,
        cx: &mut gpui::Context<Self>,
    ) -> Div {
        let colors = theme.colors;
        let steps = architecture.of_flow(flow);
        let description = flows
            .get(flow)
            .map(|flow| flow.description.clone())
            .unwrap_or_default();
        if steps.is_empty() {
            return div()
                .px(px(SpacingScale::S5))
                .py(px(SpacingScale::S6))
                .child(
                text_style(div(), TypeScale::BODY)
                    .text_color(colors.text_muted())
                    .child(
                        "Este fluxo acontece dentro de uma só parte, ou passa por partes que o \
                         Mapa não reconhece. Não há setas entre partes para desenhar.",
                    ),
            );
        }
        let narrowed = architecture.narrowed_to(flow);
        let diagram = self.architecture_diagram(theme, &narrowed, Some(flow), self.hover_step, cx);
        let mut rows = div().flex().flex_col();
        let count = steps.len();
        for (order, (interaction, step)) in steps.into_iter().enumerate() {
            let from = architecture
                .container(&interaction.from)
                .map(|container| container.name.clone())
                .unwrap_or_default();
            let to = architecture
                .container(&interaction.to)
                .map(|container| container.name.clone())
                .unwrap_or_default();
            rows = rows.child(self.step_row(theme, order, count, step, &from, &to));
        }
        div()
            .px(px(SpacingScale::S5))
            .py(px(SpacingScale::S4))
            .flex()
            .flex_col()
            .gap(px(SpacingScale::S5))
            .child(
                text_style(div(), TypeScale::BODY_SMALL)
                    .text_color(colors.text_muted())
                    .child(description),
            )
            .child(div().flex().justify_center().child(diagram))
            .child(rows)
    }

    /// A step between two parts: its number, what happens, from and to.
    fn step_row(
        &mut self,
        theme: &Theme,
        order: usize,
        count: usize,
        step: &StepRef,
        from: &str,
        to: &str,
    ) -> Stateful<Div> {
        let colors = theme.colors;
        let position = step.step;
        div()
            .id(SharedString::from(format!("overview-xstep-{order}")))
            .flex()
            .items_center()
            .gap(px(SpacingScale::S3))
            .py(px(SpacingScale::S2))
            .when(order + 1 < count, |row| {
                row.border_b_1().border_color(colors.hairline_divider())
            })
            .text_color(colors.text_secondary())
            .child(
                text_style(div(), TypeScale::META)
                    .w(px(28.0))
                    .flex_none()
                    .font_family(Theme::font_mono())
                    .text_color(colors.text_muted())
                    .child(roman(position + 1)),
            )
            .child(
                text_style(div(), TypeScale::BODY_SMALL)
                    .flex_1()
                    .min_w(px(0.0))
                    .truncate()
                    .text_color(colors.text_primary())
                    .child(step.title.clone()),
            )
            .children(step.via.clone().map(|via| {
                text_style(div(), TypeScale::META)
                    .flex_none()
                    .font_family(Theme::font_mono())
                    .text_color(colors.graph_technology())
                    .child(clipped(&via, 18))
            }))
            .child(
                text_style(div(), TypeScale::META)
                    .flex_none()
                    .text_color(colors.text_muted())
                    .child(format!("{from} → {to}")),
            )
            .hover(move |style| style.bg(colors.glass_fill_low()))
    }

    /// Who calls whom: rows are callers, columns are what they call, each
    /// filled cell the number of flow steps between the two.
    fn matrix_view(
        &mut self,
        theme: &Theme,
        flows: &[OverviewFlow],
        architecture: &Architecture,
        cx: &mut gpui::Context<Self>,
    ) -> Div {
        let colors = theme.colors;
        let count = architecture.containers.len();
        let picked = self.explorer.cell;
        let mut header = div().flex().items_center().child(div().w(px(MATRIX_LABEL)));
        for column in 0..count {
            let lit = picked.is_some_and(|(_, to)| to == column);
            header = header.child(
                text_style(div(), TypeScale::META)
                    .w(px(MATRIX_CELL))
                    .h(px(24.0))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .font_family(Theme::font_mono())
                    .text_color(if lit {
                        colors.text_primary()
                    } else {
                        colors.text_muted()
                    })
                    .child((column + 1).to_string()),
            );
        }
        let mut grid = div().flex().flex_col().child(header);
        for row in 0..count {
            let name = architecture.containers[row].name.clone();
            let lit_row = picked.is_some_and(|(from, _)| from == row);
            let mut line = div()
                .flex()
                .items_center()
                .h(px(MATRIX_CELL - 6.0))
                .when(lit_row, |line| line.bg(colors.glass_fill_low()))
                .child(
                    div()
                        .w(px(MATRIX_LABEL))
                        .flex_none()
                        .flex()
                        .items_center()
                        .gap(px(SpacingScale::S2))
                        .child(
                            text_style(div(), TypeScale::META)
                                .w(px(18.0))
                                .flex_none()
                                .font_family(Theme::font_mono())
                                .text_color(colors.text_muted())
                                .child((row + 1).to_string()),
                        )
                        .child(
                            text_style(div(), TypeScale::BODY_SMALL)
                                .min_w(px(0.0))
                                .truncate()
                                .text_color(if lit_row {
                                    colors.text_primary()
                                } else {
                                    colors.text_secondary()
                                })
                                .child(name),
                        ),
                );
            for column in 0..count {
                let steps = architecture
                    .interaction(row, column)
                    .map(|interaction| interaction.steps.len())
                    .unwrap_or(0);
                let is_picked = picked == Some((row, column));
                let id = format!("overview-cell-{row}-{column}");
                let cell = div()
                    .id(SharedString::from(id.clone()))
                    .w(px(MATRIX_CELL))
                    .h_full()
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .when(picked.is_some_and(|(_, to)| to == column), |cell| {
                        cell.bg(colors.glass_fill_low())
                    })
                    .child(if steps > 0 {
                        text_style(div(), TypeScale::META)
                            .size(px(26.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(px(6.0))
                            .border_1()
                            .border_color(colors.accent_default().alpha(if is_picked {
                                1.0
                            } else {
                                0.45
                            }))
                            .bg(tint(
                                colors.accent_default(),
                                if is_picked { 0.34 } else { 0.18 },
                            ))
                            .font_family(Theme::font_mono())
                            .text_color(colors.text_primary())
                            .child(steps.to_string())
                            .into_any_element()
                    } else if row == column {
                        div()
                            .w(px(10.0))
                            .h(px(1.0))
                            .bg(colors.hairline_divider())
                            .into_any_element()
                    } else {
                        div()
                            .size(px(3.0))
                            .rounded_full()
                            .bg(colors.hairline_divider())
                            .into_any_element()
                    });
                let cell = if steps > 0 {
                    let from = architecture.containers[row].name.clone();
                    let to = architecture.containers[column].name.clone();
                    let cell = cell
                        .cursor_pointer()
                        .role(Role::Button)
                        .aria_label(format!(
                            "{from} chama {to}: {}",
                            plural(steps, "passo", "passos")
                        ))
                        .focus_visible(focus_ring(theme))
                        .on_hover(cx.listener(move |this, hovered: &bool, _, cx| {
                            if *hovered && this.explorer.cell != Some((row, column)) {
                                this.explorer.cell = Some((row, column));
                                cx.notify();
                            }
                        }));
                    self.pressable(
                        cell,
                        &id,
                        move |this, cx| {
                            this.explorer.cell = Some((row, column));
                            cx.notify();
                        },
                        cx,
                    )
                } else {
                    cell
                };
                line = line.child(cell);
            }
            grid = grid.child(line);
        }

        // What the picked cell holds.
        let detail = match picked.and_then(|(from, to)| {
            architecture
                .interaction(from, to)
                .map(|interaction| (from, to, interaction))
        }) {
            Some((from, to, interaction)) => {
                let mut list = div().flex().flex_col().gap(px(SpacingScale::S2));
                for step in &interaction.steps {
                    list = list.child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(SpacingScale::S3))
                            .child(
                                text_style(div(), TypeScale::META)
                                    .w(px(28.0))
                                    .flex_none()
                                    .font_family(Theme::font_mono())
                                    .text_color(colors.text_muted())
                                    .child(roman(step.step + 1)),
                            )
                            .child(
                                text_style(div(), TypeScale::BODY_SMALL)
                                    .flex_1()
                                    .min_w(px(0.0))
                                    .truncate()
                                    .text_color(colors.text_primary())
                                    .child(step.title.clone()),
                            )
                            .children(step.via.clone().map(|via| {
                                text_style(div(), TypeScale::META)
                                    .flex_none()
                                    .font_family(Theme::font_mono())
                                    .text_color(colors.graph_technology())
                                    .child(clipped(&via, 18))
                            }))
                            .child(
                                text_style(div(), TypeScale::META)
                                    .flex_none()
                                    .text_color(colors.text_muted())
                                    .child(
                                        flows
                                            .get(step.flow)
                                            .map(|flow| clipped(&flow.title, 28))
                                            .unwrap_or_default(),
                                    ),
                            ),
                    );
                }
                div()
                    .flex()
                    .flex_col()
                    .gap(px(SpacingScale::S3))
                    .child(
                        text_style(div(), TypeScale::ROW_TITLE)
                            .text_color(colors.text_primary())
                            .child(format!(
                                "{} → {}",
                                architecture.containers[from].name,
                                architecture.containers[to].name
                            )),
                    )
                    .child(list)
            }
            None => div().child(
                text_style(div(), TypeScale::BODY_SMALL)
                    .text_color(colors.text_muted())
                    .child(
                        "Cada número é a quantidade de passos de fluxo que vão da parte da linha \
                         para a parte da coluna. Passe o mouse num número para ver quais.",
                    ),
            ),
        };
        div()
            .px(px(SpacingScale::S5))
            .py(px(SpacingScale::S4))
            .flex()
            .flex_col()
            .gap(px(SpacingScale::S5))
            .child(div().flex().justify_center().child(grid))
            .child(
                div()
                    .p(px(SpacingScale::S4))
                    .rounded(RadiusScale.surface())
                    .border_1()
                    .border_color(colors.hairline_divider())
                    .child(detail),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_matrix_fits_the_explorer_at_the_most_parts() {
        let widest = MATRIX_LABEL + application::architecture::MAX_CONTAINERS as f32 * MATRIX_CELL;
        assert!(widest + 2.0 * 20.0 <= EXPLORER_WIDTH, "{widest}");
    }

    #[test]
    fn six_cards_make_two_full_rows() {
        assert_eq!(SHOWN_AT_FIRST % CARDS_PER_ROW, 0);
    }
}
