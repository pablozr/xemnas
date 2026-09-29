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
use storage_sqlite::{default_db_path, SqliteStore};

use xemnas_desktop::app::{Shell, TabNext, TabPrev};

fn main() {
    if let Err(error) = telemetry::init() {
        eprintln!("telemetry init failed: {error}");
    }
    // Suppress job-handler panic payloads (PRIV-001) without changing any other
    // panic behavior.
    application::jobs::install_panic_sanitizer();

    let store = match SqliteStore::open(default_db_path()) {
        Ok(store) => store,
        Err(error) => {
            // The shell logs the technical detail through `tracing` and paints a
            // product-language error state; the raw error never reaches the UI.
            run_shell(Err(error.to_string()));
            return;
        }
    };

    let mut jobs = application::jobs::Jobs::new(store.clone());
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

    run_shell(Ok(application::projects::Projects::new(store)));

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

/// Runs the GPUI application with a ready Projects use case or a failure detail.
fn run_shell(projects: Result<application::projects::Projects<SqliteStore>, String>) {
    application().run(move |cx: &mut App| {
        xemnas_desktop::fonts::register_embedded(cx);
        cx.bind_keys([
            KeyBinding::new("tab", TabNext, None),
            KeyBinding::new("shift-tab", TabPrev, None),
        ]);

        let bounds = Bounds::centered(None, size(px(1440.0), px(1024.0)), cx);
        let view = cx.new(|cx| Shell::<SqliteStore>::new(cx, projects));
        let focus = view.read(cx).initial_focus(cx);

        let opened = cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    // The window title follows the active destination. It used
                    // to hardcode "Projects", so the OS title bar disagreed
                    // with the in-app breadcrumb the moment the user navigated
                    // to Home. The shell updates it on every navigation.
                    title: Some("xemnas — Home".into()),
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
                // so contrast is preserved either way.
                window_background: WindowBackgroundAppearance::MicaAltBackdrop,
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
