//! xemnas desktop shell.
//!
//! Composition root: opens the SQLite store, recovers interrupted jobs, starts
//! the jobs worker and the loopback-only local API, and mounts the Projects use
//! case into the shell. Opening the database is the only place the desktop app
//! names a storage type; the UI layer never sees SQL nor HTTP. A startup failure
//! paints the documented error state instead of panicking (MVP-SPEC §14).
#![warn(missing_docs)]

use gpui::prelude::*;
use gpui::{
    px, size, App, Bounds, KeyBinding, TitlebarOptions, WindowBackgroundAppearance, WindowBounds,
    WindowOptions,
};
use gpui_platform::application;
use storage_sqlite::{default_data_dir, default_db_path, SqliteStore};

use std::sync::Arc;
use xemnas_desktop::app::{
    ActivitySource, AdjustItem, CaptureStatus, ConfirmItem, FocusSearch, GoDecisions, GoReview,
    NextItem, PaletteClose, PaletteDown, PaletteRun, PaletteUp, PrevItem, RejectItem, SaveEditor,
    Shell, SnoozeItem, TabNext, TabPrev, TogglePalette,
};
use xemnas_desktop::ui::search_field::{
    Backspace, Clear, Copy, Cut, Delete, End, Home, Left, Paste, Right, SelectAll, SelectLeft,
    SelectRight,
};

mod demo;

// Windows resources (icon ID 1 + version info) compiled once by
// tools/brand-icon.ps1 and handed straight to the linker: no build script runs,
// which this machine's Application Control policy would block on every rebuild.
// Cargo runs rustc from the workspace root, so the path is relative to it.
#[cfg(all(target_os = "windows", target_env = "msvc"))]
#[link(
    name = "apps/desktop-gpui/assets/brand/xemnas.res",
    kind = "static",
    modifiers = "+verbatim,-bundle"
)]
extern "C" {}

fn main() {
    if let Err(error) = telemetry::init() {
        eprintln!("telemetry init failed: {error}");
    }
    // Suppress job-handler panic payloads (PRIV-001) without changing any other
    // panic behavior.
    application::jobs::install_panic_sanitizer();

    if std::env::args().any(|argument| argument == "--demo") {
        run_shell_mode(
            demo::store().map_err(|error| error.to_string()),
            Box::new(demo::ai_settings()),
            true,
            CaptureStatus::Demo,
        );
        return;
    }

    let store = match SqliteStore::open(default_db_path()) {
        Ok(store) => store,
        Err(error) => {
            // The shell logs the technical detail through `tracing` and paints a
            // product-language error state; the raw error never reaches the UI.
            run_shell(Err(error.to_string()), ai_settings(), CaptureStatus::Unavailable);
            return;
        }
    };

    let mut jobs = application::jobs::Jobs::new(store.clone());

    // AI settings: the profile file is seeded with the offline default so the
    // inbox keeps working without any provider (MVP-SPEC §7 line 404).
    let ai_profile_path = ai_profile_path();
    let settings = ai_settings();
    match settings.load_or_seed() {
        Ok(_) => tracing::info!(
            path = %ai_profile_path.display(),
            operation = "ai_profile",
            "AI profile ready"
        ),
        Err(error) => tracing::error!(
            error = %error,
            operation = "ai_profile",
            "could not load the AI profile"
        ),
    }

    // Capture analysis reloads the profile on every run, so a Settings change
    // takes effect without a restart. The offline fake stays the default; the
    // external provider is only reachable with consent and a stored key.
    jobs.register(
        application::jobs::ANALYZE_CAPTURE_KIND,
        std::sync::Arc::new({
            let store = store.clone();
            let settings = settings.clone();
            move |record: &application::jobs::JobRecord| analyze_capture(&store, &settings, record)
        }),
    );
    jobs.observe_with(|event| match event {
        application::jobs::JobEvent::Finished(outcome) => {
            tracing::info!(
                job_id = %outcome.job_id,
                kind = %outcome.kind,
                state = outcome.state.as_str(),
                attempts = outcome.attempts,
                "job finished"
            );
        }
        application::jobs::JobEvent::StorageError(detail) => {
            tracing::error!(
                error = %detail,
                operation = "jobs_worker",
                "jobs worker stopped on a storage failure"
            );
        }
    });

    // Recovery must finish before the worker starts: a failed recovery means the
    // queue is unreconciled, so the worker stays down and the window still opens.
    let worker = match jobs.recover() {
        Ok(report) => {
            tracing::info!(
                requeued = report.requeued,
                failed = report.failed,
                operation = "recover_jobs",
                "recovered interrupted jobs"
            );
            Some(jobs.spawn_worker())
        }
        Err(error) => {
            tracing::error!(
                error = %error,
                operation = "recover_jobs",
                "could not recover interrupted jobs; the jobs worker will not start"
            );
            None
        }
    };

    // Import whatever the adapter parked in the outbox while the desktop was
    // closed (MVP-SPEC §7.2). It runs after the jobs recovery and before the
    // local API starts, so the queue is reconciled first and the adapter only
    // discovers an app whose pending captures are already being imported.
    // Transient items stay in `pending` for the next start; a drain failure
    // only logs (MVP-SPEC §14 - an integration problem never blocks the app).
    drain_outbox(&store);

    // The local API shares the process lifetime: it runs on its own Tokio
    // runtime thread beside the jobs worker, and a startup failure is logged
    // while the window still opens (MVP-SPEC §14 - a broken integration must
    // never block the rest of the app).
    let api = match start_local_api(store.clone()) {
        Ok(api) => Some(api),
        Err(error) => {
            tracing::error!(
                error = %error,
                operation = "local_api_start",
                "could not start the local API; the window still opens"
            );
            None
        }
    };

    let capture = if api.is_some() {
        CaptureStatus::Listening
    } else {
        CaptureStatus::Unavailable
    };
    run_shell(Ok(store), settings, capture);

    // Graceful shutdown mirrors startup: stop the API first so the discovery
    // and per-session token files are removed, then stop the jobs worker.
    if let Some(api) = api {
        api.shutdown();
    }
    if let Some(worker) = worker {
        worker.stop();
        if let Err(error) = worker.join() {
            tracing::error!(
                error = %error,
                operation = "jobs_worker",
                "jobs worker stopped with an error"
            );
        }
    }
}

/// Composes the desktop use cases from the ready store, or a startup failure.
fn run_shell(store: Result<SqliteStore, String>, settings: AiSettings, capture: CaptureStatus) {
    run_shell_mode(store, Box::new(settings), false, capture);
}

/// The stored AI profile beside the database, under the data directory.
fn ai_profile_path() -> std::path::PathBuf {
    default_data_dir().join("settings").join("ai-profile.json")
}

/// AI settings over the profile file and the OS key vault. Settings do not
/// depend on the database, so the page still opens when it fails to load.
fn ai_settings() -> AiSettings {
    application::profile::AiSettings::new(
        application::profile::FileProfileStore::new(ai_profile_path()),
        ai_provider::KeyringSecretStore::new(),
    )
}

/// The window material. Mica Alt by default: it tints and blurs the wallpaper
/// and stays smooth while dragging. `XEMNAS_BACKDROP=acrylic` blurs the windows
/// behind instead (livelier, but the system may drop frames on resize);
/// `none` paints an opaque window.
fn backdrop_from_env() -> WindowBackgroundAppearance {
    match std::env::var("XEMNAS_BACKDROP").as_deref() {
        Ok("acrylic") => WindowBackgroundAppearance::Blurred,
        Ok("mica") => WindowBackgroundAppearance::MicaBackdrop,
        Ok("none") => WindowBackgroundAppearance::Opaque,
        _ => WindowBackgroundAppearance::MicaAltBackdrop,
    }
}

fn run_shell_mode(
    store: Result<SqliteStore, String>,
    settings: Box<dyn xemnas_desktop::screens::settings::AiBackend>,
    demo: bool,
    capture: CaptureStatus,
) {
    let backdrop = backdrop_from_env();
    // The status line reads the jobs table through the application port.
    let activity: Option<ActivitySource> = store.as_ref().ok().map(|store| {
        let store = store.clone();
        Arc::new(move || {
            application::jobs::JobRepository::list(&store)
                .ok()
                .map(|records| application::jobs::JobSummary::from_records(&records))
        }) as ActivitySource
    });
    application().run(move |cx: &mut App| {
        xemnas_desktop::fonts::register_embedded(cx);
        cx.bind_keys([
            KeyBinding::new("tab", TabNext, None),
            KeyBinding::new("shift-tab", TabPrev, None),
            KeyBinding::new("ctrl-k", TogglePalette, Some("xemnas")),
            KeyBinding::new("ctrl-f", FocusSearch, Some("xemnas")),
            // Single-key shortcuts never fire while a text field has focus.
            KeyBinding::new("j", NextItem, Some("xemnas && !SearchField")),
            KeyBinding::new("down", NextItem, Some("xemnas && !SearchField")),
            KeyBinding::new("k", PrevItem, Some("xemnas && !SearchField")),
            KeyBinding::new("up", PrevItem, Some("xemnas && !SearchField")),
            KeyBinding::new("c", ConfirmItem, Some("xemnas && !SearchField")),
            KeyBinding::new("r", RejectItem, Some("xemnas && !SearchField")),
            KeyBinding::new("s", SnoozeItem, Some("xemnas && !SearchField")),
            KeyBinding::new("a", AdjustItem, Some("xemnas && !SearchField")),
            KeyBinding::new("ctrl-enter", SaveEditor, Some("Editor")),
            KeyBinding::new("ctrl-1", GoReview, Some("xemnas")),
            KeyBinding::new("ctrl-2", GoDecisions, Some("xemnas")),
            KeyBinding::new("backspace", Backspace, Some("SearchField")),
            KeyBinding::new("delete", Delete, Some("SearchField")),
            KeyBinding::new("left", Left, Some("SearchField")),
            KeyBinding::new("right", Right, Some("SearchField")),
            KeyBinding::new("shift-left", SelectLeft, Some("SearchField")),
            KeyBinding::new("shift-right", SelectRight, Some("SearchField")),
            KeyBinding::new("ctrl-a", SelectAll, Some("SearchField")),
            KeyBinding::new("ctrl-v", Paste, Some("SearchField")),
            KeyBinding::new("ctrl-c", Copy, Some("SearchField")),
            KeyBinding::new("ctrl-x", Cut, Some("SearchField")),
            KeyBinding::new("home", Home, Some("SearchField")),
            KeyBinding::new("end", End, Some("SearchField")),
            KeyBinding::new("escape", Clear, Some("SearchField")),
            // Declared after Clear so the palette closes instead of clearing.
            KeyBinding::new("escape", PaletteClose, Some("Palette > SearchField")),
            KeyBinding::new("escape", PaletteClose, Some("Palette")),
            KeyBinding::new("down", PaletteDown, Some("Palette")),
            KeyBinding::new("up", PaletteUp, Some("Palette")),
            KeyBinding::new("enter", PaletteRun, Some("Palette")),
        ]);

        let compact = demo && std::env::args().any(|argument| argument == "--compact");
        let dimensions = if compact {
            size(px(1180.0), px(760.0))
        } else {
            size(px(1440.0), px(1024.0))
        };
        let bounds = Bounds::centered(None, dimensions, cx);
        let (projects, inbox, decisions) = match store {
            Ok(store) => (
                Ok(application::projects::Projects::new(store.clone())),
                Some(application::inbox::Inbox::new(store.clone())),
                Some((
                    application::decisions::Decisions::new(store.clone()),
                    application::export::Export::new(store),
                )),
            ),
            Err(error) => (Err(error), None, None),
        };
        let view = cx.new(|cx| {
            let mut shell =
                Shell::<SqliteStore>::new(cx, projects, inbox, decisions, Some(settings));
            shell.set_demo(demo);
            shell.set_backdrop(backdrop != WindowBackgroundAppearance::Opaque);
            shell.set_activity(capture, activity, cx);
            shell
        });
        let focus = view.read(cx).initial_focus(cx);

        let opened = cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some("xemnas — Projetos".into()),
                    // The key change in this pass. The app draws its own
                    // 48 px title row — mark, wordmark, breadcrumb and search —
                    // and the OS was drawing *its* title bar above it, so the
                    // window had two stacked headers: the OS one with the
                    // minimize/maximize/close buttons, then mine directly
                    // beneath it. That double bar is what read as "the header
                    // is separated from the window controls".
                    //
                    // `appears_transparent: true` hides the OS caption bar.
                    // The shell draws its own caption controls in the same
                    // row; GPUI marks their hitboxes as native window areas.
                    appears_transparent: true,
                    ..Default::default()
                }),
                app_id: Some("com.xemnas.desktop".into()),
                window_min_size: Some(size(px(1180.0), px(760.0))),
                // Mica Alt as the window material, the darker Mica variant that
                // Windows 11 provides specifically so a commanding surface —
                // the title bar and the navigation rail — reads as distinct from
                // the content beneath it. This is what the design system
                // anticipated in `docs/design-system-quiet-glass.md` §
                // "Fallback técnico" ("No Windows, Mica/blur pode ser usado no
                // fundo da janela quando o backend do GPUI estiver estável").
                //
                // The window itself paints nothing: the shell's canvas is a
                // translucent `layer.fill` over this backdrop, which is the
                // three-layer Mica model from the Fluent 2 material spec
                // (base material → commanding layer → content layer). On a
                // platform without Mica the backdrop is ignored and the same
                // translucent fill still composites against the opaque canvas,
                // so contrast is preserved either way. `XEMNAS_BACKDROP=none`
                // falls back to the opaque window if a machine trips on it.
                window_background: backdrop,
                ..Default::default()
            },
            |window, cx| {
                window.focus(&focus, cx);
                view.clone()
            },
        );
        if let Err(error) = opened {
            eprintln!("failed to open the xemnas window: {error}");
        }

        cx.activate(true);
    });
}

/// Concrete AI settings used by the composition root.
type AiSettings = application::profile::AiSettings<
    application::profile::FileProfileStore,
    ai_provider::KeyringSecretStore,
>;

/// Runs one capture analysis honoring the stored AI profile.
///
/// The profile is reloaded every run, so a Settings change needs no restart.
/// Logs carry only counts and status; the provider key never reaches a log.
fn analyze_capture(
    store: &SqliteStore,
    settings: &AiSettings,
    record: &application::jobs::JobRecord,
) -> Result<(), application::jobs::JobFailure> {
    use application::profile::{choose_extractor, ExtractorChoice};

    let capture_id = record.payload.as_str();
    let profile = match settings.load_or_seed() {
        Ok(profile) => profile,
        Err(error) => {
            tracing::error!(
                error = %error,
                operation = "analyze_capture",
                "could not load the AI profile"
            );
            // An unreadable profile must still leave a provenance row; the
            // helper forces the fixed unavailable context.
            return fail_provider_setup(
                store,
                capture_id,
                &application::extract::RunContext::unavailable(Some(record.id.clone())),
                application::extract::ProviderSetupError::ProfileUnavailable,
            );
        }
    };
    let context = application::extract::RunContext::for_profile(&profile, Some(record.id.clone()));

    match choose_extractor(Some(&profile)) {
        ExtractorChoice::OfflineFake => run_extraction_logged(
            store,
            &application::extract::FakeCandidateExtractor,
            capture_id,
            &context,
        ),
        ExtractorChoice::ExternalBlocked => {
            // Consent is missing or stale: no provider is contacted, but the
            // run is still recorded so the reason is traceable (assessment
            // "skipped"). A storage failure while recording fails the job.
            if let Err(error) =
                application::extract::record_skipped_assessment(store, capture_id, &context)
            {
                tracing::error!(
                    error = %error,
                    operation = "analyze_capture",
                    "could not record the skipped assessment"
                );
                return Err(application::jobs::JobFailure::Failed);
            }
            tracing::info!(
                operation = "analyze_capture",
                "external extraction blocked: consent missing"
            );
            Ok(())
        }
        ExtractorChoice::ExternalEnabled => {
            let secret = match settings.secret(&profile.id) {
                Ok(Some(secret)) => secret,
                Ok(None) => {
                    tracing::error!(
                        operation = "analyze_capture",
                        "external extraction blocked: provider key missing"
                    );
                    return fail_provider_setup(
                        store,
                        capture_id,
                        &context,
                        application::extract::ProviderSetupError::MissingSecret,
                    );
                }
                Err(error) => {
                    tracing::error!(
                        error = %error,
                        operation = "analyze_capture",
                        "could not read the provider key"
                    );
                    return fail_provider_setup(
                        store,
                        capture_id,
                        &context,
                        application::extract::ProviderSetupError::Keystore,
                    );
                }
            };
            match ai_provider::OpenAiCompatibleExtractor::new(&profile, secret) {
                Ok(extractor) => run_extraction_logged(store, &extractor, capture_id, &context),
                Err(error) => {
                    tracing::error!(
                        error = %error,
                        operation = "analyze_capture",
                        "could not prepare the external extractor"
                    );
                    fail_provider_setup(
                        store,
                        capture_id,
                        &context,
                        application::extract::ProviderSetupError::InvalidConfig,
                    )
                }
            }
        }
    }
}

/// Records the `failed` assessment for a provider that never started, then
/// returns the typed job failure. A storage error while recording still fails
/// the job, so provenance never silently disappears.
fn fail_provider_setup(
    store: &SqliteStore,
    capture_id: &str,
    context: &application::extract::RunContext,
    error: application::extract::ProviderSetupError,
) -> Result<(), application::jobs::JobFailure> {
    match application::extract::fail_provider_setup(store, capture_id, context, error) {
        Ok(failure) => Err(failure),
        Err(storage_error) => {
            tracing::error!(
                error = %storage_error,
                operation = "analyze_capture",
                "could not record the provider setup failure"
            );
            Err(application::jobs::JobFailure::Failed)
        }
    }
}

/// Runs an extractor and logs only counts; never artifact content.
fn run_extraction_logged<E: application::extract::CandidateExtractor>(
    store: &SqliteStore,
    extractor: &E,
    capture_id: &str,
    context: &application::extract::RunContext,
) -> Result<(), application::jobs::JobFailure> {
    match application::extract::run_extraction(store, extractor, capture_id, context) {
        Ok(report) => {
            tracing::info!(
                candidates = report.candidates,
                inserted = report.inserted,
                operation = "analyze_capture",
                "capture analysis finished"
            );
            Ok(())
        }
        Err(error) => {
            tracing::error!(
                error = %error,
                operation = "analyze_capture",
                "capture analysis failed"
            );
            Err(application::jobs::JobFailure::Failed)
        }
    }
}

/// Imports pending outbox items into the store (MVP-SPEC §7.2).
///
/// The root is `XEMNAS_OUTBOX_DIR` or `<data>/outbox`, matching the adapter.
/// Accepted items are pruned after `XEMNAS_OUTBOX_ACCEPTED_RETENTION_DAYS`
/// (default 7); rejected diagnostics are never pruned here. The report is
/// logged as counts only — filenames and envelope content stay out of the log
/// (PRIV-001).
fn drain_outbox(store: &SqliteStore) {
    let root = std::env::var_os("XEMNAS_OUTBOX_DIR")
        .filter(|value| !value.is_empty())
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| default_data_dir().join("outbox"));
    // Accept only a sane day count (1..=3650) and multiply without overflowing,
    // so a hostile environment value can never panic the boot path. Any invalid
    // value falls back to the 7-day default.
    let retention_days: u64 = std::env::var("XEMNAS_OUTBOX_ACCEPTED_RETENTION_DAYS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|days| (1..=3650).contains(days))
        .unwrap_or(7);
    let retention =
        std::time::Duration::from_secs(retention_days.checked_mul(86_400).unwrap_or(7 * 86_400));
    let ingest = application::captures::CaptureIngest::new(store.clone());
    match application::outbox::drain(&root, &ingest, retention) {
        Ok(report) => tracing::info!(
            accepted = report.accepted,
            rejected = report.rejected.values().sum::<usize>(),
            pending_remaining = report.pending_remaining,
            sending_recovered = report.sending_recovered,
            pruned = report.pruned,
            operation = "outbox_drain",
            "imported pending captures from the outbox"
        ),
        Err(error) => tracing::error!(
            error = %error,
            operation = "outbox_drain",
            "could not drain the outbox; pending items wait for the next start"
        ),
    }
}

/// Starts the loopback-only local API next to the database (MVP-SPEC §9).
///
/// The runtime directory is the state directory that already holds
/// `app.db`, so `discovery.json` and the per-session token file live with the
/// rest of the app state. Defaults are a 4 MiB body limit and a 5 s request
/// timeout; the token never reaches the UI or the logs.
fn start_local_api(store: SqliteStore) -> Result<local_api::RunningApi, local_api::ApiServerError> {
    let runtime_dir = default_db_path()
        .parent()
        .map(std::path::Path::to_path_buf)
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    let use_case = std::sync::Arc::new(application::captures::CaptureIngest::new(store));
    local_api::ApiServer::start(local_api::ApiServerConfig::new(use_case, runtime_dir))
}

#[cfg(test)]
mod tests {
    #[test]
    fn smoke() {
        assert_eq!(env!("CARGO_PKG_NAME"), "desktop-gpui");
    }
}
