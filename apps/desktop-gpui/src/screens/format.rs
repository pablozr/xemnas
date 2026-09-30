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
    }
}
