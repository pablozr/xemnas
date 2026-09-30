//! Local HTTP boundary: the only place HTTP is allowed (ARCH-001).
#![warn(missing_docs)]

pub mod auth;
pub mod discovery;
mod error;
pub mod server;

pub use auth::{constant_time_eq, generate_token};
pub use discovery::{
    generate_instance_id, remove_runtime_files, write_runtime_files, DiscoveryInfo, DISCOVERY_FILE,
    PROTOCOL_VERSION, TOKEN_FILE,
};
pub use server::{
    bind_loopback, router, serve, ApiConfig, ApiServer, ApiServerConfig, ApiServerError,
    RunningApi, DEFAULT_MAX_BODY_BYTES, DEFAULT_REQUEST_TIMEOUT,
};

/// Returns a stable identifier for this crate, used by wiring smoke tests.
pub fn crate_name() -> &'static str {
    "local-api"
}

#[cfg(test)]
mod tests {
    use super::crate_name;

    #[test]
    fn smoke() {
        assert_eq!(crate_name(), "local-api");
    }

    #[test]
    fn depends_on_application_domain_and_contracts() {
        assert_eq!(application::crate_name(), "application");
        assert_eq!(domain::crate_name(), "domain");
        assert_eq!(integration_contracts::crate_name(), "integration-contracts");
    }
}
