//! Versioned decision documents with an independent chronological index.
use super::format::short_date;
use super::{
    decision_editor::{DecisionEditor, RevisionEvent},
    evidence::{self, SourceLines},
};
use crate::ui::controls::{action_button, button_foreground, ButtonKind};
use crate::ui::{
    glass::focus_ring,
    icons::{icon, IconName},
    patterns::{
        count_chip, empty_panel, error_banner, fade_in, hover_tint, mark_selected, panel_title,
        reading_title, section_header, section_label, skeleton_list, status_pill, toast,
        track_hover, word_wrapped, READING_WIDTH, TOAST_DURATION,
    },
    search_field::{SearchChanged, SearchField},
    theme::{text_style, Theme},
    tokens::{SpacingScale, TypeScale},
};

use application::decisions::{
    DecisionDetail, DecisionFilter, DecisionPage, DecisionSearchHit, DecisionSource,
    DecisionStatus, DecisionStore, DecisionSummary, Decisions, SearchQuery,
};
use application::export::{Export, ExportDocument, ExportError, ExportFormat};
use application::inbox::InboxStore;
use application::relations::{DecisionRelations, RelationDirection, RelationStore};
use domain::relations::RelationKind;
use gpui::prelude::*;
use gpui::{
    div, px, AnyElement, ClipboardItem, Context, Entity, FocusHandle, Focusable, Render, Role,
    Subscription, Window,
};
use std::{collections::BTreeMap, path::PathBuf, time::Duration};

struct Backend<S> {
    decisions: Decisions<S>,
    export: Export<S>,
    relations: DecisionRelations<S>,
}
/// A relation of the open decision, with the other decision's question.
#[derive(Clone)]
struct LinkItem {
    kind: RelationKind,
    direction: RelationDirection,
    other_id: String,
    question: String,
    superseded: bool,
}
struct IndexEntry {
    id: String,
    question: String,
    meta: String,
    month: String,
    version: Option<i64>,
    status: Option<DecisionStatus>,
}
enum Outcome {
    Page(Result<DecisionPage, String>, bool),
    Search(Result<Vec<DecisionSearchHit>, String>),
    Detail(Result<(DecisionDetail, Vec<DecisionSource>), String>),
    Revised(Result<(DecisionDetail, Vec<DecisionSource>), String>),
    Preview(Result<ExportDocument, String>),
    Saved(Result<Option<String>, ExportError>, Option<PathBuf>),
    Links(String, Vec<LinkItem>),
    Linked(Result<&'static str, String>),
}
#[derive(Clone)]
enum Action {
    Document,
    History,
    Version(i64),
    Select(String),
    More,
    Retry,
    Revise,
    Export(ExportFormat),
    CloseExport,
    SaveExport(bool),
    CopySource,
    ExpandSource,
    Source(usize),
    Context(usize),
    Filter,
    Relate(Option<RelationKind>),
    Link(String),
}

/// All storage work runs off the UI thread; project/query generations reject stale reads.
pub struct DecisionsScreen<S: DecisionStore + InboxStore + RelationStore + Send + 'static> {
    /// Row under the pointer, driving the hover spring.
    hovered: Option<String>,
    backend: Option<Backend<S>>,
    project: Option<String>,
    generation: u64,
    busy: bool,
    loaded: bool,
    query: String,
    search: Entity<SearchField>,
    _search_subscription: Subscription,
    rows: Vec<DecisionSummary>,
    hits: Vec<DecisionSearchHit>,
    cursor: Option<String>,
    all_statuses: bool,
    selected: Option<String>,
    detail: Option<DecisionDetail>,
    sources: Vec<DecisionSource>,
    lines: Vec<Option<SourceLines>>,
    source: usize,
    expanded: bool,
    contexts: [bool; 4],
    history: bool,
    version: Option<i64>,
    editor: Option<Entity<DecisionEditor>>,
    editor_subscription: Option<Subscription>,
    preview: Option<ExportDocument>,
    overwrite: Option<PathBuf>,
    error: Option<String>,
    notice: Option<String>,
    /// Notice whose dismissal timer is already running.
    notice_scheduled: Option<String>,
    focus: BTreeMap<String, FocusHandle>,
    reader_focus: FocusHandle,
    restore_focus: bool,
    /// Relations of the open decision, once loaded.
    links: Option<Vec<LinkItem>>,
    /// The relation kind being created, while the picker is open.
    relating: Option<RelationKind>,
}
impl<S: DecisionStore + InboxStore + RelationStore + Send + 'static> DecisionsScreen<S> {
    /// Mounts the persistent document reader and its project-scoped search.
    pub fn new(
        cx: &mut Context<Self>,
        decisions: Decisions<S>,
        export: Export<S>,
        relations: DecisionRelations<S>,
    ) -> Self {
        let search = cx.new(|cx| {
            let mut field = SearchField::new(cx);
            field.set_width(240.0);
            field.set_context("Buscar nas decisões", cx);
            field
        });
        let subscription = cx.subscribe(&search, |this, _, event: &SearchChanged, cx| {
            if this.editor.is_some() || this.preview.is_some() {
                return;
            }
            this.query = event.0.clone();
            this.generation += 1;
            this.clear_document();
            this.rows.clear();
            this.hits.clear();
            this.cursor = None;
            this.loaded = false;
            let generation = this.generation;
            cx.spawn(async move |this, cx| {
                cx.background_executor()
                    .timer(Duration::from_millis(220))
                    .await;
                let _ = this.update(cx, |this, cx| {
                    if this.generation == generation {
                        this.refresh(cx);
                    }
                });
            })
            .detach();
            cx.notify();
        });
        Self {
            hovered: None,
            backend: Some(Backend {
                decisions,
                export,
                relations,
            }),
            project: None,
            generation: 0,
            busy: false,
            loaded: false,
            query: String::new(),
            search,
            _search_subscription: subscription,
            rows: Vec::new(),
            hits: Vec::new(),
            cursor: None,
            all_statuses: false,
            selected: None,
            detail: None,
            sources: Vec::new(),
            lines: Vec::new(),
            source: 0,
            expanded: false,
            contexts: [true, false, false, false],
            history: false,
            version: None,
            editor: None,
            editor_subscription: None,
            preview: None,
            overwrite: None,
            error: None,
            notice: None,
            notice_scheduled: None,
            focus: BTreeMap::new(),
            reader_focus: cx.focus_handle(),
            restore_focus: false,
            links: None,
            relating: None,
        }
    }
    /// Focuses the index search while the reader is active.
    pub fn focus_search(&self, window: &mut Window, cx: &mut Context<Self>) {
        if self.editor.is_some() || self.preview.is_some() {
            return;
        }
        window.focus(&self.search.read(cx).focus_handle(cx), cx);
    }
    fn clear_document(&mut self) {
        self.selected = None;
        self.detail = None;
        self.sources.clear();
        self.lines.clear();
        self.version = None;
        self.history = false;
        self.editor = None;
        self.editor_subscription = None;
        self.preview = None;
        self.overwrite = None;
        self.expanded = false;
        self.links = None;
        self.relating = None;
    }
    /// Loads the relations of the open decision in a second pass, so the
    /// document shows as soon as it is read.
    fn load_links(&mut self, cx: &mut Context<Self>) {
        let Some(id) = self.selected.clone() else {
            return;
        };
        self.run(cx, move |backend| {
            let links = backend
                .relations
                .of(&id)
                .map(|views| {
                    views
                        .into_iter()
                        .map(|view| {
                            let other = backend.decisions.detail(&view.other_id).ok();
                            LinkItem {
                                kind: view.kind,
                                direction: view.direction,
                                question: other
                                    .as_ref()
                                    .map(|detail| detail.summary.question.clone())
                                    .unwrap_or_else(|| "Decisão indisponível".into()),
                                superseded: other.is_some_and(|detail| {
                                    detail.summary.status == DecisionStatus::Superseded
                                }),
                                other_id: view.other_id,
                            }
                        })
                        .collect()
                })
                .unwrap_or_else(|error| {
                    tracing::error!(error = %error, operation = "relations", "list failed");
                    Vec::new()
                });
            Outcome::Links(id, links)
        });
    }
    /// Clears cross-project state before loading the selected project's records.
    pub fn set_project(&mut self, project: Option<String>, cx: &mut Context<Self>) {
        if self.project == project {
            return;
        }
        self.project = project;
        self.generation += 1;
        self.query.clear();
        self.rows.clear();
        self.hits.clear();
        self.cursor = None;
        self.clear_document();
        self.loaded = false;
        self.error = None;
        self.notice = None;
        self.search
            .update(cx, |field, cx| field.set_context("Buscar nas decisões", cx));
        self.refresh(cx);
    }
    /// Reloads the current index after returning from candidate review.
    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        if self.busy || self.project.is_none() || self.editor.is_some() || self.preview.is_some() {
            return;
        }
        self.page(false, cx);
    }
    fn page(&mut self, append: bool, cx: &mut Context<Self>) {
        if self.project.is_none() || self.busy {
            return;
        }
        let project = self.project.clone();
        let query = self.query.clone();
        if !query.trim().is_empty() {
            self.run(cx,move|backend|Outcome::Search(backend.decisions.search(&SearchQuery{query,project_id:project,limit:50}).map_err(|_|"Não foi possível buscar. Use palavras da pergunta, escolha ou justificativa.".into())));
        } else {
            let cursor = if append { self.cursor.clone() } else { None };
            let statuses = if self.all_statuses {
                vec![DecisionStatus::Accepted, DecisionStatus::Superseded]
            } else {
                vec![DecisionStatus::Accepted]
            };
            self.run(cx, move |backend| {
                Outcome::Page(
                    backend
                        .decisions
                        .list(&DecisionFilter {
                            project_id: project,
                            cursor,
                            statuses,
                            limit: 50,
                        })
                        .map_err(|_| "Não foi possível carregar as decisões.".into()),
                    append,
                )
            });
        }
    }
    fn select(&mut self, id: String, cx: &mut Context<Self>) {
        if self.busy || self.editor.is_some() || self.preview.is_some() {
            return;
        }
        self.clear_document();
        self.selected = Some(id.clone());
        let project = self.project.clone();
        self.run(cx, move |backend| {
            Outcome::Detail(load_document(&backend.decisions, &id, project.as_deref()))
        });
    }
    fn install(&mut self, detail: DecisionDetail, sources: Vec<DecisionSource>) {
        self.selected = Some(detail.summary.decision_id.clone());
        self.lines = sources
            .iter()
            .map(|source| source.artifact.as_ref().map(SourceLines::new))
            .collect();
        self.detail = Some(detail);
        self.sources = sources;
        self.source = 0;
        self.version = None;
        self.history = false;
    }
    fn run(
        &mut self,
        cx: &mut Context<Self>,
        operation: impl FnOnce(&Backend<S>) -> Outcome + Send + 'static,
    ) {
        let Some(backend) = self.backend.take() else {
            return;
        };
        self.busy = true;
        self.error = None;
        if let Some(editor) = &self.editor {
            editor.update(cx, |editor, cx| editor.set_busy(true, cx));
        }
        let generation = self.generation;
        cx.notify();
        cx.spawn(async move|this,cx|{
            let (backend,outcome)=cx.background_executor().spawn(async move{let outcome=operation(&backend);(backend,outcome)}).await;
            let _=this.update(cx,|this,cx|{
                this.backend=Some(backend);this.busy=false;
                if let Some(editor)=&this.editor{editor.update(cx,|editor,cx|editor.set_busy(false,cx));}
                if generation!=this.generation{this.refresh(cx);cx.notify();return;}
                match outcome{
                    Outcome::Page(Ok(page),append)=>{
                        this.loaded=true;if !append{this.rows.clear();}
                        for row in page.decisions{if let Some(old)=this.rows.iter_mut().find(|old|old.decision_id==row.decision_id){*old=row;}else{this.rows.push(row);}}
                        this.cursor=page.next_cursor;
                        let next=this.selected.clone().filter(|id|this.rows.iter().any(|row|row.decision_id==*id)).or_else(||this.rows.first().map(|row|row.decision_id.clone()));
                        if !append{if let Some(id)=next{this.select(id,cx);}else{this.clear_document();}}
                    }
                    Outcome::Search(Ok(hits))=>{this.hits=hits;this.loaded=true;if let Some(hit)=this.hits.first(){this.select(hit.decision_id.clone(),cx);}else{this.clear_document();}}
                    Outcome::Detail(Ok((detail,sources)))=>{this.install(detail,sources);this.load_links(cx);}
                    Outcome::Links(id,links)=>{if this.selected.as_deref()==Some(id.as_str()){this.links=Some(links);}}
                    Outcome::Linked(Ok(message))=>{this.relating=None;this.notice=Some(message.into());this.refresh(cx);}
                    Outcome::Linked(Err(error))=>this.error=Some(error),
                    Outcome::Revised(Ok((detail,sources)))=>{for row in &mut this.rows{if row.decision_id==detail.summary.decision_id{*row=detail.summary.clone();}}
                        this.editor=None;this.editor_subscription=None;this.restore_focus=true;this.install(detail,sources);this.load_links(cx);this.notice=Some("Nova versão salva. Histórico preservado.".into());}
                    Outcome::Preview(Ok(document))=>{this.preview=Some(document);this.overwrite=None;}
                    Outcome::Saved(Ok(Some(path)),_)=>{this.preview=None;this.overwrite=None;this.restore_focus=true;this.notice=Some(format!("Exportado para {path}"));}
                    Outcome::Saved(Ok(None),_)=>{},
                    Outcome::Saved(Err(ExportError::DestinationExists),path)=>{this.overwrite=path;},
                    Outcome::Saved(Err(_),_)=>this.error=Some("Não foi possível salvar o arquivo. Escolha outro destino e tente novamente.".into()),
                    Outcome::Page(Err(error),_)|Outcome::Search(Err(error))|Outcome::Detail(Err(error))|Outcome::Revised(Err(error))|Outcome::Preview(Err(error))=>{this.loaded=true;this.error=Some(error);}
                }
                cx.notify();
            });
        }).detach();
    }
    /// Loaded decisions for the command palette: id and question.
    pub fn palette_rows(&self) -> Vec<(String, String)> {
        self.rows
            .iter()
            .map(|row| (row.decision_id.clone(), row.question.clone()))
            .collect()
    }

    /// Opens a loaded decision from the command palette.
    pub fn open_decision(&mut self, id: String, cx: &mut Context<Self>) {
        self.history = false;
        self.version = None;
        self.select(id, cx);
    }

    /// Moves the index selection (loaded rows, or search hits while searching).
    pub fn move_selection(&mut self, delta: isize, window: &mut Window, cx: &mut Context<Self>) {
        if self.editor.is_some() || self.preview.is_some() {
            return;
        }
        let ids: Vec<String> = if self.query.trim().is_empty() {
            self.rows
                .iter()
                .map(|row| row.decision_id.clone())
                .collect()
        } else {
            self.hits
                .iter()
                .map(|hit| hit.decision_id.clone())
                .collect()
        };
        if ids.is_empty() {
            return;
        }
        let current = self
            .selected
            .as_ref()
            .and_then(|id| ids.iter().position(|row| row == id));
        let next = match current {
            Some(index) => (index as isize + delta).clamp(0, ids.len() as isize - 1) as usize,
            None => 0,
        };
        let id = ids[next].clone();
        if let Some(focus) = self.focus.get(&format!("row-{id}")).cloned() {
            window.focus(&focus, cx);
        }
        self.select(id, cx);
    }

    fn act(&mut self, action: Action, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        window.focus(&self.reader_focus, cx);
        self.notice = None;
        match action {
            Action::Select(id) => self.select(id, cx),
            Action::More => {
                if self.cursor.is_some() {
                    self.page(true, cx);
                }
            }
            Action::Retry => self.refresh(cx),
            Action::Document => {
                self.history = false;
                self.version = None;
            }
            Action::History => {
                self.history = true;
                self.version = None;
            }
            Action::Version(version) => {
                self.version = Some(version);
                self.history = false;
            }
            Action::Context(index) => self.contexts[index] = !self.contexts[index],
            Action::Source(index) => {
                self.source = index;
                self.expanded = false;
            }
            Action::ExpandSource => self.expanded = !self.expanded,
            Action::CopySource => {
                if let Some(artifact) = self
                    .sources
                    .get(self.source)
                    .and_then(|source| source.artifact.as_ref())
                {
                    cx.write_to_clipboard(ClipboardItem::new_string(artifact.content.clone()));
                    self.notice = Some("Trecho copiado.".into());
                }
            }
            Action::Relate(kind) => self.relating = kind,
            Action::Link(target) => {
                let (Some(kind), Some(source)) = (self.relating, self.selected.clone()) else {
                    return;
                };
                self.run(cx, move |backend| {
                    let linked = match kind {
                        RelationKind::Supersedes => backend.relations.supersede(&source, &target),
                        _ => backend.relations.relate(&source, &target, kind),
                    };
                    Outcome::Linked(linked.map(|_| relation_notice(kind)).map_err(|error| {
                        match error {
                            application::decisions::DecisionsError::InvalidRelation(reason) => {
                                format!("Relação não permitida: {reason}.")
                            }
                            application::decisions::DecisionsError::Conflict => {
                                "Uma das decisões mudou de estado. A lista foi atualizada.".into()
                            }
                            _ => "Não foi possível registrar a relação.".into(),
                        }
                    }))
                });
            }
            Action::Filter => {
                if self.editor.is_some() || self.preview.is_some() || !self.query.trim().is_empty()
                {
                    return;
                }
                self.all_statuses = !self.all_statuses;
                self.refresh(cx);
            }
            Action::CloseExport => {
                self.preview = None;
                self.overwrite = None;
            }
            Action::Export(format) => {
                if let Some(detail) = &self.detail {
                    if self.version.is_some() {
                        return;
                    }
                    let id = detail.summary.decision_id.clone();
                    self.run(cx, move |backend| {
                        Outcome::Preview(
                            backend
                                .export
                                .preview(&id, format)
                                .map_err(|_| "Não foi possível preparar a exportação.".into()),
                        )
                    });
                }
            }
            Action::SaveExport(overwrite) => {
                let Some(document) = self.preview.clone() else {
                    return;
                };
                let chosen = if overwrite {
                    self.overwrite.clone()
                } else {
                    None
                };
                if overwrite && chosen.is_none() {
                    return;
                }
                self.run(cx, move |backend| {
                    let extension = if document.format == ExportFormat::Markdown {
                        "md"
                    } else {
                        "json"
                    };
                    let destination = chosen.or_else(|| {
                        rfd::FileDialog::new()
                            .set_title("Exportar decisão")
                            .add_filter("Documento", &[extension])
                            .set_file_name(format!(
                                "decisao-{}.{}",
                                document.decision_id, extension
                            ))
                            .save_file()
                    });
                    let Some(path) = destination else {
                        return Outcome::Saved(Ok(None), None);
                    };
                    Outcome::Saved(
                        backend
                            .export
                            .write(&document, &path, overwrite)
                            .map(|result| Some(result.path)),
                        Some(path),
                    )
                });
            }
            Action::Revise => {
                let Some(detail) = self.detail.clone() else {
                    return;
                };
                if self.version.is_some() {
                    return;
                }
                let id = detail.summary.decision_id.clone();
                let project = self.project.clone();
                let expected = detail.summary.version;
                let editor = cx.new(|cx| DecisionEditor::new(&detail, cx));
                window.focus(&editor.read(cx).initial_focus(cx), cx);
                self.editor_subscription=Some(cx.subscribe(&editor,move|this,_,event:&RevisionEvent,cx|{
                    if this.busy{return;}match event{
                        RevisionEvent::Cancel=>{this.editor=None;this.editor_subscription=None;this.restore_focus=true;},
                        RevisionEvent::Save(edits)=>{let id=id.clone();let project=project.clone();let edits=edits.clone();this.run(cx,move|backend|{
                            let result=backend.decisions.revise_version(&id,expected,edits).map_err(|_|"Não foi possível salvar. A decisão pode ter recebido outra versão; cancele e atualize antes de tentar novamente.".to_owned()).and_then(|_|load_document(&backend.decisions,&id,project.as_deref()));Outcome::Revised(result)
                        });}
                    }cx.notify();
                }));
                self.editor = Some(editor);
            }
        }
        cx.notify();
    }
    fn button(
        &mut self,
        id: String,
        label: String,
        action: Action,
        selected: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let t = Theme::current(cx);
        let focus = self
            .focus
            .entry(id.clone())
            .or_insert_with(|| cx.focus_handle().tab_stop(true))
            .clone();
        let key_action = action.clone();
        let primary = matches!(action, Action::Revise | Action::SaveExport(_));
        let disclosure = if let Action::Context(index) = action {
            Some(self.contexts[index])
        } else {
            None
        };
        let counted = matches!(action, Action::History | Action::Context(_));
        let (caption, count) = if counted {
            label
                .rsplit_once('·')
                .map(|(caption, count)| (caption.trim().to_owned(), Some(count.trim().to_owned())))
                .unwrap_or((label.clone(), None))
        } else {
            (label.clone(), None)
        };
        let kind = if primary {
            ButtonKind::Primary
        } else if matches!(action, Action::Export(_) | Action::More | Action::Retry) {
            ButtonKind::Secondary
        } else {
            ButtonKind::Ghost
        };
        let foreground = if selected && kind == ButtonKind::Ghost && !self.busy {
            t.colors.text_primary()
        } else {
            button_foreground(&t, kind, !self.busy)
        };
        let glyph = match &action {
            Action::Document => Some(IconName::Decision),
            Action::History => Some(IconName::Clock),
            Action::Context(0) => Some(IconName::Target),
            Action::Context(1) => Some(IconName::CheckCircle),
            Action::Context(2) => Some(IconName::Activity),
            Action::Context(_) => Some(IconName::Rotate),
            Action::Filter => Some(IconName::Filter),
            Action::Relate(Some(_)) if label == "Relacionar" => Some(IconName::Plus),
            Action::ExpandSource => Some(IconName::Expand),
            Action::Revise => Some(IconName::Edit),
            Action::CopySource => Some(IconName::Copy),
            Action::Export(_) if label == "Exportar…" => Some(IconName::Export),
            _ => None,
        };
        let glyph_icon = glyph.map(|glyph| icon(glyph, 14.0, foreground));
        action_button(&t, id, kind, !self.busy)
            .when(selected && kind == ButtonKind::Ghost, |button| {
                button.bg(t.colors.selection()).text_color(foreground)
            })
            .aria_label(label.clone())
            .aria_selected(selected)
            .track_focus(&focus)
            .on_click(cx.listener(move |this, _, window, cx| this.act(action.clone(), window, cx)))
            .on_key_down(
                cx.listener(move |this, event: &gpui::KeyDownEvent, window, cx| {
                    if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                        this.act(key_action.clone(), window, cx);
                        cx.stop_propagation();
                    }
                }),
            )
            .children(glyph_icon)
            .child(caption)
            .children(count.map(|count| count_chip(&t, count)))
            .when(disclosure.is_some(), |button| {
                button.w_full().child(div().flex_1()).child(icon(
                    if disclosure.unwrap_or(false) {
                        IconName::ChevronDown
                    } else {
                        IconName::ChevronRight
                    },
                    12.0,
                    t.colors.text_muted(),
                ))
            })
            .into_any_element()
    }
    fn index(&mut self, compact: bool, cx: &mut Context<Self>) -> AnyElement {
        let t = Theme::current(cx);
        let searching = !self.query.trim().is_empty();
        let count = if searching {
            self.hits.len()
        } else {
            self.rows.len()
        };
        let filter = self.button(
            "decision-filter".into(),
            if searching {
                "Todos os estados · busca".into()
            } else if self.all_statuses {
                "Todos os estados".into()
            } else {
                "Confirmadas".into()
            },
            Action::Filter,
            self.all_statuses,
            cx,
        );
        let mut list = div()
            .id("decision-index-list")
            .flex_1()
            .min_h(px(0.0))
            .overflow_y_scroll()
            .pb(px(SpacingScale::S3));
        let entries: Vec<IndexEntry> = if searching {
            self.hits
                .iter()
                .map(|hit| IndexEntry {
                    id: hit.decision_id.clone(),
                    question: hit.question.clone(),
                    meta: hit.snippet.replace(['[', ']'], ""),
                    month: String::new(),
                    version: None,
                    status: None,
                })
                .collect()
        } else {
            self.rows
                .iter()
                .map(|row| IndexEntry {
                    id: row.decision_id.clone(),
                    question: row.question.clone(),
                    meta: short_date(&row.confirmed_at),
                    month: month_label(&row.confirmed_at),
                    version: Some(row.version),
                    status: Some(row.status),
                })
                .collect()
        };
        let mut group = String::new();
        for IndexEntry {
            id,
            question,
            meta,
            month,
            version,
            status,
        } in entries
        {
            if month != group {
                group = month.clone();
                list = list.child(
                    section_label(&t, &month)
                        .px(px(SpacingScale::S4))
                        .pt(px(SpacingScale::S4))
                        .pb(px(SpacingScale::S2)),
                );
            }
            let active = self.selected.as_deref() == Some(&id);
            let focus = self
                .focus
                .entry(format!("row-{id}"))
                .or_insert_with(|| cx.focus_handle().tab_stop(true))
                .clone();
            let key_id = id.clone();
            let hovered = self.hovered.as_deref() == Some(id.as_str());
            let hover_key = id.clone();
            let hover_id = gpui::ElementId::Name(format!("decision-hover-{id}").into());
            let row = div()
                .id(format!("decision-{id}"))
                .relative()
                .px(px(SpacingScale::S4))
                .py(px(SpacingScale::S3))
                .flex()
                .flex_col()
                .gap(px(SpacingScale::S1))
                .on_hover(cx.listener(move |this, hovered: &bool, _, cx| {
                    if track_hover(&mut this.hovered, hover_key.clone(), *hovered) {
                        cx.notify();
                    }
                }))
                .role(Role::Button)
                .aria_label(question.clone())
                .aria_selected(active)
                .track_focus(&focus)
                .focus_visible(focus_ring(&t))
                .cursor_pointer()
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.act(Action::Select(id.clone()), window, cx)
                }))
                .on_key_down(
                    cx.listener(move |this, event: &gpui::KeyDownEvent, window, cx| {
                        if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                            this.act(Action::Select(key_id.clone()), window, cx);
                            cx.stop_propagation();
                        }
                    }),
                );
            let row = mark_selected(row, &t, active)
                // Date and version lead; the state only appears when it
                // differs from the confirmed default the index is filtered to.
                .children(status.map(|status| {
                    div()
                        .flex()
                        .items_center()
                        .gap(px(SpacingScale::S2))
                        .child(
                            text_style(div(), TypeScale::META)
                                .text_color(t.colors.text_muted())
                                .child(meta.clone()),
                        )
                        .child(div().flex_1())
                        .when(status != DecisionStatus::Accepted, |line| {
                            line.child(status_pill(&t, t.colors.text_muted(), "Substituída"))
                        })
                        .children(version.map(|version| count_chip(&t, format!("v{version}"))))
                }))
                .child(
                    word_wrapped(&question, TypeScale::ROW_TITLE, Some(2))
                        .text_color(t.colors.text_primary()),
                )
                .when(status.is_none(), |row| {
                    row.child(
                        text_style(div(), TypeScale::META)
                            .text_color(t.colors.text_muted())
                            .child(meta),
                    )
                });
            list = list.child(hover_tint(row, hover_id, hovered, !active, &t));
        }
        if self.cursor.is_some() && !searching {
            list = list.child(self.button(
                "more-decisions".into(),
                "Carregar mais".into(),
                Action::More,
                false,
                cx,
            ));
        }
        if self.busy && count == 0 {
            list = list.child(skeleton_list(&t, "decision-skeleton", 4));
        }
        if self.loaded && count == 0 {
            list = list.child(
                text_style(div(), TypeScale::BODY_SMALL)
                    .p(px(SpacingScale::S3))
                    .text_color(t.colors.text_muted())
                    .child(if searching {
                        "Nenhum resultado. Tente outra palavra."
                    } else {
                        "As decisões confirmadas aparecerão aqui."
                    }),
            );
        }
        div()
            .w(px(if compact { 248.0 } else { 296.0 }))
            .flex_none()
            .h_full()
            .flex()
            .flex_col()
            .bg(t.colors.pane())
            .border_r_1()
            .border_color(t.colors.hairline_divider())
            .child(
                div()
                    .px(px(SpacingScale::S4))
                    .pt(px(SpacingScale::S3))
                    .pb(px(SpacingScale::S2))
                    .flex()
                    .flex_col()
                    .gap(px(SpacingScale::S3))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(SpacingScale::S2))
                            .child(panel_title(&t, "Índice"))
                            .child(count_chip(&t, count.to_string())),
                    )
                    .when(self.editor.is_none() && self.preview.is_none(), |header| {
                        header.child(self.search.clone()).child(filter)
                    })
                    .when(self.editor.is_some() || self.preview.is_some(), |header| {
                        header.child(
                            text_style(div(), TypeScale::BODY_SMALL)
                                .text_color(t.colors.text_muted())
                                .child("Conclua ou cancele para navegar no índice."),
                        )
                    }),
            )
            .child(list)
            .child(
                text_style(div(), TypeScale::META)
                    .flex_none()
                    .px(px(SpacingScale::S4))
                    .py(px(SpacingScale::S2))
                    .border_t_1()
                    .border_color(t.colors.hairline_divider())
                    .text_color(t.colors.text_muted())
                    .child(if self.busy {
                        "Carregando…".into()
                    } else if searching {
                        format!(
                            "{count} {} em pergunta, escolha e justificativa",
                            if count == 1 {
                                "resultado"
                            } else {
                                "resultados"
                            }
                        )
                    } else {
                        format!("{count} carregadas · ordem de confirmação")
                    }),
            )
            .into_any_element()
    }
    fn document(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let t = Theme::current(cx);
        let Some(mut detail) = self.detail.clone() else {
            if self.busy || !self.loaded {
                return div().into_any_element();
            }
            return if self.query.trim().is_empty() {
                crate::ui::patterns::empty_panel_mascot(
                    &t,
                    crate::screens::assistant::portrait(),
                    "Decisões",
                    "Decisões que permanecem",
                    "Confirme uma escolha na Revisão para preservar o documento, suas evidências e seu histórico aqui.",
                )
            } else {
                empty_panel(
                    &t,
                    IconName::Search,
                    "Busca",
                    "Nenhuma decisão encontrada",
                    "A busca cobre pergunta, escolha e justificativa deste projeto. Tente outra palavra.",
                )
            }
            .min_h(px(420.0))
            .into_any_element();
        };
        if self.history {
            let back = self.button(
                "document-back".into(),
                "Voltar ao documento".into(),
                Action::Document,
                false,
                cx,
            );
            let mut history = div()
                .w_full()
                .max_w(px(READING_WIDTH))
                .mx_auto()
                .flex()
                .flex_col()
                .gap(px(SpacingScale::S4))
                .child(div().flex().child(back))
                .child(text_style(div(), TypeScale::HEADING_1).child("Histórico de versões"))
                .child(text_style(div(),TypeScale::BODY_SMALL).text_color(t.colors.text_muted()).child("Cada versão conserva o documento completo. Abra uma versão para ler seu conteúdo."));
            for revision in &detail.revisions {
                history = history.child(
                    div()
                        .p(px(SpacingScale::S4))
                        .border_1()
                        .border_color(t.colors.hairline_divider())
                        .rounded(t.radius.surface())
                        .child(self.button(
                            format!("version-{}", revision.version),
                            format!(
                                "v{} · {}{}",
                                revision.version,
                                short_date(&revision.created_at),
                                if revision.version == detail.summary.version {
                                    " · atual"
                                } else {
                                    ""
                                }
                            ),
                            Action::Version(revision.version),
                            false,
                            cx,
                        ))
                        .child(
                            text_style(div(), TypeScale::HEADING_3)
                                .mt(px(SpacingScale::S3))
                                .child(revision.question.clone()),
                        )
                        .child(
                            text_style(div(), TypeScale::BODY_SMALL)
                                .mt(px(SpacingScale::S2))
                                .text_color(t.colors.text_muted())
                                .child(revision.choice.clone()),
                        ),
                );
            }
            return history.into_any_element();
        }
        if let Some(version) = self.version {
            if let Some(revision) = detail
                .revisions
                .iter()
                .find(|revision| revision.version == version)
            {
                detail.summary.question = revision.question.clone();
                detail.summary.choice = revision.choice.clone();
                detail.rationale = revision.rationale.clone();
                detail.assumptions = revision.assumptions.clone();
                detail.scope = revision.scope.clone();
                detail.qualifiers = revision.qualifiers.clone();
                detail.consequences = revision.consequences.clone();
                detail.reconsider_when = revision.reconsider_when.clone();
                detail.summary.version = revision.version;
                detail.summary.updated_at = revision.created_at.clone();
            }
        }
        let current = self.version.is_none();
        let revisions = detail.revisions.len();
        let history_link = self.button(
            "history-open".into(),
            format!(
                "{} · {revisions}",
                if revisions == 1 {
                    "Versão"
                } else {
                    "Versões"
                }
            ),
            Action::History,
            false,
            cx,
        );
        let actions: Vec<AnyElement> = if current {
            vec![
                self.button(
                    "export-open".into(),
                    "Exportar…".into(),
                    Action::Export(ExportFormat::Markdown),
                    false,
                    cx,
                ),
                self.button(
                    "revise-open".into(),
                    "Revisar".into(),
                    Action::Revise,
                    true,
                    cx,
                ),
            ]
        } else {
            vec![self.button(
                "current-version".into(),
                "Voltar à versão atual".into(),
                Action::Document,
                false,
                cx,
            )]
        };
        let badge = if detail.summary.status == DecisionStatus::Accepted {
            status_pill(&t, t.colors.status_success(), "Confirmada")
        } else {
            status_pill(&t, t.colors.text_muted(), "Substituída")
        };
        let mut document=div().w_full().max_w(px(READING_WIDTH)).mx_auto().flex().flex_col().gap(px(SpacingScale::S6))
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap(px(SpacingScale::S2))
                    .child(badge)
                    .child(
                        text_style(div(), TypeScale::META)
                            .text_color(t.colors.text_muted())
                            .child(format!(
                                "v{} · {}",
                                detail.summary.version,
                                short_date(&detail.summary.updated_at)
                            )),
                    )
                    .child(history_link)
                    .child(div().flex_1())
                    .children(actions),
            )
            .when(!current, |view| {
                view.child(
                    text_style(div(), TypeScale::BODY_SMALL)
                        .px(px(SpacingScale::S3))
                        .py(px(SpacingScale::S2))
                        .rounded(t.radius.control())
                        .bg(t.colors.selection())
                        .child("Versão histórica, somente leitura. Revisar e exportar usam a versão atual."),
                )
            })
            .child(reading_title(&detail.summary.question))
            .child(div().flex().flex_col().gap(px(SpacingScale::S2)).border_l_2().border_color(t.colors.accent_hover()).pl(px(SpacingScale::S4))
                .child(section_label(&t,"Escolha confirmada").text_color(t.colors.accent_hover()))
                .child(text_style(div(),TypeScale::HEADING_2).child(detail.summary.choice.clone())))
            .child(div().flex().flex_col().gap(px(SpacingScale::S2)).child(section_label(&t,"Justificativa"))
                .child(text_style(div(),TypeScale::BODY).text_color(t.colors.text_secondary()).child(detail.rationale.clone())));
        document = document.child(super::review_editor::qualifier_reading(
            &t,
            &detail.qualifiers,
        ));
        // One disclosure per row: opening one never reflows the others.
        let mut context = div()
            .flex()
            .flex_col()
            .gap(px(2.0))
            .border_t_1()
            .border_color(t.colors.hairline_divider())
            .pt(px(SpacingScale::S5));
        for (index, (label, items)) in [
            ("Escopo", &detail.scope),
            ("Premissas", &detail.assumptions),
            ("Consequências", &detail.consequences),
            ("Reconsiderar quando", &detail.reconsider_when),
        ]
        .into_iter()
        .enumerate()
        {
            let mut section = div()
                .w_full()
                .flex()
                .flex_col()
                .gap(px(SpacingScale::S1))
                .child(self.button(
                    format!("context-{index}"),
                    format!("{label} · {}", items.len()),
                    Action::Context(index),
                    false,
                    cx,
                ));
            if self.contexts[index] {
                if items.is_empty() {
                    section = section.child(
                        text_style(div(), TypeScale::BODY_SMALL)
                            .pl(px(SpacingScale::S8))
                            .pb(px(SpacingScale::S2))
                            .text_color(t.colors.text_muted())
                            .child("Nenhum item registrado."),
                    );
                } else {
                    section = section.children(items.iter().map(|item| {
                        text_style(div(), TypeScale::BODY_SMALL)
                            .pl(px(SpacingScale::S8))
                            .text_color(t.colors.text_secondary())
                            .child(format!("• {item}"))
                    }));
                    section = section.child(div().h(px(SpacingScale::S2)));
                }
            }
            context = context.child(section);
        }
        document = document
            .child(
                div()
                    .border_t_1()
                    .border_color(t.colors.hairline_divider())
                    .pt(px(SpacingScale::S4))
                    .flex()
                    .flex_col()
                    .gap(px(SpacingScale::S3))
                    .child(section_header(&t, "Contexto da decisão"))
                    .child(context.border_t_0().pt(px(0.0))),
            )
            .child(self.evidence(cx))
            .child(self.relations_section(cx));
        document = document.child(
            div()
                .border_t_1()
                .border_color(t.colors.hairline_divider())
                .pt(px(SpacingScale::S4))
                .flex()
                .flex_col()
                .gap(px(SpacingScale::S1))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(SpacingScale::S2))
                        .child(section_label(&t, "Origem")),
                )
                .child(
                    text_style(div(), TypeScale::META)
                        .text_color(t.colors.text_secondary())
                        .child(if detail.provenance.capture_id.is_some() {
                            "Confirmada na Revisão a partir de uma conversa capturada."
                        } else {
                            "Confirmada na Revisão; a captura de origem não foi registrada."
                        }),
                )
                .child(
                    text_style(div(), TypeScale::META)
                        .text_color(t.colors.text_muted())
                        .child(detail.provenance.project_location.clone()),
                ),
        );
        document.into_any_element()
    }
    fn relations_section(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let t = Theme::current(cx);
        let colors = t.colors;
        let accepted = self
            .detail
            .as_ref()
            .is_some_and(|detail| detail.summary.status == DecisionStatus::Accepted);
        let links = self.links.clone().unwrap_or_default();
        let open = (accepted && self.relating.is_none()).then(|| {
            self.button(
                "relate-open".into(),
                "Relacionar".into(),
                Action::Relate(Some(RelationKind::DependsOn)),
                false,
                cx,
            )
        });
        let header = section_header(&t, "Relações")
            .when(!links.is_empty(), |row| {
                row.child(count_chip(&t, links.len().to_string()))
            })
            .children(open);
        let mut panel = div()
            .border_t_1()
            .border_color(colors.hairline_divider())
            .pt(px(SpacingScale::S4))
            .flex()
            .flex_col()
            .gap(px(SpacingScale::S2))
            .child(header);
        if links.is_empty() && self.relating.is_none() {
            panel = panel.child(
                text_style(div(), TypeScale::BODY_SMALL)
                    .text_color(colors.text_muted())
                    .child(if accepted {
                        "Sem relações. Ligue esta decisão às que ela substitui, das quais \
                         depende ou com que conflita."
                    } else {
                        "Sem relações registradas."
                    }),
            );
        }
        let rows: Vec<AnyElement> = links
            .iter()
            .enumerate()
            .map(|(index, link)| {
                let target = link.other_id.clone();
                div()
                    .id(("relation", index))
                    .flex()
                    .items_center()
                    .gap(px(SpacingScale::S3))
                    .px(px(SpacingScale::S3))
                    .py(px(SpacingScale::S2))
                    .rounded(t.radius.control())
                    .border_1()
                    .border_color(colors.hairline_divider())
                    .cursor_pointer()
                    .hover(move |style| style.bg(colors.glass_fill_medium()))
                    .active(move |style| style.bg(colors.glass_fill_strong()))
                    .role(Role::Button)
                    .aria_label(format!(
                        "{}: {}",
                        relation_label(link.kind, link.direction),
                        link.question
                    ))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.act(Action::Select(target.clone()), window, cx)
                    }))
                    .child(
                        text_style(div(), TypeScale::META)
                            .w(px(112.0))
                            .flex_none()
                            .text_color(relation_color(&t, link.kind))
                            .child(relation_label(link.kind, link.direction)),
                    )
                    .child(
                        text_style(div(), TypeScale::BODY_SMALL)
                            .flex_1()
                            .min_w(px(0.0))
                            .truncate()
                            .child(link.question.clone()),
                    )
                    .when(link.superseded, |row| {
                        row.child(status_pill(&t, colors.text_muted(), "Substituída"))
                    })
                    .child(icon(IconName::ChevronRight, 14.0, colors.text_muted()))
                    .into_any_element()
            })
            .collect();
        panel = panel.children(rows);
        if let Some(kind) = self.relating {
            let picker = self.relation_picker(kind, &links, cx);
            panel = panel.child(picker);
        }
        panel.into_any_element()
    }
    fn relation_picker(
        &mut self,
        kind: RelationKind,
        links: &[LinkItem],
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let t = Theme::current(cx);
        let colors = t.colors;
        let current = self.selected.clone().unwrap_or_default();
        let candidates: Vec<(String, String)> = self
            .rows
            .iter()
            .filter(|row| {
                row.decision_id != current
                    && row.status == DecisionStatus::Accepted
                    && !links
                        .iter()
                        .any(|link| link.other_id == row.decision_id && link.kind == kind)
            })
            .map(|row| (row.decision_id.clone(), row.question.clone()))
            .collect();
        let hint = match kind {
            RelationKind::DependsOn => "Esta decisão só vale enquanto a escolhida valer.",
            RelationKind::ConflictsWith => {
                "As duas não podem valer juntas; a relação aparece nas duas."
            }
            RelationKind::Supersedes => {
                "A escolhida passa a Substituída, sai do contexto do agente e fica no histórico."
            }
        };
        let mut chips: Vec<AnyElement> = Vec::new();
        for (option, id, label) in [
            (RelationKind::DependsOn, "relate-kind-depends", "Depende de"),
            (
                RelationKind::ConflictsWith,
                "relate-kind-conflicts",
                "Conflita com",
            ),
            (
                RelationKind::Supersedes,
                "relate-kind-supersedes",
                "Substitui",
            ),
        ] {
            chips.push(self.button(
                id.into(),
                label.into(),
                Action::Relate(Some(option)),
                option == kind,
                cx,
            ));
        }
        let cancel = self.button(
            "relate-cancel".into(),
            "Cancelar".into(),
            Action::Relate(None),
            false,
            cx,
        );
        let list = if candidates.is_empty() {
            text_style(div(), TypeScale::BODY_SMALL)
                .text_color(colors.text_secondary())
                .child("Nenhuma outra decisão em vigor carregada para relacionar.")
                .into_any_element()
        } else {
            div()
                .id("relate-candidates")
                .max_h(px(240.0))
                .overflow_y_scroll()
                .flex()
                .flex_col()
                .role(Role::List)
                .children(
                    candidates
                        .into_iter()
                        .enumerate()
                        .map(|(index, (id, question))| {
                            div()
                                .id(("relate-candidate", index))
                                .flex()
                                .items_center()
                                .gap(px(SpacingScale::S2))
                                .px(px(SpacingScale::S2))
                                .py(px(SpacingScale::S2))
                                .rounded(t.radius.control())
                                .cursor_pointer()
                                .hover(move |style| style.bg(colors.glass_fill_medium()))
                                .active(move |style| style.bg(colors.glass_fill_strong()))
                                .role(Role::Button)
                                .aria_label(format!("Relacionar com: {question}"))
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    this.act(Action::Link(id.clone()), window, cx)
                                }))
                                .child(icon(IconName::Decision, 14.0, colors.text_muted()))
                                .child(
                                    text_style(div(), TypeScale::BODY_SMALL)
                                        .flex_1()
                                        .min_w(px(0.0))
                                        .truncate()
                                        .child(question),
                                )
                        }),
                )
                .into_any_element()
        };
        div()
            .p(px(SpacingScale::S3))
            .flex()
            .flex_col()
            .gap(px(SpacingScale::S3))
            .rounded(t.radius.surface())
            .border_1()
            .border_color(if kind == RelationKind::Supersedes {
                colors.status_warning()
            } else {
                colors.glass_border_card()
            })
            .bg(colors.glass_fill_card())
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(SpacingScale::S1))
                    .id("relate-kinds")
                    .role(Role::RadioGroup)
                    .aria_label("Tipo de relação")
                    .children(chips)
                    .child(div().flex_1())
                    .child(cancel),
            )
            .child(
                text_style(div(), TypeScale::META)
                    .text_color(colors.text_muted())
                    .child(hint),
            )
            .child(list)
            .into_any_element()
    }
    fn evidence(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let t = Theme::current(cx);
        let header = section_header(&t, "Evidências").child(count_chip(
            &t,
            match self.sources.len() {
                1 => "1 fonte".to_string(),
                n => format!("{n} fontes"),
            },
        ));
        let panel = div()
            .flex()
            .flex_col()
            .gap(px(SpacingScale::S3))
            .child(header);
        if self.sources.is_empty() {
            return panel
                .child(
                    text_style(div(), TypeScale::BODY_SMALL)
                        .text_color(t.colors.text_muted())
                        .child("Nenhuma fonte vinculada."),
                )
                .into_any_element();
        }
        let mut tabs = evidence::tab_strip(&t, "decision-source-tabs");
        for index in 0..self.sources.len() {
            let focus = self
                .focus
                .entry(format!("source-{index}"))
                .or_insert_with(|| cx.focus_handle().tab_stop(true))
                .clone();
            let tab = match &self.sources[index].artifact {
                Some(artifact) => evidence::tab(
                    &t,
                    ("decision-source", index),
                    artifact,
                    self.source == index,
                ),
                None => evidence::tab(
                    &t,
                    ("decision-source", index),
                    &application::inbox::ArtifactView {
                        artifact_id: self.sources[index].link.artifact_id.clone(),
                        kind: String::new(),
                        content: String::new(),
                        metadata: String::new(),
                    },
                    self.source == index,
                ),
            };
            tabs = tabs.child(
                tab.track_focus(&focus)
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.act(Action::Source(index), window, cx)
                    }))
                    .on_key_down(cx.listener(
                        move |this, event: &gpui::KeyDownEvent, window, cx| {
                            if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                                this.act(Action::Source(index), window, cx);
                                cx.stop_propagation();
                            }
                        },
                    )),
            );
        }
        let mut frame = evidence::frame(&t).child(tabs);
        let artifact = self
            .sources
            .get(self.source)
            .and_then(|source| source.artifact.clone());
        let copy = self.button(
            "copy-source".into(),
            "Copiar trecho".into(),
            Action::CopySource,
            false,
            cx,
        );
        let expand = self.button(
            "expand-source".into(),
            if self.expanded {
                "Recolher".into()
            } else {
                "Ampliar leitura".into()
            },
            Action::ExpandSource,
            self.expanded,
            cx,
        );
        let lines = self.lines.get(self.source).and_then(Option::as_ref);
        match (artifact, lines) {
            (Some(artifact), Some(lines)) => {
                let caption = evidence::caption_row(&t, &artifact, lines)
                    .child(copy)
                    .child(expand);
                frame = frame.child(caption).child(evidence::body(
                    &artifact,
                    format!(
                        "decision-code-{}-{}",
                        self.selected.as_deref().unwrap_or(""),
                        self.source
                    ),
                    lines,
                    if self.expanded { 480.0 } else { 280.0 },
                    t,
                ));
            }
            _ => {
                frame = frame.child(
                    text_style(div(), TypeScale::BODY_SMALL)
                        .p(px(SpacingScale::S5))
                        .text_color(t.colors.text_muted())
                        .child("Fonte indisponível. O vínculo de proveniência foi preservado."),
                );
            }
        }
        panel.child(frame).into_any_element()
    }
    fn export_preview(&mut self, max_height: f32, cx: &mut Context<Self>) -> AnyElement {
        let t = Theme::current(cx);
        let document = self.preview.clone().expect("preview mounted");
        let json = document.format == ExportFormat::Json;
        let mut panel = div().size_full().flex().flex_col().child(
            div()
                .h(px(48.0))
                .flex_none()
                .px(px(SpacingScale::S4))
                .flex()
                .items_center()
                .gap(px(SpacingScale::S1))
                .border_b_1()
                .border_color(t.colors.hairline_divider())
                .child(
                    text_style(div(), TypeScale::HEADING_3)
                        .flex_1()
                        .child("Prévia da exportação"),
                )
                .child(self.button(
                    "export-close".into(),
                    "Cancelar".into(),
                    Action::CloseExport,
                    false,
                    cx,
                ))
                .child(self.button(
                    "export-md".into(),
                    "Markdown".into(),
                    Action::Export(ExportFormat::Markdown),
                    !json,
                    cx,
                ))
                .child(self.button(
                    "export-json".into(),
                    "JSON".into(),
                    Action::Export(ExportFormat::Json),
                    json,
                    cx,
                )),
        );
        panel = panel.child(
            div()
                .px(px(SpacingScale::S6))
                .py(px(SpacingScale::S3))
                .flex()
                .items_center()
                .gap(px(SpacingScale::S3))
                .child(
                    text_style(div(), TypeScale::META)
                        .flex_1()
                        .text_color(t.colors.text_muted())
                        .child(format!(
                            "{} bytes · conteúdo exato da versão atual",
                            document.bytes
                        )),
                )
                .child(self.button(
                    "export-save".into(),
                    "Escolher destino…".into(),
                    Action::SaveExport(false),
                    true,
                    cx,
                )),
        );
        if let Some(path) = &self.overwrite {
            panel = panel.child(
                div()
                    .p(px(SpacingScale::S4))
                    .bg(t.colors.selection())
                    .child(
                        text_style(div(), TypeScale::BODY_SMALL)
                            .mb(px(SpacingScale::S2))
                            .child(format!(
                                "O arquivo {} já existe. Substituir seu conteúdo?",
                                path.display()
                            )),
                    )
                    .child(self.button(
                        "export-overwrite".into(),
                        "Substituir arquivo".into(),
                        Action::SaveExport(true),
                        true,
                        cx,
                    )),
            );
        }
        let source = application::inbox::ArtifactView {
            artifact_id: "export".into(),
            kind: "export_document".into(),
            content: document.content,
            metadata: format!(
                "{{\"file\":\"decisao.{}\"}}",
                if json { "json" } else { "md" }
            ),
        };
        let lines = SourceLines::new(&source);
        panel
            .child(
                div().flex_1().min_h(px(0.0)).p(px(SpacingScale::S6)).child(
                    evidence::frame(&t)
                        .child(evidence::caption_row(&t, &source, &lines))
                        .child(evidence::body(
                            &source,
                            "export-preview-content".into(),
                            &lines,
                            max_height,
                            t,
                        )),
                ),
            )
            .into_any_element()
    }
}
impl<S: DecisionStore + InboxStore + RelationStore + Send + 'static> Render for DecisionsScreen<S> {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.restore_focus {
            self.restore_focus = false;
            window.focus(&self.reader_focus, cx);
        }
        let t = Theme::current(cx);
        let compact = window.viewport_size().width < px(1300.0);
        self.search.update(cx, |field, _| {
            field.set_width(if compact { 208.0 } else { 256.0 })
        });
        let content = if let Some(editor) = &self.editor {
            div()
                .size_full()
                .flex()
                .flex_col()
                .child(div().flex_1().min_h(px(0.0)).child(editor.clone()))
                .child(self.evidence(cx))
                .into_any_element()
        } else if self.preview.is_some() {
            self.export_preview(
                (f32::from(window.viewport_size().height) - 450.0).max(140.0),
                cx,
            )
        } else {
            let document = self.document(cx);
            div()
                .size_full()
                .flex()
                .flex_col()
                .child(
                    div()
                        .id(format!(
                            "decision-reader-{}-{:?}-{}",
                            self.selected.as_deref().unwrap_or("empty"),
                            self.version,
                            self.history
                        ))
                        .flex_1()
                        .min_h(px(0.0))
                        .overflow_y_scroll()
                        .px(px(if compact {
                            SpacingScale::S6
                        } else {
                            SpacingScale::S8
                        }))
                        .py(px(SpacingScale::S8))
                        .child(fade_in(
                            div().child(document),
                            gpui::ElementId::Name(
                                format!(
                                    "decision-doc-{}-{:?}-{}",
                                    self.selected.as_deref().unwrap_or("empty"),
                                    self.version,
                                    self.history
                                )
                                .into(),
                            ),
                        )),
                )
                .into_any_element()
        };
        let mut reader = div().flex_1().min_w(px(0.0)).h_full().flex().flex_col();
        if let Some(error) = self.error.clone() {
            reader = reader.child(error_banner(&t, &error).when(
                self.editor.is_none() && self.preview.is_none(),
                |bar| {
                    bar.child(self.button(
                        "decisions-retry".into(),
                        "Tentar novamente".into(),
                        Action::Retry,
                        false,
                        cx,
                    ))
                },
            ));
        }
        if self.notice.is_some() && self.notice != self.notice_scheduled {
            self.notice_scheduled = self.notice.clone();
            let shown = self.notice.clone();
            cx.spawn(async move |this, cx| {
                cx.background_executor().timer(TOAST_DURATION).await;
                let _ = this.update(cx, |screen, cx| {
                    if screen.notice == shown {
                        screen.notice = None;
                        screen.notice_scheduled = None;
                        cx.notify();
                    }
                });
            })
            .detach();
        }
        if let Some(notice) = &self.notice {
            reader = reader.relative().child(toast(&t, notice, SpacingScale::S6));
        }
        reader = reader.child(div().flex_1().min_h(px(0.0)).child(content));
        div()
            .id("decisions-screen")
            .track_focus(&self.reader_focus)
            .size_full()
            .flex()
            .child(self.index(compact, cx))
            .child(reader)
    }
}
fn load_document<S: DecisionStore + InboxStore>(
    decisions: &Decisions<S>,
    id: &str,
    project: Option<&str>,
) -> Result<(DecisionDetail, Vec<DecisionSource>), String> {
    let detail = decisions
        .detail(id)
        .map_err(|_| "Não foi possível abrir esta decisão.".to_owned())?;
    if Some(detail.summary.project_id.as_str()) != project {
        return Err("Esta decisão não pertence ao projeto selecionado.".into());
    }
    let sources = decisions
        .sources(&detail)
        .map_err(|_| "Não foi possível carregar as fontes desta decisão.".to_owned())?;
    Ok((detail, sources))
}
fn month_label(value: &str) -> String {
    let months = [
        "Janeiro",
        "Fevereiro",
        "Março",
        "Abril",
        "Maio",
        "Junho",
        "Julho",
        "Agosto",
        "Setembro",
        "Outubro",
        "Novembro",
        "Dezembro",
    ];
    use chrono::Datelike;
    match chrono::DateTime::parse_from_rfc3339(value) {
        Ok(date) => {
            let date = date.with_timezone(&chrono::Local);
            format!("{} {}", months[date.month0() as usize], date.year())
        }
        Err(_) => "Data não registrada".into(),
    }
}

/// How a relation reads from the open decision.
fn relation_label(kind: RelationKind, direction: RelationDirection) -> &'static str {
    match (kind, direction) {
        (RelationKind::Supersedes, RelationDirection::Outgoing) => "Substitui",
        (RelationKind::Supersedes, RelationDirection::Incoming) => "Substituída por",
        (RelationKind::DependsOn, RelationDirection::Outgoing) => "Depende de",
        (RelationKind::DependsOn, RelationDirection::Incoming) => "É base de",
        (RelationKind::ConflictsWith, _) => "Conflita com",
    }
}

fn relation_color(theme: &Theme, kind: RelationKind) -> gpui::Rgba {
    match kind {
        RelationKind::Supersedes => theme.colors.accent_hover(),
        RelationKind::DependsOn => theme.colors.status_info(),
        RelationKind::ConflictsWith => theme.colors.status_warning(),
    }
}

fn relation_notice(kind: RelationKind) -> &'static str {
    match kind {
        RelationKind::Supersedes => "Decisão substituída. A anterior segue no histórico.",
        RelationKind::DependsOn => "Dependência registrada.",
        RelationKind::ConflictsWith => "Conflito registrado nas duas decisões.",
    }
}
