//! Contexto: what holds in the selected project right now, and what the agent
//! receives from it.
//!
//! Four surfaces over existing use cases: the project's context mode
//! (`ContextSettings`), the decisions in force (`Decisions::list`), the rules
//! of the project (`Claims`) and a Context Pack preview for a typed task
//! (`ContextPacks::build_pack`, exported through `application::export`). All
//! storage work runs off the UI thread; a project generation discards
//! responses that arrive after the user switched projects.

use application::knowledge_review::KnowledgeReviewApi;
use std::path::PathBuf;
use std::sync::Arc;

use application::claim_suggestions::{ClaimSuggestionStore, ClaimSuggestions};
use application::claims::{ClaimRecord, ClaimStore, Claims, NewClaim};
use application::context::{
    ContextPack, ContextPacks, ContextProvider, ContextRequest, ContextStore,
};
use application::context_settings::{
    ContextMode, ContextSettings, ContextSettingsStore, ProjectContextSettings,
};
use application::decisions::{
    DecisionFilter, DecisionStatus, DecisionStore, DecisionSummary, Decisions,
};
use application::documents::{
    DocumentIndex, DocumentKind, DocumentStore, Documents, ImportedDocument, ProjectDocument,
};
use application::export::{preview_pack, write_pack, ExportFormat};
use application::graph::GraphStore;
use application::injection::{
    short_ref, Deliveries, Delivery, DeliverySummary, InjectionHistory, InjectionMode, ItemKind,
    DEFAULT_BUDGET_TOKENS, MAX_BUDGET_TOKENS, MIN_BUDGET_TOKENS,
};
use application::projects::ProjectRepository;
use application::relations::RelationStore;
use domain::claims::ClaimKind;
use std::collections::BTreeMap;

use gpui::prelude::*;
use gpui::{
    div, px, AnyElement, Context, Div, Entity, EventEmitter, Render, Role, Subscription, Toggled,
    Window,
};

use super::format::{calendar_date, clock, day_heading, short_date, thousands};
use crate::i18n::context as t;
use crate::ui::controls::{action_button, button_foreground, ButtonKind};
use crate::ui::icons::{icon, IconName};
use crate::ui::patterns::{
    count_chip, count_up, empty_panel, error_banner, index_rail, index_row, meter, radio_list,
    radio_row, reading_page, ring, section_header, section_label, share, skeleton_list, sparkline,
    status_pill, toast, TOAST_DURATION,
};
use crate::ui::search_field::{SearchChanged, SearchField};
use crate::ui::theme::{text_style, Theme};
use crate::ui::tokens::{tint, RadiusScale, SpacingScale, TypeScale};

/// Every port the Contexto screen reads through.
pub trait ContextStores:
    DecisionStore
    + ClaimStore
    + ProjectRepository
    + ContextStore
    + RelationStore
    + ContextSettingsStore
    + GraphStore
    + DocumentStore
    + application::captures::CaptureRepository
    + InjectionHistory
    + ClaimSuggestionStore
    + application::observations::ObservationStore
    + application::observations::ObservationDeliveryStore
    + Clone
    + Send
    + 'static
{
}

impl<T> ContextStores for T where
    T: DecisionStore
        + ClaimStore
        + ProjectRepository
        + ContextStore
        + RelationStore
        + ContextSettingsStore
        + GraphStore
        + DocumentStore
        + application::captures::CaptureRepository
        + InjectionHistory
        + ClaimSuggestionStore
        + application::observations::ObservationStore
        + application::observations::ObservationDeliveryStore
        + Clone
        + Send
        + 'static
{
}

/// The use cases behind the screen, built by the composition root.
pub struct ContextServices<S> {
    /// Read-only review, independent of the mutable Context loader.
    pub reviewer: Option<Arc<dyn KnowledgeReviewApi>>,
    /// Decisions in force.
    pub decisions: Decisions<S>,
    /// Project rules.
    pub claims: Claims<S>,
    /// Context mode per project.
    pub settings: ContextSettings<S>,
    /// Context Pack builder.
    pub packs: ContextPacks<S>,
    /// Project documentation read from the folder.
    pub documents: Documents<S>,
    /// What the agent actually received.
    pub deliveries: Deliveries<S>,
    /// Rules whose source decision was replaced.
    pub derived: ClaimSuggestions<S>,
}

/// Asks the shell to open a decision in Decisões.
pub struct OpenDecision(pub String);

/// Deliveries read for the history.
const DELIVERIES_LIMIT: usize = 60;
/// Deliveries shown on the overview.
const RECENT_SHOWN: usize = 3;

/// The pages of the Contexto destination.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Section {
    KnowledgeReview,
    Overview,
    Deliveries,
    Test,
    Decisions,
    Rules,
    Documents,
    Mode,
}

/// The four entries of the index. The pages inside a group are reached by a
/// switch at the top of the page, so the index stays at four (a person holds
/// about four things at a glance) while every page is still two presses away.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Group {
    Overview,
    Sources,
    Deliveries,
    Settings,
}

impl Group {
    const ALL: [Self; 4] = [
        Self::Overview,
        Self::Sources,
        Self::Deliveries,
        Self::Settings,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::Overview => t::group_overview(),
            Self::Sources => t::group_sources(),
            Self::Deliveries => t::group_deliveries(),
            Self::Settings => t::group_settings(),
        }
    }

    fn glyph(self) -> IconName {
        match self {
            Self::Overview => IconName::Gauge,
            Self::Sources => IconName::Book,
            Self::Deliveries => IconName::Clock,
            Self::Settings => IconName::Settings,
        }
    }

    fn id(self) -> &'static str {
        match self {
            Self::Overview => "context-nav-overview",
            Self::Sources => "context-nav-sources",
            Self::Deliveries => "context-nav-deliveries",
            Self::Settings => "context-nav-mode",
        }
    }

    /// The pages of the group, in the order of its switch.
    fn members(self) -> &'static [Section] {
        match self {
            Self::Overview => &[Section::Overview],
            Self::Sources => &[
                Section::Decisions,
                Section::Rules,
                Section::Documents,
                Section::KnowledgeReview,
            ],
            Self::Deliveries => &[Section::Deliveries, Section::Test],
            Self::Settings => &[Section::Mode],
        }
    }
}

impl Section {
    /// The index entry this page belongs to.
    fn group(self) -> Group {
        match self {
            Self::Overview => Group::Overview,
            Self::Decisions | Self::Rules | Self::Documents | Self::KnowledgeReview => {
                Group::Sources
            }
            Self::Deliveries | Self::Test => Group::Deliveries,
            Self::Mode => Group::Settings,
        }
    }

    /// The id of the page's tab on the group's switch.
    fn tab_id(self) -> &'static str {
        match self {
            Self::Decisions => "context-tab-decisions",
            Self::Rules => "context-tab-rules",
            Self::Documents => "context-tab-documents",
            Self::KnowledgeReview => "context-tab-knowledge-review",
            Self::Deliveries => "context-tab-deliveries",
            Self::Test => "context-tab-test",
            Self::Overview => "context-tab-overview",
            Self::Mode => "context-tab-mode",
        }
    }

    /// The word on the group's switch.
    fn tab(self) -> &'static str {
        match self {
            Self::Decisions => t::tab_decisions(),
            Self::Rules => t::tab_rules(),
            Self::Documents => t::tab_documents(),
            Self::KnowledgeReview => t::tab_knowledge_review(),
            Self::Deliveries => t::tab_history(),
            Self::Test => t::tab_test_task(),
            Self::Overview => t::group_overview(),
            Self::Mode => t::tab_delivery_mode(),
        }
    }
}

/// Documents listed per kind before "e mais N".
const DOCUMENTS_SHOWN: usize = 8;
/// Rules of each kind built first, and how many "Mostrar mais" adds (twice).
const RULES_PAGE: usize = 10;

/// How many decisions in force the screen lists.
const IN_FORCE_LIMIT: usize = 50;

struct Snapshot {
    settings: ProjectContextSettings,
    decisions: Vec<DecisionSummary>,
    claims: Vec<ClaimRecord>,
    documents: Vec<ProjectDocument>,
    deliveries: Vec<Delivery>,
    /// Claims whose source decision was superseded.
    review: Vec<String>,
}

enum Outcome {
    Loaded(Result<Box<Snapshot>, String>),
    SettingsSaved(Result<ProjectContextSettings, String>),
    Claims(Result<Vec<ClaimRecord>, String>, &'static str),
    Pack(Result<Box<ContextPack>, String>),
    Exported(Result<Option<PathBuf>, String>),
    Documents(Result<(DocumentIndex, Vec<ProjectDocument>), String>),
    Imported(Result<(ImportedDocument, Vec<ProjectDocument>), String>),
}

/// The Contexto destination of a selected project.
pub struct ContextScreen<S: ContextStores> {
    review: Entity<super::knowledge_review::KnowledgeReviewScreen>,
    backend: Option<ContextServices<S>>,
    project: Option<String>,
    generation: u64,
    busy: bool,
    snapshot: Option<Snapshot>,
    mode: ContextMode,
    budget: Entity<SearchField>,
    statement: Entity<SearchField>,
    task: Entity<SearchField>,
    claim_kind: ClaimKind,
    confirm_retire: Option<String>,
    /// Rules built per kind (by kind index); the rest waits behind
    /// "Mostrar mais": every scroll step rebuilds the page, so what is
    /// built is what scrolls.
    shown_rules: BTreeMap<usize, usize>,
    pack: Option<ContextPack>,
    error: Option<String>,
    notice: Option<String>,
    scroll: gpui::ScrollHandle,
    /// Demo-only: scroll to the end once loaded (`--open context:end`).
    scroll_to_end: bool,
    section: Section,
    /// The page last open in each index group, by group id.
    last_in_group: BTreeMap<&'static str, Section>,
    /// The delivery whose items are open in the history.
    open_delivery: Option<String>,
    focus: std::collections::BTreeMap<&'static str, gpui::FocusHandle>,
    _subscriptions: Vec<Subscription>,
}

impl<S: ContextStores> EventEmitter<OpenDecision> for ContextScreen<S> {}

impl<S: ContextStores> ContextScreen<S> {
    /// Whether review is available for the command palette.
    pub fn has_reviewer(&self, cx: &gpui::App) -> bool {
        self.review.read(cx).available()
    }
    /// Scrolls to the end of the page once it is loaded (demo captures).
    pub fn scroll_to_end(&mut self) {
        self.scroll_to_end = true;
    }

    /// Opens a page by name (`deliveries`, `test`, `decisions`, `rules`,
    /// `documents`, `mode`); demo captures reach it without input.
    pub fn open_section(&mut self, name: &str) {
        self.section = match name {
            "knowledge-review" | "knowledge-review-results" => Section::KnowledgeReview,
            "deliveries" => Section::Deliveries,
            "test" => Section::Test,
            "decisions" => Section::Decisions,
            "rules" => Section::Rules,
            "documents" => Section::Documents,
            "mode" => Section::Mode,
            _ => Section::Overview,
        };
    }

    fn go(&mut self, section: Section, cx: &mut Context<Self>) {
        if self.section == Section::KnowledgeReview && section != self.section {
            self.leave_review(cx);
        }
        self.section = section;
        self.last_in_group.insert(section.group().id(), section);
        self.scroll.set_offset(gpui::point(px(0.0), px(0.0)));
        cx.notify();
    }
    /// Shell lifecycle hook for a cached destination that is no longer visible.
    pub fn leave_review(&mut self, cx: &mut Context<Self>) {
        self.review.update(cx, |review, cx| review.leave(cx));
    }
    /// Demo composition root may preload a synthetic report without provider calls.
    pub fn preload_review(
        &mut self,
        report: application::knowledge_review::ReviewReport,
        cx: &mut Context<Self>,
    ) {
        self.review
            .update(cx, |review, cx| review.preload(report, cx));
    }

    fn focus_for(&mut self, id: &'static str, cx: &mut Context<Self>) -> gpui::FocusHandle {
        self.focus
            .entry(id)
            .or_insert_with(|| cx.focus_handle().tab_stop(true))
            .clone()
    }

    /// Mounts the screen; nothing is read until a project is set.
    pub fn new(cx: &mut Context<Self>, services: ContextServices<S>) -> Self {
        let field = |cx: &mut Context<Self>, placeholder: &'static str| {
            cx.new(|cx| {
                let mut field = SearchField::new(cx);
                field.stretch();
                field.set_context(placeholder, cx);
                field
            })
        };
        let budget = field(cx, t::budget_placeholder());
        let statement = field(cx, t::rule_placeholder());
        let task = field(cx, t::task_placeholder());
        let subscriptions = [&budget, &statement, &task]
            .into_iter()
            .map(|field| cx.subscribe(field, |_, _, _: &SearchChanged, cx| cx.notify()))
            .collect::<Vec<_>>();
        let review = cx.new(|cx| {
            super::knowledge_review::KnowledgeReviewScreen::new(services.reviewer.clone(), cx)
        });
        let mut subscriptions = subscriptions;
        subscriptions.push(cx.subscribe(&review, |_, _, event: &OpenDecision, cx| {
            cx.emit(OpenDecision(event.0.clone()))
        }));
        Self {
            review,
            backend: Some(services),
            project: None,
            generation: 0,
            busy: false,
            snapshot: None,
            mode: ContextMode::Off,
            budget,
            statement,
            task,
            claim_kind: ClaimKind::Convention,
            confirm_retire: None,
            shown_rules: BTreeMap::new(),
            pack: None,
            error: None,
            notice: None,
            scroll: gpui::ScrollHandle::new(),
            scroll_to_end: false,
            section: Section::Overview,
            last_in_group: BTreeMap::new(),
            open_delivery: None,
            focus: std::collections::BTreeMap::new(),
            _subscriptions: subscriptions,
        }
    }

    /// Clears the previous project's state and loads the new one.
    pub fn set_project(&mut self, project: Option<String>, cx: &mut Context<Self>) {
        if self.project == project {
            return;
        }
        self.project = project;
        self.review.update(cx, |review, cx| {
            review.set_project(self.project.clone(), cx)
        });
        self.generation += 1;
        self.snapshot = None;
        self.pack = None;
        self.confirm_retire = None;
        self.error = None;
        self.notice = None;
        self.task
            .update(cx, |field, cx| field.set_context(t::task_placeholder(), cx));
        cx.notify();
    }

    /// Reloads the snapshot of the current project.
    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        if self.section == Section::KnowledgeReview {
            return;
        }
        let Some(project) = self.project.clone() else {
            return;
        };
        self.run(cx, move |backend| {
            Outcome::Loaded(load_snapshot(backend, &project).map(Box::new))
        });
    }

    fn run(
        &mut self,
        cx: &mut Context<Self>,
        operation: impl FnOnce(&ContextServices<S>) -> Outcome + Send + 'static,
    ) {
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
            Outcome::Loaded(Ok(snapshot)) => {
                self.mode = snapshot.settings.mode;
                let budget = snapshot
                    .settings
                    .budget_tokens
                    .map(|budget| budget.to_string())
                    .unwrap_or_default();
                self.budget
                    .update(cx, |field, cx| field.set_value(&budget, cx));
                self.snapshot = Some(*snapshot);
            }
            Outcome::SettingsSaved(Ok(settings)) => {
                self.mode = settings.mode;
                if let Some(snapshot) = &mut self.snapshot {
                    snapshot.settings = settings;
                }
                self.show_notice(t::mode_saved().into(), cx);
            }
            Outcome::Claims(Ok(claims), message) => {
                if let Some(snapshot) = &mut self.snapshot {
                    snapshot.claims = claims;
                }
                self.confirm_retire = None;
                self.show_notice(message.into(), cx);
            }
            Outcome::Pack(Ok(pack)) => self.pack = Some(*pack),
            Outcome::Exported(Ok(Some(path))) => {
                self.show_notice(t::preview_exported(&path.display().to_string()), cx)
            }
            Outcome::Exported(Ok(None)) => {}
            Outcome::Documents(Ok((index, documents))) => {
                if let Some(snapshot) = &mut self.snapshot {
                    snapshot.documents = documents;
                }
                let changed = index.changed + index.removed;
                self.show_notice(
                    match (index.total, changed) {
                        (0, _) => t::no_documents_found().into(),
                        (total, 0) => t::documents_read_unchanged(total),
                        (total, changed) => t::documents_read_changed(total, changed),
                    },
                    cx,
                );
            }
            Outcome::Imported(Ok((imported, documents))) => {
                if let Some(snapshot) = &mut self.snapshot {
                    snapshot.documents = documents;
                }
                self.show_notice(t::document_imported(&imported.document.path), cx);
            }
            Outcome::Loaded(Err(error))
            | Outcome::SettingsSaved(Err(error))
            | Outcome::Claims(Err(error), _)
            | Outcome::Pack(Err(error))
            | Outcome::Exported(Err(error))
            | Outcome::Documents(Err(error))
            | Outcome::Imported(Err(error)) => self.error = Some(error),
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

    /// The typed budget: `Ok(None)` when empty, an error when out of range.
    fn budget_value(&self, cx: &Context<Self>) -> Result<Option<usize>, String> {
        let text = self.budget.read(cx).value().trim().to_owned();
        if text.is_empty() {
            return Ok(None);
        }
        text.parse::<usize>()
            .ok()
            .filter(|budget| (MIN_BUDGET_TOKENS..=MAX_BUDGET_TOKENS).contains(budget))
            .map(Some)
            .ok_or_else(|| t::budget_out_of_range(MIN_BUDGET_TOKENS, MAX_BUDGET_TOKENS))
    }

    fn settings_edited(&self, cx: &Context<Self>) -> bool {
        let Some(snapshot) = &self.snapshot else {
            return false;
        };
        self.mode != snapshot.settings.mode
            || self.budget_value(cx).ok() != Some(snapshot.settings.budget_tokens)
    }

    fn save_settings(&mut self, cx: &mut Context<Self>) {
        let (Some(project), Ok(budget)) = (self.project.clone(), self.budget_value(cx)) else {
            return;
        };
        let mode = self.mode;
        self.run(cx, move |backend| {
            Outcome::SettingsSaved(backend.settings.set(&project, mode, budget).map_err(|error| {
                tracing::error!(error = %error, operation = "context_settings", "save failed");
                t::save_mode_failed().to_owned()
            }))
        });
    }

    fn add_claim(&mut self, cx: &mut Context<Self>) {
        let Some(project) = self.project.clone() else {
            return;
        };
        let statement = self.statement.read(cx).value().trim().to_owned();
        if statement.is_empty() || self.busy {
            return;
        }
        let kind = self.claim_kind;
        self.statement
            .update(cx, |field, cx| field.set_context(t::rule_placeholder(), cx));
        self.run(cx, move |backend| {
            let created = backend.claims.create(NewClaim {
                source_version: None,
                project_id: project.clone(),
                kind,
                statement,
                valid_from: None,
                valid_until: None,
                source_decision_id: None,
                qualifiers: Vec::new(),
            });
            Outcome::Claims(
                created
                    .map_err(|error| claim_failure(error.code()))
                    .and_then(|_| list_claims(backend, &project)),
                t::rule_added(),
            )
        });
    }

    fn retire_claim(&mut self, claim_id: String, cx: &mut Context<Self>) {
        let Some(project) = self.project.clone() else {
            return;
        };
        self.run(cx, move |backend| {
            Outcome::Claims(
                backend
                    .claims
                    .retire(&claim_id, None)
                    .map_err(|error| claim_failure(error.code()))
                    .and_then(|_| list_claims(backend, &project)),
                t::rule_ended(),
            )
        });
    }

    fn build_pack(&mut self, cx: &mut Context<Self>) {
        let Some(project) = self.project.clone() else {
            return;
        };
        let task = self.task.read(cx).value().trim().to_owned();
        if task.is_empty() || self.busy {
            return;
        }
        self.run(cx, move |backend| {
            Outcome::Pack(
                backend
                    .packs
                    .build_pack(ContextRequest {
                        project_id: project,
                        task,
                        as_of: None,
                        budget_chars: None,
                        files: Vec::new(),
                    })
                    .map(Box::new)
                    .map_err(|error| {
                        tracing::error!(error = %error, operation = "context_pack", "build failed");
                        t::pack_failed().to_owned()
                    }),
            )
        });
    }

    fn export_pack(&mut self, format: ExportFormat, cx: &mut Context<Self>) {
        let Some(pack) = self.pack.clone() else {
            return;
        };
        self.run(cx, move |_| {
            let document = match preview_pack(&pack, format) {
                Ok(document) => document,
                Err(_) => return Outcome::Exported(Err(t::export_render_failed().into())),
            };
            let extension = match format {
                ExportFormat::Markdown => "md",
                ExportFormat::Json => "json",
            };
            let Some(path) = rfd::FileDialog::new()
                .set_title(t::export_dialog_title())
                .add_filter(t::export_filter(), &[extension])
                .set_file_name(format!("{}.{extension}", t::export_file_stem()))
                .save_file()
            else {
                return Outcome::Exported(Ok(None));
            };
            Outcome::Exported(
                write_pack(&document, &path, true)
                    .map(|_| Some(path))
                    .map_err(|_| t::export_save_failed().into()),
            )
        });
    }

    fn render_mode(&self, theme: &Theme, cx: &mut Context<Self>) -> Div {
        let colors = theme.colors;
        let options = [
            (
                ContextMode::Off,
                t::mode_off(),
                t::mode_off_body(),
                IconName::Circle,
            ),
            (
                ContextMode::Shadow,
                t::mode_measure(),
                t::mode_measure_body(),
                IconName::Eye,
            ),
            (
                ContextMode::Inject,
                t::mode_active(),
                t::mode_active_body(),
                IconName::CheckCircle,
            ),
        ];
        let edited = self.settings_edited(cx);
        let budget_error = self.budget_value(cx).err();
        let saved_at = self
            .snapshot
            .as_ref()
            .and_then(|snapshot| snapshot.settings.updated_at.clone());
        div()
            .flex()
            .flex_col()
            .gap(px(SpacingScale::S3))
            .child(
                radio_list(theme, "context-mode")
                    .aria_label(t::mode_aria())
                    .children(options.into_iter().enumerate().map(
                        |(index, (mode, title, body, glyph))| {
                            let selected = self.mode == mode;
                            radio_row(
                                theme,
                                ("context-mode-option", mode as usize),
                                selected,
                                index > 0,
                                glyph,
                                title,
                                body,
                            )
                            .when(!selected, |row| {
                                row.hover(move |style| style.bg(colors.glass_fill_medium()))
                                    .active(move |style| style.bg(colors.glass_fill_strong()))
                            })
                            .on_click(cx.listener(
                                move |this, _, _, cx| {
                                    this.mode = mode;
                                    cx.notify();
                                },
                            ))
                        },
                    )),
            )
            .child(
                div()
                    .flex()
                    .items_end()
                    .gap(px(SpacingScale::S3))
                    .child(
                        div()
                            .w(px(240.0))
                            .flex()
                            .flex_col()
                            .gap(px(SpacingScale::S2))
                            .child(
                                text_style(div(), TypeScale::LABEL)
                                    .text_color(colors.text_secondary())
                                    .child(t::tokens_per_block()),
                            )
                            .child(self.budget.clone()),
                    )
                    .child(
                        text_style(div(), TypeScale::META)
                            .flex_1()
                            .pb(px(SpacingScale::S2))
                            .text_color(if budget_error.is_some() {
                                colors.status_danger()
                            } else {
                                colors.text_muted()
                            })
                            .child(budget_error.unwrap_or_else(|| match saved_at {
                                Some(at) if !edited => t::saved_on(&short_date(&at)),
                                _ => t::budget_default(DEFAULT_BUDGET_TOKENS),
                            })),
                    )
                    .when(edited, |row| {
                        row.child(
                            action_button(
                                theme,
                                "context-mode-save",
                                ButtonKind::Primary,
                                !self.busy && self.budget_value(cx).is_ok(),
                            )
                            .aria_label(t::save_mode_aria())
                            .on_click(cx.listener(|this, _, _, cx| this.save_settings(cx)))
                            .child(t::save()),
                        )
                    }),
            )
    }

    fn render_in_force(
        &self,
        theme: &Theme,
        decisions: &[DecisionSummary],
        cx: &mut Context<Self>,
    ) -> Div {
        let colors = theme.colors;
        div()
            .flex()
            .flex_col()
            .gap(px(SpacingScale::S2))
            .child(if decisions.is_empty() {
                text_style(div(), TypeScale::BODY_SMALL)
                    .text_color(colors.text_secondary())
                    .child(t::no_decisions())
                    .into_any_element()
            } else {
                div()
                    .id("context-in-force")
                    .flex()
                    .flex_col()
                    .rounded(RadiusScale.surface())
                    .border_1()
                    .border_color(colors.glass_border_card())
                    .overflow_hidden()
                    .role(Role::List)
                    .children(decisions.iter().enumerate().map(|(index, decision)| {
                        let id = decision.decision_id.clone();
                        div()
                            .id(("context-decision", index))
                            .flex()
                            .items_center()
                            .gap(px(SpacingScale::S3))
                            .px(px(SpacingScale::S4))
                            .py(px(SpacingScale::S3))
                            .when(index > 0, |row| {
                                row.border_t_1().border_color(colors.hairline_divider())
                            })
                            .cursor_pointer()
                            .hover(move |style| style.bg(colors.glass_fill_medium()))
                            .active(move |style| style.bg(colors.glass_fill_strong()))
                            .role(Role::Button)
                            .aria_label(t::open_decision_aria(&decision.question))
                            .on_click(
                                cx.listener(move |_, _, _, cx| cx.emit(OpenDecision(id.clone()))),
                            )
                            .child(icon(IconName::Decision, 14.0, colors.text_muted()))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w(px(0.0))
                                    .flex()
                                    .flex_col()
                                    .gap(px(2.0))
                                    .child(
                                        text_style(div(), TypeScale::ROW_TITLE)
                                            .truncate()
                                            .child(decision.question.clone()),
                                    )
                                    .child(
                                        text_style(div(), TypeScale::BODY_SMALL)
                                            .truncate()
                                            .text_color(colors.text_muted())
                                            .child(decision.choice.clone()),
                                    ),
                            )
                            .child(
                                text_style(div(), TypeScale::META)
                                    .flex_none()
                                    .text_color(colors.text_muted())
                                    .child(format!(
                                        "v{} · {}",
                                        decision.version,
                                        short_date(&decision.confirmed_at)
                                    )),
                            )
                            .child(icon(IconName::ChevronRight, 14.0, colors.text_muted()))
                    }))
                    .into_any_element()
            })
    }

    fn reindex(&mut self, cx: &mut Context<Self>) {
        let Some(project) = self.project.clone() else {
            return;
        };
        self.run(cx, move |backend| {
            Outcome::Documents(
                backend
                    .documents
                    .index(&project)
                    .map_err(document_failure)
                    .and_then(|index| {
                        list_documents(backend, &project).map(|documents| (index, documents))
                    }),
            )
        });
    }

    /// Opens the system file picker for one document; cancelling changes nothing.
    fn pick_document(&mut self, cx: &mut Context<Self>) {
        let receiver = cx.prompt_for_paths(gpui::PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some(t::import_doc_prompt().into()),
        });
        cx.spawn(async move |this, cx| match receiver.await {
            Ok(Ok(Some(paths))) => {
                if let Some(path) = paths.into_iter().next() {
                    let _ = this.update(cx, |screen, cx| screen.import_document(path, cx));
                }
            }
            Ok(Ok(None)) => {}
            error => {
                tracing::error!(
                    ?error,
                    operation = "document_picker",
                    "could not pick a file"
                );
                let _ = this.update(cx, |screen, cx| {
                    screen.error = Some(t::import_unavailable().into());
                    cx.notify();
                });
            }
        })
        .detach();
    }

    fn import_document(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        let Some(project) = self.project.clone() else {
            return;
        };
        self.run(cx, move |backend| {
            Outcome::Imported(
                backend
                    .documents
                    .import_file(application::repo_identity::shared(), &project, &path)
                    .map_err(document_failure)
                    .and_then(|imported| {
                        list_documents(backend, &project).map(|documents| (imported, documents))
                    }),
            )
        });
    }

    /// The project's own documentation, read from its folder.
    fn render_documents(
        &self,
        theme: &Theme,
        documents: &[ProjectDocument],
        cx: &mut Context<Self>,
    ) -> Div {
        let colors = theme.colors;
        let reread = action_button(theme, "context-docs-reread", ButtonKind::Ghost, !self.busy)
            .aria_label(t::reread_docs_aria())
            .on_click(cx.listener(|this, _, _, cx| this.reindex(cx)))
            .child(icon(IconName::Rotate, 13.0, colors.text_secondary()))
            .child(t::reread());
        let import = action_button(
            theme,
            "context-docs-import",
            ButtonKind::Secondary,
            !self.busy,
        )
        .aria_label(t::import_doc_aria())
        .on_click(cx.listener(|this, _, _, cx| this.pick_document(cx)))
        .child(icon(
            IconName::Plus,
            13.0,
            button_foreground(theme, ButtonKind::Secondary, !self.busy),
        ))
        .child(t::import_doc());
        let header = div()
            .flex()
            .items_center()
            .gap(px(SpacingScale::S2))
            .child(
                text_style(div(), TypeScale::META)
                    .flex_1()
                    .text_color(colors.text_muted())
                    .child(if documents.is_empty() {
                        t::no_documents_read().to_owned()
                    } else {
                        t::documents_read(documents.len())
                    }),
            )
            .child(import)
            .child(reread);
        if documents.is_empty() {
            return div()
                .flex()
                .flex_col()
                .gap(px(SpacingScale::S3))
                .child(header)
                .child(
                    text_style(div(), TypeScale::BODY_SMALL)
                        .text_color(colors.text_secondary())
                        .child(t::documents_empty()),
                );
        }
        let groups = DocumentKind::ALL.into_iter().filter_map(|kind| {
            let items: Vec<&ProjectDocument> = documents
                .iter()
                .filter(|document| document.kind == kind)
                .collect();
            (!items.is_empty()).then_some((kind, items))
        });
        let list = div()
            .flex()
            .flex_col()
            .gap(px(SpacingScale::S4))
            .children(groups.map(|(kind, items)| {
                let more = items.len().saturating_sub(DOCUMENTS_SHOWN);
                div()
                    .flex()
                    .flex_col()
                    .gap(px(SpacingScale::S1))
                    .child(
                        section_header(theme, document_kind_plural(kind))
                            .child(count_chip(theme, items.len().to_string())),
                    )
                    .children(items.into_iter().take(DOCUMENTS_SHOWN).map(|document| {
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(2.0))
                            .py(px(SpacingScale::S2))
                            .px(px(SpacingScale::S3))
                            .child(
                                text_style(div(), TypeScale::ROW_TITLE)
                                    .text_color(colors.text_primary())
                                    .child(document.title.clone()),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(SpacingScale::S2))
                                    .child(
                                        text_style(div(), TypeScale::META)
                                            .font_family(Theme::font_mono())
                                            .text_color(colors.text_muted())
                                            .child(document.path.clone()),
                                    )
                                    .when(!document.headings.is_empty(), |row| {
                                        row.child(
                                            text_style(div(), TypeScale::META)
                                                .text_color(colors.text_muted())
                                                .child(t::document_sections(
                                                    document.headings.len(),
                                                )),
                                        )
                                    }),
                            )
                    }))
                    .when(more > 0, |group| {
                        group.child(
                            text_style(div(), TypeScale::META)
                                .pl(px(SpacingScale::S6 - 2.0))
                                .text_color(colors.text_muted())
                                .child(t::more_documents(more)),
                        )
                    })
            }));
        div()
            .flex()
            .flex_col()
            .gap(px(SpacingScale::S3))
            .child(header)
            .child(list)
    }

    fn render_rules(&self, theme: &Theme, claims: &[ClaimRecord], cx: &mut Context<Self>) -> Div {
        let colors = theme.colors;
        let typed = !self.statement.read(cx).value().trim().is_empty();
        let groups = ClaimKind::ALL.into_iter().filter_map(|kind| {
            let items: Vec<&ClaimRecord> =
                claims.iter().filter(|claim| claim.kind == kind).collect();
            (!items.is_empty()).then_some((kind, items))
        });
        let list: AnyElement = if claims.is_empty() {
            text_style(div(), TypeScale::BODY_SMALL)
                .text_color(colors.text_secondary())
                .child(t::rules_empty())
                .into_any_element()
        } else {
            div()
                .flex()
                .flex_col()
                .gap(px(SpacingScale::S4))
                .children(groups.map(|(kind, items)| {
                    let total = items.len();
                    let shown = self
                        .shown_rules
                        .get(&(kind as usize))
                        .copied()
                        .unwrap_or(RULES_PAGE);
                    let hidden = total.saturating_sub(shown);
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(SpacingScale::S1))
                        .child(
                            section_header(theme, kind_plural(kind))
                                .child(count_chip(theme, total.to_string())),
                        )
                        .children(
                            items
                                .into_iter()
                                .take(shown)
                                .map(|claim| self.claim_row(theme, claim, cx)),
                        )
                        .when(hidden > 0, |group| {
                            group.child(
                                div().flex().child(
                                    action_button(
                                        theme,
                                        ("context-rules-more", kind as usize),
                                        ButtonKind::Ghost,
                                        true,
                                    )
                                    .aria_label(t::show_more_rules_aria(hidden))
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        *this
                                            .shown_rules
                                            .entry(kind as usize)
                                            .or_insert(RULES_PAGE) += 2 * RULES_PAGE;
                                        cx.notify();
                                    }))
                                    .child(t::show_more(hidden)),
                                ),
                            )
                        })
                }))
                .into_any_element()
        };
        div()
            .flex()
            .flex_col()
            .gap(px(SpacingScale::S3))
            .child(list)
            .child(
                div()
                    .mt(px(SpacingScale::S2))
                    .p(px(SpacingScale::S3))
                    .flex()
                    .flex_col()
                    .gap(px(SpacingScale::S3))
                    .rounded(RadiusScale.surface())
                    .border_1()
                    .border_color(colors.glass_border_card())
                    .bg(colors.glass_fill_card())
                    .child(
                        div()
                            .id("context-claim-kind")
                            .flex()
                            .flex_wrap()
                            .gap(px(SpacingScale::S1))
                            .role(Role::RadioGroup)
                            .aria_label(t::rule_kind_aria())
                            .children(ClaimKind::ALL.into_iter().map(|kind| {
                                let selected = self.claim_kind == kind;
                                action_button(
                                    theme,
                                    ("context-claim-kind-option", kind as usize),
                                    ButtonKind::Ghost,
                                    true,
                                )
                                .when(selected, |chip| {
                                    chip.bg(colors.selection())
                                        .text_color(colors.text_primary())
                                })
                                .role(Role::RadioButton)
                                .aria_toggled(if selected {
                                    Toggled::True
                                } else {
                                    Toggled::False
                                })
                                .aria_label(kind_label(kind))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.claim_kind = kind;
                                    cx.notify();
                                }))
                                .child(icon(
                                    kind_icon(kind),
                                    14.0,
                                    if selected {
                                        colors.accent_hover()
                                    } else {
                                        button_foreground(theme, ButtonKind::Ghost, true)
                                    },
                                ))
                                .child(kind_label(kind))
                            })),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(SpacingScale::S2))
                            .child(div().flex_1().child(self.statement.clone()))
                            .child(
                                action_button(
                                    theme,
                                    "context-claim-add",
                                    ButtonKind::Secondary,
                                    typed && !self.busy,
                                )
                                .aria_label(t::add_rule_aria())
                                .on_click(cx.listener(|this, _, _, cx| this.add_claim(cx)))
                                .child(icon(
                                    IconName::Plus,
                                    14.0,
                                    button_foreground(theme, ButtonKind::Secondary, typed),
                                ))
                                .child(t::add()),
                            ),
                    ),
            )
    }

    fn claim_row(&self, theme: &Theme, claim: &ClaimRecord, cx: &mut Context<Self>) -> AnyElement {
        let colors = theme.colors;
        let confirming = self.confirm_retire.as_deref() == Some(claim.claim_id.as_str());
        let review = self
            .snapshot
            .as_ref()
            .is_some_and(|snapshot| snapshot.review.contains(&claim.claim_id));
        let id = claim.claim_id.clone();
        let qualifiers = application::qualifiers::decode(&claim.qualifiers);
        let scope = serde_json::from_str::<Vec<String>>(&claim.inherited_scope);
        let readable = qualifiers.is_ok() && scope.is_ok();
        let qualification = match (&qualifiers, &scope) {
            (Ok(items), Ok(scope)) => super::review_editor::qualifier_reading(theme, items)
                .child(if scope.is_empty() {
                    t::inherited_scope_missing().into()
                } else {
                    t::inherited_scope(&scope.join("; "))
                })
                .into_any_element(),
            _ => error_banner(theme, t::qualifiers_unreadable())
                .id(gpui::ElementId::Name(
                    format!("claim-qualification-error-{id}").into(),
                ))
                .role(Role::Alert)
                .child(
                    action_button(
                        theme,
                        gpui::ElementId::Name(format!("claim-qualification-retry-{id}").into()),
                        ButtonKind::Ghost,
                        !self.busy,
                    )
                    .aria_label(t::retry_qualifiers_aria())
                    .on_click(cx.listener(|this, _, _, cx| this.refresh(cx)))
                    .child(t::try_again_long()),
                )
                .into_any_element(),
        };
        let row = div()
            .flex()
            .items_center()
            .gap(px(SpacingScale::S3))
            .px(px(SpacingScale::S3))
            .py(px(SpacingScale::S2))
            .rounded(theme.radius.control())
            .when(confirming, |row| row.bg(tint(colors.status_danger(), 0.07)))
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .flex()
                    .flex_col()
                    .gap(px(2.0))
                    .child(text_style(div(), TypeScale::BODY).child(claim.statement.clone()))
                    .child(qualification)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(SpacingScale::S2))
                            .when(review, |line| {
                                line.child(status_pill(
                                    theme,
                                    colors.status_warning(),
                                    t::review_pill(),
                                ))
                            })
                            .child(
                                text_style(div(), TypeScale::META)
                                    .text_color(colors.text_muted())
                                    .child(if confirming {
                                        t::retire_confirm().to_owned()
                                    } else if review {
                                        t::source_superseded().to_owned()
                                    } else {
                                        t::valid_since(&calendar_date(&claim.valid_from))
                                    }),
                            ),
                    ),
            );
        if confirming {
            let retire_id = id.clone();
            row.child(
                action_button(theme, "context-claim-cancel", ButtonKind::Ghost, true)
                    .aria_label(t::cancel())
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.confirm_retire = None;
                        cx.notify();
                    }))
                    .child(t::cancel()),
            )
            .child(
                action_button(
                    theme,
                    "context-claim-retire",
                    ButtonKind::Secondary,
                    !self.busy && readable,
                )
                .aria_label(t::retire_rule_aria())
                .on_click(cx.listener(move |this, _, _, cx| {
                    if readable && !this.busy {
                        this.retire_claim(retire_id.clone(), cx);
                    }
                }))
                .child(t::retire()),
            )
            .into_any_element()
        } else {
            row.child(
                action_button(
                    theme,
                    gpui::ElementId::Name(format!("context-claim-{id}").into()),
                    ButtonKind::Ghost,
                    !self.busy && readable,
                )
                .aria_label(t::retire_rule_aria())
                .on_click(cx.listener(move |this, _, _, cx| {
                    if !readable || this.busy {
                        return;
                    }
                    this.confirm_retire = Some(id.clone());
                    cx.notify();
                }))
                .child(t::retire()),
            )
            .into_any_element()
        }
    }

    fn render_pack(&self, theme: &Theme, cx: &mut Context<Self>) -> Div {
        let colors = theme.colors;
        let typed = !self.task.read(cx).value().trim().is_empty();
        let result = self.pack.as_ref().map(|pack| {
            let decisions = pack.decisions.len();
            let claims = pack.claims.len();
            let fill = (pack.used_chars as f32 / pack.budget_chars.max(1) as f32).clamp(0.0, 1.0);
            div()
                .flex()
                .flex_col()
                .gap(px(SpacingScale::S3))
                .p(px(SpacingScale::S4))
                .rounded(RadiusScale.surface())
                .border_1()
                .border_color(colors.glass_border_card())
                .bg(colors.glass_fill_card())
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(SpacingScale::S3))
                        .child(
                            text_style(div(), TypeScale::ROW_TITLE)
                                .flex_1()
                                .child(format!(
                                    "{} · {}{}",
                                    t::pack_decisions(decisions),
                                    t::pack_rules(claims),
                                    if pack.omitted > 0 {
                                        format!(" · {}", t::pack_omitted(pack.omitted))
                                    } else {
                                        String::new()
                                    }
                                )),
                        )
                        .child(
                            text_style(div(), TypeScale::META)
                                .text_color(colors.text_muted())
                                .child(t::chars_of(
                                    &thousands(pack.used_chars),
                                    &thousands(pack.budget_chars),
                                )),
                        ),
                )
                .child(
                    div()
                        .h(px(4.0))
                        .w_full()
                        .rounded_full()
                        .bg(colors.surface())
                        .child(
                            div()
                                .h_full()
                                .w(gpui::relative(fill))
                                .rounded_full()
                                .bg(colors.accent_default()),
                        ),
                )
                .when(decisions + claims == 0, |card| {
                    card.child(
                        text_style(div(), TypeScale::BODY_SMALL)
                            .text_color(colors.text_secondary())
                            .child(t::pack_no_match()),
                    )
                })
                .children(pack.decisions.iter().map(|decision| {
                    div()
                        .flex()
                        .gap(px(SpacingScale::S3))
                        .child(div().mt(px(3.0)).child(icon(
                            IconName::Decision,
                            14.0,
                            colors.accent_hover(),
                        )))
                        .child(
                            div()
                                .flex_1()
                                .min_w(px(0.0))
                                .flex()
                                .flex_col()
                                .child(
                                    text_style(div(), TypeScale::ROW_TITLE)
                                        .child(decision.question.clone()),
                                )
                                .child(
                                    text_style(div(), TypeScale::BODY_SMALL)
                                        .text_color(colors.text_secondary())
                                        .child(decision.choice.clone())
                                        .child(super::review_editor::qualifier_reading(
                                            theme,
                                            &decision.qualifiers,
                                        ))
                                        .child(if decision.scope.is_empty() {
                                            t::scope_missing().into()
                                        } else {
                                            t::scope(&decision.scope.join("; "))
                                        }),
                                ),
                        )
                        .child(
                            text_style(div(), TypeScale::META)
                                .text_color(colors.text_muted())
                                .child(format!("v{}", decision.version)),
                        )
                }))
                .children(pack.claims.iter().map(|claim| {
                    let kind = ClaimKind::parse(&claim.kind);
                    div()
                        .flex()
                        .gap(px(SpacingScale::S3))
                        .child(div().mt(px(3.0)).child(icon(
                            kind.map(kind_icon).unwrap_or(IconName::Shield),
                            14.0,
                            colors.text_secondary(),
                        )))
                        .child(
                            text_style(div(), TypeScale::BODY_SMALL)
                                .flex_1()
                                .child(claim.statement.clone())
                                .child(super::review_editor::qualifier_reading(
                                    theme,
                                    &claim.qualifiers,
                                ))
                                .child(if claim.inherited_scope.is_empty() {
                                    t::inherited_scope_missing().into()
                                } else {
                                    t::inherited_scope(&claim.inherited_scope.join("; "))
                                }),
                        )
                        .child(
                            text_style(div(), TypeScale::META)
                                .text_color(colors.text_muted())
                                .child({
                                    let label = kind.map(kind_label).unwrap_or(t::rule_word());
                                    if claim.matched {
                                        t::kind_matched(label)
                                    } else {
                                        label.to_owned()
                                    }
                                }),
                        )
                }))
                .child(
                    div()
                        .flex()
                        .justify_end()
                        .gap(px(SpacingScale::S2))
                        .pt(px(SpacingScale::S2))
                        .border_t_1()
                        .border_color(colors.hairline_divider())
                        .child(
                            action_button(theme, "context-pack-md", ButtonKind::Ghost, !self.busy)
                                .aria_label(t::export_markdown_aria())
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.export_pack(ExportFormat::Markdown, cx)
                                }))
                                .child(icon(
                                    IconName::Export,
                                    14.0,
                                    button_foreground(theme, ButtonKind::Ghost, true),
                                ))
                                .child("Markdown"),
                        )
                        .child(
                            action_button(
                                theme,
                                "context-pack-json",
                                ButtonKind::Ghost,
                                !self.busy,
                            )
                            .aria_label(t::export_json_aria())
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.export_pack(ExportFormat::Json, cx)
                            }))
                            .child(icon(
                                IconName::Export,
                                14.0,
                                button_foreground(theme, ButtonKind::Ghost, true),
                            ))
                            .child("JSON"),
                        ),
                )
        });
        div()
            .flex()
            .flex_col()
            .gap(px(SpacingScale::S3))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(SpacingScale::S2))
                    .child(div().flex_1().child(self.task.clone()))
                    .child(
                        action_button(
                            theme,
                            "context-pack-build",
                            ButtonKind::Primary,
                            typed && !self.busy,
                        )
                        .aria_label(t::build_preview())
                        .on_click(cx.listener(|this, _, _, cx| this.build_pack(cx)))
                        .child(t::build_preview()),
                    ),
            )
            .children(result)
    }
}

impl<S: ContextStores> ContextScreen<S> {
    /// The index of the destination: the agent, its sources and the
    /// delivery setting, with the saved mode at the foot.
    fn render_rail(&mut self, theme: &Theme, cx: &mut Context<Self>) -> Div {
        let colors = theme.colors;
        let counts = self.snapshot.as_ref().map(|snapshot| {
            (
                snapshot.decisions.len(),
                snapshot.claims.len(),
                snapshot.documents.len(),
                snapshot.deliveries.len(),
            )
        });
        let mut list = div()
            .flex()
            .flex_col()
            .gap(px(2.0))
            .px(px(SpacingScale::S2))
            .pt(px(SpacingScale::S4));
        for group in Group::ALL {
            // Only the deliveries carry a count: the others are lists whose
            // size says nothing about whether something needs attention.
            let badge = counts
                .map(|(_, _, _, deliveries)| deliveries)
                .filter(|count| group == Group::Deliveries && *count > 0)
                .map(|count| count.to_string());
            // A group opens on the page the person last used in it.
            let section = if self.section.group() == group {
                self.section
            } else {
                self.last_in_group
                    .get(group.id())
                    .copied()
                    .unwrap_or(group.members()[0])
            };
            let focus = self.focus_for(group.id(), cx);
            list = list.child(
                index_row(
                    theme,
                    group.id(),
                    self.section.group() == group,
                    group.glyph(),
                    group.label(),
                    badge,
                )
                .track_focus(&focus)
                .on_click(cx.listener(move |this, _, _, cx| this.go(section, cx)))
                .on_key_down(cx.listener(
                    move |this, event: &gpui::KeyDownEvent, _, cx| {
                        if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                            this.go(section, cx);
                            cx.stop_propagation();
                        }
                    },
                )),
            );
        }
        let foot = self.snapshot.as_ref().map(|snapshot| {
            let mode = snapshot.settings.mode;
            let budget = snapshot
                .settings
                .budget_tokens
                .unwrap_or(DEFAULT_BUDGET_TOKENS);
            div()
                .flex_none()
                .flex()
                .items_center()
                .gap(px(SpacingScale::S2))
                .px(px(SpacingScale::S4))
                .py(px(SpacingScale::S2))
                .border_t_1()
                .border_color(colors.hairline_divider())
                .child(
                    div()
                        .size(px(6.0))
                        .rounded_full()
                        .bg(mode_color(theme, mode)),
                )
                .child(
                    text_style(div(), TypeScale::META)
                        .text_color(colors.text_muted())
                        .child(match mode {
                            ContextMode::Off => t::mode_off().to_owned(),
                            mode => t::rail_foot(mode_word(mode), budget),
                        }),
                )
        });
        index_rail(theme)
            .child(
                div()
                    .id("context-rail")
                    .flex_1()
                    .min_h(px(0.0))
                    .overflow_y_scroll()
                    .child(list),
            )
            .children(foot)
    }

    /// The switch between the pages of the open group ("Decisões | Regras |
    /// Documentação | Revisar com IA"); a group of one page has none.
    fn render_switch(&mut self, theme: &Theme, cx: &mut Context<Self>) -> Option<Div> {
        let members = self.section.group().members();
        if members.len() < 2 {
            return None;
        }
        let mut track = crate::ui::patterns::segmented(theme)
            .id("context-switch")
            .role(Role::RadioGroup);
        for &section in members {
            let focus = self.focus_for(section.tab_id(), cx);
            track = track.child(
                crate::ui::patterns::segment_label(
                    theme,
                    section.tab_id(),
                    section.tab(),
                    self.section == section,
                )
                .track_focus(&focus)
                .on_click(cx.listener(move |this, _, _, cx| this.go(section, cx)))
                .on_key_down(cx.listener(
                    move |this, event: &gpui::KeyDownEvent, _, cx| {
                        if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                            this.go(section, cx);
                            cx.stop_propagation();
                        }
                    },
                )),
            );
        }
        Some(
            div()
                .flex_none()
                .flex()
                .items_center()
                .px(px(SpacingScale::S4))
                .py(px(SpacingScale::S2))
                .border_b_1()
                .border_color(theme.colors.hairline_divider())
                .child(track),
        )
    }

    /// The page of the selected section.
    fn render_page(&mut self, theme: &Theme, cx: &mut Context<Self>) -> Div {
        let Some(snapshot) = self.snapshot.as_ref() else {
            return div();
        };
        let decisions = snapshot.decisions.clone();
        let claims = snapshot.claims.clone();
        let documents = snapshot.documents.clone();
        let deliveries = snapshot.deliveries.clone();
        let settings = snapshot.settings.clone();
        let section = self.section;
        let (title, subtitle, body): (&str, &str, Div) = match section {
            Section::KnowledgeReview => unreachable!("independent read-only view"),
            Section::Overview => (
                t::overview_title(),
                t::overview_subtitle(),
                self.render_overview(theme, &settings, &decisions, &claims, &deliveries, cx),
            ),
            Section::Deliveries => (
                t::group_deliveries(),
                t::deliveries_subtitle(),
                self.render_deliveries(theme, &settings, &deliveries, cx),
            ),
            Section::Test => (
                t::tab_test_task(),
                t::test_subtitle(),
                self.render_pack(theme, cx),
            ),
            Section::Decisions => (
                t::decisions_title(),
                t::decisions_subtitle(),
                self.render_in_force(theme, &decisions, cx),
            ),
            Section::Rules => (
                t::tab_rules(),
                t::rules_subtitle(),
                self.render_rules(theme, &claims, cx),
            ),
            Section::Documents => (
                t::tab_documents(),
                t::documents_subtitle(),
                self.render_documents(theme, &documents, cx),
            ),
            Section::Mode => (
                t::tab_delivery_mode(),
                t::mode_subtitle(),
                self.render_mode(theme, cx),
            ),
        };
        div()
            .flex()
            .flex_col()
            .gap(px(SpacingScale::S8))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(SpacingScale::S1))
                    .child(text_style(div(), TypeScale::HEADING_1).child(title))
                    .child(
                        text_style(div(), TypeScale::BODY_SMALL)
                            .text_color(theme.colors.text_muted())
                            .child(subtitle),
                    ),
            )
            .child(body)
    }

    fn render_overview(
        &mut self,
        theme: &Theme,
        settings: &ProjectContextSettings,
        decisions: &[DecisionSummary],
        claims: &[ClaimRecord],
        deliveries: &[Delivery],
        cx: &mut Context<Self>,
    ) -> Div {
        let colors = theme.colors;
        let mode = settings.mode;
        let budget = settings.budget_tokens.unwrap_or(DEFAULT_BUDGET_TOKENS);
        let tint = mode_color(theme, mode);

        // Where things stand, in one sentence: a status line, not a card.
        let change = self.focus_for("context-change-mode", cx);
        let status = div()
            .flex()
            .items_start()
            .gap(px(SpacingScale::S3))
            .child(
                div()
                    .mt(px(7.0))
                    .size(px(8.0))
                    .flex_none()
                    .rounded_full()
                    .bg(tint),
            )
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .flex()
                    .flex_col()
                    .gap(px(2.0))
                    .child(text_style(div(), TypeScale::HEADING_3).child(mode_word(mode)))
                    .child(
                        text_style(div(), TypeScale::BODY_SMALL)
                            .text_color(colors.text_secondary())
                            .child(mode_sentence(mode, budget)),
                    ),
            )
            .child(
                action_button(theme, "context-change-mode", ButtonKind::Secondary, true)
                    .aria_label(t::change_mode_aria())
                    .track_focus(&change)
                    .on_click(cx.listener(|this, _, _, cx| this.go(Section::Mode, cx)))
                    .child(if mode == ContextMode::Off {
                        t::activate()
                    } else {
                        t::change_mode()
                    }),
            );

        // How it gets there: three numbered columns divided by hairlines.
        let stage = |number: &'static str, title: &'static str, lines: Vec<String>, first: bool| {
            div()
                .flex_1()
                .min_w(px(0.0))
                .flex()
                .flex_col()
                .gap(px(SpacingScale::S1))
                .when(!first, |stage| {
                    stage
                        .pl(px(SpacingScale::S5))
                        .border_l_1()
                        .border_color(colors.hairline_divider())
                })
                .child(
                    div()
                        .flex()
                        .items_baseline()
                        .gap(px(SpacingScale::S2))
                        .mb(px(SpacingScale::S1))
                        .child(
                            text_style(div(), TypeScale::META)
                                .font_family(Theme::font_mono())
                                .text_color(colors.text_muted())
                                .child(number),
                        )
                        .child(
                            text_style(div(), TypeScale::LABEL)
                                .text_color(colors.text_primary())
                                .child(title),
                        ),
                )
                .children(lines.into_iter().map(|line| {
                    text_style(div(), TypeScale::BODY_SMALL)
                        .text_color(colors.text_secondary())
                        .child(line)
                }))
        };
        let pipeline = div()
            .flex()
            .items_stretch()
            .gap(px(SpacingScale::S5))
            .child(stage(
                "I",
                t::group_sources(),
                vec![
                    t::decisions_in_force(decisions.len()),
                    t::project_rules(claims.len()),
                ],
                true,
            ))
            .child(stage(
                "II",
                t::stage_selection(),
                vec![
                    t::stage_on_request().to_owned(),
                    t::stage_on_edit().to_owned(),
                ],
                false,
            ))
            .child(stage(
                "III",
                t::stage_delivery(),
                vec![t::stage_budget(budget), t::stage_no_repeat().to_owned()],
                false,
            ));

        // The last seven days, from the audit: flat figures in one band.
        let since = (chrono::Utc::now() - chrono::Duration::days(7))
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string();
        let summary = DeliverySummary::since(deliveries, &since);
        let measured = summary.deliveries - summary.sent;
        let (per_day, sessions_per_day) = daily(deliveries, 7);
        let figure = |value: usize,
                      label: &'static str,
                      detail: String,
                      first: bool,
                      form: Option<gpui::AnyElement>| {
            div()
                .flex_1()
                .min_w(px(0.0))
                .flex()
                .flex_col()
                .gap(px(2.0))
                .py(px(SpacingScale::S4))
                .when(!first, |figure| {
                    figure
                        .pl(px(SpacingScale::S5))
                        .border_l_1()
                        .border_color(colors.hairline_divider())
                })
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(px(SpacingScale::S2))
                        .child(count_up(
                            label,
                            value,
                            text_style(div(), TypeScale::HEADING_1)
                                .text_color(colors.text_primary()),
                        ))
                        .children(form),
                )
                .child(
                    text_style(div(), TypeScale::LABEL)
                        .text_color(colors.text_secondary())
                        .child(label),
                )
                .child(
                    text_style(div(), TypeScale::META)
                        .text_color(colors.text_muted())
                        .child(detail),
                )
        };
        let metrics = div()
            .flex()
            .gap(px(SpacingScale::S5))
            .border_t_1()
            .border_b_1()
            .border_color(colors.hairline_divider())
            .child(figure(
                summary.deliveries,
                t::group_deliveries(),
                t::deliveries_detail(summary.sent, measured),
                true,
                Some(sparkline(&per_day, 56.0, 22.0, colors.accent_default()).into_any_element()),
            ))
            .child(figure(
                summary.sessions,
                t::figure_sessions(),
                t::sessions_detail().to_owned(),
                false,
                Some(
                    sparkline(&sessions_per_day, 56.0, 22.0, colors.status_info())
                        .into_any_element(),
                ),
            ))
            .child(figure(
                summary.average_tokens(),
                t::tokens_per_block(),
                t::tokens_detail(budget),
                false,
                Some(
                    ring(
                        theme,
                        share(summary.average_tokens(), budget),
                        26.0,
                        colors.status_info(),
                    )
                    .into_any_element(),
                ),
            ))
            .child(figure(
                summary.omitted,
                t::figure_omitted(),
                t::omitted_detail().to_owned(),
                false,
                Some(
                    ring(
                        theme,
                        share(summary.omitted, summary.items + summary.omitted),
                        26.0,
                        if summary.omitted > 0 {
                            colors.status_warning()
                        } else {
                            colors.status_success()
                        },
                    )
                    .into_any_element(),
                ),
            ));

        // The latest deliveries.
        let see_all = self.focus_for("context-see-deliveries", cx);
        let recent_header = div()
            .flex()
            .items_center()
            .child(
                div()
                    .flex_1()
                    .child(section_label(theme, t::recent_deliveries())),
            )
            .when(!deliveries.is_empty(), |row| {
                row.child(
                    action_button(theme, "context-see-deliveries", ButtonKind::Ghost, true)
                        .aria_label(t::see_all_aria())
                        .track_focus(&see_all)
                        .on_click(cx.listener(|this, _, _, cx| this.go(Section::Deliveries, cx)))
                        .child(t::see_all())
                        .child(icon(IconName::ChevronRight, 13.0, colors.text_secondary())),
                )
            });
        let recent: AnyElement = if deliveries.is_empty() {
            text_style(div(), TypeScale::BODY_SMALL)
                .text_color(colors.text_secondary())
                .child(empty_deliveries(mode))
                .into_any_element()
        } else {
            let mut list = delivery_list(theme);
            for (index, delivery) in deliveries.iter().take(RECENT_SHOWN).enumerate() {
                let id = delivery.injection_id.clone();
                list = list.child(
                    self.delivery_row(theme, delivery, budget, index > 0, false, cx)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.open_delivery = Some(id.clone());
                            this.go(Section::Deliveries, cx);
                        })),
                );
            }
            list.into_any_element()
        };

        div()
            .flex()
            .flex_col()
            .gap(px(SpacingScale::S8))
            .child(status)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(SpacingScale::S3))
                    .child(section_label(theme, t::how_it_arrives()))
                    .child(pipeline),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(SpacingScale::S3))
                    .child(section_label(theme, t::last_seven_days()))
                    .child(metrics),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(SpacingScale::S3))
                    .child(recent_header)
                    .child(recent),
            )
    }

    /// Every recorded block, newest first, grouped by day; a row opens to
    /// the items the agent read.
    fn render_deliveries(
        &mut self,
        theme: &Theme,
        settings: &ProjectContextSettings,
        deliveries: &[Delivery],
        cx: &mut Context<Self>,
    ) -> Div {
        let colors = theme.colors;
        let budget = settings.budget_tokens.unwrap_or(DEFAULT_BUDGET_TOKENS);
        if deliveries.is_empty() {
            let focus = self.focus_for("context-deliveries-mode", cx);
            return div()
                .flex()
                .flex_col()
                .items_start()
                .gap(px(SpacingScale::S3))
                .child(
                    text_style(div(), TypeScale::BODY_SMALL)
                        .text_color(colors.text_secondary())
                        .child(empty_deliveries(settings.mode)),
                )
                .child(
                    action_button(
                        theme,
                        "context-deliveries-mode",
                        ButtonKind::Secondary,
                        true,
                    )
                    .aria_label(t::choose_mode_aria())
                    .track_focus(&focus)
                    .on_click(cx.listener(|this, _, _, cx| this.go(Section::Mode, cx)))
                    .child(t::choose_mode()),
                );
        }
        let mut days: Vec<(String, Vec<&Delivery>)> = Vec::new();
        for delivery in deliveries {
            let day = day_heading(&delivery.created_at);
            if days.last().is_none_or(|(heading, _)| *heading != day) {
                days.push((day, Vec::new()));
            }
            if let Some((_, rows)) = days.last_mut() {
                rows.push(delivery);
            }
        }
        let mut column = div().flex().flex_col().gap(px(SpacingScale::S6));
        for (day, rows) in days {
            let mut list = delivery_list(theme);
            for (index, delivery) in rows.iter().enumerate() {
                let open = self.open_delivery.as_deref() == Some(delivery.injection_id.as_str());
                let id = delivery.injection_id.clone();
                list = list.child(
                    self.delivery_row(theme, delivery, budget, index > 0, true, cx)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.open_delivery = if this.open_delivery.as_deref() == Some(&id) {
                                None
                            } else {
                                Some(id.clone())
                            };
                            cx.notify();
                        })),
                );
                if open {
                    list = list.child(delivery_items(theme, delivery));
                }
            }
            column = column.child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(SpacingScale::S2))
                    .child(
                        div()
                            .flex()
                            .items_baseline()
                            .gap(px(SpacingScale::S2))
                            .child(
                                text_style(div(), TypeScale::ROW_TITLE)
                                    .text_color(colors.text_secondary())
                                    .child(day),
                            )
                            .child(
                                text_style(div(), TypeScale::META)
                                    .text_color(colors.text_muted())
                                    .child(t::deliveries_count(rows.len())),
                            ),
                    )
                    .child(list),
            );
        }
        column
    }

    /// One delivery: time, sent or measured, size against the budget and the
    /// session. The caller adds the press handler.
    fn delivery_row(
        &mut self,
        theme: &Theme,
        delivery: &Delivery,
        budget: usize,
        divided: bool,
        expandable: bool,
        cx: &mut Context<Self>,
    ) -> gpui::Stateful<Div> {
        let colors = theme.colors;
        let sent = delivery.mode == InjectionMode::Inject;
        let tint = if sent {
            colors.status_success()
        } else {
            colors.status_info()
        };
        let fill = (delivery.tokens as f32 / budget.max(1) as f32).clamp(0.0, 1.0);
        let open = self.open_delivery.as_deref() == Some(delivery.injection_id.as_str());
        let _ = cx;
        let status = if sent {
            t::delivery_sent()
        } else {
            t::delivery_measured()
        };
        div()
            .id(gpui::SharedString::from(format!(
                "context-delivery-{}",
                delivery.injection_id
            )))
            .flex()
            .items_center()
            .gap(px(SpacingScale::S3))
            .px(px(SpacingScale::S4))
            .py(px(SpacingScale::S3))
            .when(divided, |row| {
                row.border_t_1().border_color(colors.hairline_divider())
            })
            .cursor_pointer()
            .hover(move |style| style.bg(colors.glass_fill_medium()))
            .role(Role::Button)
            .aria_label(t::delivery_aria(
                status,
                &clock(&delivery.created_at).unwrap_or_default(),
                delivery.items.len(),
                delivery.tokens,
                &short_ref(&delivery.session_id),
            ))
            .child(
                text_style(div(), TypeScale::META)
                    .w(px(40.0))
                    .flex_none()
                    .font_family(Theme::font_mono())
                    .text_color(colors.text_muted())
                    .child(clock(&delivery.created_at).unwrap_or_default()),
            )
            .child(
                div()
                    .w(px(78.0))
                    .flex_none()
                    .child(status_pill(theme, tint, status)),
            )
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .flex()
                    .flex_col()
                    .gap(px(2.0))
                    .child(
                        text_style(div(), TypeScale::BODY_SMALL)
                            .truncate()
                            .text_color(colors.text_primary())
                            .child(delivery_headline(delivery)),
                    )
                    .child(
                        text_style(div(), TypeScale::META)
                            .text_color(colors.text_muted())
                            .child(format!(
                                "{}{}",
                                t::items_count(delivery.items.len()),
                                if delivery.omitted > 0 {
                                    format!(" · {}", t::pack_omitted(delivery.omitted))
                                } else {
                                    String::new()
                                }
                            )),
                    ),
            )
            .child(
                div()
                    .w(px(96.0))
                    .flex_none()
                    .flex()
                    .flex_col()
                    .gap(px(4.0))
                    .child(
                        text_style(div(), TypeScale::META)
                            .text_color(colors.text_muted())
                            .child(t::tokens_of(delivery.tokens, budget)),
                    )
                    .child(meter(theme, fill, tint)),
            )
            .when(expandable, |row| {
                row.child(icon(
                    if open {
                        IconName::ChevronDown
                    } else {
                        IconName::ChevronRight
                    },
                    13.0,
                    colors.text_muted(),
                ))
            })
    }
}

/// A bordered list that holds delivery rows.
fn delivery_list(theme: &Theme) -> Div {
    div()
        .flex()
        .flex_col()
        .rounded(RadiusScale.surface())
        .border_1()
        .border_color(theme.colors.glass_border_card())
        .overflow_hidden()
}

/// The items one delivery carried, as the agent read them.
fn delivery_items(theme: &Theme, delivery: &Delivery) -> Div {
    let colors = theme.colors;
    div()
        .flex()
        .flex_col()
        .gap(px(SpacingScale::S2))
        .px(px(SpacingScale::S4))
        .pl(px(SpacingScale::S4 + 40.0 + SpacingScale::S3))
        .py(px(SpacingScale::S3))
        .bg(colors.canvas_deep())
        .border_t_1()
        .border_color(colors.hairline_divider())
        .children(delivery.items.iter().map(|item| {
            div()
                .flex()
                .items_start()
                .gap(px(SpacingScale::S2))
                .child(div().mt(px(3.0)).child(icon(
                    match item.kind {
                        ItemKind::Decision => IconName::Decision,
                        ItemKind::Claim => IconName::Shield,
                    },
                    13.0,
                    colors.text_muted(),
                )))
                .child(
                    text_style(div(), TypeScale::BODY_SMALL)
                        .flex_1()
                        .text_color(if item.label.is_empty() {
                            colors.text_muted()
                        } else {
                            colors.text_secondary()
                        })
                        .child(if item.label.is_empty() {
                            t::item_missing().to_owned()
                        } else {
                            item.label.clone()
                        }),
                )
                .child(
                    text_style(div(), TypeScale::META)
                        .text_color(colors.text_muted())
                        .child(match item.kind {
                            ItemKind::Decision => t::item_decision(item.version),
                            ItemKind::Claim => t::item_rule().to_owned(),
                        }),
                )
        }))
        .when(delivery.items.is_empty(), |list| {
            list.child(
                text_style(div(), TypeScale::BODY_SMALL)
                    .text_color(colors.text_muted())
                    .child(t::block_empty()),
            )
        })
        .child(
            text_style(div(), TypeScale::META)
                .text_color(colors.text_muted())
                .child(t::agent_session(&short_ref(&delivery.session_id))),
        )
}

/// The first item, or what the block was.
fn delivery_headline(delivery: &Delivery) -> String {
    let named: Vec<&str> = delivery
        .items
        .iter()
        .map(|item| item.label.as_str())
        .filter(|label| !label.is_empty())
        .collect();
    match named.as_slice() {
        [] => t::empty_block().to_owned(),
        [only] => (*only).to_owned(),
        [first, rest @ ..] => t::first_and_more(first, rest.len()),
    }
}

fn mode_word(mode: ContextMode) -> &'static str {
    match mode {
        ContextMode::Off => t::mode_off(),
        ContextMode::Shadow => t::mode_measuring(),
        ContextMode::Inject => t::mode_active(),
    }
}

fn mode_color(theme: &Theme, mode: ContextMode) -> gpui::Rgba {
    match mode {
        ContextMode::Off => theme.colors.text_muted(),
        ContextMode::Shadow => theme.colors.status_info(),
        ContextMode::Inject => theme.colors.status_success(),
    }
}

fn mode_sentence(mode: ContextMode, budget: usize) -> String {
    match mode {
        ContextMode::Off => t::mode_sentence_off().to_owned(),
        ContextMode::Shadow => t::mode_sentence_shadow(budget),
        ContextMode::Inject => t::mode_sentence_active(budget),
    }
}

fn empty_deliveries(mode: ContextMode) -> &'static str {
    match mode {
        ContextMode::Off => t::deliveries_empty_off(),
        _ => t::deliveries_empty_on(),
    }
}

impl<S: ContextStores> Render for ContextScreen<S> {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let _probe = crate::ui::perf::Probe::start("context");
        let theme = Theme::current(cx);
        let rail = self.render_rail(&theme, cx);
        let switch = self.render_switch(&theme, cx);
        if self.section == Section::KnowledgeReview {
            return div().size_full().relative().flex().child(rail).child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .h_full()
                    .flex()
                    .flex_col()
                    .children(switch)
                    .child(
                        div()
                            .flex_1()
                            .min_h(px(0.0))
                            .flex()
                            .flex_col()
                            .child(self.review.clone()),
                    ),
            );
        }
        if self.snapshot.is_none() && !self.busy && self.error.is_none() && self.project.is_some() {
            self.refresh(cx);
        }
        let body: AnyElement = match self.snapshot.as_ref() {
            None if self.error.is_none() => div()
                .p(px(SpacingScale::S8))
                .child(skeleton_list(&theme, "context-skeleton", 5))
                .into_any_element(),
            None => empty_panel(
                &theme,
                IconName::Layers,
                t::context_word(),
                t::load_failed_title(),
                t::load_failed_body(),
            )
            .into_any_element(),
            Some(_) => {
                let page = self.render_page(&theme, cx);
                if self.scroll_to_end {
                    self.scroll_to_end = false;
                    let scroll = self.scroll.clone();
                    cx.on_next_frame(window, move |_, _, cx| {
                        scroll.scroll_to_bottom();
                        cx.notify();
                    });
                }
                reading_page("context-page", page)
                    .track_scroll(&self.scroll)
                    .into_any_element()
            }
        };
        let retry = self.error.is_some().then(|| {
            action_button(&theme, "context-retry", ButtonKind::Ghost, !self.busy)
                .aria_label(t::retry())
                .on_click(cx.listener(|this, _, _, cx| this.refresh(cx)))
                .child(t::retry())
        });
        div()
            .size_full()
            .relative()
            .flex()
            .child(rail)
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .h_full()
                    .flex()
                    .flex_col()
                    .children(self.error.clone().map(|error| {
                        error_banner(&theme, &error)
                            .id("context-error")
                            .role(Role::Alert)
                            .children(retry)
                    }))
                    .children(switch)
                    .child(div().flex_1().min_h(px(0.0)).flex().flex_col().child(body)),
            )
            .children(
                self.notice
                    .clone()
                    .map(|notice| toast(&theme, &notice, 24.0)),
            )
    }
}

fn load_snapshot<S: ContextStores>(
    backend: &ContextServices<S>,
    project: &str,
) -> Result<Snapshot, String> {
    let settings = backend.settings.get(project).map_err(|error| {
        tracing::error!(error = %error, operation = "context_settings", "read failed");
        t::read_mode_failed().to_owned()
    })?;
    let decisions = backend
        .decisions
        .list(&DecisionFilter {
            project_id: Some(project.to_owned()),
            cursor: None,
            statuses: vec![DecisionStatus::Accepted],
            limit: IN_FORCE_LIMIT,
        })
        .map_err(|error| {
            tracing::error!(error = %error, operation = "context_decisions", "list failed");
            t::read_decisions_failed().to_owned()
        })?
        .decisions;
    let claims = list_claims(backend, project)?;
    // The folder is read on each visit; an unreadable folder keeps the last
    // index instead of failing the page.
    if let Err(error) = backend.documents.index(project) {
        tracing::warn!(error = %error, operation = "documents_index", "indexing skipped");
    }
    let documents = list_documents(backend, project)?;
    let deliveries = backend
        .deliveries
        .recent(project, DELIVERIES_LIMIT)
        .map_err(|error| {
            tracing::error!(error = %error, operation = "deliveries", "list failed");
            t::read_deliveries_failed().to_owned()
        })?;
    let review = backend.derived.to_review(project).map_err(|error| {
        tracing::error!(error = %error, operation = "claims_to_review", "read failed");
        t::read_rules_failed().to_owned()
    })?;
    Ok(Snapshot {
        settings,
        decisions,
        claims,
        documents,
        deliveries,
        review,
    })
}

fn list_documents<S: ContextStores>(
    backend: &ContextServices<S>,
    project: &str,
) -> Result<Vec<ProjectDocument>, String> {
    backend.documents.list(project).map_err(|error| {
        tracing::error!(error = ?error, operation = "documents", "list failed");
        t::read_documents_failed().to_owned()
    })
}

/// Product copy for an indexing failure.
fn document_failure(error: application::documents::DocumentError) -> String {
    use application::documents::DocumentError;
    match error {
        DocumentError::FolderUnavailable => t::folder_unavailable().into(),
        DocumentError::ProjectNotFound => t::project_not_found().into(),
        DocumentError::FileUnavailable => t::import_unavailable().into(),
        DocumentError::OutsideRepository => t::import_outside().into(),
        DocumentError::NotDocumentation => t::import_not_document().into(),
        DocumentError::NotText => t::import_not_text().into(),
        DocumentError::Storage(detail) => {
            tracing::error!(error = %detail, operation = "documents_index", "storage failed");
            t::index_documents_failed().into()
        }
    }
}

fn document_kind_plural(kind: DocumentKind) -> &'static str {
    match kind {
        DocumentKind::Adr => t::kind_adr(),
        DocumentKind::Spec => t::kind_spec(),
        DocumentKind::Readme => t::kind_readme(),
        DocumentKind::Guide => t::kind_guide(),
    }
}

fn list_claims<S: ContextStores>(
    backend: &ContextServices<S>,
    project: &str,
) -> Result<Vec<ClaimRecord>, String> {
    backend.claims.list(project, None).map_err(|error| {
        tracing::error!(error = %error, operation = "claims", "list failed");
        t::read_rules_failed().to_owned()
    })
}

/// Product copy for a claim failure code.
fn claim_failure(code: &str) -> String {
    match code {
        "empty_statement" => t::rule_empty().into(),
        "statement_too_long" => t::rule_too_long().into(),
        "already_ended" | "conflict" => t::rule_changed().into(),
        _ => t::rule_save_failed().into(),
    }
}

fn kind_label(kind: ClaimKind) -> &'static str {
    match kind {
        ClaimKind::Assumption => t::kind_assumption(),
        ClaimKind::Constraint => t::kind_constraint(),
        ClaimKind::Goal => t::kind_goal(),
        ClaimKind::Convention => t::kind_convention(),
    }
}

fn kind_plural(kind: ClaimKind) -> &'static str {
    match kind {
        ClaimKind::Assumption => t::kinds_assumption(),
        ClaimKind::Constraint => t::kinds_constraint(),
        ClaimKind::Goal => t::kinds_goal(),
        ClaimKind::Convention => t::kinds_convention(),
    }
}

fn kind_icon(kind: ClaimKind) -> IconName {
    match kind {
        ClaimKind::Assumption => IconName::CheckCircle,
        ClaimKind::Constraint => IconName::Shield,
        ClaimKind::Goal => IconName::Flag,
        ClaimKind::Convention => IconName::List,
    }
}

/// Deliveries and distinct sessions per day over the last `days` days, oldest
/// first (days with none count 0), for the trend beside the figures.
fn daily(deliveries: &[Delivery], days: usize) -> (Vec<f32>, Vec<f32>) {
    let today = chrono::Utc::now().date_naive();
    let mut counts = vec![0.0_f32; days];
    let mut sessions: Vec<std::collections::BTreeSet<&str>> = vec![Default::default(); days];
    for delivery in deliveries {
        let Some(day) = delivery
            .created_at
            .get(..10)
            .and_then(|day| chrono::NaiveDate::parse_from_str(day, "%Y-%m-%d").ok())
        else {
            continue;
        };
        let ago = (today - day).num_days();
        if (0..days as i64).contains(&ago) {
            let at = days - 1 - ago as usize;
            counts[at] += 1.0;
            sessions[at].insert(delivery.session_id.as_str());
        }
    }
    (
        counts,
        sessions.iter().map(|set| set.len() as f32).collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::{claim_failure, daily, kind_label, kind_plural};
    use application::injection::{Delivery, InjectionMode};

    fn delivery(days_ago: i64, session: &str) -> Delivery {
        Delivery {
            injection_id: format!("i-{days_ago}-{session}"),
            session_id: session.into(),
            mode: InjectionMode::Shadow,
            tokens: 10,
            omitted: 0,
            created_at: (chrono::Utc::now() - chrono::Duration::days(days_ago))
                .format("%Y-%m-%dT%H:%M:%SZ")
                .to_string(),
            items: vec![],
        }
    }

    #[test]
    fn deliveries_are_counted_per_day_oldest_first() {
        let list = [
            delivery(0, "a"),
            delivery(0, "b"),
            delivery(0, "a"),
            delivery(2, "a"),
            delivery(30, "c"),
        ];
        let (per_day, sessions) = daily(&list, 7);
        assert_eq!(per_day, vec![0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 3.0]);
        assert_eq!(sessions, vec![0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 2.0]);
    }
    use domain::claims::ClaimKind;

    #[test]
    fn every_claim_kind_has_copy() {
        for kind in ClaimKind::ALL {
            assert!(!kind_label(kind).is_empty());
            assert!(!kind_plural(kind).is_empty());
        }
    }

    #[test]
    fn claim_failures_never_echo_the_code() {
        for code in [
            "empty_statement",
            "statement_too_long",
            "conflict",
            "storage",
        ] {
            assert!(!claim_failure(code).contains(code));
        }
    }
}
