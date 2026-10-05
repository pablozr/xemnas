//! Protection of editable content before external prompt limits are applied.

use crate::redact::{redact_json, redact_secrets};

/// Rechecks the operation's original destination and current credential.
pub trait Authorization {
    /// Fails closed if settings or credentials no longer authorize this operation.
    fn check(&self) -> Result<(), crate::extract::ExtractError>;
    /// Recognizes only a provider-confirmed refresh rotation of the expected credential.
    fn rotated(
        &self,
        _previous: &str,
        _replacement: &str,
    ) -> Result<(), crate::extract::ExtractError> {
        self.check()
    }
    /// Synchronizes a provider-owned refresh lineage before checking current settings.
    fn refresh_lineage(
        &self,
        _profile: &crate::profile::AiProfile,
        _chain: &[(String, String)],
    ) -> Result<(), crate::extract::ExtractError> {
        self.check()
    }
}

/// Operation-bound provider with a borrowed live settings check.
pub struct AuthorizedExtractor<'a, E, P, K> {
    pub(crate) extractor: E,
    pub(crate) settings: &'a crate::profile::AiSettings<P, K>,
    pub(crate) profile: &'a crate::profile::AiProfile,
    pub(crate) secret: String,
    pub(crate) rotation: std::cell::RefCell<Option<String>>,
}

impl<E, P, K> AuthorizedExtractor<'_, E, P, K> {
    fn authorization(&self) -> SettingsAuthorization<'_, P, K> {
        SettingsAuthorization {
            settings: self.settings,
            profile: self.profile,
            secret: &self.secret,
            rotation: &self.rotation,
        }
    }
}

impl<
        E: crate::extract::CandidateExtractor,
        P: crate::profile::ProfileStore,
        K: crate::profile::SecretStore,
    > crate::extract::CandidateExtractor for AuthorizedExtractor<'_, E, P, K>
{
    fn extract(
        &self,
        input: &crate::extract::DecisionEvidence,
        signals: &[crate::extract::RelevanceSignal],
    ) -> Result<Vec<crate::extract::CandidateProposal>, crate::extract::ExtractError> {
        self.extract_with(
            input,
            signals,
            &crate::extract::ExtractionBackground::default(),
        )
    }
    fn extract_with(
        &self,
        input: &crate::extract::DecisionEvidence,
        signals: &[crate::extract::RelevanceSignal],
        background: &crate::extract::ExtractionBackground,
    ) -> Result<Vec<crate::extract::CandidateProposal>, crate::extract::ExtractError> {
        self.extractor
            .extract_authorized(input, signals, background, &self.authorization())
    }
}

impl<
        E: crate::overview::StructuredModel,
        P: crate::profile::ProfileStore,
        K: crate::profile::SecretStore,
    > crate::overview::StructuredModel for AuthorizedExtractor<'_, E, P, K>
{
    fn complete(
        &self,
        system: &str,
        user: &str,
        name: &str,
        schema: &serde_json::Value,
    ) -> Result<String, crate::extract::ExtractError> {
        self.extractor
            .complete_authorized(system, user, name, schema, &self.authorization())
    }
}

/// Borrowed authorization tied to settings, without caching consent or credentials.
pub struct SettingsAuthorization<'a, P, K> {
    /// Operation-local expected credential after a verified provider rotation.
    pub rotation: &'a std::cell::RefCell<Option<String>>,
    /// Settings to reload on every attempt.
    pub settings: &'a crate::profile::AiSettings<P, K>,
    /// Original operation profile; changes never redirect the operation.
    pub profile: &'a crate::profile::AiProfile,
    /// Original transport credential, never redacted or logged.
    pub secret: &'a str,
}

impl<P: crate::profile::ProfileStore, K: crate::profile::SecretStore> Authorization
    for SettingsAuthorization<'_, P, K>
{
    fn check(&self) -> Result<(), crate::extract::ExtractError> {
        let blocked = || {
            crate::extract::ExtractError::Extractor(
                "configuração ou autorização do provedor mudou".into(),
            )
        };
        let current = self
            .settings
            .load()
            .map_err(|_| blocked())?
            .ok_or_else(blocked)?;
        if &current != self.profile
            || current.validate().is_err()
            || crate::profile::consent_status(&current).is_err()
            || !self
                .settings
                .credential_ready(&current)
                .map_err(|_| blocked())?
            || self
                .settings
                .secret(&current.credential_account())
                .map_err(|_| blocked())?
                .unwrap_or_default()
                != self.rotation.borrow().as_deref().unwrap_or(self.secret)
        {
            return Err(blocked());
        }
        Ok(())
    }
    fn rotated(
        &self,
        previous: &str,
        replacement: &str,
    ) -> Result<(), crate::extract::ExtractError> {
        if self.profile.kind != crate::profile::ProfileKind::ChatGptPlan
            || previous != self.rotation.borrow().as_deref().unwrap_or(self.secret)
        {
            return Err(crate::extract::ExtractError::Extractor(
                "rotação de credencial não autorizada".into(),
            ));
        }
        let original = self.rotation.replace(Some(replacement.into()));
        if let Err(error) = self.check() {
            self.rotation.replace(original);
            return Err(error);
        }
        Ok(())
    }
    fn refresh_lineage(
        &self,
        profile: &crate::profile::AiProfile,
        chain: &[(String, String)],
    ) -> Result<(), crate::extract::ExtractError> {
        if profile != self.profile || profile.kind != crate::profile::ProfileKind::ChatGptPlan {
            return self.check();
        }
        let original = self.rotation.borrow().clone();
        let mut expected = original.as_deref().unwrap_or(self.secret).to_string();
        for (previous, replacement) in chain {
            if previous == &expected {
                expected = replacement.clone();
            }
        }
        self.rotation.replace(Some(expected));
        if let Err(error) = self.check() {
            self.rotation.replace(original);
            return Err(error);
        }
        Ok(())
    }
}

/// Masks recognized secrets, decoding JSON strings before inspecting them.
/// This is pattern-based protection, not a guarantee that arbitrary secrets are found.
pub fn protected_text(text: &str) -> String {
    match serde_json::from_str::<serde_json::Value>(text) {
        Ok(value) => redact_json(&value).to_string(),
        Err(_) => redact_secrets(text),
    }
}

/// Applies a character cap only after protecting the complete source text.
pub fn limited_text(text: &str, max_chars: usize) -> String {
    protected_text(text).chars().take(max_chars).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Models the session cache lock interleaving, not HTTP or OS keyring behavior.
    #[test]
    fn waiting_operation_syncs_after_rotation_and_revocation_still_blocks() {
        use std::sync::{Arc, Barrier, Mutex};
        struct SharedProfiles(Arc<Mutex<crate::profile::AiProfile>>);
        impl crate::profile::ProfileStore for SharedProfiles {
            fn load(
                &self,
            ) -> Result<Option<crate::profile::AiProfile>, crate::profile::ProfileError>
            {
                Ok(Some(self.0.lock().unwrap().clone()))
            }
            fn save(
                &self,
                p: &crate::profile::AiProfile,
            ) -> Result<(), crate::profile::ProfileError> {
                *self.0.lock().unwrap() = p.clone();
                Ok(())
            }
        }
        struct SharedSecrets(Arc<Mutex<String>>);
        impl crate::profile::SecretStore for SharedSecrets {
            fn get_secret(&self, _: &str) -> Result<Option<String>, crate::profile::ProfileError> {
                Ok(Some(self.0.lock().unwrap().clone()))
            }
            fn set_secret(&self, _: &str, value: &str) -> Result<(), crate::profile::ProfileError> {
                *self.0.lock().unwrap() = value.into();
                Ok(())
            }
            fn delete_secret(&self, _: &str) -> Result<(), crate::profile::ProfileError> {
                self.set_secret("", "")
            }
        }
        for revoke in [false, true] {
            let mut profile = crate::profile::offline_default_profile();
            profile.kind = crate::profile::ProfileKind::ChatGptPlan;
            profile.chatgpt = Some(crate::profile::ChatGptAccount {
                plan_usage: true,
                client_id: Some("mock-client".into()),
                ..crate::profile::ChatGptAccount::default()
            });
            profile = crate::profile::grant_consent(
                &profile,
                &crate::profile::build_preview(&profile),
                "2026-01-01T00:00:00Z",
                true,
            )
            .unwrap();
            let profiles = Arc::new(Mutex::new(profile.clone()));
            let secrets = Arc::new(Mutex::new("original".to_string()));
            let cache = Arc::new(Mutex::new(Vec::<(String, String)>::new()));
            let before_cache = Arc::new(Barrier::new(2));
            let cache_guard = cache.lock().unwrap();
            let worker_profiles = profiles.clone();
            let worker_secrets = secrets.clone();
            let worker_cache = cache.clone();
            let worker_barrier = before_cache.clone();
            let original = profile.clone();
            let waiting = std::thread::spawn(move || {
                let settings = crate::profile::AiSettings::new(
                    SharedProfiles(worker_profiles),
                    SharedSecrets(worker_secrets),
                );
                let rotation = std::cell::RefCell::new(None);
                let auth = SettingsAuthorization {
                    settings: &settings,
                    profile: &original,
                    secret: "original",
                    rotation: &rotation,
                };
                auth.check().unwrap();
                worker_barrier.wait();
                let chain = worker_cache.lock().unwrap();
                // After access_token/cache wait, before any content request.
                auth.refresh_lineage(&original, &chain).is_ok()
            });
            before_cache.wait();
            *secrets.lock().unwrap() = "provider-rotated".into();
            if revoke {
                *profiles.lock().unwrap() = crate::profile::revoke_consent(&profile);
            }
            let mut cache_guard = cache_guard;
            cache_guard.push(("original".into(), "provider-rotated".into()));
            drop(cache_guard);
            assert_eq!(waiting.join().unwrap(), !revoke);
        }
    }

    struct Profiles(std::cell::RefCell<crate::profile::AiProfile>);
    impl crate::profile::ProfileStore for Profiles {
        fn load(&self) -> Result<Option<crate::profile::AiProfile>, crate::profile::ProfileError> {
            Ok(Some(self.0.borrow().clone()))
        }
        fn save(
            &self,
            profile: &crate::profile::AiProfile,
        ) -> Result<(), crate::profile::ProfileError> {
            self.0.replace(profile.clone());
            Ok(())
        }
    }
    struct Secrets(std::cell::RefCell<String>);
    impl crate::profile::SecretStore for Secrets {
        fn get_secret(&self, _: &str) -> Result<Option<String>, crate::profile::ProfileError> {
            Ok(Some(self.0.borrow().clone()))
        }
        fn set_secret(&self, _: &str, value: &str) -> Result<(), crate::profile::ProfileError> {
            self.0.replace(value.into());
            Ok(())
        }
        fn delete_secret(&self, _: &str) -> Result<(), crate::profile::ProfileError> {
            self.0.replace(String::new());
            Ok(())
        }
    }

    #[test]
    fn refresh_lineage_supports_two_operations_but_not_human_replacement_or_revocation() {
        let mut profile = crate::profile::offline_default_profile();
        profile.kind = crate::profile::ProfileKind::ChatGptPlan;
        profile.chatgpt = Some(crate::profile::ChatGptAccount {
            plan_usage: true,
            client_id: Some("synthetic-client".into()),
            ..crate::profile::ChatGptAccount::default()
        });
        profile = crate::profile::grant_consent(
            &profile,
            &crate::profile::build_preview(&profile),
            "2026-01-01T00:00:00Z",
            true,
        )
        .unwrap();
        let settings = crate::profile::AiSettings::new(
            Profiles(std::cell::RefCell::new(profile.clone())),
            Secrets(std::cell::RefCell::new("original".into())),
        );
        let a = std::cell::RefCell::new(None);
        let b = std::cell::RefCell::new(None);
        let first = SettingsAuthorization {
            settings: &settings,
            profile: &profile,
            secret: "original",
            rotation: &a,
        };
        let second = SettingsAuthorization {
            settings: &settings,
            profile: &profile,
            secret: "original",
            rotation: &b,
        };
        first.check().unwrap();
        settings
            .set_secret(&profile.credential_account(), "rotated")
            .unwrap();
        first.rotated("original", "rotated").unwrap();
        let chain = vec![("original".into(), "rotated".into())];
        second.refresh_lineage(&profile, &chain).unwrap();
        second.check().unwrap(); // Retry/second unit after another operation refreshed.
        settings
            .set_secret(&profile.credential_account(), "human-replacement")
            .unwrap();
        assert!(second.refresh_lineage(&profile, &chain).is_err());
        settings
            .set_secret(&profile.credential_account(), "rotated-again")
            .unwrap();
        settings
            .save(&crate::profile::revoke_consent(&profile))
            .unwrap();
        assert!(first.rotated("rotated", "rotated-again").is_err());
        assert_eq!(a.borrow().as_deref(), Some("rotated"));
        assert!(first.check().is_err()); // Gate after refresh must prohibit content HTTP.
    }

    #[test]
    fn protects_decoded_json_and_unicode_before_cutting() {
        let text = r#"["ação\nAPI_KEY = synthetic-private-value"]"#;
        let protected = protected_text(text);
        assert!(!protected.contains("synthetic-private-value"));
        assert!(protected.contains("[REDACTED]"));
        assert_eq!(limited_text("é sk-synthetic123456", 8), "é [REDAC");
    }

    #[test]
    fn expanded_preview_requires_reconsent_from_ingest_only_categories() {
        let mut profile = crate::profile::offline_default_profile();
        profile.kind = crate::profile::ProfileKind::OpenAiCompatible;
        profile.endpoint = Some("http://127.0.0.1:1234/v1".into());
        let mut old_preview = crate::profile::build_preview(&profile);
        old_preview.categories.truncate(4);
        old_preview.total_approximate_chars = profile.max_input_chars * 4;
        profile.external_calls_enabled = true;
        profile.consent = Some(crate::profile::ConsentRecord {
            granted_at: "2026-01-01T00:00:00Z".into(),
            preview_hash: crate::profile::preview_hash(&old_preview),
        });
        assert!(crate::profile::consent_status(&profile).is_err());
    }
}
