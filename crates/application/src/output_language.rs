//! The language the AI writes its prose in. The caller (the app) knows the
//! language of its interface and passes it in; the backend never reads UI
//! files. The content of the records is never translated: only what the
//! model writes anew, such as the overview of a project.

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
    use super::OutputLanguage;

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
