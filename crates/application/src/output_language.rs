//! The language the AI writes its prose in. The caller (the app) knows the
//! language of its interface and passes it in; the backend never reads UI
//! files. The content of the records is never translated: only what the
//! model writes anew, such as the overview of a project.

use std::sync::Arc;

/// Where a job reads the language of the interface at the moment it runs, so a
/// change of language applies to the next request without rebuilding anything.
pub type LanguageSource = Arc<dyn Fn() -> OutputLanguage + Send + Sync>;

/// A source that always answers the same language.
pub fn fixed(language: OutputLanguage) -> LanguageSource {
    Arc::new(move || language)
}

/// The first line of a request, naming the language the model writes in.
pub fn header(language: OutputLanguage) -> String {
    format!("Output language: {}\n\n", language.name())
}

/// A language the model is asked to write in, by its English name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OutputLanguage(&'static str);

impl OutputLanguage {
    /// English, the default when the tag is not known.
    pub const ENGLISH: Self = Self("English");

    /// The language of a BCP 47 tag of the interface (`pt-BR`, `zh-CN`, ...);
    /// any other tag reads as English.
    pub fn from_tag(tag: &str) -> Self {
        Self(match tag {
            "pt-BR" => "Brazilian Portuguese",
            "es" => "Spanish",
            "fr" => "French",
            "de" => "German",
            "it" => "Italian",
            "ja" => "Japanese",
            "zh-CN" => "Simplified Chinese",
            "ko" => "Korean",
            "ru" => "Russian",
            _ => return Self::ENGLISH,
        })
    }

    /// The name sent to the model.
    pub fn name(self) -> &'static str {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::{fixed, header, OutputLanguage};

    #[test]
    fn the_header_names_the_language_and_a_fixed_source_repeats_it() {
        let pt = OutputLanguage::from_tag("pt-BR");
        assert_eq!(header(pt), "Output language: Brazilian Portuguese\n\n");
        assert_eq!(fixed(pt)(), pt);
    }

    #[test]
    fn every_interface_tag_has_a_language_and_an_unknown_one_reads_as_english() {
        let names: Vec<&str> = [
            "en", "pt-BR", "es", "fr", "de", "it", "ja", "zh-CN", "ko", "ru",
        ]
        .iter()
        .map(|tag| OutputLanguage::from_tag(tag).name())
        .collect();
        assert_eq!(
            names,
            [
                "English",
                "Brazilian Portuguese",
                "Spanish",
                "French",
                "German",
                "Italian",
                "Japanese",
                "Simplified Chinese",
                "Korean",
                "Russian"
            ]
        );
        assert_eq!(OutputLanguage::from_tag("tlh"), OutputLanguage::ENGLISH);
    }
}
