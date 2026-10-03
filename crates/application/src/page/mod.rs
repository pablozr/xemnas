//! The overview as one self-contained HTML page: a reading made to be
//! understood in a minute, the second way (after the in-app view) to see
//! what a project is.
//!
//! The page holds no model-written markup. Rust serialises the stored
//! overview (summary, flows, architecture) as JSON into a fixed template, and
//! the template draws everything from that data: the summary with its source
//! chips, the architecture as a navigable diagram (pan, zoom, one flow at a
//! time, what each part calls and is called by), each flow as a sequence
//! diagram with its steps, and the list of decisions and rules it rests on.
//! Nothing is fetched: fonts, scripts and styles are inline, so the file opens
//! offline and can be sent to a colleague.

use crate::overview::ProjectOverview;

const TEMPLATE: &str = include_str!("template.html");
const DATA_MARK: &str = "/*__DATA__*/";
const TITLE_MARK: &str = "/*__TITLE__*/";

/// The page for `overview` of the project called `project`.
pub fn render(project: &str, overview: &ProjectOverview) -> String {
    let mut data = serde_json::to_value(overview).unwrap_or_default();
    if let Some(map) = data.as_object_mut() {
        map.insert("project".into(), project.into());
    }
    let title = if project.trim().is_empty() {
        "Visão do projeto · Xemnas".to_owned()
    } else {
        format!("{} · Visão · Xemnas", escape_text(project))
    };
    TEMPLATE
        .replacen(TITLE_MARK, &title, 1)
        .replacen(DATA_MARK, &script_safe(&data.to_string()), 1)
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
        "visao.html".into()
    } else {
        format!("visao-{slug}.html")
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
    use crate::overview::{Citation, OverviewParagraph};

    fn overview(text: &str) -> ProjectOverview {
        ProjectOverview {
            project_id: "p".into(),
            generated_at: "2026-09-29T10:00:00Z".into(),
            decisions: 2,
            rules: 1,
            documents: 0,
            summary: vec![OverviewParagraph {
                text: text.into(),
                citations: vec![Citation {
                    kind: "decision".into(),
                    id: "d1".into(),
                    label: "D:abcd1234".into(),
                    title: "Onde guardar?".into(),
                }],
            }],
            flows: Vec::new(),
            architecture: Architecture::default(),
        }
    }

    #[test]
    fn the_data_and_the_title_land_in_the_template() {
        let page = render("xemnas", &overview("Um projeto."));
        assert!(page.contains("Um projeto."));
        assert!(page.contains("<title>xemnas · Visão · Xemnas</title>"));
        assert!(page.contains("\"project\":\"xemnas\""));
        assert!(!page.contains(DATA_MARK) && !page.contains(TITLE_MARK));
    }

    #[test]
    fn text_from_the_model_cannot_close_the_script_or_the_title() {
        let hostile = "</script><script>alert(1)</script><!-- x";
        let page = render("</title><b>", &overview(hostile));
        let data_start = page.find("id=\"xemnas-data\">").unwrap();
        let data_end = data_start + page[data_start..].find("</script>").unwrap();
        let data = &page[data_start..data_end];
        assert!(!data.to_lowercase().contains("</script") && !data.contains("<!--"));
        assert!(page.contains("<title>&lt;/title&gt;&lt;b&gt; · Visão · Xemnas</title>"));
        assert_eq!(
            page.matches("</script>").count(),
            2,
            "only the template's own"
        );
    }

    #[test]
    fn file_names_are_plain() {
        assert_eq!(file_name("Meu Projeto / API"), "visao-meu-projeto-api.html");
        assert_eq!(file_name("///"), "visao.html");
    }
}
