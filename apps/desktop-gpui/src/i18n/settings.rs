//! Copy for the settings area. See [`crate::i18n`] for how entries are declared.

use super::{formats, strings, Language};

strings! {
    /// Settings section and card: the interface language.
    language_title { en: "Language", pt: "Idioma", es: "Idioma", fr: "Langue",
        de: "Sprache", it: "Lingua", ja: "言語", zh: "语言", ko: "언어", ru: "Язык" }
    /// Subtitle of the language section.
    language_subtitle { en: "The language of every screen.",
        pt: "O idioma de todas as telas.",
        es: "El idioma de todas las pantallas.",
        fr: "La langue de tous les écrans.",
        de: "Die Sprache aller Ansichten.",
        it: "La lingua di tutte le schermate.",
        ja: "すべての画面の表示言語。",
        zh: "所有界面的显示语言。",
        ko: "모든 화면의 표시 언어.",
        ru: "Язык всех экранов." }
    /// What the language card does.
    language_card_body {
        en: "Changes right away and is saved for next time. What you write and capture stays as written.",
        pt: "Muda na hora e fica salvo para a próxima vez. O que você escreve e captura continua como foi escrito.",
        es: "Cambia al instante y se guarda para la próxima vez. Lo que escribes y capturas queda tal como se escribió.",
        fr: "S’applique tout de suite et reste enregistré. Ce que vous écrivez et capturez reste tel quel.",
        de: "Gilt sofort und bleibt gespeichert. Was du schreibst und erfasst, bleibt so, wie es geschrieben wurde.",
        it: "Cambia subito e resta salvata per la prossima volta. Ciò che scrivi e catturi resta com’è stato scritto.",
        ja: "すぐに切り替わり、次回も保持されます。あなたが書いた内容や取り込んだ内容はそのまま残ります。",
        zh: "立即生效，并保存到下次使用。你写下和捕获的内容保持原样。",
        ko: "즉시 바뀌고 다음에도 유지됩니다. 직접 쓰거나 수집한 내용은 쓴 그대로 남습니다.",
        ru: "Меняется сразу и сохраняется на следующий раз. То, что вы пишете и сохраняете, остаётся как есть." }
    /// Palette entry that opens the language section.
    language_palette { en: "Change language", pt: "Mudar idioma", es: "Cambiar idioma",
        fr: "Changer de langue", de: "Sprache ändern", it: "Cambia lingua",
        ja: "言語を変更", zh: "更改语言", ko: "언어 변경", ru: "Сменить язык" }
}

formats! {
    /// Where a palette entry leads inside Settings: `Settings › Language`.
    settings_at(section: &str) { en: "Settings › {section}", pt: "Configurações › {section}",
        es: "Ajustes › {section}", fr: "Réglages › {section}", de: "Einstellungen › {section}",
        it: "Impostazioni › {section}", ja: "設定 › {section}", zh: "设置 › {section}",
        ko: "설정 › {section}", ru: "Настройки › {section}" }
}

/// `language` named in the interface language: `Portuguese (Brazil)` while
/// the app reads English. The picker shows it beside the native name.
pub fn language_name(language: Language) -> &'static str {
    use Language::*;
    let names: [&'static str; 10] = match super::current() {
        English => [
            "English",
            "Portuguese (Brazil)",
            "Spanish",
            "French",
            "German",
            "Italian",
            "Japanese",
            "Chinese (Simplified)",
            "Korean",
            "Russian",
        ],
        Portuguese => [
            "Inglês",
            "Português (Brasil)",
            "Espanhol",
            "Francês",
            "Alemão",
            "Italiano",
            "Japonês",
            "Chinês (simplificado)",
            "Coreano",
            "Russo",
        ],
        Spanish => [
            "Inglés",
            "Portugués (Brasil)",
            "Español",
            "Francés",
            "Alemán",
            "Italiano",
            "Japonés",
            "Chino (simplificado)",
            "Coreano",
            "Ruso",
        ],
        French => [
            "Anglais",
            "Portugais (Brésil)",
            "Espagnol",
            "Français",
            "Allemand",
            "Italien",
            "Japonais",
            "Chinois (simplifié)",
            "Coréen",
            "Russe",
        ],
        German => [
            "Englisch",
            "Portugiesisch (Brasilien)",
            "Spanisch",
            "Französisch",
            "Deutsch",
            "Italienisch",
            "Japanisch",
            "Chinesisch (vereinfacht)",
            "Koreanisch",
            "Russisch",
        ],
        Italian => [
            "Inglese",
            "Portoghese (Brasile)",
            "Spagnolo",
            "Francese",
            "Tedesco",
            "Italiano",
            "Giapponese",
            "Cinese (semplificato)",
            "Coreano",
            "Russo",
        ],
        Japanese => [
            "英語",
            "ポルトガル語（ブラジル）",
            "スペイン語",
            "フランス語",
            "ドイツ語",
            "イタリア語",
            "日本語",
            "中国語（簡体字）",
            "韓国語",
            "ロシア語",
        ],
        Chinese => [
            "英语",
            "葡萄牙语（巴西）",
            "西班牙语",
            "法语",
            "德语",
            "意大利语",
            "日语",
            "简体中文",
            "韩语",
            "俄语",
        ],
        Korean => [
            "영어",
            "포르투갈어(브라질)",
            "스페인어",
            "프랑스어",
            "독일어",
            "이탈리아어",
            "일본어",
            "중국어(간체)",
            "한국어",
            "러시아어",
        ],
        Russian => [
            "Английский",
            "Португальский (Бразилия)",
            "Испанский",
            "Французский",
            "Немецкий",
            "Итальянский",
            "Японский",
            "Китайский (упрощённый)",
            "Корейский",
            "Русский",
        ],
    };
    names[language as usize]
}
