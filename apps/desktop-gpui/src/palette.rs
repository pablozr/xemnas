//! Command palette data: what can be reached and how a query narrows it.
//!
//! The shell builds the items from data it already has (projects, loaded
//! candidates, loaded decisions, real actions) and renders them; nothing here
//! invents entries or searches beyond what is loaded.

use crate::ui::icons::IconName;

/// One reachable thing.
#[derive(Clone, Debug)]
pub struct PaletteItem<C> {
    /// Section heading the item is listed under.
    pub group: &'static str,
    /// What the person reads and types.
    pub label: String,
    /// Secondary text, matched too (a path, a state).
    pub detail: Option<String>,
    /// Glyph shown before the label.
    pub glyph: IconName,
    /// Keyboard shortcut that does the same thing, if any.
    pub shortcut: Option<&'static str>,
    /// What running the item does.
    pub command: C,
}

/// Lowercases and folds Portuguese diacritics, so "decisao" finds "Decisão".
pub fn normalize(text: &str) -> String {
    text.chars()
        .flat_map(char::to_lowercase)
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ã' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            other => other,
        })
        .collect()
}

/// Items whose label or detail contains every word of the query, in the
/// original order (groups stay together).
pub fn filter<C: Clone>(items: &[PaletteItem<C>], query: &str) -> Vec<PaletteItem<C>> {
    let words: Vec<String> = normalize(query)
        .split_whitespace()
        .map(str::to_owned)
        .collect();
    items
        .iter()
        .filter(|item| {
            let haystack = normalize(&format!(
                "{} {}",
                item.label,
                item.detail.as_deref().unwrap_or("")
            ));
            words.iter().all(|word| haystack.contains(word.as_str()))
        })
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(label: &str, detail: Option<&str>) -> PaletteItem<u8> {
        PaletteItem {
            group: "g",
            label: label.into(),
            detail: detail.map(str::to_owned),
            glyph: IconName::File,
            shortcut: None,
            command: 0,
        }
    }

    #[test]
    fn folds_accents_and_case() {
        assert_eq!(normalize("Decisão Ação"), "decisao acao");
    }

    #[test]
    fn every_word_must_match_label_or_detail() {
        let items = [
            item("Como versionar decisões?", Some("Confirmada")),
            item("kpi-front", Some("C:/Projects/kpi-front")),
        ];
        assert_eq!(filter(&items, "versionar decisoes").len(), 1);
        assert_eq!(filter(&items, "projects kpi").len(), 1);
        assert_eq!(filter(&items, "versionar kpi").len(), 0);
        assert_eq!(filter(&items, "").len(), 2);
    }
}
