//! Phase 3 of the Wind Tunnel protocol — purposelessness detection.
//!
//! Inputs:  runtime records for the full ensemble E and for each
//!          ablated ensemble E \ {u}.
//! Outputs: `PurposeReport` with per-unit contribution scores and the
//!          list of practically purposeless units.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use wt_graph::UnitId;
use wt_dynamic::RuntimeRecord;

// ── S estimate ────────────────────────────────────────────────────────────────

/// S(x) = max(0, dist(x, centre) − radius) + beta
///
/// Same formula as src/validation.py. The centre, radius, and beta of the
/// action-cell C* must be provided by the caller because they are
/// domain-specific (not inferrable from traces alone).
pub fn s_estimate(state: &[f64], centre: &[f64], radius: f64, beta: f64) -> f64 {
    let dist = state.iter().zip(centre.iter())
        .map(|(x, c)| (x - c).powi(2))
        .sum::<f64>()
        .sqrt();
    (dist - radius).max(0.0) + beta
}

// ── OR-success probability ────────────────────────────────────────────────────

/// P_OR(E, Λ) = fraction of terminus states with S ≤ threshold.
///
/// Definition [OR-Success Probability].
pub fn or_success(
    records:   &[RuntimeRecord],
    centre:    &[f64],
    radius:    f64,
    beta:      f64,
    threshold: f64,
) -> f64 {
    if records.is_empty() { return 0.0; }
    let passing = records.iter()
        .filter(|r| s_estimate(&r.terminus, centre, radius, beta) <= threshold)
        .count();
    passing as f64 / records.len() as f64
}

// ── Contribution score ────────────────────────────────────────────────────────

/// δS(u, E) = P_OR(E, Λ) − P_OR(E \ {u}, Λ)
///
/// Definition [Contribution Score].  A score of 0 means the unit is
/// practically purposeless; positive score means it raises the success rate.
pub fn contribution_score(
    full_records:    &[RuntimeRecord],
    ablated_records: &[RuntimeRecord],
    centre:          &[f64],
    radius:          f64,
    beta:            f64,
    threshold:       f64,
) -> f64 {
    let p_full    = or_success(full_records, centre, radius, beta, threshold);
    let p_ablated = or_success(ablated_records, centre, radius, beta, threshold);
    p_full - p_ablated
}

// ── Purposelessness detection ─────────────────────────────────────────────────

/// Units whose contribution score is below `alpha` (significance level).
pub fn purposeless_units(
    scores: &HashMap<UnitId, f64>,
    alpha:  f64,
) -> Vec<UnitId> {
    let mut result: Vec<UnitId> = scores.iter()
        .filter_map(|(id, &score)| if score < alpha { Some(id.clone()) } else { None })
        .collect();
    result.sort_by(|a, b| a.0.cmp(&b.0));
    result
}

// ── Purpose report ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurposeReport {
    pub p_or_full:  f64,
    pub scores:     HashMap<String, f64>,   // UnitId → score (String key for JSON)
    pub purposeless: Vec<String>,
}

impl PurposeReport {
    /// Build a report given the full ensemble's records, a map from each unit
    /// id to its ablated ensemble records, and action-cell parameters.
    pub fn compute(
        full_records:         &[RuntimeRecord],
        ablated_by_unit:      &HashMap<UnitId, Vec<RuntimeRecord>>,
        centre:               &[f64],
        radius:               f64,
        beta:                 f64,
        threshold:            f64,
        purposeless_alpha:    f64,
    ) -> Self {
        let p_full = or_success(full_records, centre, radius, beta, threshold);

        let mut scores_typed: HashMap<UnitId, f64> = HashMap::new();
        for (uid, ablated) in ablated_by_unit {
            let score = contribution_score(
                full_records, ablated, centre, radius, beta, threshold,
            );
            scores_typed.insert(uid.clone(), score);
        }

        let purposeless_ids = purposeless_units(&scores_typed, purposeless_alpha);

        PurposeReport {
            p_or_full:   p_full,
            scores:      scores_typed.into_iter().map(|(k, v)| (k.0, v)).collect(),
            purposeless: purposeless_ids.into_iter().map(|id| id.0).collect(),
        }
    }

    pub fn to_json(&self) -> anyhow::Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use wt_dynamic::{Sample, Trajectory, RuntimeRecord};

    fn record_at(id: &str, terminus: Vec<f64>) -> RuntimeRecord {
        let sample = Sample { t: 1.0, state: terminus.clone() };
        let traj   = Trajectory { unit_id: UnitId::new(id), samples: vec![sample] };
        RuntimeRecord::from_trajectory(traj)
    }

    #[test]
    fn s_estimate_inside_cell() {
        let s = s_estimate(&[0.5], &[0.0], 1.0, 2.5);
        assert!((s - 2.5).abs() < 1e-9, "inside cell: s = {s}");
    }

    #[test]
    fn s_estimate_outside_cell() {
        let s = s_estimate(&[3.0], &[0.0], 1.0, 2.5);
        assert!((s - 4.5).abs() < 1e-9, "outside cell: s = {s}");
    }

    #[test]
    fn or_success_all_inside() {
        let records = vec![
            record_at("u1", vec![0.5]),
            record_at("u2", vec![0.3]),
        ];
        let p = or_success(&records, &[0.0], 1.0, 2.5, 3.0);
        assert!((p - 1.0).abs() < 1e-9, "all inside: p = {p}");
    }

    #[test]
    fn contribution_zero_for_irrelevant_unit() {
        // Full and ablated records are identical → score = 0.
        let records = vec![record_at("u1", vec![0.5])];
        let score = contribution_score(&records, &records, &[0.0], 1.0, 2.5, 3.0);
        assert!(score.abs() < 1e-9);
    }

    #[test]
    fn purposeless_filter() {
        let mut scores = HashMap::new();
        scores.insert(UnitId::new("u1"), 0.0);
        scores.insert(UnitId::new("u2"), 0.3);
        let p = purposeless_units(&scores, 0.05);
        assert_eq!(p, vec![UnitId::new("u1")]);
    }
}
