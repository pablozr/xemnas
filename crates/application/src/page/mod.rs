//! The architecture and the flows of a project as one self-contained HTML
//! page: the part of the overview that is easier to understand drawn than
//! written, made to be explored and sent to a colleague.
//!
//! The page holds no model-written markup. Rust serialises the stored
//! overview (flows and architecture, never the summary) as JSON into a fixed
//! template, and the template draws everything from that data: the
//! architecture as a navigable map (pan, zoom, what each part calls and is
//! called by, a guided walk through one flow with the camera following each
//! step), each flow as a sequence diagram with its steps and the records they
//! rest on. Beside them go the decisions in force and the rules valid now
//! ([`knowledge`]), each tied to the parts it touches, so the page can say why
//! a part is built the way it is. Nothing is fetched: the app's fonts (subset, OFL), the exact mark
//! of the product, the styles and the scripts are inline, so the file opens
//! offline.

mod knowledge;

pub use knowledge::{
    assemble, first_sentence, PageDecision, PageEntity, PageKnowledge, PageRelation, PageRule,
    MAX_PAGE_DECISIONS, MAX_PAGE_RULES,
};

use crate::overview::ProjectOverview;

const TEMPLATE: &str = include_str!("template.html");
const MARK_SVG: &str = include_str!("../../../../apps/desktop-gpui/assets/brand/xemnas-mark.svg");
const FONT_INTER: &str = include_str!("fonts/inter.woff.b64");
const FONT_MONO: &str = include_str!("fonts/mono.woff.b64");
const FONT_DISPLAY: &str = include_str!("fonts/display.woff.b64");
const DATA_MARK: &str = "/*__DATA__*/";
const TITLE_MARK: &str = "/*__TITLE__*/";
const LOGO_MARK: &str = "<!--__LOGO__-->";

/// The page for `overview` of the project called `project`, with the
/// decisions and rules that explain it.
pub fn render(project: &str, overview: &ProjectOverview, knowledge: &PageKnowledge) -> String {
    let mut data = serde_json::to_value(overview).unwrap_or_default();
    if let Some(map) = data.as_object_mut() {
        // The summary stays in the app; the page is the architecture and the flows.
        map.remove("summary");
        map.insert("project".into(), project.into());
        map.insert(
            "knowledge".into(),
            serde_json::to_value(knowledge).unwrap_or_default(),
        );
    }
    let title = if project.trim().is_empty() {
        "Arquitetura e fluxos · Xemnas".to_owned()
    } else {
        format!("{} · Arquitetura e fluxos · Xemnas", escape_text(project))
    };
    TEMPLATE
        .replacen(TITLE_MARK, &title, 1)
        .replacen(LOGO_MARK, &logo(), 1)
        .replacen("/*__FONT_INTER__*/", FONT_INTER.trim(), 1)
        .replacen("/*__FONT_MONO__*/", FONT_MONO.trim(), 1)
        .replacen("/*__FONT_DISPLAY__*/", FONT_DISPLAY.trim(), 1)
        .replacen(DATA_MARK, &script_safe(&data.to_string()), 1)
}

/// The product mark, exactly as in the app, ready to sit inline: its ids are
/// prefixed so they cannot meet the page's own gradients.
fn logo() -> String {
    let start = MARK_SVG.find("<defs>").unwrap_or(0);
    let end = MARK_SVG.rfind("</svg>").unwrap_or(MARK_SVG.len());
    let body = MARK_SVG[start..end]
        .replace("id=\"", "id=\"xm-")
        .replace("url(#", "url(#xm-");
    format!(
        "<svg class=\"mark\" viewBox=\"0 0 1024 1024\" role=\"img\" aria-label=\"Xemnas\">{body}</svg>"
    )
}

/// File name for the page of a project, safe on every file system.
pub fn file_name(project: &str) -> String {
    let slug: String = project
        .chars()
        .map(|c| {
            if c.is_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    let slug = slug
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    if slug.is_empty() {
        "arquitetura.html".into()
    } else {
        format!("arquitetura-{slug}.html")
    }
}

/// JSON inside a `<script>` element: nothing in it may close the element or
/// open a comment, and the two line separators break older parsers.
fn script_safe(json: &str) -> String {
    json.replace("</", "<\\/")
        .replace("<!--", "<\\!--")
        .replace('\u{2028}', "\\u2028")
        .replace('\u{2029}', "\\u2029")
}

fn escape_text(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::architecture::Architecture;
    use crate::overview::{Citation, OverviewFlow, OverviewParagraph, OverviewStep};

    fn overview(text: &str) -> ProjectOverview {
        ProjectOverview {
            project_id: "p".into(),
            generated_at: "2026-09-29T10:00:00Z".into(),
            decisions: 2,
            rules: 1,
            documents: 0,
            summary: vec![OverviewParagraph {
                text: "Resumo que fica no app.".into(),
                citations: Vec::new(),
            }],
            flows: vec![OverviewFlow {
                title: text.into(),
                description: text.into(),
                steps: vec![OverviewStep {
                    title: text.into(),
                    text: text.into(),
                    entity_id: None,
                    entity_name: None,
                    via: None,
                    citations: vec![Citation {
                        kind: "decision".into(),
                        id: "d1".into(),
                        label: "D:abcd1234".into(),
                        title: text.into(),
                    }],
                }],
            }],
            architecture: Architecture::default(),
        }
    }

    fn knowledge(text: &str) -> PageKnowledge {
        let entity = PageEntity {
            id: "e1".into(),
            name: text.into(),
            kind: "component".into(),
            edge: "affects".into(),
        };
        PageKnowledge {
            decisions: vec![PageDecision {
                id: "d1".into(),
                label: "D:abcd1234".into(),
                question: text.into(),
                choice: text.into(),
                rationale: text.into(),
                reason: text.into(),
                assumptions: vec![text.into()],
                consequences: vec![text.into()],
                reconsider_when: vec![text.into()],
                scope: vec![text.into()],
                criteria: vec!["data_or_contract".into()],
                entities: vec![entity.clone()],
                containers: vec!["e1".into()],
                relations: vec![PageRelation {
                    kind: "conflicts_with".into(),
                    outgoing: true,
                    other_id: "d2".into(),
                    other_label: "D:ef567890".into(),
                    other_title: text.into(),
                    in_force: true,
                }],
                confirmed_at: "2026-09-01T00:00:00Z".into(),
            }],
            rules: vec![PageRule {
                id: "r1".into(),
                label: "R:1234abcd".into(),
                kind: "constraint".into(),
                statement: text.into(),
                entities: vec![entity],
                containers: vec!["e1".into()],
            }],
        }
    }

    #[test]
    fn the_decisions_ride_along_with_their_context() {
        let page = render(
            "xemnas",
            &overview("x"),
            &knowledge("Outbox porque o app fecha"),
        );
        assert!(page.contains("\"knowledge\":{\"decisions\":[{"));
        assert!(page.contains("\"label\":\"D:abcd1234\""));
        assert!(page.contains("\"reconsider_when\":[\"Outbox porque o app fecha\"]"));
        assert!(page.contains("\"kind\":\"conflicts_with\""));
        assert!(page.contains("\"label\":\"R:1234abcd\""));
    }

    #[test]
    fn the_flows_and_the_title_land_in_the_template_without_the_summary() {
        let page = render(
            "xemnas",
            &overview("Captura até candidato"),
            &knowledge("x"),
        );
        assert!(page.contains("Captura até candidato"));
        assert!(!page.contains("Resumo que fica no app."));
        assert!(page.contains("<title>xemnas · Arquitetura e fluxos · Xemnas</title>"));
        assert!(page.contains("\"project\":\"xemnas\""));
        for mark in [DATA_MARK, TITLE_MARK, LOGO_MARK, "/*__FONT_"] {
            assert!(!page.contains(mark), "{mark} left in the page");
        }
    }

    #[test]
    fn the_page_carries_the_exact_mark_and_the_fonts() {
        let page = render("xemnas", &overview("x"), &knowledge("x"));
        assert!(page.contains("class=\"mark\""));
        assert!(page.contains("id=\"xm-tile\"") && page.contains("url(#xm-chosen)"));
        assert!(
            page.contains("data:font/woff;base64,d09GRg"),
            "woff magic number"
        );
    }

    #[test]
    fn text_from_the_model_cannot_close_the_script_or_the_title() {
        let hostile = "</script><script>alert(1)</script><!-- x\u{2028}y";
        let page = render("</title><b>", &overview(hostile), &knowledge(hostile));
        let data_start = page.find("id=\"xemnas-data\">").unwrap();
        let data_end = data_start + page[data_start..].find("</script>").unwrap();
        let data = &page[data_start..data_end];
        assert!(!data.to_lowercase().contains("</script") && !data.contains("<!--"));
        assert!(!data.contains('\u{2028}'));
        // Every decision and rule field carries the text, escaped: question,
        // choice, rationale, reason, four lists, entity names, the relation's
        // title, the statement, plus the five fields of the flow.
        let escaped = "<\\/script><script>alert(1)<\\/script><\\!-- x\\u2028y";
        assert_eq!(data.matches(escaped).count(), 17);
        assert!(
            page.contains("<title>&lt;/title&gt;&lt;b&gt; · Arquitetura e fluxos · Xemnas</title>")
        );
        assert_eq!(
            page.matches("</script>").count(),
            2,
            "only the template's own"
        );
    }

    #[test]
    fn file_names_are_plain() {
        assert_eq!(
            file_name("Meu Projeto / API"),
            "arquitetura-meu-projeto-api.html"
        );
        assert_eq!(file_name("///"), "arquitetura.html");
    }
}
