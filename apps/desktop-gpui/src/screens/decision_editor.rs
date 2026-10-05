//! Explicit revision form. Unchanged fields retain their original snapshots.
use crate::app::SaveEditor;
use crate::ui::controls::{action_button, ButtonKind};
use crate::ui::patterns::{action_footer, form_field, reading_page, section_label};
use crate::ui::search_field::{SearchChanged, SearchField};
use crate::ui::theme::{text_style, Theme};
use crate::ui::tokens::{SpacingScale, TypeScale};
use application::decisions::{DecisionDetail, DecisionEdits};
use gpui::prelude::*;
use gpui::{
    div, px, App, Context, Entity, EventEmitter, FocusHandle, Focusable, Render, Subscription,
    Window,
};

pub(super) enum RevisionEvent {
    Cancel,
    Save(DecisionEdits),
}
pub(super) struct DecisionEditor {
    fields: [Entity<SearchField>; 7],
    original: [String; 7],
    original_arrays: [Vec<String>; 4],
    subscriptions: Vec<Subscription>,
    focus: [FocusHandle; 2],
    busy: bool,
    qualifiers: super::review_editor::QualifierFields,
    original_qualifiers: Vec<application::qualifiers::KnowledgeQualifier>,
}
impl EventEmitter<RevisionEvent> for DecisionEditor {}
const LABELS: [&str; 7] = [
    "Pergunta",
    "Escolha",
    "Justificativa",
    "Premissas",
    "Escopo",
    "Consequências",
    "Reconsiderar quando",
];
impl DecisionEditor {
    pub(super) fn new(detail: &DecisionDetail, cx: &mut Context<Self>) -> Self {
        let arrays = [
            detail.assumptions.clone(),
            detail.scope.clone(),
            detail.consequences.clone(),
            detail.reconsider_when.clone(),
        ];
        let original = [
            detail.summary.question.clone(),
            detail.summary.choice.clone(),
            detail.rationale.clone(),
            arrays[0].join("\n"),
            arrays[1].join("\n"),
            arrays[2].join("\n"),
            arrays[3].join("\n"),
        ];
        let fields = std::array::from_fn(|i| {
            cx.new(|cx| {
                let mut field = SearchField::new(cx);
                field.multiline(if i == 2 { 160.0 } else { 100.0 });
                field.set_context(LABELS[i], cx);
                field.set_value(&original[i], cx);
                field
            })
        });
        let subscriptions = fields
            .iter()
            .map(|field| cx.subscribe(field, |_, _, _: &SearchChanged, cx| cx.notify()))
            .collect();
        Self {
            qualifiers: super::review_editor::QualifierFields::new(&detail.qualifiers, cx),
            original_qualifiers: detail.qualifiers.clone(),
            fields,
            original,
            original_arrays: arrays,
            subscriptions,
            focus: std::array::from_fn(|_| cx.focus_handle().tab_stop(true)),
            busy: false,
        }
    }
    pub(super) fn initial_focus(&self, cx: &App) -> FocusHandle {
        self.fields[0].read(cx).focus_handle(cx)
    }
    pub(super) fn set_busy(&mut self, busy: bool, cx: &mut Context<Self>) {
        self.busy = busy;
        cx.notify();
    }
    fn edits(&self, cx: &App) -> DecisionEdits {
        let value = |i: usize| {
            let value = self.fields[i].read(cx).value();
            (value != self.original[i].replace("\r\n", "\n").replace('\r', "\n"))
                .then(|| value.to_owned())
        };
        let array = |i: usize| {
            value(i + 3)
                .map(|value| {
                    value
                        .lines()
                        .map(str::trim)
                        .filter(|item| !item.is_empty())
                        .map(str::to_owned)
                        .collect::<Vec<_>>()
                })
                .filter(|items| items != &self.original_arrays[i])
        };
        DecisionEdits {
            question: value(0),
            choice: value(1),
            rationale: value(2),
            assumptions: array(0),
            scope: array(1),
            consequences: array(2),
            reconsider_when: array(3),
            qualifiers: {
                let values = self.qualifiers.values(cx);
                (values != self.original_qualifiers).then_some(values)
            },
        }
    }
    fn submit(&mut self, save: bool, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        if save {
            let edits = self.edits(cx);
            if !edits.is_empty() && edits.validate().is_ok() {
                cx.emit(RevisionEvent::Save(edits));
            }
        } else {
            cx.emit(RevisionEvent::Cancel);
        }
    }
}
impl Render for DecisionEditor {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = Theme::current(cx);
        let edits = self.edits(cx);
        let valid = edits.validate().is_ok();
        let _ = &self.subscriptions;
        let column = div()
            .flex()
            .flex_col()
            .gap(px(SpacingScale::S6))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(SpacingScale::S2))
                    .child(section_label(&t, "Revisar decisão"))
                    .child(
                        text_style(div(), TypeScale::BODY)
                            .text_color(t.colors.text_secondary())
                            .child("A versão anterior permanece no histórico."),
                    ),
            )
            .children(LABELS.iter().enumerate().map(|(index, label)| {
                form_field(
                    &t,
                    label,
                    (index >= 3).then_some("Um item por linha."),
                    self.fields[index].clone(),
                )
            }))
            .child(self.qualifiers.render(&t));
        let message = if self.busy {
            Some(("Salvando…", false))
        } else if !edits.is_empty() && !valid {
            Some((
                "Preencha os campos obrigatórios e respeite os limites de tamanho.",
                true,
            ))
        } else if edits.is_empty() {
            Some(("Nenhuma alteração ainda.", false))
        } else {
            None
        };
        let actions = [false, true].into_iter().enumerate().map(|(index, save)| {
            let enabled = !self.busy && (!save || (valid && !edits.is_empty()));
            let kind = if save {
                ButtonKind::Primary
            } else {
                ButtonKind::Ghost
            };
            action_button(&t, ("revision-action", index), kind, enabled)
                .px(px(SpacingScale::S4))
                .aria_label(if save {
                    "Salvar nova versão"
                } else {
                    "Cancelar revisão"
                })
                .track_focus(&self.focus[index])
                .on_click(cx.listener(move |this, _, _, cx| this.submit(save, cx)))
                .on_key_down(cx.listener(move |this, event: &gpui::KeyDownEvent, _, cx| {
                    if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                        this.submit(save, cx);
                        cx.stop_propagation();
                    }
                }))
                .child(if save {
                    "Salvar nova versão"
                } else {
                    "Cancelar"
                })
        });
        div()
            .key_context("Editor")
            .on_action(cx.listener(|this, _: &SaveEditor, _, cx| this.submit(true, cx)))
            .size_full()
            .flex()
            .flex_col()
            .child(reading_page("revision-fields", column))
            .child(action_footer(&t, message).children(actions))
    }
}
