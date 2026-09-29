//! Presentation of redacted capture artifacts, using recorded provenance only.

use crate::ui::theme::{code_style, text_style, Theme};
use crate::ui::tokens::{SpacingScale, TypeScale};
use application::inbox::ArtifactView;
use gpui::prelude::*;
use gpui::{div, px, AnyElement};

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
        _ => "Fonte da captura",
    }
}

pub(super) fn snippet(artifact: &ArtifactView, index: usize) -> AnyElement {
    let theme = Theme::quiet_glass();
    let metadata = metadata(artifact);
    let path = metadata
        .get("file")
        .or_else(|| metadata.get("path"))
        .and_then(|value| value.as_str());
    let start = metadata
        .get("start_line")
        .or_else(|| metadata.get("line_start"))
        .and_then(|value| value.as_u64())
        .filter(|line| *line > 0);
    let code = artifact.kind == "diff_hunk" || metadata.get("language").is_some();
    let lines: Vec<_> = artifact
        .content
        .lines()
        .enumerate()
        .map(|(offset, line)| {
            div()
                .flex()
                .min_w(px(0.0))
                .child(
                    code_style(div(), TypeScale::CODE)
                        .w(px(48.0))
                        .flex_none()
                        .text_color(theme.colors.text_muted())
                        .child(start.unwrap_or(1).saturating_add(offset as u64).to_string()),
                )
                .child(
                    code_style(div(), TypeScale::CODE)
                        .whitespace_nowrap()
                        .text_color(if line.starts_with('+') && !line.starts_with("+++") {
                            theme.colors.accent_emphasis()
                        } else {
                            theme.colors.text_secondary()
                        })
                        .child(if line.is_empty() {
                            " ".to_owned()
                        } else {
                            line.to_owned()
                        }),
                )
        })
        .collect();
    div()
        .w_full()
        .min_w(px(0.0))
        .flex()
        .flex_col()
        .rounded(theme.radius.control())
        .border_1()
        .border_color(theme.colors.glass_border())
        .bg(theme.colors.rail())
        .child(
            div()
                .p(px(SpacingScale::S3))
                .flex()
                .flex_col()
                .gap(px(SpacingScale::S1))
                .border_b_1()
                .border_color(theme.colors.hairline_divider())
                .child(
                    text_style(div(), TypeScale::BODY_SMALL)
                        .child(path.map(str::to_owned).unwrap_or_else(|| label(artifact))),
                )
                .child(
                    text_style(div(), TypeScale::META)
                        .text_color(theme.colors.text_muted())
                        .child(format!(
                            "{} · {}",
                            kind_label(&artifact.kind),
                            if start.is_some() {
                                "linhas da fonte"
                            } else if code {
                                "linhas do trecho"
                            } else {
                                "texto da captura"
                            }
                        )),
                ),
        )
        .child(
            div()
                .id(("evidence-code", index))
                .max_h(px(240.0))
                .overflow_y_scroll()
                .overflow_x_scroll()
                .p(px(SpacingScale::S4))
                .children(lines),
        )
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
}
