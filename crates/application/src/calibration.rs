//! Does the extractor's confidence tell what a person accepts?
//!
//! Every candidate carries a confidence the model gave itself. It is an
//! estimate, not a probability, and any automatic approval would lean on it,
//! so it has to earn that: among the candidates a person already decided, do
//! the ones with high confidence get accepted more than the ones with low?
//! This module answers from the decided candidates alone (a pure function,
//! no storage, no interface), so the answer can gate any later automation.
//!
//! The measure is the chance that a candidate a person kept had a higher
//! confidence than one they dismissed (the area under the ROC curve, from
//! ranks, ties counting half): 0.5 is a coin toss, 1.0 separates perfectly.

use serde::{Deserialize, Serialize};

/// Decided candidates needed before the answer means anything.
pub const MIN_DECIDED: usize = 30;
/// Lower edges of the confidence bins after the first: `[0, .5)`, `[.5, .7)`,
/// `[.7, .85)` and `[.85, 1]`.
pub const BIN_EDGES: [f64; 3] = [0.5, 0.7, 0.85];
/// Separation from which the confidence is taken to predict acceptance.
pub const PREDICTS_AT: f64 = 0.70;
/// Separation from which it is taken to say something, but weakly.
pub const WEAK_AT: f64 = 0.60;

/// What a person did with a candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// Confirmed as proposed.
    Accepted,
    /// Confirmed after editing it.
    Edited,
    /// Rejected.
    Dismissed,
}

impl Outcome {
    /// Whether the candidate was kept (as proposed or edited).
    pub fn kept(self) -> bool {
        self != Self::Dismissed
    }
}

/// One decided candidate.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sample {
    /// The extractor's confidence, `[0, 1]`.
    pub confidence: f64,
    /// How much it matters, `[0, 1]`.
    pub significance: f64,
    /// What was decided.
    pub outcome: Outcome,
}

/// What the numbers say about trusting the confidence.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    /// Too few decisions, or only one kind: no answer yet.
    #[default]
    TooFew,
    /// The confidence does not tell accepted from dismissed.
    NoSignal,
    /// It tells them apart, but not well enough to lean on.
    Weak,
    /// It predicts what a person accepts.
    Predicts,
}

impl Verdict {
    /// Whether automation may rely on the confidence.
    pub fn allows_automation(self) -> bool {
        self == Self::Predicts
    }
}

/// Decided candidates inside one confidence range.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Bin {
    /// Inclusive lower edge.
    pub from: f64,
    /// Upper edge (inclusive only for the last bin).
    pub to: f64,
    /// Decided candidates in the range.
    pub total: i64,
    /// Accepted as proposed.
    pub accepted: i64,
    /// Accepted after editing.
    pub edited: i64,
    /// Dismissed.
    pub dismissed: i64,
}

impl Bin {
    /// Share kept (accepted or edited), when the bin has any.
    pub fn kept_share(&self) -> Option<f64> {
        (self.total > 0).then(|| (self.accepted + self.edited) as f64 / self.total as f64)
    }
}

/// The answer: how the decided candidates split by confidence and whether the
/// confidence tells them apart.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Calibration {
    /// Candidates decided (accepted, edited or dismissed).
    pub decided: i64,
    /// Accepted as proposed.
    pub accepted: i64,
    /// Accepted after editing.
    pub edited: i64,
    /// Dismissed.
    pub dismissed: i64,
    /// The four confidence bins, lowest first.
    pub bins: Vec<Bin>,
    /// Separation of kept from dismissed by confidence, when both exist.
    pub confidence_separation: Option<f64>,
    /// The same by significance.
    pub significance_separation: Option<f64>,
    /// What to conclude.
    pub verdict: Verdict,
}

/// Separation of the kept samples from the dismissed ones by `value`: the
/// chance a kept one ranks above a dismissed one, ties counting half. `None`
/// when either kind is missing.
fn separation(samples: &[Sample], value: impl Fn(&Sample) -> f64) -> Option<f64> {
    let kept = samples
        .iter()
        .filter(|sample| sample.outcome.kept())
        .count();
    let dismissed = samples.len() - kept;
    if kept == 0 || dismissed == 0 {
        return None;
    }
    let mut ranked: Vec<(f64, bool)> = samples
        .iter()
        .map(|sample| (value(sample), sample.outcome.kept()))
        .collect();
    ranked.sort_by(|a, b| a.0.total_cmp(&b.0));
    // Average rank (from 1) over each run of equal values.
    let mut kept_rank_sum = 0.0;
    let mut start = 0;
    while start < ranked.len() {
        let mut end = start;
        while end + 1 < ranked.len() && ranked[end + 1].0 == ranked[start].0 {
            end += 1;
        }
        let rank = (start + end) as f64 / 2.0 + 1.0;
        kept_rank_sum += ranked[start..=end]
            .iter()
            .filter(|(_, is_kept)| *is_kept)
            .count() as f64
            * rank;
        start = end + 1;
    }
    let (kept, dismissed) = (kept as f64, dismissed as f64);
    Some((kept_rank_sum - kept * (kept + 1.0) / 2.0) / (kept * dismissed))
}

/// Which bin a confidence falls in.
fn bin_of(confidence: f64) -> usize {
    BIN_EDGES.iter().filter(|edge| confidence >= **edge).count()
}

/// Summarizes decided candidates.
pub fn calibrate(samples: &[Sample]) -> Calibration {
    let mut bins: Vec<Bin> = (0..=BIN_EDGES.len())
        .map(|at| Bin {
            from: if at == 0 { 0.0 } else { BIN_EDGES[at - 1] },
            to: BIN_EDGES.get(at).copied().unwrap_or(1.0),
            ..Bin::default()
        })
        .collect();
    let mut calibration = Calibration::default();
    for sample in samples {
        let bin = &mut bins[bin_of(sample.confidence.clamp(0.0, 1.0))];
        bin.total += 1;
        calibration.decided += 1;
        match sample.outcome {
            Outcome::Accepted => {
                bin.accepted += 1;
                calibration.accepted += 1;
            }
            Outcome::Edited => {
                bin.edited += 1;
                calibration.edited += 1;
            }
            Outcome::Dismissed => {
                bin.dismissed += 1;
                calibration.dismissed += 1;
            }
        }
    }
    calibration.bins = bins;
    calibration.confidence_separation = separation(samples, |sample| sample.confidence);
    calibration.significance_separation = separation(samples, |sample| sample.significance);
    calibration.verdict = match calibration.confidence_separation {
        Some(value) if samples.len() >= MIN_DECIDED => {
            if value >= PREDICTS_AT {
                Verdict::Predicts
            } else if value >= WEAK_AT {
                Verdict::Weak
            } else {
                Verdict::NoSignal
            }
        }
        _ => Verdict::TooFew,
    };
    calibration
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(confidence: f64, outcome: Outcome) -> Sample {
        Sample {
            confidence,
            significance: 0.5,
            outcome,
        }
    }

    #[test]
    fn a_confidence_that_orders_the_outcomes_predicts() {
        // High confidence kept, low dismissed, 40 of each.
        let mut samples = Vec::new();
        for n in 0..40 {
            samples.push(sample(0.8 + n as f64 * 0.004, Outcome::Accepted));
            samples.push(sample(0.3 + n as f64 * 0.004, Outcome::Dismissed));
        }
        let calibration = calibrate(&samples);
        assert_eq!(calibration.confidence_separation, Some(1.0));
        assert_eq!(calibration.verdict, Verdict::Predicts);
        assert!(calibration.verdict.allows_automation());
        assert_eq!(calibration.decided, 80);
        assert_eq!(calibration.bins[0].dismissed, 40);
        assert_eq!(
            calibration.bins[2].accepted + calibration.bins[3].accepted,
            40
        );
        assert_eq!(calibration.bins[3].kept_share(), Some(1.0));
        assert_eq!(calibration.bins[0].kept_share(), Some(0.0));
    }

    #[test]
    fn a_confidence_unrelated_to_the_outcome_is_a_coin_toss() {
        // The same confidences for kept and dismissed.
        let mut samples = Vec::new();
        for n in 0..40 {
            let confidence = 0.4 + (n % 10) as f64 * 0.05;
            samples.push(sample(confidence, Outcome::Accepted));
            samples.push(sample(confidence, Outcome::Dismissed));
        }
        let calibration = calibrate(&samples);
        assert_eq!(calibration.confidence_separation, Some(0.5));
        assert_eq!(calibration.verdict, Verdict::NoSignal);
        assert!(!calibration.verdict.allows_automation());
    }

    #[test]
    fn a_confidence_that_is_backwards_is_no_signal() {
        let mut samples = Vec::new();
        for n in 0..40 {
            samples.push(sample(0.3 + n as f64 * 0.004, Outcome::Accepted));
            samples.push(sample(0.8 + n as f64 * 0.004, Outcome::Dismissed));
        }
        assert_eq!(calibrate(&samples).verdict, Verdict::NoSignal);
    }

    #[test]
    fn a_middling_separation_is_weak() {
        // Kept ones are high more often (5 in 8) than dismissed ones (3 in 8).
        let mut samples = Vec::new();
        for n in 0..40 {
            let kept_high = n % 8 < 5;
            let dismissed_high = n % 8 < 3;
            samples.push(sample(
                if kept_high { 0.85 } else { 0.45 },
                Outcome::Accepted,
            ));
            samples.push(sample(
                if dismissed_high { 0.85 } else { 0.45 },
                Outcome::Dismissed,
            ));
        }
        let calibration = calibrate(&samples);
        let value = calibration.confidence_separation.unwrap();
        assert!(value > WEAK_AT && value < PREDICTS_AT, "{value}");
        assert_eq!(calibration.verdict, Verdict::Weak);
    }

    #[test]
    fn too_few_decisions_or_one_kind_only_gives_no_answer() {
        let few: Vec<Sample> = (0..10)
            .flat_map(|n| {
                [
                    sample(0.9, Outcome::Accepted),
                    sample(0.2 + n as f64 * 0.01, Outcome::Dismissed),
                ]
            })
            .collect();
        assert_eq!(calibrate(&few).verdict, Verdict::TooFew, "20 decisions");
        let one_kind: Vec<Sample> = (0..60).map(|_| sample(0.9, Outcome::Accepted)).collect();
        let calibration = calibrate(&one_kind);
        assert_eq!(calibration.confidence_separation, None);
        assert_eq!(calibration.verdict, Verdict::TooFew);
        assert_eq!(calibrate(&[]).decided, 0);
        assert_eq!(calibrate(&[]).bins.len(), 4);
    }

    #[test]
    fn edited_counts_as_kept_but_apart_from_accepted() {
        let samples = [
            sample(0.9, Outcome::Edited),
            sample(0.9, Outcome::Accepted),
            sample(0.1, Outcome::Dismissed),
        ];
        let calibration = calibrate(&samples);
        assert_eq!((calibration.accepted, calibration.edited), (1, 1));
        assert_eq!(calibration.bins[3].kept_share(), Some(1.0));
        assert_eq!(calibration.confidence_separation, Some(1.0));
    }

    #[test]
    fn bins_follow_the_edges() {
        assert_eq!(bin_of(0.0), 0);
        assert_eq!(bin_of(0.499), 0);
        assert_eq!(bin_of(0.5), 1);
        assert_eq!(bin_of(0.7), 2);
        assert_eq!(bin_of(0.85), 3);
        assert_eq!(bin_of(1.0), 3);
    }
}
