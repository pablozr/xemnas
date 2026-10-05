//! Date presentation shared by every screen: "Sep 29, 2026" in the local zone
//! and the interface language ([`crate::i18n::format`]).
//!
//! Review, Decisions and Details used three formats (`29 set 2026`,
//! `29/09/2026`, `2026-09-29 10:00`) for the same kind of fact.

use chrono::{DateTime, Datelike, Local, Timelike};

use crate::i18n::format::{self as t, Elapsed};

fn local(value: &str) -> Option<DateTime<Local>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|date| date.with_timezone(&Local))
}

fn day(date: &DateTime<Local>) -> String {
    t::date(date.day(), date.month0(), date.year())
}

/// `Sep 29, 2026`; an unparsable value is shown as recorded.
pub(super) fn short_date(value: &str) -> String {
    local(value)
        .map(|date| day(&date))
        .unwrap_or_else(|| value.to_owned())
}

/// How long ago, in the shortest honest form: `now`, `17 min`, `2 h`,
/// `3 d`; a week or more (or a future time) is the date, `Sep 29, 2026`.
pub(super) fn relative(value: &str) -> String {
    relative_at(value, Local::now())
}

fn relative_at(value: &str, now: DateTime<Local>) -> String {
    let Some(date) = local(value) else {
        return value.to_owned();
    };
    let seconds = (now - date).num_seconds();
    match seconds {
        s if s < 0 => day(&date),
        s if s < 60 => t::just_now().to_owned(),
        s if s < 3600 => t::elapsed(s / 60, Elapsed::Minutes),
        s if s < 86_400 => t::elapsed(s / 3600, Elapsed::Hours),
        s if s < 7 * 86_400 => t::elapsed(s / 86_400, Elapsed::Days),
        _ => day(&date),
    }
}

/// `Sep 29, 2026, 10:00`; an unparsable value is shown as recorded.
pub(super) fn date_time(value: &str) -> String {
    local(value)
        .map(|date| format!("{}, {:02}:{:02}", day(&date), date.hour(), date.minute()))
        .unwrap_or_else(|| value.to_owned())
}

/// A calendar date kept as recorded (`2026-09-01…` → `Sep 1, 2026`): rule
/// validity is a day, not an instant, so it never shifts with the zone.
pub(super) fn calendar_date(value: &str) -> String {
    chrono::NaiveDate::parse_from_str(value.get(..10).unwrap_or(value), "%Y-%m-%d")
        .map(|date| t::date(date.day(), date.month0(), date.year()))
        .unwrap_or_else(|_| value.to_owned())
}

/// `10:00` in the local zone, or nothing for an unparsable value.
pub(super) fn clock(value: &str) -> Option<String> {
    local(value).map(|date| format!("{:02}:{:02}", date.hour(), date.minute()))
}

/// Day heading of a feed: `Today`, `Yesterday` or `Sep 29, 2026`.
pub(super) fn day_heading(value: &str) -> String {
    let Some(date) = local(value) else {
        return value.to_owned();
    };
    let today = Local::now().date_naive();
    match (today - date.date_naive()).num_days() {
        0 => t::today().to_owned(),
        1 => t::yesterday().to_owned(),
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

/// `8192` → `8,192` (`8.192`, `8 192`, by language).
pub(super) fn thousands(value: usize) -> String {
    let separator = t::thousands_separator();
    let digits = value.to_string();
    let mut out = String::new();
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            out.push(separator);
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
            t::date(expected.day(), expected.month0(), expected.year())
        );
        assert!(date_time(value).starts_with(&short_date(value)));
        assert_eq!(short_date("short"), "short");
        assert_eq!(date_time("short"), "short");
        assert_eq!(calendar_date("2026-09-01T00:00:00Z"), "Sep 1, 2026");
        assert_eq!(calendar_date("2026-09-01"), "Sep 1, 2026");
        assert_eq!(clock("short"), None);
    }

    #[test]
    fn says_how_long_ago_for_the_last_week() {
        let now = DateTime::parse_from_rfc3339("2026-10-02T12:00:00Z")
            .unwrap()
            .with_timezone(&Local);
        let ago = |value: &str| relative_at(value, now);
        assert_eq!(ago("2026-10-02T11:59:50Z"), "now");
        assert_eq!(ago("2026-10-02T11:43:00Z"), "17 min");
        assert_eq!(ago("2026-10-02T10:00:00Z"), "2 h");
        assert_eq!(ago("2026-09-29T12:00:00Z"), "3 d");
        assert_eq!(
            ago("2026-09-01T12:00:00Z"),
            short_date("2026-09-01T12:00:00Z")
        );
        assert_eq!(
            ago("2026-10-05T12:00:00Z"),
            short_date("2026-10-05T12:00:00Z")
        );
        assert_eq!(ago("short"), "short");
    }

    #[test]
    fn formats_counts_with_thousands_separators() {
        assert_eq!(thousands(0), "0");
        assert_eq!(thousands(999), "999");
        assert_eq!(thousands(8_192), "8,192");
        assert_eq!(thousands(1_000_000), "1,000,000");
    }
}
