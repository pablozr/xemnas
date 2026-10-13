//! RFC 3339 UTC clock shared by the application use cases.

/// Returns the current time as an RFC 3339 UTC timestamp.
pub(crate) fn now_rfc3339() -> String {
    let since_epoch = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let seconds = since_epoch.as_secs();
    let days = (seconds / 86_400) as i64;
    let seconds_of_day = seconds % 86_400;
    let (year, month, day) = civil_from_days(days);
    let hour = seconds_of_day / 3_600;
    let minute = (seconds_of_day % 3_600) / 60;
    let second = seconds_of_day % 60;
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

/// `timestamp` (RFC 3339 UTC, `Z`) moved by `hours`, in the same shape; `None`
/// when it does not parse.
pub(crate) fn add_hours(timestamp: &str, hours: i64) -> Option<String> {
    add_seconds(timestamp, hours * 3_600)
}

/// `timestamp` (RFC 3339 UTC, `Z`) moved by `seconds`, in the same shape;
/// `None` when it does not parse.
pub(crate) fn add_seconds(timestamp: &str, seconds: i64) -> Option<String> {
    let total = epoch_seconds(timestamp)? + seconds;
    let days = total.div_euclid(86_400);
    let of_day = total.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    Some(format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        of_day / 3_600,
        (of_day % 3_600) / 60,
        of_day % 60
    ))
}

/// Seconds from `from` to `to` (RFC 3339 UTC, `Z`): positive when `to` is
/// later; `None` when either does not parse.
pub(crate) fn seconds_between(from: &str, to: &str) -> Option<i64> {
    Some(epoch_seconds(to)? - epoch_seconds(from)?)
}

/// Seconds since 1970-01-01T00:00:00Z of an RFC 3339 UTC timestamp.
fn epoch_seconds(timestamp: &str) -> Option<i64> {
    let number =
        |range: std::ops::Range<usize>| -> Option<i64> { timestamp.get(range)?.parse().ok() };
    if timestamp.len() < 19 || timestamp.get(4..5)? != "-" || timestamp.get(10..11)? != "T" {
        return None;
    }
    let (year, month, day) = (number(0..4)?, number(5..7)?, number(8..10)?);
    let (hour, minute, second) = (number(11..13)?, number(14..16)?, number(17..19)?);
    Some(days_from_civil(year, month, day) * 86_400 + hour * 3_600 + minute * 60 + second)
}

/// Days since 1970-01-01 of a civil date (the inverse of [`civil_from_days`]).
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = year - era * 400;
    let month_prime = if month > 2 { month - 3 } else { month + 9 };
    let day_of_year = (153 * month_prime + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

/// Converts a count of days since 1970-01-01 into `(year, month, day)`.
fn civil_from_days(days_since_epoch: i64) -> (i64, i64, i64) {
    let shifted = days_since_epoch + 719_468;
    let era = if shifted >= 0 {
        shifted
    } else {
        shifted - 146_096
    } / 146_097;
    let day_of_era = shifted - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = if month_prime < 10 {
        month_prime + 3
    } else {
        month_prime - 9
    };
    let year = if month <= 2 { year + 1 } else { year };
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::{add_hours, civil_from_days, days_from_civil, now_rfc3339};

    #[test]
    fn days_from_civil_is_the_inverse_of_civil_from_days() {
        for days in [-1, 0, 59, 60, 365, 19_723, 20_000, 25_000] {
            let (year, month, day) = civil_from_days(days);
            assert_eq!(days_from_civil(year, month, day), days);
        }
    }

    #[test]
    fn hours_are_added_across_days_months_and_years() {
        assert_eq!(
            add_hours("2026-10-02T12:30:05Z", 24).as_deref(),
            Some("2026-10-03T12:30:05Z")
        );
        assert_eq!(
            add_hours("2026-12-31T23:00:00Z", 2).as_deref(),
            Some("2027-01-01T01:00:00Z")
        );
        assert_eq!(
            add_hours("2028-02-28T12:00:00Z", 24).as_deref(),
            Some("2028-02-29T12:00:00Z"),
            "2028 is a leap year"
        );
        assert_eq!(
            add_hours("2026-10-02T12:00:00Z", -13).as_deref(),
            Some("2026-10-01T23:00:00Z")
        );
        assert_eq!(add_hours("not a date", 1), None);
    }

    #[test]
    fn seconds_between_counts_forward_and_back() {
        use super::seconds_between;
        let from = "2026-03-01T10:05:00Z";
        assert_eq!(seconds_between(from, "2026-03-01T10:20:00Z"), Some(900));
        assert_eq!(seconds_between("2026-03-01T10:20:00Z", from), Some(-900));
        assert_eq!(seconds_between(from, "not a date"), None);
    }

    #[test]
    fn civil_from_days_matches_known_dates() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(19_723), (2024, 1, 1));
    }

    #[test]
    fn timestamp_is_rfc3339_shaped() {
        let timestamp = now_rfc3339();
        assert_eq!(timestamp.len(), 20, "unexpected timestamp: {timestamp}");
        assert!(timestamp.ends_with('Z'));
        assert_eq!(&timestamp[4..5], "-");
        assert_eq!(&timestamp[10..11], "T");
    }
}
