//! Date presentation shared by every screen: "29 set 2026" in the local zone.
//!
//! Review, Decisions and Details used three formats (`29 set 2026`,
//! `29/09/2026`, `2026-09-29 10:00`) for the same kind of fact.

use chrono::{DateTime, Datelike, Local, Timelike};

const MONTHS: [&str; 12] = [
    "jan", "fev", "mar", "abr", "mai", "jun", "jul", "ago", "set", "out", "nov", "dez",
];

fn local(value: &str) -> Option<DateTime<Local>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|date| date.with_timezone(&Local))
}

fn day(date: &DateTime<Local>) -> String {
    format!(
        "{} {} {}",
        date.day(),
        MONTHS[date.month0() as usize],
        date.year()
    )
}

/// `29 set 2026`; an unparsable value is shown as recorded.
pub(super) fn short_date(value: &str) -> String {
    local(value)
        .map(|date| day(&date))
        .unwrap_or_else(|| value.to_owned())
}

/// `29 set 2026, 10:00`; an unparsable value is shown as recorded.
pub(super) fn date_time(value: &str) -> String {
    local(value)
        .map(|date| format!("{}, {:02}:{:02}", day(&date), date.hour(), date.minute()))
        .unwrap_or_else(|| value.to_owned())
}

/// A calendar date kept as recorded (`2026-09-01…` → `1 set 2026`): rule
/// validity is a day, not an instant, so it never shifts with the zone.
pub(super) fn calendar_date(value: &str) -> String {
    chrono::NaiveDate::parse_from_str(value.get(..10).unwrap_or(value), "%Y-%m-%d")
        .map(|date| {
            format!(
                "{} {} {}",
                date.day(),
                MONTHS[date.month0() as usize],
                date.year()
            )
        })
        .unwrap_or_else(|_| value.to_owned())
}

/// `10:00` in the local zone, or nothing for an unparsable value.
pub(super) fn clock(value: &str) -> Option<String> {
    local(value).map(|date| format!("{:02}:{:02}", date.hour(), date.minute()))
}

/// Day heading of a feed: `Hoje`, `Ontem` or `29 set 2026`.
pub(super) fn day_heading(value: &str) -> String {
    let Some(date) = local(value) else {
        return value.to_owned();
    };
    let today = Local::now().date_naive();
    match (today - date.date_naive()).num_days() {
        0 => "Hoje".to_owned(),
        1 => "Ontem".to_owned(),
        _ => day(&date),
    }
}

/// `text` cut at a word near `max` characters, with an ellipsis.
pub(super) fn clipped(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_owned();
    }
    let cut: String = text.chars().take(max).collect();
    let cut = cut.rsplit_once(' ').map_or(cut.as_str(), |(head, _)| head);
    format!("{}…", cut.trim_end_matches([',', ';', ':', '.']))
}

/// `1 regra`, `3 regras`: the count with the right form of the noun.
pub(super) fn plural(count: usize, one: &str, many: &str) -> String {
    format!("{count} {}", if count == 1 { one } else { many })
}

/// `4` → `IV`: the Organization counts its members in Roman numerals, and
/// the app numbers ordered things (stages, flows, steps) the same way.
pub(super) fn roman(value: usize) -> String {
    const TABLE: [(usize, &str); 13] = [
        (1000, "M"),
        (900, "CM"),
        (500, "D"),
        (400, "CD"),
        (100, "C"),
        (90, "XC"),
        (50, "L"),
        (40, "XL"),
        (10, "X"),
        (9, "IX"),
        (5, "V"),
        (4, "IV"),
        (1, "I"),
    ];
    let mut rest = value;
    let mut out = String::new();
    for (step, glyph) in TABLE {
        while rest >= step {
            out.push_str(glyph);
            rest -= step;
        }
    }
    out
}

/// `8192` → `8.192`.
pub(super) fn thousands(value: usize) -> String {
    let digits = value.to_string();
    let mut out = String::new();
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            out.push('.');
        }
        out.push(digit);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_in_the_local_zone_and_keeps_unparsable_values() {
        let value = "2026-01-02T03:04:05Z";
        let expected = DateTime::parse_from_rfc3339(value)
            .unwrap()
            .with_timezone(&Local);
        assert_eq!(
            short_date(value),
            format!(
                "{} {} {}",
                expected.day(),
                MONTHS[expected.month0() as usize],
                expected.year()
            )
        );
        assert!(date_time(value).starts_with(&short_date(value)));
        assert_eq!(short_date("short"), "short");
        assert_eq!(date_time("short"), "short");
        assert_eq!(calendar_date("2026-09-01T00:00:00Z"), "1 set 2026");
        assert_eq!(calendar_date("2026-09-01"), "1 set 2026");
        assert_eq!(clock("short"), None);
    }

    #[test]
    fn counts_in_roman_numerals() {
        assert_eq!(roman(1), "I");
        assert_eq!(roman(4), "IV");
        assert_eq!(roman(9), "IX");
        assert_eq!(roman(13), "XIII");
        assert_eq!(roman(0), "");
    }

    #[test]
    fn formats_counts_with_thousands_separators() {
        assert_eq!(thousands(0), "0");
        assert_eq!(thousands(999), "999");
        assert_eq!(thousands(8_192), "8.192");
        assert_eq!(thousands(1_000_000), "1.000.000");
    }
}
