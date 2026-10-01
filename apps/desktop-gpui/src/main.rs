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
use storage_sqlite::SqliteStore;

use std::sync::Arc;
use xemnas_desktop::app::{
    ActivitySource, AdjustItem, CaptureStatus, ConfirmItem, FocusSearch, GoContext, GoDecisions,
    GoMap, GoOverview, GoReview, NextItem, PaletteClose, PaletteDown, PaletteRun, PaletteUp,
    PrevItem, RejectItem, SaveEditor, Shell, SnoozeItem, TabNext, TabPrev, TogglePalette,
};
use xemnas_desktop::screens::settings::SettingsServices;
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
        let store = demo::store().map_err(|error| error.to_string());
        let ai = demo::ai_settings();
        // Sample paths and a fictitious port: the demo starts no API, and the
        // OpenCode section reports on these as on any other environment.
        let data_dir = std::env::temp_dir().join("xemnas-demo");
        let environment = application::integration::IntegrationEnvironment {
            outbox_dir: data_dir.join("outbox"),
            runtime_dir: data_dir.join("runtime"),
            data_dir,
            api: Some(application::integration::LocalApiEndpoint {
                port: 51234,
                protocol_version: local_api::PROTOCOL_VERSION,
            }),
        };
        // The demo lists sample models and offers no ChatGPT sign-in: it never
        // touches the network or the OS key vault.
        let providers = ProviderPorts {
            catalog: Some(std::sync::Arc::new(demo::SampleCatalog)),
            account: None,
        };
        let overview = overview_api(ai.clone(), Arc::new(ai_provider::ChatGptSession::default()));
        let services = settings_services(
            Box::new(ai.clone()),
            providers,
            store.as_ref().ok(),
            environment,
            ai,
        );
        run_shell_mode(store, services, overview, true, CaptureStatus::Demo);
        return;
    }

    let paths = application::AppPaths::from_env();
    let store = match SqliteStore::open(&paths.database) {
        Ok(store) => store,
        Err(error) => {
            // The shell logs the technical detail through `tracing` and paints a
            // product-language error state; the raw error never reaches the UI.
            let providers =
                provider_ports(&std::sync::Arc::new(ai_provider::ChatGptSession::default()));
            let services = SettingsServices {
                ai: Box::new(ai_settings(&paths.ai_profile)),
                catalog: providers.catalog,
                account: providers.account,
                integration: None,
                diagnostics: None,
            };
            run_shell_mode(
                Err(error.to_string()),
                services,
                overview_api(ai_settings(&paths.ai_profile), Arc::default()),
                false,
                CaptureStatus::Unavailable,
            );
            return;
        }
    };

    let mut jobs = application::jobs::Jobs::new(store.clone());

    // AI settings: the profile file is seeded with the offline default so the
    // inbox keeps working without any provider (MVP-SPEC §7 line 404).
    let ai_profile_path = paths.ai_profile.clone();
    let settings = ai_settings(&ai_profile_path);
    // One ChatGPT session per process: Settings (sign-in, models) and the
    // jobs worker (extraction) share it so token refreshes never race.
    let chatgpt = std::sync::Arc::new(ai_provider::ChatGptSession::default());
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
            let analysis = application::analysis::AnalyzeCapture::new(
                store.clone(),
                settings.clone(),
                ai_provider::ProviderFactory::new(chatgpt.clone()),
            );
            move |record: &application::jobs::JobRecord| analyze_capture(&analysis, record)
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
    drain_outbox(&store, &paths.outbox_dir);

    // The local API shares the process lifetime: it runs on its own Tokio
    // runtime thread beside the jobs worker, and a startup failure is logged
    // while the window still opens (MVP-SPEC §14 - a broken integration must
    // never block the rest of the app).
    let api = match start_local_api(store.clone(), &paths.runtime_dir) {
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
    let environment = application::integration::IntegrationEnvironment {
        data_dir: paths.data_dir.clone(),
        outbox_dir: paths.outbox_dir.clone(),
        runtime_dir: paths.runtime_dir.clone(),
        api: api
            .as_ref()
            .map(|api| application::integration::LocalApiEndpoint {
                port: api.address().port(),
                protocol_version: local_api::PROTOCOL_VERSION,
            }),
    };
    let services = settings_services(
        Box::new(settings.clone()),
        provider_ports(&chatgpt),
        Some(&store),
        environment,
        settings,
    );
    let overview = overview_api(ai_settings(&paths.ai_profile), chatgpt.clone());
    run_shell_mode(Ok(store), services, overview, false, capture);

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

/// The provider ports of the settings page (ADR-0004).
struct ProviderPorts {
    catalog: Option<std::sync::Arc<dyn application::providers::ModelCatalog>>,
    account: Option<std::sync::Arc<dyn application::providers::PlanAccount>>,
}

/// Model catalog and ChatGPT sign-in over the process-wide ChatGPT session,
/// shared with the extraction jobs so token refreshes never race.
fn provider_ports(chatgpt: &std::sync::Arc<ai_provider::ChatGptSession>) -> ProviderPorts {
    ProviderPorts {
        catalog: Some(std::sync::Arc::new(ai_provider::HttpModelCatalog::new(
            chatgpt.clone(),
        ))),
        account: Some(chatgpt.clone()),
    }
}

/// The settings page's use cases. Integration and diagnostics read the
/// database, so they exist only when it opened.
fn settings_services<P, K>(
    ai: Box<dyn xemnas_desktop::screens::settings::AiBackend>,
    providers: ProviderPorts,
    store: Option<&SqliteStore>,
    environment: application::integration::IntegrationEnvironment,
    diagnostics_settings: application::profile::AiSettings<P, K>,
) -> SettingsServices
where
    P: application::profile::ProfileStore + Send + 'static,
    K: application::profile::SecretStore + Send + 'static,
{
    use xemnas_desktop::screens::settings::{DiagnosticsService, IntegrationService};
    let outbox_dir = environment.outbox_dir.clone();
    SettingsServices {
        ai,
        catalog: providers.catalog,
        account: providers.account,
        integration: store.map(|store| {
            Box::new(IntegrationService::new(
                application::integration::Integration::new(store.clone(), environment),
                outbox_dir.clone(),
            )) as Box<dyn xemnas_desktop::screens::settings::IntegrationBackend>
        }),
        diagnostics: store.map(|store| {
            Box::new(DiagnosticsService::new(
                application::diagnostics::Diagnostics::new(
                    store.clone(),
                    diagnostics_settings,
                    outbox_dir,
                ),
                application::jobs::Jobs::new(store.clone()),
            )) as Box<dyn xemnas_desktop::screens::settings::DiagnosticsBackend>
        }),
    }
}

/// Concrete AI settings used by the composition root.
type AiSettings = application::profile::AiSettings<
    application::profile::FileProfileStore,
    ai_provider::KeyringSecretStore,
>;

/// AI settings over the profile file and the OS key vault. Settings do not
/// depend on the database, so the page still opens when it fails to load.
fn ai_settings(profile: &std::path::Path) -> AiSettings {
    application::profile::AiSettings::new(
        application::profile::FileProfileStore::new(profile),
        ai_provider::KeyringSecretStore::new(),
    )
}

/// The window material. Opaque by default: the system material showed a
/// strip of raw backdrop and a floating-card edge that read as a rendering
/// fault. `XEMNAS_BACKDROP=mica-alt|mica|acrylic` opts back into a material
/// for experiments; the glass frame only switches on in that case.
fn backdrop_from_env() -> WindowBackgroundAppearance {
    match std::env::var("XEMNAS_BACKDROP").as_deref() {
        Ok("acrylic") => WindowBackgroundAppearance::Blurred,
        Ok("mica") => WindowBackgroundAppearance::MicaBackdrop,
        Ok("mica-alt") => WindowBackgroundAppearance::MicaAltBackdrop,
        _ => WindowBackgroundAppearance::Opaque,
    }
}

/// Builds the project overview over the opened store.
type OverviewFactory = Box<dyn FnOnce(SqliteStore) -> Arc<dyn application::overview::OverviewApi>>;

/// The overview reloads the AI profile on every generation, like analysis.
fn overview_api<P, K>(
    settings: application::profile::AiSettings<P, K>,
    chatgpt: Arc<ai_provider::ChatGptSession>,
) -> OverviewFactory
where
    P: application::profile::ProfileStore + Send + Sync + 'static,
    K: application::profile::SecretStore + Send + Sync + 'static,
{
    Box::new(move |store| {
        Arc::new(application::overview::ProjectOverviews::new(
            store,
            settings,
            ai_provider::ProviderFactory::new(chatgpt),
        ))
    })
}

fn run_shell_mode(
    store: Result<SqliteStore, String>,
    settings: SettingsServices,
    overview: OverviewFactory,
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
            KeyBinding::new("ctrl-3", GoContext, Some("xemnas")),
            KeyBinding::new("ctrl-4", GoMap, Some("xemnas")),
            KeyBinding::new("ctrl-0", GoOverview, Some("xemnas")),
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
        let (projects, inbox, decisions, context, map, overview) = match store {
            Ok(store) => (
                Ok(application::projects::Projects::new(store.clone())),
                Some(application::inbox::Inbox::new(store.clone())),
                Some((
                    application::decisions::Decisions::new(store.clone()),
                    application::export::Export::new(store.clone()),
                    application::relations::DecisionRelations::new(store.clone()),
                )),
                Some(xemnas_desktop::screens::context::ContextServices {
                    decisions: application::decisions::Decisions::new(store.clone()),
                    claims: application::claims::Claims::new(store.clone()),
                    settings: application::context_settings::ContextSettings::new(store.clone()),
                    packs: application::context::ContextPacks::new(store.clone()),
                }),
                Some(xemnas_desktop::screens::map::MapServices {
                    graph: application::graph::KnowledgeGraph::new(store.clone()),
                    decisions: application::decisions::Decisions::new(store.clone()),
                    claims: application::claims::Claims::new(store.clone()),
                }),
                Some(overview(store)),
            ),
            Err(error) => (Err(error), None, None, None, None, None),
        };
        let view = cx.new(|cx| {
            let mut shell = Shell::<SqliteStore>::new(
                cx,
                projects,
                inbox,
                decisions,
                context,
                map,
                Some(settings),
            );
            if let Some(overview) = overview {
                shell.set_overview(overview, cx);
            }
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
                // Opaque unless `XEMNAS_BACKDROP` asks for a material.
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

/// Analysis use case wired with the concrete stores and provider.
type Analysis = application::analysis::AnalyzeCapture<
    SqliteStore,
    application::profile::FileProfileStore,
    ai_provider::KeyringSecretStore,
    ai_provider::ProviderFactory,
>;

/// Runs one capture analysis and logs only counts, codes and status.
fn analyze_capture(
    analysis: &Analysis,
    record: &application::jobs::JobRecord,
) -> Result<(), application::jobs::JobFailure> {
    use application::analysis::AnalysisOutcome;

    match analysis.run(&record.payload, Some(record.id.clone())) {
        Ok(AnalysisOutcome::Extracted(report)) => {
            tracing::info!(
                candidates = report.candidates,
                inserted = report.inserted,
                operation = "analyze_capture",
                "capture analysis finished"
            );
            Ok(())
        }
        Ok(AnalysisOutcome::Skipped) => {
            tracing::info!(
                operation = "analyze_capture",
                "external extraction blocked: consent missing"
            );
            Ok(())
        }
        Ok(AnalysisOutcome::SetupFailed(error)) => {
            tracing::error!(
                code = error.code(),
                operation = "analyze_capture",
                "the AI provider could not start"
            );
            Err(application::jobs::JobFailure::Failed)
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
fn drain_outbox(store: &SqliteStore, root: &std::path::Path) {
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
    match application::outbox::drain(root, &ingest, retention) {
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
fn start_local_api(
    store: SqliteStore,
    runtime_dir: &std::path::Path,
) -> Result<local_api::RunningApi, local_api::ApiServerError> {
    let use_case = std::sync::Arc::new(application::captures::CaptureIngest::new(store.clone()));
    let mut config = local_api::ApiServerConfig::new(use_case, runtime_dir);
    config.services.context = Some(std::sync::Arc::new(
        application::injection::ContextInjection::new(store.clone()),
    ));
    config.services.agent = Some(std::sync::Arc::new(
        application::agent_access::AgentAccess::new(store),
    ));
    local_api::ApiServer::start(config)
}

#[cfg(test)]
mod tests {
    #[test]
    fn smoke() {
        assert_eq!(env!("CARGO_PKG_NAME"), "desktop-gpui");
    }
}
