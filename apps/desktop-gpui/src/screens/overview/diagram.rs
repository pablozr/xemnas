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

use super::OverviewScreen;
use crate::screens::format::{plural, roman};
use crate::ui::theme::{text_style, Theme};
use crate::ui::tokens::{RadiusScale, SpacingScale, TypeScale};
use crate::ui::tooltip::tooltip;

/// Width of the diagram: the reading column.
const WIDTH: f32 = 760.0;
/// Free margin at each side, where an arrow between boxes of one column runs.
const MARGIN: f32 = 14.0;
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
/// Space between the lanes an arrow takes over the top of the boxes.
const LANE_STEP: f32 = 9.0;
/// Radius of the corners of an arrow.
const CORNER: f32 = 9.0;

/// Where the containers sit.
#[derive(Debug, Clone, PartialEq)]
pub struct Layout {
    /// Top-left of each container, in the order of [`Architecture::containers`].
    pub boxes: Vec<(f32, f32)>,
    /// Column of each container (`None` for those no flow touches, which wait
    /// in a grid below).
    pub column: Vec<Option<usize>>,
    /// Width of every box.
    pub width: f32,
    /// Space between columns.
    pub gap: f32,
    /// Left edge of the first column.
    pub start_x: f32,
    /// Room kept above the boxes for the lanes of arrows that go over them.
    pub arc_room: f32,
    /// Container pairs whose arrow skips a column or runs backwards, the ones
    /// that take a lane (the shortest first, the innermost).
    pub over: Vec<(usize, usize)>,
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
    let usable = WIDTH - 2.0 * MARGIN;
    let width =
        ((usable - gap * (columns as f32 - 1.0)) / columns as f32).clamp(MIN_WIDTH, MAX_WIDTH);
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
    // An arrow that skips a column, or runs backwards, goes over the top of
    // the boxes in a lane of its own; the room for the lanes is kept above.
    let mut over: Vec<(usize, usize)> = Vec::new();
    for interaction in architecture.interactions.iter().filter(|interaction| {
        only_flow.is_none_or(|at| interaction.steps.iter().any(|step| step.flow == at))
    }) {
        if let (Some(from), Some(to)) = (index(&interaction.from), index(&interaction.to)) {
            if from != to && linked[from] && linked[to] && !over.contains(&(from, to)) {
                let (a, b) = (column_of(from), column_of(to));
                if a != b && b != a + 1 {
                    over.push((from, to));
                }
            }
        }
    }
    over.sort_by_key(|(from, to)| (column_of(*from).abs_diff(column_of(*to)), *from, *to));
    let arc_room = if over.is_empty() {
        0.0
    } else {
        LANE_STEP * over.len() as f32 + 20.0
    };
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
        column: (0..count)
            .map(|node| linked[node].then(|| column_of(node)))
            .collect(),
        boxes,
        width,
        gap,
        start_x,
        arc_room,
        over,
        height,
    }
}

/// Where each interaction's arrow runs, as a polyline in the diagram's own
/// coordinates (`None` where an end has no place among the columns). The
/// routes are orthogonal: out of the side of a box, along the free channel
/// between columns, into the facing side of the next. An arrow that skips a
/// column or runs backwards leaves by the channel, goes over the boxes in its
/// own lane and comes down the channel before its target, so it never crosses
/// a box.
pub fn route(architecture: &Architecture, laid: &Layout) -> Vec<Option<Vec<Point<f32>>>> {
    let index = |id: &str| {
        architecture
            .containers
            .iter()
            .position(|container| container.entity_id == id)
    };
    let ends: Vec<Option<(usize, usize)>> = architecture
        .interactions
        .iter()
        .map(|interaction| {
            let (from, to) = (index(&interaction.from)?, index(&interaction.to)?);
            (from != to && laid.column[from].is_some() && laid.column[to].is_some())
                .then_some((from, to))
        })
        .collect();
    // The channels each arrow uses: (gap, pair, end of the arrow).
    let gaps = |from: usize, to: usize| -> [Option<usize>; 2] {
        let (a, b) = (laid.column[from].unwrap_or(0), laid.column[to].unwrap_or(0));
        if a == b {
            [None, None]
        } else if b == a + 1 {
            [Some(a), None]
        } else if b > a {
            [Some(a), Some(b - 1)]
        } else {
            [Some(a - 1), Some(b)]
        }
    };
    let mut uses: std::collections::BTreeMap<usize, Vec<(usize, usize, usize)>> =
        std::collections::BTreeMap::new();
    for (from, to) in ends.iter().flatten() {
        for (end, gap) in gaps(*from, *to).into_iter().enumerate() {
            if let Some(gap) = gap {
                let entry = uses.entry(gap).or_default();
                if !entry.contains(&(*from, *to, end)) {
                    entry.push((*from, *to, end));
                }
            }
        }
    }
    // The x of an arrow's channel in a gap: its slot among those sharing it.
    let channel = |gap: usize, key: (usize, usize, usize)| -> f32 {
        let all = &uses[&gap];
        let slot = all.iter().position(|other| *other == key).unwrap_or(0) as f32;
        let step = ((laid.gap - 14.0) / all.len() as f32).min(7.0);
        laid.start_x
            + gap as f32 * (laid.width + laid.gap)
            + laid.width
            + laid.gap / 2.0
            + (slot - (all.len() as f32 - 1.0) / 2.0) * step
    };
    let middle = |node: usize| laid.boxes[node].1 + BOX_HEIGHT / 2.0;
    architecture
        .interactions
        .iter()
        .enumerate()
        .map(|(at, interaction)| {
            let (from, to) = ends[at]?;
            // The reverse arrow of a pair runs beside this one.
            let shift = if architecture
                .interactions
                .iter()
                .any(|other| other.from == interaction.to && other.to == interaction.from)
                && from > to
            {
                8.0
            } else {
                0.0
            };
            let (a, b) = (laid.column[from]?, laid.column[to]?);
            let (bf, bt) = (laid.boxes[from], laid.boxes[to]);
            let (yf, yt) = (middle(from) + shift, middle(to) + shift);
            let w = laid.width;
            let out = (from, to, 0);
            let into = (from, to, 1);
            let lane = laid.over.iter().position(|pair| *pair == (from, to));
            let lane_y = |lane: usize| laid.arc_room - 12.0 - lane as f32 * LANE_STEP;
            Some(if a == b {
                // Same column: down the free margin beside the boxes.
                let x = bf.0.max(bt.0) + w + 9.0 + shift;
                vec![
                    point(bf.0 + w, yf),
                    point(x, yf),
                    point(x, yt),
                    point(bt.0 + w, yt),
                ]
            } else if b == a + 1 {
                let x = channel(a, out);
                if (yf - yt).abs() < 1.0 {
                    vec![point(bf.0 + w, yf), point(bt.0, yt)]
                } else {
                    vec![
                        point(bf.0 + w, yf),
                        point(x, yf),
                        point(x, yt),
                        point(bt.0, yt),
                    ]
                }
            } else {
                let y = lane_y(lane?);
                if b > a {
                    let (x1, x2) = (channel(a, out), channel(b - 1, into));
                    vec![
                        point(bf.0 + w, yf),
                        point(x1, yf),
                        point(x1, y),
                        point(x2, y),
                        point(x2, yt),
                        point(bt.0, yt),
                    ]
                } else {
                    let (x1, x2) = (channel(a - 1, out), channel(b, into));
                    vec![
                        point(bf.0, yf),
                        point(x1, yf),
                        point(x1, y),
                        point(x2, y),
                        point(x2, yt),
                        point(bt.0 + w, yt),
                    ]
                }
            })
        })
        .collect()
}

/// The polyline with its corners rounded.
fn rounded(points: &[Point<f32>]) -> Vec<Point<f32>> {
    if points.len() < 3 {
        return points.to_vec();
    }
    let mut out = vec![points[0]];
    for corner in 1..points.len() - 1 {
        let (before, here, after) = (points[corner - 1], points[corner], points[corner + 1]);
        let toward = |from: Point<f32>, to: Point<f32>, reach: f32| {
            let (dx, dy) = (to.x - from.x, to.y - from.y);
            let length = (dx * dx + dy * dy).sqrt().max(0.001);
            (
                point(from.x + dx / length * reach, from.y + dy / length * reach),
                length,
            )
        };
        let reach = |other: Point<f32>| {
            let (dx, dy) = (other.x - here.x, other.y - here.y);
            ((dx * dx + dy * dy).sqrt() / 2.0).min(CORNER)
        };
        let radius = reach(before).min(reach(after));
        let (start, _) = toward(here, before, radius);
        let (end, _) = toward(here, after, radius);
        out.push(start);
        for step in 1..4 {
            let t = step as f32 / 4.0;
            let u = 1.0 - t;
            out.push(point(
                u * u * start.x + 2.0 * u * t * here.x + t * t * end.x,
                u * u * start.y + 2.0 * u * t * here.y + t * t * end.y,
            ));
        }
        out.push(end);
    }
    out.push(points[points.len() - 1]);
    out
}

/// The point halfway along a polyline.
fn halfway(points: &[Point<f32>]) -> Point<f32> {
    let lengths: Vec<f32> = points
        .windows(2)
        .map(|pair| ((pair[1].x - pair[0].x).powi(2) + (pair[1].y - pair[0].y).powi(2)).sqrt())
        .collect();
    let mut left = lengths.iter().sum::<f32>() / 2.0;
    for (pair, length) in points.windows(2).zip(&lengths) {
        if left <= *length && *length > 0.0 {
            let t = left / length;
            return point(
                pair[0].x + (pair[1].x - pair[0].x) * t,
                pair[0].y + (pair[1].y - pair[0].y) * t,
            );
        }
        left -= length;
    }
    points[0]
}

/// One arrow to paint, in the diagram's own coordinates.
struct Arrow {
    points: Vec<Point<f32>>,
    colour: Hsla,
    width: f32,
}

fn paint_arrows(bounds: Bounds<Pixels>, arrows: &[Arrow], window: &mut Window) {
    let (ox, oy) = (f32::from(bounds.origin.x), f32::from(bounds.origin.y));
    let at = |p: Point<f32>| point(px(ox + p.x), px(oy + p.y));
    for arrow in arrows {
        let Some((&tip, rest)) = arrow.points.split_last() else {
            continue;
        };
        let Some(&before) = rest.last() else {
            continue;
        };
        let mut line = PathBuilder::stroke(px(arrow.width));
        line.move_to(at(arrow.points[0]));
        for p in &arrow.points[1..] {
            line.line_to(at(*p));
        }
        if let Ok(path) = line.build() {
            window.paint_path(path, arrow.colour);
        }
        // The head: a small triangle on the line's last direction.
        let (dx, dy) = (tip.x - before.x, tip.y - before.y);
        let length = (dx * dx + dy * dy).sqrt().max(1.0);
        let (ux, uy) = (dx / length, dy / length);
        let size = 5.0 + arrow.width;
        let base = point(tip.x - ux * size, tip.y - uy * size);
        let wing = |sign: f32| {
            point(
                base.x - uy * size * 0.55 * sign,
                base.y + ux * size * 0.55 * sign,
            )
        };
        let mut head = PathBuilder::fill();
        head.add_polygon(&[at(tip), at(wing(1.0)), at(wing(-1.0))], true);
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
        let routes = route(architecture, &laid);
        let mut arrows: Vec<Arrow> = Vec::new();
        let mut badges: Vec<(Point<f32>, usize)> = Vec::new();
        let muted: Hsla = Hsla::from(colors.text_muted());
        let accent: Hsla = Hsla::from(colors.accent_default());
        let hovered = self.hover_box;
        for (interaction, path) in architecture.interactions.iter().zip(&routes) {
            let Some(path) = path else {
                continue;
            };
            let points = rounded(path);
            let touches = hovered.is_some_and(|at| {
                [&interaction.from, &interaction.to].iter().any(|id| {
                    architecture
                        .containers
                        .get(at)
                        .is_some_and(|c| &c.entity_id == *id)
                })
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
                let (colour, width) = match (hovered.is_some(), touches) {
                    (true, true) => (accent, 1.8),
                    (true, false) => (muted.opacity(0.14), 1.0),
                    _ => (muted.opacity(if flow.is_some() { 0.18 } else { 0.55 }), 1.0),
                };
                arrows.push(Arrow {
                    points,
                    colour,
                    width,
                });
            } else {
                let hot = steps.iter().any(|step| Some(step.step) == emphasis);
                let middle = halfway(&points);
                arrows.push(Arrow {
                    points,
                    colour: accent.opacity(if emphasis.is_none() || hot { 1.0 } else { 0.4 }),
                    width: if hot { 2.6 } else { 1.8 },
                });
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
            let element_id = format!("overview-box-{at}");
            let id = container.entity_id.clone();
            let facts = {
                let mut facts = vec![plural(container.decisions, "decisão", "decisões")];
                if container.parts > 0 {
                    facts.push(plural(container.parts, "parte", "partes"));
                }
                facts.join(" · ")
            };
            let label = format!(
                "{}: {}{} Ver detalhes.",
                container.name,
                facts,
                if container.conflicts > 0 {
                    format!(", {}", plural(container.conflicts, "conflito", "conflitos"))
                } else {
                    String::new()
                }
            );
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
                        .child(container.role.clone()),
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
                .tooltip(tooltip(format!("{} · ver detalhes", container.name), None))
                .focus_visible(crate::ui::controls::focus_ring(theme))
                .when(container.conflicts > 0, |boxed| {
                    boxed.child(
                        div()
                            .absolute()
                            .top(px(8.0))
                            .right(px(8.0))
                            .size(px(7.0))
                            .rounded_full()
                            .bg(colors.status_warning()),
                    )
                })
                .on_hover(cx.listener(move |this, hovered: &bool, _, cx| {
                    let next = if *hovered {
                        Some(at)
                    } else if this.hover_box == Some(at) {
                        None
                    } else {
                        this.hover_box
                    };
                    if this.hover_box != next {
                        this.hover_box = next;
                        cx.notify();
                    }
                }))
                .child(content)
                .child(
                    div().flex().gap(px(SpacingScale::S2)).child(
                        text_style(div(), TypeScale::META)
                            .truncate()
                            .text_color(colors.text_muted())
                            .child(facts),
                    ),
                );
            diagram = diagram.child(self.pressable(
                boxed,
                &element_id,
                move |this, cx| this.open_part_by_id(&id, cx),
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
                via: None,
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

    /// 12 containers: a spine of five, branches, a long skip, a back edge, a
    /// cycle and some that no flow touches.
    fn busy() -> Architecture {
        let names = [
            "app", "api", "jobs", "ai", "store", "queue", "search", "mail", "auth", "cache",
            "admin", "docs",
        ];
        Architecture {
            containers: names.iter().map(|name| container(name)).collect(),
            interactions: vec![
                link("app", "api"),
                link("api", "jobs"),
                link("jobs", "ai"),
                link("ai", "store"),
                link("api", "auth"),
                link("auth", "store"),
                link("api", "store"),
                link("app", "store"),
                link("store", "app"),
                link("jobs", "queue"),
                link("queue", "jobs"),
                link("queue", "mail"),
                link("api", "search"),
                link("search", "store"),
                link("api", "cache"),
            ],
            hidden: 0,
        }
    }

    /// Whether an axis-aligned segment goes through the inside of a box.
    fn cuts(a: Point<f32>, b: Point<f32>, (x, y): (f32, f32), width: f32) -> bool {
        let (lo_x, hi_x) = (a.x.min(b.x), a.x.max(b.x));
        let (lo_y, hi_y) = (a.y.min(b.y), a.y.max(b.y));
        hi_x > x + 1.0 && lo_x < x + width - 1.0 && hi_y > y + 1.0 && lo_y < y + BOX_HEIGHT - 1.0
    }

    #[test]
    fn no_arrow_of_a_busy_architecture_runs_through_a_box() {
        let architecture = busy();
        for only_flow in [None, Some(0)] {
            let laid = layout(&architecture, only_flow);
            for (interaction, path) in architecture
                .interactions
                .iter()
                .zip(route(&architecture, &laid))
            {
                let Some(path) = path else { continue };
                for pair in path.windows(2) {
                    for (at, top_left) in laid.boxes.iter().enumerate() {
                        assert!(
                            !cuts(pair[0], pair[1], *top_left, laid.width),
                            "{} to {} runs through {}",
                            interaction.from,
                            interaction.to,
                            architecture.containers[at].name
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn a_busy_architecture_stays_inside_the_column_and_boxes_never_overlap() {
        let architecture = busy();
        let laid = layout(&architecture, None);
        for (x, _) in &laid.boxes {
            assert!(*x >= 0.0 && x + laid.width <= WIDTH + 0.5);
        }
        for (i, a) in laid.boxes.iter().enumerate() {
            for b in laid.boxes.iter().skip(i + 1) {
                let apart = (a.0 - b.0).abs() >= laid.width || (a.1 - b.1).abs() >= BOX_HEIGHT;
                assert!(apart, "{a:?} overlaps {b:?}");
            }
        }
        for path in route(&architecture, &laid).into_iter().flatten() {
            for p in path {
                assert!(p.x >= 0.0 && p.x <= WIDTH && p.y >= 0.0 && p.y <= laid.height);
            }
        }
    }

    #[test]
    fn every_arrow_between_placed_boxes_gets_a_route() {
        let architecture = busy();
        let laid = layout(&architecture, None);
        let routes = route(&architecture, &laid);
        assert_eq!(routes.len(), architecture.interactions.len());
        let linked = architecture
            .interactions
            .iter()
            .zip(&routes)
            .filter(|(interaction, _)| interaction.from != interaction.to)
            .count();
        assert_eq!(routes.iter().flatten().count(), linked);
    }

    #[test]
    fn an_empty_architecture_has_no_height() {
        assert_eq!(layout(&Architecture::default(), None).height, 0.0);
    }
}
