//! UTC timestamps in the app's `YYYY-MM-DDTHH:MM:SSZ` format.

/// A validated UTC timestamp; ordering is chronological.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Timestamp(String);

impl Timestamp {
    /// Parses `YYYY-MM-DDTHH:MM:SSZ` or a bare `YYYY-MM-DD` (midnight UTC).
    pub fn parse(value: &str) -> Option<Self> {
        let value = value.trim();
        let full = if value.len() == 10 {
            format!("{value}T00:00:00Z")
        } else {
            value.to_string()
        };
        is_well_formed(&full).then_some(Self(full))
    }

    /// The canonical text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Timestamp {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

fn is_well_formed(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 20 {
        return false;
    }
    let separators = [
        (4, b'-'),
        (7, b'-'),
        (10, b'T'),
        (13, b':'),
        (16, b':'),
        (19, b'Z'),
    ];
    if separators
        .iter()
        .any(|(index, byte)| bytes[*index] != *byte)
    {
        return false;
    }
    let number = |range: std::ops::Range<usize>| -> Option<u32> {
        let text = value.get(range)?;
        text.bytes()
            .all(|byte| byte.is_ascii_digit())
            .then(|| text.parse().ok())
            .flatten()
    };
    let (Some(month), Some(day), Some(hour), Some(minute), Some(second), Some(_)) = (
        number(5..7),
        number(8..10),
        number(11..13),
        number(14..16),
        number(17..19),
        number(0..4),
    ) else {
        return false;
    };
    (1..=12).contains(&month) && (1..=31).contains(&day) && hour < 24 && minute < 60 && second < 60
}

#[cfg(test)]
mod tests {
    use super::Timestamp;

    #[test]
    fn parses_full_and_date_only_forms() {
        assert_eq!(
            Timestamp::parse("2026-09-30T12:00:00Z").map(|t| t.to_string()),
            Some("2026-09-30T12:00:00Z".to_string())
        );
        assert_eq!(
            Timestamp::parse(" 2026-09-30 ").map(|t| t.to_string()),
            Some("2026-09-30T00:00:00Z".to_string())
        );
    }

    #[test]
    fn rejects_malformed_values() {
        for value in [
            "",
            "2026-13-01",
            "2026-09-32",
            "2026-09-30T24:00:00Z",
            "2026-09-30T12:00:00",
            "2026-09-30T12:00:00+03:00",
            "30/09/2026",
            "20a6-09-30",
        ] {
            assert_eq!(Timestamp::parse(value), None, "{value}");
        }
    }

    #[test]
    fn orders_chronologically() {
        let earlier = Timestamp::parse("2026-01-31").expect("valid");
        let later = Timestamp::parse("2026-02-01T00:00:00Z").expect("valid");
        assert!(earlier < later);
    }
}
