//! Labelled editing form; submitting is explicit and cancellation writes nothing.
use crate::app::SaveEditor;
use crate::ui::controls::{action_button, ButtonKind};
use crate::ui::patterns::{action_footer, form_field, reading_page, section_label};
use crate::ui::search_field::SearchField;
use crate::ui::theme::{text_style, Theme};
use crate::ui::tokens::{SpacingScale, TypeScale};
use application::inbox::CandidateEdits;
use application::qualifiers::{KnowledgeQualifier, QualifierKind};
use gpui::prelude::*;
use gpui::{div, px, App, Context, Entity, EventEmitter, FocusHandle, Focusable, Render, Window};

const LABELS: [&str; 3] = ["Pergunta", "Escolha sugerida", "Motivo"];

pub(super) enum EditorEvent {
    Cancel,
    Save(CandidateEdits, bool),
}

pub(super) struct ReviewEditor {
    fields: [Entity<SearchField>; 3],
    original: CandidateEdits,
    focus: [FocusHandle; 3],
    busy: bool,
    qualifiers: QualifierFields,
    evidence: Vec<application::inbox::ArtifactView>,
}
impl EventEmitter<EditorEvent> for ReviewEditor {}
impl ReviewEditor {
    pub(super) fn initial_focus(&self, cx: &App) -> FocusHandle {
        self.fields[0].read(cx).focus_handle(cx)
    }
    pub(super) fn set_evidence(&mut self, evidence: Vec<application::inbox::ArtifactView>) {
        self.evidence = evidence;
    }
    pub(super) fn new(original: CandidateEdits, cx: &mut Context<Self>) -> Self {
        let values = [&original.question, &original.choice, &original.rationale];
        let fields = std::array::from_fn(|index| {
            cx.new(|cx| {
                let mut field = SearchField::new(cx);
                if index == 2 {
                    field.multiline(120.0);
                } else {
                    field.stretch();
                }
                field.set_context(LABELS[index], cx);
                field.set_value(values[index], cx);
                field
            })
        });
        Self {
            qualifiers: QualifierFields::new(&original.qualifiers, cx),
            fields,
            original,
            focus: std::array::from_fn(|_| cx.focus_handle().tab_stop(true)),
            busy: false,
            evidence: Vec::new(),
        }
    }
    pub(super) fn set_busy(&mut self, busy: bool, cx: &mut Context<Self>) {
        self.busy = busy;
        cx.notify();
    }
    fn submit(&mut self, index: usize, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        if index == 0 {
            cx.emit(EditorEvent::Cancel);
            return;
        }
        let originals = [
            &self.original.question,
            &self.original.choice,
            &self.original.rationale,
        ];
        // Single-line controls preserve unchanged multiline values verbatim.
        let values: [String; 3] = std::array::from_fn(|index| {
            let value = self.fields[index].read(cx).value();
            if value
                == originals[index]
                    .replace("\r\n", " ")
                    .replace(['\r', '\n'], " ")
            {
                originals[index].clone()
            } else {
                value.to_owned()
            }
        });
        cx.emit(EditorEvent::Save(
            CandidateEdits {
                question: values[0].clone(),
                choice: values[1].clone(),
                rationale: values[2].clone(),
                qualifiers: self.qualifiers.values(cx),
            },
            index == 2,
        ));
    }
}
impl Render for ReviewEditor {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::current(cx);
        let hints = [
            None,
            Some("A escolha que será registrada se você confirmar."),
            Some("Por que essa escolha foi feita, nas palavras da equipe."),
        ];
        let column = div()
            .flex()
            .flex_col()
            .gap(px(SpacingScale::S6))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(SpacingScale::S2))
                    .child(section_label(&theme, "Ajustar candidato"))
                    .child(
                        text_style(div(), TypeScale::BODY)
                            .text_color(theme.colors.text_secondary())
                            .child("Revise a pergunta, a escolha e o motivo antes de confirmar."),
                    ),
            )
            .children(LABELS.into_iter().enumerate().map(|(index, label)| {
                form_field(&theme, label, hints[index], self.fields[index].clone())
            }))
            .child(self.qualifiers.render(&theme))
            .child(section_label(&theme, "Evidências para consulta"))
            .children(self.evidence.iter().enumerate().map(|(index, artifact)| {
                let lines = super::evidence::SourceLines::new(artifact);
                super::evidence::frame(&theme)
                    .child(super::evidence::caption_row(&theme, artifact, &lines))
                    .child(super::evidence::body(
                        artifact,
                        format!("review-editor-evidence-{index}"),
                        &lines,
                        240.0,
                        theme,
                    ))
            }));
        let actions = ["Cancelar", "Salvar ajustes", "Salvar e confirmar"]
            .into_iter()
            .enumerate()
            .map(|(index, label)| {
                let kind = match index {
                    0 => ButtonKind::Ghost,
                    1 => ButtonKind::Secondary,
                    _ => ButtonKind::Primary,
                };
                action_button(&theme, ("editor-action", index), kind, !self.busy)
                    .px(px(SpacingScale::S4))
                    .aria_label(label)
                    .track_focus(&self.focus[index])
                    .on_click(cx.listener(move |this, _, _, cx| this.submit(index, cx)))
                    .on_key_down(cx.listener(move |this, event: &gpui::KeyDownEvent, _, cx| {
                        if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                            this.submit(index, cx);
                            cx.stop_propagation();
                        }
                    }))
                    .child(label)
            });
        div()
            .id("review-editor")
            .key_context("Editor")
            .on_action(cx.listener(|this, _: &SaveEditor, _, cx| this.submit(2, cx)))
            .size_full()
            .flex()
            .flex_col()
            .child(reading_page("review-editor-fields", column))
            .child(
                action_footer(&theme, self.busy.then_some(("Salvando…", false))).children(actions),
            )
    }
}

pub(super) fn qualifier_label(kind: QualifierKind) -> &'static str {
    match kind {
        QualifierKind::Attribution => "Atribuição",
        QualifierKind::Scope => "Escopo",
        QualifierKind::Validation => "Validação",
    }
}

pub(super) fn qualifier_reading(theme: &Theme, items: &[KnowledgeQualifier]) -> gpui::Div {
    // Nothing informed is the common case: no section instead of "none".
    if items.is_empty() {
        return div();
    }
    div()
        .flex()
        .flex_col()
        .gap(px(SpacingScale::S2))
        .child(section_label(theme, "Alcance e ressalvas"))
        .children(items.iter().map(|item| {
            div()
                .flex()
                .flex_col()
                .gap(px(SpacingScale::S1))
                .child(text_style(div(), TypeScale::BODY).child(item.text.clone()))
                .child(
                    text_style(div(), TypeScale::META)
                        .text_color(theme.colors.text_muted())
                        .child(format!(
                            "{} · {}",
                            qualifier_label(item.kind),
                            if item.artifact_id.is_some() {
                                "citado da evidência"
                            } else {
                                "escrito na revisão, sem fonte"
                            }
                        )),
                )
        }))
}

pub(super) struct QualifierFields {
    original: Vec<KnowledgeQualifier>,
    fields: Vec<Entity<SearchField>>,
    additions: Vec<(QualifierKind, Entity<SearchField>)>,
}

impl QualifierFields {
    pub(super) fn new<T: 'static>(items: &[KnowledgeQualifier], cx: &mut Context<T>) -> Self {
        let fields = items
            .iter()
            .map(|item| {
                cx.new(|cx| {
                    let mut field = SearchField::new(cx);
                    field.multiline(100.0);
                    field.set_context(qualifier_label(item.kind), cx);
                    field.set_value(&item.text, cx);
                    field
                })
            })
            .collect();
        let additions = [
            QualifierKind::Attribution,
            QualifierKind::Scope,
            QualifierKind::Validation,
        ]
        .into_iter()
        .map(|kind| {
            (
                kind,
                cx.new(|cx| {
                    let mut field = SearchField::new(cx);
                    field.multiline(100.0);
                    field.set_context(qualifier_label(kind), cx);
                    field
                }),
            )
        })
        .collect();
        Self {
            original: items.to_vec(),
            fields,
            additions,
        }
    }

    pub(super) fn values(&self, cx: &App) -> Vec<KnowledgeQualifier> {
        let mut items = Vec::new();
        for (original, field) in self.original.iter().zip(&self.fields) {
            if let Some(item) = edited_qualifier(original, field.read(cx).value()) {
                items.push(item);
            }
        }
        for (kind, field) in &self.additions {
            let text = field.read(cx).value();
            if !text.trim().is_empty() {
                items.push(KnowledgeQualifier {
                    kind: *kind,
                    text: text.into(),
                    artifact_id: None,
                });
            }
        }
        items
    }

    pub(super) fn render(&self, theme: &Theme) -> gpui::Div {
        div().flex().flex_col().gap(px(SpacingScale::S4))
            .child(section_label(theme, "Alcance e ressalvas"))
            .child("Alterar uma citação a torna declaração do revisor, sem fonte verificada. Apague o texto para remover.")
            .children(self.original.iter().zip(&self.fields).map(|(item, field)| {
                form_field(theme, qualifier_label(item.kind), Some(if item.artifact_id.is_some() {
                    "Citação da evidência; consulte a fonte antes de alterar."
                } else { "Declaração do revisor · sem fonte verificada." }), field.clone())
            }))
            .children(self.additions.iter().map(|(kind, field)| {
                form_field(theme, qualifier_label(*kind), Some("Adicionar declaração do revisor (opcional)."), field.clone())
            }))
    }
}

fn edited_qualifier(original: &KnowledgeQualifier, text: &str) -> Option<KnowledgeQualifier> {
    if text.trim().is_empty() {
        return None;
    }
    if text == original.text.replace("\r\n", "\n").replace('\r', "\n") {
        return Some(original.clone());
    }
    Some(KnowledgeQualifier {
        kind: original.kind,
        text: text.into(),
        artifact_id: None,
    })
}

#[cfg(test)]
mod qualifier_tests {
    use super::*;
    #[test]
    fn editing_never_manufactures_source_support() {
        let item = KnowledgeQualifier {
            kind: QualifierKind::Validation,
            text: "Teste bloqueado localmente".into(),
            artifact_id: Some("source".into()),
        };
        assert_eq!(edited_qualifier(&item, &item.text), Some(item.clone()));
        assert_eq!(
            edited_qualifier(&item, "Simulação apenas")
                .unwrap()
                .artifact_id,
            None
        );
        assert!(edited_qualifier(&item, " ").is_none());
    }
}
