//! Why the app could not start, in the terms the person can act on.
//!
//! The composition root maps the storage failure into this enum; the shell
//! paints the cause-specific screen. The technical text travels only to the
//! "Copy details" action and the log, never to the screen.

use gpui::prelude::*;
use gpui::{div, AnyElement, App, ClickEvent, Div, Window};

use crate::i18n::app as t;
use crate::ui::controls::{action_button, ButtonKind};
use crate::ui::icons::IconName;
use crate::ui::patterns::empty_panel_actions;
use crate::ui::theme::Theme;
use crate::ui::tokens::SpacingScale;

/// The cause of a failed start, with the technical detail kept for support.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StartupError {
    /// The database file could not be opened, configured or created.
    CannotOpen {
        /// The raw error, for the log and "Copy details".
        detail: String,
    },
    /// The database was made by another version and cannot be updated.
    Migration {
        /// The raw error, for the log and "Copy details".
        detail: String,
    },
}

impl StartupError {
    /// The short technical line copied by "Copy details".
    pub fn technical_line(&self) -> String {
        match self {
            Self::CannotOpen { detail } => format!("could not open the database: {detail}"),
            Self::Migration { detail } => format!("could not migrate the database: {detail}"),
        }
    }

    /// The raw error, for `tracing`.
    pub fn detail(&self) -> &str {
        match self {
            Self::CannotOpen { detail } | Self::Migration { detail } => detail,
        }
    }

    fn title(&self) -> &'static str {
        match self {
            Self::CannotOpen { .. } => t::startup_error_open_title(),
            Self::Migration { .. } => t::startup_error_migration_title(),
        }
    }

    fn explanation(&self) -> String {
        let (detail, hint) = match self {
            Self::CannotOpen { .. } => {
                (t::startup_error_open_detail(), t::startup_error_open_hint())
            }
            Self::Migration { .. } => (
                t::startup_error_migration_detail(),
                t::startup_error_migration_hint(),
            ),
        };
        format!("{detail} {hint}")
    }
}

/// The blocking state shown instead of the app: what happened, what to do,
/// and the two real actions (the data folder is the primary one).
pub fn view(
    theme: &Theme,
    error: &StartupError,
    on_open_folder: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_copy: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Div {
    let actions: AnyElement = div()
        .flex()
        .flex_wrap()
        .gap(gpui::px(SpacingScale::S2))
        .child(
            action_button(theme, "startup-open-folder", ButtonKind::Primary, true)
                .aria_label(t::startup_error_open_folder())
                .on_click(on_open_folder)
                .child(t::startup_error_open_folder()),
        )
        .child(
            action_button(theme, "startup-copy-details", ButtonKind::Secondary, true)
                .aria_label(t::startup_error_copy())
                .on_click(on_copy)
                .child(t::startup_error_copy()),
        )
        .into_any_element();
    let glyph = match error {
        StartupError::CannotOpen { .. } => IconName::Folder,
        StartupError::Migration { .. } => IconName::Shield,
    };
    empty_panel_actions(
        theme,
        glyph,
        t::startup_error_eyebrow(),
        error.title(),
        &error.explanation(),
        actions,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn technical_line_names_the_cause_and_keeps_the_detail() {
        let migration = StartupError::Migration {
            detail: "table x already exists".into(),
        };
        assert_eq!(
            migration.technical_line(),
            "could not migrate the database: table x already exists"
        );
        let open = StartupError::CannotOpen {
            detail: "denied".into(),
        };
        assert_eq!(open.technical_line(), "could not open the database: denied");
        assert_eq!(open.detail(), "denied");
    }

    #[test]
    fn each_cause_has_its_own_copy_and_never_the_raw_detail() {
        let migration = StartupError::Migration {
            detail: "RAW".into(),
        };
        let open = StartupError::CannotOpen {
            detail: "RAW".into(),
        };
        assert_ne!(migration.title(), open.title());
        assert!(!migration.explanation().contains("RAW"));
        assert!(!open.explanation().contains("RAW"));
        // Reopening never fixes a migration failure.
        assert!(!migration
            .explanation()
            .to_lowercase()
            .contains("disk space"));
        assert!(!migration
            .explanation()
            .to_lowercase()
            .contains("free space"));
    }
}
