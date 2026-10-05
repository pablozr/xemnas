//! Mapa: the project's components and technologies and what ties decisions
//! and rules to them (ADR-0005).
//!
//! An index on the left (suggestions, file lens, timeline, then every
//! component and technology) and the reading column on the right. Never the
//! whole graph at once: one entity and what is around it. Every read and
//! write goes through `KnowledgeGraph` off the UI thread; a project
//! generation discards answers that arrive after the user switched projects.

use std::collections::BTreeMap;
use std::sync::Arc;

use application::claim_suggestions::{ClaimSuggestionStore, ClaimSuggestionView, ClaimSuggestions};
use application::claims::{ClaimStore, Claims};
use application::decisions::{DecisionFilter, DecisionStatus, DecisionStore, Decisions};
use application::graph::{
    EntityDetail, EntityEdit, EntityRecord, FileLens, GraphStore, KnowledgeGraph, LinkRequest,
    MapEntity, NewEntity, NodeSummary, ProjectGraph, ProjectMap, Suggestion, SuggestionReport,
    TimelineEvent, TimelineKind,
};
use application::projects::ProjectRepository;
use application::relation_suggestions::{
    RelationSuggestionStore, RelationSuggestionView, RelationSuggestions,
};
use application::relations::RelationStore;
use domain::entities::{EdgeKind, EntityKind, NodeKind};
use domain::relations::RelationKind;
use gpui::prelude::*;
use gpui::{
    canvas, div, list, point, px, AnyElement, Context, Div, Entity, EventEmitter, FocusHandle,
    Hsla, ListAlignment, ListState, PathBuilder, Pixels, Render, Role, SharedString, Stateful,
    Subscription, Toggled, Window,
};

use super::context::OpenDecision;
use super::format::{calendar_date, clipped, clock, day_heading, plural, short_date};
use super::graph::{GraphCanvas, GraphEvent};
use crate::ui::controls::{action_button, icon_action, ButtonKind};
use crate::ui::icons::{icon, IconName};
use crate::ui::list::{reveal_footer, scroll_thumb, ScrollMemory};
use crate::ui::patterns::{
    count_chip, empty_panel, error_banner, mark_selected, panel_title, reading_page, rich_sentence,
    section_header, section_label, segment, segmented, skeleton_list, suggestion_card,
    suggestion_section, tag, toast, READING_WIDTH, TOAST_DURATION,
};
use crate::ui::search_field::{SearchChanged, SearchField};
use crate::ui::theme::{text_style, Theme};
use crate::ui::tokens::{RadiusScale, SpacingScale, TypeScale};
use crate::ui::tooltip::tooltip;

/// Every port the Mapa screen reads through.
pub trait MapStores:
    GraphStore
    + RelationStore
    + RelationSuggestionStore
    + ClaimSuggestionStore
    + ClaimStore
    + ProjectRepository
    + DecisionStore
    + Clone
    + Send
    + 'static
{
}

impl<T> MapStores for T where
    T: GraphStore
        + RelationStore
        + RelationSuggestionStore
        + ClaimSuggestionStore
        + ClaimStore
        + ProjectRepository
        + DecisionStore
        + Clone
        + Send
        + 'static
{
}

/// The use cases the Mapa screen needs.
pub struct MapServices<S> {
    /// Entities, edges, suggestions and queries.
    pub graph: KnowledgeGraph<S>,
    /// Decisions in force, for linking.
    pub decisions: Decisions<S>,
    /// Rules of the project, for linking.
    pub claims: Claims<S>,
    /// Relations between decisions waiting for confirmation.
    pub relations: RelationSuggestions<S>,
    /// Context derived from decisions, waiting for confirmation.
    pub context: ClaimSuggestions<S>,
}

/// Decisions offered when linking.
const PICK_LIMIT: usize = 100;
/// Files offered as quick picks in the file lens.
const RECENT_FILES: usize = 6;
/// Days an entity counts as recently active on the overview.
const RECENT_DAYS: i64 = 14;
/// Rows a long list builds first, and how many more "Mostrar mais" adds
/// (twice this).
const LIST_PAGE: usize = 12;
/// Parts a component block lists before counting the rest.
const PART_CHIPS: usize = 6;
/// Most rows a "show all" builds at once; beyond it only pages are offered.
const MOST_AT_ONCE: usize = 200;
/// Timeline events built first.
const TIMELINE_PAGE: usize = 30;
/// Nodes per side of the neighborhood diagram.
const MAX_SIDE: usize = 6;
/// Height of a neighborhood node.
const NODE_H: f32 = 52.0;
/// Vertical gap between neighborhood nodes.
const NODE_GAP: f32 = 10.0;
/// Horizontal gap the curves cross between columns.
const COLUMN_GAP: f32 = 48.0;

/// What the reading column shows.
#[derive(Clone, Debug, PartialEq, Eq)]
enum View {
    Overview,
    Suggestions,
    File,
    Timeline,
    Entity(String),
    Form(Option<String>),
}

/// What a picker links to the open entity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PickKind {
    Decision,
    Claim,
    Parent,
}

struct Picker {
    kind: PickKind,
    items: Vec<(String, String, String)>,
}

/// How the overview draws the map.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Layout {
    Blocks,
    Graph,
}

struct MapData {
    map: Arc<ProjectMap>,
    graph: ProjectGraph,
    /// Relations between decisions waiting for confirmation.
    relations: Arc<Vec<RelationSuggestionView>>,
    /// Context derived from decisions, waiting for confirmation.
    derived: Arc<Vec<ClaimSuggestionView>>,
    /// Components created from the declared workspace on this load.
    assembled: usize,
    suggestions: Arc<Vec<Suggestion>>,
    proposals: Arc<SuggestionReport>,
    /// Files the latest decisions changed: quick picks for the file lens.
    recent_files: Vec<String>,
}

enum Outcome {
    Loaded(Result<Box<MapData>, String>),
    Detail(Result<Box<(EntityDetail, Vec<TimelineEvent>)>, String>),
    Timeline(Result<Vec<TimelineEvent>, String>),
    Lens(Result<Box<FileLens>, String>),
    Picker(Result<Picker, String>),
    Changed {
        result: Result<Option<String>, String>,
        data: Option<Box<MapData>>,
        notice: &'static str,
    },
}

type Operation<S> = Box<dyn FnOnce(&MapServices<S>) -> Outcome + Send>;

/// The Mapa destination of a selected project.
pub struct MapScreen<S: MapStores> {
    backend: Option<MapServices<S>>,
    project: Option<String>,
    generation: u64,
    busy: bool,
    data: Option<MapData>,
    view: View,
    /// Shared snapshots: a render takes a pointer, never a copy of the lists.
    detail: Option<Arc<(EntityDetail, Vec<TimelineEvent>)>>,
    timeline: Option<Arc<Vec<TimelineEvent>>>,
    lens: Option<Arc<FileLens>>,
    picker: Option<Picker>,
    form_kind: EntityKind,
    name: Entity<SearchField>,
    patterns: Entity<SearchField>,
    aliases: Entity<SearchField>,
    description: Entity<SearchField>,
    path: Entity<SearchField>,
    filter: Entity<SearchField>,
    confirm_retire: bool,
    error: Option<String>,
    notice: Option<String>,
    focus: BTreeMap<String, FocusHandle>,
    /// How many rows of each long list are shown (by list key); the rest
    /// waits behind "Mostrar mais". Every scroll step rebuilds the view, so
    /// what is built is what scrolls: a long list is built a page at a time.
    shown: BTreeMap<String, usize>,
    /// The index's rows (views, headings, entities) and the virtual list
    /// that shows only those in view; the rows are rebuilt when their shape
    /// changes.
    index_rows: Vec<IndexRow>,
    /// Shown inside the Revisão: only the suggestions page, no index.
    embedded: bool,
    index_list: ListState,
    index_scroll: ScrollMemory,
    /// The Blocos layout: what it reads from the map, its rows and the
    /// virtual list that builds only those in view.
    overview_index: Option<Arc<OverviewIndex>>,
    overview_rows: Vec<OverviewRow>,
    overview_list: ListState,
    overview_scroll: ScrollMemory,
    /// Whether the graph holds older data than the map: its layout is made
    /// when the Grafo layout is first shown.
    graph_stale: bool,
    /// Demo-only view to open once the map loads (`--open map:<view>`).
    route: Option<String>,
    layout: Layout,
    graph: Entity<GraphCanvas>,
    /// Project the graph was last filled for, to play the entrance once.
    graph_project: Option<String>,
    _subscriptions: Vec<Subscription>,
}

/// Asks the shell to open the queue of suggested ties (the Revisão), where
/// the map's suggestions are reviewed beside the candidates.
pub struct OpenSuggestions;

impl<S: MapStores> EventEmitter<OpenDecision> for MapScreen<S> {}
impl<S: MapStores> EventEmitter<OpenSuggestions> for MapScreen<S> {}

impl<S: MapStores> MapScreen<S> {
    /// Mounts the screen; nothing is read until a project is set.
    pub fn new(cx: &mut Context<Self>, services: MapServices<S>) -> Self {
        let field = |cx: &mut Context<Self>, placeholder: &'static str| {
            cx.new(|cx| {
                let mut field = SearchField::new(cx);
                field.stretch();
                field.set_context(placeholder, cx);
                field
            })
        };
        let name = field(cx, "Ex.: storage-sqlite");
        let patterns = field(cx, "Ex.: crates/storage-sqlite/**, migrations/*.sql");
        let aliases = field(cx, "Ex.: armazenamento, banco");
        let description = field(cx, "Uma frase sobre o que é");
        let path = field(cx, "Ex.: crates/storage-sqlite/src/store.rs");
        let filter = field(cx, "Filtrar");
        let mut subscriptions: Vec<Subscription> =
            [&name, &patterns, &aliases, &description, &path, &filter]
                .into_iter()
                .map(|field| cx.subscribe(field, |_, _, _: &SearchChanged, cx| cx.notify()))
                .collect();
        let graph = cx.new(GraphCanvas::new);
        subscriptions.push(cx.subscribe(&graph, |this, _, event: &GraphEvent, cx| {
            this.on_graph(event, cx)
        }));
        let index_scroll = ScrollMemory::default();
        let index_list = ListState::new(0, ListAlignment::Top, px(480.0));
        index_scroll.attach(&index_list);
        let overview_scroll = ScrollMemory::default();
        let overview_list = ListState::new(0, ListAlignment::Top, px(640.0));
        overview_scroll.attach(&overview_list);
        Self {
            backend: Some(services),
            project: None,
            generation: 0,
            busy: false,
            data: None,
            view: View::Overview,
            detail: None,
            timeline: None,
            lens: None,
            picker: None,
            form_kind: EntityKind::Component,
            name,
            patterns,
            aliases,
            description,
            path,
            filter,
            confirm_retire: false,
            error: None,
            notice: None,
            focus: BTreeMap::new(),
            shown: BTreeMap::new(),
            index_rows: Vec::new(),
            embedded: false,
            index_list,
            index_scroll,
            overview_index: None,
            overview_rows: Vec::new(),
            overview_list,
            overview_scroll,
            graph_stale: true,
            route: None,
            layout: Layout::Blocks,
            graph,
            graph_project: None,
            _subscriptions: subscriptions,
        }
    }

    fn on_graph(&mut self, event: &GraphEvent, cx: &mut Context<Self>) {
        match event {
            GraphEvent::OpenEntity(id) => self.open_entity(id.clone(), cx),
            GraphEvent::OpenDecision(id) => cx.emit(OpenDecision(id.clone())),
            GraphEvent::Confirm(edge) => {
                let id = edge.clone();
                self.mutate(cx, "Vínculo confirmado.", move |backend| {
                    backend.graph.confirm(&id).map(|_| None)
                });
            }
            GraphEvent::Reject(edge) => {
                let id = edge.clone();
                self.mutate(cx, "Sugestão rejeitada; ela não volta.", move |backend| {
                    backend.graph.invalidate(&id).map(|_| None)
                });
            }
        }
    }

    /// Hands fresh data to the graph; a new project plays the entrance.
    fn fill_graph(&mut self, cx: &mut Context<Self>) {
        // The graph's layout is costly and only the Grafo layout shows it:
        // in Blocos it waits, and is made when the person opens it.
        if self.layout != Layout::Graph {
            self.graph_stale = true;
            return;
        }
        self.graph_stale = false;
        let Some(data) = self.data.as_ref() else {
            return;
        };
        let fresh = self.graph_project != self.project;
        self.graph_project = self.project.clone();
        let graph = data.graph.clone();
        self.graph
            .update(cx, |canvas, cx| canvas.set_graph(&graph, fresh, cx));
    }

    fn set_layout(&mut self, layout: Layout, cx: &mut Context<Self>) {
        self.layout = layout;
        if layout == Layout::Graph && self.graph_stale {
            self.fill_graph(cx);
        }
        cx.notify();
    }

    /// Shows only the suggestions page (inside the Revisão) or the map
    /// again. The Revisão is where suggestions are reviewed; the map's own
    /// index no longer lists them.
    pub fn set_embedded(&mut self, embedded: bool, cx: &mut Context<Self>) {
        if self.embedded == embedded {
            return;
        }
        self.embedded = embedded;
        self.view = if embedded {
            View::Suggestions
        } else {
            View::Overview
        };
        cx.notify();
    }

    /// Suggestions waiting for a person: ties between decisions and map
    /// items, derived context, and components or technologies the project
    /// declares that the map does not have yet.
    pub fn pending_suggestions(&self) -> usize {
        self.data.as_ref().map_or(0, |data| {
            data.suggestions.len()
                + data.relations.len()
                + data.derived.len()
                + data.proposals.components.len()
                + data.proposals.technologies.len()
        })
    }

    /// Opens `timeline`, `suggestions`, `file` or `entity:<name>` once the
    /// map is loaded; used by the demo to reach a view without input.
    pub fn open_route(&mut self, route: String) {
        self.route = Some(route);
    }

    fn follow_route(&mut self, cx: &mut Context<Self>) {
        let Some(route) = self.route.take() else {
            return;
        };
        match route.as_str() {
            "timeline" => self.open_view(View::Timeline, cx),
            "graph" => self.set_layout(Layout::Graph, cx),
            other if other.starts_with("graph:") => {
                let name = other.trim_start_matches("graph:").to_owned();
                self.set_layout(Layout::Graph, cx);
                self.graph
                    .update(cx, |canvas, cx| canvas.select_named(&name, cx));
            }
            "file" => self.open_view(View::File, cx),
            other => {
                let name = other.strip_prefix("entity:").unwrap_or(other);
                let id = self.data.as_ref().and_then(|data| {
                    data.map
                        .entities
                        .iter()
                        .find(|row| row.entity.name == name)
                        .map(|row| row.entity.entity_id.clone())
                });
                if let Some(id) = id {
                    self.open_entity(id, cx);
                }
            }
        }
    }

    /// Clears the previous project's state and loads the new one.
    pub fn set_project(&mut self, project: Option<String>, cx: &mut Context<Self>) {
        if self.project == project {
            return;
        }
        self.project = project;
        self.generation += 1;
        self.data = None;
        self.detail = None;
        self.timeline = None;
        self.lens = None;
        self.picker = None;
        self.view = if self.embedded {
            View::Suggestions
        } else {
            View::Overview
        };
        self.shown.clear();
        self.error = None;
        self.notice = None;
        self.refresh(cx);
    }

    /// Reloads the map; refreshing also derives new suggestions.
    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        let Some(project) = self.project.clone() else {
            return;
        };
        self.run(
            cx,
            Box::new(move |backend| Outcome::Loaded(load(backend, &project).map(Box::new))),
        );
    }

    fn run(&mut self, cx: &mut Context<Self>, operation: Operation<S>) {
        let Some(backend) = self.backend.take() else {
            return;
        };
        self.busy = true;
        self.error = None;
        let generation = self.generation;
        cx.notify();
        cx.spawn(async move |this, cx| {
            let (backend, outcome) = cx
                .background_executor()
                .spawn(async move {
                    let outcome = operation(&backend);
                    (backend, outcome)
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                this.backend = Some(backend);
                this.busy = false;
                if generation != this.generation {
                    this.refresh(cx);
                    return;
                }
                this.apply(outcome, cx);
                cx.notify();
            });
        })
        .detach();
    }

    fn apply(&mut self, outcome: Outcome, cx: &mut Context<Self>) {
        match outcome {
            Outcome::Loaded(Ok(data)) => {
                let assembled = data.assembled;
                self.data = Some(*data);
                self.fill_graph(cx);
                if assembled > 0 {
                    self.show_notice(
                        match assembled {
                            1 => "Mapa montado a partir do workspace: 1 componente.".to_owned(),
                            n => format!("Mapa montado a partir do workspace: {n} componentes."),
                        },
                        cx,
                    );
                }
                if self.route.is_some() {
                    return self.follow_route(cx);
                }
                // A view on an entity that no longer exists falls back.
                if let View::Entity(id) = &self.view {
                    let id = id.clone();
                    if self.entity(&id).is_some() {
                        self.open_entity(id, cx);
                    } else {
                        self.view = View::Overview;
                    }
                }
            }
            Outcome::Detail(Ok(detail)) => self.detail = Some(Arc::new(*detail)),
            Outcome::Timeline(Ok(events)) => self.timeline = Some(Arc::new(events)),
            Outcome::Lens(Ok(lens)) => self.lens = Some(Arc::new(*lens)),
            Outcome::Picker(Ok(picker)) => {
                self.filter.update(cx, |field, cx| field.set_value("", cx));
                self.picker = Some(picker);
            }
            Outcome::Changed {
                result: Ok(open),
                data,
                notice,
            } => {
                if let Some(data) = data {
                    self.data = Some(*data);
                    self.fill_graph(cx);
                }
                self.picker = None;
                self.confirm_retire = false;
                self.show_notice(notice.into(), cx);
                match open {
                    Some(id) => self.open_entity(id, cx),
                    None => {
                        if let View::Entity(id) = self.view.clone() {
                            if self.entity(&id).is_some() {
                                self.open_entity(id, cx);
                            } else {
                                self.view = View::Overview;
                            }
                        }
                    }
                }
            }
            Outcome::Loaded(Err(error))
            | Outcome::Detail(Err(error))
            | Outcome::Timeline(Err(error))
            | Outcome::Lens(Err(error))
            | Outcome::Picker(Err(error))
            | Outcome::Changed {
                result: Err(error), ..
            } => self.error = Some(error),
        }
    }

    fn show_notice(&mut self, message: String, cx: &mut Context<Self>) {
        self.notice = Some(message.clone());
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(TOAST_DURATION).await;
            let _ = this.update(cx, |this, cx| {
                if this.notice.as_deref() == Some(message.as_str()) {
                    this.notice = None;
                    cx.notify();
                }
            });
        })
        .detach();
    }

    fn entity(&self, id: &str) -> Option<&MapEntity> {
        self.data
            .as_ref()?
            .map
            .entities
            .iter()
            .find(|row| row.entity.entity_id == id)
    }

    /// Opens one entity's detail (also used by other destinations).
    pub fn show_entity(&mut self, id: String, cx: &mut Context<Self>) {
        self.open_entity(id, cx);
    }

    fn open_entity(&mut self, id: String, cx: &mut Context<Self>) {
        let Some(project) = self.project.clone() else {
            return;
        };
        self.view = View::Entity(id.clone());
        self.shown.clear();
        self.picker = None;
        self.confirm_retire = false;
        if self
            .detail
            .as_ref()
            .is_none_or(|snapshot| snapshot.0.entity.entity_id != id)
        {
            self.detail = None;
        }
        self.run(
            cx,
            Box::new(move |backend| {
                let detail = backend.graph.entity_detail(&id, None).map_err(|error| {
                    failure(
                        "entity_detail",
                        error.code(),
                        "Não foi possível abrir o item.",
                    )
                });
                let events = backend
                    .graph
                    .timeline(&project, Some(&id), None, None)
                    .map_err(|error| {
                        failure("timeline", error.code(), "Não foi possível ler a história.")
                    });
                Outcome::Detail(detail.and_then(|detail| Ok(Box::new((detail, events?)))))
            }),
        );
    }

    fn open_view(&mut self, view: View, cx: &mut Context<Self>) {
        self.shown.clear();
        self.picker = None;
        self.confirm_retire = false;
        match &view {
            View::Entity(id) => return self.open_entity(id.clone(), cx),
            View::Timeline => {
                if let Some(project) = self.project.clone() {
                    self.view = view;
                    self.run(
                        cx,
                        Box::new(move |backend| {
                            Outcome::Timeline(
                                backend.graph.timeline(&project, None, None, None).map_err(
                                    |error| {
                                        failure(
                                            "timeline",
                                            error.code(),
                                            "Não foi possível ler a linha do tempo.",
                                        )
                                    },
                                ),
                            )
                        }),
                    );
                    return;
                }
            }
            View::Form(editing) => {
                let entity = editing
                    .as_ref()
                    .and_then(|id| self.entity(id))
                    .map(|row| row.entity.clone());
                self.fill_form(entity.as_ref(), cx);
            }
            View::Overview | View::Suggestions | View::File => {}
        }
        self.view = view;
        cx.notify();
    }

    fn fill_form(&mut self, entity: Option<&EntityRecord>, cx: &mut Context<Self>) {
        self.form_kind = entity.map_or(EntityKind::Component, |entity| entity.kind);
        let values = [
            entity.map(|entity| entity.name.clone()).unwrap_or_default(),
            entity
                .map(|entity| entity.patterns.join(", "))
                .unwrap_or_default(),
            entity
                .map(|entity| entity.aliases.join(", "))
                .unwrap_or_default(),
            entity
                .map(|entity| entity.description.clone())
                .unwrap_or_default(),
        ];
        for (field, value) in [&self.name, &self.patterns, &self.aliases, &self.description]
            .into_iter()
            .zip(values)
        {
            field.update(cx, |field, cx| field.set_value(&value, cx));
        }
    }

    fn value(&self, field: &Entity<SearchField>, cx: &Context<Self>) -> String {
        field.read(cx).value().trim().to_owned()
    }

    fn list(&self, field: &Entity<SearchField>, cx: &Context<Self>) -> Vec<String> {
        self.value(field, cx)
            .split([',', ';'])
            .map(|item| item.trim().to_owned())
            .filter(|item| !item.is_empty())
            .collect()
    }

    fn save_form(&mut self, editing: Option<String>, cx: &mut Context<Self>) {
        let Some(project) = self.project.clone() else {
            return;
        };
        let edit = EntityEdit {
            name: self.value(&self.name, cx),
            description: self.value(&self.description, cx),
            patterns: if self.form_kind == EntityKind::Component {
                self.list(&self.patterns, cx)
            } else {
                Vec::new()
            },
            aliases: self.list(&self.aliases, cx),
        };
        let kind = self.form_kind;
        self.mutate(
            cx,
            if editing.is_some() {
                "Item atualizado."
            } else {
                "Item criado. As sugestões foram atualizadas."
            },
            move |backend| match editing {
                Some(id) => backend
                    .graph
                    .update_entity(&id, edit)
                    .map(|entity| Some(entity.entity_id)),
                None => backend
                    .graph
                    .create_entity(NewEntity {
                        project_id: project,
                        kind: Some(kind),
                        name: edit.name,
                        description: edit.description,
                        patterns: edit.patterns,
                        aliases: edit.aliases,
                    })
                    .map(|entity| Some(entity.entity_id)),
            },
        );
    }

    /// Runs a graph change, then reloads the map (which re-derives suggestions).
    fn mutate(
        &mut self,
        cx: &mut Context<Self>,
        notice: &'static str,
        change: impl FnOnce(&MapServices<S>) -> Result<Option<String>, application::graph::GraphError>
            + Send
            + 'static,
    ) {
        let Some(project) = self.project.clone() else {
            return;
        };
        self.run(
            cx,
            Box::new(move |backend| {
                let result = change(backend).map_err(|error| change_failure(error.code()));
                let data = result
                    .is_ok()
                    .then(|| load(backend, &project).ok().map(Box::new))
                    .flatten();
                Outcome::Changed {
                    result,
                    data,
                    notice,
                }
            }),
        );
    }

    /// Like [`Self::mutate`] for derived context.
    fn mutate_context(
        &mut self,
        cx: &mut Context<Self>,
        notice: &'static str,
        change: impl FnOnce(
                &MapServices<S>,
            ) -> Result<(), application::claim_suggestions::ClaimSuggestionError>
            + Send
            + 'static,
    ) {
        let Some(project) = self.project.clone() else {
            return;
        };
        self.run(
            cx,
            Box::new(move |backend| {
                let result = change(backend)
                    .map(|_| None)
                    .map_err(|error| match error.code() {
                        "not_found" => "A sugestão mudou. O mapa foi atualizado.".to_owned(),
                        "claim" => "A regra não pôde ser criada; ela pode já existir.".to_owned(),
                        code => {
                            tracing::error!(code, operation = "claim_suggestion", "change failed");
                            "Não foi possível salvar a regra.".to_owned()
                        }
                    });
                let data = result
                    .is_ok()
                    .then(|| load(backend, &project).ok().map(Box::new))
                    .flatten();
                Outcome::Changed {
                    result,
                    data,
                    notice,
                }
            }),
        );
    }

    /// Like [`Self::mutate`] for changes made through decisions (relations).
    fn mutate_decisions(
        &mut self,
        cx: &mut Context<Self>,
        notice: &'static str,
        change: impl FnOnce(&MapServices<S>) -> Result<(), application::decisions::DecisionsError>
            + Send
            + 'static,
    ) {
        let Some(project) = self.project.clone() else {
            return;
        };
        self.run(
            cx,
            Box::new(move |backend| {
                let result = change(backend)
                    .map(|_| None)
                    .map_err(|error| match error.code() {
                        "invalid_relation" => {
                            "Essa relação não cabe mais (ciclo ou já registrada).".to_owned()
                        }
                        "conflict" | "not_found" => {
                            "Uma das decisões mudou. O mapa foi atualizado.".to_owned()
                        }
                        code => {
                            tracing::error!(code, operation = "relation", "relation change failed");
                            "Não foi possível registrar a relação.".to_owned()
                        }
                    });
                let data = result
                    .is_ok()
                    .then(|| load(backend, &project).ok().map(Box::new))
                    .flatten();
                Outcome::Changed {
                    result,
                    data,
                    notice,
                }
            }),
        );
    }

    fn open_picker(&mut self, kind: PickKind, cx: &mut Context<Self>) {
        let (Some(project), View::Entity(entity_id)) = (self.project.clone(), self.view.clone())
        else {
            return;
        };
        let linked: Vec<String> = self
            .detail
            .as_ref()
            .map(|snapshot| {
                let detail = &snapshot.0;
                detail
                    .decisions
                    .iter()
                    .chain(&detail.claims)
                    .chain(&detail.parent)
                    .map(|row| row.node.id.clone())
                    .collect()
            })
            .unwrap_or_default();
        let components: Vec<(String, String, String)> = self
            .data
            .as_ref()
            .map(|data| {
                data.map
                    .entities
                    .iter()
                    .filter(|row| {
                        row.entity.kind == EntityKind::Component
                            && row.entity.entity_id != entity_id
                    })
                    .map(|row| {
                        (
                            row.entity.entity_id.clone(),
                            row.entity.name.clone(),
                            row.entity.patterns.join(", "),
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();
        self.run(
            cx,
            Box::new(move |backend| {
                let items = match kind {
                    PickKind::Decision => backend
                        .decisions
                        .list(&DecisionFilter {
                            project_id: Some(project.clone()),
                            statuses: vec![DecisionStatus::Accepted],
                            limit: PICK_LIMIT,
                            cursor: None,
                        })
                        .map(|page| {
                            page.decisions
                                .into_iter()
                                .map(|decision| {
                                    (decision.decision_id, decision.question, decision.choice)
                                })
                                .collect()
                        })
                        .map_err(|error| {
                            failure(
                                "pick_decisions",
                                error.code(),
                                "Não foi possível listar as decisões.",
                            )
                        }),
                    PickKind::Claim => backend
                        .claims
                        .list(&project, Some(&today()))
                        .map(|claims| {
                            claims
                                .into_iter()
                                .map(|claim| {
                                    (
                                        claim.claim_id,
                                        claim.statement,
                                        claim.kind.as_str().to_owned(),
                                    )
                                })
                                .collect()
                        })
                        .map_err(|error| {
                            failure(
                                "pick_claims",
                                error.code(),
                                "Não foi possível listar as regras.",
                            )
                        }),
                    PickKind::Parent => Ok(components),
                };
                Outcome::Picker(items.map(|items: Vec<(String, String, String)>| {
                    Picker {
                        kind,
                        items: items
                            .into_iter()
                            .filter(|(id, _, _)| !linked.contains(id))
                            .collect(),
                    }
                }))
            }),
        );
    }

    fn pick(&mut self, index: usize, cx: &mut Context<Self>) {
        let (Some(picker), View::Entity(entity_id)) = (&self.picker, self.view.clone()) else {
            return;
        };
        let Some((id, _, _)) = self.filtered(picker, cx).get(index).cloned() else {
            return;
        };
        let request = match picker.kind {
            PickKind::Decision => {
                let kind = match self.entity(&entity_id).map(|row| row.entity.kind) {
                    Some(EntityKind::Technology) => EdgeKind::Uses,
                    _ => EdgeKind::Affects,
                };
                LinkRequest {
                    kind,
                    source_kind: NodeKind::Decision,
                    source_id: id,
                    entity_id,
                }
            }
            PickKind::Claim => LinkRequest {
                kind: EdgeKind::AppliesTo,
                source_kind: NodeKind::Claim,
                source_id: id,
                entity_id,
            },
            PickKind::Parent => LinkRequest {
                kind: EdgeKind::PartOf,
                source_kind: NodeKind::Entity,
                source_id: entity_id,
                entity_id: id,
            },
        };
        self.mutate(cx, "Vínculo registrado.", move |backend| {
            backend.graph.link(request).map(|_| None)
        });
    }

    fn filtered(&self, picker: &Picker, cx: &Context<Self>) -> Vec<(String, String, String)> {
        let query = self.value(&self.filter, cx).to_lowercase();
        picker
            .items
            .iter()
            .filter(|(_, label, detail)| {
                query.is_empty()
                    || label.to_lowercase().contains(&query)
                    || detail.to_lowercase().contains(&query)
            })
            .cloned()
            .collect()
    }

    fn open_lens(&mut self, cx: &mut Context<Self>) {
        let (Some(project), path) = (self.project.clone(), self.value(&self.path, cx)) else {
            return;
        };
        if path.is_empty() {
            return;
        }
        self.run(
            cx,
            Box::new(move |backend| {
                Outcome::Lens(
                    backend
                        .graph
                        .file_lens(&project, &path, None)
                        .map(Box::new)
                        .map_err(|error| {
                            failure(
                                "file_lens",
                                error.code(),
                                "Não foi possível ler esse caminho.",
                            )
                        }),
                )
            }),
        );
    }

    // ---- small builders -------------------------------------------------

    /// Rows of list `key` to build now; `first` is the opening page.
    fn limit(&self, key: &str, first: usize) -> usize {
        self.shown.get(key).copied().unwrap_or(first)
    }

    /// The footer under a list cut after some rows: how far along it is, a
    /// button for the next two pages and, while what is left is small
    /// enough to build at once, one for all of it.
    fn more_row(
        &mut self,
        key: &'static str,
        first: usize,
        remaining: usize,
        cx: &mut Context<Self>,
    ) -> Div {
        let theme = Theme::current(cx);
        let shown = self.limit(key, first);
        let step = (2 * first).min(remaining);
        let more = self.button(
            format!("map-more-{key}"),
            ButtonKind::Ghost,
            true,
            format!("Mostrar mais {step}"),
            move |this, cx| {
                let shown = this.limit(key, first);
                this.shown
                    .insert(key.to_owned(), shown.saturating_add(2 * first));
                cx.notify();
            },
            cx,
        );
        let all = (remaining > step && remaining <= MOST_AT_ONCE).then(|| {
            self.button(
                format!("map-all-{key}"),
                ButtonKind::Ghost,
                true,
                format!("Mostrar todas ({remaining})"),
                move |this, cx| {
                    this.shown.insert(key.to_owned(), usize::MAX);
                    cx.notify();
                },
                cx,
            )
        });
        reveal_footer(&theme, shown, shown + remaining, Some(more), all)
    }

    fn focus_for(&mut self, id: &str, cx: &mut Context<Self>) -> FocusHandle {
        self.focus
            .entry(id.to_owned())
            .or_insert_with(|| cx.focus_handle().tab_stop(true))
            .clone()
    }

    fn button(
        &mut self,
        id: impl Into<String>,
        kind: ButtonKind,
        enabled: bool,
        label: impl Into<SharedString>,
        on_press: impl Fn(&mut Self, &mut Context<Self>) + 'static,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let id = id.into();
        let label = label.into();
        let theme = Theme::current(cx);
        let focus = self.focus_for(&id, cx);
        let on_press = std::rc::Rc::new(on_press);
        let on_key = on_press.clone();
        action_button(&theme, SharedString::from(id), kind, enabled)
            .aria_label(label.clone())
            .track_focus(&focus)
            .on_click(cx.listener(move |this, _, _, cx| {
                if enabled {
                    on_press(this, cx);
                }
            }))
            .on_key_down(cx.listener(move |this, event: &gpui::KeyDownEvent, _, cx| {
                if enabled && matches!(event.keystroke.key.as_str(), "enter" | "space") {
                    on_key(this, cx);
                    cx.stop_propagation();
                }
            }))
            .child(label)
    }

    /// A selectable, keyboard-reachable row.
    fn row(
        &mut self,
        id: String,
        selected: bool,
        label: &str,
        on_press: impl Fn(&mut Self, &mut Context<Self>) + 'static,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let theme = Theme::current(cx);
        let colors = theme.colors;
        let focus = self.focus_for(&id, cx);
        let on_press = std::rc::Rc::new(on_press);
        let on_key = on_press.clone();
        mark_selected(
            div()
                .id(SharedString::from(id))
                .relative()
                .flex()
                .items_center()
                .gap(px(SpacingScale::S3))
                .px(px(SpacingScale::S3))
                .py(px(SpacingScale::S2))
                .rounded(theme.radius.control())
                .cursor_pointer()
                .role(Role::Button)
                .aria_label(label.to_owned())
                .track_focus(&focus)
                .focus_visible(crate::ui::controls::focus_ring(&theme))
                .when(!selected, |row| {
                    row.hover(move |style| style.bg(colors.glass_fill_medium()))
                        .active(move |style| style.bg(colors.glass_fill_strong()))
                })
                .on_click(cx.listener(move |this, _, _, cx| on_press(this, cx)))
                .on_key_down(cx.listener(move |this, event: &gpui::KeyDownEvent, _, cx| {
                    if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                        on_key(this, cx);
                        cx.stop_propagation();
                    }
                })),
            &theme,
            selected,
        )
    }

    // ---- index ----------------------------------------------------------

    /// The index as a virtual list: only the rows in view are built, so a
    /// project with thousands of components scrolls as lightly as one with
    /// ten.
    fn render_index(&mut self, theme: &Theme, cx: &mut Context<Self>) -> Div {
        let colors = theme.colors;
        let Some(data) = self.data.as_ref() else {
            return index_frame(theme).child(div().p(px(SpacingScale::S4)).child(skeleton_list(
                theme,
                "map-index-skeleton",
                6,
            )));
        };
        let count = data.map.entities.len();
        let as_of = data.map.as_of.clone();
        let rows = index_rows(&data.map.entities);
        if rows != self.index_rows {
            let old = self.index_rows.len();
            self.index_list.splice(0..old, rows.len());
            self.index_rows = rows;
        }
        let new_focus = self.focus_for("map-new", cx);
        let new_button = icon_action(theme, "map-new", "Novo componente ou tecnologia")
            .track_focus(&new_focus)
            .tooltip(tooltip("Novo componente ou tecnologia", None))
            .on_click(cx.listener(|this, _, _, cx| this.open_view(View::Form(None), cx)))
            .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, _, cx| {
                if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                    this.open_view(View::Form(None), cx);
                    cx.stop_propagation();
                }
            }))
            .child(icon(IconName::Plus, 14.0, colors.text_secondary()));

        let rows = list(
            self.index_list.clone(),
            cx.processor(|this, ix: usize, _, cx| this.index_item(ix, cx)),
        )
        .size_full();

        index_frame(theme)
            .child(
                div()
                    .px(px(SpacingScale::S4))
                    .pt(px(SpacingScale::S3))
                    .pb(px(SpacingScale::S2))
                    .flex()
                    .items_center()
                    .gap(px(SpacingScale::S2))
                    .child(panel_title(theme, "Mapa"))
                    .child(count_chip(theme, count.to_string()))
                    .child(div().flex_1())
                    .child(new_button),
            )
            .child(
                div()
                    .id("map-index")
                    .relative()
                    .flex_1()
                    .min_h(px(0.0))
                    .px(px(SpacingScale::S2))
                    .pb(px(SpacingScale::S3))
                    .child(rows)
                    .child(scroll_thumb(theme, &self.index_list, &self.index_scroll)),
            )
            .child(
                text_style(div(), TypeScale::META)
                    .flex_none()
                    .px(px(SpacingScale::S4))
                    .py(px(SpacingScale::S2))
                    .border_t_1()
                    .border_color(colors.hairline_divider())
                    .text_color(colors.text_muted())
                    .child(if self.busy {
                        "Carregando…".to_owned()
                    } else {
                        format!("Situação em {}", short_date(&as_of))
                    }),
            )
    }

    /// One row of the index, built when the list asks for it.
    fn index_item(&mut self, ix: usize, cx: &mut Context<Self>) -> AnyElement {
        let theme = Theme::current(cx);
        let Some(&row) = self.index_rows.get(ix) else {
            return div().into_any_element();
        };
        let element = match row {
            IndexRow::View(position) => self.view_row(&theme, position, cx).into_any_element(),
            IndexRow::Heading(kind) => div()
                .px(px(SpacingScale::S3))
                .pt(px(SpacingScale::S4))
                .pb(px(SpacingScale::S1))
                .child(section_label(&theme, kind_plural(kind)))
                .into_any_element(),
            IndexRow::Nothing(kind) => text_style(div(), TypeScale::BODY_SMALL)
                .px(px(SpacingScale::S3))
                .text_color(theme.colors.text_muted())
                .child(match kind {
                    EntityKind::Component => "Nenhum ainda. Crie um ou veja as sugestões.",
                    EntityKind::Technology => "Nenhuma ainda.",
                })
                .into_any_element(),
            IndexRow::Entity(position) => {
                let Some(entity) = self
                    .data
                    .as_ref()
                    .and_then(|data| data.map.entities.get(position))
                    .cloned()
                else {
                    return div().into_any_element();
                };
                self.entity_row(&theme, &entity, cx).into_any_element()
            }
        };
        div().w_full().pb(px(2.0)).child(element).into_any_element()
    }

    /// One of the fixed views at the top of the index.
    fn view_row(
        &mut self,
        theme: &Theme,
        position: usize,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let colors = theme.colors;
        let (view, glyph, label, badge): (View, IconName, &str, Option<String>) = match position {
            0 => (View::Overview, IconName::Graph, "Visão geral", None),
            1 => (View::File, IconName::File, "Lente de arquivo", None),
            _ => (View::Timeline, IconName::Clock, "Linha do tempo", None),
        };
        let selected = self.view == view;
        self.row(
            format!("map-view-{label}"),
            selected,
            label,
            move |this, cx| this.open_view(view.clone(), cx),
            cx,
        )
        .child(icon(
            glyph,
            16.0,
            if selected {
                colors.text_primary()
            } else {
                colors.text_muted()
            },
        ))
        .child(
            text_style(div(), TypeScale::ROW_TITLE)
                .flex_1()
                .text_color(if selected {
                    colors.text_primary()
                } else {
                    colors.text_secondary()
                })
                .child(label),
        )
        .children(badge.map(|badge| count_chip(theme, badge)))
    }

    /// A component or technology of the index.
    fn entity_row(
        &mut self,
        theme: &Theme,
        row: &MapEntity,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let colors = theme.colors;
        let id = row.entity.entity_id.clone();
        let selected = self.view == View::Entity(id.clone());
        let meta = weight(row);
        let target = id.clone();
        self.row(
            format!("map-entity-{id}"),
            selected,
            &row.entity.name,
            move |this, cx| this.open_entity(target.clone(), cx),
            cx,
        )
        .child(
            div()
                .flex_1()
                .min_w(px(0.0))
                .flex()
                .flex_col()
                .child(
                    text_style(div(), TypeScale::ROW_TITLE)
                        .truncate()
                        .text_color(if selected {
                            colors.text_primary()
                        } else {
                            colors.text_secondary()
                        })
                        .child(row.entity.name.clone()),
                )
                .child(
                    text_style(div(), TypeScale::META)
                        .truncate()
                        .text_color(colors.text_muted())
                        .child(meta),
                ),
        )
        .when(row.conflicts > 0, |element| {
            element.child(
                div()
                    .size(px(6.0))
                    .flex_none()
                    .rounded_full()
                    .bg(colors.status_warning()),
            )
        })
    }

    // ---- reading column -------------------------------------------------

    fn render_suggestions(&mut self, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
        let colors = theme.colors;
        let Some(data) = self.data.as_ref() else {
            return div().into_any_element();
        };
        let suggestions = Arc::clone(&data.suggestions);
        let relations = Arc::clone(&data.relations);
        let derived = Arc::clone(&data.derived);
        let proposals = Arc::clone(&data.proposals);
        let (components, technologies) = (&proposals.components, &proposals.technologies);
        if suggestions.is_empty()
            && relations.is_empty()
            && derived.is_empty()
            && components.is_empty()
            && technologies.is_empty()
        {
            return empty_panel(
                theme,
                IconName::Lightbulb,
                "Sugestões",
                "Nada para revisar",
                "Sugestões aparecem quando decisões confirmadas tocam arquivos ou adicionam \
                 dependências. Crie componentes com padrões de caminho para ligá-los às decisões.",
            )
            .min_h(px(420.0))
            .into_any_element();
        }
        let mut column = div()
            .flex()
            .flex_col()
            .gap(px(SpacingScale::S8))
            .child(page_header(
                theme,
                "Sugestões",
                "O que o trabalho capturado indica. Nada entra no mapa sem a sua confirmação.",
            ));
        if !relations.is_empty() {
            let mut list = div().flex().flex_col();
            let shown = relations.len().min(self.limit("sug-relations", LIST_PAGE));
            for (index, suggestion) in relations.iter().take(shown).enumerate() {
                let record = &suggestion.record;
                let confirm_id = record.suggestion_id.clone();
                let reject_id = record.suggestion_id.clone();
                let confirm = self.button(
                    format!("map-relation-confirm-{index}"),
                    ButtonKind::Secondary,
                    !self.busy,
                    "Confirmar",
                    move |this, cx| {
                        let id = confirm_id.clone();
                        this.mutate_decisions(cx, "Relação registrada.", move |backend| {
                            backend.relations.confirm(&id)
                        });
                    },
                    cx,
                );
                let reject = self.button(
                    format!("map-relation-reject-{index}"),
                    ButtonKind::Ghost,
                    !self.busy,
                    "Rejeitar",
                    move |this, cx| {
                        let id = reject_id.clone();
                        this.mutate_decisions(
                            cx,
                            "Relação rejeitada; ela não volta.",
                            move |backend| backend.relations.reject(&id),
                        );
                    },
                    cx,
                );
                let verb = match record.kind {
                    RelationKind::DependsOn => "depende de",
                    RelationKind::ConflictsWith => "conflita com",
                    RelationKind::Supersedes => "substitui",
                };
                let (sentence, effect) = relation_wording(
                    record.kind,
                    &suggestion.from_question,
                    &suggestion.to_question,
                );
                let lead = div()
                    .flex()
                    .flex_col()
                    .gap(px(SpacingScale::S2))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(SpacingScale::S2))
                            .child(
                                tag(theme, verb)
                                    .when(record.kind == RelationKind::ConflictsWith, |tag| {
                                        tag.text_color(colors.status_warning())
                                    }),
                            )
                            .child(
                                text_style(div(), TypeScale::META)
                                    .text_color(colors.text_muted())
                                    .child("entre duas decisões"),
                            ),
                    )
                    .child(rich_sentence(theme, &sentence, TypeScale::BODY))
                    .when(!record.reason.is_empty(), |lead| {
                        lead.child(
                            text_style(div(), TypeScale::BODY_SMALL)
                                .text_color(colors.text_muted())
                                .child(format!("Por quê: {}", record.reason)),
                        )
                    });
                list = list.child(suggestion_card(
                    theme,
                    lead,
                    Some(record.quote.clone()),
                    &effect,
                    div()
                        .flex()
                        .gap(px(SpacingScale::S2))
                        .child(reject)
                        .child(confirm),
                ));
            }
            if relations.len() > shown {
                list = list.child(self.more_row(
                    "sug-relations",
                    LIST_PAGE,
                    relations.len() - shown,
                    cx,
                ));
            }
            column = column.child(suggestion_section(
                theme,
                "Relações entre decisões",
                "Ao ler as decisões confirmadas, o xemnas notou que algumas se apoiam, \
                 substituem ou contradizem outras. Cada cartão diz quais duas decisões, \
                 qual é a relação e o trecho em que ela aparece.",
                list,
            ));
        }
        if !derived.is_empty() {
            let mut list = div().flex().flex_col();
            let shown = derived.len().min(self.limit("sug-context", LIST_PAGE));
            for (index, suggestion) in derived.iter().take(shown).enumerate() {
                let record = &suggestion.record;
                let confirm_id = record.suggestion_id.clone();
                let reject_id = record.suggestion_id.clone();
                let confirm = self.button(
                    format!("map-context-confirm-{index}"),
                    ButtonKind::Secondary,
                    !self.busy
                        && application::qualifiers::decode(&record.qualifiers).is_ok()
                        && serde_json::from_str::<Vec<String>>(&record.inherited_scope).is_ok(),
                    "Confirmar",
                    move |this, cx| {
                        let id = confirm_id.clone();
                        this.mutate_context(
                            cx,
                            "Regra criada a partir da decisão.",
                            move |backend| backend.context.confirm(&id).map(|_| ()),
                        );
                    },
                    cx,
                );
                let reject = self.button(
                    format!("map-context-reject-{index}"),
                    ButtonKind::Ghost,
                    !self.busy,
                    "Rejeitar",
                    move |this, cx| {
                        let id = reject_id.clone();
                        this.mutate_context(
                            cx,
                            "Sugestão rejeitada; ela não volta.",
                            move |backend| backend.context.reject(&id),
                        );
                    },
                    cx,
                );
                let qualifiers = application::qualifiers::decode(&record.qualifiers);
                let inherited = serde_json::from_str::<Vec<String>>(&record.inherited_scope);
                let qualification = match (qualifiers, inherited) {
                    (Ok(items), Ok(scope)) => {
                        super::review_editor::qualifier_reading(theme, &items)
                            .child(if scope.is_empty() {
                                "Escopo herdado não informado.".into()
                            } else {
                                format!("Escopo herdado: {}", scope.join("; "))
                            })
                            .into_any_element()
                    }
                    _ => error_banner(
                        theme,
                        "Não foi possível ler qualificadores ou escopo. Confirmação indisponível.",
                    )
                    .id(gpui::ElementId::Name(
                        format!("map-qualification-error-{index}").into(),
                    ))
                    .role(Role::Alert)
                    .child(self.button(
                        format!("map-qualification-retry-{index}"),
                        ButtonKind::Ghost,
                        !self.busy,
                        "Tentar novamente",
                        |this, cx| this.refresh(cx),
                        cx,
                    ))
                    .into_any_element(),
                };
                let scope = if suggestion.scope.is_empty() {
                    "Sem vínculos de escopo informados".to_owned()
                } else {
                    format!(
                        "Vale em {}",
                        suggestion
                            .scope
                            .iter()
                            .map(|(_, name)| name.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                };
                let kind_name = claim_label(record.kind.as_str());
                let lead = div()
                    .flex()
                    .flex_col()
                    .gap(px(SpacingScale::S2))
                    .child(rich_sentence(
                        theme,
                        &[
                            ("Da decisão".to_owned(), false),
                            (format!("“{}”", suggestion.decision_question), true),
                            (
                                format!(
                                    "o xemnas tirou {} {}:",
                                    claim_article(record.kind.as_str()),
                                    kind_name.to_lowercase()
                                ),
                                false,
                            ),
                        ],
                        TypeScale::BODY_SMALL,
                    ))
                    .child(
                        div()
                            .flex()
                            .items_start()
                            .gap(px(SpacingScale::S2))
                            .child(tag(theme, kind_name))
                            .child(
                                text_style(div(), TypeScale::BODY)
                                    .flex_1()
                                    .min_w(px(0.0))
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(colors.text_primary())
                                    .child(record.statement.clone()),
                            ),
                    )
                    .child(qualification);
                let effect = format!(
                    "vira {} {} do projeto ({}) e o agente passa a recebê-{} no contexto.",
                    claim_article(record.kind.as_str()),
                    kind_name.to_lowercase(),
                    scope.to_lowercase(),
                    claim_pronoun(record.kind.as_str()),
                );
                list = list.child(suggestion_card(
                    theme,
                    lead,
                    Some(record.quote.clone()),
                    &effect,
                    div()
                        .flex()
                        .gap(px(SpacingScale::S2))
                        .child(reject)
                        .child(confirm),
                ));
            }
            if derived.len() > shown {
                list =
                    list.child(self.more_row("sug-context", LIST_PAGE, derived.len() - shown, cx));
            }
            column = column.child(suggestion_section(
                theme,
                "Contexto sugerido",
                "Regras, premissas e restrições que parecem valer para o projeto, tiradas do \
                 texto de decisões já confirmadas. Confirmar transforma a frase em uma regra \
                 do projeto (aba Contexto); rejeitar descarta a frase e ela não volta.",
                list,
            ));
        }
        if !suggestions.is_empty() {
            let mut list = div().flex().flex_col();
            let shown = suggestions.len().min(self.limit("sug-links", LIST_PAGE));
            for (index, suggestion) in suggestions.iter().take(shown).enumerate() {
                let confirm_id = suggestion.edge_id.clone();
                let reject_id = suggestion.edge_id.clone();
                let confirm = self.button(
                    format!("map-confirm-{index}"),
                    ButtonKind::Secondary,
                    !self.busy,
                    "Confirmar",
                    move |this, cx| {
                        let id = confirm_id.clone();
                        this.mutate(cx, "Vínculo confirmado.", move |backend| {
                            backend.graph.confirm(&id).map(|_| None)
                        });
                    },
                    cx,
                );
                let reject = self.button(
                    format!("map-reject-{index}"),
                    ButtonKind::Ghost,
                    !self.busy,
                    "Rejeitar",
                    move |this, cx| {
                        let id = reject_id.clone();
                        this.mutate(cx, "Sugestão rejeitada; ela não volta.", move |backend| {
                            backend.graph.invalidate(&id).map(|_| None)
                        });
                    },
                    cx,
                );
                let (sentence, effect) = link_wording(suggestion);
                let lead = rich_sentence(theme, &sentence, TypeScale::BODY);
                list = list.child(suggestion_card(
                    theme,
                    lead,
                    None,
                    &effect,
                    div()
                        .flex()
                        .gap(px(SpacingScale::S2))
                        .child(reject)
                        .child(confirm),
                ));
            }
            // Several ties from the same evidence are usually confirmed
            // together; one action takes them all.
            let ids: Vec<String> = suggestions
                .iter()
                .map(|suggestion| suggestion.edge_id.clone())
                .collect();
            let all = (ids.len() > 1).then(|| {
                let count = ids.len();
                self.button(
                    "map-confirm-all",
                    ButtonKind::Secondary,
                    !self.busy,
                    format!("Confirmar os {count}"),
                    move |this, cx| {
                        let ids = ids.clone();
                        this.mutate(cx, "Vínculos confirmados.", move |backend| {
                            for id in ids {
                                backend.graph.confirm(&id)?;
                            }
                            Ok(None)
                        });
                    },
                    cx,
                )
            });
            column = column.child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(SpacingScale::S3))
                    .child(section_header(theme, "Vínculos sugeridos").children(all))
                    .child(
                        text_style(div(), TypeScale::BODY_SMALL)
                            .text_color(colors.text_muted())
                            .child(
                                "Decisões e regras que tocaram arquivos ou dependências ligados a \
                                 um componente ou tecnologia do Mapa. Confirmar liga os dois: a \
                                 decisão aparece na página do componente e acompanha quem edita \
                                 aqueles arquivos.",
                            ),
                    )
                    .child(list),
            );
        }
        if !components.is_empty() || !technologies.is_empty() {
            let mut list = div().flex().flex_col();
            let proposals = components
                .iter()
                .map(|proposal| {
                    (
                        EntityKind::Component,
                        proposal.name.clone(),
                        Some(proposal.pattern.clone()),
                        proposal.decisions,
                        proposal.description.clone(),
                        proposal.declared,
                    )
                })
                .chain(technologies.iter().map(|proposal| {
                    (
                        EntityKind::Technology,
                        proposal.name.clone(),
                        None,
                        proposal.decisions,
                        String::new(),
                        None,
                    )
                }));
            for (index, (kind, name, pattern, decisions, description, declared)) in
                proposals.enumerate()
            {
                let create_description = description.clone();
                let project = self.project.clone().unwrap_or_default();
                let (create_name, create_pattern) = (name.clone(), pattern.clone());
                let create = self.button(
                    format!("map-create-{index}"),
                    ButtonKind::Secondary,
                    !self.busy,
                    "Criar",
                    move |this, cx| {
                        let input = NewEntity {
                            project_id: project.clone(),
                            kind: Some(kind),
                            name: create_name.clone(),
                            description: create_description.clone(),
                            patterns: create_pattern.clone().into_iter().collect(),
                            ..NewEntity::default()
                        };
                        this.mutate(
                            cx,
                            "Item criado. As sugestões foram atualizadas.",
                            move |backend| backend.graph.create_entity(input).map(|_| None),
                        );
                    },
                    cx,
                );
                let (sentence, effect) =
                    item_wording(kind, &name, pattern.as_deref(), declared, decisions);
                let lead = div()
                    .flex()
                    .flex_col()
                    .gap(px(SpacingScale::S2))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(SpacingScale::S2))
                            .child(tag(theme, kind_label(kind)))
                            .child(
                                text_style(div(), TypeScale::ROW_TITLE)
                                    .text_color(colors.text_primary())
                                    .child(name.clone()),
                            ),
                    )
                    .child(rich_sentence(theme, &sentence, TypeScale::BODY))
                    .when(!description.is_empty(), |lead| {
                        lead.child(
                            text_style(div(), TypeScale::BODY_SMALL)
                                .text_color(colors.text_muted())
                                .child(description.clone()),
                        )
                    });
                list = list.child(suggestion_card(
                    theme,
                    lead,
                    None,
                    &effect,
                    div().flex().child(create),
                ));
            }
            // Declared members can be taken together.
            let declared: Vec<NewEntity> = components
                .iter()
                .filter(|proposal| proposal.declared.is_some())
                .map(|proposal| NewEntity {
                    project_id: self.project.clone().unwrap_or_default(),
                    kind: Some(EntityKind::Component),
                    name: proposal.name.clone(),
                    description: proposal.description.clone(),
                    patterns: vec![proposal.pattern.clone()],
                    ..NewEntity::default()
                })
                .collect();
            let all = (declared.len() > 1).then(|| {
                let count = declared.len();
                self.button(
                    "map-create-declared",
                    ButtonKind::Secondary,
                    !self.busy,
                    format!("Criar os {count} do workspace"),
                    move |this, cx| {
                        let inputs = declared.clone();
                        this.mutate(cx, "Componentes do workspace criados.", move |backend| {
                            for input in inputs {
                                match backend.graph.create_entity(input) {
                                    Ok(_) | Err(application::graph::GraphError::DuplicateName) => {}
                                    Err(error) => return Err(error),
                                }
                            }
                            Ok(None)
                        });
                    },
                    cx,
                )
            });
            column = column.child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(SpacingScale::S3))
                    .child(section_header(theme, "Itens sugeridos").children(all))
                    .child(list),
            );
        }
        reading_page("map-suggestions", column).into_any_element()
    }

    fn render_entity(&mut self, id: &str, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
        let colors = theme.colors;
        let Some(snapshot) = self
            .detail
            .clone()
            .filter(|snapshot| snapshot.0.entity.entity_id == id)
        else {
            return reading_page(
                "map-entity-loading",
                skeleton_list(theme, "map-entity-skeleton", 5),
            )
            .into_any_element();
        };
        let (detail, events) = (&snapshot.0, &snapshot.1);
        let entity = detail.entity.clone();
        let component = entity.kind == EntityKind::Component;
        let busy = self.busy;

        let mut actions: Vec<AnyElement> = vec![
            self.button(
                "map-link-decision",
                ButtonKind::Secondary,
                !busy,
                "Vincular decisão",
                |this, cx| this.open_picker(PickKind::Decision, cx),
                cx,
            )
            .into_any_element(),
            self.button(
                "map-link-claim",
                ButtonKind::Secondary,
                !busy,
                "Vincular regra",
                |this, cx| this.open_picker(PickKind::Claim, cx),
                cx,
            )
            .into_any_element(),
        ];
        if component {
            actions.push(
                self.button(
                    "map-link-parent",
                    ButtonKind::Ghost,
                    !busy,
                    "Faz parte de…",
                    |this, cx| this.open_picker(PickKind::Parent, cx),
                    cx,
                )
                .into_any_element(),
            );
        }
        let edit_id = entity.entity_id.clone();
        actions.push(
            self.button(
                "map-edit",
                ButtonKind::Ghost,
                !busy,
                "Editar",
                move |this, cx| this.open_view(View::Form(Some(edit_id.clone())), cx),
                cx,
            )
            .into_any_element(),
        );
        actions.push(
            self.button(
                "map-retire",
                ButtonKind::Ghost,
                !busy,
                "Aposentar",
                |this, cx| {
                    this.confirm_retire = true;
                    cx.notify();
                },
                cx,
            )
            .into_any_element(),
        );

        let confirmation = self.confirm_retire.then(|| {
            let retire_id = entity.entity_id.clone();
            let cancel = self.button("map-retire-cancel", ButtonKind::Ghost, true, "Cancelar", |this, cx| {
                this.confirm_retire = false;
                cx.notify();
            }, cx);
            let confirm = self.button("map-retire-confirm", ButtonKind::Secondary, !busy, "Aposentar item", move |this, cx| {
                let id = retire_id.clone();
                this.view = View::Overview;
                this.mutate(cx, "Item aposentado; a história continua.", move |backend| backend.graph.retire_entity(&id).map(|_| None));
            }, cx);
            div()
                .id("map-retire-confirmation")
                .flex()
                .flex_col()
                .gap(px(SpacingScale::S3))
                .p(px(SpacingScale::S3))
                .rounded(theme.radius.control())
                .border_1()
                .border_color(colors.hairline_divider())
                .role(Role::Alert)
                .child(
                    text_style(div(), TypeScale::BODY_SMALL)
                        .text_color(colors.text_secondary())
                        .child("Aposentar tira o item do mapa e desfaz os vínculos dele. Nada é apagado: a linha do tempo continua mostrando o que valeu."),
                )
                .child(div().flex().justify_end().gap(px(SpacingScale::S2)).child(cancel).child(confirm))
        });

        let picker = self.render_picker(theme, cx);

        let mut column = div().flex().flex_col().gap(px(SpacingScale::S8)).child(
            div()
                .flex()
                .flex_col()
                .gap(px(SpacingScale::S2))
                .child(section_label(theme, kind_label(entity.kind)))
                .child(text_style(div(), TypeScale::HEADING_1).child(entity.name.clone()))
                .when(!entity.description.is_empty(), |header| {
                    header.child(
                        text_style(div(), TypeScale::BODY)
                            .text_color(colors.text_secondary())
                            .child(entity.description.clone()),
                    )
                })
                .when(!entity.patterns.is_empty(), |header| {
                    header.child(
                        text_style(div(), TypeScale::BODY_SMALL)
                            .font_family(Theme::font_mono())
                            .text_color(colors.text_muted())
                            .child(entity.patterns.join("  ·  ")),
                    )
                })
                .when(!entity.aliases.is_empty(), |header| {
                    header.child(
                        text_style(div(), TypeScale::META)
                            .text_color(colors.text_muted())
                            .child(format!("Também chamado de {}", entity.aliases.join(", "))),
                    )
                })
                .child(
                    div()
                        .pt(px(SpacingScale::S2))
                        .flex()
                        .flex_wrap()
                        .gap(px(SpacingScale::S2))
                        .children(actions),
                )
                .children(confirmation)
                .children(picker),
        );

        let diagram = self.neighborhood_diagram(theme, detail, cx);
        column = column.child(section(theme, "Vizinhança", diagram));
        let decisions = self.node_list(
            theme,
            "map-decisions",
            &detail.decisions,
            true,
            "Nenhuma decisão ligada. Vincule uma ou confirme sugestões.",
            cx,
        );
        column = column.child(section(theme, "Decisões em vigor", decisions));
        if !detail.conflicts.is_empty() {
            let mut list = div().flex().flex_col();
            for (index, (left, right)) in detail.conflicts.iter().enumerate() {
                list = list.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(SpacingScale::S3))
                        .py(px(SpacingScale::S2))
                        .when(index > 0, |row| {
                            row.border_t_1().border_color(colors.hairline_divider())
                        })
                        .child(icon(IconName::Info, 16.0, colors.status_warning()))
                        .child(
                            text_style(div(), TypeScale::BODY_SMALL)
                                .flex_1()
                                .min_w(px(0.0))
                                .child(format!("{}  ×  {}", left.label, right.label)),
                        ),
                );
            }
            column = column.child(section(theme, "Conflitos neste item", list));
        }
        let claims = self.node_list(
            theme,
            "map-claims",
            &detail.claims,
            false,
            "Nenhuma regra ligada.",
            cx,
        );
        column = column.child(section(theme, "Regras que se aplicam", claims));
        if component {
            let mut structure = div().flex().flex_col();
            let parent: Vec<NodeSummary> = detail.parent.clone().into_iter().collect();
            if !parent.is_empty() {
                structure = structure.child(self.entity_list(
                    theme,
                    "map-parent",
                    "Faz parte de",
                    &parent,
                    cx,
                ));
            }
            if !detail.parts.is_empty() {
                structure = structure.child(self.entity_list(
                    theme,
                    "map-parts",
                    "Partes",
                    &detail.parts,
                    cx,
                ));
            }
            if !parent.is_empty() || !detail.parts.is_empty() {
                column = column.child(section(theme, "Estrutura", structure));
            }
        }
        if !detail.impact.is_empty() {
            let impact = self.node_list(theme, "map-impact", &detail.impact, true, "", cx);
            column = column.child(section(
                theme,
                "Impacto: decisões que dependem deste item",
                impact,
            ));
        }
        let (timeline, hidden) = timeline_list(
            theme,
            events,
            self.limit("map-entity-timeline", TIMELINE_PAGE),
        );
        let timeline = if hidden > 0 {
            timeline.child(self.more_row("map-entity-timeline", TIMELINE_PAGE, hidden, cx))
        } else {
            timeline
        };
        column = column.child(section(theme, "Linha do tempo", timeline));
        reading_page(format!("map-entity-{id}"), column).into_any_element()
    }

    fn render_picker(&mut self, theme: &Theme, cx: &mut Context<Self>) -> Option<Div> {
        let picker = self.picker.as_ref()?;
        let colors = theme.colors;
        let items = self.filtered(picker, cx);
        let kind = picker.kind;
        let title = match kind {
            PickKind::Decision => "Escolha a decisão",
            PickKind::Claim => "Escolha a regra",
            PickKind::Parent => "Escolha o componente que contém este",
        };
        let close = self.button(
            "map-picker-close",
            ButtonKind::Ghost,
            true,
            "Fechar",
            |this, cx| {
                this.picker = None;
                cx.notify();
            },
            cx,
        );
        let mut list = div()
            .id("map-picker-list")
            .max_h(px(280.0))
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap(px(2.0));
        if items.is_empty() {
            list = list.child(
                text_style(div(), TypeScale::BODY_SMALL)
                    .p(px(SpacingScale::S2))
                    .text_color(colors.text_muted())
                    .child("Nada para vincular aqui."),
            );
        }
        for (index, (_, label, detail)) in items.iter().enumerate() {
            let row = self
                .row(
                    format!("map-pick-{index}"),
                    false,
                    label,
                    move |this, cx| this.pick(index, cx),
                    cx,
                )
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.0))
                        .flex()
                        .flex_col()
                        .child(
                            text_style(div(), TypeScale::ROW_TITLE)
                                .truncate()
                                .child(label.clone()),
                        )
                        .child(
                            text_style(div(), TypeScale::META)
                                .truncate()
                                .text_color(colors.text_muted())
                                .child(detail.clone()),
                        ),
                );
            list = list.child(row);
        }
        Some(
            div()
                .flex()
                .flex_col()
                .gap(px(SpacingScale::S2))
                .p(px(SpacingScale::S3))
                .rounded(RadiusScale.surface())
                .border_1()
                .border_color(colors.glass_border_card())
                .bg(colors.glass_fill_card())
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(SpacingScale::S2))
                        .child(
                            text_style(div(), TypeScale::ROW_TITLE)
                                .flex_1()
                                .child(title),
                        )
                        .child(close),
                )
                .child(self.filter.clone())
                .child(list),
        )
    }

    fn node_list(
        &mut self,
        theme: &Theme,
        prefix: &'static str,
        rows: &[NodeSummary],
        decisions: bool,
        empty: &'static str,
        cx: &mut Context<Self>,
    ) -> Div {
        let colors = theme.colors;
        let mut list = div().flex().flex_col().gap(px(2.0));
        if rows.is_empty() && !empty.is_empty() {
            return list.child(
                text_style(div(), TypeScale::BODY_SMALL)
                    .text_color(colors.text_muted())
                    .child(empty),
            );
        }
        let key = prefix;
        let shown = rows.len().min(self.limit(key, LIST_PAGE));
        for (index, row) in rows.iter().take(shown).enumerate() {
            let id = row.node.id.clone();
            let body = div()
                .flex_1()
                .min_w(px(0.0))
                .flex()
                .flex_col()
                .gap(px(2.0))
                .child(text_style(div(), TypeScale::ROW_TITLE).child(row.label.clone()))
                .child(
                    text_style(div(), TypeScale::BODY_SMALL)
                        .text_color(colors.text_muted())
                        .child(if decisions {
                            format!("{} · {}", row.detail, short_date(&row.at))
                        } else {
                            format!(
                                "{} · desde {}",
                                claim_label(&row.detail),
                                calendar_date(&row.at)
                            )
                        }),
                );
            if decisions {
                let element = self
                    .row(
                        format!("{prefix}-{index}"),
                        false,
                        &row.label,
                        move |_, cx| cx.emit(OpenDecision(id.clone())),
                        cx,
                    )
                    .tooltip(tooltip("Abrir em Decisões", None))
                    .child(body)
                    .child(icon(IconName::ChevronRight, 14.0, colors.text_muted()));
                list = list.child(element);
            } else {
                list = list.child(
                    div()
                        .px(px(SpacingScale::S3))
                        .py(px(SpacingScale::S2))
                        .flex()
                        .child(body),
                );
            }
        }
        if rows.len() > shown {
            list = list.child(self.more_row(key, LIST_PAGE, rows.len() - shown, cx));
        }
        list
    }

    fn entity_list(
        &mut self,
        theme: &Theme,
        prefix: &str,
        title: &str,
        rows: &[NodeSummary],
        cx: &mut Context<Self>,
    ) -> Div {
        let colors = theme.colors;
        let mut list = div().flex().flex_col().gap(px(2.0)).child(
            text_style(div(), TypeScale::META)
                .px(px(SpacingScale::S3))
                .text_color(colors.text_muted())
                .child(title.to_owned()),
        );
        for (index, row) in rows.iter().enumerate() {
            let id = row.node.id.clone();
            let element = self
                .row(
                    format!("{prefix}-{index}"),
                    false,
                    &row.label,
                    move |this, cx| this.open_entity(id.clone(), cx),
                    cx,
                )
                .child(icon(IconName::Component, 16.0, colors.text_muted()))
                .child(
                    text_style(div(), TypeScale::ROW_TITLE)
                        .flex_1()
                        .child(row.label.clone()),
                );
            list = list.child(element);
        }
        list
    }

    fn render_file(&mut self, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
        let colors = theme.colors;
        let has_path = !self.value(&self.path, cx).is_empty();
        let show = self.button(
            "map-lens-run",
            ButtonKind::Primary,
            has_path && !self.busy,
            "Ver o que vale",
            |this, cx| this.open_lens(cx),
            cx,
        );
        let mut column = div()
            .flex()
            .flex_col()
            .gap(px(SpacingScale::S8))
            .child(page_header(
                theme,
                "Lente de arquivo",
                "O que vale para um arquivo: os componentes que o contêm, as decisões em vigor e as regras ligadas a eles.",
            ))
            .child(
                div()
                    .flex()
                    .items_end()
                    .gap(px(SpacingScale::S3))
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .gap(px(SpacingScale::S2))
                            .child(section_label(theme, "Caminho relativo ao projeto"))
                            .child(self.path.clone()),
                    )
                    .child(show),
            );
        let recent = self
            .data
            .as_ref()
            .map(|data| data.recent_files.clone())
            .unwrap_or_default();
        if self.lens.is_none() && !recent.is_empty() {
            let mut picks = div().flex().flex_col().gap(px(2.0));
            for (index, file) in recent.into_iter().enumerate() {
                let pick = file.clone();
                picks = picks.child(
                    self.row(
                        format!("map-lens-recent-{index}"),
                        false,
                        &file,
                        move |this, cx| {
                            let value = pick.clone();
                            this.path
                                .update(cx, |field, cx| field.set_value(&value, cx));
                            this.open_lens(cx);
                        },
                        cx,
                    )
                    .child(icon(IconName::File, 14.0, colors.text_muted()))
                    .child(
                        text_style(div(), TypeScale::BODY_SMALL)
                            .flex_1()
                            .min_w(px(0.0))
                            .truncate()
                            .font_family(Theme::font_mono())
                            .text_color(colors.text_secondary())
                            .child(file),
                    ),
                );
            }
            column = column.child(section(theme, "Arquivos das últimas decisões", picks));
        }
        if let Some(lens) = self.lens.clone() {
            if lens.components.is_empty() {
                column = column.child(
                    text_style(div(), TypeScale::BODY)
                        .text_color(colors.text_secondary())
                        .child(format!("Nenhum componente cobre {}. Crie um com um padrão de caminho que o inclua.", lens.path)),
                );
            } else {
                column = column.child(section(
                    theme,
                    "Componentes",
                    div().child(
                        text_style(div(), TypeScale::BODY).child(
                            lens.components
                                .iter()
                                .map(|row| row.label.clone())
                                .collect::<Vec<_>>()
                                .join(" › "),
                        ),
                    ),
                ));
                let decisions = self.node_list(
                    theme,
                    "map-lens-decisions",
                    &lens.decisions,
                    true,
                    "Nenhuma decisão em vigor ligada a esses componentes.",
                    cx,
                );
                column = column.child(section(theme, "Decisões em vigor", decisions));
                let claims = self.node_list(
                    theme,
                    "map-lens-claims",
                    &lens.claims,
                    false,
                    "Nenhuma regra ligada.",
                    cx,
                );
                column = column.child(section(theme, "Regras", claims));
            }
        }
        reading_page("map-file", column).into_any_element()
    }

    fn render_timeline(&mut self, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
        let Some(events) = self.timeline.clone() else {
            return reading_page(
                "map-timeline-loading",
                skeleton_list(theme, "map-timeline-skeleton", 6),
            )
            .into_any_element();
        };
        let (timeline, hidden) =
            timeline_list(theme, &events, self.limit("map-timeline", TIMELINE_PAGE));
        let timeline = if hidden > 0 {
            timeline.child(self.more_row("map-timeline", TIMELINE_PAGE, hidden, cx))
        } else {
            timeline
        };
        let column = div()
            .flex()
            .flex_col()
            .gap(px(SpacingScale::S8))
            .child(page_header(
                theme,
                "Linha do tempo",
                "O que passou a valer e o que deixou de valer no projeto, do mais recente ao mais antigo.",
            ))
            .child(timeline);
        reading_page("map-timeline", column).into_any_element()
    }

    fn render_form(
        &mut self,
        editing: Option<String>,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let colors = theme.colors;
        let named = !self.value(&self.name, cx).is_empty();
        let save_editing = editing.clone();
        let save = self.button(
            "map-form-save",
            ButtonKind::Primary,
            named && !self.busy,
            if editing.is_some() { "Salvar" } else { "Criar" },
            move |this, cx| this.save_form(save_editing.clone(), cx),
            cx,
        );
        let back = editing.clone();
        let cancel = self.button(
            "map-form-cancel",
            ButtonKind::Ghost,
            true,
            "Cancelar",
            move |this, cx| {
                let view = back.clone().map_or(View::Overview, View::Entity);
                this.open_view(view, cx)
            },
            cx,
        );
        let kinds: Vec<AnyElement> = EntityKind::ALL
            .into_iter()
            .map(|kind| {
                let selected = self.form_kind == kind;
                let locked = editing.is_some();
                let id = format!("map-form-kind-{}", kind.as_str());
                let focus = self.focus_for(&id, cx);
                mark_selected(
                    div()
                        .id(SharedString::from(id))
                        .relative()
                        .flex_1()
                        .flex()
                        .items_center()
                        .gap(px(SpacingScale::S3))
                        .px(px(SpacingScale::S3))
                        .py(px(SpacingScale::S2))
                        .rounded(theme.radius.control())
                        .border_1()
                        .border_color(colors.hairline_divider())
                        .when(!locked, |option| option.cursor_pointer())
                        .role(Role::RadioButton)
                        .aria_label(kind_label(kind))
                        .aria_toggled(if selected {
                            Toggled::True
                        } else {
                            Toggled::False
                        })
                        .track_focus(&focus)
                        .focus_visible(crate::ui::controls::focus_ring(theme))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            if !locked {
                                this.form_kind = kind;
                                cx.notify();
                            }
                        }))
                        .child(icon(
                            kind_icon(kind),
                            16.0,
                            if selected {
                                colors.text_primary()
                            } else {
                                colors.text_muted()
                            },
                        ))
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .child(
                                    text_style(div(), TypeScale::ROW_TITLE).child(kind_label(kind)),
                                )
                                .child(
                                    text_style(div(), TypeScale::META)
                                        .text_color(colors.text_muted())
                                        .child(match kind {
                                            EntityKind::Component => {
                                                "Uma parte do projeto, achada por caminhos"
                                            }
                                            EntityKind::Technology => {
                                                "Linguagem, biblioteca ou serviço usado"
                                            }
                                        }),
                                ),
                        ),
                    theme,
                    selected,
                )
                .into_any_element()
            })
            .collect();
        let field = |label: &str, hint: &str, control: Entity<SearchField>| {
            div()
                .flex()
                .flex_col()
                .gap(px(SpacingScale::S2))
                .child(section_label(theme, label))
                .child(control)
                .child(
                    text_style(div(), TypeScale::META)
                        .text_color(colors.text_muted())
                        .child(hint.to_owned()),
                )
        };
        let mut column = div()
            .flex()
            .flex_col()
            .gap(px(SpacingScale::S6))
            .child(page_header(
                theme,
                if editing.is_some() { "Editar item" } else { "Novo item do mapa" },
                "Componentes ligam decisões a caminhos do projeto; tecnologias, às dependências que elas adotam.",
            ))
            .child(div().id("map-form-kind").flex().gap(px(SpacingScale::S3)).role(Role::RadioGroup).aria_label("Tipo").children(kinds))
            .child(field("Nome", "Como o time chama esta parte.", self.name.clone()));
        if self.form_kind == EntityKind::Component {
            column = column.child(field(
                "Padrões de caminho",
                "Separados por vírgula. * vale dentro de uma pasta, ** atravessa pastas; sem curinga, vale a pasta inteira.",
                self.patterns.clone(),
            ));
        }
        column = column
            .child(field(
                "Apelidos",
                "Outros nomes, separados por vírgula. Resolvem dependências com nome diferente.",
                self.aliases.clone(),
            ))
            .child(field("Descrição", "Opcional.", self.description.clone()))
            .child(
                div()
                    .flex()
                    .justify_end()
                    .gap(px(SpacingScale::S2))
                    .child(cancel)
                    .child(save),
            );
        reading_page("map-form", column).into_any_element()
    }
}

impl<S: MapStores> MapScreen<S> {
    /// The project at a glance, like a C4 container view drawn by the real
    /// decisions: one block per top-level component, its parts inside, a
    /// square per decision in force, and technologies as chips.
    /// "Blocos | Grafo": two drawings of the same map.
    fn layout_switch(&mut self, theme: &Theme, cx: &mut Context<Self>) -> Div {
        let mut track = segmented(theme);
        for (layout, id, glyph, label) in [
            (
                Layout::Blocks,
                "map-layout-blocks",
                IconName::Blocks,
                "Blocos",
            ),
            (Layout::Graph, "map-layout-graph", IconName::Graph, "Grafo"),
        ] {
            let focus = self.focus_for(id, cx);
            track = track.child(
                segment(theme, id, glyph, label, self.layout == layout)
                    .track_focus(&focus)
                    .on_click(cx.listener(move |this, _, _, cx| this.set_layout(layout, cx)))
                    .on_key_down(cx.listener(move |this, event: &gpui::KeyDownEvent, _, cx| {
                        if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                            this.set_layout(layout, cx);
                            cx.stop_propagation();
                        }
                    })),
            );
        }
        track
    }

    /// The graph fills the whole content area under a short header.
    fn render_graph(&mut self, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
        let switch = self.layout_switch(theme, cx);
        div()
            .size_full()
            .flex()
            .flex_col()
            .gap(px(SpacingScale::S5))
            .px(px(SpacingScale::S8))
            .pt(px(SpacingScale::S8))
            .pb(px(SpacingScale::S6))
            .child(
                // The header sits where the Blocks page puts it, so switching
                // drawings moves nothing but the drawing.
                div()
                    .w_full()
                    .max_w(px(READING_WIDTH))
                    .mx_auto()
                    .flex()
                    .items_start()
                    .gap(px(SpacingScale::S4))
                    .child(div().flex_1().min_w(px(0.0)).child(page_header(
                        theme,
                        "Mapa do projeto",
                        "Cada ilha é um componente com o que vale nele. Passe o mouse para ver as ligações; clique para abrir.",
                    )))
                    .child(switch),
            )
            .child(
                div()
                    .flex_1()
                    .min_h(px(0.0))
                    .child(self.graph.clone()),
            )
            .into_any_element()
    }

    fn render_overview(&mut self, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
        let Some(data) = self.data.as_ref() else {
            return div().into_any_element();
        };
        let map = Arc::clone(&data.map);
        let entities = &map.entities;
        if entities.is_empty() {
            let open = self.button(
                "map-overview-suggestions",
                ButtonKind::Primary,
                true,
                "Ver sugestões",
                |_, cx| cx.emit(OpenSuggestions),
                cx,
            );
            return reading_page(
                "map-overview-empty",
                div()
                    .flex()
                    .flex_col()
                    .gap(px(SpacingScale::S4))
                    .child(page_header(
                        theme,
                        "Mapa do projeto",
                        "Ainda vazio. Componentes ligam as decisões às partes do código; comece pelas sugestões, que vêm dos arquivos que as decisões tocaram.",
                    ))
                    .child(div().flex().child(open)),
            )
            .into_any_element();
        }
        if self.layout == Layout::Graph {
            return self.render_graph(theme, cx);
        }
        // What the Blocos layout needs from the map, found once per map (not
        // per row, per frame): which entities are roots, technologies, and
        // each block's parts.
        if self
            .overview_index
            .as_ref()
            .is_none_or(|index| !Arc::ptr_eq(&index.map, &map))
        {
            self.overview_index = Some(Arc::new(OverviewIndex::new(Arc::clone(&map))));
        }
        let rows = overview_rows(self.overview_index.as_deref().expect("index just built"));
        if rows != self.overview_rows {
            let old = self.overview_rows.len();
            self.overview_list.splice(0..old, rows.len());
            self.overview_rows = rows;
        }
        let list = list(
            self.overview_list.clone(),
            cx.processor(|this, ix: usize, _, cx| this.overview_item(ix, cx)),
        )
        .size_full();
        div()
            .id("map-overview")
            .relative()
            .flex_1()
            .min_h(px(0.0))
            .child(list)
            .child(scroll_thumb(
                theme,
                &self.overview_list,
                &self.overview_scroll,
            ))
            .into_any_element()
    }

    /// One row of the Blocos layout, built when the virtual list asks.
    fn overview_item(&mut self, ix: usize, cx: &mut Context<Self>) -> AnyElement {
        let theme = Theme::current(cx);
        let colors = theme.colors;
        let (Some(&row), Some(index)) = (self.overview_rows.get(ix), self.overview_index.clone())
        else {
            return div().into_any_element();
        };
        let map = Arc::clone(&index.map);
        let last = ix + 1 == self.overview_rows.len();
        let element = match row {
            OverviewRow::Header => {
                let switch = self.layout_switch(&theme, cx);
                div()
                    .flex()
                    .items_start()
                    .gap(px(SpacingScale::S4))
                    .child(div().flex_1().min_w(px(0.0)).child(page_header(
                        &theme,
                        "Mapa do projeto",
                        "A arquitetura desenhada pelas decisões confirmadas. Abra um bloco para ver o que vale ali.",
                    )))
                    .child(switch)
                    .into_any_element()
            }
            OverviewRow::Label(label) => div()
                .pt(px(SpacingScale::S8))
                .pb(px(SpacingScale::S3))
                .child(section_label(&theme, label))
                .into_any_element(),
            OverviewRow::Blocks(first) => {
                let recent_since = (chrono::Utc::now() - chrono::Duration::days(RECENT_DAYS))
                    .format("%Y-%m-%dT%H:%M:%SZ")
                    .to_string();
                let pair = &index.tops[first..(first + 2).min(index.tops.len())];
                // Blocks on one line share a height; weight is in the text.
                let mut line = div()
                    .flex()
                    .gap(px(SpacingScale::S3))
                    .items_stretch()
                    .pb(px(SpacingScale::S3));
                for &top in pair {
                    let entity = &map.entities[top];
                    let parts: Vec<&MapEntity> = index
                        .parts
                        .get(entity.entity.entity_id.as_str())
                        .map(|parts| parts.iter().map(|&at| &map.entities[at]).collect())
                        .unwrap_or_default();
                    let block = self.component_block(&theme, entity, &parts, &recent_since, cx);
                    line = line.child(div().flex_1().min_w(px(0.0)).flex().child(block.flex_1()));
                }
                if pair.len() == 1 {
                    line = line.child(div().flex_1());
                }
                line.into_any_element()
            }
            OverviewRow::Technologies(first) => {
                let mut chips = div()
                    .flex()
                    .flex_wrap()
                    .gap(px(SpacingScale::S2))
                    .pb(px(SpacingScale::S2));
                for &at in
                    &index.technologies[first..(first + TECH_ROW).min(index.technologies.len())]
                {
                    let row = &map.entities[at];
                    let id = row.entity.entity_id.clone();
                    chips = chips.child(
                        self.row(
                            format!("map-tech-{id}"),
                            false,
                            &row.entity.name,
                            move |this, cx| this.open_entity(id.clone(), cx),
                            cx,
                        )
                        .border_1()
                        .border_color(colors.glass_border_card())
                        .bg(colors.glass_fill_card())
                        .child(icon(IconName::Cpu, 14.0, colors.text_muted()))
                        .child(
                            text_style(div(), TypeScale::ROW_TITLE).child(row.entity.name.clone()),
                        )
                        .child(
                            text_style(div(), TypeScale::META)
                                .text_color(colors.text_muted())
                                .child(row.decisions.to_string()),
                        ),
                    );
                }
                chips.into_any_element()
            }
            OverviewRow::Conflict => div()
                .pt(px(SpacingScale::S6))
                .child(legend_item(
                    &theme,
                    status_dot(colors.status_warning()),
                    "decisões em conflito",
                ))
                .into_any_element(),
        };
        // The reading column, centred; the page's own padding at the ends.
        div()
            .w_full()
            .px(px(SpacingScale::S8))
            .when(ix == 0, |row| row.pt(px(SpacingScale::S8)))
            .when(last, |row| row.pb(px(SpacingScale::S8)))
            .child(
                div()
                    .w_full()
                    .max_w(px(READING_WIDTH))
                    .mx_auto()
                    .child(element),
            )
            .into_any_element()
    }

    fn component_block(
        &mut self,
        theme: &Theme,
        row: &MapEntity,
        parts: &[&MapEntity],
        recent_since: &str,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let colors = theme.colors;
        let id = row.entity.entity_id.clone();
        let recent = row
            .last_activity
            .as_deref()
            .is_some_and(|at| at >= recent_since);
        let focus = self.focus_for(&format!("map-block-{id}"), cx);
        let open_id = id.clone();
        let key_id = id.clone();
        let activity = row
            .last_activity
            .as_deref()
            .filter(|_| recent)
            .map(|at| format!("mudou em {}", short_date(at)));
        let mut parts_row = div().flex().flex_wrap().gap(px(SpacingScale::S2));
        for part in parts.iter().take(PART_CHIPS) {
            let part_id = part.entity.entity_id.clone();
            parts_row = parts_row.child(
                self.row(
                    format!("map-part-{part_id}"),
                    false,
                    &part.entity.name,
                    move |this, cx| this.open_entity(part_id.clone(), cx),
                    cx,
                )
                .border_1()
                .border_color(colors.hairline_divider())
                .child(text_style(div(), TypeScale::META).child(part.entity.name.clone()))
                .child(
                    text_style(div(), TypeScale::META)
                        .text_color(colors.text_muted())
                        .child(part.decisions.to_string()),
                ),
            );
        }
        // A block shows a handful of parts and counts the rest: the list of
        // all of them lives on the component's page.
        if parts.len() > PART_CHIPS {
            parts_row = parts_row.child(
                text_style(div(), TypeScale::META)
                    .px(px(SpacingScale::S2))
                    .py(px(SpacingScale::S2))
                    .text_color(colors.text_muted())
                    .child(format!("+{} partes", parts.len() - PART_CHIPS)),
            );
        }
        div()
            .id(SharedString::from(format!("map-block-{id}")))
            .flex()
            .flex_col()
            .gap(px(SpacingScale::S2))
            .p(px(SpacingScale::S4))
            .rounded(RadiusScale.surface())
            .border_1()
            .border_color(colors.glass_border_card())
            .bg(colors.glass_fill_card())
            .cursor_pointer()
            .hover(move |style| {
                style
                    .border_color(colors.glass_border_card_hover())
                    .bg(colors.glass_fill_medium())
            })
            .role(Role::Button)
            .aria_label(format!("{}: {}", row.entity.name, weight(row)))
            .track_focus(&focus)
            .focus_visible(crate::ui::controls::focus_ring(theme))
            .on_click(cx.listener(move |this, _, _, cx| this.open_entity(open_id.clone(), cx)))
            .on_key_down(cx.listener(move |this, event: &gpui::KeyDownEvent, _, cx| {
                if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                    this.open_entity(key_id.clone(), cx);
                    cx.stop_propagation();
                }
            }))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(SpacingScale::S2))
                    .child(
                        text_style(div(), TypeScale::HEADING_3)
                            .flex_1()
                            .min_w(px(0.0))
                            .truncate()
                            .child(row.entity.name.clone()),
                    )
                    .when(row.conflicts > 0, |line| {
                        line.child(status_dot(colors.status_warning()))
                    }),
            )
            .when(!row.entity.patterns.is_empty(), |block| {
                block.child(
                    text_style(div(), TypeScale::META)
                        .font_family(Theme::font_mono())
                        .truncate()
                        .text_color(colors.text_muted())
                        .child(row.entity.patterns.join("  ")),
                )
            })
            .when(!parts.is_empty(), |block| {
                block.child(parts_row.mt(px(SpacingScale::S1)))
            })
            .child(div().flex_1())
            .child(
                div()
                    .flex()
                    .items_baseline()
                    .gap(px(SpacingScale::S2))
                    .pt(px(SpacingScale::S2))
                    .child(
                        text_style(div(), TypeScale::META)
                            .flex_1()
                            .text_color(if row.decisions + row.claims == 0 {
                                colors.text_muted()
                            } else {
                                colors.text_secondary()
                            })
                            .child(weight(row)),
                    )
                    .children(activity.map(|line| {
                        text_style(div(), TypeScale::META)
                            .text_color(colors.text_muted())
                            .child(line)
                    })),
            )
    }

    /// One entity in the middle, the decisions tied to it on the left and the
    /// rules on the right, joined by curves: a layered ego view, never the
    /// whole graph.
    fn neighborhood_diagram(
        &mut self,
        theme: &Theme,
        detail: &EntityDetail,
        cx: &mut Context<Self>,
    ) -> Div {
        let colors = theme.colors;
        let left: Vec<NodeSummary> = detail.decisions.iter().take(MAX_SIDE).cloned().collect();
        let right: Vec<NodeSummary> = detail.claims.iter().take(MAX_SIDE).cloned().collect();
        let hidden = detail.decisions.len().saturating_sub(MAX_SIDE)
            + detail.claims.len().saturating_sub(MAX_SIDE);
        let rows = left.len().max(right.len()).max(1);
        let height = rows as f32 * NODE_H + (rows - 1) as f32 * NODE_GAP;
        let (left_count, right_count) = (left.len(), right.len());
        let line = Hsla::from(colors.glass_border_card_hover());
        let dashed = Hsla::from(colors.text_muted()).opacity(0.6);
        let edges = canvas(
            |_, _, _| {},
            move |bounds, _, window, _| {
                let column = (bounds.size.width - px(2.0 * COLUMN_GAP)) / 3.0;
                let top = bounds.origin.y;
                let center_y = top + px(height / 2.0);
                let center_left = bounds.origin.x + column + px(COLUMN_GAP);
                let center_right = center_left + column;
                let node_y = |count: usize, index: usize| {
                    let stack = count as f32 * NODE_H + count.saturating_sub(1) as f32 * NODE_GAP;
                    top + px((height - stack) / 2.0
                        + index as f32 * (NODE_H + NODE_GAP)
                        + NODE_H / 2.0)
                };
                for index in 0..left_count {
                    let start = point(bounds.origin.x + column, node_y(left_count, index));
                    let end = point(center_left, center_y);
                    curve(window, start, end, line, false);
                }
                for index in 0..right_count {
                    let start = point(center_right, center_y);
                    let end = point(center_right + px(COLUMN_GAP), node_y(right_count, index));
                    curve(window, start, end, dashed, true);
                }
            },
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full();

        let mut left_column = div()
            .flex_1()
            .min_w(px(0.0))
            .h_full()
            .flex()
            .flex_col()
            .justify_center()
            .gap(px(NODE_GAP));
        for (index, node) in left.iter().enumerate() {
            let id = node.node.id.clone();
            let element = self
                .diagram_node(
                    theme,
                    format!("map-ego-left-{index}"),
                    &node.label,
                    &node.detail,
                    false,
                    cx,
                )
                .tooltip(tooltip("Abrir em Decisões", None))
                .on_click(cx.listener(move |_, _, _, cx| cx.emit(OpenDecision(id.clone()))));
            left_column = left_column.child(element);
        }
        if left.is_empty() {
            left_column = left_column.child(
                text_style(div(), TypeScale::META)
                    .text_color(colors.text_muted())
                    .child("Nenhuma decisão ligada."),
            );
        }
        let entity = &detail.entity;
        let center = div()
            .flex_1()
            .min_w(px(0.0))
            .h_full()
            .flex()
            .flex_col()
            .justify_center()
            .child(
                div()
                    .h(px(NODE_H))
                    .px(px(SpacingScale::S3))
                    .flex()
                    .flex_col()
                    .justify_center()
                    .rounded(RadiusScale.surface())
                    .border_1()
                    .border_color(colors.accent_default())
                    .bg(colors.selection())
                    .child(
                        text_style(div(), TypeScale::ROW_TITLE)
                            .truncate()
                            .child(entity.name.clone()),
                    )
                    .child(
                        text_style(div(), TypeScale::META)
                            .truncate()
                            .text_color(colors.text_muted())
                            .child(kind_label(entity.kind)),
                    ),
            );
        let mut right_column = div()
            .flex_1()
            .min_w(px(0.0))
            .h_full()
            .flex()
            .flex_col()
            .justify_center()
            .gap(px(NODE_GAP));
        for (index, node) in right.iter().enumerate() {
            right_column = right_column.child(self.diagram_node(
                theme,
                format!("map-ego-right-{index}"),
                &node.label,
                claim_label(&node.detail),
                true,
                cx,
            ));
        }
        if right.is_empty() {
            right_column = right_column.child(
                text_style(div(), TypeScale::META)
                    .text_color(colors.text_muted())
                    .child("Nenhuma regra ligada."),
            );
        }
        div()
            .flex()
            .flex_col()
            .gap(px(SpacingScale::S2))
            .child(
                div()
                    .id("map-ego")
                    .relative()
                    .w_full()
                    .h(px(height))
                    .flex()
                    .gap(px(COLUMN_GAP))
                    .role(Role::Group)
                    .aria_label(format!(
                        "{}: {} decisões à esquerda, {} regras à direita",
                        entity.name, left_count, right_count
                    ))
                    .child(edges)
                    .child(left_column)
                    .child(center)
                    .child(right_column),
            )
            .when(hidden > 0, |diagram| {
                diagram.child(
                    text_style(div(), TypeScale::META)
                        .text_color(colors.text_muted())
                        .child(format!("Mais {hidden} nas listas abaixo.")),
                )
            })
    }

    fn diagram_node(
        &mut self,
        theme: &Theme,
        id: String,
        label: &str,
        detail: &str,
        rule: bool,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let colors = theme.colors;
        let focus = self.focus_for(&id, cx);
        div()
            .id(SharedString::from(id))
            .h(px(NODE_H))
            .px(px(SpacingScale::S3))
            .flex()
            .flex_col()
            .justify_center()
            .rounded(RadiusScale.surface())
            .border_1()
            .border_color(colors.glass_border_card())
            .bg(colors.glass_fill_card())
            .when(!rule, |node| {
                node.cursor_pointer()
                    .hover(move |style| {
                        style
                            .border_color(colors.glass_border_card_hover())
                            .bg(colors.glass_fill_medium())
                    })
                    .role(Role::Button)
                    .track_focus(&focus)
                    .focus_visible(crate::ui::controls::focus_ring(theme))
            })
            .aria_label(label.to_owned())
            .child(
                text_style(div(), TypeScale::BODY_SMALL)
                    .truncate()
                    .child(label.to_owned()),
            )
            .child(
                text_style(div(), TypeScale::META)
                    .truncate()
                    .text_color(colors.text_muted())
                    .child(detail.to_owned()),
            )
    }
}

/// A stroked cubic curve from `start` to `end`, bending horizontally.
fn curve(
    window: &mut Window,
    start: gpui::Point<Pixels>,
    end: gpui::Point<Pixels>,
    color: Hsla,
    dashed: bool,
) {
    let bend = (end.x - start.x) / 2.0;
    let mut builder = PathBuilder::stroke(px(1.25));
    if dashed {
        builder = builder.dash_array(&[px(4.0), px(4.0)]);
    }
    builder.move_to(start);
    builder.cubic_bezier_to(
        end,
        point(start.x + bend, start.y),
        point(end.x - bend, end.y),
    );
    if let Ok(path) = builder.build() {
        window.paint_path(path, color);
    }
}

fn status_dot(color: gpui::Rgba) -> Div {
    div().size(px(7.0)).flex_none().rounded_full().bg(color)
}

fn legend_item(theme: &Theme, mark: Div, label: &str) -> Div {
    div()
        .flex()
        .items_center()
        .gap(px(SpacingScale::S2))
        .child(mark)
        .child(
            text_style(div(), TypeScale::META)
                .text_color(theme.colors.text_muted())
                .child(label.to_owned()),
        )
}

impl<S: MapStores> Render for MapScreen<S> {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let _probe = crate::ui::perf::Probe::start("map");
        let theme = Theme::current(cx);
        let index = (!self.embedded).then(|| self.render_index(&theme, cx));
        let body: AnyElement = if self.data.is_none() {
            if self.error.is_some() {
                empty_panel(
                    &theme,
                    IconName::Graph,
                    "Mapa",
                    "Não foi possível carregar o mapa",
                    "Tente de novo; seus dados não foram alterados.",
                )
                .into_any_element()
            } else {
                div()
                    .p(px(SpacingScale::S8))
                    .child(skeleton_list(&theme, "map-skeleton", 5))
                    .into_any_element()
            }
        } else {
            match self.view.clone() {
                View::Overview => self.render_overview(&theme, cx),
                View::Suggestions => self.render_suggestions(&theme, cx),
                View::File => self.render_file(&theme, cx),
                View::Timeline => self.render_timeline(&theme, cx),
                View::Entity(id) => self.render_entity(&id, &theme, cx),
                View::Form(editing) => self.render_form(editing, &theme, cx),
            }
        };
        let retry = self.error.is_some().then(|| {
            action_button(&theme, "map-retry", ButtonKind::Ghost, !self.busy)
                .aria_label("Tentar de novo")
                .on_click(cx.listener(|this, _, _, cx| this.refresh(cx)))
                .child("Tentar de novo")
        });
        div()
            .size_full()
            .relative()
            .flex()
            .children(index)
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .h_full()
                    .flex()
                    .flex_col()
                    .children(self.error.clone().map(|error| {
                        error_banner(&theme, &error)
                            .id("map-error")
                            .role(Role::Alert)
                            .children(retry)
                    }))
                    .child(div().flex_1().min_h(px(0.0)).flex().flex_col().child(body)),
            )
            .children(
                self.notice
                    .clone()
                    .map(|notice| toast(&theme, &notice, 24.0)),
            )
    }
}

// ---- helpers --------------------------------------------------------------

fn load<S: MapStores>(backend: &MapServices<S>, project: &str) -> Result<MapData, String> {
    // An empty map assembles itself from what the project declares.
    let assembled = backend.graph.assemble(project).map_err(|error| {
        failure(
            "assemble",
            error.code(),
            "Não foi possível montar o mapa a partir do projeto.",
        )
    })?;
    let proposals = backend
        .graph
        .refresh_suggestions(project)
        .map_err(|error| {
            failure(
                "refresh_suggestions",
                error.code(),
                "Não foi possível atualizar as sugestões.",
            )
        })?;
    let map = backend
        .graph
        .project_map(project, None)
        .map_err(|error| failure("project_map", error.code(), "Não foi possível ler o mapa."))?;
    let suggestions = backend.graph.suggestions(project).map_err(|error| {
        failure(
            "suggestions",
            error.code(),
            "Não foi possível ler as sugestões.",
        )
    })?;
    let graph = backend
        .graph
        .project_graph(project, None)
        .map_err(|error| {
            failure(
                "project_graph",
                error.code(),
                "Não foi possível ler o grafo.",
            )
        })?;
    let relations = backend.relations.pending(project).map_err(|error| {
        failure(
            "relation_suggestions",
            error.code(),
            "Não foi possível ler as relações sugeridas.",
        )
    })?;
    let derived = backend.context.pending(project).map_err(|error| {
        failure(
            "claim_suggestions",
            error.code(),
            "Não foi possível ler o contexto sugerido.",
        )
    })?;
    let recent_files = backend
        .graph
        .recent_files(project, RECENT_FILES)
        .map_err(|error| {
            failure(
                "recent_files",
                error.code(),
                "Não foi possível ler os arquivos das decisões.",
            )
        })?;
    Ok(MapData {
        map: Arc::new(map),
        graph,
        relations: Arc::new(relations),
        derived: Arc::new(derived),
        assembled,
        suggestions: Arc::new(suggestions),
        proposals: Arc::new(proposals),
        recent_files,
    })
}

/// Logs the technical code and returns product copy.
fn failure(operation: &'static str, code: &str, message: &str) -> String {
    tracing::error!(code, operation, "map operation failed");
    message.to_owned()
}

/// Product copy for a failed change.
fn change_failure(code: &str) -> String {
    match code {
        "duplicate_name" => "Já existe um item desse tipo com esse nome ou apelido.".into(),
        "duplicate_edge" => "Esse vínculo já existe.".into(),
        "empty_name" => "Dê um nome com ao menos uma letra ou dígito.".into(),
        "name_too_long" => "O nome passa de 80 caracteres.".into(),
        "description_too_long" => "A descrição passa de 500 caracteres.".into(),
        "invalid_pattern" => {
            "Use caminhos relativos ao projeto, como crates/app/**, sem .. nem letra de disco."
                .into()
        }
        "cycle" => "Esse vínculo criaria um ciclo de componentes.".into(),
        "self_reference" => "Um componente não pode fazer parte de si mesmo.".into(),
        "second_parent" => {
            "Esse componente já faz parte de outro. Desfaça o vínculo antigo primeiro.".into()
        }
        "conflict" => "O item mudou enquanto você editava. O mapa foi atualizado.".into(),
        "invalid_request" => "Use até 20 padrões e 20 apelidos por item.".into(),
        _ => {
            tracing::error!(code, operation = "map_change", "map change failed");
            "Não foi possível salvar a mudança no mapa.".into()
        }
    }
}

fn today() -> String {
    chrono::Utc::now().format("%Y-%m-%d").to_string()
}

/// A row of the index, by what it points at; rows become elements only
/// when they scroll into view.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum IndexRow {
    /// One of the three fixed views.
    View(usize),
    /// The heading of a kind.
    Heading(EntityKind),
    /// What a kind with no entities says.
    Nothing(EntityKind),
    /// The entity at this position of the map.
    Entity(usize),
}

/// The index's rows for these entities: the views, then each kind with its
/// entities in order.
fn index_rows(entities: &[MapEntity]) -> Vec<IndexRow> {
    let mut rows: Vec<IndexRow> = (0..3).map(IndexRow::View).collect();
    for kind in EntityKind::ALL {
        rows.push(IndexRow::Heading(kind));
        let before = rows.len();
        rows.extend(
            entities
                .iter()
                .enumerate()
                .filter(|(_, row)| row.entity.kind == kind)
                .map(|(position, _)| IndexRow::Entity(position)),
        );
        if rows.len() == before {
            rows.push(IndexRow::Nothing(kind));
        }
    }
    rows
}

/// Technology chips per row of the Blocos layout.
const TECH_ROW: usize = 4;

/// A row of the Blocos layout, by key; rows become elements only when they
/// scroll into view.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OverviewRow {
    /// Title, subtitle and the layout switch.
    Header,
    /// A section heading.
    Label(&'static str),
    /// Two component blocks, starting at this position in the roots.
    Blocks(usize),
    /// A row of technology chips, starting at this position.
    Technologies(usize),
    /// The legend for conflicts, when there are any.
    Conflict,
}

/// What the Blocos layout reads from a map, indexed once per map.
struct OverviewIndex {
    map: Arc<ProjectMap>,
    /// Positions (in the map's entities) of the top-level components.
    tops: Vec<usize>,
    /// Positions of the technologies.
    technologies: Vec<usize>,
    /// Positions of each component's parts, by the component's id.
    parts: BTreeMap<String, Vec<usize>>,
    /// Whether any top-level component has a conflict.
    conflicted: bool,
}

impl OverviewIndex {
    fn new(map: Arc<ProjectMap>) -> Self {
        let parent_of: BTreeMap<&str, &str> = map
            .part_of
            .iter()
            .map(|(part, parent)| (part.as_str(), parent.as_str()))
            .collect();
        let mut tops = Vec::new();
        let mut technologies = Vec::new();
        let mut parts: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        for (at, row) in map.entities.iter().enumerate() {
            let id = row.entity.entity_id.as_str();
            if let Some(parent) = parent_of.get(id) {
                parts.entry((*parent).to_owned()).or_default().push(at);
            }
            match row.entity.kind {
                EntityKind::Technology => technologies.push(at),
                EntityKind::Component if !parent_of.contains_key(id) => tops.push(at),
                EntityKind::Component => {}
            }
        }
        let conflicted = tops.iter().any(|&at| map.entities[at].conflicts > 0);
        Self {
            map,
            tops,
            technologies,
            parts,
            conflicted,
        }
    }
}

/// The rows of the Blocos layout for an index.
fn overview_rows(index: &OverviewIndex) -> Vec<OverviewRow> {
    let mut rows = vec![OverviewRow::Header, OverviewRow::Label("Componentes")];
    rows.extend((0..index.tops.len()).step_by(2).map(OverviewRow::Blocks));
    if !index.technologies.is_empty() {
        rows.push(OverviewRow::Label("Tecnologias"));
        rows.extend(
            (0..index.technologies.len())
                .step_by(TECH_ROW)
                .map(OverviewRow::Technologies),
        );
    }
    if index.conflicted {
        rows.push(OverviewRow::Conflict);
    }
    rows
}

fn index_frame(theme: &Theme) -> Div {
    crate::ui::patterns::index_rail(theme)
}

fn page_header(theme: &Theme, title: &str, subtitle: &str) -> Div {
    div()
        .flex()
        .flex_col()
        .gap(px(SpacingScale::S1))
        .child(text_style(div(), TypeScale::HEADING_1).child(title.to_owned()))
        .child(
            text_style(div(), TypeScale::BODY_SMALL)
                .text_color(theme.colors.text_muted())
                .child(subtitle.to_owned()),
        )
}

fn section(theme: &Theme, label: &str, content: Div) -> Div {
    div()
        .flex()
        .flex_col()
        .gap(px(SpacingScale::S3))
        .child(section_label(theme, label))
        .child(content)
}

/// Upkeep names listed on a condensed line before "e mais N".
const UPKEEP_SHOWN: usize = 4;
/// Longest node name on a condensed line.
const UPKEEP_CHARS: usize = 28;

/// Diameter of a timeline marker; the rail runs through its center.
const MARKER: f32 = 22.0;

/// The project history as a feed: newest first, grouped by day, on a rail
/// with one marker per kind (Primer Timeline anatomy). Decisions and rules
/// are full items; map upkeep (items created, links confirmed) collapses
/// into one condensed line per run so it never drowns what changed.
fn timeline_list(theme: &Theme, events: &[TimelineEvent], limit: usize) -> (Div, usize) {
    let colors = theme.colors;
    let mut list = div().flex().flex_col();
    if events.is_empty() {
        return (
            list.child(
                text_style(div(), TypeScale::BODY_SMALL)
                    .text_color(colors.text_muted())
                    .child("Nada aconteceu aqui ainda."),
            ),
            0,
        );
    }
    let mut ordered: Vec<&TimelineEvent> = events.iter().collect();
    ordered.sort_by(|left, right| right.at.cmp(&left.at));
    let hidden = ordered.len().saturating_sub(limit);
    ordered.truncate(limit);

    // Day → runs of events (a run is one full event or consecutive upkeep
    // of the same kind).
    let mut days: Vec<(String, Vec<Vec<&TimelineEvent>>)> = Vec::new();
    for event in ordered {
        let heading = event_day(event);
        if days.last().is_none_or(|(day, _)| *day != heading) {
            days.push((heading, Vec::new()));
        }
        let runs = &mut days.last_mut().expect("day pushed").1;
        let joins = runs.last().is_some_and(|run| {
            upkeep(event.kind) && run.first().is_some_and(|first| first.kind == event.kind)
        });
        if joins {
            runs.last_mut().expect("run").push(event);
        } else {
            runs.push(vec![event]);
        }
    }

    for (day_index, (heading, runs)) in days.into_iter().enumerate() {
        // Upkeep is condensed, so the heading counts what changed.
        let changes = runs
            .iter()
            .flatten()
            .filter(|event| !upkeep(event.kind))
            .count();
        list = list.child(
            div()
                .flex()
                .items_baseline()
                .gap(px(SpacingScale::S2))
                .when(day_index > 0, |row| row.mt(px(SpacingScale::S6)))
                .mb(px(SpacingScale::S3))
                .child(
                    text_style(div(), TypeScale::ROW_TITLE)
                        .text_color(colors.text_secondary())
                        .child(heading),
                )
                .child(
                    text_style(div(), TypeScale::META)
                        .text_color(colors.text_muted())
                        .child(if changes == 0 {
                            "manutenção do mapa".to_owned()
                        } else {
                            plural(changes, "mudança", "mudanças")
                        }),
                ),
        );
        let last = runs.len().saturating_sub(1);
        for (index, run) in runs.into_iter().enumerate() {
            let key = format!("timeline-in-{day_index}-{index}");
            list = list.child(crate::ui::motion::cascade(
                gpui::ElementId::Name(key.into()),
                index,
                timeline_item(theme, &run, index == last),
            ));
        }
    }
    (list, hidden)
}

/// The day an event belongs to; rule validity is a calendar date.
fn event_day(event: &TimelineEvent) -> String {
    match event.kind {
        TimelineKind::ClaimStarted | TimelineKind::ClaimEnded => calendar_date(&event.at),
        _ => day_heading(&event.at),
    }
}

/// Map upkeep, condensed in the feed.
fn upkeep(kind: TimelineKind) -> bool {
    matches!(
        kind,
        TimelineKind::EntityCreated
            | TimelineKind::EntityRetired
            | TimelineKind::EdgeConfirmed
            | TimelineKind::EdgeInvalidated
    )
}

fn timeline_item(theme: &Theme, run: &[&TimelineEvent], last: bool) -> Div {
    let colors = theme.colors;
    let first = run[0];
    let condensed = upkeep(first.kind);
    let (label, glyph, color) = timeline_copy(first.kind, theme);
    let marker = if condensed {
        div()
            .size(px(MARKER))
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .size(px(7.0))
                    .rounded_full()
                    .border_1()
                    .border_color(colors.text_muted())
                    .bg(colors.content()),
            )
    } else {
        div()
            .size(px(MARKER))
            .flex()
            .items_center()
            .justify_center()
            .rounded_full()
            .border_1()
            .border_color(color.alpha(0.35))
            .bg(colors.content())
            .child(icon(glyph, 12.0, color))
    };
    let rail = div()
        .w(px(MARKER))
        .flex_none()
        .flex()
        .flex_col()
        .items_center()
        .child(marker)
        .when(!last, |rail| {
            rail.child(div().w(px(1.0)).flex_1().bg(colors.hairline_divider()))
        });
    let time = text_style(div(), TypeScale::META)
        .flex_none()
        .text_color(colors.text_muted())
        .children(match first.kind {
            TimelineKind::ClaimStarted | TimelineKind::ClaimEnded => None,
            _ => clock(&first.at),
        });
    let body = if condensed {
        let names: Vec<String> = run
            .iter()
            .map(|event| match &event.other {
                Some(other) => format!(
                    "{} → {}",
                    clipped(&event.node.label, UPKEEP_CHARS),
                    clipped(&other.label, UPKEEP_CHARS)
                ),
                None => clipped(&event.node.label, UPKEEP_CHARS),
            })
            .collect();
        let shown = names
            .iter()
            .take(UPKEEP_SHOWN)
            .cloned()
            .collect::<Vec<_>>()
            .join(" · ");
        let rest = names.len().saturating_sub(UPKEEP_SHOWN);
        div()
            .flex_1()
            .min_w(px(0.0))
            .flex()
            .flex_col()
            .gap(px(2.0))
            .pt(px(3.0))
            .child(
                div()
                    .flex()
                    .items_baseline()
                    .gap(px(SpacingScale::S3))
                    .child(
                        text_style(div(), TypeScale::BODY_SMALL)
                            .flex_1()
                            .text_color(colors.text_secondary())
                            .child(if run.len() == 1 {
                                label.to_owned()
                            } else {
                                upkeep_headline(first.kind, run.len())
                            }),
                    )
                    .child(time),
            )
            .child(
                text_style(div(), TypeScale::META)
                    .text_color(colors.text_muted())
                    .child(if rest > 0 {
                        format!("{shown} e mais {rest}")
                    } else {
                        shown
                    }),
            )
    } else {
        let detail = match (&first.other, first.kind) {
            (Some(other), TimelineKind::DecisionSuperseded) => {
                format!("Substituída por: {}", other.label)
            }
            _ => first.node.detail.clone(),
        };
        div()
            .flex_1()
            .min_w(px(0.0))
            .flex()
            .flex_col()
            .gap(px(2.0))
            .child(
                div()
                    .flex()
                    .items_baseline()
                    .gap(px(SpacingScale::S3))
                    .pt(px(3.0))
                    .child(
                        text_style(div(), TypeScale::META)
                            .flex_1()
                            .text_color(color)
                            .child(label),
                    )
                    .child(time),
            )
            .child(text_style(div(), TypeScale::BODY).child(first.node.label.clone()))
            .when(!detail.is_empty(), |body| {
                body.child(
                    text_style(div(), TypeScale::BODY_SMALL)
                        .text_color(colors.text_muted())
                        .child(claim_or_text(first.kind, &detail)),
                )
            })
    };
    div()
        .flex()
        .gap(px(SpacingScale::S3))
        .child(rail)
        .child(body.pb(px(if last { 0.0 } else { SpacingScale::S4 })))
}

/// A rule's detail is its kind literal; a decision's is its choice.
fn claim_or_text(kind: TimelineKind, detail: &str) -> String {
    match kind {
        TimelineKind::ClaimStarted | TimelineKind::ClaimEnded => claim_label(detail).to_owned(),
        _ => detail.to_owned(),
    }
}

fn timeline_copy(kind: TimelineKind, theme: &Theme) -> (&'static str, IconName, gpui::Rgba) {
    let colors = theme.colors;
    match kind {
        TimelineKind::DecisionConfirmed => (
            "Decisão confirmada",
            IconName::Check,
            colors.status_success(),
        ),
        TimelineKind::DecisionSuperseded => (
            "Decisão substituída",
            IconName::Rotate,
            colors.status_warning(),
        ),
        TimelineKind::ClaimStarted => (
            "Regra passou a valer",
            IconName::Shield,
            colors.status_info(),
        ),
        TimelineKind::ClaimEnded => (
            "Regra deixou de valer",
            IconName::Shield,
            colors.text_muted(),
        ),
        TimelineKind::EntityCreated => ("Entrou no mapa", IconName::Plus, colors.text_muted()),
        TimelineKind::EntityRetired => ("Aposentado", IconName::Circle, colors.text_muted()),
        TimelineKind::EdgeConfirmed => {
            ("Ligações confirmadas", IconName::Link, colors.text_muted())
        }
        TimelineKind::EdgeInvalidated => {
            ("Ligações desfeitas", IconName::Link, colors.text_muted())
        }
    }
}

/// "8 itens entraram no mapa": a condensed run says how many, then what.
fn upkeep_headline(kind: TimelineKind, count: usize) -> String {
    match kind {
        TimelineKind::EntityCreated => format!("{count} itens entraram no mapa"),
        TimelineKind::EntityRetired => format!("{count} itens aposentados"),
        TimelineKind::EdgeConfirmed => format!("{count} ligações confirmadas"),
        TimelineKind::EdgeInvalidated => format!("{count} ligações desfeitas"),
        _ => plural(count, "evento", "eventos"),
    }
}

fn weight(row: &MapEntity) -> String {
    let mut parts = vec![plural(row.decisions, "decisão", "decisões")];
    if row.claims > 0 {
        parts.push(plural(row.claims, "regra", "regras"));
    }
    if row.conflicts > 0 {
        parts.push(plural(row.conflicts, "conflito", "conflitos"));
    }
    parts.join(" · ")
}

/// "a" or "o" for a kind of rule, as the Portuguese sentence needs it.
fn claim_article(kind: &str) -> &'static str {
    match kind {
        "assumption" => "uma",
        "constraint" => "uma",
        "goal" => "um",
        "convention" => "uma",
        _ => "uma",
    }
}

/// The pronoun that goes with [`claim_article`] ("recebê-la", "recebê-lo").
fn claim_pronoun(kind: &str) -> &'static str {
    match claim_article(kind) {
        "um" => "lo",
        _ => "la",
    }
}

/// A quoted name in the strong face.
fn named(text: &str) -> (String, bool) {
    (format!("“{}”", clipped(text, 90)), true)
}

fn plain(text: &str) -> (String, bool) {
    (text.to_owned(), false)
}

/// What a relation between two decisions says in a sentence, and what
/// confirming it changes.
fn relation_wording(kind: RelationKind, from: &str, to: &str) -> (Vec<(String, bool)>, String) {
    match kind {
        RelationKind::DependsOn => (
            vec![
                plain("A decisão"),
                named(from),
                plain("depende da decisão"),
                named(to),
                plain("só faz sentido porque a segunda foi tomada."),
            ],
            "a relação fica registrada na seção Relações das duas decisões, e quem ler a \
             primeira vê de qual outra ela depende."
                .to_owned(),
        ),
        RelationKind::ConflictsWith => (
            vec![
                plain("A decisão"),
                named(from),
                plain("parece contradizer a decisão"),
                named(to),
                plain("."),
            ],
            "a contradição fica registrada nas duas decisões; resolva substituindo uma delas."
                .to_owned(),
        ),
        RelationKind::Supersedes => (
            vec![
                plain("A decisão"),
                named(from),
                plain("parece ter substituído a decisão"),
                named(to),
                plain("."),
            ],
            "a substituição fica registrada na seção Relações das duas decisões.".to_owned(),
        ),
    }
}

/// What a suggested tie says in a sentence (by what it ties and why), and
/// what confirming it changes.
fn link_wording(suggestion: &Suggestion) -> (Vec<(String, bool)>, String) {
    let source = &suggestion.source.label;
    let target = &suggestion.entity.label;
    let rule = suggestion.source.node.kind == NodeKind::Claim;
    let what = if rule { "A regra" } else { "A decisão" };
    if let Some(quote) = application::graph::mention_quote(&suggestion.reason) {
        let (kind, owner) = (
            if suggestion.kind == EdgeKind::Uses {
                "tecnologia usada"
            } else {
                "parte afetada"
            },
            if rule { "regra" } else { "decisão" },
        );
        return (
            vec![
                plain(what),
                named(source),
                plain("cita"),
                named(target),
                plain("no próprio texto:"),
                (format!("“{quote}”"), true),
                plain("."),
            ],
            format!(
                "{target} passa a constar como {kind} por essa {owner} no Mapa. A ligação veio \
                 só do texto; confira o trecho antes de confirmar."
            ),
        );
    }
    match suggestion.kind {
        EdgeKind::Uses => (
            vec![
                plain(what),
                named(source),
                plain("adicionou a dependência"),
                (suggestion.reason.clone(), true),
                plain("que é a tecnologia"),
                named(target),
                plain("do Mapa."),
            ],
            format!(
                "{target} passa a constar como tecnologia usada por essa decisão, na página dela e \
                 na do Mapa."
            ),
        ),
        _ if rule => (
            vec![
                plain(what),
                named(source),
                plain("se aplica a mudanças em"),
                (suggestion.reason.clone(), true),
                plain(", arquivo que pertence ao componente"),
                named(target),
                plain("."),
            ],
            format!(
                "a regra passa a valer em {target} e é entregue ao agente quando ele edita \
                 arquivos desse componente."
            ),
        ),
        _ => (
            vec![
                plain(what),
                named(source),
                plain("mexeu em"),
                (suggestion.reason.clone(), true),
                plain(", arquivo que pertence ao componente"),
                named(target),
                plain("."),
            ],
            format!(
                "a decisão passa a valer para {target}: aparece na página do componente e é \
                 entregue ao agente quando ele edita arquivos dele."
            ),
        ),
    }
}

/// What a proposed component or technology says in a sentence, and what
/// creating it changes.
fn item_wording(
    kind: EntityKind,
    name: &str,
    pattern: Option<&str>,
    declared: Option<application::graph::WorkspaceKind>,
    decisions: usize,
) -> (Vec<(String, bool)>, String) {
    match (kind, pattern, declared) {
        (EntityKind::Component, Some(pattern), Some(source)) => (
            vec![
                plain("O"),
                plain(source.label()),
                plain("declara a parte"),
                named(name),
                plain("("),
                (pattern.to_owned(), true),
                plain("), mas ela ainda não está no Mapa."),
            ],
            "o componente entra no Mapa e as decisões que mexeram nesses arquivos passam a \
             poder ser ligadas a ele."
                .to_owned(),
        ),
        (EntityKind::Component, pattern, _) => (
            vec![
                plain(&format!(
                    "{} {} mexeu{} em arquivos de",
                    if decisions == 1 { "Uma" } else { "Várias" },
                    if decisions == 1 { "decisão" } else { "decisões" },
                    if decisions == 1 { "" } else { "ram" },
                )),
                (pattern.unwrap_or(name).to_owned(), true),
                plain("que nenhum componente do Mapa cobre."),
            ],
            format!(
                "o componente {name} entra no Mapa e {} ligada{} a ele.",
                plural(decisions, "decisão fica", "decisões ficam"),
                if decisions == 1 { "" } else { "s" },
            ),
        ),
        (EntityKind::Technology, ..) => (
            vec![
                plain(&format!(
                    "A dependência {name} foi adicionada em {}, mas ainda não é uma tecnologia do Mapa.",
                    plural(decisions, "decisão", "decisões")
                )),
            ],
            format!(
                "{name} entra no Mapa como tecnologia e as decisões que a usam passam a estar \
                 ligadas a ela."
            ),
        ),
    }
}

fn kind_label(kind: EntityKind) -> &'static str {
    match kind {
        EntityKind::Component => "Componente",
        EntityKind::Technology => "Tecnologia",
    }
}

fn kind_plural(kind: EntityKind) -> &'static str {
    match kind {
        EntityKind::Component => "Componentes",
        EntityKind::Technology => "Tecnologias",
    }
}

fn kind_icon(kind: EntityKind) -> IconName {
    match kind {
        EntityKind::Component => IconName::Component,
        EntityKind::Technology => IconName::Cpu,
    }
}

fn claim_label(kind: &str) -> &'static str {
    match kind {
        "assumption" => "Premissa",
        "constraint" => "Restrição",
        "goal" => "Objetivo",
        "convention" => "Convenção",
        _ => "Regra",
    }
}

#[cfg(test)]
mod tests {
    use super::{change_failure, weight};
    use application::graph::{EntityRecord, MapEntity};
    use domain::entities::EntityKind;

    #[test]
    fn change_failures_never_echo_the_code() {
        for code in [
            "duplicate_name",
            "invalid_pattern",
            "cycle",
            "conflict",
            "storage",
        ] {
            assert!(!change_failure(code).contains(code));
        }
    }

    #[test]
    fn weights_read_naturally() {
        let row = MapEntity {
            entity: EntityRecord {
                entity_id: "e".into(),
                project_id: "p".into(),
                kind: EntityKind::Component,
                name: "x".into(),
                key: "x".into(),
                description: String::new(),
                patterns: Vec::new(),
                aliases: Vec::new(),
                created_at: "2026-01-01T00:00:00Z".into(),
                retired_at: None,
            },
            decisions: 1,
            claims: 2,
            last_activity: None,
            conflicts: 0,
        };
        assert_eq!(weight(&row), "1 decisão · 2 regras");
    }
}
