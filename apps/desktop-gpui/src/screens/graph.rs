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
//!
//! It stays light at any size (the project's pillar is performance). A
//! component with more decisions or rules than the drawing can show keeps a
//! handful and folds the rest into one group node ("+590 decisões") that
//! opens the component's page, where the full lists live; counts stay true.
//! Painting only visits what is on screen and drops to a cheap mode (straight
//! single-stroke links, no signal, flat nodes) when many links are in view.

use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
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
/// Decisions and rules the whole drawing keeps as individual nodes, shared
/// among the components that have any; the rest fold into group nodes.
const LEAF_BUDGET: usize = 260;
/// Fewest and most leaves one component keeps before folding.
const MIN_LEAVES: usize = 2;
const MAX_LEAVES: usize = 10;
/// Folding fewer than this many is not worth a group node.
const FOLD_SLACK: usize = 3;
/// Above this many links in view, painting drops to the cheap mode.
const CROWDED_LINKS: usize = 140;
/// Below this zoom (and above [`PANORAMA_FROM`] nodes) the map is a
/// panorama: components, technologies and groups only; single decisions and
/// rules come back when the camera comes closer. It leaves again above
/// `PANORAMA_BELOW + PANORAMA_BAND`, so a zoom resting at the edge does not
/// make the picture flicker.
const PANORAMA_BELOW: f32 = 0.45;
const PANORAMA_BAND: f32 = 0.1;
/// Smallest map that ever becomes a panorama.
const PANORAMA_FROM: usize = 150;
/// Beyond the viewport, how far a node may sit and still be painted.
const CULL_MARGIN: f32 = 48.0;
/// Width of the side card.
const CARD_WIDTH: f32 = 300.0;

/// How much of the painting work is spared; `lod` is the app's. The others
/// exist to measure what each saving is worth (`XEMNAS_GRAPH_RENDER`):
/// `full` visits every node and link with all effects, `cull` adds the
/// viewport cut, `cheap` adds the cheap mode for crowded views, `lod` adds the
/// panorama on top.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RenderMode {
    Full,
    Cull,
    Cheap,
    Lod,
}

fn render_mode() -> RenderMode {
    static MODE: std::sync::OnceLock<RenderMode> = std::sync::OnceLock::new();
    *MODE.get_or_init(|| match std::env::var("XEMNAS_GRAPH_RENDER").as_deref() {
        Ok("full") => RenderMode::Full,
        Ok("cull") => RenderMode::Cull,
        Ok("cheap") => RenderMode::Cheap,
        _ => RenderMode::Lod,
    })
}

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
    label: SharedString,
    detail: SharedString,
    /// For a group node, how many decisions or rules it stands for.
    folded: usize,
    /// For a group node, the component whose page lists them.
    home: Option<SharedString>,
    /// Index of the top-level component it gathers around.
    cluster: Option<usize>,
    degree: usize,
    /// Decisions and rules tied to it (components and technologies).
    weight: (usize, usize),
    conflict: bool,
    /// Tied to a suggested link: kept visible at every scale.
    suggested: bool,
    pos: (f32, f32),
    vel: (f32, f32),
    /// Displayed opacity, eased toward the focus target every frame.
    shade: f32,
    /// Order of appearance in the entrance, 0..1.
    stagger: f32,
}

impl Node {
    /// What the node is, for the card and screen readers.
    fn caption(&self) -> &'static str {
        match (self.folded > 0, self.kind) {
            (true, Kind::Rule) => "Regras agrupadas",
            (true, _) => "Decisões agrupadas",
            _ => self.kind.label(),
        }
    }

    fn radius(&self) -> f32 {
        if self.folded > 0 {
            return (6.0 + (self.folded as f32).ln() * 1.7).min(14.0);
        }
        match self.kind {
            Kind::Component => 11.0 + (self.degree.min(12) as f32) * 1.3,
            Kind::Technology => 8.5,
            Kind::Decision => 3.6,
            Kind::Rule => 4.2,
        }
    }

    fn charge(&self) -> f32 {
        if self.folded > 0 {
            return 1100.0;
        }
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
    /// Layouts started; a result is installed only if it is the latest.
    generation: u64,
    /// The newest generation, readable from the layout thread: a layout
    /// whose number is behind it stops between steps.
    latest: Arc<AtomicU64>,
    /// Whether a layout is still being computed.
    pending: bool,
    /// Whether the map is drawn as a panorama (see [`PANORAMA_BELOW`]).
    panorama: bool,
    /// A node to select once the layout in progress is installed (demo).
    select_later: Option<String>,
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
            generation: 0,
            latest: Arc::new(AtomicU64::new(0)),
            pending: false,
            panorama: false,
            select_later: None,
            focus: cx.focus_handle().tab_stop(true),
            buttons: BTreeMap::new(),
        }
    }

    /// Replaces the data, keeping the place of nodes already shown; only a
    /// first load (or a new project) plays the entrance.
    ///
    /// The force layout is the heaviest thing the app computes (hundreds of
    /// milliseconds at a few hundred nodes, seconds at a thousand), so it
    /// runs off the interface thread and is installed when ready; a newer
    /// call supersedes an older one still running.
    pub fn set_graph(&mut self, graph: &ProjectGraph, fresh: bool, cx: &mut Context<Self>) {
        self.generation += 1;
        self.latest.store(self.generation, Ordering::Relaxed);
        self.pending = true;
        let generation = self.generation;
        let latest = Arc::clone(&self.latest);
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
        let graph = graph.clone();
        cx.spawn(async move |this, cx| {
            let Some((nodes, links)) = cx
                .background_executor()
                .spawn(async move {
                    let _probe = crate::ui::perf::Probe::start("graph-layout");
                    let (mut nodes, links) = build(&graph);
                    let known = place(&mut nodes, &links, &previous);
                    let finished = settle_with(
                        solver_for(nodes.len()),
                        &mut nodes,
                        &links,
                        if known {
                            SETTLE_STEPS / 4
                        } else {
                            SETTLE_STEPS
                        },
                        known,
                        &|| latest.load(Ordering::Relaxed) != generation,
                    );
                    stagger(&mut nodes);
                    finished.then_some((nodes, links))
                })
                .await
            else {
                // A newer layout (or another project) took over mid-way.
                return;
            };
            let _ = this.update(cx, |canvas, cx| {
                if canvas.generation != generation {
                    return;
                }
                canvas.pending = false;
                canvas.nodes = nodes;
                canvas.links = links;
                canvas.hover = None;
                canvas.selected =
                    selected.and_then(|node| canvas.nodes.iter().position(|row| row.node == node));
                if let Some(label) = canvas.select_later.take() {
                    canvas.selected = canvas.nodes.iter().position(|node| node.label == label);
                }
                if fresh || !canvas.fitted {
                    canvas.born = Instant::now();
                    canvas.fitted = false;
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Selects the node named `label`, so demo captures reach the focus
    /// state without input.
    pub fn select_named(&mut self, label: &str, cx: &mut Context<Self>) {
        let found = self.nodes.iter().position(|node| node.label == label);
        if found.is_some() {
            self.selected = found;
            cx.notify();
        } else if self.pending {
            self.select_later = Some(label.to_owned());
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
            let links = std::mem::take(&mut self.links);
            tick(&mut self.nodes, &links, self.heat);
            self.links = links;
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
        let count = |kind: Kind| {
            self.nodes
                .iter()
                .filter(|node| node.kind == kind)
                .map(|node| node.folded.max(1))
                .sum::<usize>()
        };
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
        // A quiet plate behind it: on a crowded map the nodes would show
        // through the words.
        div()
            .flex()
            .flex_wrap()
            .gap_x(px(SpacingScale::S3))
            .gap_y(px(SpacingScale::S1))
            .px(px(SpacingScale::S3))
            .py(px(SpacingScale::S2))
            .rounded(theme.radius.control())
            .border_1()
            .border_color(colors.hairline_divider())
            .bg(colors.canvas_deep().alpha(0.82))
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
                    .size(px(10.0))
                    .rounded_full()
                    .border_1()
                    .border_color(colors.graph_decision())
                    .child(
                        div()
                            .m(px(2.5))
                            .size(px(3.0))
                            .rounded_full()
                            .bg(colors.graph_decision()),
                    ),
                "Agrupadas",
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
                .aria_label(format!("{}: {}", other_node.caption(), other_node.label))
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

        let (open_label, open_event) = match (&node.home, node.kind) {
            (Some(home), _) => (
                Some("Ver todas no Mapa"),
                Some(GraphEvent::OpenEntity(home.to_string())),
            ),
            (_, Kind::Component | Kind::Technology) => (
                Some("Abrir no Mapa"),
                Some(GraphEvent::OpenEntity(node.node.id.clone())),
            ),
            (_, Kind::Decision) => (
                Some("Abrir em Decisões"),
                Some(GraphEvent::OpenDecision(node.node.id.clone())),
            ),
            (_, Kind::Rule) => (None, None),
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
                                                format!("{} · em conflito", node.caption())
                                            } else {
                                                node.caption().to_owned()
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
        let leave_above = if self.panorama {
            PANORAMA_BELOW + PANORAMA_BAND
        } else {
            PANORAMA_BELOW
        };
        self.panorama = self.nodes.len() > PANORAMA_FROM && self.camera.scale < leave_above;
        let omitted = if self.panorama && render_mode() == RenderMode::Lod {
            self.nodes
                .iter()
                .enumerate()
                .filter(|(index, node)| self.visible(*index) && leaf_hidden_in_panorama(node))
                .count()
        } else {
            0
        };
        let bloom = if reduce_motion {
            1.0
        } else {
            (self.born.elapsed().as_secs_f32() / BLOOM_SECONDS).min(1.0)
        };
        let focus = self.focus_set();
        let motion = !reduce_motion;
        if busy || bloom < 1.0 || !self.fitted {
            // Settling, the entrance and camera moves: every frame.
            window.request_animation_frame();
        } else if motion && self.links.len() <= CROWDED_LINKS {
            // Only the ambient signal moves: the shared 30 Hz clock, which
            // parks as soon as the graph leaves the screen.
            crate::ui::motion::clock::phase(
                std::time::Duration::from_secs(1),
                cx.entity_id(),
                crate::ui::motion::clock::Rate::Smooth,
                cx,
            );
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
            crowded: Cell::new(false),
            panorama: self.panorama && render_mode() == RenderMode::Lod,
            mode: render_mode(),
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
        // The hint sits in the header's own row and wraps under the filters
        // when the canvas is narrow, instead of lying over them.
        let hint = (self.selected.is_none()).then(|| {
            text_style(div(), TypeScale::META)
                .flex_none()
                .pt(px(5.0))
                .text_color(colors.text_muted())
                .child(if omitted > 0 {
                    format!("Panorama · {omitted} decisões e regras ocultas · aproxime para vê-las")
                } else {
                    "Clique para focar · arraste para mover · role para aproximar".to_owned()
                })
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
                .map(|n| n.folded.max(1))
                .sum::<usize>(),
            self.nodes
                .iter()
                .filter(|n| n.kind == Kind::Rule)
                .map(|n| n.folded.max(1))
                .sum::<usize>(),
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
                    .right(px(if self.selected.is_some() {
                        CARD_WIDTH + SpacingScale::S4 * 2.0
                    } else {
                        SpacingScale::S4
                    }))
                    .flex()
                    .flex_wrap()
                    .items_start()
                    .justify_between()
                    .gap_x(px(SpacingScale::S4))
                    .gap_y(px(SpacingScale::S2))
                    .child(chips)
                    .children(hint),
            )
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
    /// Set at paint: many links in view, so the cheap mode is on.
    crowded: Cell<bool>,
    /// Zoomed out on a big map: single decisions and rules stay out.
    panorama: bool,
    mode: RenderMode,
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
            // Only what is on screen is visited: at hundreds of nodes the
            // frame's cost follows the viewport, not the project.
            let _probe = crate::ui::perf::Probe::start("graph-paint");
            let live = self.on_screen(bounds);
            let drawn = self.links_on_screen(bounds, &live);
            self.crowded.set(
                matches!(self.mode, RenderMode::Cheap | RenderMode::Lod)
                    && drawn.len() > CROWDED_LINKS,
            );
            self.paint_grid(bounds, theme, window);
            self.paint_links(bounds, theme, window, &drawn);
            // Small first so components sit on top.
            let mut order: Vec<usize> =
                (0..self.nodes.len()).filter(|index| live[*index]).collect();
            order.sort_by_key(|index| -kind_order(self.nodes[*index].0.kind));
            for index in &order {
                self.paint_node(bounds, *index, theme, window);
            }
            for index in &order {
                self.paint_label(bounds, *index, theme, window, cx);
            }
        });
    }

    /// Which nodes are shown and inside the viewport (with a margin for
    /// their halo and label).
    fn on_screen(&self, bounds: Bounds<Pixels>) -> Vec<bool> {
        let (ox, oy) = (f32::from(bounds.origin.x), f32::from(bounds.origin.y));
        let (width, height) = (f32::from(bounds.size.width), f32::from(bounds.size.height));
        self.nodes
            .iter()
            .map(|(node, shown)| {
                if !shown || (self.panorama && leaf_hidden_in_panorama(node)) {
                    return false;
                }
                if self.mode == RenderMode::Full {
                    return true;
                }
                let (x, y) = self.screen(bounds, node.pos);
                x >= ox - CULL_MARGIN
                    && x <= ox + width + CULL_MARGIN
                    && y >= oy - CULL_MARGIN
                    && y <= oy + height + CULL_MARGIN
            })
            .collect()
    }

    /// The links whose box meets the viewport.
    fn links_on_screen(&self, bounds: Bounds<Pixels>, live: &[bool]) -> Vec<usize> {
        let (ox, oy) = (f32::from(bounds.origin.x), f32::from(bounds.origin.y));
        let (width, height) = (f32::from(bounds.size.width), f32::from(bounds.size.height));
        self.links
            .iter()
            .enumerate()
            .filter(|(_, link)| {
                if self.panorama
                    && (leaf_hidden_in_panorama(&self.nodes[link.a].0)
                        || leaf_hidden_in_panorama(&self.nodes[link.b].0))
                {
                    return false;
                }
                if self.mode == RenderMode::Full || live[link.a] || live[link.b] {
                    return true;
                }
                // Neither end is in view; a long link can still cross it.
                let (pa, pb) = (
                    self.screen(bounds, self.nodes[link.a].0.pos),
                    self.screen(bounds, self.nodes[link.b].0.pos),
                );
                pa.0.min(pb.0) <= ox + width
                    && pa.0.max(pb.0) >= ox
                    && pa.1.min(pb.1) <= oy + height
                    && pa.1.max(pb.1) >= oy
            })
            .map(|(number, _)| number)
            .collect()
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
    fn paint_links(
        &self,
        bounds: Bounds<Pixels>,
        theme: &Theme,
        window: &mut Window,
        drawn: &[usize],
    ) {
        let colors = theme.colors;
        let signal = Hsla::from(colors.graph_component());
        let focus_center = self.focus.as_ref().map(|(center, _)| *center);
        for &number in drawn {
            let link = &self.links[number];
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
            // In a crowd, a link is one straight hairline: the glow, the
            // curve, the dashes and the signal are what make a thousand of
            // them heavy, and none is readable at that density.
            let cheap = self.crowded.get() && !lit;
            let strokes: &[(f32, f32)] = if lit {
                &[(6.0, 0.06), (3.0, 0.14), (1.4, 1.0)]
            } else if cheap {
                &[(1.0, core * 0.7)]
            } else {
                &[(3.5, 0.035), (1.0, core)]
            };
            for &(width, alpha) in strokes {
                let mut builder = PathBuilder::stroke(px(width));
                if dashed && width < 2.0 && !cheap {
                    builder = builder.dash_array(&[px(3.0), px(4.0)]);
                }
                builder.move_to(point(px(start.0), px(start.1)));
                let segments = if cheap { 1 } else { 18 };
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
            if self.motion && grown >= 1.0 && shade > 0.3 && !cheap {
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
                // A lens: soft bloom, dark body, bright rim, an orbit and a
                // core. In a crowd only the body, rim and core are drawn.
                let flat = self.crowded.get();
                if !flat {
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
                }
                circle(
                    window,
                    (x, y),
                    radius,
                    ground.opacity(shade),
                    Some((signal.opacity(0.9 * shade), 1.25)),
                );
                if !flat {
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
                }
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
                if node.folded > 0 {
                    // A group: a ring around a core, bigger the more it holds.
                    circle(
                        window,
                        (x, y),
                        radius + 5.0,
                        tint.opacity(0.07 * shade),
                        None,
                    );
                    circle(
                        window,
                        (x, y),
                        radius,
                        ground.opacity(shade),
                        Some((tint.opacity(0.9 * shade), 1.5)),
                    );
                    circle(
                        window,
                        (x, y),
                        radius * 0.4,
                        tint.opacity(0.9 * shade),
                        None,
                    );
                } else {
                    circle(
                        window,
                        (x, y),
                        radius + 4.0,
                        tint.opacity(0.08 * shade),
                        None,
                    );
                    circle(window, (x, y), radius, tint.opacity(0.95 * shade), None);
                }
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
                if node.folded > 0 {
                    circle(
                        window,
                        (x, y),
                        radius * 0.4,
                        tint.opacity(0.9 * shade),
                        None,
                    );
                }
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
        let major = matches!(node.kind, Kind::Component | Kind::Technology) || node.folded > 0;
        // Small nodes name themselves only under the pointer (or when zoomed
        // far in): their neighbors are listed in the side card instead.
        let center = self
            .focus
            .as_ref()
            .is_some_and(|(center, _)| *center == index);
        let shown = if major {
            let floor = if self.crowded.get() { 0.9 } else { 0.5 };
            self.camera.scale >= floor || in_focus
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

/// Whether a node is left out of the panorama: a single decision or rule
/// with nothing that must be seen (no conflict, no suggestion). Groups,
/// components and technologies always stay.
fn leaf_hidden_in_panorama(node: &Node) -> bool {
    node.folded == 0
        && matches!(node.kind, Kind::Decision | Kind::Rule)
        && !node.conflict
        && !node.suggested
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
    let (dropped, crowds) = fold_crowds(graph);
    let tied: BTreeSet<&NodeRef> = graph
        .edges
        .iter()
        .chain(graph.suggested.iter().map(|(_, edge)| edge))
        .flat_map(|edge| [&edge.from, &edge.to])
        .collect();
    let mut nodes: Vec<Node> = graph
        .nodes
        .iter()
        .filter(|row| {
            !dropped.contains(&row.summary.node)
                && (row.summary.node.kind == NodeKind::Entity || tied.contains(&row.summary.node))
        })
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
            let detail: SharedString = match kind {
                Kind::Rule => rule_kind(&summary.detail).into(),
                Kind::Decision => summary.detail.clone().into(),
                _ => SharedString::default(),
            };
            Node {
                node: summary.node.clone(),
                kind,
                label: summary.label.clone().into(),
                detail,
                folded: 0,
                home: None,
                cluster: None,
                degree: 0,
                weight: (0, 0),
                conflict: false,
                suggested: false,
                pos: (0.0, 0.0),
                vel: (0.0, 0.0),
                shade: 1.0,
                stagger: 0.0,
            }
        })
        .collect();
    for crowd in &crowds {
        let rule = crowd.rule;
        nodes.push(Node {
            node: NodeRef {
                kind: if rule {
                    NodeKind::Claim
                } else {
                    NodeKind::Decision
                },
                id: format!(
                    "group:{}:{}",
                    crowd.home.id,
                    if rule { "rules" } else { "decisions" }
                ),
            },
            kind: if rule { Kind::Rule } else { Kind::Decision },
            label: format!(
                "+{} {}",
                crowd.count,
                if rule { "regras" } else { "decisões" }
            )
            .into(),
            detail: "Agrupadas para manter o grafo leve. Abra o componente para ver todas.".into(),
            folded: crowd.count,
            home: Some(crowd.home.id.clone().into()),
            cluster: None,
            degree: 0,
            weight: (0, 0),
            conflict: false,
            suggested: false,
            pos: (0.0, 0.0),
            vel: (0.0, 0.0),
            shade: 1.0,
            stagger: 0.0,
        });
    }
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
    // Each group hangs from its component like the leaves it folded.
    for crowd in &crowds {
        let group = NodeRef {
            kind: if crowd.rule {
                NodeKind::Claim
            } else {
                NodeKind::Decision
            },
            id: format!(
                "group:{}:{}",
                crowd.home.id,
                if crowd.rule { "rules" } else { "decisions" }
            ),
        };
        if let (Some(&a), Some(&b)) = (index.get(&group), index.get(&crowd.home)) {
            let kind = if crowd.rule {
                LinkKind::AppliesTo
            } else {
                LinkKind::Affects
            };
            links.push(Link { a, b, kind });
            // The folded ones still count toward what the component carries.
            if crowd.rule {
                nodes[b].weight.1 += crowd.count - 1;
            } else {
                nodes[b].weight.0 += crowd.count - 1;
            }
        }
    }
    for (edge_id, edge) in &graph.suggested {
        let (Some(&a), Some(&b)) = (index.get(&edge.from), index.get(&edge.to)) else {
            continue;
        };
        nodes[a].suggested = true;
        nodes[b].suggested = true;
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

/// Decisions or rules folded into one group node for a component.
struct Crowd {
    home: NodeRef,
    rule: bool,
    count: usize,
}

/// Finds the components with more decisions or rules than the drawing should
/// show. Each keeps a share of [`LEAF_BUDGET`] (always the same ones, in id
/// order) and the rest are folded: the returned set is dropped from the
/// drawing and a [`Crowd`] stands for them.
fn fold_crowds(graph: &ProjectGraph) -> (BTreeSet<NodeRef>, Vec<Crowd>) {
    let mut by_home: BTreeMap<(NodeRef, bool), Vec<NodeRef>> = BTreeMap::new();
    let mut seen = BTreeSet::new();
    for edge in &graph.edges {
        if !matches!(edge.kind.as_str(), "affects" | "applies_to")
            || edge.from.kind == NodeKind::Entity
            || edge.to.kind != NodeKind::Entity
            || !seen.insert(edge.from.clone())
        {
            continue;
        }
        by_home
            .entry((edge.to.clone(), edge.from.kind == NodeKind::Claim))
            .or_default()
            .push(edge.from.clone());
    }
    let cap = (LEAF_BUDGET / by_home.len().max(1)).clamp(MIN_LEAVES, MAX_LEAVES);
    let mut dropped = BTreeSet::new();
    let mut crowds = Vec::new();
    for ((home, rule), mut leaves) in by_home {
        if leaves.len() < cap + FOLD_SLACK {
            continue;
        }
        leaves.sort();
        let hidden = leaves.split_off(cap);
        crowds.push(Crowd {
            home,
            rule,
            count: hidden.len(),
        });
        dropped.extend(hidden);
    }
    (dropped, crowds)
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
/// How repulsion and collision between nodes are found. They differ only in
/// cost: the benchmark in the tests (`layout_benchmark`) compares them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(not(test), allow(dead_code))]
enum Solver {
    /// Every pair, every step. The reference: exact and quadratic.
    AllPairs,
    /// Bucketed in a grid; only neighbouring cells meet. Exact within reach.
    Grid,
    /// A quadtree gathers far groups into one force (Barnes-Hut); collision
    /// still looks only at close neighbours.
    BarnesHut,
}

/// Up to this many nodes the exact all-pairs solver is used: it is the
/// fastest there (the tree's bookkeeping costs more than it saves) and exact.
const EXACT_UP_TO: usize = 300;

/// The solver the app lays out with, by size. The release benchmark
/// (`layout_benchmark`, results in `docs/pesquisas/escalabilidade-renderizacao-fontes.md`)
/// had Barnes-Hut 3 to 6 times faster than the grid from 1 000 nodes up,
/// with the same quality; the grid was never the fastest.
fn solver_for(count: usize) -> Solver {
    if count <= EXACT_UP_TO {
        Solver::AllPairs
    } else {
        Solver::BarnesHut
    }
}

#[cfg(test)]
fn settle(nodes: &mut [Node], links: &[Link], steps: usize, gentle: bool) {
    settle_with(
        solver_for(nodes.len()),
        nodes,
        links,
        steps,
        gentle,
        &|| false,
    );
}

/// Settles with `solver`, giving up early when `cancelled` says so (checked
/// between steps: a newer layout, another project or a closed screen makes
/// the result worthless). Returns whether it ran to the end.
fn settle_with(
    solver: Solver,
    nodes: &mut [Node],
    links: &[Link],
    steps: usize,
    gentle: bool,
    cancelled: &dyn Fn() -> bool,
) -> bool {
    let mut heat: f32 = if gentle { 0.3 } else { 1.0 };
    // A big map gets fewer, proportionally cooler steps: the cost of a step
    // grows with the pairs of nodes, and the picture settles all the same.
    let planned = steps;
    let steps = if nodes.len() > FULL_STEPS_UP_TO {
        (steps * FULL_STEPS_UP_TO / nodes.len()).max(steps / 4)
    } else {
        steps
    };
    let cooling = 0.988_f32.powf(planned as f32 / steps.max(1) as f32);
    for _ in 0..steps {
        if cancelled() {
            return false;
        }
        tick_with(solver, nodes, links, heat);
        heat = (heat * cooling).max(0.02);
    }
    for node in nodes.iter_mut() {
        node.vel = (0.0, 0.0);
    }
    true
}

/// One step: repulsion, springs, island cohesion, gentle centering and
/// collision.
fn tick(nodes: &mut [Node], links: &[Link], heat: f32) {
    tick_with(solver_for(nodes.len()), nodes, links, heat);
}

fn tick_with(solver: Solver, nodes: &mut [Node], links: &[Link], heat: f32) {
    let count = nodes.len();
    match solver {
        Solver::AllPairs => repel_all_pairs(nodes, heat),
        Solver::Grid => repel_grid(nodes, heat),
        Solver::BarnesHut => {
            repel_barnes_hut(nodes, heat);
            collide_grid(nodes);
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
    let separate = |nodes: &mut [Node], i: usize, j: usize| {
        let (dx, dy) = (
            nodes[j].pos.0 - nodes[i].pos.0,
            nodes[j].pos.1 - nodes[i].pos.1,
        );
        let distance = (dx * dx + dy * dy).sqrt().max(0.1);
        let reach = ISLAND_REACH;
        if distance < reach {
            let push = (reach - distance) * 0.05 * heat.max(0.2);
            let (ux, uy) = (dx / distance, dy / distance);
            nodes[i].vel.0 -= ux * push;
            nodes[i].vel.1 -= uy * push;
            nodes[j].vel.0 += ux * push;
            nodes[j].vel.1 += uy * push;
        }
    };
    if solver == Solver::AllPairs {
        for (order, &i) in tops.iter().enumerate() {
            for &j in tops.iter().skip(order + 1) {
                separate(nodes, i, j);
            }
        }
    } else {
        // Same pairs (within reach), found through a grid of the reach's size.
        let cell = |pos: (f32, f32)| {
            let c = |v: f32| (v / ISLAND_REACH).floor().clamp(-100_000.0, 100_000.0) as i32;
            (c(pos.0), c(pos.1))
        };
        let mut grid: std::collections::HashMap<(i32, i32), Vec<usize>> =
            std::collections::HashMap::with_capacity(tops.len());
        for &top in &tops {
            grid.entry(cell(nodes[top].pos)).or_default().push(top);
        }
        let mut near: Vec<usize> = Vec::new();
        for &i in &tops {
            near.clear();
            let (cx, cy) = cell(nodes[i].pos);
            for gx in cx - 1..=cx + 1 {
                for gy in cy - 1..=cy + 1 {
                    if let Some(members) = grid.get(&(gx, gy)) {
                        near.extend(members.iter().copied().filter(|j| *j > i));
                    }
                }
            }
            near.sort_unstable();
            for &j in &near {
                separate(nodes, i, j);
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
        let speed = node.vel.0.hypot(node.vel.1);
        if speed > MAX_STEP {
            let scale = MAX_STEP / speed;
            node.vel.0 *= scale;
            node.vel.1 *= scale;
        }
        node.pos.0 += node.vel.0;
        node.pos.1 += node.vel.1;
        node.vel.0 *= 0.55;
        node.vel.1 *= 0.55;
    }
}

/// How far apart two islands' tops are kept.
const ISLAND_REACH: f32 = 230.0;

/// The push between one pair, and the collision that keeps shapes and
/// labels from piling up. Shared by the all-pairs and grid solvers.
fn pair_interaction(nodes: &mut [Node], i: usize, j: usize, heat: f32) {
    let (dx, dy) = (
        nodes[j].pos.0 - nodes[i].pos.0,
        nodes[j].pos.1 - nodes[i].pos.1,
    );
    let mut d2 = dx * dx + dy * dy;
    if d2 < 0.01 {
        d2 = 0.01;
    }
    let distance = d2.sqrt();
    let strength = ((nodes[i].charge() + nodes[j].charge()) * 0.5 / d2 * heat).min(MAX_PUSH);
    let (ux, uy) = (dx / distance, dy / distance);
    nodes[i].vel.0 -= ux * strength;
    nodes[i].vel.1 -= uy * strength;
    nodes[j].vel.0 += ux * strength;
    nodes[j].vel.1 += uy * strength;
    let room = nodes[i].radius() + nodes[j].radius() + 14.0;
    if distance < room {
        let push = (room - distance) * 0.25;
        nodes[i].pos.0 -= ux * push;
        nodes[i].pos.1 -= uy * push;
        nodes[j].pos.0 += ux * push;
        nodes[j].pos.1 += uy * push;
    }
}

fn repel_all_pairs(nodes: &mut [Node], heat: f32) {
    for i in 0..nodes.len() {
        for j in i + 1..nodes.len() {
            pair_interaction(nodes, i, j, heat);
        }
    }
}

#[cfg_attr(not(test), allow(dead_code))]
/// Repulsion fades with the square of the distance and nothing collides
/// beyond a few dozen pixels, so pairs farther apart than REACH are not
/// looked at: nodes are bucketed in a grid of that size and each one meets
/// only its own and the eight surrounding cells, in index order.
fn repel_grid(nodes: &mut [Node], heat: f32) {
    let count = nodes.len();
    let mut grid: std::collections::HashMap<(i32, i32), Vec<usize>> =
        std::collections::HashMap::with_capacity(count);
    for (index, node) in nodes.iter().enumerate() {
        grid.entry(cell_of(node.pos)).or_default().push(index);
    }
    let mut near: Vec<usize> = Vec::new();
    for i in 0..count {
        near.clear();
        let (cx, cy) = cell_of(nodes[i].pos);
        for gx in cx - 1..=cx + 1 {
            for gy in cy - 1..=cy + 1 {
                if let Some(cell) = grid.get(&(gx, gy)) {
                    near.extend(cell.iter().copied().filter(|j| *j > i));
                }
            }
        }
        near.sort_unstable();
        for &j in &near {
            pair_interaction(nodes, i, j, heat);
        }
    }
}

/// Side of the grid cell collision uses: the widest room two nodes can need
/// (two big components and the margin) fits in a neighbouring cell.
const COLLIDE_CELL: f32 = 80.0;

/// Collision only, through a fine grid (the repulsion of the Barnes-Hut
/// solver comes from the tree).
fn collide_grid(nodes: &mut [Node]) {
    let count = nodes.len();
    let cell = |pos: (f32, f32)| {
        let c = |v: f32| (v / COLLIDE_CELL).floor().clamp(-100_000.0, 100_000.0) as i32;
        (c(pos.0), c(pos.1))
    };
    let mut grid: std::collections::HashMap<(i32, i32), Vec<usize>> =
        std::collections::HashMap::with_capacity(count);
    for (index, node) in nodes.iter().enumerate() {
        grid.entry(cell(node.pos)).or_default().push(index);
    }
    let mut near: Vec<usize> = Vec::new();
    for i in 0..count {
        near.clear();
        let (cx, cy) = cell(nodes[i].pos);
        for gx in cx - 1..=cx + 1 {
            for gy in cy - 1..=cy + 1 {
                if let Some(members) = grid.get(&(gx, gy)) {
                    near.extend(members.iter().copied().filter(|j| *j > i));
                }
            }
        }
        near.sort_unstable();
        for &j in &near {
            let (dx, dy) = (
                nodes[j].pos.0 - nodes[i].pos.0,
                nodes[j].pos.1 - nodes[i].pos.1,
            );
            let distance = (dx * dx + dy * dy).sqrt().max(0.1);
            let room = nodes[i].radius() + nodes[j].radius() + 14.0;
            if distance < room {
                let (ux, uy) = (dx / distance, dy / distance);
                let push = (room - distance) * 0.25;
                nodes[i].pos.0 -= ux * push;
                nodes[i].pos.1 -= uy * push;
                nodes[j].pos.0 += ux * push;
                nodes[j].pos.1 += uy * push;
            }
        }
    }
}

/// Barnes-Hut opening angle: a group is one force when its size over its
/// distance is under this. Higher is faster and rougher.
const THETA: f32 = 0.9;

/// One square of the quadtree.
struct Quad {
    /// Centre and half side.
    cx: f32,
    cy: f32,
    half: f32,
    /// Summed charge, charge-weighted centre and how many nodes it holds.
    charge: f32,
    com: (f32, f32),
    count: usize,
    /// Children (north-west, north-east, south-west, south-east), or none.
    children: Option<[usize; 4]>,
    /// The nodes held while it is a leaf.
    members: Vec<usize>,
}

/// A leaf splits past this many nodes, unless it is too small to split.
const LEAF_CAPACITY: usize = 6;

fn build_quadtree(nodes: &[Node]) -> Vec<Quad> {
    let (mut min_x, mut min_y, mut max_x, mut max_y) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for node in nodes {
        min_x = min_x.min(node.pos.0);
        min_y = min_y.min(node.pos.1);
        max_x = max_x.max(node.pos.0);
        max_y = max_y.max(node.pos.1);
    }
    let half = ((max_x - min_x).max(max_y - min_y) / 2.0).max(1.0) + 1.0;
    let mut arena = vec![Quad {
        cx: (min_x + max_x) / 2.0,
        cy: (min_y + max_y) / 2.0,
        half,
        charge: 0.0,
        com: (0.0, 0.0),
        count: 0,
        children: None,
        members: Vec::new(),
    }];
    for (index, node) in nodes.iter().enumerate() {
        quad_insert(&mut arena, nodes, 0, index, node.pos, 0);
    }
    quad_summarise(&mut arena, nodes, 0);
    arena
}

fn quad_insert(
    arena: &mut Vec<Quad>,
    nodes: &[Node],
    at: usize,
    index: usize,
    pos: (f32, f32),
    depth: usize,
) {
    if let Some(children) = arena[at].children {
        let child = children[quadrant(&arena[at], pos)];
        quad_insert(arena, nodes, child, index, pos, depth + 1);
        return;
    }
    arena[at].members.push(index);
    if arena[at].members.len() <= LEAF_CAPACITY || depth >= 24 {
        return;
    }
    // Split: four children, and the members move down into them.
    let (cx, cy, half) = (arena[at].cx, arena[at].cy, arena[at].half / 2.0);
    let first = arena.len();
    for (dx, dy) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
        arena.push(Quad {
            cx: cx + dx * half,
            cy: cy + dy * half,
            half,
            charge: 0.0,
            com: (0.0, 0.0),
            count: 0,
            children: None,
            members: Vec::new(),
        });
    }
    arena[at].children = Some([first, first + 1, first + 2, first + 3]);
    let moved = std::mem::take(&mut arena[at].members);
    for member in moved {
        let pos = nodes[member].pos;
        let child = first + quadrant(&arena[at], pos);
        quad_insert(arena, nodes, child, member, pos, depth + 1);
    }
}

fn quadrant(quad: &Quad, pos: (f32, f32)) -> usize {
    usize::from(pos.0 >= quad.cx) + 2 * usize::from(pos.1 >= quad.cy)
}

fn quad_summarise(arena: &mut [Quad], nodes: &[Node], at: usize) {
    if let Some(children) = arena[at].children {
        let (mut charge, mut x, mut y, mut count) = (0.0_f32, 0.0_f32, 0.0_f32, 0);
        for child in children {
            quad_summarise(arena, nodes, child);
            let quad = &arena[child];
            charge += quad.charge;
            x += quad.com.0 * quad.charge;
            y += quad.com.1 * quad.charge;
            count += quad.count;
        }
        arena[at].charge = charge;
        arena[at].count = count;
        arena[at].com = if charge > 0.0 {
            (x / charge, y / charge)
        } else {
            (arena[at].cx, arena[at].cy)
        };
    } else {
        let (mut charge, mut x, mut y) = (0.0_f32, 0.0_f32, 0.0_f32);
        for &member in &arena[at].members {
            let node = &nodes[member];
            let q = node.charge();
            charge += q;
            x += node.pos.0 * q;
            y += node.pos.1 * q;
        }
        arena[at].charge = charge;
        arena[at].count = arena[at].members.len();
        arena[at].com = if charge > 0.0 {
            (x / charge, y / charge)
        } else {
            (arena[at].cx, arena[at].cy)
        };
    }
}

/// Repulsion by the tree: the same law and reach as the grid solver, with
/// groups far enough away (by [`THETA`]) counted as one body.
fn repel_barnes_hut(nodes: &mut [Node], heat: f32) {
    let arena = build_quadtree(nodes);
    let mut stack: Vec<usize> = Vec::new();
    for i in 0..nodes.len() {
        let (px, py) = nodes[i].pos;
        let charge = nodes[i].charge();
        let (mut fx, mut fy) = (0.0_f32, 0.0_f32);
        stack.clear();
        stack.push(0);
        while let Some(at) = stack.pop() {
            let quad = &arena[at];
            if quad.count == 0 {
                continue;
            }
            // Out of reach altogether: the box is farther than REACH.
            let gap_x = ((quad.cx - px).abs() - quad.half).max(0.0);
            let gap_y = ((quad.cy - py).abs() - quad.half).max(0.0);
            if gap_x * gap_x + gap_y * gap_y > REACH * REACH {
                continue;
            }
            let leaf = quad.children.is_none();
            if !leaf {
                let (dx, dy) = (quad.com.0 - px, quad.com.1 - py);
                let d2 = (dx * dx + dy * dy).max(0.01);
                let size = quad.half * 2.0;
                // A group is one body only when it is small for its
                // distance and well clear of anything that could collide.
                if size * size < THETA * THETA * d2 && d2.sqrt() > size + COLLIDE_CELL {
                    let distance = d2.sqrt();
                    let n = quad.count as f32;
                    let strength = ((n * charge + quad.charge) * 0.5 / d2 * heat).min(MAX_PUSH * n);
                    fx -= dx / distance * strength;
                    fy -= dy / distance * strength;
                    continue;
                }
                if let Some(children) = quad.children {
                    stack.extend(children);
                }
                continue;
            }
            for &j in &quad.members {
                if j == i {
                    continue;
                }
                let (dx, dy) = (nodes[j].pos.0 - px, nodes[j].pos.1 - py);
                let d2 = (dx * dx + dy * dy).max(0.01);
                let distance = d2.sqrt();
                let strength = ((charge + nodes[j].charge()) * 0.5 / d2 * heat).min(MAX_PUSH);
                fx -= dx / distance * strength;
                fy -= dy / distance * strength;
            }
        }
        nodes[i].vel.0 += fx;
        nodes[i].vel.1 += fy;
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

/// Up to this many nodes the layout takes all its steps.
const FULL_STEPS_UP_TO: usize = 200;
/// The strongest push one pair of nodes can give in a step. Without it, two
/// nodes born almost on the same point (a component with hundreds of
/// decisions) push each other with a force that grows without bound, and
/// the positions end up infinite.
const MAX_PUSH: f32 = 30.0;
/// The farthest a node moves in one step.
const MAX_STEP: f32 = 45.0;
/// Pairs of nodes farther apart than this do not repel each other.
const REACH: f32 = 420.0;

/// The grid cell of a position. Clamped, so a runaway position (or NaN)
/// can neither overflow the neighbour arithmetic nor panic.
fn cell_of(pos: (f32, f32)) -> (i32, i32) {
    let cell = |value: f32| (value / REACH).floor().clamp(-100_000.0, 100_000.0) as i32;
    (cell(pos.0), cell(pos.1))
}

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

    /// A long-lived project: tens of components, hundreds of decisions and
    /// rules. The layout must stay finite and bounded (the grid ignores far
    /// pairs, so a regression would show as nodes flying apart).
    fn large() -> ProjectGraph {
        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        for c in 0..40 {
            nodes.push(node(
                NodeKind::Entity,
                &format!("c{c}"),
                &format!("modulo-{c}"),
                "component",
            ));
            if c % 5 != 0 {
                edges.push(edge(
                    NodeRef::entity(format!("c{c}")),
                    NodeRef::entity(format!("c{}", c - c % 5)),
                    "part_of",
                ));
            }
        }
        for d in 0..600 {
            nodes.push(node(
                NodeKind::Decision,
                &format!("d{d}"),
                &format!("Decisão {d}"),
                "Sim",
            ));
            edges.push(edge(
                NodeRef::decision(format!("d{d}")),
                NodeRef::entity(format!("c{}", if d % 5 < 3 { 0 } else { d % 40 })),
                "affects",
            ));
        }
        for r in 0..200 {
            nodes.push(node(
                NodeKind::Claim,
                &format!("r{r}"),
                &format!("Regra {r}"),
                "convention",
            ));
            edges.push(edge(
                NodeRef::claim(format!("r{r}")),
                NodeRef::entity(format!("c{}", r % 40)),
                "applies_to",
            ));
        }
        ProjectGraph {
            nodes,
            edges,
            suggested: Vec::new(),
        }
    }

    #[test]
    fn crowded_components_fold_their_decisions_into_groups() {
        let (nodes, links) = build(&large());
        // 40 components, 600 decisions and 200 rules, but the drawing keeps
        // a bounded number of nodes.
        assert!(nodes.len() < 400, "{} nodes", nodes.len());
        let docs = nodes.iter().position(|n| n.label == "modulo-0").unwrap();
        let group = nodes
            .iter()
            .position(|n| {
                n.folded > 0 && n.kind == Kind::Decision && n.home.as_deref() == Some("c0")
            })
            .expect("the crowded component has a group");
        assert!(links.iter().any(|link| link.a == group && link.b == docs));
        // Nothing is lost from the counts: group plus leaves is every decision.
        let decisions: usize = nodes
            .iter()
            .filter(|n| n.kind == Kind::Decision)
            .map(|n| n.folded.max(1))
            .sum();
        assert_eq!(decisions, 600);
        assert!(nodes[docs].weight.0 >= 300);
    }

    #[test]
    fn a_large_map_settles_finite_and_bounded() {
        let (mut nodes, links) = build(&large());
        place(&mut nodes, &links, &BTreeMap::new());
        let started = std::time::Instant::now();
        settle(&mut nodes, &links, SETTLE_STEPS, false);
        eprintln!("settled {} nodes in {:?}", nodes.len(), started.elapsed());
        for node in &nodes {
            assert!(
                node.pos.0.is_finite() && node.pos.1.is_finite(),
                "{}",
                node.label
            );
            assert!(
                node.pos.0.abs() < 20_000.0 && node.pos.1.abs() < 20_000.0,
                "{}: {:?}",
                node.label,
                node.pos
            );
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

    // ---- layout benchmark ------------------------------------------------

    /// A synthetic map straight at the layout level (no `build`): islands of
    /// `size` nodes, a component and its decisions.
    fn islands(total: usize, size: usize) -> (Vec<Node>, Vec<Link>) {
        let mut nodes: Vec<Node> = Vec::with_capacity(total);
        let mut links = Vec::new();
        let mut head = 0;
        for n in 0..total {
            let first = n % size == 0;
            if first {
                head = n;
            }
            nodes.push(Node {
                node: NodeRef {
                    kind: if first {
                        NodeKind::Entity
                    } else {
                        NodeKind::Decision
                    },
                    id: format!("n{n}"),
                },
                kind: if first {
                    Kind::Component
                } else {
                    Kind::Decision
                },
                label: format!("n{n}").into(),
                detail: SharedString::default(),
                folded: 0,
                home: None,
                cluster: Some(head),
                degree: 0,
                weight: (0, 0),
                conflict: false,
                suggested: false,
                pos: (0.0, 0.0),
                vel: (0.0, 0.0),
                shade: 1.0,
                stagger: 0.0,
            });
            if !first {
                links.push(Link {
                    a: n,
                    b: head,
                    kind: LinkKind::Affects,
                });
            }
        }
        for link in &links {
            nodes[link.a].degree += 1;
            nodes[link.b].degree += 1;
        }
        (nodes, links)
    }

    /// One component with every other node hanging from it: the worst case
    /// for a grid, since everything sits within reach of everything.
    fn hub(total: usize) -> (Vec<Node>, Vec<Link>) {
        islands(total, total)
    }

    /// Quality numbers of a finished layout: pairs of shapes overlapping,
    /// how far links are from their rest length (mean of length / rest),
    /// and the radius that holds the picture.
    fn quality(nodes: &[Node], links: &[Link]) -> (usize, f32, f32) {
        let mut grid: std::collections::HashMap<(i32, i32), Vec<usize>> =
            std::collections::HashMap::new();
        let cell = |pos: (f32, f32)| ((pos.0 / 80.0).floor() as i32, (pos.1 / 80.0).floor() as i32);
        for (index, node) in nodes.iter().enumerate() {
            grid.entry(cell(node.pos)).or_default().push(index);
        }
        let mut overlaps = 0;
        for (i, a) in nodes.iter().enumerate() {
            let (cx, cy) = cell(a.pos);
            for gx in cx - 1..=cx + 1 {
                for gy in cy - 1..=cy + 1 {
                    for &j in grid
                        .get(&(gx, gy))
                        .into_iter()
                        .flatten()
                        .filter(|j| **j > i)
                    {
                        let b = &nodes[j];
                        let gap =
                            ((a.pos.0 - b.pos.0).powi(2) + (a.pos.1 - b.pos.1).powi(2)).sqrt();
                        if gap < a.radius() + b.radius() {
                            overlaps += 1;
                        }
                    }
                }
            }
        }
        let ratio = links
            .iter()
            .map(|link| {
                let (a, b) = (&nodes[link.a], &nodes[link.b]);
                ((a.pos.0 - b.pos.0).powi(2) + (a.pos.1 - b.pos.1).powi(2)).sqrt() / link.rest()
            })
            .sum::<f32>()
            / links.len().max(1) as f32;
        let extent = nodes
            .iter()
            .map(|node| node.pos.0.hypot(node.pos.1))
            .fold(0.0_f32, f32::max);
        (overlaps, ratio, extent)
    }

    /// Compares the solvers on the same maps. Slow and meant for a release
    /// build, so it is ignored by default:
    ///
    /// `cargo test --release -p desktop-gpui --lib layout_benchmark -- --ignored --nocapture`
    #[test]
    #[ignore = "benchmark: run in release with --ignored --nocapture"]
    fn layout_benchmark() {
        let sizes = [250_usize, 1_000, 2_500, 5_000, 10_000];
        println!("solver,map,nodes,links,ms,overlaps,link_ratio,extent");
        for (map, make) in [
            (
                "ilhas",
                (|n| islands(n, 25)) as fn(usize) -> (Vec<Node>, Vec<Link>),
            ),
            ("hub", hub as fn(usize) -> (Vec<Node>, Vec<Link>)),
        ] {
            for &total in &sizes {
                for solver in [Solver::AllPairs, Solver::Grid, Solver::BarnesHut] {
                    // The reference is quadratic: past a point it only costs time.
                    let limit = match (solver, map) {
                        (Solver::AllPairs, "hub") => 2_500,
                        (Solver::AllPairs, _) => 5_000,
                        (Solver::Grid, "hub") => 5_000,
                        _ => usize::MAX,
                    };
                    if total > limit {
                        continue;
                    }
                    let mut times = Vec::new();
                    let mut last = None;
                    for _ in 0..2 {
                        let (mut nodes, links) = make(total);
                        place(&mut nodes, &links, &BTreeMap::new());
                        let started = std::time::Instant::now();
                        settle_with(solver, &mut nodes, &links, SETTLE_STEPS, false, &|| false);
                        times.push(started.elapsed().as_secs_f64() * 1000.0);
                        last = Some((nodes, links));
                    }
                    times.sort_by(f64::total_cmp);
                    let (nodes, links) = last.expect("ran");
                    let (overlaps, ratio, extent) = quality(&nodes, &links);
                    println!(
                        "{solver:?},{map},{total},{},{:.1},{overlaps},{ratio:.2},{extent:.0}",
                        links.len(),
                        times[0]
                    );
                }
            }
        }
    }

    #[test]
    fn barnes_hut_keeps_the_layout_finite_and_comparable_to_the_grid() {
        let (mut grid, links) = islands(1_000, 25);
        let mut tree = grid.clone();
        place(&mut grid, &links, &BTreeMap::new());
        place(&mut tree, &links, &BTreeMap::new());
        settle_with(
            Solver::Grid,
            &mut grid,
            &links,
            SETTLE_STEPS,
            false,
            &|| false,
        );
        settle_with(
            Solver::BarnesHut,
            &mut tree,
            &links,
            SETTLE_STEPS,
            false,
            &|| false,
        );
        for node in &tree {
            assert!(node.pos.0.is_finite() && node.pos.1.is_finite());
            assert!(node.pos.0.abs() < 50_000.0 && node.pos.1.abs() < 50_000.0);
        }
        let (grid_overlaps, grid_ratio, _) = quality(&grid, &links);
        let (tree_overlaps, tree_ratio, _) = quality(&tree, &links);
        assert!(
            tree_ratio < grid_ratio * 1.5 + 0.5,
            "links stretched: tree {tree_ratio}, grid {grid_ratio}"
        );
        assert!(
            tree_overlaps <= grid_overlaps + 50,
            "overlaps: tree {tree_overlaps}, grid {grid_overlaps}"
        );
    }

    #[test]
    fn a_cancelled_layout_stops_between_steps() {
        let (mut nodes, links) = islands(500, 25);
        place(&mut nodes, &links, &BTreeMap::new());
        let steps = std::cell::Cell::new(0);
        let finished = settle_with(
            Solver::Grid,
            &mut nodes,
            &links,
            SETTLE_STEPS,
            false,
            &|| {
                steps.set(steps.get() + 1);
                steps.get() > 3
            },
        );
        assert!(!finished);
        assert_eq!(steps.get(), 4);
    }
}
