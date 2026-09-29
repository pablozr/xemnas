//! RFC 3339 UTC clock shared by the application use cases.
//!
//! The application must not depend on a calendar crate for a value the domain
//! treats as an opaque string, so the civil date is derived directly from the
//! UNIX epoch. Projects and Jobs persist timestamps in this exact format
//! (`%Y-%m-%dT%H:%M:%SZ`, UTC), which is why the conversion lives in one place
//! instead of being copied per use case.

/// Returns the current time as an RFC 3339 UTC timestamp.
///
/// If the system clock predates 1970 the epoch itself is used.
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

/// Converts a count of days since 1970-01-01 into `(year, month, day)`.
///
/// Howard Hinnant's `civil_from_days` algorithm, valid for the full proleptic
/// Gregorian calendar and therefore for any timestamp the system clock can
/// produce.
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
    use super::{civil_from_days, now_rfc3339};

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
