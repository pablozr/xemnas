//! Shared cap on concurrent AI-provider calls, across every job lane.
//!
//! Workers of all lanes share one [`ProviderLimiter`]: at most `limit` calls
//! run at once, a waiting call from the `now` lane goes before waiting calls
//! of the other lanes, and after the provider asks to slow down (HTTP 429)
//! nobody calls it until the pause ends: the caller gets the time left and
//! requeues its job instead of holding a worker.

use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use crate::jobs::{current_lane, Lane};

/// Fewest and most provider calls at once the setting allows.
pub const PARALLEL_RANGE: std::ops::RangeInclusive<u8> = 1..=4;

/// Provider calls at once when nothing was chosen.
pub const DEFAULT_PARALLEL: u8 = 2;

/// Why no permit was handed out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// The provider asked for a pause; this much of it is left.
    Paused(Duration),
    /// The app is shutting down.
    Closed,
}

#[derive(Debug)]
struct State {
    limit: usize,
    active: usize,
    urgent_waiting: usize,
    paused_until: Option<Instant>,
    closed: bool,
}

/// Counting semaphore with priority for the `now` lane and a shared pause.
#[derive(Debug, Clone)]
pub struct ProviderLimiter {
    inner: Arc<(Mutex<State>, Condvar)>,
}

impl ProviderLimiter {
    /// A limiter for `limit` calls at once (at least one).
    pub fn new(limit: usize) -> Self {
        Self {
            inner: Arc::new((
                Mutex::new(State {
                    limit: limit.max(1),
                    active: 0,
                    urgent_waiting: 0,
                    paused_until: None,
                    closed: false,
                }),
                Condvar::new(),
            )),
        }
    }

    fn state(&self) -> std::sync::MutexGuard<'_, State> {
        self.inner
            .0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Waits for a free slot. Calls made from a `now` worker go first.
    pub fn acquire(&self) -> Result<Permit, Refusal> {
        let urgent = current_lane() == Some(Lane::Now);
        let mut state = self.state();
        if urgent {
            state.urgent_waiting += 1;
        }
        let result = loop {
            if state.closed {
                break Err(Refusal::Closed);
            }
            if let Some(until) = state.paused_until {
                let now = Instant::now();
                if until > now {
                    break Err(Refusal::Paused(until - now));
                }
                state.paused_until = None;
            }
            let yields = !urgent && state.urgent_waiting > 0;
            if state.active < state.limit && !yields {
                state.active += 1;
                break Ok(Permit {
                    limiter: self.clone(),
                });
            }
            state = self
                .inner
                .1
                .wait(state)
                .unwrap_or_else(|poisoned| poisoned.into_inner());
        };
        if urgent {
            state.urgent_waiting -= 1;
            self.inner.1.notify_all();
        }
        result
    }

    /// Stops every call for `delay` (keeps a longer pause already set).
    pub fn pause_for(&self, delay: Duration) {
        let until = Instant::now() + delay;
        let mut state = self.state();
        if state.paused_until.is_none_or(|current| current < until) {
            state.paused_until = Some(until);
        }
        self.inner.1.notify_all();
    }

    /// Refuses every waiting and future call; used on shutdown so no worker
    /// stays blocked here.
    pub fn close(&self) {
        self.state().closed = true;
        self.inner.1.notify_all();
    }

    /// Calls running right now.
    pub fn active(&self) -> usize {
        self.state().active
    }
}

/// One running provider call; dropping it frees the slot.
#[derive(Debug)]
pub struct Permit {
    limiter: ProviderLimiter,
}

impl Drop for Permit {
    fn drop(&mut self) {
        let mut state = self.limiter.state();
        state.active = state.active.saturating_sub(1);
        self.limiter.inner.1.notify_all();
    }
}

#[cfg(test)]
mod tests {
    use super::{ProviderLimiter, Refusal};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::time::Duration;

    #[test]
    fn never_more_calls_than_the_limit() {
        let limiter = ProviderLimiter::new(2);
        let running = Arc::new(AtomicUsize::new(0));
        let peak = Arc::new(AtomicUsize::new(0));
        let threads: Vec<_> = (0..8)
            .map(|_| {
                let (limiter, running, peak) = (limiter.clone(), running.clone(), peak.clone());
                std::thread::spawn(move || {
                    for _ in 0..5 {
                        let _permit = limiter.acquire().expect("permit");
                        let now = running.fetch_add(1, Ordering::SeqCst) + 1;
                        peak.fetch_max(now, Ordering::SeqCst);
                        std::thread::sleep(Duration::from_millis(2));
                        running.fetch_sub(1, Ordering::SeqCst);
                    }
                })
            })
            .collect();
        for thread in threads {
            thread.join().expect("thread");
        }
        assert_eq!(peak.load(Ordering::SeqCst), 2);
        assert_eq!(limiter.active(), 0);
    }

    #[test]
    fn a_pause_refuses_calls_with_the_time_left_and_close_wakes_waiters() {
        let limiter = ProviderLimiter::new(1);
        limiter.pause_for(Duration::from_secs(30));
        match limiter.acquire() {
            Err(Refusal::Paused(left)) => assert!(left > Duration::from_secs(25)),
            other => panic!("expected a pause, got {other:?}"),
        }

        let limiter = ProviderLimiter::new(1);
        let held = limiter.acquire().expect("first permit");
        let waiter = {
            let limiter = limiter.clone();
            std::thread::spawn(move || limiter.acquire().map(|_| ()))
        };
        std::thread::sleep(Duration::from_millis(20));
        limiter.close();
        assert_eq!(waiter.join().expect("thread"), Err(Refusal::Closed));
        drop(held);
    }
}
