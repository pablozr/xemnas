//! The architecture of the project as a diagram: its containers as boxes and
//! the steps of its flows as arrows between them (C4, level 2 and its dynamic
//! view). The model is derived in `application::architecture`; this file only
//! lays it out and draws it.
//!
//! Boxes are real elements (focusable, labelled, clickable) over a canvas that
//! paints the arrows, so the diagram is as reachable by keyboard and screen
//! reader as the list of flows beside it, which stays the textual equivalent.

use application::architecture::{Architecture, Container};
use gpui::prelude::*;
use gpui::{
    canvas, div, point, px, Bounds, Div, Hsla, PathBuilder, Pixels, Point, Role, SharedString,
    Window,
};

use super::{OpenEntity, OverviewScreen};
use crate::screens::format::{plural, roman};
use crate::ui::theme::{text_style, Theme};
use crate::ui::tokens::{RadiusScale, SpacingScale, TypeScale};
use crate::ui::tooltip::tooltip;

/// Width of the diagram: the reading column.
const WIDTH: f32 = 760.0;
/// Most columns (layers) the flows spread over; deeper chains fold into the
/// last column.
const MAX_COLUMNS: usize = 4;
/// Boxes per row in the grid of containers no flow touches.
const GRID_COLUMNS: usize = 4;
/// Height of a box.
const BOX_HEIGHT: f32 = 92.0;
/// Space between boxes of a column, and between the flow area and the grid.
const ROW_GAP: f32 = 28.0;
/// Narrowest and widest a box gets.
const MIN_WIDTH: f32 = 150.0;
const MAX_WIDTH: f32 = 224.0;
/// Side of a step badge.
const BADGE: f32 = 22.0;
/// Room above the boxes for an arrow that goes over the top of others.
const ARC_ROOM: f32 = 64.0;

/// Where the containers sit.
#[derive(Debug, Clone, PartialEq)]
pub struct Layout {
    /// Top-left of each container, in the order of [`Architecture::containers`].
    pub boxes: Vec<(f32, f32)>,
    /// Width of every box.
    pub width: f32,
    /// Total height.
    pub height: f32,
}

/// Lays the containers out in columns, left to right: what a flow starts in
/// on the left, where it ends on the right, so arrows mostly run sideways and
/// the picture reads like the flow does. Containers no flow touches wait in a
/// grid below. Deterministic: the same architecture always draws the same.
///
/// With `only_flow`, only that flow's interactions decide the order, so its
/// path reads left to right even when other flows run the other way; the
/// containers it does not touch wait in the grid below.
pub fn layout(architecture: &Architecture, only_flow: Option<usize>) -> Layout {
    let count = architecture.containers.len();
    let index = |id: &str| {
        architecture
            .containers
            .iter()
            .position(|container| container.entity_id == id)
    };
    let mut edges: Vec<(usize, usize)> = Vec::new();
    for interaction in architecture.interactions.iter().filter(|interaction| {
        only_flow.is_none_or(|at| interaction.steps.iter().any(|step| step.flow == at))
    }) {
        if let (Some(from), Some(to)) = (index(&interaction.from), index(&interaction.to)) {
            if from != to && !edges.contains(&(from, to)) {
                edges.push((from, to));
            }
        }
    }
    // Layer = longest path from a source, over the edges that do not close a
    // cycle (found by a depth-first walk in container order).
    fn walk(
        at: usize,
        edges: &[(usize, usize)],
        state: &mut [u8],
        forward: &mut Vec<(usize, usize)>,
    ) {
        state[at] = 1;
        for &(from, to) in edges.iter().filter(|(from, _)| *from == at) {
            match state[to] {
                0 => {
                    forward.push((from, to));
                    walk(to, edges, state, forward);
                }
                2 => forward.push((from, to)),
                _ => {}
            }
        }
        state[at] = 2;
    }
    let mut state = vec![0_u8; count];
    let mut forward: Vec<(usize, usize)> = Vec::new();
    let sources: Vec<usize> = (0..count)
        .filter(|node| !edges.iter().any(|(_, to)| to == node))
        .chain(0..count)
        .collect();
    for start in sources {
        if state[start] == 0 {
            walk(start, &edges, &mut state, &mut forward);
        }
    }
    let mut layer = vec![0_usize; count];
    for _ in 0..count {
        let mut changed = false;
        for &(from, to) in &forward {
            if layer[to] < layer[from] + 1 {
                layer[to] = layer[from] + 1;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    let linked: Vec<bool> = (0..count)
        .map(|node| edges.iter().any(|(from, to)| *from == node || *to == node))
        .collect();
    // Columns: the layers in use, folded at MAX_COLUMNS.
    let mut used: Vec<usize> = (0..count)
        .filter(|node| linked[*node])
        .map(|node| layer[node])
        .collect();
    used.sort_unstable();
    used.dedup();
    let column_of = |node: usize| {
        used.iter()
            .position(|at| *at == layer[node])
            .unwrap_or(0)
            .min(MAX_COLUMNS - 1)
    };
    let columns = used.len().clamp(1, MAX_COLUMNS);
    let gap = if columns <= 3 { 56.0 } else { 40.0 };
    let width =
        ((WIDTH - gap * (columns as f32 - 1.0)) / columns as f32).clamp(MIN_WIDTH, MAX_WIDTH);
    let span = columns as f32 * width + (columns as f32 - 1.0) * gap;
    let start_x = (WIDTH - span) / 2.0;

    // Members of each column, ordered by where what feeds them sits.
    let mut members: Vec<Vec<usize>> = vec![Vec::new(); columns];
    for node in (0..count).filter(|node| linked[*node]) {
        members[column_of(node)].push(node);
    }
    let mut rank = vec![0.0_f32; count];
    for column in &mut members {
        let feed = |node: usize, rank: &[f32]| {
            let from: Vec<f32> = edges
                .iter()
                .filter(|(_, to)| *to == node)
                .map(|(from, _)| rank[*from])
                .collect();
            if from.is_empty() {
                node as f32
            } else {
                from.iter().sum::<f32>() / from.len() as f32
            }
        };
        let snapshot = rank.clone();
        column.sort_by(|a, b| {
            feed(*a, &snapshot)
                .partial_cmp(&feed(*b, &snapshot))
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(a.cmp(b))
        });
        for (at, node) in column.iter().enumerate() {
            rank[*node] = at as f32;
        }
    }
    let tallest = members.iter().map(Vec::len).max().unwrap_or(0);
    let column_height = |rows: usize| {
        if rows == 0 {
            0.0
        } else {
            rows as f32 * BOX_HEIGHT + (rows as f32 - 1.0) * ROW_GAP
        }
    };
    // An arrow that skips a column goes over the top of what is between; the
    // room for the arc is reserved above the boxes.
    let skips = edges.iter().any(|(from, to)| {
        linked[*from] && linked[*to] && column_of(*from).abs_diff(column_of(*to)) >= 2
    });
    let arc_room = if skips { ARC_ROOM } else { 0.0 };
    let flow_height = column_height(tallest);
    let mut boxes = vec![(0.0, 0.0); count];
    for (column_at, column) in members.iter().enumerate() {
        // Short columns sit centred against the tallest one.
        let offset = (flow_height - column_height(column.len())) / 2.0;
        for (row_at, node) in column.iter().enumerate() {
            boxes[*node] = (
                start_x + column_at as f32 * (width + gap),
                arc_room + offset + row_at as f32 * (BOX_HEIGHT + ROW_GAP),
            );
        }
    }
    // The grid of containers no flow touches, below.
    let loose: Vec<usize> = (0..count).filter(|node| !linked[*node]).collect();
    let mut height = flow_height;
    if !loose.is_empty() {
        let top = if flow_height > 0.0 {
            arc_room + flow_height + ROW_GAP * 1.5
        } else {
            0.0
        };
        for (at, node) in loose.iter().enumerate() {
            let (row, col) = (at / GRID_COLUMNS, at % GRID_COLUMNS);
            let in_row = (loose.len() - row * GRID_COLUMNS).min(GRID_COLUMNS);
            let used = in_row as f32 * width + (in_row as f32 - 1.0) * 40.0;
            boxes[*node] = (
                (WIDTH - used) / 2.0 + col as f32 * (width + 40.0),
                top + row as f32 * (BOX_HEIGHT + ROW_GAP),
            );
        }
        height = top + column_height(loose.len().div_ceil(GRID_COLUMNS));
    } else {
        height += arc_room;
    }
    Layout {
        boxes,
        width,
        height,
    }
}

/// Where an arrow leaves a box and enters another: the facing sides.
fn anchors(from: (f32, f32), to: (f32, f32), width: f32, shift: f32) -> (Point<f32>, Point<f32>) {
    let centre = |top_left: (f32, f32)| (top_left.0 + width / 2.0, top_left.1 + BOX_HEIGHT / 2.0);
    let (a, b) = (centre(from), centre(to));
    if (b.0 - a.0).abs() > width / 2.0 {
        let right = b.0 > a.0;
        let start_x = if right { from.0 + width } else { from.0 };
        let end_x = if right { to.0 } else { to.0 + width };
        (point(start_x, a.1 + shift), point(end_x, b.1 + shift))
    } else {
        let down = b.1 > a.1;
        let start_y = if down { from.1 + BOX_HEIGHT } else { from.1 };
        let end_y = if down { to.1 } else { to.1 + BOX_HEIGHT };
        (point(a.0 + shift, start_y), point(b.0 + shift, end_y))
    }
}

/// A point of the arrow's curve (`bend` 0 is a straight line).
fn quad(start: Point<f32>, end: Point<f32>, bend: f32, t: f32) -> Point<f32> {
    let control = point(
        (start.x + end.x) / 2.0 - (end.y - start.y) * bend,
        (start.y + end.y) / 2.0 + (end.x - start.x) * bend,
    );
    let u = 1.0 - t;
    point(
        u * u * start.x + 2.0 * u * t * control.x + t * t * end.x,
        u * u * start.y + 2.0 * u * t * control.y + t * t * end.y,
    )
}

/// The first bend (straight, then to either side) whose curve does not run
/// through a box other than the two it joins.
fn clear_bend(
    start: Point<f32>,
    end: Point<f32>,
    others: &[(f32, f32)],
    width: f32,
) -> Option<f32> {
    for bend in [0.0, 0.2, -0.2, 0.38, -0.38] {
        let clear = (1..12).all(|step| {
            let at = quad(start, end, bend, step as f32 / 12.0);
            !others.iter().any(|(x, y)| {
                at.x > x - 4.0
                    && at.x < x + width + 4.0
                    && at.y > y - 4.0
                    && at.y < y + BOX_HEIGHT + 4.0
            })
        });
        if clear {
            return Some(bend);
        }
    }
    None
}

/// One arrow to paint, in the diagram's own coordinates.
struct Arrow {
    start: Point<f32>,
    end: Point<f32>,
    bend: f32,
    colour: Hsla,
    width: f32,
}

fn paint_arrows(bounds: Bounds<Pixels>, arrows: &[Arrow], window: &mut Window) {
    let (ox, oy) = (f32::from(bounds.origin.x), f32::from(bounds.origin.y));
    let at = |p: Point<f32>| point(px(ox + p.x), px(oy + p.y));
    for arrow in arrows {
        let mut line = PathBuilder::stroke(px(arrow.width));
        line.move_to(at(arrow.start));
        let steps = 14;
        for step in 1..=steps {
            line.line_to(at(quad(
                arrow.start,
                arrow.end,
                arrow.bend,
                step as f32 / steps as f32,
            )));
        }
        if let Ok(path) = line.build() {
            window.paint_path(path, arrow.colour);
        }
        // The head: a small triangle on the line's last direction.
        let before = quad(arrow.start, arrow.end, arrow.bend, 0.94);
        let (dx, dy) = (arrow.end.x - before.x, arrow.end.y - before.y);
        let length = (dx * dx + dy * dy).sqrt().max(1.0);
        let (ux, uy) = (dx / length, dy / length);
        let size = 5.0 + arrow.width;
        let base = point(arrow.end.x - ux * size, arrow.end.y - uy * size);
        let wing = |sign: f32| {
            point(
                base.x - uy * size * 0.55 * sign,
                base.y + ux * size * 0.55 * sign,
            )
        };
        let mut head = PathBuilder::fill();
        head.add_polygon(&[at(arrow.end), at(wing(1.0)), at(wing(-1.0))], true);
        if let Ok(path) = head.build() {
            window.paint_path(path, arrow.colour);
        }
    }
}

impl OverviewScreen {
    /// The diagram of the project's containers. With `flow`, that flow's steps
    /// are drawn as numbered arrows and the containers it does not touch are
    /// dimmed; `emphasis` is a step (by position) to highlight further.
    pub(super) fn architecture_diagram(
        &mut self,
        theme: &Theme,
        architecture: &Architecture,
        flow: Option<usize>,
        emphasis: Option<usize>,
        cx: &mut gpui::Context<Self>,
    ) -> Div {
        let colors = theme.colors;
        let laid = layout(architecture, flow);
        let width = laid.width;
        let in_flow = |container: &Container| flow.is_none_or(|at| container.flows.contains(&at));

        // Arrows: the flow's steps strong, the others a quiet hairline.
        let index = |id: &str| {
            architecture
                .containers
                .iter()
                .position(|container| container.entity_id == id)
        };
        let mut arrows: Vec<Arrow> = Vec::new();
        let mut badges: Vec<(Point<f32>, usize)> = Vec::new();
        let muted: Hsla = Hsla::from(colors.text_muted());
        let accent: Hsla = Hsla::from(colors.accent_default());
        for interaction in &architecture.interactions {
            let (Some(from), Some(to)) = (index(&interaction.from), index(&interaction.to)) else {
                continue;
            };
            // The reverse arrow of a pair runs beside this one.
            let shift = if architecture
                .interactions
                .iter()
                .any(|other| other.from == interaction.to && other.to == interaction.from)
                && interaction.from > interaction.to
            {
                12.0
            } else {
                0.0
            };
            let (mut start, mut end) = anchors(laid.boxes[from], laid.boxes[to], width, shift);
            let others: Vec<(f32, f32)> = laid
                .boxes
                .iter()
                .enumerate()
                .filter(|(at, _)| *at != from && *at != to)
                .map(|(_, top_left)| *top_left)
                .collect();
            let bend = clear_bend(start, end, &others, width).unwrap_or_else(|| {
                // Over the top: from the top of one box to the top of the
                // other, in the room the layout kept above the boxes.
                let centre = |top_left: (f32, f32)| point(top_left.0 + width / 2.0, top_left.1);
                start = centre(laid.boxes[from]);
                end = centre(laid.boxes[to]);
                if end.x > start.x {
                    -0.26
                } else {
                    0.26
                }
            });
            let steps: Vec<_> = flow
                .map(|at| {
                    interaction
                        .steps
                        .iter()
                        .filter(|step| step.flow == at)
                        .collect()
                })
                .unwrap_or_default();
            if steps.is_empty() {
                arrows.push(Arrow {
                    start,
                    end,
                    bend,
                    colour: muted.opacity(if flow.is_some() { 0.18 } else { 0.7 }),
                    width: 1.2,
                });
            } else {
                let hot = steps.iter().any(|step| Some(step.step) == emphasis);
                arrows.push(Arrow {
                    start,
                    end,
                    bend,
                    colour: accent.opacity(if emphasis.is_none() || hot { 1.0 } else { 0.4 }),
                    width: if hot { 2.6 } else { 1.8 },
                });
                let middle = quad(start, end, bend, 0.5);
                for (order, step) in steps.iter().enumerate() {
                    // Several steps on one arrow stack their badges.
                    badges.push((
                        point(
                            middle.x,
                            middle.y + order as f32 * (BADGE + 2.0)
                                - (steps.len() as f32 - 1.0) * (BADGE + 2.0) / 2.0,
                        ),
                        step.step,
                    ));
                }
            }
        }
        let paint = canvas(
            |_, _, _| (),
            move |bounds, _, window, _| paint_arrows(bounds, &arrows, window),
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full();

        let mut diagram = div()
            .relative()
            .w(px(WIDTH))
            .max_w_full()
            .h(px(laid.height))
            .child(paint);

        for (at, container) in architecture.containers.iter().enumerate() {
            let (x, y) = laid.boxes[at];
            let touched = in_flow(container);
            let id = container.entity_id.clone();
            let element_id = format!("overview-box-{at}");
            let facts = {
                let mut facts = vec![plural(container.decisions, "decisão", "decisões")];
                if container.parts > 0 {
                    facts.push(plural(container.parts, "parte", "partes"));
                }
                facts.join(" · ")
            };
            let label = format!("{}: {} Abrir no Mapa.", container.name, facts);
            let mut content = div()
                .flex()
                .flex_col()
                .gap(px(2.0))
                .child(
                    text_style(div(), TypeScale::ROW_TITLE)
                        .truncate()
                        .text_color(colors.text_primary())
                        .child(container.name.clone()),
                )
                .child(
                    text_style(div(), TypeScale::META)
                        .truncate()
                        .text_color(colors.text_muted())
                        .child(if container.role.is_empty() {
                            facts.clone()
                        } else {
                            container.role.clone()
                        }),
                );
            if !container.technologies.is_empty() {
                content = content.child(
                    text_style(div(), TypeScale::META)
                        .truncate()
                        .font_family(Theme::font_mono())
                        .text_color(colors.graph_technology())
                        .child(container.technologies.join(" · ")),
                );
            }
            let boxed = div()
                .id(SharedString::from(element_id.clone()))
                .absolute()
                .left(px(x))
                .top(px(y))
                .w(px(width))
                .h(px(BOX_HEIGHT))
                .p(px(SpacingScale::S3))
                .flex()
                .flex_col()
                .justify_between()
                .rounded(RadiusScale.surface())
                .border_1()
                .border_color(if flow.is_some() && touched {
                    colors.accent_default().alpha(0.7)
                } else {
                    colors.glass_border_card()
                })
                .bg(colors.glass_fill_card())
                .when(!touched, |boxed| boxed.opacity(0.45))
                .cursor_pointer()
                .hover(move |style| style.border_color(colors.glass_border_card_hover()))
                .role(Role::Link)
                .aria_label(label)
                .tooltip(tooltip(format!("{} · abrir no Mapa", container.name), None))
                .focus_visible(crate::ui::controls::focus_ring(theme))
                .child(content)
                .child(
                    text_style(div(), TypeScale::META)
                        .text_color(colors.text_muted())
                        .child(facts),
                );
            diagram = diagram.child(self.pressable(
                boxed,
                &element_id,
                move |_, cx| cx.emit(OpenEntity(id.clone())),
                cx,
            ));
        }
        for (centre, step) in badges {
            diagram = diagram.child(
                div()
                    .absolute()
                    .left(px(centre.x - BADGE / 2.0))
                    .top(px(centre.y - BADGE / 2.0))
                    .min_w(px(BADGE))
                    .h(px(BADGE))
                    .px(px(4.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_full()
                    .border_1()
                    .border_color(colors.accent_default())
                    .bg(colors.canvas_raised())
                    .child(
                        text_style(div(), TypeScale::META)
                            .font_family(Theme::font_mono())
                            .text_color(colors.text_primary())
                            .child(roman(step + 1)),
                    ),
            );
        }
        diagram
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use application::architecture::{Architecture, Container, Interaction, StepRef};

    fn container(id: &str) -> Container {
        Container {
            entity_id: id.into(),
            name: id.into(),
            ..Container::default()
        }
    }

    fn link(from: &str, to: &str) -> Interaction {
        Interaction {
            from: from.into(),
            to: to.into(),
            steps: vec![StepRef {
                flow: 0,
                step: 1,
                title: "x".into(),
            }],
        }
    }

    #[test]
    fn what_feeds_another_sits_above_it() {
        let architecture = Architecture {
            containers: vec![container("api"), container("jobs"), container("store")],
            interactions: vec![link("api", "jobs"), link("jobs", "store")],
            hidden: 0,
        };
        let laid = layout(&architecture, None);
        assert!(laid.boxes[0].0 < laid.boxes[1].0, "api left of jobs");
        assert!(laid.boxes[1].0 < laid.boxes[2].0, "jobs left of store");
        assert_eq!(
            laid.boxes[0].1, laid.boxes[2].1,
            "a chain reads on one line"
        );
        assert!(laid.height > 0.0);
    }

    #[test]
    fn the_order_of_the_containers_list_does_not_decide_the_order_on_screen() {
        let architecture = Architecture {
            containers: vec![container("store"), container("ai"), container("app")],
            interactions: vec![link("app", "ai"), link("ai", "store")],
            hidden: 0,
        };
        let laid = layout(&architecture, None);
        assert!(laid.boxes[2].0 < laid.boxes[1].0, "app left of ai");
        assert!(laid.boxes[1].0 < laid.boxes[0].0, "ai left of store");
    }

    #[test]
    fn a_cycle_does_not_hang_and_every_box_gets_a_place() {
        let architecture = Architecture {
            containers: vec![container("a"), container("b")],
            interactions: vec![link("a", "b"), link("b", "a")],
            hidden: 0,
        };
        let laid = layout(&architecture, None);
        assert_eq!(laid.boxes.len(), 2);
        assert!(laid.boxes[0] != laid.boxes[1]);
    }

    #[test]
    fn containers_without_flows_gather_below_and_wrap_in_rows() {
        let architecture = Architecture {
            containers: vec![
                container("a"),
                container("b"),
                container("c"),
                container("d"),
                container("e"),
            ],
            interactions: vec![link("a", "b")],
            hidden: 0,
        };
        let laid = layout(&architecture, None);
        // c, d, e share a row below the a to b chain.
        assert_eq!(laid.boxes[2].1, laid.boxes[3].1);
        assert_eq!(laid.boxes[3].1, laid.boxes[4].1);
        assert!(laid.boxes[2].1 > laid.boxes[1].1);
        for (x, _) in &laid.boxes {
            assert!(*x >= 0.0 && x + laid.width <= WIDTH + 0.5);
        }
    }

    #[test]
    fn an_empty_architecture_has_no_height() {
        assert_eq!(layout(&Architecture::default(), None).height, 0.0);
    }
}
