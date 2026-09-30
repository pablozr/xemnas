//! Versioned contracts exchanged with external adapters (for example OpenCode).
#![warn(missing_docs)]

pub mod capture;

/// Returns a stable identifier for this crate, used by wiring smoke tests.
pub fn crate_name() -> &'static str {
    "integration-contracts"
}

#[cfg(test)]
mod tests {
    use super::crate_name;

    #[test]
    fn smoke() {
        assert_eq!(crate_name(), "integration-contracts");
    }
}
