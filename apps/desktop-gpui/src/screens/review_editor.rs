//! Labelled editing form; submitting is explicit and cancellation writes nothing.
use crate::ui::glass::focus_ring;
use crate::ui::search_field::SearchField;
use crate::ui::theme::{text_style, Theme};
use crate::ui::tokens::{SpacingScale, TypeScale};
use application::inbox::CandidateEdits;
use gpui::prelude::*;
use gpui::{
    div, px, App, Context, Entity, EventEmitter, FocusHandle, Focusable, Render, Role, Window,
};

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
        let labels = ["Pergunta", "Escolha sugerida", "Motivo"];
        let fields = std::array::from_fn(|index| {
            cx.new(|cx| {
                let mut field = SearchField::new(cx);
                field.stretch();
                field.set_context(labels[index], cx);
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
        let theme = Theme::quiet_glass();
        div()
            .id("review-editor")
            .size_full()
            .overflow_y_scroll()
            .p(px(SpacingScale::S8))
            .flex()
            .flex_col()
            .gap(px(SpacingScale::S6))
            .child(text_style(div(), TypeScale::HEADING_1).child("Ajustar candidato"))
            .child(
                text_style(div(), TypeScale::BODY_SMALL)
                    .text_color(theme.colors.text_muted())
                    .child("Revise a pergunta, a escolha e o motivo antes de confirmar."),
            )
            .children(
                ["Pergunta", "Escolha sugerida", "Motivo"]
                    .into_iter()
                    .enumerate()
                    .map(|(index, label)| {
                        div()
                            .w_full()
                            .flex()
                            .flex_col()
                            .gap(px(SpacingScale::S2))
                            .child(text_style(div(), TypeScale::HEADING_3).child(label))
                            .child(self.fields[index].clone())
                    }),
            )
            .child(
                div().flex().flex_wrap().gap(px(SpacingScale::S2)).children(
                    ["Cancelar", "Salvar ajustes", "Salvar e confirmar"]
                        .into_iter()
                        .enumerate()
                        .map(|(index, label)| {
                            text_style(div(), TypeScale::BODY_SMALL)
                                .id(("editor-action", index))
                                .px(px(SpacingScale::S4))
                                .h(px(40.0))
                                .flex()
                                .items_center()
                                .rounded(theme.radius.control())
                                .border_1()
                                .border_color(theme.colors.glass_border())
                                .bg(if index == 2 {
                                    theme.colors.accent_emphasis()
                                } else {
                                    theme.colors.rail()
                                })
                                .text_color(if self.busy {
                                    theme.colors.text_disabled()
                                } else if index == 2 {
                                    theme.colors.accent_on_emphasis()
                                } else {
                                    theme.colors.text_primary()
                                })
                                .role(Role::Button)
                                .aria_label(label)
                                .track_focus(&self.focus[index])
                                .focus_visible(focus_ring(&theme))
                                .cursor_pointer()
                                .on_click(cx.listener(move |this, _, _, cx| this.submit(index, cx)))
                                .on_key_down(cx.listener(
                                    move |this, event: &gpui::KeyDownEvent, _, cx| {
                                        if matches!(event.keystroke.key.as_str(), "enter" | "space")
                                        {
                                            this.submit(index, cx);
                                            cx.stop_propagation();
                                        }
                                    },
                                ))
                                .child(label)
                        }),
                ),
            )
    }
}
