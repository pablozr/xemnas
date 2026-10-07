//! The conflict panel of Review: when the automatic review left a candidate
//! because it contradicts another item, both sides are shown together with
//! what each choice does. The choice runs in `application`
//! ([`application::conflicts`]); nothing here decides what a pick means.

use application::auto_approval::Conflict;
use application::conflicts::{ConflictView, Resolution, Side, SideKind};
use application::inbox::InboxStore;
use application::review_exception::ReviewExceptionStore;
use gpui::prelude::*;
use gpui::{
    div, px, Context, Div, Entity, FocusHandle, Focusable, Role, Stateful, Subscription, Window,
};

use super::super::format::short_date;
use super::InboxScreen;
use crate::i18n::inbox as t;
use crate::ui::controls::{action_button, ButtonKind};
use crate::ui::patterns::{compare_card, compare_pair, form_field, section_header, skeleton_list};
use crate::ui::search_field::{SearchChanged, SearchField};
use crate::ui::theme::{text_style, Theme};
use crate::ui::tokens::{SpacingScale, TypeScale};

/// The panel of the selected candidate.
pub(super) struct Panel {
    pub(super) candidate_id: String,
    other: Conflict,
    view: ConflictView,
    /// The two scope fields once "As duas valem" was picked.
    scoping: Option<[Entity<SearchField>; 2]>,
    subscriptions: Vec<Subscription>,
    /// A pick is being applied.
    busy: bool,
    focus: [FocusHandle; 3],
}

/// What a side says about itself, for its card.
fn tags_of(side: &Side) -> Vec<String> {
    vec![
        match side.kind {
            SideKind::Decision => t::kind_decision(),
            SideKind::Rule => t::kind_rule(),
        }
        .to_owned(),
        if side.in_force {
            t::conflict_state_in_force()
        } else {
            t::conflict_state_candidate()
        }
        .to_owned(),
    ]
}

fn facts_of(side: &Side) -> Vec<String> {
    let mut facts = vec![
        match &side.origin {
            Some(path) => t::conflict_origin_file(path),
            None => t::conflict_origin_capture().to_owned(),
        },
        if side.in_force {
            t::conflict_date_in_force(&short_date(&side.created_at))
        } else {
            t::conflict_date_candidate(&short_date(&side.created_at))
        },
    ];
    if !side.scope.is_empty() {
        facts.push(t::conflict_scope_line(&side.scope.join(" · ")));
    }
    facts
}

impl<S: InboxStore + ReviewExceptionStore + Send + 'static> InboxScreen<S> {
    /// Reads the conflict of the selected candidate, off the interface
    /// thread, when the review left it for the person naming another item.
    pub(super) fn load_conflict(&mut self, cx: &mut Context<Self>) {
        self.conflict = None;
        self.conflict_loading = false;
        let Some(id) = self.selected.clone() else {
            return;
        };
        let Some(entry) = self
            .left_for_you(&id)
            .filter(|entry| entry.conflicts_with.is_some())
            .cloned()
        else {
            return;
        };
        let (Some(api), Some(other)) = (self.approvals.clone(), entry.conflicts_with.clone())
        else {
            return;
        };
        self.conflict_loading = true;
        cx.spawn(async move |this, cx| {
            let view = cx
                .background_executor()
                .spawn(async move { api.conflict(&entry).ok().flatten() })
                .await;
            let _ = this.update(cx, |this, cx| {
                if this.selected.as_deref() != Some(id.as_str()) {
                    return;
                }
                this.conflict_loading = false;
                this.conflict = view.map(|view| Panel {
                    candidate_id: id,
                    other,
                    view,
                    scoping: None,
                    subscriptions: Vec::new(),
                    busy: false,
                    focus: std::array::from_fn(|_| cx.focus_handle().tab_stop(true)),
                });
                cx.notify();
            });
        })
        .detach();
    }

    /// Whether the panel stands for the candidate open in the pane; the
    /// pane's own actions give way to it.
    pub(super) fn conflict_open(&self) -> bool {
        self.conflict
            .as_ref()
            .is_some_and(|panel| self.selected.as_deref() == Some(panel.candidate_id.as_str()))
    }

    /// Applies a pick through `application`, then refreshes the queue.
    fn resolve_conflict(&mut self, resolution: Resolution, cx: &mut Context<Self>) {
        let (Some(api), Some(panel)) = (self.approvals.clone(), self.conflict.as_mut()) else {
            return;
        };
        if panel.busy {
            return;
        }
        panel.busy = true;
        let (id, other) = (panel.candidate_id.clone(), panel.other.clone());
        let notice = match resolution {
            Resolution::KeepThis => t::notice_conflict_this(),
            Resolution::KeepOther => t::notice_conflict_other(),
            Resolution::KeepBoth { .. } => t::notice_conflict_both(),
        };
        cx.notify();
        cx.spawn(async move |this, cx| {
            let done = cx
                .background_executor()
                .spawn(async move { api.resolve(&id, &other, &resolution).is_ok() })
                .await;
            let _ = this.update(cx, |this, cx| {
                if let Some(panel) = this.conflict.as_mut() {
                    panel.busy = false;
                }
                // The refresh clears the error, so the failure is set after it.
                this.page(false, cx);
                if done {
                    this.conflict = None;
                    this.notice = Some(notice);
                } else {
                    this.error = Some(t::conflict_failed());
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Opens the step that asks for one scope line per side, focused on the
    /// first.
    fn open_scoping(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(first) = self.make_scoping(cx) {
            window.focus(&first, cx);
        }
    }

    /// Builds the two scope fields; the focus handle of the first.
    fn make_scoping(&mut self, cx: &mut Context<Self>) -> Option<FocusHandle> {
        let panel = self.conflict.as_mut()?;
        let fields: [Entity<SearchField>; 2] = std::array::from_fn(|_| {
            cx.new(|cx| {
                let mut field = SearchField::new(cx);
                field.stretch();
                field.set_context(t::conflict_scope_hint(), cx);
                field
            })
        });
        panel.subscriptions = fields
            .iter()
            .map(|field| {
                cx.subscribe(field, |_, _, _: &SearchChanged, cx| {
                    cx.notify();
                })
            })
            .collect();
        let first = fields[0].read(cx).focus_handle(cx);
        panel.scoping = Some(fields);
        cx.notify();
        Some(first)
    }

    fn close_scoping(&mut self, cx: &mut Context<Self>) {
        if let Some(panel) = self.conflict.as_mut() {
            panel.scoping = None;
            panel.subscriptions.clear();
        }
        cx.notify();
    }

    /// The scope lines typed so far, when both are filled in.
    fn scopes(&self, cx: &Context<Self>) -> Option<(String, String)> {
        let fields = self.conflict.as_ref()?.scoping.as_ref()?;
        let line = |field: &Entity<SearchField>| field.read(cx).value().trim().to_owned();
        let (this_scope, other_scope) = (line(&fields[0]), line(&fields[1]));
        (!this_scope.is_empty() && !other_scope.is_empty()).then_some((this_scope, other_scope))
    }

    /// The panel, or a skeleton while it is read; nothing when the
    /// candidate has no conflict.
    pub(super) fn conflict_section(
        &self,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> Option<Stateful<Div>> {
        if self.conflict_loading {
            return Some(div().id("conflict-loading").child(skeleton_list(
                theme,
                "conflict-loading-rows",
                2,
            )));
        }
        let panel = self.conflict.as_ref().filter(|_| self.conflict_open())?;
        let view = &panel.view;
        let colors = theme.colors;
        Some(
            div()
                .id("conflict-panel")
                .role(Role::Group)
                .aria_label(t::conflict_aria())
                .flex()
                .flex_col()
                .gap(px(SpacingScale::S3))
                .child(section_header(theme, t::conflict_title()))
                .child(
                    text_style(div(), TypeScale::BODY_SMALL)
                        .text_color(colors.text_secondary())
                        .child(t::left_for_you_reason(&view.reason)),
                )
                .child(compare_pair(
                    compare_card(
                        theme,
                        t::conflict_side_this(),
                        tags_of(&view.this),
                        &view.this.question,
                        &view.this.choice,
                        facts_of(&view.this),
                        true,
                    ),
                    compare_card(
                        theme,
                        t::conflict_side_other(),
                        tags_of(&view.other),
                        &view.other.question,
                        &view.other.choice,
                        facts_of(&view.other),
                        false,
                    ),
                ))
                .child(if panel.scoping.is_some() {
                    self.scoping_step(panel, theme, cx)
                } else {
                    self.picks(panel, theme, cx)
                }),
        )
    }

    /// The three picks, each with the one line that says what it does.
    fn picks(&self, panel: &Panel, theme: &Theme, cx: &mut Context<Self>) -> Div {
        let view = &panel.view;
        let other_in_force = view.other.in_force;
        let rows: [(&'static str, &'static str, bool); 3] = [
            (
                t::conflict_keep_this(),
                if !view.can_replace {
                    t::conflict_effect_cannot_replace()
                } else if other_in_force {
                    t::conflict_effect_replaces()
                } else {
                    t::conflict_effect_rejects_other()
                },
                view.can_replace,
            ),
            (
                t::conflict_keep_other(),
                if other_in_force {
                    t::conflict_effect_other_stays()
                } else {
                    t::conflict_effect_other_accepted()
                },
                true,
            ),
            (t::conflict_keep_both(), t::conflict_effect_both(), true),
        ];
        let mut list = div().flex().flex_col().gap(px(SpacingScale::S2));
        for (index, (label, effect, offered)) in rows.into_iter().enumerate() {
            let enabled = offered && !panel.busy;
            list = list.child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap(px(SpacingScale::S3))
                    .child(self.pick_button(panel, index, label, enabled, cx))
                    .child(
                        text_style(div(), TypeScale::BODY_SMALL)
                            .flex_1()
                            .min_w(px(220.0))
                            .text_color(theme.colors.text_secondary())
                            .child(effect),
                    ),
            );
        }
        list
    }

    fn pick_button(
        &self,
        panel: &Panel,
        index: usize,
        label: &'static str,
        enabled: bool,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let theme = Theme::current(cx);
        action_button(
            &theme,
            ("conflict-pick", index),
            ButtonKind::Secondary,
            enabled,
        )
        .w(px(168.0))
        .justify_center()
        .aria_label(label)
        .track_focus(&panel.focus[index])
        .on_click(cx.listener(move |this, _, window, cx| {
            if enabled {
                this.pick(index, window, cx);
            }
        }))
        .on_key_down(
            cx.listener(move |this, event: &gpui::KeyDownEvent, window, cx| {
                if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                    if enabled {
                        this.pick(index, window, cx);
                    }
                    cx.stop_propagation();
                }
            }),
        )
        .child(label)
    }

    fn pick(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        match index {
            0 => self.resolve_conflict(Resolution::KeepThis, cx),
            1 => self.resolve_conflict(Resolution::KeepOther, cx),
            _ => self.open_scoping(window, cx),
        }
    }

    /// The step after "As duas valem": one line of scope for each side.
    fn scoping_step(&self, panel: &Panel, theme: &Theme, cx: &mut Context<Self>) -> Div {
        let Some(fields) = panel.scoping.as_ref() else {
            return div();
        };
        let scopes = self.scopes(cx);
        let enabled = scopes.is_some() && !panel.busy;
        div()
            .flex()
            .flex_col()
            .gap(px(SpacingScale::S3))
            .child(section_header(theme, t::conflict_scope_title()))
            .child(form_field(
                theme,
                t::conflict_scope_this(),
                None,
                fields[0].clone(),
            ))
            .child(form_field(
                theme,
                t::conflict_scope_other(),
                None,
                fields[1].clone(),
            ))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_end()
                    .gap(px(SpacingScale::S2))
                    .child(
                        action_button(theme, "conflict-scope-back", ButtonKind::Ghost, !panel.busy)
                            .aria_label(t::conflict_scope_back())
                            .track_focus(&panel.focus[0])
                            .on_click(cx.listener(|this, _, _, cx| this.close_scoping(cx)))
                            .child(t::conflict_scope_back()),
                    )
                    .child(
                        action_button(
                            theme,
                            "conflict-scope-confirm",
                            ButtonKind::Primary,
                            enabled,
                        )
                        .aria_label(t::conflict_scope_confirm())
                        .track_focus(&panel.focus[2])
                        .on_click(cx.listener(|this, _, _, cx| {
                            if let Some((this_scope, other_scope)) = this.scopes(cx) {
                                this.resolve_conflict(
                                    Resolution::KeepBoth {
                                        this_scope,
                                        other_scope,
                                    },
                                    cx,
                                );
                            }
                        }))
                        .child(t::conflict_scope_confirm()),
                    ),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn side(in_force: bool, origin: Option<&str>, scope: &[&str]) -> Side {
        Side {
            kind: SideKind::Rule,
            id: "s".into(),
            in_force,
            question: "q".into(),
            choice: "c".into(),
            origin: origin.map(str::to_owned),
            created_at: "2026-09-29T10:00:00Z".into(),
            scope: scope.iter().map(|line| (*line).to_owned()).collect(),
        }
    }

    #[test]
    fn a_side_says_its_kind_and_whether_it_stands() {
        let waiting = tags_of(&side(false, None, &[]));
        let standing = tags_of(&side(true, None, &[]));
        assert_eq!(waiting.len(), 2);
        assert_eq!(waiting[0], standing[0], "same kind");
        assert_ne!(waiting[1], standing[1], "different state");
        assert_ne!(
            tags_of(&Side {
                kind: SideKind::Decision,
                ..side(false, None, &[])
            })[0],
            waiting[0]
        );
    }

    #[test]
    fn facts_carry_the_origin_the_date_and_only_a_scope_that_exists() {
        let plain = facts_of(&side(false, Some("docs/adr/0003.md"), &[]));
        assert_eq!(plain.len(), 2);
        assert!(plain[0].contains("docs/adr/0003.md"), "{plain:?}");
        let scoped = facts_of(&side(true, None, &["só na API", "só no núcleo"]));
        assert_eq!(scoped.len(), 3);
        assert!(scoped[2].contains("só na API · só no núcleo"), "{scoped:?}");
        assert_ne!(
            facts_of(&side(true, None, &[]))[0],
            plain[0],
            "a capture is not a file"
        );
    }
}
