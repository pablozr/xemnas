//! The interface language and every piece of copy the app shows.
//!
//! Copy lives in one module per area (`i18n::inbox`, `i18n::settings`, ...)
//! as plain functions: `t::inbox::empty_title()`. Each one is declared with
//! [`strings!`] (fixed text, `&'static str`) or [`formats!`] (text with
//! values, `String`), and both macros demand every [`Language`], so a missing
//! translation is a compile error instead of a blank or an English fallback
//! in the interface. Plurals and other copy that changes shape per language
//! are ordinary functions matching on [`current`].
//!
//! The choice is one process-wide atomic: a lookup is a relaxed load and a
//! `match` over static strings, with no allocation, map or parsing at
//! runtime. Changing it repaints every window (see
//! [`crate::ui::appearance::update`]); text a view already stored (a notice,
//! a prepared row) follows on its next refresh.
//!
//! What the person wrote or captured (decisions, project names, evidence) is
//! content, not copy, and is never translated.

use std::sync::atomic::{AtomicU8, Ordering};

pub mod app;
pub mod assistant;
pub mod common;
pub mod context;
pub mod decisions;
pub mod format;
pub mod graph;
pub mod inbox;
pub mod knowledge_review;
pub mod map;
pub mod overview;
pub mod projects;
pub mod settings;

/// A language the interface is offered in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Language {
    /// English, the default.
    #[default]
    English,
    /// Brazilian Portuguese.
    Portuguese,
    /// Spanish.
    Spanish,
    /// French.
    French,
    /// German.
    German,
    /// Italian.
    Italian,
    /// Japanese.
    Japanese,
    /// Simplified Chinese.
    Chinese,
    /// Korean.
    Korean,
    /// Russian.
    Russian,
}

impl Language {
    /// Every language, in the order the picker lists them.
    pub const ALL: [Self; 10] = [
        Self::English,
        Self::Portuguese,
        Self::Spanish,
        Self::French,
        Self::German,
        Self::Italian,
        Self::Japanese,
        Self::Chinese,
        Self::Korean,
        Self::Russian,
    ];

    /// Stable BCP 47 tag for the saved preference.
    pub fn id(self) -> &'static str {
        match self {
            Self::English => "en",
            Self::Portuguese => "pt-BR",
            Self::Spanish => "es",
            Self::French => "fr",
            Self::German => "de",
            Self::Italian => "it",
            Self::Japanese => "ja",
            Self::Chinese => "zh-CN",
            Self::Korean => "ko",
            Self::Russian => "ru",
        }
    }

    /// The language saved as `id`, if it is still offered.
    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|language| language.id() == id)
    }

    /// The language's name in itself, so anyone can find their own in the
    /// picker whatever the interface currently reads.
    pub fn native_name(self) -> &'static str {
        match self {
            Self::English => "English",
            Self::Portuguese => "Português (Brasil)",
            Self::Spanish => "Español",
            Self::French => "Français",
            Self::German => "Deutsch",
            Self::Italian => "Italiano",
            Self::Japanese => "日本語",
            Self::Chinese => "简体中文",
            Self::Korean => "한국어",
            Self::Russian => "Русский",
        }
    }

    fn from_index(index: u8) -> Self {
        Self::ALL
            .get(usize::from(index))
            .copied()
            .unwrap_or_default()
    }
}

static CURRENT: AtomicU8 = AtomicU8::new(Language::English as u8);

/// The language the interface is in.
pub fn current() -> Language {
    Language::from_index(CURRENT.load(Ordering::Relaxed))
}

/// Switches the interface language. Callers repaint the windows.
pub fn set(language: Language) {
    CURRENT.store(language as u8, Ordering::Relaxed);
}

/// Picks the form for `n` in languages with one singular and one plural
/// (English, Portuguese, Spanish, German, Italian). French also treats 0 as
/// singular; see [`french_one`].
pub fn one(n: usize) -> bool {
    n == 1
}

/// French singular: 0 and 1.
pub fn french_one(n: usize) -> bool {
    n <= 1
}

/// Russian plural category: `0` one (1, 21, 31...), `1` few (2-4, 22-24...),
/// `2` many (everything else, including 11-14).
pub fn russian_form(n: usize) -> usize {
    let (tens, units) = (n % 100, n % 10);
    if units == 1 && tens != 11 {
        0
    } else if (2..=4).contains(&units) && !(12..=14).contains(&tens) {
        1
    } else {
        2
    }
}

/// Declares fixed copy: one `fn name() -> &'static str` per entry, with a
/// literal for every language.
///
/// ```ignore
/// strings! {
///     /// The empty inbox.
///     empty_title { en: "Nothing to review", pt: "Nada para revisar", es: "...",
///         fr: "...", de: "...", it: "...", ja: "...", zh: "...", ko: "...", ru: "..." }
/// }
/// ```
macro_rules! strings {
    ($(
        $(#[$meta:meta])*
        $name:ident {
            en: $en:literal, pt: $pt:literal, es: $es:literal, fr: $fr:literal,
            de: $de:literal, it: $it:literal, ja: $ja:literal, zh: $zh:literal,
            ko: $ko:literal, ru: $ru:literal $(,)?
        }
    )*) => {$(
        $(#[$meta])*
        pub fn $name() -> &'static str {
            match $crate::i18n::current() {
                $crate::i18n::Language::English => $en,
                $crate::i18n::Language::Portuguese => $pt,
                $crate::i18n::Language::Spanish => $es,
                $crate::i18n::Language::French => $fr,
                $crate::i18n::Language::German => $de,
                $crate::i18n::Language::Italian => $it,
                $crate::i18n::Language::Japanese => $ja,
                $crate::i18n::Language::Chinese => $zh,
                $crate::i18n::Language::Korean => $ko,
                $crate::i18n::Language::Russian => $ru,
            }
        }
    )*};
}

/// Declares copy with values: one `fn name(args) -> String` per entry. Each
/// literal is a `format!` string that names the arguments inline
/// (`"{count} decisions"`), so translations may reorder them freely.
///
/// ```ignore
/// formats! {
///     /// Window title.
///     title(project: &str) { en: "{project} — xemnas", pt: "{project} — xemnas", ... }
/// }
/// ```
macro_rules! formats {
    ($(
        $(#[$meta:meta])*
        $name:ident ( $($arg:ident : $ty:ty),* $(,)? ) {
            en: $en:literal, pt: $pt:literal, es: $es:literal, fr: $fr:literal,
            de: $de:literal, it: $it:literal, ja: $ja:literal, zh: $zh:literal,
            ko: $ko:literal, ru: $ru:literal $(,)?
        }
    )*) => {$(
        $(#[$meta])*
        // Some entries name many values; the arity is the copy's, not a design smell.
        #[allow(clippy::too_many_arguments)]
        pub fn $name($($arg: $ty),*) -> String {
            match $crate::i18n::current() {
                $crate::i18n::Language::English => format!($en),
                $crate::i18n::Language::Portuguese => format!($pt),
                $crate::i18n::Language::Spanish => format!($es),
                $crate::i18n::Language::French => format!($fr),
                $crate::i18n::Language::German => format!($de),
                $crate::i18n::Language::Italian => format!($it),
                $crate::i18n::Language::Japanese => format!($ja),
                $crate::i18n::Language::Chinese => format!($zh),
                $crate::i18n::Language::Korean => format!($ko),
                $crate::i18n::Language::Russian => format!($ru),
            }
        }
    )*};
}

pub(crate) use {formats, strings};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_round_trip_and_english_is_the_default() {
        for language in Language::ALL {
            assert_eq!(Language::from_id(language.id()), Some(language));
        }
        assert_eq!(Language::from_id("xx"), None);
        assert_eq!(Language::default(), Language::English);
        assert_eq!(current(), Language::English);
    }

    #[test]
    fn formatted_copy_names_its_arguments() {
        assert_eq!(settings::settings_at("Language"), "Settings › Language");
    }

    #[test]
    fn russian_plural_categories() {
        let forms: Vec<usize> = [1, 2, 4, 5, 11, 12, 14, 21, 22, 25, 101, 111]
            .into_iter()
            .map(russian_form)
            .collect();
        assert_eq!(forms, [0, 1, 1, 2, 2, 2, 2, 0, 1, 2, 0, 2]);
    }
}
