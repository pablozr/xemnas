//! Copy shared by the UI layer (`ui::*`): theme names and the vocabulary of
//! reusable patterns. See [`crate::i18n`] for how entries are declared.

use super::{formats, strings};

strings! {
    /// Theme: the original bluish graphite.
    theme_quiet_glass { en: "Quiet Glass", pt: "Quiet Glass", es: "Quiet Glass",
        fr: "Quiet Glass", de: "Quiet Glass", it: "Quiet Glass", ja: "Quiet Glass",
        zh: "Quiet Glass", ko: "Quiet Glass", ru: "Quiet Glass" }
    /// Blurb of the Quiet Glass theme.
    theme_quiet_glass_blurb { en: "Bluish graphite with lavender. The original.",
        pt: "Grafite azulado com lavanda. O original.",
        es: "Grafito azulado con lavanda. El original.",
        fr: "Graphite bleuté et lavande. L’original.",
        de: "Bläuliches Graphit mit Lavendel. Das Original.",
        it: "Grafite bluastra con lavanda. L’originale.",
        ja: "青みがかったグラファイトとラベンダー。オリジナル。",
        zh: "带蓝调的石墨灰配薰衣草紫。原版。",
        ko: "푸른빛 그래파이트와 라벤더. 오리지널.",
        ru: "Голубоватый графит с лавандой. Оригинал." }
    /// Theme: neutral charcoal.
    theme_charcoal { en: "Charcoal", pt: "Carvão", es: "Carbón", fr: "Charbon",
        de: "Anthrazit", it: "Carbone", ja: "チャコール", zh: "炭黑", ko: "차콜", ru: "Уголь" }
    /// Blurb of the Charcoal theme.
    theme_charcoal_blurb { en: "Neutral, editorial charcoal with lavender.",
        pt: "Carvão neutro, editorial, com lavanda.",
        es: "Carbón neutro y editorial, con lavanda.",
        fr: "Charbon neutre et éditorial, avec lavande.",
        de: "Neutrales, redaktionelles Anthrazit mit Lavendel.",
        it: "Carbone neutro ed editoriale, con lavanda.",
        ja: "ニュートラルで端正なチャコールとラベンダー。",
        zh: "中性、杂志感的炭黑配薰衣草紫。",
        ko: "중립적이고 단정한 차콜과 라벤더.",
        ru: "Нейтральный строгий уголь с лавандой." }
    /// Theme: deep black and cold silver.
    theme_organization { en: "Organization", pt: "Organização", es: "Organización",
        fr: "Organisation", de: "Organisation", it: "Organizzazione", ja: "機関",
        zh: "组织", ko: "기관", ru: "Организация" }
    /// Blurb of the Organization theme.
    theme_organization_blurb { en: "Deep black and cold silver, like the coats.",
        pt: "Preto profundo e prata fria, como os casacos.",
        es: "Negro profundo y plata fría, como los abrigos.",
        fr: "Noir profond et argent froid, comme les manteaux.",
        de: "Tiefes Schwarz und kühles Silber, wie die Mäntel.",
        it: "Nero profondo e argento freddo, come i cappotti.",
        ja: "あのコートのような深い黒と冷たい銀。",
        zh: "深邃的黑与冷银，如同那些大衣。",
        ko: "그 코트처럼 깊은 검정과 차가운 은색.",
        ru: "Глубокий чёрный и холодное серебро, как те плащи." }
    /// Theme: ink green with sage.
    theme_moss { en: "Moss", pt: "Musgo", es: "Musgo", fr: "Mousse", de: "Moos",
        it: "Muschio", ja: "モス", zh: "苔绿", ko: "모스", ru: "Мох" }
    /// Blurb of the Moss theme.
    theme_moss_blurb { en: "Ink green with sage. Calm and organic.",
        pt: "Verde-tinta com sálvia. Calmo e orgânico.",
        es: "Verde tinta con salvia. Tranquilo y orgánico.",
        fr: "Vert encre et sauge. Calme et organique.",
        de: "Tintengrün mit Salbei. Ruhig und organisch.",
        it: "Verde inchiostro con salvia. Calmo e organico.",
        ja: "インクグリーンとセージ。穏やかで自然。",
        zh: "墨绿配鼠尾草绿。沉静而自然。",
        ko: "잉크 그린과 세이지. 차분하고 자연스러운.",
        ru: "Чернильно-зелёный с шалфеем. Спокойно и естественно." }
    /// Theme: navy with a cold cyan.
    theme_midnight { en: "Midnight", pt: "Meia-noite", es: "Medianoche", fr: "Minuit",
        de: "Mitternacht", it: "Mezzanotte", ja: "ミッドナイト", zh: "午夜", ko: "미드나잇",
        ru: "Полночь" }
    /// Blurb of the Midnight theme.
    theme_midnight_blurb { en: "Navy blue with a cool, quiet cyan.",
        pt: "Azul-marinho com um ciano frio e discreto.",
        es: "Azul marino con un cian frío y discreto.",
        fr: "Bleu marine et cyan froid et discret.",
        de: "Marineblau mit einem kühlen, dezenten Cyan.",
        it: "Blu navy con un ciano freddo e discreto.",
        ja: "ネイビーに控えめで冷たいシアン。",
        zh: "海军蓝配一抹冷静低调的青色。",
        ko: "네이비에 차갑고 은은한 시안.",
        ru: "Тёмно-синий с холодным сдержанным бирюзовым." }
}

strings! {
    /// Status label: confirmed.
    status_success { en: "Confirmed", pt: "Confirmado", es: "Confirmado", fr: "Confirmé",
        de: "Bestätigt", it: "Confermato", ja: "確定済み", zh: "已确认", ko: "확인됨",
        ru: "Подтверждено" }
    /// Status label: pending.
    status_warning { en: "Pending", pt: "Pendente", es: "Pendiente", fr: "En attente",
        de: "Ausstehend", it: "In sospeso", ja: "保留中", zh: "待处理", ko: "대기 중",
        ru: "Ожидает" }
    /// Status label: error.
    status_danger { en: "Error", pt: "Erro", es: "Error", fr: "Erreur", de: "Fehler",
        it: "Errore", ja: "エラー", zh: "错误", ko: "오류", ru: "Ошибка" }
    /// Status label: information.
    status_info { en: "Information", pt: "Informação", es: "Información", fr: "Information",
        de: "Information", it: "Informazioni", ja: "情報", zh: "信息", ko: "정보",
        ru: "Информация" }
    /// Heading above the quote that gave rise to a suggestion.
    suggestion_source { en: "Excerpt that gave rise to the suggestion",
        pt: "Trecho que originou a sugestão",
        es: "Fragmento que originó la sugerencia",
        fr: "Extrait à l’origine de la suggestion",
        de: "Auszug, aus dem der Vorschlag stammt",
        it: "Brano da cui nasce il suggerimento",
        ja: "提案のもとになった一節",
        zh: "产生该建议的片段",
        ko: "제안의 출처가 된 구절",
        ru: "Фрагмент, из которого возникло предложение" }
    /// Accessible name of a loading skeleton.
    loading { en: "Loading", pt: "Carregando", es: "Cargando", fr: "Chargement",
        de: "Wird geladen", it: "Caricamento", ja: "読み込み中", zh: "加载中",
        ko: "불러오는 중", ru: "Загрузка" }
    /// Default placeholder and accessible name of a search field.
    search_projects { en: "Search projects", pt: "Buscar projetos", es: "Buscar proyectos",
        fr: "Rechercher des projets", de: "Projekte suchen", it: "Cerca progetti",
        ja: "プロジェクトを検索", zh: "搜索项目", ko: "프로젝트 검색", ru: "Искать проекты" }
    /// Hint inside a focused search field.
    search_clear_hint { en: "Esc · clear", pt: "Esc · limpar", es: "Esc · borrar",
        fr: "Échap · effacer", de: "Esc · leeren", it: "Esc · cancella", ja: "Esc · クリア",
        zh: "Esc · 清除", ko: "Esc · 지우기", ru: "Esc · очистить" }
    /// Built-in background "never".
    wallpaper_never { en: "The city that never existed", pt: "A cidade que nunca existiu",
        es: "La ciudad que nunca existió", fr: "La ville qui n’a jamais existé",
        de: "Die Stadt, die es nie gab", it: "La città che non è mai esistita",
        ja: "存在しなかった街", zh: "从未存在的城市", ko: "존재한 적 없는 도시",
        ru: "Город, которого никогда не было" }
    /// Built-in background "castle".
    wallpaper_castle { en: "The castle", pt: "O castelo", es: "El castillo",
        fr: "Le château", de: "Das Schloss", it: "Il castello", ja: "城", zh: "城堡",
        ko: "성", ru: "Замок" }
    /// Built-in background "thirteen".
    wallpaper_thirteen { en: "Where nothing gathers", pt: "Onde nada se reúne",
        es: "Donde nada se reúne", fr: "Là où rien ne se rassemble",
        de: "Wo nichts zusammenkommt", it: "Dove nulla si riunisce",
        ja: "何も集まらない場所", zh: "万物不聚之处", ko: "아무것도 모이지 않는 곳",
        ru: "Где ничто не собирается" }
    /// Built-in background "chain".
    wallpaper_chain { en: "Chain", pt: "Corrente", es: "Cadena", fr: "Chaîne", de: "Kette",
        it: "Catena", ja: "鎖", zh: "锁链", ko: "사슬", ru: "Цепь" }
}

formats! {
    /// Explains what confirming a suggestion will do.
    on_confirm(effect: &str) {
        en: "On confirming, {effect}", pt: "Ao confirmar, {effect}",
        es: "Al confirmar, {effect}", fr: "En confirmant, {effect}",
        de: "Beim Bestätigen {effect}", it: "Alla conferma, {effect}",
        ja: "確定すると、{effect}", zh: "确认后，{effect}", ko: "확인하면 {effect}",
        ru: "При подтверждении {effect}" }
    /// Position in a list: "3 of 40".
    of_total(shown: usize, total: usize) {
        en: "{shown} of {total}", pt: "{shown} de {total}", es: "{shown} de {total}",
        fr: "{shown} sur {total}", de: "{shown} von {total}", it: "{shown} di {total}",
        ja: "{shown} / {total}", zh: "{shown} / {total}", ko: "{shown} / {total}",
        ru: "{shown} из {total}" }
}

/// A count with its noun, from one `[one, few, many]` triple per language in
/// [`Language::ALL`] order. Only Russian tells `few` from `many`; French
/// counts 0 as singular; Japanese, Chinese and Korean attach the counter to
/// the number without a space and do not inflect.
pub fn counted(n: usize, forms: [[&'static str; 3]; 10]) -> String {
    use super::Language;

    let language = super::current();
    let [one_form, few_form, many_form] = forms[language as usize];
    match language {
        Language::Japanese | Language::Chinese | Language::Korean => {
            format!("{n}{many_form}")
        }
        Language::Russian => {
            let form = [one_form, few_form, many_form][super::russian_form(n)];
            format!("{n} {form}")
        }
        Language::French if super::french_one(n) => format!("{n} {one_form}"),
        _ if language != Language::French && super::one(n) => format!("{n} {one_form}"),
        _ => format!("{n} {many_form}"),
    }
}
