//! Whether captures reach the project, read from what the app really knows:
//! the capture status set at startup and the project's latest captures. The
//! app cannot see an agent-side hook or MCP that failed, so the honest sign
//! of that case is the time since the last capture ([`STALE_AFTER`]).

use application::capture_progress::{AssessmentReason, CaptureProgress, CaptureState};
use chrono::{DateTime, Local};

use super::format::{local, relative};
use crate::app::CaptureStatus;
use crate::i18n::inbox_ticket as tt;

/// A listening capture with nothing new for this long reads as stalled.
pub(crate) const STALE_AFTER: chrono::Duration = chrono::Duration::hours(24);

/// The state of the capture shown on the empty queue. Failures are not a
/// state: they add to the listening ones ([`CaptureView::failed`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CaptureHealth {
    /// Listening, the last capture is recent.
    Active,
    /// Listening, no capture loaded yet.
    Waiting,
    /// Listening, the last capture is [`STALE_AFTER`] old or older.
    Stale,
    /// The local API did not start.
    Stopped,
    /// Sample data, no capture runs.
    Demo,
}

/// What the empty queue and the project bar say about the capture.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CaptureView {
    pub health: CaptureHealth,
    /// `received_at` of the latest capture, as recorded (the screen formats it).
    pub last: Option<String>,
    /// Captures of the project whose analysis failed.
    pub failed: usize,
}

/// Reads the capture state of one project at `now`. A date that does not
/// parse never makes the capture stale.
pub(crate) fn capture_view(
    status: CaptureStatus,
    progress: &[CaptureProgress],
    now: DateTime<Local>,
) -> CaptureView {
    let latest = progress
        .iter()
        .filter_map(|capture| local(&capture.received_at).map(|date| (date, capture)))
        .max_by_key(|(date, _)| *date);
    let last = latest
        .map(|(_, capture)| capture)
        .or(progress.first())
        .map(|capture| capture.received_at.clone());
    let failed = progress
        .iter()
        .filter(|capture| analysis_failed(capture))
        .count();
    let health = match status {
        CaptureStatus::Unavailable => CaptureHealth::Stopped,
        CaptureStatus::Demo => CaptureHealth::Demo,
        CaptureStatus::Listening if progress.is_empty() => CaptureHealth::Waiting,
        CaptureStatus::Listening => match latest {
            Some((date, _)) if now - date >= STALE_AFTER => CaptureHealth::Stale,
            _ => CaptureHealth::Active,
        },
    };
    CaptureView {
        health,
        last,
        failed,
    }
}

impl CaptureView {
    /// The last capture as "2 h" or "Sep 29, 2026"; empty with none.
    pub(crate) fn last_relative(&self) -> String {
        self.last.as_deref().map(relative).unwrap_or_default()
    }

    /// Title and body of a capture that stopped or where nothing arrives,
    /// shared by the empty queue's card and the project bar's pill.
    pub(crate) fn warning(&self) -> Option<(String, &'static str)> {
        match self.health {
            CaptureHealth::Stopped => Some((
                tt::capture_stopped_title().to_owned(),
                tt::capture_stopped_body(),
            )),
            CaptureHealth::Stale => Some((
                tt::capture_stale_title(&self.last_relative()),
                tt::capture_stale_body(),
            )),
            _ => None,
        }
    }
}

/// The pill in the project bar: only when something needs a look.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PillKind {
    /// Nothing arrives for a day or more.
    Stale,
    /// The capture is stopped.
    Stopped,
    /// Captures arrive, but some analyses failed.
    Failed,
}

/// Which pill the project bar shows, if any: a stalled or stopped capture
/// wins over failures (one pill at a time); working, waiting and demo
/// captures show none.
pub(crate) fn pill_kind(view: &CaptureView) -> Option<PillKind> {
    match view.health {
        CaptureHealth::Stale => Some(PillKind::Stale),
        CaptureHealth::Stopped => Some(PillKind::Stopped),
        CaptureHealth::Demo => None,
        CaptureHealth::Active | CaptureHealth::Waiting => {
            (view.failed > 0).then_some(PillKind::Failed)
        }
    }
}

/// A failed job, or a finished one whose assessment failed (also the rule of
/// the capture pill in Diagnostics, `inbox::capture_pill`).
pub(crate) fn analysis_failed(capture: &CaptureProgress) -> bool {
    capture.state == CaptureState::Failed
        || (capture.state == CaptureState::Completed && capture.reason == AssessmentReason::Failed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now() -> DateTime<Local> {
        DateTime::parse_from_rfc3339("2026-10-02T12:00:00Z")
            .unwrap()
            .with_timezone(&Local)
    }

    fn capture(
        received_at: &str,
        state: CaptureState,
        reason: AssessmentReason,
    ) -> CaptureProgress {
        CaptureProgress {
            project_id: "p".into(),
            capture_id: received_at.into(),
            received_at: received_at.into(),
            title: None,
            adapter: None,
            job_id: None,
            attempts: 1,
            state,
            reason,
            source: None,
            model: None,
            durable: 0,
            detail: 0,
            candidates: Default::default(),
            failure_detail: None,
            can_retry: false,
        }
    }

    fn done(received_at: &str) -> CaptureProgress {
        capture(
            received_at,
            CaptureState::Completed,
            AssessmentReason::Candidates,
        )
    }

    #[test]
    fn listening_with_recent_capture_is_active() {
        let view = capture_view(
            CaptureStatus::Listening,
            &[done("2026-10-01T09:00:00Z"), done("2026-10-02T11:43:00Z")],
            now(),
        );
        assert_eq!(view.health, CaptureHealth::Active);
        assert_eq!(view.last.as_deref(), Some("2026-10-02T11:43:00Z"));
        assert_eq!(view.failed, 0);
    }

    #[test]
    fn listening_without_captures_is_waiting() {
        let view = capture_view(CaptureStatus::Listening, &[], now());
        assert_eq!(view.health, CaptureHealth::Waiting);
        assert_eq!(view.last, None);
    }

    #[test]
    fn listening_after_24h_is_stale() {
        let at = |value| capture_view(CaptureStatus::Listening, &[done(value)], now()).health;
        assert_eq!(at("2026-10-01T12:00:00Z"), CaptureHealth::Stale);
        assert_eq!(at("2026-10-01T12:01:00Z"), CaptureHealth::Active);
    }

    #[test]
    fn unavailable_is_stopped_regardless_of_captures() {
        for progress in [vec![], vec![done("2026-10-02T11:43:00Z")]] {
            let view = capture_view(CaptureStatus::Unavailable, &progress, now());
            assert_eq!(view.health, CaptureHealth::Stopped);
        }
    }

    #[test]
    fn demo_is_demo() {
        let view = capture_view(CaptureStatus::Demo, &[done("2026-09-29T10:00:00Z")], now());
        assert_eq!(view.health, CaptureHealth::Demo);
    }

    #[test]
    fn failed_counts_both_failure_shapes() {
        let progress = [
            capture(
                "2026-10-02T11:00:00Z",
                CaptureState::Failed,
                AssessmentReason::Unknown,
            ),
            capture(
                "2026-10-02T10:00:00Z",
                CaptureState::Completed,
                AssessmentReason::Failed,
            ),
            done("2026-10-02T09:00:00Z"),
            capture(
                "2026-10-02T08:00:00Z",
                CaptureState::Queued,
                AssessmentReason::Unknown,
            ),
        ];
        let view = capture_view(CaptureStatus::Listening, &progress, now());
        assert_eq!(view.failed, 2);
        assert_eq!(view.health, CaptureHealth::Active);
    }

    fn view(health: CaptureHealth, failed: usize) -> CaptureView {
        CaptureView {
            health,
            last: None,
            failed,
        }
    }

    #[test]
    fn ok_states_have_no_pill() {
        for health in [
            CaptureHealth::Active,
            CaptureHealth::Waiting,
            CaptureHealth::Demo,
        ] {
            assert_eq!(pill_kind(&view(health, 0)), None, "{health:?}");
        }
        assert_eq!(pill_kind(&view(CaptureHealth::Demo, 2)), None);
    }

    #[test]
    fn stale_and_stopped_pill_warning() {
        assert_eq!(
            pill_kind(&view(CaptureHealth::Stale, 0)),
            Some(PillKind::Stale)
        );
        assert_eq!(
            pill_kind(&view(CaptureHealth::Stopped, 0)),
            Some(PillKind::Stopped)
        );
    }

    #[test]
    fn failures_alone_pill_danger() {
        assert_eq!(
            pill_kind(&view(CaptureHealth::Active, 1)),
            Some(PillKind::Failed)
        );
        assert_eq!(
            pill_kind(&view(CaptureHealth::Waiting, 3)),
            Some(PillKind::Failed)
        );
    }

    #[test]
    fn warning_wins_over_failures() {
        assert_eq!(
            pill_kind(&view(CaptureHealth::Stale, 4)),
            Some(PillKind::Stale)
        );
        assert_eq!(
            pill_kind(&view(CaptureHealth::Stopped, 4)),
            Some(PillKind::Stopped)
        );
    }

    #[test]
    fn unparseable_dates_never_make_it_stale() {
        let view = capture_view(CaptureStatus::Listening, &[done("not a date")], now());
        assert_eq!(view.health, CaptureHealth::Active);
        assert_eq!(view.last.as_deref(), Some("not a date"));
    }
}
