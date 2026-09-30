//! Per-project context injection settings, chosen in the app.

use crate::clock::now_rfc3339;
use crate::context::ContextError;
use crate::injection::{InjectionMode, MAX_BUDGET_TOKENS, MIN_BUDGET_TOKENS};
use crate::projects::ProjectRepository;

/// How context is used for a project's agent turns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ContextMode {
    /// Nothing is computed or sent.
    #[default]
    Off,
    /// Blocks are computed and recorded, never sent.
    Shadow,
    /// Blocks are appended to the agent prompt.
    Inject,
}

impl ContextMode {
    /// Persisted literal.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Shadow => "shadow",
            Self::Inject => "inject",
        }
    }

    /// Parses a persisted literal.
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "off" => Some(Self::Off),
            "shadow" => Some(Self::Shadow),
            "inject" => Some(Self::Inject),
            _ => None,
        }
    }

    /// Delivery mode for the audit, or `None` when off.
    pub fn delivery(&self) -> Option<InjectionMode> {
        match self {
            Self::Off => None,
            Self::Shadow => Some(InjectionMode::Shadow),
            Self::Inject => Some(InjectionMode::Inject),
        }
    }
}

/// Context settings of one project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectContextSettings {
    /// Project the settings belong to.
    pub project_id: String,
    /// Injection mode.
    pub mode: ContextMode,
    /// Token budget per block; the server default when `None`.
    pub budget_tokens: Option<usize>,
    /// RFC 3339 time of the last change; `None` while never saved.
    pub updated_at: Option<String>,
}

impl ProjectContextSettings {
    fn default_for(project_id: &str) -> Self {
        Self {
            project_id: project_id.to_string(),
            mode: ContextMode::Off,
            budget_tokens: None,
            updated_at: None,
        }
    }
}

/// Persistence port for context settings.
pub trait ContextSettingsStore {
    /// Stored settings of a project, if any were saved.
    fn context_settings(
        &self,
        project_id: &str,
    ) -> Result<Option<ProjectContextSettings>, ContextError>;

    /// Inserts or replaces the settings of a project.
    fn save_context_settings(&self, settings: &ProjectContextSettings) -> Result<(), ContextError>;
}

/// Reads and changes a project's context settings.
#[derive(Debug, Clone)]
pub struct ContextSettings<S> {
    store: S,
}

impl<S: ContextSettingsStore + ProjectRepository> ContextSettings<S> {
    /// Wraps the store.
    pub fn new(store: S) -> Self {
        Self { store }
    }

    /// Current settings; `off` when never saved.
    ///
    /// # Errors
    ///
    /// `project_not_found` for an unknown project, `storage` otherwise.
    pub fn get(&self, project_id: &str) -> Result<ProjectContextSettings, ContextError> {
        self.require_project(project_id)?;
        Ok(self
            .store
            .context_settings(project_id)?
            .unwrap_or_else(|| ProjectContextSettings::default_for(project_id)))
    }

    /// Saves the mode and budget of a project.
    ///
    /// # Errors
    ///
    /// `invalid_request` for a budget outside 50..=2000, `project_not_found`.
    pub fn set(
        &self,
        project_id: &str,
        mode: ContextMode,
        budget_tokens: Option<usize>,
    ) -> Result<ProjectContextSettings, ContextError> {
        if budget_tokens
            .is_some_and(|budget| !(MIN_BUDGET_TOKENS..=MAX_BUDGET_TOKENS).contains(&budget))
        {
            return Err(ContextError::InvalidRequest(
                "o orçamento deve ficar entre 50 e 2.000 tokens".into(),
            ));
        }
        self.require_project(project_id)?;
        let settings = ProjectContextSettings {
            project_id: project_id.to_string(),
            mode,
            budget_tokens,
            updated_at: Some(now_rfc3339()),
        };
        self.store.save_context_settings(&settings)?;
        Ok(settings)
    }

    fn require_project(&self, project_id: &str) -> Result<(), ContextError> {
        self.store
            .get(project_id)
            .map_err(|error| ContextError::Storage(error.to_string()))?
            .map(|_| ())
            .ok_or(ContextError::ProjectNotFound)
    }
}
