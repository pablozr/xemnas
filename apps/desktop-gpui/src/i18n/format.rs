//! Dates, durations and numbers in the interface language.

use super::{current, strings, Language};

const MONTHS_EN: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];
const MONTHS_PT: [&str; 12] = [
    "jan", "fev", "mar", "abr", "mai", "jun", "jul", "ago", "set", "out", "nov", "dez",
];
const MONTHS_ES: [&str; 12] = [
    "ene", "feb", "mar", "abr", "may", "jun", "jul", "ago", "sept", "oct", "nov", "dic",
];
const MONTHS_FR: [&str; 12] = [
    "janv.", "févr.", "mars", "avr.", "mai", "juin", "juil.", "août", "sept.", "oct.", "nov.",
    "déc.",
];
const MONTHS_DE: [&str; 12] = [
    "Jan.", "Feb.", "März", "Apr.", "Mai", "Juni", "Juli", "Aug.", "Sept.", "Okt.", "Nov.", "Dez.",
];
const MONTHS_IT: [&str; 12] = [
    "gen", "feb", "mar", "apr", "mag", "giu", "lug", "ago", "set", "ott", "nov", "dic",
];
const MONTHS_RU: [&str; 12] = [
    "янв.",
    "февр.",
    "мар.",
    "апр.",
    "мая",
    "июн.",
    "июл.",
    "авг.",
    "сент.",
    "окт.",
    "нояб.",
    "дек.",
];

/// A calendar day, `month0` counted from zero: `Sep 29, 2026`,
/// `29 set 2026`, `2026年9月29日`.
pub fn date(day: u32, month0: u32, year: i32) -> String {
    let month = month0.min(11) as usize;
    let number = month + 1;
    match current() {
        Language::English => format!("{} {day}, {year}", MONTHS_EN[month]),
        Language::Portuguese => format!("{day} {} {year}", MONTHS_PT[month]),
        Language::Spanish => format!("{day} {} {year}", MONTHS_ES[month]),
        Language::French => format!("{day} {} {year}", MONTHS_FR[month]),
        Language::German => format!("{day}. {} {year}", MONTHS_DE[month]),
        Language::Italian => format!("{day} {} {year}", MONTHS_IT[month]),
        Language::Japanese | Language::Chinese => format!("{year}年{number}月{day}日"),
        Language::Korean => format!("{year}년 {number}월 {day}일"),
        Language::Russian => format!("{day} {} {year}", MONTHS_RU[month]),
    }
}

strings! {
    /// Less than a minute ago.
    just_now { en: "now", pt: "agora", es: "ahora", fr: "à l’instant", de: "jetzt",
        it: "ora", ja: "たった今", zh: "刚刚", ko: "방금", ru: "сейчас" }
    /// Feed heading for today.
    today { en: "Today", pt: "Hoje", es: "Hoy", fr: "Aujourd’hui", de: "Heute",
        it: "Oggi", ja: "今日", zh: "今天", ko: "오늘", ru: "Сегодня" }
    /// Feed heading for yesterday.
    yesterday { en: "Yesterday", pt: "Ontem", es: "Ayer", fr: "Hier", de: "Gestern",
        it: "Ieri", ja: "昨日", zh: "昨天", ko: "어제", ru: "Вчера" }
}

/// Unit of an elapsed time, in its shortest form.
#[derive(Clone, Copy)]
pub enum Elapsed {
    /// Minutes.
    Minutes,
    /// Hours.
    Hours,
    /// Days.
    Days,
}

/// `17 min`, `2 h`, `3 d` and their equivalents; Japanese, Chinese and
/// Korean say "ago", since a bare `3日` reads as a day of the month.
pub fn elapsed(value: i64, unit: Elapsed) -> String {
    let [minutes, hours, days] = match current() {
        Language::English | Language::Portuguese | Language::Spanish | Language::Italian => {
            [" min", " h", " d"]
        }
        Language::French => [" min", " h", " j"],
        Language::German => [" Min.", " Std.", " T."],
        Language::Japanese => ["分前", "時間前", "日前"],
        Language::Chinese => ["分钟前", "小时前", "天前"],
        Language::Korean => ["분 전", "시간 전", "일 전"],
        Language::Russian => [" мин", " ч", " д"],
    };
    let suffix = match unit {
        Elapsed::Minutes => minutes,
        Elapsed::Hours => hours,
        Elapsed::Days => days,
    };
    format!("{value}{suffix}")
}

/// The digit-group separator: `8,192`, `8.192`, `8 192`.
pub fn thousands_separator() -> char {
    match current() {
        Language::English | Language::Japanese | Language::Chinese | Language::Korean => ',',
        Language::Portuguese | Language::Spanish | Language::German | Language::Italian => '.',
        Language::French | Language::Russian => '\u{202F}',
    }
}

/// The separator between a list item and the next: `, ` or `、`.
pub fn list_separator() -> &'static str {
    match current() {
        Language::Japanese | Language::Chinese => "、",
        _ => ", ",
    }
}
