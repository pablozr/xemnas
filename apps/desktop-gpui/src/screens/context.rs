//! Contexto: what holds in the selected project right now, and what the agent
//! receives from it.
//!
//! Four surfaces over existing use cases: the project's context mode
//! (`ContextSettings`), the decisions in force (`Decisions::list`), the rules
//! of the project (`Claims`) and a Context Pack preview for a typed task
//! (`ContextPacks::build_pack`, exported through `application::export`). All
//! storage work runs off the UI thread; a project generation discards
//! responses that arrive after the user switched projects.

use std::path::PathBuf;

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
    DocumentIndex, DocumentKind, DocumentStore, Documents, ProjectDocument,
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
use gpui::prelude::*;
use gpui::{
    div, px, AnyElement, Context, Div, Entity, EventEmitter, Render, Role, Subscription, Toggled,
    Window,
};

use super::format::{calendar_date, clock, day_heading, short_date, thousands};
use crate::ui::controls::{action_button, button_foreground, ButtonKind};
use crate::ui::icons::{icon, IconName};
use crate::ui::patterns::{
    count_chip, empty_panel, error_banner, index_rail, index_row, panel_title, reading_page,
    section_label, skeleton_list, status_pill, toast, TOAST_DURATION,
};
use crate::ui::search_field::{SearchChanged, SearchField};
use crate::ui::theme::{text_style, Theme};
use crate::ui::tokens::{tint, SpacingScale, TypeScale};

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
    + InjectionHistory
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
        + InjectionHistory
        + Clone
        + Send
        + 'static
{
}

/// The use cases behind the screen, built by the composition root.
pub struct ContextServices<S> {
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
    Overview,
    Deliveries,
    Test,
    Decisions,
    Rules,
    Documents,
    Mode,
}

impl Section {
    fn label(self) -> &'static str {
        match self {
            Self::Overview => "Visão geral",
            Self::Deliveries => "Entregas",
            Self::Test => "Testar uma tarefa",
            Self::Decisions => "Decisões em vigor",
            Self::Rules => "Regras",
            Self::Documents => "Documentação",
            Self::Mode => "Modo de entrega",
        }
    }

    fn glyph(self) -> IconName {
        match self {
            Self::Overview => IconName::Activity,
            Self::Deliveries => IconName::Clock,
            Self::Test => IconName::Target,
            Self::Decisions => IconName::File,
            Self::Rules => IconName::Shield,
            Self::Documents => IconName::List,
            Self::Mode => IconName::Settings,
        }
    }

    fn id(self) -> &'static str {
        match self {
            Self::Overview => "context-nav-overview",
            Self::Deliveries => "context-nav-deliveries",
            Self::Test => "context-nav-test",
            Self::Decisions => "context-nav-decisions",
            Self::Rules => "context-nav-rules",
            Self::Documents => "context-nav-documents",
            Self::Mode => "context-nav-mode",
        }
    }
}

/// Documents listed per kind before "e mais N".
const DOCUMENTS_SHOWN: usize = 8;

/// How many decisions in force the screen lists.
const IN_FORCE_LIMIT: usize = 50;

struct Snapshot {
    settings: ProjectContextSettings,
    decisions: Vec<DecisionSummary>,
    claims: Vec<ClaimRecord>,
    documents: Vec<ProjectDocument>,
    deliveries: Vec<Delivery>,
}

enum Outcome {
    Loaded(Result<Box<Snapshot>, String>),
    SettingsSaved(Result<ProjectContextSettings, String>),
    Claims(Result<Vec<ClaimRecord>, String>, &'static str),
    Pack(Result<Box<ContextPack>, String>),
    Exported(Result<Option<PathBuf>, String>),
    Documents(Result<(DocumentIndex, Vec<ProjectDocument>), String>),
}

/// The Contexto destination of a selected project.
pub struct ContextScreen<S: ContextStores> {
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
    pack: Option<ContextPack>,
    error: Option<String>,
    notice: Option<String>,
    scroll: gpui::ScrollHandle,
    /// Demo-only: scroll to the end once loaded (`--open context:end`).
    scroll_to_end: bool,
    section: Section,
    /// The delivery whose items are open in the history.
    open_delivery: Option<String>,
    focus: std::collections::BTreeMap<&'static str, gpui::FocusHandle>,
    _subscriptions: Vec<Subscription>,
}

impl<S: ContextStores> EventEmitter<OpenDecision> for ContextScreen<S> {}

impl<S: ContextStores> ContextScreen<S> {
    /// Scrolls to the end of the page once it is loaded (demo captures).
    pub fn scroll_to_end(&mut self) {
        self.scroll_to_end = true;
    }

    /// Opens a page by name (`deliveries`, `test`, `decisions`, `rules`,
    /// `documents`, `mode`); demo captures reach it without input.
    pub fn open_section(&mut self, name: &str) {
        self.section = match name {
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
        self.section = section;
        self.scroll.set_offset(gpui::point(px(0.0), px(0.0)));
        cx.notify();
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
        let budget = field(cx, "Padrão do servidor (300)");
        let statement = field(cx, "Ex.: toda migração é forward-only");
        let task = field(cx, "Ex.: adicionar exportação em CSV das decisões");
        let subscriptions = [&budget, &statement, &task]
            .into_iter()
            .map(|field| cx.subscribe(field, |_, _, _: &SearchChanged, cx| cx.notify()))
            .collect();
        Self {
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
            pack: None,
            error: None,
            notice: None,
            scroll: gpui::ScrollHandle::new(),
            scroll_to_end: false,
            section: Section::Overview,
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
        self.generation += 1;
        self.snapshot = None;
        self.pack = None;
        self.confirm_retire = None;
        self.error = None;
        self.notice = None;
        self.task.update(cx, |field, cx| {
            field.set_context("Ex.: adicionar exportação em CSV das decisões", cx)
        });
        self.refresh(cx);
    }

    /// Reloads the snapshot of the current project.
    pub fn refresh(&mut self, cx: &mut Context<Self>) {
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
                self.show_notice("Modo de contexto salvo.".into(), cx);
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
                self.show_notice(format!("Prévia exportada para {}", path.display()), cx)
            }
            Outcome::Exported(Ok(None)) => {}
            Outcome::Documents(Ok((index, documents))) => {
                if let Some(snapshot) = &mut self.snapshot {
                    snapshot.documents = documents;
                }
                let changed = index.changed + index.removed;
                self.show_notice(
                    match (index.total, changed) {
                        (0, _) => "Nenhum documento encontrado na pasta do projeto.".into(),
                        (total, 0) => format!("Documentação lida: {total}, sem mudanças."),
                        (total, changed) => {
                            format!("Documentação lida: {total}, {changed} com mudanças.")
                        }
                    },
                    cx,
                );
            }
            Outcome::Loaded(Err(error))
            | Outcome::SettingsSaved(Err(error))
            | Outcome::Claims(Err(error), _)
            | Outcome::Pack(Err(error))
            | Outcome::Exported(Err(error))
            | Outcome::Documents(Err(error)) => self.error = Some(error),
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
            .ok_or_else(|| {
                format!("Use um número entre {MIN_BUDGET_TOKENS} e {MAX_BUDGET_TOKENS} tokens.")
            })
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
                "Não foi possível salvar o modo de contexto.".to_owned()
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
        self.statement.update(cx, |field, cx| {
            field.set_context("Ex.: toda migração é forward-only", cx)
        });
        self.run(cx, move |backend| {
            let created = backend.claims.create(NewClaim {
                project_id: project.clone(),
                kind,
                statement,
                valid_from: None,
                valid_until: None,
                source_decision_id: None,
            });
            Outcome::Claims(
                created
                    .map_err(|error| claim_failure(error.code()))
                    .and_then(|_| list_claims(backend, &project)),
                "Regra adicionada.",
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
                "Regra encerrada. Ela continua no histórico.",
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
                        "Não foi possível montar a prévia. Descreva a tarefa com outras palavras."
                            .to_owned()
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
                Err(_) => {
                    return Outcome::Exported(Err("Não foi possível gerar o arquivo.".into()))
                }
            };
            let extension = match format {
                ExportFormat::Markdown => "md",
                ExportFormat::Json => "json",
            };
            let Some(path) = rfd::FileDialog::new()
                .set_title("Exportar prévia de contexto")
                .add_filter("Documento", &[extension])
                .set_file_name(format!("contexto.{extension}"))
                .save_file()
            else {
                return Outcome::Exported(Ok(None));
            };
            Outcome::Exported(
                write_pack(&document, &path, true)
                    .map(|_| Some(path))
                    .map_err(|_| "Não foi possível salvar o arquivo nesse destino.".into()),
            )
        });
    }

    fn render_mode(&self, theme: &Theme, cx: &mut Context<Self>) -> Div {
        let colors = theme.colors;
        let options = [
            (
                ContextMode::Off,
                "Desligado",
                "Nada é calculado nem enviado ao agente.",
                IconName::Circle,
            ),
            (
                ContextMode::Shadow,
                "Medir",
                "Calcula o bloco e registra quanto seria enviado, sem enviar.",
                IconName::Eye,
            ),
            (
                ContextMode::Inject,
                "Ativo",
                "Anexa um bloco compacto de decisões e regras ao pedido do agente.",
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
                div()
                    .id("context-mode")
                    .flex()
                    .gap(px(SpacingScale::S2))
                    .role(Role::RadioGroup)
                    .aria_label("Modo de contexto")
                    .children(options.into_iter().map(|(mode, title, body, glyph)| {
                        let selected = self.mode == mode;
                        div()
                            .id(("context-mode-option", mode as usize))
                            .flex_1()
                            .min_w(px(0.0))
                            .flex()
                            .flex_col()
                            .gap(px(SpacingScale::S2))
                            .p(px(SpacingScale::S3))
                            .rounded(px(10.0))
                            .border_1()
                            .border_color(if selected {
                                colors.accent_default()
                            } else {
                                colors.glass_border_card()
                            })
                            .bg(if selected {
                                colors.selection()
                            } else {
                                colors.glass_fill_card()
                            })
                            .when(!selected, |option| {
                                option
                                    .hover(move |style| {
                                        style
                                            .bg(colors.glass_fill_medium())
                                            .border_color(colors.glass_border_card_hover())
                                    })
                                    .active(move |style| style.bg(colors.glass_fill_strong()))
                            })
                            .cursor_pointer()
                            .role(Role::RadioButton)
                            .aria_label(title)
                            .aria_toggled(if selected {
                                Toggled::True
                            } else {
                                Toggled::False
                            })
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.mode = mode;
                                cx.notify();
                            }))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(SpacingScale::S2))
                                    .child(icon(
                                        glyph,
                                        14.0,
                                        if selected {
                                            colors.accent_hover()
                                        } else {
                                            colors.text_muted()
                                        },
                                    ))
                                    .child(text_style(div(), TypeScale::ROW_TITLE).child(title)),
                            )
                            .child(
                                text_style(div(), TypeScale::META)
                                    .text_color(colors.text_muted())
                                    .child(body),
                            )
                    })),
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
                                    .child("Tokens por bloco"),
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
                                Some(at) if !edited => format!("Salvo em {}.", short_date(&at)),
                                _ => {
                                    format!("Vazio usa o padrão de {DEFAULT_BUDGET_TOKENS} tokens.")
                                }
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
                            .aria_label("Salvar modo de contexto")
                            .on_click(cx.listener(|this, _, _, cx| this.save_settings(cx)))
                            .child("Salvar"),
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
                    .child("Nenhuma decisão confirmada ainda. Confirme candidatos em Revisão.")
                    .into_any_element()
            } else {
                div()
                    .id("context-in-force")
                    .flex()
                    .flex_col()
                    .rounded(px(10.0))
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
                            .aria_label(format!("Abrir decisão: {}", decision.question))
                            .on_click(
                                cx.listener(move |_, _, _, cx| cx.emit(OpenDecision(id.clone()))),
                            )
                            .child(icon(IconName::File, 14.0, colors.text_muted()))
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

    /// The project's own documentation, read from its folder.
    fn render_documents(
        &self,
        theme: &Theme,
        documents: &[ProjectDocument],
        cx: &mut Context<Self>,
    ) -> Div {
        let colors = theme.colors;
        let reread = action_button(theme, "context-docs-reread", ButtonKind::Ghost, !self.busy)
            .aria_label("Ler a documentação de novo")
            .on_click(cx.listener(|this, _, _, cx| this.reindex(cx)))
            .child(icon(IconName::Rotate, 13.0, colors.text_secondary()))
            .child("Ler de novo");
        let header = div()
            .flex()
            .items_center()
            .gap(px(SpacingScale::S2))
            .child(div().flex_1())
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
                        .child(
                            "Nenhum documento encontrado. Arquivos Markdown em docs/, specs/, \
                             adr/ e o README da raiz aparecem aqui.",
                        ),
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
                        div()
                            .flex()
                            .items_center()
                            .gap(px(SpacingScale::S2))
                            .child(icon(IconName::File, 14.0, colors.text_secondary()))
                            .child(
                                text_style(div(), TypeScale::LABEL)
                                    .text_color(colors.text_secondary())
                                    .child(document_kind_plural(kind)),
                            )
                            .child(count_chip(theme, items.len().to_string())),
                    )
                    .children(items.into_iter().take(DOCUMENTS_SHOWN).map(|document| {
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(2.0))
                            .py(px(SpacingScale::S2))
                            .pl(px(SpacingScale::S6 - 2.0))
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
                                                .child(match document.headings.len() {
                                                    1 => "· 1 seção".to_owned(),
                                                    n => format!("· {n} seções"),
                                                }),
                                        )
                                    }),
                            )
                    }))
                    .when(more > 0, |group| {
                        group.child(
                            text_style(div(), TypeScale::META)
                                .pl(px(SpacingScale::S6 - 2.0))
                                .text_color(colors.text_muted())
                                .child(format!("E mais {more}")),
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
                .child(
                    "Sem regras ainda. Premissas, restrições, objetivos e convenções valem para \
                     todas as tarefas do projeto e entram no contexto do agente.",
                )
                .into_any_element()
        } else {
            div()
                .flex()
                .flex_col()
                .gap(px(SpacingScale::S4))
                .children(groups.map(|(kind, items)| {
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(SpacingScale::S1))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(SpacingScale::S2))
                                .child(icon(kind_icon(kind), 14.0, colors.text_secondary()))
                                .child(
                                    text_style(div(), TypeScale::LABEL)
                                        .text_color(colors.text_secondary())
                                        .child(kind_plural(kind)),
                                )
                                .child(count_chip(theme, items.len().to_string())),
                        )
                        .children(
                            items
                                .into_iter()
                                .map(|claim| self.claim_row(theme, claim, cx)),
                        )
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
                    .rounded(px(10.0))
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
                            .aria_label("Tipo da regra")
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
                                .aria_label("Adicionar regra")
                                .on_click(cx.listener(|this, _, _, cx| this.add_claim(cx)))
                                .child(icon(
                                    IconName::Plus,
                                    14.0,
                                    button_foreground(theme, ButtonKind::Secondary, typed),
                                ))
                                .child("Adicionar"),
                            ),
                    ),
            )
    }

    fn claim_row(&self, theme: &Theme, claim: &ClaimRecord, cx: &mut Context<Self>) -> AnyElement {
        let colors = theme.colors;
        let confirming = self.confirm_retire.as_deref() == Some(claim.claim_id.as_str());
        let id = claim.claim_id.clone();
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
                    .child(
                        text_style(div(), TypeScale::META)
                            .text_color(colors.text_muted())
                            .child(if confirming {
                                "Encerrar hoje? Ela deixa de valer e fica no histórico.".to_owned()
                            } else {
                                format!("Vale desde {}", calendar_date(&claim.valid_from))
                            }),
                    ),
            );
        if confirming {
            let retire_id = id.clone();
            row.child(
                action_button(theme, "context-claim-cancel", ButtonKind::Ghost, true)
                    .aria_label("Cancelar")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.confirm_retire = None;
                        cx.notify();
                    }))
                    .child("Cancelar"),
            )
            .child(
                action_button(
                    theme,
                    "context-claim-retire",
                    ButtonKind::Secondary,
                    !self.busy,
                )
                .aria_label("Encerrar regra")
                .on_click(
                    cx.listener(move |this, _, _, cx| this.retire_claim(retire_id.clone(), cx)),
                )
                .child("Encerrar"),
            )
            .into_any_element()
        } else {
            row.child(
                action_button(
                    theme,
                    gpui::ElementId::Name(format!("context-claim-{id}").into()),
                    ButtonKind::Ghost,
                    !self.busy,
                )
                .aria_label("Encerrar regra")
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.confirm_retire = Some(id.clone());
                    cx.notify();
                }))
                .child("Encerrar"),
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
                .rounded(px(10.0))
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
                                    "{decisions} decisão(ões) · {claims} regra(s){}",
                                    if pack.omitted > 0 {
                                        format!(" · {} fora do orçamento", pack.omitted)
                                    } else {
                                        String::new()
                                    }
                                )),
                        )
                        .child(
                            text_style(div(), TypeScale::META)
                                .text_color(colors.text_muted())
                                .child(format!(
                                    "{} de {} caracteres",
                                    thousands(pack.used_chars),
                                    thousands(pack.budget_chars)
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
                            .child(
                                "Nada deste projeto casou com a tarefa. O agente não receberia \
                                 contexto para ela.",
                            ),
                    )
                })
                .children(pack.decisions.iter().map(|decision| {
                    div()
                        .flex()
                        .gap(px(SpacingScale::S3))
                        .child(div().mt(px(3.0)).child(icon(
                            IconName::File,
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
                                        .child(decision.choice.clone()),
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
                            kind.map(kind_icon).unwrap_or(IconName::Target),
                            14.0,
                            colors.text_secondary(),
                        )))
                        .child(
                            text_style(div(), TypeScale::BODY_SMALL)
                                .flex_1()
                                .child(claim.statement.clone()),
                        )
                        .child(
                            text_style(div(), TypeScale::META)
                                .text_color(colors.text_muted())
                                .child(format!(
                                    "{}{}",
                                    kind.map(kind_label).unwrap_or("Regra"),
                                    if claim.matched { " · casou" } else { "" }
                                )),
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
                                .aria_label("Exportar em Markdown")
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
                            .aria_label("Exportar em JSON")
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
                        .aria_label("Montar prévia")
                        .on_click(cx.listener(|this, _, _, cx| this.build_pack(cx)))
                        .child("Montar prévia"),
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
        let groups: [(&str, &[Section]); 3] = [
            (
                "Agente",
                &[Section::Overview, Section::Deliveries, Section::Test],
            ),
            (
                "Fontes",
                &[Section::Decisions, Section::Rules, Section::Documents],
            ),
            ("Ajustes", &[Section::Mode]),
        ];
        let mut list = div()
            .flex()
            .flex_col()
            .gap(px(2.0))
            .px(px(SpacingScale::S2));
        for (label, sections) in groups {
            list = list.child(
                div()
                    .px(px(SpacingScale::S3))
                    .pt(px(SpacingScale::S4))
                    .pb(px(SpacingScale::S1))
                    .child(section_label(theme, label)),
            );
            for &section in sections {
                let badge = counts
                    .and_then(|(decisions, rules, documents, deliveries)| match section {
                        Section::Decisions => Some(decisions),
                        Section::Rules => Some(rules),
                        Section::Documents => Some(documents),
                        Section::Deliveries => Some(deliveries),
                        _ => None,
                    })
                    .filter(|count| *count > 0)
                    .map(|count| count.to_string());
                let focus = self.focus_for(section.id(), cx);
                list = list.child(
                    index_row(
                        theme,
                        section.id(),
                        self.section == section,
                        section.glyph(),
                        section.label(),
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
                            ContextMode::Off => "Desligado".to_owned(),
                            mode => format!("{} · até {budget} tokens", mode_word(mode)),
                        }),
                )
        });
        index_rail(theme)
            .child(
                div()
                    .flex_none()
                    .px(px(SpacingScale::S4))
                    .pt(px(SpacingScale::S4))
                    .child(panel_title(theme, "Contexto")),
            )
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
            Section::Overview => (
                "Contexto do agente",
                "O que o agente de código recebe deste projeto, de onde vem e como chega a ele.",
                self.render_overview(theme, &settings, &decisions, &claims, &deliveries, cx),
            ),
            Section::Deliveries => (
                "Entregas",
                "Cada bloco calculado para o agente: se foi enviado ou só medido, quanto \
                 ocupou e o que ficou de fora do orçamento.",
                self.render_deliveries(theme, &settings, &deliveries, cx),
            ),
            Section::Test => (
                "Testar uma tarefa",
                "Descreva uma tarefa como pediria ao agente e veja quais decisões e regras \
                 entrariam no contexto dela.",
                self.render_pack(theme, cx),
            ),
            Section::Decisions => (
                "Decisões em vigor",
                "Confirmadas e não substituídas: entram no contexto quando a tarefa toca \
                 no que decidem. Abra uma para ver o histórico.",
                self.render_in_force(theme, &decisions, cx),
            ),
            Section::Rules => (
                "Regras",
                "Premissas, restrições, objetivos e convenções que valem para todas as \
                 tarefas do projeto.",
                self.render_rules(theme, &claims, cx),
            ),
            Section::Documents => (
                "Documentação",
                "Lida da pasta do projeto (docs/, specs/, ADRs e README). Alimenta a Visão \
                 como fonte; não é enviada ao agente nem vira decisão.",
                self.render_documents(theme, &documents, cx),
            ),
            Section::Mode => (
                "Modo de entrega",
                "Quando o agente recebe contexto e quanto cabe em cada bloco.",
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

        // Where things stand, in one sentence.
        let change = self.focus_for("context-change-mode", cx);
        let status = div()
            .flex()
            .items_center()
            .gap(px(SpacingScale::S4))
            .p(px(SpacingScale::S5))
            .rounded(px(12.0))
            .border_1()
            .border_color(colors.glass_border_card())
            .bg(colors.glass_fill_card())
            .child(
                div()
                    .size(px(28.0))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_full()
                    .bg(tint.alpha(0.16))
                    .child(div().size(px(10.0)).rounded_full().bg(tint)),
            )
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .flex()
                    .flex_col()
                    .gap(px(2.0))
                    .child(text_style(div(), TypeScale::HEADING_2).child(mode_word(mode)))
                    .child(
                        text_style(div(), TypeScale::BODY_SMALL)
                            .text_color(colors.text_secondary())
                            .child(mode_sentence(mode, budget)),
                    ),
            )
            .child(
                action_button(theme, "context-change-mode", ButtonKind::Secondary, true)
                    .aria_label("Mudar o modo de entrega")
                    .track_focus(&change)
                    .on_click(cx.listener(|this, _, _, cx| this.go(Section::Mode, cx)))
                    .child(if mode == ContextMode::Off {
                        "Ativar"
                    } else {
                        "Mudar modo"
                    }),
            );

        // How it gets there: sources, selection, delivery.
        let stage = |number: &'static str, title: &'static str, lines: Vec<String>, note: &str| {
            div()
                .flex_1()
                .min_w(px(0.0))
                .flex()
                .flex_col()
                .gap(px(SpacingScale::S2))
                .p(px(SpacingScale::S4))
                .rounded(px(10.0))
                .border_1()
                .border_color(colors.hairline_divider())
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(SpacingScale::S2))
                        .child(
                            div()
                                .size(px(18.0))
                                .flex()
                                .items_center()
                                .justify_center()
                                .rounded_full()
                                .border_1()
                                .border_color(colors.glass_border_card_hover())
                                .child(
                                    text_style(div(), TypeScale::META)
                                        .text_color(colors.text_secondary())
                                        .child(number),
                                ),
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
                .when(!note.is_empty(), |stage| {
                    stage.child(
                        text_style(div(), TypeScale::META)
                            .text_color(colors.text_muted())
                            .child(note.to_owned()),
                    )
                })
        };
        let arrow = || {
            div().flex_none().self_center().child(icon(
                IconName::ChevronRight,
                14.0,
                colors.text_disabled(),
            ))
        };
        let pipeline = div()
            .flex()
            .items_stretch()
            .gap(px(SpacingScale::S2))
            .child(stage(
                "1",
                "Fontes",
                vec![
                    plural(decisions.len(), "decisão em vigor", "decisões em vigor"),
                    plural(claims.len(), "regra do projeto", "regras do projeto"),
                ],
                "A documentação fica na Visão.",
            ))
            .child(arrow())
            .child(stage(
                "2",
                "Seleção",
                vec![
                    "No pedido: o texto e os arquivos citados.".to_owned(),
                    "Na edição: o que o mapa liga ao arquivo.".to_owned(),
                ],
                "",
            ))
            .child(arrow())
            .child(stage(
                "3",
                "Entrega",
                vec![
                    format!("Até {budget} tokens por bloco."),
                    "Sem repetir o que a sessão já recebeu.".to_owned(),
                ],
                "",
            ));

        // The last seven days, from the audit.
        let since = (chrono::Utc::now() - chrono::Duration::days(7))
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string();
        let summary = DeliverySummary::since(deliveries, &since);
        let measured = summary.deliveries - summary.sent;
        let tile = |value: String, label: &'static str, detail: String| {
            div()
                .flex_1()
                .min_w(px(0.0))
                .flex()
                .flex_col()
                .gap(px(2.0))
                .p(px(SpacingScale::S4))
                .rounded(px(10.0))
                .bg(colors.glass_fill_card())
                .border_1()
                .border_color(colors.glass_border_card())
                .child(
                    text_style(div(), TypeScale::META)
                        .text_color(colors.text_muted())
                        .child(label),
                )
                .child(
                    text_style(div(), TypeScale::DISPLAY)
                        .text_color(colors.text_primary())
                        .child(value),
                )
                .child(
                    text_style(div(), TypeScale::META)
                        .text_color(colors.text_muted())
                        .child(detail),
                )
        };
        let metrics = div()
            .flex()
            .gap(px(SpacingScale::S3))
            .child(tile(
                summary.deliveries.to_string(),
                "Entregas",
                format!("{} enviadas · {measured} medidas", summary.sent),
            ))
            .child(tile(
                summary.sessions.to_string(),
                "Sessões do agente",
                "conversas que receberam".to_owned(),
            ))
            .child(tile(
                summary.average_tokens().to_string(),
                "Tokens por bloco",
                format!("média · limite {budget}"),
            ))
            .child(tile(
                summary.omitted.to_string(),
                "Fora do orçamento",
                "itens relevantes que não couberam".to_owned(),
            ));

        // The latest deliveries.
        let see_all = self.focus_for("context-see-deliveries", cx);
        let recent_header = div()
            .flex()
            .items_center()
            .child(
                div()
                    .flex_1()
                    .child(section_label(theme, "Entregas recentes")),
            )
            .when(!deliveries.is_empty(), |row| {
                row.child(
                    action_button(theme, "context-see-deliveries", ButtonKind::Ghost, true)
                        .aria_label("Ver todas as entregas")
                        .track_focus(&see_all)
                        .on_click(cx.listener(|this, _, _, cx| this.go(Section::Deliveries, cx)))
                        .child("Ver todas")
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
                    .child(section_label(theme, "Como chega ao agente"))
                    .child(pipeline),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(SpacingScale::S3))
                    .child(section_label(theme, "Últimos 7 dias"))
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
                    .aria_label("Escolher o modo de entrega")
                    .track_focus(&focus)
                    .on_click(cx.listener(|this, _, _, cx| this.go(Section::Mode, cx)))
                    .child("Escolher modo"),
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
                                    .child(plural(rows.len(), "entrega", "entregas")),
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
            .aria_label(format!(
                "{} às {}: {} itens, {} tokens, sessão {}",
                if sent { "Enviado" } else { "Medido" },
                clock(&delivery.created_at).unwrap_or_default(),
                delivery.items.len(),
                delivery.tokens,
                short_ref(&delivery.session_id)
            ))
            .child(
                text_style(div(), TypeScale::META)
                    .w(px(40.0))
                    .flex_none()
                    .font_family(Theme::font_mono())
                    .text_color(colors.text_muted())
                    .child(clock(&delivery.created_at).unwrap_or_default()),
            )
            .child(div().w(px(78.0)).flex_none().child(status_pill(
                theme,
                tint,
                if sent { "Enviado" } else { "Medido" },
            )))
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
                                plural(delivery.items.len(), "item", "itens"),
                                if delivery.omitted > 0 {
                                    format!(" · {} fora do orçamento", delivery.omitted)
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
                            .child(format!("{} / {budget} tokens", delivery.tokens)),
                    )
                    .child(
                        div()
                            .h(px(3.0))
                            .w_full()
                            .rounded_full()
                            .bg(colors.surface())
                            .child(
                                div()
                                    .h_full()
                                    .w(gpui::relative(fill))
                                    .rounded_full()
                                    .bg(tint),
                            ),
                    ),
            )
            .child(
                text_style(div(), TypeScale::META)
                    .w(px(64.0))
                    .flex_none()
                    .font_family(Theme::font_mono())
                    .text_color(colors.text_muted())
                    .child(short_ref(&delivery.session_id)),
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
        .rounded(px(10.0))
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
                        ItemKind::Decision => IconName::File,
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
                            "Item que não existe mais".to_owned()
                        } else {
                            item.label.clone()
                        }),
                )
                .child(
                    text_style(div(), TypeScale::META)
                        .text_color(colors.text_muted())
                        .child(match item.kind {
                            ItemKind::Decision => format!("decisão v{}", item.version),
                            ItemKind::Claim => "regra".to_owned(),
                        }),
                )
        }))
        .when(delivery.items.is_empty(), |list| {
            list.child(
                text_style(div(), TypeScale::BODY_SMALL)
                    .text_color(colors.text_muted())
                    .child("Nada do projeto casou com o pedido; o bloco saiu vazio."),
            )
        })
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
        [] => "Bloco vazio".to_owned(),
        [only] => (*only).to_owned(),
        [first, rest @ ..] => format!("{first} e mais {}", rest.len()),
    }
}

fn mode_word(mode: ContextMode) -> &'static str {
    match mode {
        ContextMode::Off => "Desligado",
        ContextMode::Shadow => "Medindo",
        ContextMode::Inject => "Ativo",
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
        ContextMode::Off => "Nada é calculado nem enviado. Ative para que o agente receba, a \
                             cada pedido, as decisões e regras que importam para a tarefa."
            .to_owned(),
        ContextMode::Shadow => format!(
            "O bloco é calculado e registrado em Entregas, mas não vai para o agente. Use \
             para ver o que seria enviado (até {budget} tokens) antes de ativar."
        ),
        ContextMode::Inject => format!(
            "A cada pedido e a cada edição, o agente recebe um bloco de até {budget} tokens \
             com as decisões e regras que valem para aquela tarefa."
        ),
    }
}

fn empty_deliveries(mode: ContextMode) -> &'static str {
    match mode {
        ContextMode::Off => {
            "Nenhuma entrega: o modo está desligado. Escolha Medir para ver o que seria \
             enviado, ou Ativo para enviar."
        }
        _ => {
            "Nenhuma entrega ainda. Elas aparecem quando o agente fizer um pedido neste \
             projeto com o plugin conectado."
        }
    }
}

/// `1 regra`, `3 regras`.
fn plural(count: usize, one: &str, many: &str) -> String {
    if count == 1 {
        format!("1 {one}")
    } else {
        format!("{count} {many}")
    }
}

impl<S: ContextStores> Render for ContextScreen<S> {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::current(cx);
        let rail = self.render_rail(&theme, cx);
        let body: AnyElement = match self.snapshot.as_ref() {
            None if self.error.is_none() => div()
                .p(px(SpacingScale::S8))
                .child(skeleton_list(&theme, "context-skeleton", 5))
                .into_any_element(),
            None => empty_panel(
                &theme,
                IconName::Layers,
                "Contexto",
                "Não foi possível carregar o contexto",
                "Tente de novo; seus dados não foram alterados.",
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
                .aria_label("Tentar de novo")
                .on_click(cx.listener(|this, _, _, cx| this.refresh(cx)))
                .child("Tentar de novo")
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
        "Não foi possível ler o modo de contexto.".to_owned()
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
            "Não foi possível ler as decisões em vigor.".to_owned()
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
            "Não foi possível ler as entregas ao agente.".to_owned()
        })?;
    Ok(Snapshot {
        settings,
        decisions,
        claims,
        documents,
        deliveries,
    })
}

fn list_documents<S: ContextStores>(
    backend: &ContextServices<S>,
    project: &str,
) -> Result<Vec<ProjectDocument>, String> {
    backend.documents.list(project).map_err(|error| {
        tracing::error!(error = ?error, operation = "documents", "list failed");
        "Não foi possível ler a documentação do projeto.".to_owned()
    })
}

/// Product copy for an indexing failure.
fn document_failure(error: application::documents::DocumentError) -> String {
    use application::documents::DocumentError;
    match error {
        DocumentError::FolderUnavailable => {
            "A pasta do projeto não está acessível; a última leitura continua valendo.".into()
        }
        DocumentError::ProjectNotFound => "Projeto não encontrado.".into(),
        DocumentError::Storage(detail) => {
            tracing::error!(error = %detail, operation = "documents_index", "storage failed");
            "Não foi possível ler a documentação. Tente de novo.".into()
        }
    }
}

fn document_kind_plural(kind: DocumentKind) -> &'static str {
    match kind {
        DocumentKind::Adr => "Registros de decisão (ADR)",
        DocumentKind::Spec => "Especificações",
        DocumentKind::Readme => "READMEs",
        DocumentKind::Guide => "Guias e notas",
    }
}

fn list_claims<S: ContextStores>(
    backend: &ContextServices<S>,
    project: &str,
) -> Result<Vec<ClaimRecord>, String> {
    backend.claims.list(project, None).map_err(|error| {
        tracing::error!(error = %error, operation = "claims", "list failed");
        "Não foi possível ler as regras do projeto.".to_owned()
    })
}

/// Product copy for a claim failure code.
fn claim_failure(code: &str) -> String {
    match code {
        "empty_statement" => "Escreva a regra antes de adicionar.".into(),
        "statement_too_long" => "A regra ficou longa demais; resuma em uma frase.".into(),
        "already_ended" | "conflict" => {
            "A regra mudou enquanto você editava. A lista foi atualizada.".into()
        }
        _ => "Não foi possível salvar a regra.".into(),
    }
}

fn kind_label(kind: ClaimKind) -> &'static str {
    match kind {
        ClaimKind::Assumption => "Premissa",
        ClaimKind::Constraint => "Restrição",
        ClaimKind::Goal => "Objetivo",
        ClaimKind::Convention => "Convenção",
    }
}

fn kind_plural(kind: ClaimKind) -> &'static str {
    match kind {
        ClaimKind::Assumption => "Premissas",
        ClaimKind::Constraint => "Restrições",
        ClaimKind::Goal => "Objetivos",
        ClaimKind::Convention => "Convenções",
    }
}

fn kind_icon(kind: ClaimKind) -> IconName {
    match kind {
        ClaimKind::Assumption => IconName::CheckCircle,
        ClaimKind::Constraint => IconName::Shield,
        ClaimKind::Goal => IconName::Target,
        ClaimKind::Convention => IconName::List,
    }
}

#[cfg(test)]
mod tests {
    use super::{claim_failure, kind_label, kind_plural};
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
