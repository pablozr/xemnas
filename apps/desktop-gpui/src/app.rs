//! Desktop navigation between tracked projects and the candidate reading Inbox.

use application::decisions::{DecisionStore, Decisions};
use application::export::Export;
use application::inbox::{Inbox, InboxStore};
use application::jobs::JobSummary;
use application::overview::OverviewApi;
use application::projects::{ProjectRepository, Projects};
use application::relations::DecisionRelations;
use gpui::prelude::*;
use gpui::{
    actions, deferred, div, px, AnimationExt, App, BoxShadow, Context, ElementId, Entity,
    FocusHandle, Focusable, Render, Role, SpringAnimation, Subscription, Window, WindowControlArea,
};
use std::sync::Arc;

use crate::fonts::{app_icon, wordmark};
use crate::palette::{self, PaletteItem};
use crate::screens::context::{ContextScreen, ContextServices, ContextStores, OpenDecision};
use crate::screens::decisions::DecisionsScreen;
use crate::screens::inbox::InboxScreen;
use crate::screens::map::{MapScreen, MapServices};
use crate::screens::overview::{OpenEntity, OverviewScreen};
use crate::screens::projects::{ProjectChanged, ProjectsScreen};
use crate::screens::settings::{CloseSettings, SettingsScreen, SettingsSection, SettingsServices};
use crate::ui::controls::icon_action;
use crate::ui::feedback::error_state;
use crate::ui::glass::focus_ring;
use crate::ui::icons::{icon, IconName};
use crate::ui::patterns::{count_chip, fade_in, kbd, section_label, track_hover};
use crate::ui::search_field::{SearchChanged, SearchField};
use crate::ui::theme::{text_style, Backdrop, Theme, ThemeMode};
use crate::ui::tokens::MotionTokens;
use crate::ui::tokens::{ControlSize, SpacingScale, TypeScale};
use crate::ui::tooltip::tooltip;

actions!(
    xemnas,
    [
        /// Focuses the next control.
        TabNext,
        /// Focuses the previous control.
        TabPrev,
        /// Focuses the current destination's search field.
        FocusSearch,
        /// Selects the next item of the current list.
        NextItem,
        /// Selects the previous item of the current list.
        PrevItem,
        /// Confirms the candidate being read.
        ConfirmItem,
        /// Rejects the candidate being read.
        RejectItem,
        /// Snoozes or resumes the candidate being read.
        SnoozeItem,
        /// Opens the adjust form for the candidate being read.
        AdjustItem,
        /// Submits the open editor with its primary action.
        SaveEditor,
        /// Opens Revisão.
        GoReview,
        /// Opens Decisões.
        GoDecisions,
        /// Opens Contexto.
        GoContext,
        /// Opens Mapa.
        GoMap,
        /// Opens Visão.
        GoOverview,
        /// Opens or closes the command palette.
        TogglePalette,
        /// Moves the palette highlight down.
        PaletteDown,
        /// Moves the palette highlight up.
        PaletteUp,
        /// Runs the highlighted palette item.
        PaletteRun,
        /// Closes the palette.
        PaletteClose
    ]
);

/// Height of the custom title bar (brand, theme and caption controls).
const TITLE_BAR_HEIGHT: f32 = 40.0;
/// Height of the project bar that carries the breadcrumb and destinations.
const PROJECT_BAR_HEIGHT: f32 = 44.0;

/// Whether captures from the OpenCode adapter can reach this app right now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaptureStatus {
    /// The loopback API is listening.
    Listening,
    /// The API could not start; captures wait in the adapter's outbox.
    Unavailable,
    /// Sample data: no integrations run.
    Demo,
}

/// Reads the current background work. Called off the UI thread.
pub type ActivitySource = Arc<dyn Fn() -> Option<JobSummary> + Send + Sync>;

/// How often the status line re-reads the jobs table.
const ACTIVITY_POLL: std::time::Duration = std::time::Duration::from_secs(5);

/// What a palette item does.
#[derive(Clone, Debug)]
enum Command {
    Go(Destination),
    Project(String),
    Candidate(String),
    Decision(String),
    ProjectPanel,
    OpenFolder,
    ToggleTheme,
    OpenSettings,
    SettingsAt(SettingsSection),
}

/// The open palette: its query field and highlighted row.
struct Palette {
    field: Entity<SearchField>,
    highlighted: usize,
    _subscription: Subscription,
}

/// Rows shown at once before the palette scrolls.
const PALETTE_MAX_HEIGHT: f32 = 380.0;

/// Clicks that land on the breadcrumb within this window of a dismissal are
/// the same gesture that dismissed the panel, not a request to reopen it.
const PANEL_REOPEN_GUARD: std::time::Duration = std::time::Duration::from_millis(250);

/// The places a selected project can be read from. Its properties are not a
/// destination: they open in the panel under the project name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Destination {
    Review,
    Decisions,
    Context,
    Map,
    Overview,
}

impl Destination {
    fn index(self) -> usize {
        match self {
            Self::Review => 0,
            Self::Decisions => 1,
            Self::Context => 2,
            Self::Map => 3,
            Self::Overview => 4,
        }
    }

    fn id(self) -> &'static str {
        match self {
            Self::Review => "nav-inbox",
            Self::Decisions => "nav-decisions",
            Self::Context => "nav-context",
            Self::Map => "nav-map",
            Self::Overview => "nav-overview",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Review => "Revisão",
            Self::Decisions => "Decisões",
            Self::Context => "Contexto",
            Self::Map => "Mapa",
            Self::Overview => "Visão",
        }
    }

    fn glyph(self) -> IconName {
        match self {
            Self::Review => IconName::List,
            Self::Decisions => IconName::File,
            Self::Context => IconName::Layers,
            Self::Map => IconName::Graph,
            Self::Overview => IconName::Compass,
        }
    }

    fn shortcut(self) -> &'static str {
        match self {
            Self::Review => "Ctrl 1",
            Self::Decisions => "Ctrl 2",
            Self::Context => "Ctrl 3",
            Self::Map => "Ctrl 4",
            Self::Overview => "Ctrl 0",
        }
    }
}

/// Persistent product screens with contextual search and native window controls.
pub struct Shell<R: ProjectRepository + InboxStore + DecisionStore + ContextStores> {
    theme: Theme,
    focus: FocusHandle,
    search: Entity<SearchField>,
    _search_subscription: Subscription,
    _project_subscription: Option<Subscription>,
    _inbox_subscription: Option<Subscription>,
    projects: Option<Entity<ProjectsScreen<R>>>,
    inbox: Option<Entity<InboxScreen<R>>>,
    decisions: Option<Entity<DecisionsScreen<R>>>,
    settings: Option<Entity<SettingsScreen>>,
    _settings_subscription: Option<Subscription>,
    settings_open: bool,
    settings_focus: FocusHandle,
    destination: Destination,
    destination_focus: [FocusHandle; 5],
    context: Option<Entity<ContextScreen<R>>>,
    _context_subscription: Option<Subscription>,
    map: Option<Entity<MapScreen<R>>>,
    _map_subscription: Option<Subscription>,
    overview: Option<Entity<OverviewScreen>>,
    _overview_subscriptions: Vec<Subscription>,
    /// Destination tab under the pointer, driving the hover spring.
    hovered_tab: Option<Destination>,
    theme_focus: FocusHandle,
    project_focus: FocusHandle,
    /// Whether the project panel under the breadcrumb is open.
    project_panel: bool,
    panel_dismissed_at: Option<std::time::Instant>,
    demo: bool,
    palette: Option<Palette>,
    capture: Option<CaptureStatus>,
    activity: Option<JobSummary>,
    /// Whether the window was opened over a system material (Mica/Acrylic).
    backdrop: bool,
    /// Demo-only destination to open once projects load (`--open`).
    route: Option<String>,
}

impl<R: ProjectRepository + InboxStore + DecisionStore + ContextStores> Shell<R> {
    /// Mounts both use cases once, retaining their state across navigation.
    pub fn new(
        cx: &mut Context<Self>,
        projects: Result<Projects<R>, String>,
        inbox: Option<Inbox<R>>,
        decisions: Option<(Decisions<R>, Export<R>, DecisionRelations<R>)>,
        context: Option<ContextServices<R>>,
        map: Option<MapServices<R>>,
        settings: Option<SettingsServices>,
    ) -> Self {
        let screen = match projects {
            Ok(projects) => {
                let screen = cx.new(|cx| ProjectsScreen::new(cx, projects));
                screen.update(cx, |screen, cx| screen.start(cx, ""));
                Some(screen)
            }
            Err(detail) => {
                tracing::error!(error = %detail, operation = "open_database", "could not open projects database");
                None
            }
        };
        let search = cx.new(SearchField::new);
        search.update(cx, |search, cx| {
            search.set_context("Filtrar candidatos carregados", cx)
        });
        let search_subscription = cx.subscribe(&search, |shell, _, event: &SearchChanged, cx| {
            if shell.destination == Destination::Review {
                if let Some(screen) = &shell.inbox {
                    screen.update(cx, |screen, cx| screen.set_query(&event.0, cx));
                }
            }
        });
        let project_subscription = screen.as_ref().map(|screen| {
            cx.subscribe(screen, |shell, _, event: &ProjectChanged, cx| {
                if let Some(inbox) = &shell.inbox {
                    inbox.update(cx, |inbox, cx| {
                        inbox.set_project(
                            event
                                .0
                                .as_ref()
                                .map(|project| project.id().as_str().to_owned()),
                            cx,
                        );
                    });
                }
                if let Some(context) = &shell.context {
                    let project = event
                        .0
                        .as_ref()
                        .map(|project| project.id().as_str().to_owned());
                    context.update(cx, |screen, cx| screen.set_project(project, cx));
                }
                if let Some(map) = &shell.map {
                    let project = event
                        .0
                        .as_ref()
                        .map(|project| project.id().as_str().to_owned());
                    map.update(cx, |screen, cx| screen.set_project(project, cx));
                }
                if let Some(overview) = &shell.overview {
                    let project = event
                        .0
                        .as_ref()
                        .map(|project| project.id().as_str().to_owned());
                    overview.update(cx, |screen, cx| screen.set_project(project, cx));
                }
                if let Some(decisions) = &shell.decisions {
                    decisions.update(cx, |screen, cx| {
                        screen.set_project(
                            event
                                .0
                                .as_ref()
                                .map(|project| project.id().as_str().to_owned()),
                            cx,
                        )
                    });
                }
                if shell.destination == Destination::Review {
                    shell.search.update(cx, |search, cx| {
                        search.set_context("Filtrar candidatos carregados", cx)
                    });
                }
                shell.project_panel = false;
                cx.notify();
            })
        });
        let inbox = inbox.map(|inbox| {
            cx.new(|cx| {
                let mut screen = InboxScreen::new(cx, inbox);
                screen.attach_search(search.clone());
                screen
            })
        });
        let inbox_subscription = inbox
            .as_ref()
            .map(|screen| cx.observe(screen, |_, _, cx| cx.notify()));
        let decisions = decisions.map(|(decisions, export, relations)| {
            cx.new(|cx| DecisionsScreen::new(cx, decisions, export, relations))
        });
        let context = context.map(|services| cx.new(|cx| ContextScreen::new(cx, services)));
        let context_subscription = context.as_ref().map(|screen| {
            cx.subscribe(screen, |shell, _, event: &OpenDecision, cx| {
                shell.show_decision(event.0.clone(), cx)
            })
        });
        let map = map.map(|services| cx.new(|cx| MapScreen::new(cx, services)));
        let map_subscription = map.as_ref().map(|screen| {
            cx.subscribe(screen, |shell, _, event: &OpenDecision, cx| {
                shell.show_decision(event.0.clone(), cx)
            })
        });
        let settings = settings.map(|services| cx.new(|cx| SettingsScreen::new(cx, services)));
        let settings_subscription = settings.as_ref().map(|screen| {
            cx.subscribe(screen, |shell, _, _: &CloseSettings, cx| {
                shell.settings_open = false;
                cx.notify();
            })
        });
        Self {
            theme: Theme::quiet_glass(),
            focus: cx.focus_handle(),
            search: search.clone(),
            _search_subscription: search_subscription,
            _project_subscription: project_subscription,
            _inbox_subscription: inbox_subscription,
            projects: screen,
            inbox,
            decisions,
            settings,
            _settings_subscription: settings_subscription,
            settings_open: false,
            settings_focus: cx.focus_handle().tab_stop(true),
            destination: Destination::Review,
            destination_focus: [
                cx.focus_handle().tab_stop(true),
                cx.focus_handle().tab_stop(true),
                cx.focus_handle().tab_stop(true),
                cx.focus_handle().tab_stop(true),
                cx.focus_handle().tab_stop(true),
            ],
            context,
            _context_subscription: context_subscription,
            map,
            _map_subscription: map_subscription,
            overview: None,
            _overview_subscriptions: Vec::new(),
            hovered_tab: None,
            theme_focus: cx.focus_handle().tab_stop(true),
            project_focus: cx.focus_handle().tab_stop(true),
            project_panel: false,
            panel_dismissed_at: None,
            demo: false,
            palette: None,
            capture: None,
            activity: None,
            backdrop: false,
            route: None,
        }
    }

    /// Shows capture and background-job status under the project list and
    /// keeps it current by re-reading `source` every few seconds.
    pub fn set_activity(
        &mut self,
        capture: CaptureStatus,
        source: Option<ActivitySource>,
        cx: &mut Context<Self>,
    ) {
        self.capture = Some(capture);
        let Some(source) = source else {
            return;
        };
        cx.spawn(async move |this, cx| loop {
            let read = source.clone();
            let summary = cx.background_executor().spawn(async move { read() }).await;
            if this
                .update(cx, |shell, cx| {
                    if shell.activity != summary {
                        shell.activity = summary;
                        cx.notify();
                    }
                })
                .is_err()
            {
                break;
            }
            cx.background_executor().timer(ACTIVITY_POLL).await;
        })
        .detach();
    }

    /// The status line at the foot of the sidebar: whether captures arrive
    /// and what the extractor is doing. Only real states, never a guess.
    fn status_line(&self, cx: &mut Context<Self>) -> Option<gpui::AnyElement> {
        let theme = self.theme;
        let capture = self.capture?;
        let (dot, label, hint) = match capture {
            CaptureStatus::Listening => (
                theme.colors.status_success(),
                "Captura ativa",
                "O app recebe capturas do OpenCode pela API local.",
            ),
            CaptureStatus::Unavailable => (
                theme.colors.status_warning(),
                "Captura indisponível",
                "A API local não iniciou; as capturas aguardam na outbox do adapter.",
            ),
            CaptureStatus::Demo => (
                theme.colors.status_info(),
                "Demonstração",
                "Dados fictícios em memória; nenhuma integração roda.",
            ),
        };
        let summary = self.activity.unwrap_or_default();
        let working = summary.queued + summary.running;
        Some(
            text_style(div(), TypeScale::META)
                .id("activity-status")
                .flex_none()
                .px(px(SpacingScale::S4))
                .py(px(SpacingScale::S3))
                .flex()
                .items_center()
                .gap(px(SpacingScale::S2))
                .border_t_1()
                .border_color(theme.colors.hairline_divider())
                .text_color(theme.colors.text_muted())
                .role(Role::Button)
                .aria_label(format!("{label}. Abrir detalhes"))
                .tooltip(tooltip(hint, None))
                .when(self.settings.is_some(), |line| {
                    let hover = theme.colors.glass_fill_medium();
                    let section = if summary.failed > 0 {
                        SettingsSection::Diagnostics
                    } else {
                        SettingsSection::OpenCode
                    };
                    line.cursor_pointer()
                        .hover(move |style| style.bg(hover))
                        .on_click(
                            cx.listener(move |this, _, _, cx| this.open_settings_at(section, cx)),
                        )
                })
                .child(div().size(px(6.0)).flex_none().rounded_full().bg(dot))
                .child(div().flex_1().truncate().child(label))
                .when(working > 0, |line| {
                    line.child(icon(
                        IconName::Activity,
                        12.0,
                        theme.colors.accent_default(),
                    ))
                    .child(format!("Extraindo {working}"))
                })
                .when(summary.failed > 0, |line| {
                    line.child(div().text_color(theme.colors.status_danger()).child(
                        match summary.failed {
                            1 => "1 falha".to_string(),
                            n => format!("{n} falhas"),
                        },
                    ))
                })
                .into_any_element(),
        )
    }

    /// Records that the window has a system material behind it.
    pub fn set_backdrop(&mut self, backdrop: bool) {
        self.backdrop = backdrop;
    }

    /// Mounts Visão over its use case; without one the destination stays
    /// empty, like the others when the database fails.
    pub fn set_overview(&mut self, api: Arc<dyn OverviewApi>, cx: &mut Context<Self>) {
        let screen = cx.new(|_| OverviewScreen::new(api));
        self._overview_subscriptions = vec![
            cx.subscribe(&screen, |shell, _, event: &OpenDecision, cx| {
                shell.show_decision(event.0.clone(), cx)
            }),
            cx.subscribe(&screen, |shell, _, event: &OpenEntity, cx| {
                shell.show_entity(event.0.clone(), cx)
            }),
        ];
        let project = self
            .projects
            .as_ref()
            .and_then(|projects| projects.read(cx).selected_project())
            .map(|project| project.id().as_str().to_owned());
        screen.update(cx, |screen, cx| screen.set_project(project, cx));
        self.overview = Some(screen);
    }

    /// Opens `<destination>[:<view>]` on the first demo project once the
    /// list loads, so captures reach a screen without clicks or focus.
    pub fn open_route(&mut self, route: String) {
        self.route = Some(route);
    }

    fn follow_route(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(projects) = self.projects.clone() else {
            return;
        };
        let Some(project) = projects
            .read(cx)
            .palette_projects()
            .into_iter()
            .map(|(id, _, _)| id)
            .find(|id| id == "demo-xemnas")
        else {
            return;
        };
        let Some(route) = self.route.take() else {
            return;
        };
        projects.update(cx, |screen, cx| screen.select_project(project, window, cx));
        let (destination, view) = route.split_once(':').unwrap_or((route.as_str(), ""));
        let destination = match destination {
            "overview" => Destination::Overview,
            "decisions" => Destination::Decisions,
            "context" => Destination::Context,
            "map" => Destination::Map,
            _ => Destination::Review,
        };
        if !view.is_empty() {
            let view = view.to_owned();
            match destination {
                Destination::Map => {
                    if let Some(map) = &self.map {
                        map.update(cx, |screen, _| screen.open_route(view));
                    }
                }
                Destination::Context => {
                    if let Some(screen) = &self.context {
                        screen.update(cx, |screen, _| screen.scroll_to_end());
                    }
                }
                Destination::Overview => {
                    if let (Some(screen), Some(flow)) = (
                        &self.overview,
                        view.strip_prefix("flow")
                            .and_then(|index| index.parse().ok()),
                    ) {
                        screen.update(cx, |screen, _| screen.open_flow(flow));
                    }
                }
                _ => {}
            }
        }
        self.switch_to(destination, window, cx);
    }

    /// Identifies opt-in sample data visibly, without changing navigation.
    pub fn set_demo(&mut self, demo: bool) {
        self.demo = demo;
    }

    /// Focuses the actual next action rather than an inert path field.
    pub fn initial_focus(&self, cx: &App) -> FocusHandle {
        self.projects
            .as_ref()
            .map(|screen| screen.read(cx).initial_focus())
            .unwrap_or_else(|| self.focus.clone())
    }

    fn on_tab_next(&mut self, _: &TabNext, window: &mut Window, cx: &mut Context<Self>) {
        window.focus_next(cx);
    }

    fn on_tab_prev(&mut self, _: &TabPrev, window: &mut Window, cx: &mut Context<Self>) {
        window.focus_prev(cx);
    }

    fn on_focus_search(&mut self, _: &FocusSearch, window: &mut Window, cx: &mut Context<Self>) {
        if self.settings_open {
            return;
        }
        if self
            .projects
            .as_ref()
            .and_then(|screen| screen.read(cx).selected_project())
            .is_none()
        {
            window.focus(&self.initial_focus(cx), cx);
            return;
        }
        if self.destination == Destination::Decisions {
            if let Some(screen) = &self.decisions {
                screen.update(cx, |screen, cx| screen.focus_search(window, cx));
            }
            return;
        }
        if self.destination != Destination::Review {
            self.switch_to(Destination::Review, window, cx);
        }
        window.focus(&self.search.read(cx).focus_handle(cx), cx);
    }

    fn toggle_palette(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.palette.take().is_some() {
            window.focus(&self.focus, cx);
            cx.notify();
            return;
        }
        self.project_panel = false;
        let field = cx.new(|cx| {
            let mut field = SearchField::new(cx);
            field.stretch();
            field.set_context("Ir para projeto, candidato, decisão ou ação…", cx);
            field
        });
        let subscription = cx.subscribe(&field, |shell, _, _: &SearchChanged, cx| {
            if let Some(palette) = &mut shell.palette {
                palette.highlighted = 0;
            }
            cx.notify();
        });
        window.focus(&field.read(cx).focus_handle(cx), cx);
        self.palette = Some(Palette {
            field,
            highlighted: 0,
            _subscription: subscription,
        });
        cx.notify();
    }

    /// Everything the palette can reach right now, from data already loaded.
    fn palette_items(&self, cx: &App) -> Vec<PaletteItem<Command>> {
        let mut items = Vec::new();
        let selected = self
            .projects
            .as_ref()
            .and_then(|screen| screen.read(cx).selected_project());
        if selected.is_some() {
            for destination in [
                Destination::Overview,
                Destination::Review,
                Destination::Decisions,
                Destination::Context,
                Destination::Map,
            ] {
                items.push(PaletteItem {
                    group: "Ir para",
                    label: destination.label().to_owned(),
                    detail: None,
                    glyph: destination.glyph(),
                    shortcut: Some(destination.shortcut()),
                    command: Command::Go(destination),
                });
            }
        }
        if let Some(inbox) = &self.inbox {
            for (id, question) in inbox.read(cx).palette_rows() {
                items.push(PaletteItem {
                    group: "Candidatos",
                    label: question,
                    detail: None,
                    glyph: IconName::List,
                    shortcut: None,
                    command: Command::Candidate(id),
                });
            }
        }
        if let Some(decisions) = &self.decisions {
            for (id, question) in decisions.read(cx).palette_rows() {
                items.push(PaletteItem {
                    group: "Decisões",
                    label: question,
                    detail: None,
                    glyph: IconName::File,
                    shortcut: None,
                    command: Command::Decision(id),
                });
            }
        }
        if let Some(projects) = &self.projects {
            for (id, name, location) in projects.read(cx).palette_projects() {
                items.push(PaletteItem {
                    group: "Projetos",
                    label: name,
                    detail: Some(location),
                    glyph: IconName::Folder,
                    shortcut: None,
                    command: Command::Project(id),
                });
            }
        }
        if selected.is_some() {
            items.push(PaletteItem {
                group: "Ações",
                label: "Propriedades do projeto".into(),
                detail: None,
                glyph: IconName::Info,
                shortcut: None,
                command: Command::ProjectPanel,
            });
        }
        items.push(PaletteItem {
            group: "Ações",
            label: "Abrir pasta…".into(),
            detail: Some("Acompanhar um novo projeto".into()),
            glyph: IconName::FolderPlus,
            shortcut: None,
            command: Command::OpenFolder,
        });
        let mode = cx.try_global::<ThemeMode>().copied().unwrap_or_default();
        items.push(PaletteItem {
            group: "Ações",
            label: format!("Usar tema {}", mode.toggled().label()),
            detail: None,
            glyph: IconName::Contrast,
            shortcut: None,
            command: Command::ToggleTheme,
        });
        if self.settings.is_some() {
            items.push(PaletteItem {
                group: "Ações",
                label: "Configurações".into(),
                detail: None,
                glyph: IconName::Settings,
                shortcut: None,
                command: Command::OpenSettings,
            });
            for (section, label, detail, glyph) in [
                (
                    SettingsSection::OpenCode,
                    "Testar conexão com o OpenCode",
                    "Configurações › OpenCode",
                    IconName::Link,
                ),
                (
                    SettingsSection::Diagnostics,
                    "Diagnóstico e tarefas",
                    "Configurações › Diagnóstico",
                    IconName::Activity,
                ),
                (
                    SettingsSection::Ai,
                    "IA e privacidade",
                    "Configurações › Extração, chave e envio",
                    IconName::Shield,
                ),
            ] {
                items.push(PaletteItem {
                    group: "Configurações",
                    label: label.into(),
                    detail: Some(detail.into()),
                    glyph,
                    shortcut: None,
                    command: Command::SettingsAt(section),
                });
            }
        }
        items
    }

    fn visible_palette_items(&self, cx: &App) -> Vec<PaletteItem<Command>> {
        let query = self
            .palette
            .as_ref()
            .map(|palette| palette.field.read(cx).value().to_owned())
            .unwrap_or_default();
        palette::filter(&self.palette_items(cx), &query)
    }

    fn move_palette(&mut self, delta: isize, cx: &mut Context<Self>) {
        let count = self.visible_palette_items(cx).len();
        if let Some(palette) = &mut self.palette {
            if count > 0 {
                palette.highlighted =
                    (palette.highlighted as isize + delta).rem_euclid(count as isize) as usize;
            }
            cx.notify();
        }
    }

    fn run_palette(&mut self, index: Option<usize>, window: &mut Window, cx: &mut Context<Self>) {
        let items = self.visible_palette_items(cx);
        let Some(palette) = self.palette.take() else {
            return;
        };
        let Some(item) = items.get(index.unwrap_or(palette.highlighted)).cloned() else {
            self.palette = Some(palette);
            return;
        };
        window.focus(&self.focus, cx);
        match item.command {
            Command::Go(destination) => self.switch_to(destination, window, cx),
            Command::Project(id) => {
                if let Some(projects) = &self.projects {
                    projects.update(cx, |screen, cx| screen.select_project(id, window, cx));
                }
            }
            Command::Candidate(id) => {
                self.switch_to(Destination::Review, window, cx);
                if let Some(inbox) = &self.inbox {
                    inbox.update(cx, |screen, cx| screen.open_candidate(id, window, cx));
                }
            }
            Command::Decision(id) => {
                self.switch_to(Destination::Decisions, window, cx);
                if let Some(decisions) = &self.decisions {
                    decisions.update(cx, |screen, cx| screen.open_decision(id, cx));
                }
            }
            Command::ProjectPanel => {
                self.project_panel = false;
                self.panel_dismissed_at = None;
                self.toggle_project_panel(window, cx);
            }
            Command::OpenFolder => {
                if let Some(projects) = &self.projects {
                    projects.update(cx, |screen, cx| screen.open_folder_dialog(cx));
                }
            }
            Command::ToggleTheme => self.toggle_theme(window, cx),
            Command::OpenSettings => self.toggle_settings(window, cx),
            Command::SettingsAt(section) => self.open_settings_at(section, cx),
        }
        cx.notify();
    }

    /// The palette overlay: a dimmed window and a centred list under a field.
    fn render_palette(&mut self, cx: &mut Context<Self>) -> Option<gpui::AnyElement> {
        let theme = self.theme;
        let field = self.palette.as_ref()?.field.clone();
        let highlighted = self.palette.as_ref()?.highlighted;
        let items = self.visible_palette_items(cx);
        let mut list = div()
            .id("palette-list")
            .max_h(px(PALETTE_MAX_HEIGHT))
            .overflow_y_scroll()
            .py(px(SpacingScale::S1));
        let mut group = "";
        for (index, item) in items.iter().enumerate() {
            if item.group != group {
                group = item.group;
                list = list.child(
                    section_label(&theme, group)
                        .px(px(SpacingScale::S4))
                        .pt(px(SpacingScale::S3))
                        .pb(px(SpacingScale::S1)),
                );
            }
            let active = index == highlighted;
            let foreground = if active {
                theme.colors.text_primary()
            } else {
                theme.colors.text_secondary()
            };
            list = list.child(
                text_style(div(), TypeScale::BODY_SMALL)
                    .id(("palette-item", index))
                    .mx(px(SpacingScale::S2))
                    .h(px(ControlSize::MD + 4.0))
                    .px(px(SpacingScale::S2))
                    .flex()
                    .items_center()
                    .gap(px(SpacingScale::S2))
                    .rounded(theme.radius.control())
                    // On the floating surface `selection` is nearly the panel colour;
                    // a lavender veil reads as the highlight on both materials.
                    .when(active, |row| row.bg(theme.colors.glass_fill_medium()))
                    .text_color(foreground)
                    .cursor_pointer()
                    .on_mouse_move(cx.listener(move |this, _, _, cx| {
                        if let Some(palette) = &mut this.palette {
                            if palette.highlighted != index {
                                palette.highlighted = index;
                                cx.notify();
                            }
                        }
                    }))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.run_palette(Some(index), window, cx)
                    }))
                    .child(icon(item.glyph, 14.0, foreground))
                    .child(div().min_w(px(0.0)).truncate().child(item.label.clone()))
                    .children(item.detail.clone().map(|detail| {
                        text_style(div(), TypeScale::META)
                            .min_w(px(0.0))
                            .flex_1()
                            .truncate()
                            .text_color(theme.colors.text_muted())
                            .child(detail)
                    }))
                    .when(item.detail.is_none(), |row| row.child(div().flex_1()))
                    .children(item.shortcut.map(|key| kbd(theme.colors.text_muted(), key))),
            );
        }
        if items.is_empty() {
            list = list.child(
                text_style(div(), TypeScale::BODY_SMALL)
                    .px(px(SpacingScale::S4))
                    .py(px(SpacingScale::S4))
                    .text_color(theme.colors.text_muted())
                    .child("Nada corresponde. A paleta procura no que já está carregado."),
            );
        }
        let panel = div()
            .id("palette")
            .key_context("Palette")
            .w(px(560.0))
            .max_w(gpui::relative(0.9))
            .mt(gpui::relative(0.12))
            .flex()
            .flex_col()
            .rounded(theme.radius.dialog())
            .border_1()
            .border_color(theme.colors.hairline_divider())
            .bg(theme.colors.floating())
            .shadow(vec![BoxShadow::new(
                px(0.0),
                px(24.0),
                theme.colors.shadow_emphasis().into(),
            )
            .blur_radius(px(48.0))])
            .occlude()
            .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_action(cx.listener(|this, _: &PaletteDown, _, cx| this.move_palette(1, cx)))
            .on_action(cx.listener(|this, _: &PaletteUp, _, cx| this.move_palette(-1, cx)))
            .on_action(
                cx.listener(|this, _: &PaletteRun, window, cx| this.run_palette(None, window, cx)),
            )
            .on_action(
                cx.listener(|this, _: &PaletteClose, window, cx| this.toggle_palette(window, cx)),
            )
            .child(
                div()
                    .p(px(SpacingScale::S2))
                    .border_b_1()
                    .border_color(theme.colors.hairline_divider())
                    .child(field),
            )
            .child(list)
            .child(
                text_style(div(), TypeScale::META)
                    .px(px(SpacingScale::S4))
                    .py(px(SpacingScale::S2))
                    .flex()
                    .gap(px(SpacingScale::S3))
                    .border_t_1()
                    .border_color(theme.colors.hairline_divider())
                    .text_color(theme.colors.text_muted())
                    .child("↑↓ navegar")
                    .child("Enter abrir")
                    .child("Esc fechar"),
            );
        Some(
            deferred(
                div()
                    .id("palette-scrim")
                    .absolute()
                    .inset_0()
                    .flex()
                    .justify_center()
                    .items_start()
                    .bg(theme.colors.scrim())
                    .occlude()
                    .on_mouse_down(
                        gpui::MouseButton::Left,
                        cx.listener(|this, _, window, cx| {
                            if this.palette.is_some() {
                                this.toggle_palette(window, cx);
                            }
                        }),
                    )
                    .child(fade_in(div().child(panel), "palette-in")),
            )
            .with_priority(3)
            .into_any_element(),
        )
    }

    fn on_move(&mut self, delta: isize, window: &mut Window, cx: &mut Context<Self>) {
        match self.destination {
            Destination::Review => {
                if let Some(screen) = &self.inbox {
                    screen.update(cx, |screen, cx| screen.move_selection(delta, window, cx));
                }
            }
            Destination::Decisions => {
                if let Some(screen) = &self.decisions {
                    screen.update(cx, |screen, cx| screen.move_selection(delta, window, cx));
                }
            }
            Destination::Context | Destination::Map | Destination::Overview => {}
        }
    }

    fn on_review_key(&mut self, action: usize, window: &mut Window, cx: &mut Context<Self>) {
        if self.destination != Destination::Review {
            return;
        }
        if let Some(screen) = &self.inbox {
            screen.update(cx, |screen, cx| screen.run_shortcut(action, window, cx));
        }
    }

    /// Opens a decision in Decisões from another destination (Contexto).
    fn show_decision(&mut self, id: String, cx: &mut Context<Self>) {
        self.destination = Destination::Decisions;
        if let Some(decisions) = &self.decisions {
            decisions.update(cx, |screen, cx| screen.open_decision(id, cx));
        }
        cx.notify();
    }

    /// Opens an entity in the Mapa from another destination (Visão).
    fn show_entity(&mut self, id: String, cx: &mut Context<Self>) {
        self.destination = Destination::Map;
        if let Some(map) = &self.map {
            map.update(cx, |screen, cx| screen.show_entity(id, cx));
        }
        cx.notify();
    }

    fn switch_to(&mut self, destination: Destination, window: &mut Window, cx: &mut Context<Self>) {
        self.destination = destination;
        window.focus(&self.destination_focus[destination.index()], cx);
        match destination {
            Destination::Review => {
                self.search.update(cx, |search, cx| {
                    search.set_context("Filtrar candidatos carregados", cx)
                });
                if let Some(screen) = &self.inbox {
                    screen.update(cx, |screen, cx| screen.refresh(cx));
                }
            }
            Destination::Decisions => {
                if let Some(screen) = &self.decisions {
                    screen.update(cx, |screen, cx| screen.refresh(cx));
                }
            }
            Destination::Context => {
                if let Some(screen) = &self.context {
                    screen.update(cx, |screen, cx| screen.refresh(cx));
                }
            }
            Destination::Map => {
                if let Some(screen) = &self.map {
                    screen.update(cx, |screen, cx| screen.refresh(cx));
                }
            }
            Destination::Overview => {
                if let Some(screen) = &self.overview {
                    screen.update(cx, |screen, cx| screen.refresh(cx));
                }
            }
        }
        cx.notify();
    }

    fn toggle_project_panel(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let just_dismissed = self
            .panel_dismissed_at
            .take()
            .is_some_and(|at| at.elapsed() < PANEL_REOPEN_GUARD);
        self.project_panel = !self.project_panel && !just_dismissed;
        if self.project_panel {
            if let Some(projects) = &self.projects {
                window.focus(&projects.read(cx).panel_focus(), cx);
            }
        }
        cx.notify();
    }

    fn close_project_panel(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.project_panel {
            self.project_panel = false;
            window.focus(&self.project_focus, cx);
            cx.notify();
        }
    }

    /// The project name as a quiet button: it opens the project panel.
    fn project_crumb(&self, name: String, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let open = self.project_panel;
        text_style(div(), TypeScale::ROW_TITLE)
            .id("project-crumb")
            .h(px(ControlSize::SM))
            .px(px(SpacingScale::S2))
            .flex()
            .items_center()
            .gap(px(6.0))
            .max_w(px(280.0))
            .rounded(theme.radius.control())
            .when(open, |crumb| crumb.bg(theme.colors.selection()))
            .when(!open, |crumb| {
                crumb.hover(move |style| style.bg(theme.colors.hover_veil()))
            })
            .role(Role::Button)
            .aria_label(format!("{name}: propriedades do projeto"))
            .aria_expanded(open)
            .tooltip(tooltip("Propriedades do projeto", None))
            .track_focus(&self.project_focus)
            .focus_visible(focus_ring(&theme))
            .cursor_pointer()
            .on_click(cx.listener(|this, _, window, cx| this.toggle_project_panel(window, cx)))
            .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, window, cx| {
                if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                    this.toggle_project_panel(window, cx);
                    cx.stop_propagation();
                }
            }))
            .child(div().min_w(px(0.0)).truncate().child(name))
            .child(icon(IconName::ChevronDown, 12.0, theme.colors.text_muted()))
    }

    /// One tab recipe for every project destination: quiet at rest, a soft
    /// filled pill when selected, no underline or outline competing with it.
    fn nav_tab(&self, destination: Destination, cx: &mut Context<Self>) -> impl IntoElement {
        let hovered = self.hovered_tab == Some(destination);
        let theme = self.theme;
        let selected = self.destination == destination;
        let foreground = if selected {
            theme.colors.text_primary()
        } else {
            theme.colors.text_muted()
        };
        let glyph = icon(destination.glyph(), 14.0, foreground);
        let count = (destination == Destination::Review).then(|| {
            self.inbox
                .as_ref()
                .and_then(|screen| screen.read(cx).total_count())
                .map(|count| count.to_string())
                .unwrap_or_else(|| "…".into())
        });
        text_style(div(), TypeScale::BODY_SMALL)
            .id(destination.id())
            .h(px(ControlSize::SM))
            .px(px(10.0))
            .flex()
            .items_center()
            .gap(px(SpacingScale::S2))
            .rounded(theme.radius.control())
            .role(Role::Tab)
            .aria_label(destination.label())
            .aria_selected(selected)
            .track_focus(&self.destination_focus[destination.index()])
            .focus_visible(focus_ring(&theme))
            .cursor_pointer()
            .when(selected, |tab| {
                tab.bg(theme.colors.selection())
                    .text_color(theme.colors.text_primary())
            })
            .on_hover(cx.listener(move |this, hovered: &bool, _, cx| {
                if track_hover(&mut this.hovered_tab, destination, *hovered) {
                    cx.notify();
                }
            }))
            .on_click(
                cx.listener(move |this, _, window, cx| this.switch_to(destination, window, cx)),
            )
            .on_key_down(
                cx.listener(move |this, event: &gpui::KeyDownEvent, window, cx| {
                    if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                        this.switch_to(destination, window, cx);
                        cx.stop_propagation();
                    }
                }),
            )
            .child(glyph)
            .child(destination.label())
            .children(count.map(|count| count_chip(&theme, count)))
            .tooltip(tooltip(destination.label(), Some(destination.shortcut())))
            .with_spring(
                ElementId::Name(format!("{}-hover", destination.id()).into()),
                SpringAnimation::new(MotionTokens::HOVER_SPRING).to(hovered && !selected),
                move |tab, phase| {
                    if selected {
                        return tab;
                    }
                    let veil = theme.colors.hover_veil();
                    tab.bg(phase.interpolate_between_clamped(0.0..=1.0, veil.alpha(0.0), veil))
                        .text_color(phase.interpolate_between_clamped(
                            0.0..=1.0,
                            theme.colors.text_muted(),
                            theme.colors.text_secondary(),
                        ))
                },
            )
    }

    fn toggle_theme(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let mode = cx
            .try_global::<ThemeMode>()
            .copied()
            .unwrap_or_default()
            .toggled();
        cx.set_global(mode);
        window.refresh();
        cx.notify();
    }

    /// A quiet icon action in the title bar; the palette name lives in the
    /// accessible label instead of a permanent text chip beside the brand.
    fn theme_button(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let mode = cx.try_global::<ThemeMode>().copied().unwrap_or_default();
        let label = format!(
            "Tema: {}. Alternar para {}",
            mode.label(),
            mode.toggled().label()
        );
        icon_action(&theme, "theme-switch", &label)
            .tooltip(tooltip(format!("Tema: {}", mode.toggled().label()), None))
            .mr(px(SpacingScale::S2))
            .track_focus(&self.theme_focus)
            .on_click(cx.listener(Self::on_theme_click))
            .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, window, cx| {
                if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                    this.toggle_theme(window, cx);
                    cx.stop_propagation();
                }
            }))
            .child(icon(
                IconName::Contrast,
                16.0,
                theme.colors.text_secondary(),
            ))
    }

    /// Opens the app-level settings page, or returns to the projects.
    /// Opens the settings page on `section` (the sidebar status line leads to
    /// the integration or to the failures behind its counts).
    fn open_settings_at(&mut self, section: SettingsSection, cx: &mut Context<Self>) {
        let Some(screen) = &self.settings else {
            return;
        };
        self.settings_open = true;
        screen.update(cx, |screen, cx| screen.open_section(section, cx));
        cx.notify();
    }

    fn toggle_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(screen) = &self.settings else {
            return;
        };
        self.settings_open = !self.settings_open;
        if self.settings_open {
            screen.update(cx, |screen, cx| screen.open(cx));
        } else {
            window.focus(&self.initial_focus(cx), cx);
        }
        cx.notify();
    }

    fn settings_button(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let open = self.settings_open;
        icon_action(
            &theme,
            "settings-open",
            if open {
                "Fechar configurações"
            } else {
                "Configurações"
            },
        )
        .mr(px(SpacingScale::S1))
        .track_focus(&self.settings_focus)
        .aria_selected(open)
        .when(open, |button| button.bg(theme.colors.selection()))
        .on_click(cx.listener(|this, _, window, cx| this.toggle_settings(window, cx)))
        .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, window, cx| {
            if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                this.toggle_settings(window, cx);
                cx.stop_propagation();
            }
        }))
        .child(icon(
            IconName::Settings,
            16.0,
            if open {
                theme.colors.text_primary()
            } else {
                theme.colors.text_secondary()
            },
        ))
    }

    fn on_theme_click(
        &mut self,
        _: &gpui::ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.toggle_theme(window, cx);
    }
}

impl<R: ProjectRepository + InboxStore + DecisionStore + ContextStores> Render for Shell<R> {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.route.is_some() {
            self.follow_route(window, cx);
        }
        // The material follows the system appearance; in a light theme it
        // turns pale under a dark palette, so the chrome stays opaque there.
        let glass = self.backdrop
            && matches!(
                window.appearance(),
                gpui::WindowAppearance::Dark | gpui::WindowAppearance::VibrantDark
            );
        if cx.try_global::<Backdrop>().copied() != Some(Backdrop(glass)) {
            cx.set_global(Backdrop(glass));
        }
        self.theme = Theme::current(cx);
        let theme = self.theme;
        // The settings page names itself while open; otherwise the project leads
        // the title so Alt+Tab and the taskbar tell windows and projects apart.
        let project_name = self
            .projects
            .as_ref()
            .and_then(|screen| screen.read(cx).selected_project())
            .map(|project| project.name().to_owned());
        let window_title = if self.settings_open {
            "xemnas — Configurações".to_owned()
        } else {
            match project_name {
                Some(name) => format!("{name} · {} — xemnas", self.destination.label()),
                None => "xemnas".to_owned(),
            }
        };
        window.set_window_title(&window_title);

        let title = div()
            .id("title-bar")
            .h(px(TITLE_BAR_HEIGHT))
            .flex_none()
            .pl(px(SpacingScale::S4))
            .flex()
            .items_center()
            .when(!theme.colors.is_glass(), |bar| bar.bg(theme.colors.rail()))
            .when(!theme.colors.is_glass(), |bar| {
                bar.border_b_1()
                    .border_color(theme.colors.hairline_divider())
            })
            .child(
                div()
                    .flex_1()
                    .h_full()
                    .flex()
                    .items_center()
                    .gap(px(SpacingScale::S2))
                    .window_control_area(WindowControlArea::Drag)
                    .child(app_icon(18.0))
                    .child(wordmark(&theme, 14.0, 600.0))
                    .when(self.demo, |brand| {
                        brand.child(
                            text_style(div(), TypeScale::META)
                                .ml(px(SpacingScale::S2))
                                .px(px(SpacingScale::S2))
                                .rounded(px(4.0))
                                .border_1()
                                .border_color(theme.colors.hairline_divider())
                                .text_color(theme.colors.text_muted())
                                .child("Demonstração · dados fictícios"),
                        )
                    }),
            )
            .when(self.settings.is_some(), |title| {
                title.child(self.settings_button(cx))
            })
            .child(self.theme_button(cx))
            .child(window_controls(&theme, window.is_maximized()));

        let selected = self
            .projects
            .as_ref()
            .and_then(|screen| screen.read(cx).selected_project());
        let settings = self.settings.clone().filter(|_| self.settings_open);
        let body: gpui::AnyElement = if let Some(settings) = settings {
            fade_in(
                div().size_full().child(settings),
                ElementId::Name("content-settings".into()),
            )
            .into_any_element()
        } else if let Some(projects) = self.projects.clone() {
            let status = self.status_line(cx);
            let sidebar = projects.update(cx, |screen, cx| screen.render_sidebar(status, cx));
            let mut panel = if self.project_panel {
                projects.update(cx, |screen, cx| screen.render_project_panel(cx))
            } else {
                None
            };
            let content = if selected.is_some() && self.destination == Destination::Overview {
                self.overview
                    .as_ref()
                    .map(|screen| screen.clone().into_any_element())
                    .unwrap_or_else(|| div().into_any_element())
            } else if selected.is_some() && self.destination == Destination::Map {
                self.map
                    .as_ref()
                    .map(|screen| screen.clone().into_any_element())
                    .unwrap_or_else(|| div().into_any_element())
            } else if selected.is_some() && self.destination == Destination::Context {
                self.context
                    .as_ref()
                    .map(|screen| screen.clone().into_any_element())
                    .unwrap_or_else(|| div().into_any_element())
            } else if selected.is_some() && self.destination == Destination::Decisions {
                self.decisions
                    .as_ref()
                    .map(|screen| screen.clone().into_any_element())
                    .unwrap_or_else(|| div().into_any_element())
            } else if selected.is_some() && self.destination == Destination::Review {
                self.inbox
                    .as_ref()
                    .map(|screen| screen.clone().into_any_element())
                    .unwrap_or_else(|| div().into_any_element())
            } else {
                projects.update(cx, |screen, cx| screen.render_details(cx))
            };
            div()
                .size_full()
                .flex()
                .child(sidebar)
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.0))
                        .h_full()
                        .flex()
                        .flex_col()
                        .bg(theme.colors.content())
                        // Over the material the content is a card: margins on
                        // the free sides, flush against the sidebar, 10 px
                        // radius, hairline edge and a card shadow (Fluent's
                        // card pattern; Arc and Zen frame content the same way).
                        .when(theme.colors.is_glass(), |content| {
                            content
                                .mr(px(SpacingScale::S2))
                                .mb(px(SpacingScale::S2))
                                .rounded(theme.radius.surface())
                                .border_1()
                                .border_color(theme.colors.glass_border_card())
                                .overflow_hidden()
                                .shadow(vec![
                                    BoxShadow::new(
                                        px(0.0),
                                        px(8.0),
                                        theme.colors.shadow_low().into(),
                                    )
                                    .blur_radius(px(24.0)),
                                    BoxShadow::new(
                                        px(0.0),
                                        px(1.0),
                                        theme.colors.inset_highlight().alpha(0.06).into(),
                                    )
                                    .inset(),
                                ])
                        })
                        // One bar: the project as breadcrumb (its properties open
                        // in a panel from there), then its destinations.
                        .children(selected.as_ref().map(|project| {
                            div()
                                .relative()
                                .h(px(PROJECT_BAR_HEIGHT))
                                .flex_none()
                                .px(px(SpacingScale::S3))
                                .flex()
                                .items_center()
                                .gap(px(SpacingScale::S1))
                                .border_b_1()
                                .border_color(theme.colors.hairline_divider())
                                .child(self.project_crumb(project.name().to_owned(), cx))
                                .child(
                                    text_style(div(), TypeScale::BODY_SMALL)
                                        .text_color(theme.colors.text_disabled())
                                        .child("/"),
                                )
                                .child(
                                    div()
                                        .id("project-destinations")
                                        .flex()
                                        .gap(px(SpacingScale::S1))
                                        .role(Role::TabList)
                                        .child(self.nav_tab(Destination::Overview, cx))
                                        .child(self.nav_tab(Destination::Review, cx))
                                        .child(self.nav_tab(Destination::Decisions, cx))
                                        .child(self.nav_tab(Destination::Context, cx))
                                        .child(self.nav_tab(Destination::Map, cx)),
                                )
                                .children(panel.take().map(|panel| {
                                    deferred(
                                        div()
                                            .id("project-panel")
                                            .absolute()
                                            .top(px(PROJECT_BAR_HEIGHT - 4.0))
                                            .left(px(SpacingScale::S3))
                                            .rounded(theme.radius.surface())
                                            .border_1()
                                            .border_color(theme.colors.hairline_divider())
                                            .bg(theme.colors.floating())
                                            .shadow(vec![BoxShadow::new(
                                                px(0.0),
                                                px(12.0),
                                                theme.colors.shadow_emphasis().into(),
                                            )
                                            .blur_radius(px(32.0))])
                                            .occlude()
                                            .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                                                this.project_panel = false;
                                                this.panel_dismissed_at =
                                                    Some(std::time::Instant::now());
                                                cx.notify();
                                            }))
                                            .child(fade_in(div().child(panel), "project-panel-in")),
                                    )
                                    .with_priority(1)
                                }))
                        }))
                        .child(fade_in(
                            div().flex_1().min_h(px(0.0)).child(content),
                            ElementId::Name(
                                format!(
                                    "content-{:?}-{}",
                                    self.destination,
                                    selected
                                        .as_ref()
                                        .map(|project| project.id().as_str())
                                        .unwrap_or("none")
                                )
                                .into(),
                            ),
                        )),
                )
                .into_any_element()
        } else {
            error_state(
                &theme,
                "startup-error",
                "Não foi possível abrir o banco de dados",
                "Os projetos acompanhados não puderam ser carregados.",
                "Feche e abra o app novamente. Se o erro persistir, verifique o espaço em disco.",
            )
            .into_any_element()
        };

        let palette = self.render_palette(cx);
        div()
            .id("xemnas-shell")
            .role(Role::Application)
            .aria_label("xemnas")
            .size_full()
            .flex()
            .flex_col()
            // Over the material the frame tint is painted once, for the whole
            // window: title bar, sidebar and the gutter around the content card
            // are one surface, so no strip shows raw material beside a tinted one.
            .bg(if theme.colors.is_glass() {
                theme.colors.chrome()
            } else {
                theme.colors.canvas()
            })
            .font_family(Theme::font_interface())
            .text_color(theme.colors.text_primary())
            .track_focus(&self.focus)
            .key_context("xemnas")
            .on_action(cx.listener(Self::on_tab_next))
            .on_action(cx.listener(Self::on_tab_prev))
            .on_action(cx.listener(Self::on_focus_search))
            .on_action(
                cx.listener(|this, _: &TogglePalette, window, cx| this.toggle_palette(window, cx)),
            )
            .on_action(cx.listener(|this, _: &GoReview, window, cx| {
                this.switch_to(Destination::Review, window, cx)
            }))
            .on_action(cx.listener(|this, _: &GoContext, window, cx| {
                this.switch_to(Destination::Context, window, cx)
            }))
            .on_action(cx.listener(|this, _: &GoOverview, window, cx| {
                this.switch_to(Destination::Overview, window, cx)
            }))
            .on_action(cx.listener(|this, _: &GoMap, window, cx| {
                this.switch_to(Destination::Map, window, cx)
            }))
            .on_action(cx.listener(|this, _: &GoDecisions, window, cx| {
                this.switch_to(Destination::Decisions, window, cx)
            }))
            .on_action(cx.listener(|this, _: &NextItem, window, cx| this.on_move(1, window, cx)))
            .on_action(cx.listener(|this, _: &PrevItem, window, cx| this.on_move(-1, window, cx)))
            .on_action(
                cx.listener(|this, _: &RejectItem, window, cx| this.on_review_key(0, window, cx)),
            )
            .on_action(
                cx.listener(|this, _: &SnoozeItem, window, cx| this.on_review_key(1, window, cx)),
            )
            .on_action(
                cx.listener(|this, _: &AdjustItem, window, cx| this.on_review_key(2, window, cx)),
            )
            .on_action(
                cx.listener(|this, _: &ConfirmItem, window, cx| this.on_review_key(3, window, cx)),
            )
            .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, window, cx| {
                if event.keystroke.key == "escape" && this.project_panel {
                    this.close_project_panel(window, cx);
                    cx.stop_propagation();
                }
            }))
            .relative()
            .child(title)
            .child(div().flex_1().min_h(px(0.0)).overflow_hidden().child(body))
            .children(palette)
    }
}

/// Client-side Windows caption controls; GPUI forwards their hitboxes to the OS.
/// The system performs minimize, maximize/restore and close, including Snap Layouts.
fn window_controls(theme: &Theme, maximized: bool) -> impl IntoElement {
    div()
        .flex()
        .flex_none()
        .h_full()
        .font_family("Segoe Fluent Icons")
        .child(caption_button(
            theme,
            "window-minimize",
            "Minimizar",
            "\u{e921}",
            WindowControlArea::Min,
            false,
        ))
        .child(caption_button(
            theme,
            "window-maximize",
            if maximized { "Restaurar" } else { "Maximizar" },
            if maximized { "\u{e923}" } else { "\u{e922}" },
            WindowControlArea::Max,
            false,
        ))
        .child(caption_button(
            theme,
            "window-close",
            "Fechar",
            "\u{e8bb}",
            WindowControlArea::Close,
            true,
        ))
}

fn caption_button(
    theme: &Theme,
    id: &'static str,
    label: &'static str,
    glyph: &'static str,
    area: WindowControlArea,
    destructive: bool,
) -> impl IntoElement {
    let hover = if destructive {
        theme.colors.status_danger()
    } else {
        theme.colors.hover_veil()
    };
    div()
        .id(id)
        .w(px(46.0))
        .h_full()
        .flex()
        .items_center()
        .justify_center()
        .text_size(px(10.0))
        .text_color(theme.colors.text_secondary())
        .hover(move |style| style.bg(hover).text_color(theme.colors.text_primary()))
        .window_control_area(area)
        .role(Role::Button)
        .aria_label(label)
        .child(glyph)
}
