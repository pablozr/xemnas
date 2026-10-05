//! One bounded clock for everything that moves on its own (the mascot's
//! float, the signal running along the graph's links).
//!
//! A repeating `with_animation` asks for a frame on every display refresh
//! for as long as it is mounted; the mascot is always mounted, so the window
//! never went idle. Zeron measured the same pattern pinning a window at the
//! display rate (36% CPU). Here a view asks for the phase of a loop and takes
//! a short lease; one timer notifies the leased views at 30 or 15 Hz, and
//! parks when no lease is renewed (the view was hidden or closed). Reduced
//! motion returns a still phase and schedules nothing.
//!
//! Adapted from Zeron's `PulseClock` (MIT, © 2026 Wing,
//! github.com/zeronsh/zeron at 27480d99); see `NOTICE`.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use gpui::{App, EntityId, Global};

/// One tick of the clock (~30 Hz).
const TICK: Duration = Duration::from_millis(33);
/// How long a view stays driven after its last paint asked for a phase.
const LEASE: Duration = Duration::from_millis(300);

/// How often a leased view repaints.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rate {
    /// ~30 Hz: thin lines and text, where steps would show.
    Smooth,
    /// ~15 Hz: slow, large motion (a float of a few pixels).
    Calm,
}

impl Rate {
    fn stride(self) -> u64 {
        match self {
            Self::Smooth => 1,
            Self::Calm => 2,
        }
    }
}

struct Lease {
    until: Instant,
    stride: u64,
}

struct Clock {
    epoch: Instant,
    leases: HashMap<EntityId, Lease>,
    tick: u64,
    running: bool,
    /// Whether the window has the focus; nothing moves on its own while it
    /// does not (the loops park and nothing is repainted for them).
    active: bool,
}

impl Global for Clock {}

impl Default for Clock {
    fn default() -> Self {
        Self {
            epoch: Instant::now(),
            leases: HashMap::new(),
            tick: 0,
            running: false,
            active: true,
        }
    }
}

/// Tells the clock whether the window has the focus. Losing it parks every
/// loop; regaining it repaints once so the views take their leases again.
pub fn set_window_active(active: bool, cx: &mut App) {
    let clock = cx.default_global::<Clock>();
    if clock.active == active {
        return;
    }
    clock.active = active;
    if active {
        cx.refresh_windows();
    }
}

/// Whether the window has the focus (true until told otherwise).
pub fn window_active(cx: &App) -> bool {
    cx.try_global::<Clock>().is_none_or(|clock| clock.active)
}

/// Phase in `[0, 1)` of a loop lasting `period`, shared by every view so
/// loops stay in step; renews `view`'s lease at `rate`. Call it while
/// painting the moving thing, every frame it is visible.
pub fn phase(period: Duration, view: EntityId, rate: Rate, cx: &mut App) -> f32 {
    if cx.reduce_motion() {
        return 0.0;
    }
    lease(view, rate, cx);
    let clock = cx.default_global::<Clock>();
    (clock.epoch.elapsed().as_secs_f32() / period.as_secs_f32().max(0.001)).fract()
}

/// Seconds since the clock's epoch, renewing `view`'s lease (for motion
/// computed from time rather than a loop).
pub fn seconds(view: EntityId, rate: Rate, cx: &mut App) -> f32 {
    if !cx.reduce_motion() {
        lease(view, rate, cx);
    }
    cx.default_global::<Clock>().epoch.elapsed().as_secs_f32()
}

fn lease(view: EntityId, rate: Rate, cx: &mut App) {
    let now = Instant::now();
    let clock = cx.default_global::<Clock>();
    let entry = clock.leases.entry(view).or_insert(Lease {
        until: now,
        stride: rate.stride(),
    });
    entry.until = now + LEASE;
    entry.stride = rate.stride();
    if clock.running {
        return;
    }
    clock.running = true;
    cx.spawn(async move |cx| loop {
        cx.background_executor().timer(TICK).await;
        let parked = cx.update(|cx| {
            let clock = cx.default_global::<Clock>();
            let now = Instant::now();
            clock.leases.retain(|_, lease| lease.until > now);
            if !clock.active {
                clock.leases.clear();
            }
            if clock.leases.is_empty() {
                clock.running = false;
                return true;
            }
            clock.tick = clock.tick.wrapping_add(1);
            let tick = clock.tick;
            let due: Vec<EntityId> = clock
                .leases
                .iter()
                .filter(|(_, lease)| tick.is_multiple_of(lease.stride))
                .map(|(view, _)| *view)
                .collect();
            for view in due {
                cx.notify(view);
            }
            false
        });
        if parked {
            break;
        }
    })
    .detach();
}
