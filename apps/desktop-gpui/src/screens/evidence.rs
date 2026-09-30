//! Presentation of redacted capture artifacts, using recorded provenance only.

use crate::ui::glass::focus_ring;
use crate::ui::icons::Icon;
use crate::ui::theme::{code_style, text_style, Theme};
use crate::ui::tokens::{SpacingScale, TypeScale};
use application::inbox::ArtifactView;
use gpui::prelude::*;
use gpui::{
    div, px, transparent_black, uniform_list, AnyElement, Div, ElementId,
    ListHorizontalSizingBehavior, Role, Stateful,
};
use std::sync::Arc;

/// Parsed once per loaded source; only visible rows are built during rendering.
pub(super) struct SourceLines {
    lines: Arc<Vec<String>>,
    widest: usize,
}
impl SourceLines {
    pub(super) fn new(artifact: &ArtifactView) -> Self {
        let mut lines: Vec<String> = artifact.content.lines().map(str::to_owned).collect();
        if lines.is_empty() {
            lines.push(String::new());
        }
        let widest = lines
            .iter()
            .enumerate()
            .max_by_key(|(_, line)| line.chars().count())
            .map(|(index, _)| index)
            .unwrap_or(0);
        Self {
            lines: Arc::new(lines),
            widest,
        }
    }
}

pub(super) fn label(artifact: &ArtifactView) -> String {
    let metadata = metadata(artifact);
    metadata
        .get("file")
        .or_else(|| metadata.get("path"))
        .and_then(|value| value.as_str())
        .map(|path| path.rsplit(['/', '\\']).next().unwrap_or(path).to_owned())
        .unwrap_or_else(|| kind_label(&artifact.kind).to_owned())
}

fn metadata(artifact: &ArtifactView) -> serde_json::Value {
    serde_json::from_str(&artifact.metadata).unwrap_or(serde_json::Value::Null)
}

fn kind_label(kind: &str) -> &str {
    match kind {
        "diff_hunk" => "Trecho de alteração",
        "user_text" => "Mensagem do usuário",
        "assistant_text" => "Resposta do assistente",
        "tool_summary" => "Resultado de ferramenta",
        "export_document" => "Documento de exportação",
        _ => "Fonte da captura",
    }
}

/// Recorded path, or the source label when nothing was recorded.
fn path(artifact: &ArtifactView) -> String {
    let metadata = metadata(artifact);
    metadata
        .get("file")
        .or_else(|| metadata.get("path"))
        .and_then(|value| value.as_str())
        .map(str::to_owned)
        .unwrap_or_else(|| label(artifact))
}

fn start_line(artifact: &ArtifactView) -> Option<u64> {
    let metadata = metadata(artifact);
    metadata
        .get("start_line")
        .or_else(|| metadata.get("line_start"))
        .and_then(|value| value.as_u64())
        .filter(|start| *start > 0)
}

fn is_code(artifact: &ArtifactView) -> bool {
    artifact.kind == "diff_hunk"
        || artifact.kind == "export_document"
        || metadata(artifact).get("language").is_some()
}

/// The one-line description under a source: what it is and which lines.
fn description(artifact: &ArtifactView, source: &SourceLines) -> String {
    let range = match start_line(artifact) {
        Some(start) => format!(
            "linhas {start}–{}",
            start.saturating_add(source.lines.len().saturating_sub(1) as u64)
        ),
        None if artifact.kind == "export_document" => "conteúdo exato para salvar".into(),
        None if is_code(artifact) => format!("{} linhas do trecho", source.lines.len()),
        None => "texto da captura".into(),
    };
    format!("{} · {range}", kind_label(&artifact.kind))
}

/// The well that holds a source: tabs, caption and body share one border.
pub(super) fn frame(theme: &Theme) -> Div {
    div()
        .w_full()
        .min_w(px(0.0))
        .flex_none()
        .flex()
        .flex_col()
        .rounded(theme.radius.surface())
        .border_1()
        .border_color(theme.colors.hairline_divider())
        .bg(theme.colors.canvas_deep())
        .overflow_hidden()
}

/// The horizontal strip of source tabs; long strips scroll sideways.
pub(super) fn tab_strip(theme: &Theme, id: impl Into<ElementId>) -> Stateful<Div> {
    div()
        .id(id)
        .w_full()
        .min_w(px(0.0))
        .flex()
        .overflow_x_scroll()
        .px(px(SpacingScale::S1))
        .border_b_1()
        .border_color(theme.colors.hairline_divider())
        .bg(theme.colors.canvas_raised())
        .role(Role::TabList)
}

/// One source tab: file icon and name, selected by a 2 px underline only.
///
/// Screens attach focus and handlers. The icon sits inside the hit area, so
/// the whole tab selects the source.
pub(super) fn tab(
    theme: &Theme,
    id: impl Into<ElementId>,
    artifact: &ArtifactView,
    selected: bool,
) -> Stateful<Div> {
    let colors = theme.colors;
    text_style(div(), TypeScale::BODY_SMALL)
        .id(id)
        .flex_none()
        .h(px(36.0))
        .px(px(SpacingScale::S3))
        .flex()
        .items_center()
        .gap(px(SpacingScale::S2))
        .border_b_2()
        .border_color(if selected {
            colors.accent_hover().into()
        } else {
            transparent_black()
        })
        .text_color(if selected {
            colors.text_primary()
        } else {
            colors.text_muted()
        })
        .when(!selected, |tab| {
            tab.hover(move |style| style.text_color(colors.text_secondary()))
        })
        .role(Role::Tab)
        .aria_label(label(artifact))
        .aria_selected(selected)
        .focus_visible(focus_ring(theme))
        .cursor_pointer()
        .child(Icon::file(theme, 14.0, !selected))
        .child(div().max_w(px(200.0)).truncate().child(label(artifact)))
}

/// Path in monospace, then what the source is. Screens may append actions.
pub(super) fn caption_row(theme: &Theme, artifact: &ArtifactView, source: &SourceLines) -> Div {
    div()
        .min_h(px(36.0))
        .px(px(SpacingScale::S3))
        .py(px(6.0))
        .flex()
        .items_center()
        .gap(px(SpacingScale::S2))
        .border_b_1()
        .border_color(theme.colors.hairline_divider())
        .child(
            div()
                .flex_1()
                .min_w(px(0.0))
                .flex()
                .items_baseline()
                .gap(px(SpacingScale::S2))
                .child(
                    code_style(div(), TypeScale::META)
                        .min_w(px(0.0))
                        .flex_shrink(1.0)
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .text_ellipsis_start()
                        .text_color(theme.colors.text_secondary())
                        .child(path(artifact)),
                )
                .child(
                    text_style(div(), TypeScale::META)
                        .flex_none()
                        .text_color(theme.colors.text_muted())
                        .child(description(artifact, source)),
                ),
        )
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum LineKind {
    Added,
    Removed,
    Hunk,
    Context,
}

fn classify(line: &str, diff: bool) -> LineKind {
    if !diff {
        LineKind::Context
    } else if line.starts_with("@@") {
        LineKind::Hunk
    } else if line.starts_with('+') && !line.starts_with("+++") {
        LineKind::Added
    } else if line.starts_with('-') && !line.starts_with("---") {
        LineKind::Removed
    } else {
        LineKind::Context
    }
}

/// Gutter numbers: removed lines and hunk headers do not advance the new
/// file's numbering, so they get an empty gutter instead of a wrong number.
fn line_numbers(lines: &[String], start: u64, diff: bool) -> Vec<Option<u64>> {
    let mut next = start;
    lines
        .iter()
        .map(|line| match classify(line, diff) {
            LineKind::Removed | LineKind::Hunk => None,
            LineKind::Added | LineKind::Context => {
                let number = next;
                next = next.saturating_add(1);
                Some(number)
            }
        })
        .collect()
}

/// The source body: virtualised code rows, or wrapped prose for text.
pub(super) fn body(
    artifact: &ArtifactView,
    id: String,
    source: &SourceLines,
    max_height: f32,
    theme: Theme,
) -> AnyElement {
    let body_id = format!("{id}-body");
    if !is_code(artifact) {
        return div()
            .id(format!("{body_id}-prose"))
            .max_h(px(max_height))
            .w_full()
            .overflow_y_scroll()
            .p(px(SpacingScale::S4))
            .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
            .child(
                text_style(div(), TypeScale::BODY)
                    .text_color(theme.colors.text_secondary())
                    .child(artifact.content.clone()),
            )
            .into_any_element();
    }
    let diff = artifact.kind == "diff_hunk";
    let lines = source.lines.clone();
    let numbers = Arc::new(line_numbers(
        &lines,
        start_line(artifact).unwrap_or(1),
        diff,
    ));
    let row_height = TypeScale::CODE.line_height;
    let padding = SpacingScale::S2;
    let height = (lines.len() as f32 * row_height).min(max_height - padding * 2.0);
    let code_list = uniform_list(id, lines.len(), move |range, _, _| {
        range
            .map(|offset| {
                let line = &lines[offset];
                let kind = classify(line, diff);
                div()
                    .h(px(row_height))
                    .w_full()
                    .flex()
                    .min_w(px(0.0))
                    .when(kind == LineKind::Added, |row| {
                        row.bg(theme.colors.diff_added())
                    })
                    .when(kind == LineKind::Removed, |row| {
                        row.bg(theme.colors.diff_removed())
                    })
                    .child(
                        code_style(div(), TypeScale::CODE)
                            .w(px(52.0))
                            .flex_none()
                            .pr(px(SpacingScale::S4))
                            .text_right()
                            .text_color(theme.colors.text_disabled())
                            .child(
                                numbers[offset]
                                    .map(|number| number.to_string())
                                    .unwrap_or_default(),
                            ),
                    )
                    .child(
                        code_style(div(), TypeScale::CODE)
                            .whitespace_nowrap()
                            .pr(px(SpacingScale::S4))
                            .text_color(match kind {
                                LineKind::Added => theme.colors.text_primary(),
                                LineKind::Removed => theme.colors.text_muted(),
                                LineKind::Hunk => theme.colors.status_info(),
                                LineKind::Context => theme.colors.text_secondary(),
                            })
                            .child(if line.is_empty() {
                                " ".to_owned()
                            } else {
                                line.to_owned()
                            }),
                    )
            })
            .collect::<Vec<_>>()
    })
    .with_width_from_item(Some(source.widest))
    .with_horizontal_sizing_behavior(ListHorizontalSizingBehavior::Unconstrained)
    .h(px(height))
    .w_full();
    div()
        .id(body_id)
        .w_full()
        .min_w(px(0.0))
        .overflow_hidden()
        .py(px(padding))
        .on_scroll_wheel(|_, _, cx| cx.stop_propagation())
        .child(code_list)
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn uses_recorded_file_and_falls_back_without_inventing_a_path() {
        let mut source = ArtifactView {
            artifact_id: "a".into(),
            kind: "diff_hunk".into(),
            content: "fn main() {}".into(),
            metadata: r#"{"file":"src/inbox.rs"}"#.into(),
        };
        assert_eq!(label(&source), "inbox.rs");
        source.metadata = "broken".into();
        assert_eq!(label(&source), "Trecho de alteração");
    }

    #[test]
    fn diff_gutter_skips_removed_lines_and_hunk_headers() {
        let lines: Vec<String> = ["@@ -1,2 +1,2 @@", " keep", "-old", "+new", " tail"]
            .into_iter()
            .map(str::to_owned)
            .collect();
        assert_eq!(
            line_numbers(&lines, 10, true),
            vec![None, Some(10), None, Some(11), Some(12)]
        );
        assert_eq!(
            line_numbers(&lines, 1, false),
            vec![Some(1), Some(2), Some(3), Some(4), Some(5)]
        );
    }
}
