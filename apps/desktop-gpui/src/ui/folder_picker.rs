//! Picking a directory with the operating system's own file dialog.
//!
//! The previous Home screen shipped a hand-rolled directory browser: a
//! `read_dir` listing rendered as rows inside the app. It worked, and it was the
//! wrong answer. A home-grown file browser is the single clearest tell of an
//! interface that was designed without a reference — the real thing has
//! favourites, search, navigation history, a sidebar, drive letters, network
//! paths and keyboard traversal, and reimplementing a quarter of it looks worse
//! than not having it at all. Zed, VS Code, Obsidian and every other desktop app
//! in this category shell out to the platform picker for exactly this reason.
//!
//! `rfd` is used rather than a hand-written COM call because it is a thin
//! wrapper: on Windows it drives `IFileDialog` (the same dialog the shell shows
//! for "Select Folder"), on macOS `NSOpenPanel`, and on Linux either GTK or
//! `xdg-portal`. The result is one call, and the person gets the dialog they
//! already know from every other application on their machine.
//!
//! ## Why the dialog runs on a background thread
//!
//! `pick_folder` blocks until the person answers. Calling it from the UI thread
//! would freeze the window for as long as they take to browse, and GPUI would
//! report the window as unresponsive. It is dispatched to the background
//! executor (ASYNC-001: nothing blocking belongs on the UI thread) and the
//! result is applied back through `cx.spawn`, exactly like the repository reads.
//!
//! ## Platform note
//!
//! The Windows build uses the modern `IFileDialog` with no `common-controls-v6`
//! feature, which is what keeps the dialog using the system appearance instead
//! of the legacy Win32 common-controls look. That is a deliberate choice: the
//! app's own chrome is Quiet Glass, and a legacy dialog in the middle of it
//! reads as a different application.

use std::path::PathBuf;

/// Opens the platform's folder picker and returns the chosen path.
///
/// Returns `None` when the person cancels. Errors are returned rather than
/// swallowed: the caller shows a product-language message, and a silent `None`
/// is indistinguishable from a cancel.
pub fn pick_folder() -> Result<Option<PathBuf>, FolderPickError> {
    let handle = rfd::FileDialog::new()
        .set_title("Escolher um diretório")
        // A project is a directory that already exists; a file is not a
        // candidate, so the picker only offers folders.
        .set_directory(default_start_directory())
        .pick_folder();
    Ok(handle)
}

/// The folder the picker opens on.
///
/// The person's home directory rather than the last one used: a picker that
/// reopens wherever it was left is disorienting, and a *remembered* location
/// would be state the app does not yet own.
fn default_start_directory() -> PathBuf {
    for key in ["USERPROFILE", "HOME"] {
        if let Ok(value) = std::env::var(key) {
            let trimmed = value.trim();
            if !trimmed.is_empty() && std::path::Path::new(trimmed).is_dir() {
                return PathBuf::from(trimmed);
            }
        }
    }
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

/// Why the picker could not be shown.
///
/// The variant is deliberately coarse. The person cannot act on "the COM object
/// could not be coerced" and neither can the UI layer, so the message is generic
/// and the detail is logged.
#[derive(Debug)]
pub enum FolderPickError {
    /// The platform dialog could not be created or shown.
    Unavailable(std::io::Error),
}

impl std::fmt::Display for FolderPickError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FolderPickError::Unavailable(error) => {
                write!(f, "o seletor de diretórios não pôde ser aberto: {error}")
            }
        }
    }
}
