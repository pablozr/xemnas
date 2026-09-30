//! Versioned decision documents with an independent chronological index.
use super::{
    decision_editor::{DecisionEditor, RevisionEvent},
    evidence::{self, SourceLines},
};
use crate::ui::{
    glass::focus_ring,
    icons::Icon,
    search_field::{SearchChanged, SearchField},
    theme::{text_style, Theme},
    tokens::TypeScale,
};
use application::decisions::{
    DecisionDetail, DecisionFilter, DecisionPage, DecisionSearchHit, DecisionSource,
    DecisionStatus, DecisionStore, DecisionSummary, Decisions, SearchQuery,
};
use application::export::{Export, ExportDocument, ExportError, ExportFormat};
use application::inbox::InboxStore;
use gpui::prelude::*;
use gpui::{
    div, px, AnyElement, ClipboardItem, Context, Entity, FocusHandle, Focusable, Render, Role,
    Subscription, Window,
};
use std::{collections::BTreeMap, path::PathBuf, time::Duration};

struct Backend<S> {
    decisions: Decisions<S>,
    export: Export<S>,
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
}

/// All storage work runs off the UI thread; project/query generations reject stale reads.
pub struct DecisionsScreen<S: DecisionStore + InboxStore + Send + 'static> {
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
    focus: BTreeMap<String, FocusHandle>,
    reader_focus: FocusHandle,
    restore_focus: bool,
}
impl<S: DecisionStore + InboxStore + Send + 'static> DecisionsScreen<S> {
    /// Mounts the persistent document reader and its project-scoped search.
    pub fn new(cx: &mut Context<Self>, decisions: Decisions<S>, export: Export<S>) -> Self {
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
            backend: Some(Backend { decisions, export }),
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
            focus: BTreeMap::new(),
            reader_focus: cx.focus_handle(),
            restore_focus: false,
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
                    Outcome::Detail(Ok((detail,sources)))=>this.install(detail,sources),
                    Outcome::Revised(Ok((detail,sources)))=>{for row in &mut this.rows{if row.decision_id==detail.summary.decision_id{*row=detail.summary.clone();}}
                        this.editor=None;this.editor_subscription=None;this.restore_focus=true;this.install(detail,sources);this.notice=Some("Nova versão salva. Histórico preservado.".into());}
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
        let icon = match &action {
            Action::Document => Some(Icon::file(&t, 14.0, !selected).into_any_element()),
            Action::History => Some(Icon::clock(&t, 14.0, !selected).into_any_element()),
            Action::Context(0) => Some(Icon::folder(&t, 14.0, true).into_any_element()),
            Action::Context(1) => Some(Icon::layers(&t, 14.0).into_any_element()),
            Action::Context(2) => Some(Icon::activity(&t, 14.0).into_any_element()),
            Action::Context(_) => Some(Icon::clock(&t, 14.0, true).into_any_element()),
            Action::Filter => Some(Icon::filter(&t, 14.0).into_any_element()),
            Action::ExpandSource => Some(Icon::expand(&t, 14.0).into_any_element()),
            Action::Revise => Some(Icon::edit(&t, 14.0).into_any_element()),
            Action::CopySource => Some(Icon::copy(&t, 14.0).into_any_element()),
            Action::Export(_) if label == "Exportar…" => {
                Some(Icon::export(&t, 14.0).into_any_element())
            }
            _ => None,
        };
        text_style(div(), TypeScale::BODY_SMALL)
            .id(id)
            .h(px(32.0))
            .px(px(10.0))
            .flex()
            .items_center()
            .gap(px(5.0))
            .rounded(px(6.0))
            .border_1()
            .border_color(if primary {
                t.colors.decision_accent()
            } else if matches!(action, Action::Export(_) | Action::More | Action::Retry) {
                t.colors.decision_line()
            } else {
                t.colors.canvas()
            })
            .bg(if primary {
                t.colors.decision_accent()
            } else if selected {
                t.colors.decision_selected()
            } else {
                t.colors.canvas()
            })
            .text_color(if self.busy {
                t.colors.text_disabled()
            } else if primary {
                t.colors.accent_on_emphasis()
            } else {
                t.colors.text_secondary()
            })
            .role(Role::Button)
            .aria_label(label.clone())
            .aria_selected(selected)
            .track_focus(&focus)
            .focus_visible(focus_ring(&t))
            .cursor_pointer()
            .hover(move |style| {
                style.bg(if primary {
                    t.colors.accent_emphasis()
                } else {
                    t.colors.decision_selected()
                })
            })
            .on_click(cx.listener(move |this, _, window, cx| this.act(action.clone(), window, cx)))
            .on_key_down(
                cx.listener(move |this, event: &gpui::KeyDownEvent, window, cx| {
                    if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                        this.act(key_action.clone(), window, cx);
                        cx.stop_propagation();
                    }
                }),
            )
            .children(icon)
            .child(caption)
            .children(count.map(|count| {
                text_style(div(), TypeScale::META)
                    .px(px(5.0))
                    .rounded(px(4.0))
                    .bg(t.colors.decision_layer())
                    .text_color(t.colors.text_muted())
                    .child(count)
            }))
            .when(disclosure.is_some(), |button| {
                button
                    .w_full()
                    .child(div().flex_1())
                    .child(Icon::disclosure(&t, 12.0, disclosure.unwrap_or(false)))
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
            .p(px(12.0));
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
                    text_style(div(), TypeScale::META)
                        .px(px(9.0))
                        .pt(px(20.0))
                        .pb(px(9.0))
                        .text_color(t.colors.text_muted())
                        .child(month),
                );
            }
            let active = self.selected.as_deref() == Some(&id);
            let focus = self
                .focus
                .entry(format!("row-{id}"))
                .or_insert_with(|| cx.focus_handle().tab_stop(true))
                .clone();
            let key_id = id.clone();
            list = list.child(
                div()
                    .id(format!("decision-{id}"))
                    .mb(px(5.0))
                    .px(px(10.0))
                    .py(px(12.0))
                    .relative()
                    .border_1()
                    .border_color(if active {
                        t.colors.glass_edge_lavender()
                    } else {
                        t.colors.decision_rail()
                    })
                    .rounded(px(7.0))
                    .bg(if active {
                        t.colors.decision_selected()
                    } else {
                        t.colors.decision_rail()
                    })
                    .flex()
                    .flex_col()
                    .gap(px(7.0))
                    .role(Role::Button)
                    .aria_label(question.clone())
                    .aria_selected(active)
                    .track_focus(&focus)
                    .focus_visible(focus_ring(&t))
                    .cursor_pointer()
                    .hover(move |style| {
                        style
                            .bg(if active {
                                t.colors.decision_selected()
                            } else {
                                t.colors.decision_layer()
                            })
                            .border_color(if active {
                                t.colors.glass_edge_lavender()
                            } else {
                                t.colors.decision_line()
                            })
                    })
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.act(Action::Select(id.clone()), window, cx)
                    }))
                    .on_key_down(cx.listener(
                        move |this, event: &gpui::KeyDownEvent, window, cx| {
                            if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                                this.act(Action::Select(key_id.clone()), window, cx);
                                cx.stop_propagation();
                            }
                        },
                    ))
                    .when(active, |row| {
                        row.child(
                            div()
                                .absolute()
                                .left(px(0.0))
                                .top(px(15.0))
                                .bottom(px(15.0))
                                .w(px(2.0))
                                .rounded_full()
                                .bg(t.colors.decision_accent()),
                        )
                    })
                    .children(status.map(|status| {
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.0))
                            .child(div().size(px(5.0)).rounded_full().bg(
                                if status == DecisionStatus::Accepted {
                                    t.colors.decision_confirmed()
                                } else {
                                    t.colors.text_muted()
                                },
                            ))
                            .child(
                                text_style(div(), TypeScale::META)
                                    .text_color(t.colors.text_muted())
                                    .child(if status == DecisionStatus::Accepted {
                                        "Confirmada"
                                    } else {
                                        "Substituída"
                                    }),
                            )
                            .child(div().flex_1())
                            .children(version.map(|version| {
                                text_style(div(), TypeScale::META)
                                    .px(px(5.0))
                                    .rounded(px(4.0))
                                    .bg(t.colors.decision_layer())
                                    .text_color(t.colors.decision_accent())
                                    .child(format!("v{version}"))
                            }))
                    }))
                    .child(
                        text_style(div(), TypeScale::BODY_SMALL)
                            .text_size(px(12.0))
                            .line_height(px(20.0))
                            .line_clamp(2)
                            .text_color(t.colors.text_primary())
                            .child(question),
                    )
                    .child(
                        div()
                            .flex()
                            .items_start()
                            .gap(px(5.0))
                            .when(!searching, |meta| meta.child(Icon::clock(&t, 11.0, true)))
                            .child(
                                text_style(div(), TypeScale::META)
                                    .text_color(t.colors.text_muted())
                                    .child(meta),
                            ),
                    ),
            );
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
        if self.loaded && count == 0 {
            list = list.child(
                text_style(div(), TypeScale::BODY_SMALL)
                    .p(px(12.0))
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
            .bg(t.colors.decision_rail())
            .border_l_1()
            .border_color(t.colors.decision_line())
            .child(
                div()
                    .p(px(20.0))
                    .flex()
                    .flex_col()
                    .gap(px(12.0))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .child(Icon::list(&t, 15.0))
                            .child(
                                text_style(div(), TypeScale::HEADING_3)
                                    .flex_1()
                                    .child("Índice de decisões"),
                            )
                            .child(
                                text_style(div(), TypeScale::META)
                                    .px(px(6.0))
                                    .py(px(2.0))
                                    .rounded(px(4.0))
                                    .bg(t.colors.decision_layer())
                                    .text_color(t.colors.text_muted())
                                    .child(count.to_string()),
                            ),
                    )
                    .child(
                        text_style(div(), TypeScale::META)
                            .text_color(t.colors.text_muted())
                            .child("Escolhas preservadas neste projeto."),
                    )
                    .child(
                        text_style(div(), TypeScale::META)
                            .text_color(t.colors.text_muted())
                            .child(if self.busy {
                                "Carregando…".into()
                            } else if searching {
                                format!(
                                    "{count} {} · limite de 50",
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
                div()
                    .flex_none()
                    .p(px(16.0))
                    .border_t_1()
                    .border_color(t.colors.decision_line())
                    .child(
                        text_style(div(), TypeScale::META)
                            .text_color(t.colors.text_muted())
                            .child(if searching {
                                "Busca em pergunta, escolha e justificativa."
                            } else {
                                "Histórico e fontes preservados. Quantidade de itens carregados."
                            }),
                    ),
            )
            .into_any_element()
    }
    fn document(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let t = Theme::current(cx);
        let Some(mut detail) = self.detail.clone() else {
            return div().p(px(40.0)).child(text_style(div(),TypeScale::HEADING_2).child(if self.busy{"Carregando decisão…"}else if !self.query.trim().is_empty(){"Nenhuma decisão encontrada"}else{"Decisões que permanecem"}))
                .child(text_style(div(),TypeScale::BODY_SMALL).mt(px(12.0)).text_color(t.colors.text_muted()).child("Confirme uma escolha na Revisão para preservar o documento, suas evidências e seu histórico." )).into_any_element();
        };
        if self.history {
            let mut history=div().flex().flex_col().gap(px(16.0)).child(text_style(div(),TypeScale::HEADING_1).child("Histórico de versões"))
                .child(text_style(div(),TypeScale::BODY_SMALL).text_color(t.colors.text_muted()).child("Cada versão conserva o documento completo. Abra uma versão para ler seu conteúdo."));
            for revision in &detail.revisions {
                history = history.child(
                    div()
                        .p(px(16.0))
                        .border_1()
                        .border_color(t.colors.decision_line())
                        .rounded(px(8.0))
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
                                .mt(px(12.0))
                                .child(revision.question.clone()),
                        )
                        .child(
                            text_style(div(), TypeScale::BODY_SMALL)
                                .mt(px(8.0))
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
                detail.consequences = revision.consequences.clone();
                detail.reconsider_when = revision.reconsider_when.clone();
                detail.summary.version = revision.version;
                detail.summary.updated_at = revision.created_at.clone();
            }
        }
        let current = self.version.is_none();
        let badge = div()
            .flex()
            .items_center()
            .gap(px(6.0))
            .rounded(px(5.0))
            .px(px(8.0))
            .py(px(4.0))
            .bg(t.colors.decision_layer())
            .child(div().size(px(6.0)).rounded_full().bg(
                if detail.summary.status == DecisionStatus::Accepted {
                    t.colors.decision_confirmed()
                } else {
                    t.colors.text_muted()
                },
            ))
            .child(text_style(div(), TypeScale::META).child(
                if detail.summary.status == DecisionStatus::Accepted {
                    "Confirmada"
                } else {
                    "Substituída"
                },
            ));
        let mut document=div().w_full().max_w(px(740.0)).mx_auto().flex().flex_col().gap(px(24.0))
            .when(!current,|view|view.child(text_style(div(),TypeScale::BODY_SMALL).p(px(12.0)).rounded(px(6.0)).bg(t.colors.decision_selected()).child("Versão histórica · somente leitura. Volte a Documento para revisar ou exportar a versão atual.")))
            .child(div().flex().items_center().gap(px(10.0)).child(badge).child(text_style(div(),TypeScale::META).text_color(t.colors.text_muted()).child(format!("v{} · {}",detail.summary.version,short_date(&detail.summary.updated_at)))))
            .child(text_style(div(),TypeScale::HEADING_1).text_size(px(28.0)).line_height(px(37.0)).font_weight(gpui::FontWeight::MEDIUM).child(detail.summary.question.clone()))
            .child(div().flex().flex_col().gap(px(9.0)).border_l_2().border_color(t.colors.decision_accent()).pl(px(18.0))
                .child(text_style(div(),TypeScale::META).text_color(t.colors.decision_accent()).child("ESCOLHA CONFIRMADA"))
                .child(text_style(div(),TypeScale::HEADING_2).child(detail.summary.choice.clone())))
            .child(div().flex().flex_col().gap(px(10.0)).child(text_style(div(),TypeScale::META).text_color(t.colors.text_muted()).child("JUSTIFICATIVA"))
                .child(text_style(div(),TypeScale::BODY_SMALL).line_height(px(24.0)).text_color(t.colors.text_secondary()).child(detail.rationale.clone())));
        let mut context = div()
            .flex()
            .flex_wrap()
            .gap(px(16.0))
            .border_t_1()
            .border_color(t.colors.decision_line())
            .pt(px(20.0));
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
                .w(gpui::relative(0.46))
                .min_w(px(180.0))
                .flex()
                .flex_col()
                .gap(px(10.0))
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
                            .text_color(t.colors.text_muted())
                            .child("Nenhum item registrado."),
                    );
                } else {
                    section = section.children(items.iter().map(|item| {
                        text_style(div(), TypeScale::BODY_SMALL)
                            .text_color(t.colors.text_secondary())
                            .child(format!("• {item}"))
                    }));
                }
            }
            context = context.child(section);
        }
        document = document
            .child(
                div()
                    .border_t_1()
                    .border_color(t.colors.decision_line())
                    .pt(px(18.0))
                    .flex()
                    .flex_col()
                    .gap(px(12.0))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .child(Icon::layers(&t, 15.0))
                            .child(
                                text_style(div(), TypeScale::HEADING_3)
                                    .child("Contexto da decisão"),
                            ),
                    )
                    .child(context.border_t_0().pt(px(0.0))),
            )
            .child(self.evidence(cx));
        document = document.child(
            div()
                .border_t_1()
                .border_color(t.colors.decision_line())
                .pt(px(16.0))
                .flex()
                .flex_col()
                .gap(px(5.0))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(7.0))
                        .child(Icon::link(&t, 13.0))
                        .child(
                            text_style(div(), TypeScale::META)
                                .text_color(t.colors.text_muted())
                                .child("PROVENIÊNCIA"),
                        ),
                )
                .child(
                    text_style(div(), TypeScale::META)
                        .text_color(t.colors.text_secondary())
                        .child(format!(
                            "Captura: {}",
                            detail
                                .provenance
                                .capture_id
                                .as_deref()
                                .unwrap_or("não registrada")
                        )),
                )
                .child(
                    text_style(div(), TypeScale::META)
                        .text_color(t.colors.text_muted())
                        .child(format!("Candidato: {}", detail.provenance.candidate_id)),
                ),
        );
        document.into_any_element()
    }
    fn evidence(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let t = Theme::current(cx);
        let mut panel = div()
            .flex()
            .flex_col()
            .gap(px(12.0))
            .border_t_1()
            .border_color(t.colors.decision_line())
            .pt(px(20.0))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(Icon::link(&t, 15.0))
                    .child(
                        text_style(div(), TypeScale::HEADING_3)
                            .flex_1()
                            .child("Evidências"),
                    )
                    .child(
                        text_style(div(), TypeScale::META)
                            .px(px(7.0))
                            .py(px(3.0))
                            .rounded(px(5.0))
                            .bg(t.colors.decision_layer())
                            .text_color(t.colors.text_muted())
                            .child(format!(
                                "{} {}",
                                self.sources.len(),
                                if self.sources.len() == 1 {
                                    "fonte"
                                } else {
                                    "fontes"
                                }
                            )),
                    ),
            );
        if self.sources.is_empty() {
            return panel
                .child(
                    text_style(div(), TypeScale::BODY_SMALL)
                        .text_color(t.colors.text_muted())
                        .child("Nenhuma fonte vinculada."),
                )
                .into_any_element();
        }
        let mut tabs = div()
            .id("decision-source-tabs")
            .flex()
            .gap(px(2.0))
            .overflow_x_scroll()
            .border_b_1()
            .border_color(t.colors.decision_line())
            .bg(t.colors.decision_layer());
        for index in 0..self.sources.len() {
            let source = &self.sources[index];
            let label = source
                .artifact
                .as_ref()
                .map(evidence::label)
                .unwrap_or_else(|| source.link.artifact_id.clone());
            tabs = tabs.child(
                div()
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap(px(2.0))
                    .pl(px(8.0))
                    .border_b_2()
                    .border_color(if self.source == index {
                        t.colors.decision_accent()
                    } else {
                        t.colors.decision_layer()
                    })
                    .child(Icon::file(&t, 14.0, self.source != index))
                    .child(self.button(
                        format!("source-{index}"),
                        label,
                        Action::Source(index),
                        self.source == index,
                        cx,
                    )),
            );
        }
        let mut terminal = div()
            .min_w(px(0.0))
            .rounded(px(7.0))
            .border_1()
            .border_color(t.colors.decision_line())
            .overflow_hidden()
            .child(tabs);
        if let Some(artifact) = self
            .sources
            .get(self.source)
            .and_then(|source| source.artifact.clone())
        {
            let caption = self
                .lines
                .get(self.source)
                .and_then(Option::as_ref)
                .map(|lines| evidence::caption(&artifact, lines));
            terminal = terminal.child(
                div()
                    .flex()
                    .justify_end()
                    .gap(px(6.0))
                    .p(px(8.0))
                    .items_center()
                    .border_b_1()
                    .border_color(t.colors.decision_line())
                    .bg(t.colors.decision_rail())
                    .children(caption.map(|(path, description)| {
                        div()
                            .flex_1()
                            .min_w(px(0.0))
                            .flex()
                            .flex_col()
                            .gap(px(4.0))
                            .px(px(5.0))
                            .child(
                                crate::ui::theme::code_style(div(), TypeScale::META)
                                    .truncate()
                                    .text_color(t.colors.text_secondary())
                                    .child(path),
                            )
                            .child(
                                text_style(div(), TypeScale::META)
                                    .text_color(t.colors.text_muted())
                                    .child(description),
                            )
                    }))
                    .child(self.button(
                        "copy-source".into(),
                        "Copiar trecho".into(),
                        Action::CopySource,
                        false,
                        cx,
                    ))
                    .child(self.button(
                        "expand-source".into(),
                        if self.expanded {
                            "Recolher".into()
                        } else {
                            "Ampliar leitura".into()
                        },
                        Action::ExpandSource,
                        self.expanded,
                        cx,
                    )),
            );
            if let Some(lines) = self.lines.get(self.source).and_then(Option::as_ref) {
                terminal = terminal.child(evidence::snippet_body(
                    &artifact,
                    format!(
                        "decision-code-{}-{}",
                        self.selected.as_deref().unwrap_or(""),
                        self.source
                    ),
                    lines,
                    if self.expanded { 460.0 } else { 240.0 },
                    t,
                ));
            }
        } else {
            terminal = terminal.child(
                text_style(div(), TypeScale::BODY_SMALL)
                    .p(px(20.0))
                    .text_color(t.colors.text_muted())
                    .child("Fonte indisponível. O vínculo de proveniência foi preservado."),
            );
        }
        panel = panel.child(terminal);
        panel.into_any_element()
    }
    fn export_preview(&mut self, max_height: f32, cx: &mut Context<Self>) -> AnyElement {
        let t = Theme::current(cx);
        let document = self.preview.clone().expect("preview mounted");
        let json = document.format == ExportFormat::Json;
        let mut panel = div().size_full().flex().flex_col().child(
            div()
                .h(px(60.0))
                .flex_none()
                .px(px(24.0))
                .flex()
                .items_center()
                .gap(px(8.0))
                .border_b_1()
                .border_color(t.colors.decision_line())
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
                .px(px(24.0))
                .py(px(14.0))
                .flex()
                .items_center()
                .gap(px(12.0))
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
            panel =
                panel.child(
                    div()
                        .p(px(16.0))
                        .bg(t.colors.decision_selected())
                        .child(text_style(div(), TypeScale::BODY_SMALL).mb(px(10.0)).child(
                            format!(
                                "O arquivo {} já existe. Substituir seu conteúdo?",
                                path.display()
                            ),
                        ))
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
                div()
                    .flex_1()
                    .min_h(px(0.0))
                    .p(px(24.0))
                    .child(evidence::snippet_with_height(
                        &source,
                        "export-preview-content".into(),
                        &lines,
                        max_height,
                        t,
                    )),
            )
            .into_any_element()
    }
}
impl<S: DecisionStore + InboxStore + Send + 'static> Render for DecisionsScreen<S> {
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
            editor.clone().into_any_element()
        } else if self.preview.is_some() {
            self.export_preview(
                (f32::from(window.viewport_size().height) - 450.0).max(140.0),
                cx,
            )
        } else {
            let can_act = self.detail.is_some() && self.version.is_none();
            let toolbar = div()
                .h(px(60.0))
                .flex_none()
                .px(px(24.0))
                .flex()
                .items_center()
                .gap(px(6.0))
                .border_b_1()
                .border_color(t.colors.decision_line())
                .child(self.button(
                    "document-tab".into(),
                    "Documento".into(),
                    Action::Document,
                    !self.history,
                    cx,
                ))
                .child(self.button(
                    "history-tab".into(),
                    format!(
                            "Histórico · {}",
                            self.detail
                                .as_ref()
                                .map(|detail| detail.revisions.len())
                                .unwrap_or(0)
                        ),
                    Action::History,
                    self.history,
                    cx,
                ))
                .child(div().flex_1())
                .when(can_act, |bar| {
                    bar.child(self.button(
                        "export-open".into(),
                        "Exportar…".into(),
                        Action::Export(ExportFormat::Markdown),
                        false,
                        cx,
                    ))
                    .child(self.button(
                        "revise-open".into(),
                        "Revisar".into(),
                        Action::Revise,
                        true,
                        cx,
                    ))
                });
            let document = self.document(cx);
            div()
                .size_full()
                .flex()
                .flex_col()
                .child(toolbar)
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
                        .px(px(if compact { 28.0 } else { 34.0 }))
                        .py(px(if compact { 25.0 } else { 38.0 }))
                        .child(document),
                )
                .into_any_element()
        };
        let mut reader = div()
            .flex_1()
            .min_w(px(0.0))
            .h_full()
            .flex()
            .flex_col()
            .bg(t.colors.decision_canvas());
        if let Some(error) = self.error.clone() {
            reader = reader.child(
                div()
                    .p(px(12.0))
                    .flex()
                    .items_center()
                    .gap(px(12.0))
                    .border_b_1()
                    .border_color(t.colors.decision_line())
                    .child(
                        text_style(div(), TypeScale::BODY_SMALL)
                            .flex_1()
                            .text_color(t.colors.status_danger())
                            .child(error),
                    )
                    .when(self.editor.is_none() && self.preview.is_none(), |bar| {
                        bar.child(self.button(
                            "decisions-retry".into(),
                            "Tentar novamente".into(),
                            Action::Retry,
                            false,
                            cx,
                        ))
                    }),
            );
        }
        if let Some(notice) = &self.notice {
            reader = reader.child(
                text_style(div(), TypeScale::BODY_SMALL)
                    .px(px(24.0))
                    .py(px(10.0))
                    .bg(t.colors.decision_layer())
                    .child(notice.clone()),
            );
        }
        reader = reader.child(div().flex_1().min_h(px(0.0)).child(content));
        div()
            .id("decisions-screen")
            .track_focus(&self.reader_focus)
            .size_full()
            .flex()
            .bg(t.colors.decision_canvas())
            .child(reader)
            .child(self.index(compact, cx))
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
fn short_date(value: &str) -> String {
    chrono::DateTime::parse_from_rfc3339(value)
        .map(|date| {
            date.with_timezone(&chrono::Local)
                .format("%d/%m/%Y")
                .to_string()
        })
        .unwrap_or_else(|_| "Data indisponível".into())
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
