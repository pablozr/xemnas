//! Explicit revision form. Unchanged fields retain their original snapshots.
use crate::ui::controls::{action_button, ButtonKind};
use crate::ui::search_field::{SearchChanged, SearchField};
use crate::ui::theme::{text_style, Theme};
use crate::ui::tokens::TypeScale;
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
        }
    }
    fn submit(&mut self, save: bool, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        if save {
            let edits = self.edits(cx);
            if edits.validate().is_ok() {
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
        div().size_full().flex().flex_col()
            .child(div().h(px(60.0)).flex_none().px(px(28.0)).flex().items_center().gap(px(8.0)).border_b_1().border_color(t.colors.hairline_divider())
                .child(text_style(div(),TypeScale::HEADING_3).flex_1().child("Revisar decisão"))
                .children([false,true].into_iter().enumerate().map(|(i,save)|{
                    let enabled=!self.busy&&(!save||valid);
                    let kind=if save{ButtonKind::Primary}else{ButtonKind::Ghost};
                    action_button(&t,("revision-action",i),kind,enabled)
                        .aria_label(if save{"Salvar nova versão"}else{"Cancelar revisão"}).track_focus(&self.focus[i])
                        .on_click(cx.listener(move|this,_,_,cx|this.submit(save,cx)))
                        .on_key_down(cx.listener(move|this,event:&gpui::KeyDownEvent,_,cx|{if matches!(event.keystroke.key.as_str(),"enter"|"space"){this.submit(save,cx);cx.stop_propagation();}}))
                        .child(if save&&self.busy{"Salvando…"}else if save{"Salvar nova versão"}else{"Cancelar"})
                })))
            .child(div().id("revision-fields").flex_1().min_h(px(0.0)).overflow_y_scroll().px(px(34.0)).py(px(28.0))
                .child(text_style(div(),TypeScale::BODY_SMALL).mb(px(24.0)).text_color(t.colors.text_muted()).child("A versão anterior permanece no histórico. Nas listas, use um item por linha."))
                .children(LABELS.iter().enumerate().map(|(i,label)|div().mb(px(20.0)).flex().flex_col().gap(px(8.0)).child(text_style(div(),TypeScale::HEADING_3).child(*label)).child(self.fields[i].clone()))))
            .when(!edits.is_empty()&&!valid,|view|view.child(text_style(div(),TypeScale::BODY_SMALL).p(px(12.0)).text_color(t.colors.status_danger()).child("Preencha os campos obrigatórios e respeite os limites de tamanho.")))
    }
}
