//! Labelled editing form; submitting is explicit and cancellation writes nothing.
use crate::app::SaveEditor;
use crate::ui::controls::{action_button, ButtonKind};
use crate::ui::patterns::{action_footer, form_field, reading_page, section_label};
use crate::ui::search_field::SearchField;
use crate::ui::theme::{text_style, Theme};
use crate::ui::tokens::{SpacingScale, TypeScale};
use application::inbox::CandidateEdits;
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
}
impl EventEmitter<EditorEvent> for ReviewEditor {}
impl ReviewEditor {
    pub(super) fn initial_focus(&self, cx: &App) -> FocusHandle {
        self.fields[0].read(cx).focus_handle(cx)
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
            fields,
            original,
            focus: std::array::from_fn(|_| cx.focus_handle().tab_stop(true)),
            busy: false,
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
