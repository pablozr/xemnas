//! Grafo: the project map as a constellation (ADR-0005, revised).
//!
//! The whole map at once, readable at a glance: every top-level component is
//! an island (a soft halo) with its parts, decisions and rules gathered
//! around it; technologies sit where the decisions that use them pull them.
//! The layout is a force simulation settled before the first frame from
//! deterministic seeds, so the same map always opens the same way and never
//! dances; motion is reserved for meaning: the entrance, the focus that dims
//! everything but a node's neighborhood, and the pulses that run along the
//! focused links. Selection opens a side card with the node's ties and the
//! real actions (open, confirm or reject a suggestion).

use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;
use std::time::Instant;

use application::graph::{NodeRef, ProjectGraph};
use domain::entities::{EntityKind, NodeKind};
use gpui::prelude::*;
use gpui::{
    canvas, div, font, point, px, quad, size, App, Bounds, Context, Div, EventEmitter, FocusHandle,
    Hsla, KeyDownEvent, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, PathBuilder,
    Pixels, Point, Role, ScrollWheelEvent, SharedString, Stateful, TextAlign, TextRun, Window,
};

use super::format::clipped;
use crate::ui::controls::{action_button, focus_ring, icon_action, ButtonKind};
use crate::ui::icons::{icon, IconName};
use crate::ui::theme::{text_style, Theme};
use crate::ui::tokens::{RadiusScale, SpacingScale, TypeScale};
use crate::ui::tooltip::tooltip;

/// What the graph asks of the Mapa.
pub enum GraphEvent {
    /// Open an entity's detail.
    OpenEntity(String),
    /// Open a decision in Decisões.
    OpenDecision(String),
    /// Confirm a suggested edge.
    Confirm(String),
    /// Reject a suggested edge.
    Reject(String),
}

/// Simulation steps run before the first frame.
const SETTLE_STEPS: usize = 420;
/// Entrance length, in seconds.
const BLOOM_SECONDS: f32 = 0.9;
/// Longest label on a node.
const LABEL_CHARS: usize = 30;
/// Zoom limits.
const MIN_SCALE: f32 = 0.35;
const MAX_SCALE: f32 = 2.6;
/// Below this zoom only components and technologies keep their names.
const DETAIL_SCALE: f32 = 2.2;
/// Width of the side card.
const CARD_WIDTH: f32 = 300.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Component,
    Technology,
    Decision,
    Rule,
}

impl Kind {
    fn label(self) -> &'static str {
        match self {
            Self::Component => "Componente",
            Self::Technology => "Tecnologia",
            Self::Decision => "Decisão",
            Self::Rule => "Regra",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum LinkKind {
    PartOf,
    Affects,
    Uses,
    AppliesTo,
    Conflict,
    Relation,
    Suggested(String),
}

#[derive(Clone, Debug)]
struct Node {
    node: NodeRef,
    kind: Kind,
    label: String,
    detail: String,
    /// Index of the top-level component it gathers around.
    cluster: Option<usize>,
    degree: usize,
    /// Decisions and rules tied to it (components and technologies).
    weight: (usize, usize),
    conflict: bool,
    pos: (f32, f32),
    vel: (f32, f32),
    /// Displayed opacity, eased toward the focus target every frame.
    shade: f32,
    /// Order of appearance in the entrance, 0..1.
    stagger: f32,
}

impl Node {
    fn radius(&self) -> f32 {
        match self.kind {
            Kind::Component => 11.0 + (self.degree.min(12) as f32) * 1.3,
            Kind::Technology => 8.5,
            Kind::Decision => 3.6,
            Kind::Rule => 4.2,
        }
    }

    fn charge(&self) -> f32 {
        match self.kind {
            Kind::Component => 5200.0,
            Kind::Technology => 1600.0,
            Kind::Decision | Kind::Rule => 520.0,
        }
    }
}

#[derive(Clone, Debug)]
struct Link {
    a: usize,
    b: usize,
    kind: LinkKind,
}

impl Link {
    fn rest(&self) -> f32 {
        match self.kind {
            LinkKind::PartOf => 96.0,
            LinkKind::Affects | LinkKind::AppliesTo => 58.0,
            LinkKind::Uses => 120.0,
            LinkKind::Conflict | LinkKind::Relation => 70.0,
            LinkKind::Suggested(_) => 90.0,
        }
    }

    fn other(&self, index: usize) -> usize {
        if self.a == index {
            self.b
        } else {
            self.a
        }
    }
}

/// Which layers are on.
#[derive(Clone, Copy, Debug)]
struct Layers {
    decisions: bool,
    rules: bool,
    technologies: bool,
    suggestions: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Camera {
    x: f32,
    y: f32,
    scale: f32,
}

enum Drag {
    Node {
        index: usize,
        from: Point<Pixels>,
        moved: bool,
    },
    Pan {
        from: Point<Pixels>,
        camera: Camera,
        moved: bool,
    },
}

/// The interactive project graph.
pub struct GraphCanvas {
    nodes: Vec<Node>,
    links: Vec<Link>,
    layers: Layers,
    camera: Camera,
    target: Option<Camera>,
    hover: Option<usize>,
    selected: Option<usize>,
    drag: Option<Drag>,
    /// Canvas bounds in window coordinates, written at paint.
    bounds: Rc<Cell<Bounds<Pixels>>>,
    born: Instant,
    clock: Instant,
    /// Simulation heat while a node is dragged.
    heat: f32,
    fitted: bool,
    focus: FocusHandle,
    buttons: BTreeMap<String, FocusHandle>,
}

impl EventEmitter<GraphEvent> for GraphCanvas {}

impl GraphCanvas {
    /// An empty graph; data arrives with [`Self::set_graph`].
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            nodes: Vec::new(),
            links: Vec::new(),
            layers: Layers {
                decisions: true,
                rules: true,
                technologies: true,
                suggestions: true,
            },
            camera: Camera {
                x: 0.0,
                y: 0.0,
                scale: 1.0,
            },
            target: None,
            hover: None,
            selected: None,
            drag: None,
            bounds: Rc::new(Cell::new(Bounds::default())),
            born: Instant::now(),
            clock: Instant::now(),
            heat: 0.0,
            fitted: false,
            focus: cx.focus_handle().tab_stop(true),
            buttons: BTreeMap::new(),
        }
    }

    /// Replaces the data, keeping the place of nodes already shown; only a
    /// first load (or a new project) plays the entrance.
    pub fn set_graph(&mut self, graph: &ProjectGraph, fresh: bool, cx: &mut Context<Self>) {
        let previous: BTreeMap<NodeRef, (f32, f32)> = if fresh {
            BTreeMap::new()
        } else {
            self.nodes
                .iter()
                .map(|node| (node.node.clone(), node.pos))
                .collect()
        };
        let selected = self
            .selected
            .and_then(|index| self.nodes.get(index))
            .map(|node| node.node.clone());
        let (mut nodes, links) = build(graph);
        let known = place(&mut nodes, &links, &previous);
        settle(
            &mut nodes,
            &links,
            if known {
                SETTLE_STEPS / 4
            } else {
                SETTLE_STEPS
            },
            known,
        );
        stagger(&mut nodes);
        self.nodes = nodes;
        self.links = links;
        self.hover = None;
        self.selected =
            selected.and_then(|node| self.nodes.iter().position(|row| row.node == node));
        if fresh || !self.fitted {
            self.born = Instant::now();
            self.fitted = false;
        }
        cx.notify();
    }

    /// Selects the node named `label`, so demo captures reach the focus
    /// state without input.
    pub fn select_named(&mut self, label: &str, cx: &mut Context<Self>) {
        let found = self.nodes.iter().position(|node| node.label == label);
        if found.is_some() {
            self.selected = found;
            cx.notify();
        }
    }

    /// Whether there is anything to draw.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    fn visible(&self, index: usize) -> bool {
        match self.nodes[index].kind {
            Kind::Component => true,
            Kind::Technology => self.layers.technologies,
            Kind::Decision => self.layers.decisions,
            Kind::Rule => self.layers.rules,
        }
    }

    fn link_visible(&self, link: &Link) -> bool {
        self.visible(link.a)
            && self.visible(link.b)
            && (self.layers.suggestions || !matches!(link.kind, LinkKind::Suggested(_)))
    }

    /// The focused node and its visible neighbors.
    fn focus_set(&self) -> Option<(usize, BTreeSet<usize>)> {
        let center = self.hover.or(self.selected)?;
        let mut set = BTreeSet::from([center]);
        for link in &self.links {
            if self.link_visible(link) && (link.a == center || link.b == center) {
                set.insert(link.other(center));
            }
        }
        Some((center, set))
    }

    fn to_world(&self, position: Point<Pixels>) -> (f32, f32) {
        let bounds = self.bounds.get();
        let x = f32::from(position.x - bounds.origin.x);
        let y = f32::from(position.y - bounds.origin.y);
        (
            (x - self.camera.x) / self.camera.scale,
            (y - self.camera.y) / self.camera.scale,
        )
    }

    fn hit(&self, position: Point<Pixels>) -> Option<usize> {
        let (x, y) = self.to_world(position);
        let slack = 4.0 / self.camera.scale;
        self.nodes
            .iter()
            .enumerate()
            .filter(|(index, _)| self.visible(*index))
            .map(|(index, node)| {
                let distance = ((node.pos.0 - x).powi(2) + (node.pos.1 - y).powi(2)).sqrt();
                (index, distance - node.radius().max(7.0) - slack)
            })
            .filter(|(_, gap)| *gap <= 0.0)
            .min_by(|left, right| left.1.total_cmp(&right.1))
            .map(|(index, _)| index)
    }

    /// The camera that frames every visible node.
    fn fit_camera(&self) -> Option<Camera> {
        let bounds = self.bounds.get();
        let (width, height) = (f32::from(bounds.size.width), f32::from(bounds.size.height));
        if width < 1.0 || height < 1.0 {
            return None;
        }
        let visible: Vec<&Node> = self
            .nodes
            .iter()
            .enumerate()
            .filter(|(index, _)| self.visible(*index))
            .map(|(_, node)| node)
            .collect();
        if visible.is_empty() {
            return None;
        }
        let (mut left, mut top, mut right, mut bottom) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
        for node in visible {
            let r = node.radius() + 28.0;
            left = left.min(node.pos.0 - r);
            right = right.max(node.pos.0 + r);
            top = top.min(node.pos.1 - r);
            bottom = bottom.max(node.pos.1 + r);
        }
        // The side card covers the right of the canvas while open.
        let usable = if self.selected.is_some() {
            width - CARD_WIDTH - SpacingScale::S6
        } else {
            width
        };
        // Room for the layer chips above and the legend below.
        let (above, below) = (64.0, 84.0);
        let height_free = height - above - below;
        let scale = (usable / (right - left))
            .min(height_free / (bottom - top))
            .clamp(MIN_SCALE, 1.7);
        Some(Camera {
            scale,
            x: usable / 2.0 - (left + right) / 2.0 * scale,
            y: above + height_free / 2.0 - (top + bottom) / 2.0 * scale,
        })
    }

    fn zoom_by(&mut self, factor: f32, anchor: Option<(f32, f32)>, cx: &mut Context<Self>) {
        let bounds = self.bounds.get();
        let (ax, ay) = anchor.unwrap_or((
            f32::from(bounds.size.width) / 2.0,
            f32::from(bounds.size.height) / 2.0,
        ));
        let base = self.target.unwrap_or(self.camera);
        let scale = (base.scale * factor).clamp(MIN_SCALE, MAX_SCALE);
        let applied = scale / base.scale;
        let next = Camera {
            scale,
            x: ax - (ax - base.x) * applied,
            y: ay - (ay - base.y) * applied,
        };
        if anchor.is_some() {
            self.camera = next;
            self.target = None;
        } else {
            self.target = Some(next);
        }
        cx.notify();
    }

    fn select(&mut self, index: Option<usize>, cx: &mut Context<Self>) {
        self.selected = index;
        if let Some(index) = index {
            // Bring the node into the free part of the canvas.
            let bounds = self.bounds.get();
            let usable = f32::from(bounds.size.width) - CARD_WIDTH - SpacingScale::S6;
            let node = &self.nodes[index];
            let screen_x = node.pos.0 * self.camera.scale + self.camera.x;
            let screen_y = node.pos.1 * self.camera.scale + self.camera.y;
            let height = f32::from(bounds.size.height);
            if !(48.0..=usable - 48.0).contains(&screen_x)
                || !(48.0..=height - 48.0).contains(&screen_y)
            {
                self.target = Some(Camera {
                    scale: self.camera.scale,
                    x: usable / 2.0 - node.pos.0 * self.camera.scale,
                    y: height / 2.0 - node.pos.1 * self.camera.scale,
                });
            }
        }
        self.clock = Instant::now();
        cx.notify();
    }

    fn toggle_layer(&mut self, layer: usize, cx: &mut Context<Self>) {
        match layer {
            0 => self.layers.decisions = !self.layers.decisions,
            1 => self.layers.rules = !self.layers.rules,
            2 => self.layers.technologies = !self.layers.technologies,
            _ => self.layers.suggestions = !self.layers.suggestions,
        }
        if self.selected.is_some_and(|index| !self.visible(index)) {
            self.selected = None;
        }
        cx.notify();
    }

    // ---- input ------------------------------------------------------------

    fn on_down(&mut self, event: &MouseDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus, cx);
        self.target = None;
        self.drag = Some(match self.hit(event.position) {
            Some(index) => Drag::Node {
                index,
                from: event.position,
                moved: false,
            },
            None => Drag::Pan {
                from: event.position,
                camera: self.camera,
                moved: false,
            },
        });
    }

    fn on_move(&mut self, event: &MouseMoveEvent, _: &mut Window, cx: &mut Context<Self>) {
        let pressed = event.pressed_button == Some(MouseButton::Left);
        if !pressed {
            self.drag = None;
        }
        let position = event.position;
        let world = self.to_world(position);
        let scale = self.camera.scale;
        match &mut self.drag {
            Some(Drag::Node { index, from, moved }) => {
                if *moved || distance(*from, position) > 3.0 {
                    *moved = true;
                    let node = &mut self.nodes[*index];
                    node.pos = world;
                    node.vel = (0.0, 0.0);
                    self.heat = self.heat.max(0.25);
                    cx.notify();
                }
            }
            Some(Drag::Pan {
                from,
                camera,
                moved,
            }) => {
                if *moved || distance(*from, position) > 3.0 {
                    *moved = true;
                    self.camera = Camera {
                        x: camera.x + f32::from(position.x - from.x),
                        y: camera.y + f32::from(position.y - from.y),
                        scale,
                    };
                    cx.notify();
                }
            }
            None => {
                let hover = self.hit(position);
                if hover != self.hover {
                    self.hover = hover;
                    self.clock = Instant::now();
                    cx.notify();
                }
            }
        }
    }

    fn on_up(&mut self, _: &MouseUpEvent, _: &mut Window, cx: &mut Context<Self>) {
        match self.drag.take() {
            Some(Drag::Node {
                index,
                moved: false,
                ..
            }) => {
                let next = (self.selected != Some(index)).then_some(index);
                self.select(next, cx);
            }
            Some(Drag::Pan { moved: false, .. }) => self.select(None, cx),
            _ => {}
        }
    }

    fn on_wheel(&mut self, event: &ScrollWheelEvent, _: &mut Window, cx: &mut Context<Self>) {
        let delta = event.delta.pixel_delta(px(20.0));
        let bounds = self.bounds.get();
        let anchor = (
            f32::from(event.position.x - bounds.origin.x),
            f32::from(event.position.y - bounds.origin.y),
        );
        let factor = (-f32::from(delta.y) * 0.0022).exp();
        self.zoom_by(factor, Some(anchor), cx);
    }

    fn on_key(&mut self, event: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        match event.keystroke.key.as_str() {
            "escape" if self.selected.is_some() => {
                self.select(None, cx);
                cx.stop_propagation();
            }
            "0" => {
                self.target = self.fit_camera();
                cx.notify();
            }
            "+" | "=" => self.zoom_by(1.25, None, cx),
            "-" => self.zoom_by(0.8, None, cx),
            _ => {}
        }
    }

    // ---- animation --------------------------------------------------------

    /// Advances heat, camera easing and focus shading; whether another frame
    /// is needed.
    fn step(&mut self, reduce_motion: bool) -> bool {
        let mut busy = false;
        if self.heat > 0.01 {
            let links = self.links.clone();
            tick(&mut self.nodes, &links, self.heat);
            self.heat *= 0.94;
            busy = true;
        }
        if let Some(target) = self.target {
            let ease = if reduce_motion { 1.0 } else { 0.18 };
            self.camera = Camera {
                x: lerp(self.camera.x, target.x, ease),
                y: lerp(self.camera.y, target.y, ease),
                scale: lerp(self.camera.scale, target.scale, ease),
            };
            if (self.camera.x - target.x).abs() < 0.5
                && (self.camera.y - target.y).abs() < 0.5
                && (self.camera.scale - target.scale).abs() < 0.002
            {
                self.camera = target;
                self.target = None;
            } else {
                busy = true;
            }
        }
        let focus = self.focus_set().map(|(_, set)| set);
        for index in 0..self.nodes.len() {
            let goal = match &focus {
                Some(set) if !set.contains(&index) => 0.16,
                _ => 1.0,
            };
            let node = &mut self.nodes[index];
            let ease = if reduce_motion { 1.0 } else { 0.22 };
            node.shade = lerp(node.shade, goal, ease);
            if (node.shade - goal).abs() > 0.01 {
                busy = true;
            } else {
                node.shade = goal;
            }
        }
        busy
    }

    // ---- overlays ---------------------------------------------------------

    fn button_focus(&mut self, id: &str, cx: &mut Context<Self>) -> FocusHandle {
        self.buttons
            .entry(id.to_owned())
            .or_insert_with(|| cx.focus_handle().tab_stop(true))
            .clone()
    }

    /// A focusable control that runs `on_press` on click, Enter or Space.
    fn pressable(
        &mut self,
        element: Stateful<Div>,
        id: &str,
        on_press: impl Fn(&mut Self, &mut Context<Self>) + 'static,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let focus = self.button_focus(id, cx);
        let on_press = Rc::new(on_press);
        let on_key = on_press.clone();
        element
            .track_focus(&focus)
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_click(cx.listener(move |this, _, _, cx| on_press(this, cx)))
            .on_key_down(cx.listener(move |this, event: &KeyDownEvent, _, cx| {
                if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                    on_key(this, cx);
                    cx.stop_propagation();
                }
            }))
    }

    fn layer_chips(&mut self, theme: &Theme, cx: &mut Context<Self>) -> Div {
        let colors = theme.colors;
        let count = |kind: Kind| self.nodes.iter().filter(|node| node.kind == kind).count();
        let suggested = self
            .links
            .iter()
            .filter(|link| matches!(link.kind, LinkKind::Suggested(_)))
            .count();
        let chips = [
            (
                "Decisões",
                count(Kind::Decision),
                self.layers.decisions,
                colors.graph_decision(),
            ),
            (
                "Regras",
                count(Kind::Rule),
                self.layers.rules,
                colors.status_info(),
            ),
            (
                "Tecnologias",
                count(Kind::Technology),
                self.layers.technologies,
                colors.graph_technology(),
            ),
            (
                "Sugestões",
                suggested,
                self.layers.suggestions,
                colors.status_warning(),
            ),
        ];
        let mut row = div().flex().gap(px(SpacingScale::S1));
        for (layer, (label, count, on, color)) in chips.into_iter().enumerate() {
            let id = format!("graph-layer-{layer}");
            let chip = div()
                .id(SharedString::from(id.clone()))
                .flex()
                .items_center()
                .gap(px(6.0))
                .h(px(26.0))
                .px(px(SpacingScale::S2 + 2.0))
                .rounded(px(13.0))
                .border_1()
                .border_color(if on {
                    colors.glass_border()
                } else {
                    colors.hairline_divider()
                })
                .bg(if on {
                    colors.glass_fill_medium()
                } else {
                    colors.canvas_deep().alpha(0.7)
                })
                .cursor_pointer()
                .role(Role::Switch)
                .aria_label(format!(
                    "{label}: {}",
                    if on { "visíveis" } else { "ocultas" }
                ))
                .focus_visible(focus_ring(theme))
                .child(div().size(px(7.0)).rounded_full().bg(if on {
                    color
                } else {
                    colors.text_disabled()
                }))
                .child(
                    text_style(div(), TypeScale::META)
                        .text_color(if on {
                            colors.text_primary()
                        } else {
                            colors.text_muted()
                        })
                        .child(format!("{label} {count}")),
                );
            row = row.child(self.pressable(
                chip,
                &id,
                move |this, cx| this.toggle_layer(layer, cx),
                cx,
            ));
        }
        row
    }

    fn zoom_controls(&mut self, theme: &Theme, cx: &mut Context<Self>) -> Div {
        let colors = theme.colors;
        let mut column = div()
            .flex()
            .flex_col()
            .p(px(2.0))
            .rounded(RadiusScale.surface())
            .border_1()
            .border_color(colors.glass_border())
            .bg(colors.canvas_deep().alpha(0.88));
        for (id, glyph, label, key) in [
            ("graph-zoom-in", IconName::Plus, "Aproximar", "+"),
            ("graph-zoom-out", IconName::Minus, "Afastar", "-"),
            ("graph-fit", IconName::Fit, "Enquadrar tudo", "0"),
        ] {
            let button = icon_action(theme, id, label)
                .tooltip(tooltip(label, Some(key)))
                .child(icon(glyph, 14.0, colors.text_secondary()));
            column = column.child(self.pressable(
                button,
                id,
                move |this, cx| match key {
                    "+" => this.zoom_by(1.25, None, cx),
                    "-" => this.zoom_by(0.8, None, cx),
                    _ => {
                        this.target = this.fit_camera();
                        cx.notify();
                    }
                },
                cx,
            ));
        }
        column
    }

    fn legend(&self, theme: &Theme) -> Div {
        let colors = theme.colors;
        let item = |mark: Div, label: &'static str| {
            div()
                .flex()
                .items_center()
                .gap(px(6.0))
                .child(div().w(px(14.0)).flex().justify_center().child(mark))
                .child(
                    text_style(div(), TypeScale::META)
                        .text_color(colors.text_muted())
                        .child(label),
                )
        };
        div()
            .flex()
            .flex_wrap()
            .gap_x(px(SpacingScale::S3))
            .gap_y(px(SpacingScale::S1))
            .child(item(
                div()
                    .size(px(12.0))
                    .rounded_full()
                    .border_1()
                    .border_color(colors.graph_component())
                    .bg(colors.graph_component().alpha(0.25)),
                "Componente",
            ))
            .child(item(
                div()
                    .size(px(10.0))
                    .rounded(px(3.0))
                    .border_1()
                    .border_color(colors.graph_technology())
                    .bg(colors.graph_technology().alpha(0.25)),
                "Tecnologia",
            ))
            .child(item(
                div()
                    .size(px(7.0))
                    .rounded_full()
                    .bg(colors.graph_decision()),
                "Decisão",
            ))
            .child(item(
                div()
                    .size(px(8.0))
                    .rounded(px(1.0))
                    .border_1()
                    .border_color(colors.status_info()),
                "Regra",
            ))
            .child(item(
                div()
                    .size(px(7.0))
                    .rounded_full()
                    .bg(colors.status_warning()),
                "Em conflito",
            ))
            .child(item(
                div().flex().gap(px(2.0)).children(
                    (0..3).map(|_| div().w(px(3.0)).h(px(1.5)).bg(colors.status_warning())),
                ),
                "Sugestão",
            ))
    }

    fn side_card(&mut self, theme: &Theme, cx: &mut Context<Self>) -> Option<Stateful<Div>> {
        let index = self.selected?;
        let colors = theme.colors;
        let node = self.nodes.get(index)?.clone();
        let mut ties: Vec<(usize, &'static str)> = Vec::new();
        let mut suggestions: Vec<(String, usize)> = Vec::new();
        for link in &self.links {
            if link.a != index && link.b != index {
                continue;
            }
            let other = link.other(index);
            match &link.kind {
                LinkKind::Suggested(edge) => suggestions.push((edge.clone(), other)),
                kind => ties.push((other, tie_label(kind, link.a == index))),
            }
        }
        ties.sort_by_key(|(other, _)| (kind_order(self.nodes[*other].kind), *other));

        let close = icon_action(theme, "graph-card-close", "Fechar")
            .tooltip(tooltip("Fechar", Some("Esc")))
            .child(icon(IconName::Close, 14.0, colors.text_muted()));
        let close = self.pressable(
            close,
            "graph-card-close",
            |this, cx| this.select(None, cx),
            cx,
        );

        let mut list = div().flex().flex_col();
        for (row_index, (other, relation)) in ties.iter().take(14).enumerate() {
            let other_node = &self.nodes[*other];
            let id = format!("graph-tie-{row_index}");
            let target = *other;
            let row = div()
                .id(SharedString::from(id.clone()))
                .flex()
                .items_center()
                .gap(px(SpacingScale::S2))
                .px(px(SpacingScale::S2))
                .py(px(6.0))
                .rounded(theme.radius.control())
                .cursor_pointer()
                .hover(move |style| style.bg(colors.glass_fill_low()))
                .role(Role::Button)
                .aria_label(format!("{}: {}", other_node.kind.label(), other_node.label))
                .focus_visible(focus_ring(theme))
                .child(swatch(theme, other_node))
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.0))
                        .flex()
                        .flex_col()
                        .child(
                            text_style(div(), TypeScale::BODY_SMALL)
                                .text_color(colors.text_primary())
                                .child(clipped(&other_node.label, 44)),
                        )
                        .child(
                            text_style(div(), TypeScale::META)
                                .text_color(colors.text_muted())
                                .child(*relation),
                        ),
                );
            list = list.child(self.pressable(
                row,
                &id,
                move |this, cx| this.select(Some(target), cx),
                cx,
            ));
        }
        let more = ties.len().saturating_sub(14);

        let mut proposals = div().flex().flex_col().gap(px(SpacingScale::S2));
        for (row_index, (edge, other)) in suggestions.iter().enumerate() {
            let confirm_edge = edge.clone();
            let reject_edge = edge.clone();
            let confirm = action_button(
                theme,
                SharedString::from(format!("graph-confirm-{row_index}")),
                ButtonKind::Secondary,
                true,
            )
            .aria_label("Confirmar ligação")
            .child("Confirmar");
            let confirm = self.pressable(
                confirm,
                &format!("graph-confirm-{row_index}"),
                move |_, cx| cx.emit(GraphEvent::Confirm(confirm_edge.clone())),
                cx,
            );
            let reject = action_button(
                theme,
                SharedString::from(format!("graph-reject-{row_index}")),
                ButtonKind::Ghost,
                true,
            )
            .aria_label("Rejeitar ligação")
            .child("Rejeitar");
            let reject = self.pressable(
                reject,
                &format!("graph-reject-{row_index}"),
                move |_, cx| cx.emit(GraphEvent::Reject(reject_edge.clone())),
                cx,
            );
            proposals = proposals.child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(SpacingScale::S2))
                    .p(px(SpacingScale::S3))
                    .rounded(px(8.0))
                    .border_1()
                    .border_dashed()
                    .border_color(colors.status_warning().alpha(0.45))
                    .child(
                        text_style(div(), TypeScale::BODY_SMALL)
                            .text_color(colors.text_secondary())
                            .child(format!(
                                "Ligar a {}?",
                                clipped(&self.nodes[*other].label, 40)
                            )),
                    )
                    .child(
                        div()
                            .flex()
                            .gap(px(SpacingScale::S2))
                            .child(confirm)
                            .child(reject),
                    ),
            );
        }

        let (open_label, open_event) = match node.kind {
            Kind::Component | Kind::Technology => (
                Some("Abrir no Mapa"),
                Some(GraphEvent::OpenEntity(node.node.id.clone())),
            ),
            Kind::Decision => (
                Some("Abrir em Decisões"),
                Some(GraphEvent::OpenDecision(node.node.id.clone())),
            ),
            Kind::Rule => (None, None),
        };
        let open = open_label.zip(open_event).map(|(label, event)| {
            let event = Rc::new(Cell::new(Some(event)));
            let button = action_button(theme, "graph-open", ButtonKind::Primary, true)
                .aria_label(label)
                .child(label)
                .child(icon(
                    IconName::ArrowUpRight,
                    13.0,
                    colors.accent_on_emphasis(),
                ));
            self.pressable(
                button,
                "graph-open",
                move |_, cx| {
                    if let Some(event) = event.take() {
                        cx.emit(event);
                    }
                },
                cx,
            )
        });

        let card = div()
            .id("graph-card")
            .absolute()
            .top(px(SpacingScale::S4))
            .right(px(SpacingScale::S4))
            .bottom(px(SpacingScale::S4))
            .w(px(CARD_WIDTH))
            .flex()
            .flex_col()
            .rounded(RadiusScale.dialog())
            .border_1()
            .border_color(colors.glass_border())
            .bg(colors.canvas_raised().alpha(0.96))
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
            .child(
                div()
                    .flex()
                    .items_start()
                    .gap(px(SpacingScale::S2))
                    .p(px(SpacingScale::S4))
                    .pb(px(SpacingScale::S3))
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.0))
                            .flex()
                            .flex_col()
                            .gap(px(SpacingScale::S1))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(6.0))
                                    .child(swatch(theme, &node))
                                    .child(
                                        text_style(div(), TypeScale::META)
                                            .text_color(colors.text_muted())
                                            .child(if node.conflict {
                                                format!("{} · em conflito", node.kind.label())
                                            } else {
                                                node.kind.label().to_owned()
                                            }),
                                    ),
                            )
                            .child(
                                text_style(div(), TypeScale::HEADING_2)
                                    .text_color(colors.text_primary())
                                    .child(node.label.clone()),
                            )
                            .when(!node.detail.is_empty(), |column| {
                                column.child(
                                    text_style(div(), TypeScale::BODY_SMALL)
                                        .text_color(colors.text_secondary())
                                        .child(node.detail.clone()),
                                )
                            }),
                    )
                    .child(close),
            )
            .child(
                div()
                    .id("graph-card-body")
                    .flex_1()
                    .min_h(px(0.0))
                    .overflow_y_scroll()
                    .px(px(SpacingScale::S2))
                    .flex()
                    .flex_col()
                    .gap(px(SpacingScale::S3))
                    .when(!suggestions.is_empty(), |body| {
                        body.child(
                            div()
                                .px(px(SpacingScale::S2))
                                .flex()
                                .flex_col()
                                .gap(px(SpacingScale::S2))
                                .child(card_label(theme, "Sugestões"))
                                .child(proposals),
                        )
                    })
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(SpacingScale::S1))
                            .child(div().px(px(SpacingScale::S2)).child(card_label(
                                theme,
                                if ties.is_empty() {
                                    "Sem ligações confirmadas"
                                } else {
                                    "Ligações"
                                },
                            )))
                            .child(list)
                            .when(more > 0, |column| {
                                column.child(
                                    text_style(div(), TypeScale::META)
                                        .px(px(SpacingScale::S2))
                                        .text_color(colors.text_muted())
                                        .child(format!("E mais {more}")),
                                )
                            }),
                    ),
            )
            .children(open.map(|open| {
                div()
                    .p(px(SpacingScale::S4))
                    .border_t_1()
                    .border_color(colors.hairline_divider())
                    .flex()
                    .child(open)
            }));
        Some(card)
    }
}

impl Render for GraphCanvas {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::current(cx);
        let colors = theme.colors;
        let reduce_motion = cx.reduce_motion();
        if !self.fitted {
            if let Some(camera) = self.fit_camera() {
                self.camera = camera;
                self.fitted = true;
            }
        }
        let busy = self.step(reduce_motion);
        let bloom = if reduce_motion {
            1.0
        } else {
            (self.born.elapsed().as_secs_f32() / BLOOM_SECONDS).min(1.0)
        };
        let focus = self.focus_set();
        let motion = !reduce_motion;
        if busy || bloom < 1.0 || motion || !self.fitted {
            window.request_animation_frame();
        }

        let scene = Scene {
            nodes: self
                .nodes
                .iter()
                .enumerate()
                .map(|(index, node)| (node.clone(), self.visible(index)))
                .collect(),
            links: self
                .links
                .iter()
                .filter(|link| self.link_visible(link))
                .cloned()
                .collect(),
            camera: self.camera,
            bloom,
            focus,
            selected: self.selected,
            time: self.clock.elapsed().as_secs_f32(),
            motion,
        };
        let bounds_cell = self.bounds.clone();
        let painter = canvas(
            move |bounds, _, _| bounds_cell.set(bounds),
            move |bounds, _, window, cx| scene.paint(bounds, &theme, window, cx),
        )
        .size_full();

        let chips = self.layer_chips(&theme, cx);
        let zoom = self.zoom_controls(&theme, cx);
        let legend = self.legend(&theme);
        let card = self.side_card(&theme, cx);
        let hint = (self.selected.is_none()).then(|| {
            text_style(div(), TypeScale::META)
                .absolute()
                .top(px(SpacingScale::S4 + 5.0))
                .right(px(SpacingScale::S4))
                .text_color(colors.text_muted())
                .child("Clique para focar · arraste para mover · role para aproximar")
        });
        let summary = format!(
            "Grafo do projeto: {} componentes, {} decisões, {} regras, {} tecnologias.",
            self.nodes
                .iter()
                .filter(|n| n.kind == Kind::Component)
                .count(),
            self.nodes
                .iter()
                .filter(|n| n.kind == Kind::Decision)
                .count(),
            self.nodes.iter().filter(|n| n.kind == Kind::Rule).count(),
            self.nodes
                .iter()
                .filter(|n| n.kind == Kind::Technology)
                .count(),
        );
        let cursor_pointer = self.hover.is_some();
        div()
            .id("graph-canvas")
            .size_full()
            .relative()
            .overflow_hidden()
            .rounded(RadiusScale.dialog())
            .border_1()
            .border_color(colors.hairline_divider())
            .bg(colors.canvas_deep())
            .track_focus(&self.focus)
            .key_context("Graph")
            .role(Role::Figure)
            .aria_label(summary)
            .focus_visible(focus_ring(&theme))
            .when(cursor_pointer, |canvas| canvas.cursor_pointer())
            .on_key_down(cx.listener(Self::on_key))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_down))
            .on_mouse_move(cx.listener(Self::on_move))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_up))
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|this, _: &MouseUpEvent, _, _| this.drag = None),
            )
            .on_scroll_wheel(cx.listener(Self::on_wheel))
            .child(painter)
            .child(
                div()
                    .absolute()
                    .top(px(SpacingScale::S4))
                    .left(px(SpacingScale::S4))
                    .child(chips),
            )
            .children(hint)
            .child(
                div()
                    .absolute()
                    .bottom(px(SpacingScale::S4))
                    .left(px(SpacingScale::S4))
                    .max_w(px(420.0))
                    .child(legend),
            )
            .child(
                div()
                    .absolute()
                    .bottom(px(SpacingScale::S4))
                    .right(px(if self.selected.is_some() {
                        CARD_WIDTH + SpacingScale::S4 * 2.0
                    } else {
                        SpacingScale::S4
                    }))
                    .child(zoom),
            )
            .children(card)
    }
}

// ---- painting ---------------------------------------------------------------

/// Everything one frame paints, detached from the entity.
struct Scene {
    nodes: Vec<(Node, bool)>,
    links: Vec<Link>,
    camera: Camera,
    bloom: f32,
    focus: Option<(usize, BTreeSet<usize>)>,
    selected: Option<usize>,
    /// Seconds since the graph opened: drives the signal along the links.
    time: f32,
    /// Whether anything moves (off under reduced motion).
    motion: bool,
}

impl Scene {
    fn appear(&self, node: &Node) -> f32 {
        // Each node lights up in its own slot of the entrance.
        let start = node.stagger * 0.55;
        ((self.bloom - start) / 0.45).clamp(0.0, 1.0)
    }

    fn screen(&self, bounds: Bounds<Pixels>, pos: (f32, f32)) -> (f32, f32) {
        (
            f32::from(bounds.origin.x) + pos.0 * self.camera.scale + self.camera.x,
            f32::from(bounds.origin.y) + pos.1 * self.camera.scale + self.camera.y,
        )
    }

    fn paint(&self, bounds: Bounds<Pixels>, theme: &Theme, window: &mut Window, cx: &mut App) {
        window.with_content_mask(Some(gpui::ContentMask { bounds }), |window| {
            self.paint_grid(bounds, theme, window);
            self.paint_links(bounds, theme, window);
            // Small first so components sit on top.
            let mut order: Vec<usize> = (0..self.nodes.len())
                .filter(|index| self.nodes[*index].1)
                .collect();
            order.sort_by_key(|index| -kind_order(self.nodes[*index].0.kind));
            for index in &order {
                self.paint_node(bounds, *index, theme, window);
            }
            for index in &order {
                self.paint_label(bounds, *index, theme, window, cx);
            }
        });
    }

    /// A faint dot field that pans and zooms with the map.
    fn paint_grid(&self, bounds: Bounds<Pixels>, theme: &Theme, window: &mut Window) {
        let step = 22.0 * self.camera.scale;
        if step < 10.0 {
            return;
        }
        let dot = Hsla::from(theme.colors.graph_component()).opacity(0.07);
        let (ox, oy) = (f32::from(bounds.origin.x), f32::from(bounds.origin.y));
        let (width, height) = (f32::from(bounds.size.width), f32::from(bounds.size.height));
        let start_x = self.camera.x.rem_euclid(step);
        let start_y = self.camera.y.rem_euclid(step);
        let mut y = start_y;
        while y < height {
            let mut x = start_x;
            while x < width {
                window.paint_quad(gpui::fill(
                    Bounds::new(
                        point(px(ox + x - 0.6), px(oy + y - 0.6)),
                        size(px(1.2), px(1.2)),
                    ),
                    dot,
                ));
                x += step;
            }
            y += step;
        }
    }

    /// Luminous hairlines: a wide faint stroke under a thin bright one, drawn
    /// in from the component outward, with signal running along them.
    fn paint_links(&self, bounds: Bounds<Pixels>, theme: &Theme, window: &mut Window) {
        let colors = theme.colors;
        let signal = Hsla::from(colors.graph_component());
        let focus_center = self.focus.as_ref().map(|(center, _)| *center);
        for (number, link) in self.links.iter().enumerate() {
            let (a, b) = (&self.nodes[link.a].0, &self.nodes[link.b].0);
            // The line grows out of the earlier of its two ends.
            let start_at = a.stagger.min(b.stagger) * 0.55;
            let grown = ((self.bloom - start_at) / 0.6).clamp(0.0, 1.0);
            if grown <= 0.0 {
                continue;
            }
            let grown = ease_out_cubic(grown);
            let lit = focus_center.is_some_and(|center| link.a == center || link.b == center);
            let shade = a.shade.min(b.shade);
            // Draw from the component side when there is one.
            let (from, to) = if kind_order(a.kind) <= kind_order(b.kind) {
                (a, b)
            } else {
                (b, a)
            };
            let (start, end) = (self.screen(bounds, from.pos), self.screen(bounds, to.pos));
            let control = bend(start, end);
            let (tint, core, dashed) = match &link.kind {
                LinkKind::Suggested(_) => (Hsla::from(colors.status_warning()), 0.75, true),
                LinkKind::Conflict => (Hsla::from(colors.status_warning()), 0.8, false),
                LinkKind::AppliesTo => (Hsla::from(colors.status_info()), 0.6, true),
                LinkKind::PartOf => (signal, 0.55, false),
                _ => (signal, 0.45, false),
            };
            let strokes: &[(f32, f32)] = if lit {
                &[(6.0, 0.06), (3.0, 0.14), (1.4, 1.0)]
            } else {
                &[(3.5, 0.035), (1.0, core)]
            };
            for &(width, alpha) in strokes {
                let mut builder = PathBuilder::stroke(px(width));
                if dashed && width < 2.0 {
                    builder = builder.dash_array(&[px(3.0), px(4.0)]);
                }
                builder.move_to(point(px(start.0), px(start.1)));
                let segments = 18;
                for step in 1..=segments {
                    let t = grown * step as f32 / segments as f32;
                    let p = quad_point(start, control, end, t);
                    builder.line_to(point(px(p.0), px(p.1)));
                }
                if let Ok(path) = builder.build() {
                    window.paint_path(path, tint.opacity(alpha * shade));
                }
            }
            // Signal: a short comet running along every line; faster and
            // brighter on the focused node's lines.
            if self.motion && grown >= 1.0 && shade > 0.3 {
                let (speed, heads, bright) = if lit { (0.55, 2, 1.0) } else { (0.16, 1, 0.55) };
                let phase = seed(&format!("{number}"), 7);
                for head in 0..heads {
                    let t = (self.time * speed + phase + head as f32 / heads as f32).fract();
                    for trail in 0..5 {
                        let back = t - trail as f32 * 0.018;
                        if !(0.0..=1.0).contains(&back) {
                            continue;
                        }
                        let p = quad_point(start, control, end, back);
                        let fade = 1.0 - trail as f32 / 5.0;
                        let edge_fade = (back * (1.0 - back) * 6.0).min(1.0);
                        circle(
                            window,
                            p,
                            if trail == 0 { 1.7 } else { 1.2 },
                            tint.opacity(bright * fade * edge_fade * shade),
                            None,
                        );
                    }
                }
            }
        }
    }

    fn paint_node(&self, bounds: Bounds<Pixels>, index: usize, theme: &Theme, window: &mut Window) {
        let colors = theme.colors;
        let node = &self.nodes[index].0;
        let appear = self.appear(node);
        if appear <= 0.0 {
            return;
        }
        let grow = ease_out_back(appear);
        let (x, y) = self.screen(bounds, node.pos);
        let zoom = self.camera.scale.clamp(0.6, 1.6);
        let radius = node.radius() * zoom * grow;
        // A component nothing was decided about yet stays dimmer.
        let quiet = if node.kind == Kind::Component && node.weight == (0, 0) {
            0.6
        } else {
            1.0
        };
        let shade = node.shade * appear * quiet;
        let ground = Hsla::from(colors.canvas_deep());
        let signal = Hsla::from(colors.graph_component());
        match node.kind {
            Kind::Component => {
                // A lens: soft bloom, dark body, bright rim, an orbit and a core.
                circle(
                    window,
                    (x, y),
                    radius + 14.0,
                    signal.opacity(0.035 * shade),
                    None,
                );
                circle(
                    window,
                    (x, y),
                    radius + 7.0,
                    signal.opacity(0.05 * shade),
                    None,
                );
                circle(
                    window,
                    (x, y),
                    radius,
                    ground.opacity(shade),
                    Some((signal.opacity(0.9 * shade), 1.25)),
                );
                circle(
                    window,
                    (x, y),
                    radius + 4.5,
                    gpui::transparent_black(),
                    Some((signal.opacity(0.16 * shade), 1.0)),
                );
                circle(
                    window,
                    (x, y),
                    radius * 0.34 + 1.5,
                    signal.opacity(0.25 * shade),
                    None,
                );
                circle(
                    window,
                    (x, y),
                    radius * 0.34,
                    signal.opacity(0.95 * shade),
                    None,
                );
            }
            Kind::Technology => {
                let tint = Hsla::from(colors.graph_technology());
                let r = radius * 1.05;
                let corners: Vec<Point<Pixels>> = (0..6)
                    .map(|corner| {
                        let angle = std::f32::consts::FRAC_PI_3 * corner as f32
                            + std::f32::consts::FRAC_PI_6;
                        point(px(x + r * angle.cos()), px(y + r * angle.sin()))
                    })
                    .collect();
                let mut body = PathBuilder::fill();
                body.add_polygon(&corners, true);
                if let Ok(path) = body.build() {
                    window.paint_path(path, ground.opacity(shade));
                }
                let mut rim = PathBuilder::stroke(px(1.25));
                rim.add_polygon(&corners, true);
                if let Ok(path) = rim.build() {
                    window.paint_path(path, tint.opacity(0.85 * shade));
                }
                circle(window, (x, y), 1.6, tint.opacity(0.9 * shade), None);
            }
            Kind::Decision => {
                let tint = if node.conflict {
                    Hsla::from(colors.status_warning())
                } else {
                    Hsla::from(colors.graph_decision())
                };
                circle(
                    window,
                    (x, y),
                    radius + 4.0,
                    tint.opacity(0.08 * shade),
                    None,
                );
                circle(window, (x, y), radius, tint.opacity(0.95 * shade), None);
            }
            Kind::Rule => {
                let tint = Hsla::from(colors.status_info());
                let side = radius * 1.8;
                window.paint_quad(quad(
                    Bounds::new(
                        point(px(x - side / 2.0), px(y - side / 2.0)),
                        size(px(side), px(side)),
                    ),
                    px(1.0),
                    ground.opacity(shade),
                    px(1.25),
                    tint.opacity(0.9 * shade),
                    gpui::BorderStyle::Solid,
                ));
            }
        }
        let focused = self
            .focus
            .as_ref()
            .is_some_and(|(center, _)| *center == index);
        if self.selected == Some(index) || focused {
            self.paint_reticle(window, (x, y), radius, signal, self.selected == Some(index));
        }
    }

    /// Four corner brackets locking onto the node, turning slowly.
    fn paint_reticle(
        &self,
        window: &mut Window,
        center: (f32, f32),
        radius: f32,
        tint: Hsla,
        locked: bool,
    ) {
        let reach = radius + if locked { 9.0 } else { 7.0 };
        let turn = if self.motion { self.time * 0.6 } else { 0.0 };
        let arm = 0.38;
        for quarter in 0..4 {
            let mid =
                std::f32::consts::FRAC_PI_2 * quarter as f32 + std::f32::consts::FRAC_PI_4 + turn;
            let mut builder = PathBuilder::stroke(px(if locked { 1.5 } else { 1.0 }));
            let steps = 6;
            for step in 0..=steps {
                let angle = mid - arm / 2.0 + arm * step as f32 / steps as f32;
                let p = point(
                    px(center.0 + reach * angle.cos()),
                    px(center.1 + reach * angle.sin()),
                );
                if step == 0 {
                    builder.move_to(p);
                } else {
                    builder.line_to(p);
                }
            }
            if let Ok(path) = builder.build() {
                window.paint_path(path, tint.opacity(if locked { 1.0 } else { 0.6 }));
            }
        }
    }

    fn paint_label(
        &self,
        bounds: Bounds<Pixels>,
        index: usize,
        theme: &Theme,
        window: &mut Window,
        cx: &mut App,
    ) {
        let colors = theme.colors;
        let node = &self.nodes[index].0;
        let appear = self.appear(node);
        let in_focus = self
            .focus
            .as_ref()
            .is_some_and(|(_, set)| set.contains(&index));
        let major = matches!(node.kind, Kind::Component | Kind::Technology);
        // Small nodes name themselves only under the pointer (or when zoomed
        // far in): their neighbors are listed in the side card instead.
        let center = self
            .focus
            .as_ref()
            .is_some_and(|(center, _)| *center == index);
        let shown = if major {
            self.camera.scale >= 0.5 || in_focus
        } else {
            center || self.camera.scale >= DETAIL_SCALE
        };
        if !shown || appear < 0.6 {
            return;
        }
        let shade = node.shade * ((appear - 0.6) / 0.4).clamp(0.0, 1.0);
        let (x, y) = self.screen(bounds, node.pos);
        let zoom = self.camera.scale.clamp(0.6, 1.6);
        let mut top = y
            + node.radius() * zoom
            + if node.kind == Kind::Component {
                11.0
            } else {
                7.0
            };
        let (size_px, line_height, color) = match node.kind {
            Kind::Component => (11.5, 15.0, colors.text_primary()),
            Kind::Technology => (10.5, 14.0, colors.text_secondary()),
            _ => (10.5, 14.0, colors.text_secondary()),
        };
        let text = clipped(&node.label, LABEL_CHARS);
        let mut lines = vec![(text, size_px, line_height, Hsla::from(color).opacity(shade))];
        if node.kind == Kind::Component && self.camera.scale >= 0.7 {
            lines.push((
                weight_line(node.weight),
                9.5,
                13.0,
                // An empty component is not news: its line stays quiet.
                if node.weight == (0, 0) {
                    Hsla::from(colors.text_muted()).opacity(shade)
                } else {
                    Hsla::from(colors.graph_component()).opacity(0.7 * shade)
                },
            ));
        }
        for (text, size_px, line_height, color) in lines {
            let text: SharedString = text.into();
            let run = TextRun {
                len: text.len(),
                font: font(Theme::font_mono()),
                color,
                background_color: None,
                underline: None,
                strikethrough: None,
            };
            let line = window
                .text_system()
                .shape_line(text, px(size_px), &[run], None);
            let width = f32::from(line.width);
            window.paint_quad(quad(
                Bounds::new(
                    point(px(x - width / 2.0 - 3.0), px(top)),
                    size(px(width + 6.0), px(line_height)),
                ),
                px(2.0),
                Hsla::from(colors.canvas_deep()).opacity(0.72 * shade),
                px(0.0),
                gpui::transparent_black(),
                gpui::BorderStyle::Solid,
            ));
            let _ = line.paint(
                point(px(x - width / 2.0), px(top)),
                px(line_height),
                TextAlign::Left,
                None,
                window,
                cx,
            );
            top += line_height;
        }
    }
}

fn circle(
    window: &mut Window,
    center: (f32, f32),
    radius: f32,
    fill: impl Into<gpui::Background>,
    border: Option<(Hsla, f32)>,
) {
    let (border_color, border_width) = border.unwrap_or((gpui::transparent_black(), 0.0));
    window.paint_quad(quad(
        Bounds::new(
            point(px(center.0 - radius), px(center.1 - radius)),
            size(px(radius * 2.0), px(radius * 2.0)),
        ),
        px(radius),
        fill,
        px(border_width),
        border_color,
        gpui::BorderStyle::Solid,
    ));
}

fn swatch(theme: &Theme, node: &Node) -> Div {
    let colors = theme.colors;
    let mark = div().flex_none();
    let mark = match node.kind {
        Kind::Component => mark
            .size(px(10.0))
            .rounded_full()
            .border_1()
            .border_color(colors.graph_component())
            .bg(colors.graph_component().alpha(0.3)),
        Kind::Technology => mark
            .size(px(9.0))
            .rounded(px(2.5))
            .border_1()
            .border_color(colors.graph_technology())
            .bg(colors.graph_technology().alpha(0.3)),
        Kind::Decision => mark.size(px(7.0)).rounded_full().bg(if node.conflict {
            colors.status_warning()
        } else {
            colors.graph_decision()
        }),
        Kind::Rule => mark
            .size(px(8.0))
            .rounded(px(1.0))
            .border_1()
            .border_color(colors.status_info()),
    };
    div().w(px(12.0)).flex().justify_center().child(mark)
}

/// "2 decisões · 1 regra", or "sem decisões".
fn weight_line((decisions, rules): (usize, usize)) -> String {
    let mut parts = Vec::new();
    match decisions {
        0 => {}
        1 => parts.push("1 decisão".to_owned()),
        n => parts.push(format!("{n} decisões")),
    }
    match rules {
        0 => {}
        1 => parts.push("1 regra".to_owned()),
        n => parts.push(format!("{n} regras")),
    }
    if parts.is_empty() {
        "sem decisões".to_owned()
    } else {
        parts.join(" · ")
    }
}

fn card_label(theme: &Theme, label: &'static str) -> Div {
    text_style(div(), TypeScale::META)
        .text_color(theme.colors.text_muted())
        .child(label)
}

fn tie_label(kind: &LinkKind, outgoing: bool) -> &'static str {
    match (kind, outgoing) {
        (LinkKind::PartOf, true) => "faz parte de",
        (LinkKind::PartOf, false) => "parte deste componente",
        (LinkKind::Affects, true) => "muda este componente",
        (LinkKind::Affects, false) => "decisão que muda este item",
        (LinkKind::Uses, true) => "usa esta tecnologia",
        (LinkKind::Uses, false) => "decisão que usa esta tecnologia",
        (LinkKind::AppliesTo, true) => "vale para",
        (LinkKind::AppliesTo, false) => "regra que vale aqui",
        (LinkKind::Conflict, _) => "em conflito",
        (LinkKind::Relation, true) => "depende de",
        (LinkKind::Relation, false) => "depende desta",
        (LinkKind::Suggested(_), _) => "sugerida",
    }
}

fn kind_order(kind: Kind) -> i32 {
    match kind {
        Kind::Component => 0,
        Kind::Technology => 1,
        Kind::Decision => 2,
        Kind::Rule => 3,
    }
}

// ---- layout -----------------------------------------------------------------

fn build(graph: &ProjectGraph) -> (Vec<Node>, Vec<Link>) {
    // A decision or rule tied to nothing has no place on the drawing: a dot
    // floating alone reads as a glitch. It stays in the lists and counts.
    let tied: BTreeSet<&NodeRef> = graph
        .edges
        .iter()
        .chain(graph.suggested.iter().map(|(_, edge)| edge))
        .flat_map(|edge| [&edge.from, &edge.to])
        .collect();
    let mut nodes: Vec<Node> = graph
        .nodes
        .iter()
        .filter(|row| row.summary.node.kind == NodeKind::Entity || tied.contains(&row.summary.node))
        .map(|row| {
            let summary = &row.summary;
            let kind = match summary.node.kind {
                NodeKind::Decision => Kind::Decision,
                NodeKind::Claim => Kind::Rule,
                NodeKind::Entity if summary.detail == EntityKind::Technology.as_str() => {
                    Kind::Technology
                }
                NodeKind::Entity => Kind::Component,
            };
            let detail = match kind {
                Kind::Rule => rule_kind(&summary.detail).to_owned(),
                Kind::Decision => summary.detail.clone(),
                _ => String::new(),
            };
            Node {
                node: summary.node.clone(),
                kind,
                label: summary.label.clone(),
                detail,
                cluster: None,
                degree: 0,
                weight: (0, 0),
                conflict: false,
                pos: (0.0, 0.0),
                vel: (0.0, 0.0),
                shade: 1.0,
                stagger: 0.0,
            }
        })
        .collect();
    let index: BTreeMap<NodeRef, usize> = nodes
        .iter()
        .enumerate()
        .map(|(position, node)| (node.node.clone(), position))
        .collect();
    let mut links = Vec::new();
    for edge in &graph.edges {
        let (Some(&a), Some(&b)) = (index.get(&edge.from), index.get(&edge.to)) else {
            continue;
        };
        let kind = match edge.kind.as_str() {
            "part_of" => LinkKind::PartOf,
            "affects" => LinkKind::Affects,
            "uses" => LinkKind::Uses,
            "applies_to" => LinkKind::AppliesTo,
            "conflicts_with" => LinkKind::Conflict,
            _ => LinkKind::Relation,
        };
        if kind == LinkKind::Conflict {
            nodes[a].conflict = true;
            nodes[b].conflict = true;
        }
        links.push(Link { a, b, kind });
    }
    for (edge_id, edge) in &graph.suggested {
        let (Some(&a), Some(&b)) = (index.get(&edge.from), index.get(&edge.to)) else {
            continue;
        };
        links.push(Link {
            a,
            b,
            kind: LinkKind::Suggested(edge_id.clone()),
        });
    }
    for link in &links {
        nodes[link.a].degree += 1;
        nodes[link.b].degree += 1;
        match link.kind {
            LinkKind::Affects | LinkKind::Uses => nodes[link.b].weight.0 += 1,
            LinkKind::AppliesTo => nodes[link.b].weight.1 += 1,
            _ => {}
        }
    }

    // Clusters: each component climbs `part_of` to its top; decisions and
    // rules join the island of the first component they are tied to.
    let parent: BTreeMap<usize, usize> = links
        .iter()
        .filter(|link| link.kind == LinkKind::PartOf)
        .map(|link| (link.a, link.b))
        .collect();
    let top = |mut at: usize| {
        for _ in 0..16 {
            match parent.get(&at) {
                Some(next) => at = *next,
                None => break,
            }
        }
        at
    };
    for (position, node) in nodes.iter_mut().enumerate() {
        if node.kind == Kind::Component {
            node.cluster = Some(top(position));
        }
    }
    for link in &links {
        if matches!(link.kind, LinkKind::Affects | LinkKind::AppliesTo) {
            let target = nodes[link.b].cluster;
            let source = &mut nodes[link.a];
            if source.cluster.is_none() {
                source.cluster = target;
            }
        }
    }
    (nodes, links)
}

fn rule_kind(literal: &str) -> &'static str {
    match literal {
        "assumption" => "Premissa",
        "constraint" => "Restrição",
        "goal" => "Objetivo",
        "convention" => "Convenção",
        _ => "Regra",
    }
}

/// Deterministic seed in 0..1 for an id.
fn seed(id: &str, salt: u64) -> f32 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325 ^ salt;
    for byte in id.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    (hash >> 11) as f32 / (1u64 << 53) as f32
}

/// Initial places: islands on a ring, members around their island, the
/// rest on an outer ring; nodes already on screen keep theirs. Returns
/// whether most nodes were known (a refresh, not a first load).
fn place(nodes: &mut [Node], links: &[Link], previous: &BTreeMap<NodeRef, (f32, f32)>) -> bool {
    let mut islands: Vec<usize> = nodes
        .iter()
        .enumerate()
        .filter(|(position, node)| node.cluster == Some(*position))
        .map(|(position, _)| position)
        .collect();
    islands.sort_by(|a, b| {
        nodes[*a]
            .label
            .to_lowercase()
            .cmp(&nodes[*b].label.to_lowercase())
    });
    let ring = 120.0 + 70.0 * (islands.len() as f32).sqrt();
    let mut centers: BTreeMap<usize, (f32, f32)> = BTreeMap::new();
    for (order, island) in islands.iter().enumerate() {
        let angle = order as f32 / islands.len().max(1) as f32 * std::f32::consts::TAU - 1.2;
        let reach = if islands.len() == 1 { 0.0 } else { ring };
        centers.insert(*island, (angle.cos() * reach, angle.sin() * reach));
    }
    let mut known = 0;
    for node in nodes.iter_mut() {
        if let Some(pos) = previous.get(&node.node) {
            node.pos = *pos;
            known += 1;
            continue;
        }
        let angle = seed(&node.node.id, 1) * std::f32::consts::TAU;
        let spread = 30.0 + seed(&node.node.id, 2) * 60.0;
        node.pos = match node.cluster.and_then(|cluster| centers.get(&cluster)) {
            Some(center) => (
                center.0 + angle.cos() * spread,
                center.1 + angle.sin() * spread,
            ),
            None => {
                let outer = ring + 170.0 + seed(&node.node.id, 3) * 60.0;
                (angle.cos() * outer, angle.sin() * outer)
            }
        };
    }
    // New nodes of a refresh start beside a known neighbor.
    if known > 0 {
        for link in links {
            for (from, to) in [(link.a, link.b), (link.b, link.a)] {
                if !previous.contains_key(&nodes[from].node)
                    && previous.contains_key(&nodes[to].node)
                {
                    let anchor = nodes[to].pos;
                    let angle = seed(&nodes[from].node.id, 4) * std::f32::consts::TAU;
                    nodes[from].pos =
                        (anchor.0 + angle.cos() * 40.0, anchor.1 + angle.sin() * 40.0);
                }
            }
        }
    }
    known * 2 > nodes.len()
}

/// Runs the simulation to rest.
fn settle(nodes: &mut [Node], links: &[Link], steps: usize, gentle: bool) {
    let mut heat: f32 = if gentle { 0.3 } else { 1.0 };
    for _ in 0..steps {
        tick(nodes, links, heat);
        heat = (heat * 0.988).max(0.02);
    }
    for node in nodes.iter_mut() {
        node.vel = (0.0, 0.0);
    }
}

/// One step: repulsion, springs, island cohesion, gentle centering and
/// collision.
fn tick(nodes: &mut [Node], links: &[Link], heat: f32) {
    let count = nodes.len();
    for i in 0..count {
        for j in (i + 1)..count {
            let (dx, dy) = (
                nodes[j].pos.0 - nodes[i].pos.0,
                nodes[j].pos.1 - nodes[i].pos.1,
            );
            let mut d2 = dx * dx + dy * dy;
            if d2 < 0.01 {
                d2 = 0.01;
            }
            let distance = d2.sqrt();
            let strength = (nodes[i].charge() + nodes[j].charge()) * 0.5 / d2 * heat;
            let (ux, uy) = (dx / distance, dy / distance);
            nodes[i].vel.0 -= ux * strength;
            nodes[i].vel.1 -= uy * strength;
            nodes[j].vel.0 += ux * strength;
            nodes[j].vel.1 += uy * strength;
            // Collision keeps shapes and labels from piling up.
            let room = nodes[i].radius() + nodes[j].radius() + 14.0;
            if distance < room {
                let push = (room - distance) * 0.25;
                nodes[i].pos.0 -= ux * push;
                nodes[i].pos.1 -= uy * push;
                nodes[j].pos.0 += ux * push;
                nodes[j].pos.1 += uy * push;
            }
        }
    }
    for link in links {
        let (a, b) = (link.a, link.b);
        let (dx, dy) = (
            nodes[b].pos.0 - nodes[a].pos.0,
            nodes[b].pos.1 - nodes[a].pos.1,
        );
        let distance = (dx * dx + dy * dy).sqrt().max(0.1);
        let pull = (distance - link.rest()) * 0.06 * heat;
        let (ux, uy) = (dx / distance, dy / distance);
        nodes[a].vel.0 += ux * pull;
        nodes[a].vel.1 += uy * pull;
        nodes[b].vel.0 -= ux * pull;
        nodes[b].vel.1 -= uy * pull;
    }
    // Islands keep apart: tops repel within a wide reach.
    let tops: Vec<usize> = (0..count)
        .filter(|index| nodes[*index].cluster == Some(*index))
        .collect();
    for (order, &i) in tops.iter().enumerate() {
        for &j in tops.iter().skip(order + 1) {
            let (dx, dy) = (
                nodes[j].pos.0 - nodes[i].pos.0,
                nodes[j].pos.1 - nodes[i].pos.1,
            );
            let distance = (dx * dx + dy * dy).sqrt().max(0.1);
            let reach = 230.0;
            if distance < reach {
                let push = (reach - distance) * 0.05 * heat.max(0.2);
                let (ux, uy) = (dx / distance, dy / distance);
                nodes[i].vel.0 -= ux * push;
                nodes[i].vel.1 -= uy * push;
                nodes[j].vel.0 += ux * push;
                nodes[j].vel.1 += uy * push;
            }
        }
    }
    let anchors: Vec<Option<(f32, f32)>> = nodes
        .iter()
        .map(|node| node.cluster.map(|cluster| nodes[cluster].pos))
        .collect();
    for (node, anchor) in nodes.iter_mut().zip(anchors) {
        if let Some(anchor) = anchor {
            node.vel.0 += (anchor.0 - node.pos.0) * 0.012 * heat;
            node.vel.1 += (anchor.1 - node.pos.1) * 0.012 * heat;
        }
        // Gravity keeps islands and loose technologies near the middle, so
        // the map reads as one picture instead of scattered parts.
        node.vel.0 -= node.pos.0 * 0.011 * heat;
        node.vel.1 -= node.pos.1 * 0.011 * heat;
        node.pos.0 += node.vel.0;
        node.pos.1 += node.vel.1;
        node.vel.0 *= 0.55;
        node.vel.1 *= 0.55;
    }
}

/// Entrance order: islands first, from the center out.
fn stagger(nodes: &mut [Node]) {
    let far = nodes
        .iter()
        .map(|node| (node.pos.0.powi(2) + node.pos.1.powi(2)).sqrt())
        .fold(1.0_f32, f32::max);
    for node in nodes.iter_mut() {
        let distance = (node.pos.0.powi(2) + node.pos.1.powi(2)).sqrt() / far;
        let lead = if node.kind == Kind::Component {
            0.0
        } else {
            0.25
        };
        node.stagger = (distance * 0.75 + lead).min(1.0);
    }
}

// ---- math -------------------------------------------------------------------

fn lerp(from: f32, to: f32, amount: f32) -> f32 {
    from + (to - from) * amount
}

fn ease_out_cubic(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(3)
}

fn ease_out_back(t: f32) -> f32 {
    let c1 = 1.4;
    let c3 = c1 + 1.0;
    1.0 + c3 * (t - 1.0).powi(3) + c1 * (t - 1.0).powi(2)
}

fn distance(a: Point<Pixels>, b: Point<Pixels>) -> f32 {
    let (dx, dy) = (f32::from(a.x - b.x), f32::from(a.y - b.y));
    (dx * dx + dy * dy).sqrt()
}

/// Control point of a gentle bend, always to the same side.
fn bend(start: (f32, f32), end: (f32, f32)) -> (f32, f32) {
    let (mx, my) = ((start.0 + end.0) / 2.0, (start.1 + end.1) / 2.0);
    (mx + (end.1 - start.1) * 0.12, my - (end.0 - start.0) * 0.12)
}

fn quad_point(start: (f32, f32), control: (f32, f32), end: (f32, f32), t: f32) -> (f32, f32) {
    let u = 1.0 - t;
    (
        u * u * start.0 + 2.0 * u * t * control.0 + t * t * end.0,
        u * u * start.1 + 2.0 * u * t * control.1 + t * t * end.1,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use application::graph::{GraphEdge, GraphNode, NodeSummary};

    fn node(kind: NodeKind, id: &str, label: &str, detail: &str) -> GraphNode {
        GraphNode {
            summary: NodeSummary {
                node: NodeRef {
                    kind,
                    id: id.into(),
                },
                label: label.into(),
                detail: detail.into(),
                at: String::new(),
            },
            depth: 0,
            active: true,
        }
    }

    fn edge(from: NodeRef, to: NodeRef, kind: &str) -> GraphEdge {
        GraphEdge {
            from,
            to,
            kind: kind.into(),
        }
    }

    fn sample() -> ProjectGraph {
        ProjectGraph {
            nodes: vec![
                node(NodeKind::Entity, "app", "application", "component"),
                node(NodeKind::Entity, "jobs", "jobs", "component"),
                node(NodeKind::Entity, "sql", "SQLite", "technology"),
                node(NodeKind::Decision, "d1", "Fila local?", "Sim"),
                node(NodeKind::Claim, "r1", "Erros em português", "convention"),
            ],
            edges: vec![
                edge(NodeRef::entity("jobs"), NodeRef::entity("app"), "part_of"),
                edge(NodeRef::decision("d1"), NodeRef::entity("jobs"), "affects"),
                edge(NodeRef::decision("d1"), NodeRef::entity("sql"), "uses"),
                edge(NodeRef::claim("r1"), NodeRef::entity("app"), "applies_to"),
            ],
            suggested: vec![(
                "edge-9".into(),
                edge(NodeRef::claim("r1"), NodeRef::entity("jobs"), "applies_to"),
            )],
        }
    }

    #[test]
    fn islands_follow_part_of_and_ties() {
        let (nodes, links) = build(&sample());
        let app = 0;
        assert_eq!(
            nodes[1].cluster,
            Some(app),
            "a part joins its top component"
        );
        assert_eq!(
            nodes[3].cluster,
            Some(app),
            "a decision joins the island it changes"
        );
        assert_eq!(nodes[2].kind, Kind::Technology);
        assert_eq!(nodes[2].cluster, None, "technologies float between islands");
        assert_eq!(nodes[4].detail, "Convenção");
        assert!(links
            .iter()
            .any(|link| link.kind == LinkKind::Suggested("edge-9".into())));
    }

    #[test]
    fn the_same_map_always_settles_the_same_way_without_overlap() {
        let lay = || {
            let (mut nodes, links) = build(&sample());
            place(&mut nodes, &links, &BTreeMap::new());
            settle(&mut nodes, &links, SETTLE_STEPS, false);
            nodes
        };
        let (first, second) = (lay(), lay());
        for (a, b) in first.iter().zip(&second) {
            assert_eq!(a.pos, b.pos, "deterministic");
            assert!(a.pos.0.is_finite() && a.pos.1.is_finite());
        }
        for (i, a) in first.iter().enumerate() {
            for b in first.iter().skip(i + 1) {
                let gap = ((a.pos.0 - b.pos.0).powi(2) + (a.pos.1 - b.pos.1).powi(2)).sqrt();
                assert!(
                    gap > a.radius() + b.radius(),
                    "{} over {}",
                    a.label,
                    b.label
                );
            }
        }
    }

    #[test]
    fn a_refresh_keeps_known_nodes_in_place() {
        let (mut nodes, links) = build(&sample());
        place(&mut nodes, &links, &BTreeMap::new());
        settle(&mut nodes, &links, SETTLE_STEPS, false);
        let before: BTreeMap<NodeRef, (f32, f32)> = nodes
            .iter()
            .map(|node| (node.node.clone(), node.pos))
            .collect();
        let (mut again, links) = build(&sample());
        assert!(place(&mut again, &links, &before));
        settle(&mut again, &links, SETTLE_STEPS / 4, true);
        for node in &again {
            let old = before[&node.node];
            let moved = ((node.pos.0 - old.0).powi(2) + (node.pos.1 - old.1).powi(2)).sqrt();
            assert!(moved < 40.0, "{} moved {moved}", node.label);
        }
    }
}
