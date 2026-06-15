//! Phase 2 of the Wind Tunnel protocol — dynamic analysis.
//!
//! Inputs:  runtime traces (JSONL files, one RuntimeRecord per unit).
//! Outputs: `DynamicReport` with R_dyn time series, holonomy violations,
//!          and aperture drift events.

use std::f64::consts::PI;
use std::path::Path;

use serde::{Deserialize, Serialize};
use wt_graph::{SystemGraph, UnitId};

// ── Trace types ───────────────────────────────────────────────────────────────

/// One observation from a running unit at time t.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sample {
    pub t:     f64,
    pub state: Vec<f64>,
}

/// Full trajectory of a unit under load.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trajectory {
    pub unit_id: UnitId,
    pub samples: Vec<Sample>,
}

/// Everything recorded for a single unit during a wind-tunnel run.
/// Corresponds to Definition [Runtime Record].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeRecord {
    pub unit_id:    UnitId,
    pub trajectory: Trajectory,
    /// State at end of observation window: Γ_u = γ_u(T).
    pub terminus:   Vec<f64>,
    /// All states visited — used for aperture-drift checking.
    pub memory:     Vec<Vec<f64>>,
}

impl RuntimeRecord {
    pub fn from_trajectory(traj: Trajectory) -> Self {
        let terminus = traj.samples.last()
            .map(|s| s.state.clone())
            .unwrap_or_default();
        let memory = traj.samples.iter().map(|s| s.state.clone()).collect();
        let unit_id = traj.unit_id.clone();
        Self { unit_id, trajectory: traj, terminus, memory }
    }
}

// ── Trace loading ─────────────────────────────────────────────────────────────

/// Load runtime records from a directory of JSONL files (one file per unit).
/// Each line in a file is a JSON-serialised `Sample`; the filename (stem) is
/// used as the unit id.
pub fn load_traces(dir: &Path) -> anyhow::Result<Vec<RuntimeRecord>> {
    use std::io::BufRead;
    let mut records = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path  = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("jsonl") { continue; }
        let unit_id = UnitId::new(
            path.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown"),
        );
        let file = std::fs::File::open(&path)?;
        let reader = std::io::BufReader::new(file);
        let mut samples = Vec::new();
        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() { continue; }
            let sample: Sample = serde_json::from_str(&line)?;
            samples.push(sample);
        }
        let traj = Trajectory { unit_id: unit_id.clone(), samples };
        records.push(RuntimeRecord::from_trajectory(traj));
    }
    Ok(records)
}

// ── Phase estimation ──────────────────────────────────────────────────────────

/// Extract a phase angle θ̂ ∈ [0, 2π) from a unit's trajectory at time t.
/// Corresponds to the θ̂_k(t) in Definition [Dynamic Order Parameter].
pub trait PhaseEstimator: Send + Sync {
    fn estimate(&self, record: &RuntimeRecord, t: f64) -> f64;
}

/// Default estimator: linearly interpolate the first state dimension and
/// map it to [0, 2π) via sin/cos approximation.
pub struct ScalarPhaseEstimator;

impl PhaseEstimator for ScalarPhaseEstimator {
    fn estimate(&self, record: &RuntimeRecord, t: f64) -> f64 {
        let samples = &record.trajectory.samples;
        if samples.is_empty() { return 0.0; }
        // Linear interpolation between bracketing samples.
        let val = interp_scalar(samples, t, 0);
        // Map to [0, 2π) via modular winding; treat the scalar as a phase angle.
        val.rem_euclid(2.0 * PI)
    }
}

fn interp_scalar(samples: &[Sample], t: f64, dim: usize) -> f64 {
    if samples.len() == 1 {
        return samples[0].state.get(dim).copied().unwrap_or(0.0);
    }
    // Find bracketing samples.
    let i = samples.partition_point(|s| s.t <= t).saturating_sub(1);
    let i = i.min(samples.len() - 2);
    let s0 = &samples[i];
    let s1 = &samples[i + 1];
    let v0 = s0.state.get(dim).copied().unwrap_or(0.0);
    let v1 = s1.state.get(dim).copied().unwrap_or(0.0);
    if (s1.t - s0.t).abs() < 1e-12 { return v0; }
    let alpha = (t - s0.t) / (s1.t - s0.t);
    v0 + alpha * (v1 - v0)
}

// ── Dynamic order parameter ───────────────────────────────────────────────────

/// R_dyn(t) = |1/n Σ exp(iθ̂_k(t))|
///
/// Definition [Dynamic Order Parameter].
pub fn r_dyn(records: &[RuntimeRecord], t: f64, estimator: &dyn PhaseEstimator) -> f64 {
    if records.is_empty() { return 0.0; }
    let (re, im) = records.iter()
        .map(|rec| {
            let theta = estimator.estimate(rec, t);
            (theta.cos(), theta.sin())
        })
        .fold((0.0_f64, 0.0_f64), |(ar, ai), (r, i)| (ar + r, ai + i));
    let n = records.len() as f64;
    ((re / n).powi(2) + (im / n).powi(2)).sqrt()
}

/// Time series of R_dyn sampled every `dt` seconds over the trajectory span.
pub fn r_dyn_series(
    records:   &[RuntimeRecord],
    dt:        f64,
    estimator: &dyn PhaseEstimator,
) -> Vec<(f64, f64)> {
    let t_max = records.iter()
        .flat_map(|r| r.trajectory.samples.iter().map(|s| s.t))
        .fold(f64::NEG_INFINITY, f64::max);
    if t_max <= 0.0 { return vec![]; }
    let steps = ((t_max / dt).ceil() as usize).max(1);
    (0..=steps)
        .map(|i| {
            let t = (i as f64 * dt).min(t_max);
            (t, r_dyn(records, t, estimator))
        })
        .collect()
}

// ── Holonomy measurement ──────────────────────────────────────────────────────

/// A live holonomy violation detected on a directed cycle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HolonomyViolation {
    pub cycle:     Vec<UnitId>,
    /// ‖Γ_{i_k} - T_c^spec(γ_{i_1}(0))‖
    pub magnitude: f64,
}

/// Euclidean norm of a state vector difference.
fn norm_diff(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b.iter())
        .map(|(x, y)| (x - y).powi(2))
        .sum::<f64>()
        .sqrt()
}

/// Measure dynamic holonomy for one cycle.
///
/// `spec_fn` maps the initial state of the first unit to the *intended*
/// terminal state after a full traversal of the cycle.
pub fn holonomy_dyn(
    cycle:   &[UnitId],
    records: &[RuntimeRecord],
    spec_fn: &dyn Fn(&[f64]) -> Vec<f64>,
) -> Option<f64> {
    if cycle.is_empty() { return None; }
    let first_id = &cycle[0];
    let last_id  = cycle.last().unwrap();

    let first_rec = records.iter().find(|r| &r.unit_id == first_id)?;
    let last_rec  = records.iter().find(|r| &r.unit_id == last_id)?;

    let initial_state = first_rec.trajectory.samples.first()?.state.clone();
    let actual_term   = &last_rec.terminus;
    let spec_term     = spec_fn(&initial_state);

    Some(norm_diff(actual_term, &spec_term))
}

/// Find all live holonomy violations across cycles detected in the graph,
/// using a uniform identity spec (zero-drift expectation) as the default.
pub fn live_violations(
    graph:   &SystemGraph,
    records: &[RuntimeRecord],
    tol:     f64,
    max_len: usize,
) -> Vec<HolonomyViolation> {
    // Default spec: identity (the cycle should return to its initial state).
    let identity: &dyn Fn(&[f64]) -> Vec<f64> = &|x: &[f64]| x.to_vec();
    graph.cycles(max_len)
        .into_iter()
        .filter_map(|cycle| {
            let mag = holonomy_dyn(&cycle, records, identity)?;
            if mag > tol {
                Some(HolonomyViolation { cycle, magnitude: mag })
            } else {
                None
            }
        })
        .collect()
}

// ── Aperture drift ────────────────────────────────────────────────────────────

/// A moment where a unit's state left its declared production set.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftEvent {
    pub unit_id: UnitId,
    pub t:       f64,
    pub state:   Vec<f64>,
}

/// Check whether `state` is outside the declared production set.
/// For now: if the production set is non-empty and the first state dimension
/// exceeds the count of declared tags (a simple proxy), flag as drift.
///
/// Real deployments will plug in a domain-specific check via a closure.
pub fn aperture_drift_with<F>(
    record: &RuntimeRecord,
    is_outside: F,
) -> Vec<DriftEvent>
where
    F: Fn(&[f64]) -> bool,
{
    record.trajectory.samples.iter()
        .filter_map(|s| {
            if is_outside(&s.state) {
                Some(DriftEvent {
                    unit_id: record.unit_id.clone(),
                    t:       s.t,
                    state:   s.state.clone(),
                })
            } else {
                None
            }
        })
        .collect()
}

// ── Dynamic report ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicReport {
    pub r_dyn_series:  Vec<(f64, f64)>,
    pub violations:    Vec<HolonomyViolation>,
    pub drift_events:  Vec<DriftEvent>,
    /// Regime classification based on final R_dyn value.
    pub regime_actual: String,
}

impl DynamicReport {
    pub fn compute(
        graph:   &SystemGraph,
        records: &[RuntimeRecord],
        dt:      f64,
        tol:     f64,
        max_len: usize,
    ) -> Self {
        let estimator = ScalarPhaseEstimator;
        let series    = r_dyn_series(records, dt, &estimator);
        let final_r   = series.last().map(|(_, r)| *r).unwrap_or(0.0);
        let regime    = wt_static::Regime::classify(final_r).as_str().to_owned();
        let violations = live_violations(graph, records, tol, max_len);
        Self {
            r_dyn_series:  series,
            violations,
            drift_events:  vec![],
            regime_actual: regime,
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

    fn make_record(id: &str, states: &[(f64, f64)]) -> RuntimeRecord {
        let samples = states.iter()
            .map(|(t, v)| Sample { t: *t, state: vec![*v] })
            .collect();
        let traj = Trajectory { unit_id: UnitId::new(id), samples };
        RuntimeRecord::from_trajectory(traj)
    }

    #[test]
    fn r_dyn_phase_locked() {
        // All units at the same phase → R_dyn = 1.
        let records = vec![
            make_record("u1", &[(0.0, 0.0), (1.0, 0.0)]),
            make_record("u2", &[(0.0, 0.0), (1.0, 0.0)]),
            make_record("u3", &[(0.0, 0.0), (1.0, 0.0)]),
        ];
        let est = ScalarPhaseEstimator;
        let r = r_dyn(&records, 0.5, &est);
        assert!((r - 1.0).abs() < 1e-9, "r_dyn = {r}");
    }

    #[test]
    fn holonomy_zero_for_identity() {
        let records = vec![
            make_record("u1", &[(0.0, 3.0), (1.0, 3.0)]),
            make_record("u2", &[(0.0, 3.0), (1.0, 3.0)]),
            make_record("u3", &[(0.0, 3.0), (1.0, 3.0)]),
        ];
        let cycle = vec![UnitId::new("u1"), UnitId::new("u2"), UnitId::new("u3")];
        let mag = holonomy_dyn(&cycle, &records, &|x: &[f64]| x.to_vec());
        assert!(mag.unwrap() < 1e-9);
    }

    #[test]
    fn holonomy_nonzero_with_drift() {
        // The last unit's terminus is 0.3 away from spec.
        let records = vec![
            make_record("u1", &[(0.0, 0.0)]),
            make_record("u3", &[(0.0, 0.3)]),  // terminus = [0.3]
        ];
        let cycle   = vec![UnitId::new("u1"), UnitId::new("u3")];
        // Spec: identity → expected terminus = [0.0]
        let mag = holonomy_dyn(&cycle, &records, &|_| vec![0.0]);
        assert!((mag.unwrap() - 0.3).abs() < 1e-9);
    }
}
