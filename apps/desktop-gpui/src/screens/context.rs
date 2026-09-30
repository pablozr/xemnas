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
use application::export::{preview_pack, write_pack, ExportFormat};
use application::injection::{DEFAULT_BUDGET_TOKENS, MAX_BUDGET_TOKENS, MIN_BUDGET_TOKENS};
use application::projects::ProjectRepository;
use application::relations::RelationStore;
use domain::claims::ClaimKind;
use gpui::prelude::*;
use gpui::{
    div, px, AnyElement, Context, Div, Entity, EventEmitter, Render, Role, Subscription, Toggled,
    Window,
};

use super::format::{short_date, thousands};
use crate::ui::controls::{action_button, button_foreground, ButtonKind};
use crate::ui::icons::{icon, IconName};
use crate::ui::patterns::{
    count_chip, empty_panel, error_banner, reading_page, section_label, skeleton_list, toast,
    TOAST_DURATION,
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
}

/// Asks the shell to open a decision in Decisões.
pub struct OpenDecision(pub String);

/// How many decisions in force the screen lists.
const IN_FORCE_LIMIT: usize = 50;

struct Snapshot {
    settings: ProjectContextSettings,
    decisions: Vec<DecisionSummary>,
    claims: Vec<ClaimRecord>,
}

enum Outcome {
    Loaded(Result<Box<Snapshot>, String>),
    SettingsSaved(Result<ProjectContextSettings, String>),
    Claims(Result<Vec<ClaimRecord>, String>, &'static str),
    Pack(Result<Box<ContextPack>, String>),
    Exported(Result<Option<PathBuf>, String>),
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
    _subscriptions: Vec<Subscription>,
}

impl<S: ContextStores> EventEmitter<OpenDecision> for ContextScreen<S> {}

impl<S: ContextStores> ContextScreen<S> {
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
            Outcome::Loaded(Err(error))
            | Outcome::SettingsSaved(Err(error))
            | Outcome::Claims(Err(error), _)
            | Outcome::Pack(Err(error))
            | Outcome::Exported(Err(error)) => self.error = Some(error),
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
            .child(section_label(theme, "O que o agente recebe"))
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
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(SpacingScale::S2))
                    .child(section_label(theme, "Decisões em vigor"))
                    .child(count_chip(theme, decisions.len().to_string())),
            )
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
            .child(section_label(theme, "Regras do projeto"))
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
                                format!("Vale desde {}", short_date(&claim.valid_from))
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
            .child(section_label(theme, "Prévia do contexto"))
            .child(
                text_style(div(), TypeScale::BODY_SMALL)
                    .text_color(colors.text_secondary())
                    .child(
                        "Descreva uma tarefa como pediria ao agente e veja quais decisões e \
                         regras entrariam no contexto dela.",
                    ),
            )
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

impl<S: ContextStores> Render for ContextScreen<S> {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::current(cx);
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
            Some(snapshot) => {
                let decisions = snapshot.decisions.clone();
                let claims = snapshot.claims.clone();
                let mode = self.render_mode(&theme, cx);
                let in_force = self.render_in_force(&theme, &decisions, cx);
                let rules = self.render_rules(&theme, &claims, cx);
                let pack = self.render_pack(&theme, cx);
                reading_page(
                    "context-page",
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(SpacingScale::S8))
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap(px(SpacingScale::S1))
                                .child(
                                    text_style(div(), TypeScale::HEADING_1)
                                        .child("O que vale neste projeto"),
                                )
                                .child(
                                    text_style(div(), TypeScale::BODY_SMALL)
                                        .text_color(theme.colors.text_muted())
                                        .child(
                                            "Decisões em vigor e regras que o agente recebe \
                                             como contexto, e como ele as recebe.",
                                        ),
                                ),
                        )
                        .child(mode)
                        .child(in_force)
                        .child(rules)
                        .child(pack),
                )
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
            .flex_col()
            .children(self.error.clone().map(|error| {
                error_banner(&theme, &error)
                    .id("context-error")
                    .role(Role::Alert)
                    .children(retry)
            }))
            .child(div().flex_1().min_h(px(0.0)).flex().flex_col().child(body))
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
    Ok(Snapshot {
        settings,
        decisions,
        claims,
    })
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
