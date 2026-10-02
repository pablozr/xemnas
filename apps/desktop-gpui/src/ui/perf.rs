//! Opt-in render timing for the screens (`XEMNAS_PERF=1`).
//!
//! Every scroll step or animation frame re-runs the `render` of the view that
//! owns the moving element, so how long that takes is how smooth the app
//! feels. With `XEMNAS_PERF` set, a [`Probe`] appends one line per render to
//! `xemnas-perf.log` in the temp folder: the name and the milliseconds spent
//! building the element tree. Without it the probe costs one cached lookup.
//!
//! Build time is not the whole frame (layout, paint and presenting follow),
//! and the window's pacing belongs to the platform, so use it to compare
//! screens and to see what grows with the data, not as an absolute rate.

use std::io::Write;
use std::sync::OnceLock;
use std::time::Instant;

fn epoch() -> Instant {
    static EPOCH: OnceLock<Instant> = OnceLock::new();
    *EPOCH.get_or_init(Instant::now)
}

fn enabled() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("XEMNAS_PERF").is_some())
}

/// Times from construction to drop; each line also says when it started,
/// in milliseconds since the first probe, to line renders up with events.
pub struct Probe(&'static str, Option<Instant>);

impl Probe {
    /// Starts timing a render named `name`.
    pub fn start(name: &'static str) -> Self {
        Self(name, enabled().then(Instant::now))
    }
}

impl Drop for Probe {
    fn drop(&mut self) {
        let Some(start) = self.1 else {
            return;
        };
        let path = std::env::temp_dir().join("xemnas-perf.log");
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
        {
            let _ = writeln!(
                file,
                "{} {:.2} at={:.0}",
                self.0,
                start.elapsed().as_secs_f64() * 1000.0,
                start.duration_since(epoch()).as_secs_f64() * 1000.0
            );
        }
    }
}
