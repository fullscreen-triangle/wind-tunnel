//! End-to-end tests using a synthetic 3-unit directed cycle.
//!
//! The cycle (u1 → u2 → u3 → u1) is the explicit witness from the proof of
//! the Local Invisibility Theorem.  Each edge applies a drift δ to the scalar
//! state.  After k traversals:
//!
//!   x1^(k) = x1^(0) + 3 * k * δ
//!
//! When δ = 0 the cycle is holonomy-free; the ensemble is Phase-locked.
//! When δ > 0 holonomy accumulates and violations are detected.

use std::collections::HashMap;

use wt_dynamic::{DynamicReport, RuntimeRecord, Sample, Trajectory};
use wt_graph::{Edge, SystemGraph, Unit, UnitId};
use wt_purpose::PurposeReport;
use wt_report::WindTunnelMetric;
use wt_static::{r_est, Regime, StaticReport};

// ── Fixture builders ──────────────────────────────────────────────────────────

fn three_cycle_graph() -> SystemGraph {
    SystemGraph::new(
        vec![
            Unit::new("u1", vec!["scalar".into()], vec!["scalar".into()], 1.0),
            Unit::new("u2", vec!["scalar".into()], vec!["scalar".into()], 1.0),
            Unit::new("u3", vec!["scalar".into()], vec!["scalar".into()], 1.0),
        ],
        vec![
            Edge::new("u1", "u2"),
            Edge::new("u2", "u3"),
            Edge::new("u3", "u1"),
        ],
    )
}

/// Simulate k traversals with per-edge drift δ.
/// Reproduces E05: x1^(k) = 0.3 + 3*k*δ.
fn simulate_cycle(k: usize, delta: f64) -> Vec<RuntimeRecord> {
    let x0 = 0.3_f64;
    let dt = 1.0_f64;

    let mut u1_samples = Vec::new();
    let mut u2_samples = Vec::new();
    let mut u3_samples = Vec::new();

    for step in 0..=k {
        let t    = step as f64 * dt;
        let x1_k = x0 + 3.0 * step as f64 * delta;
        u1_samples.push(Sample { t, state: vec![x1_k] });
        u2_samples.push(Sample { t, state: vec![x1_k + delta] });
        u3_samples.push(Sample { t, state: vec![x1_k + 2.0 * delta] });
    }

    let make = |id: &str, samples: Vec<Sample>| {
        RuntimeRecord::from_trajectory(Trajectory { unit_id: UnitId::new(id), samples })
    };
    vec![make("u1", u1_samples), make("u2", u2_samples), make("u3", u3_samples)]
}

fn ablate(records: &[RuntimeRecord]) -> HashMap<UnitId, Vec<RuntimeRecord>> {
    records.iter().map(|r| {
        let ab = records.iter().filter(|o| o.unit_id != r.unit_id).cloned().collect();
        (r.unit_id.clone(), ab)
    }).collect()
}

// ── Static phase ──────────────────────────────────────────────────────────────

#[test]
fn static_homogeneous_cycle_is_phase_locked() {
    let g = three_cycle_graph();
    let r = r_est(&g);
    assert!((r - 1.0).abs() < 1e-9, "R_est = {r}");
    assert_eq!(Regime::classify(r), Regime::PhaseLocked);
}

#[test]
fn static_type_mismatch_degrades_regime() {
    let g = SystemGraph::new(
        vec![
            Unit::new("u1", vec!["scalar".into()], vec!["scalar".into()], 1.0),
            Unit::new("u2", vec!["scalar".into()], vec!["scalar".into()], 1.0),
            Unit::new("u3", vec!["int".into()],    vec!["int".into()],    1.0),
        ],
        vec![Edge::new("u1","u2"), Edge::new("u2","u3"), Edge::new("u3","u1")],
    );
    let r = r_est(&g);
    assert!(r < 1.0, "type mismatch must lower R_est; got {r}");
    assert_ne!(Regime::classify(r), Regime::PhaseLocked);
}

#[test]
fn static_report_finds_cycle_candidate_on_heterogeneous_cycle() {
    let g = SystemGraph::new(
        vec![
            Unit::new("u1", vec!["scalar".into()], vec!["scalar".into()], 1.0),
            Unit::new("u2", vec!["int".into()],    vec!["int".into()],    5.0),
            Unit::new("u3", vec!["bool".into()],   vec!["bool".into()],   3.0),
        ],
        vec![Edge::new("u1","u2"), Edge::new("u2","u3"), Edge::new("u3","u1")],
    );
    let report = StaticReport::compute(&g, 10, 0.1);
    assert!(!report.cycle_candidates.is_empty(), "high-tension cycle should be a candidate");
}

// ── E05: Local Invisibility witness ──────────────────────────────────────────

#[test]
fn e05_zero_delta_zero_holonomy() {
    // δ = 0 → identity spec satisfied → no violations.
    let g       = three_cycle_graph();
    let records = simulate_cycle(10, 0.0);
    let report  = DynamicReport::compute(&g, &records, 0.5, 1e-6, 10);
    assert!(report.violations.is_empty(), "δ=0 must produce no violations");
}

#[test]
fn e05_nonzero_delta_produces_violation() {
    // δ = 0.05 → after 10 traversals terminus differs from initial by 1.5.
    let g       = three_cycle_graph();
    let records = simulate_cycle(10, 0.05);
    let report  = DynamicReport::compute(&g, &records, 0.5, 1e-6, 10);
    assert!(!report.violations.is_empty(), "δ=0.05 must produce violations");
    assert!(report.violations[0].magnitude > 0.1);
}

#[test]
fn e05_holonomy_grows_with_traversal_count() {
    let g   = three_cycle_graph();
    let r5  = DynamicReport::compute(&g, &simulate_cycle(5,  0.05), 0.5, 1e-6, 10);
    let r20 = DynamicReport::compute(&g, &simulate_cycle(20, 0.05), 0.5, 1e-6, 10);
    let m5  = r5.violations.first().map(|v| v.magnitude).unwrap_or(0.0);
    let m20 = r20.violations.first().map(|v| v.magnitude).unwrap_or(0.0);
    assert!(m20 > m5, "holonomy must grow: k=5→{m5:.4}, k=20→{m20:.4}");
}

#[test]
fn e05_holonomy_magnitude_matches_formula() {
    // After k=10 traversals with δ=0.05:
    // x1^(10) = 0.3 + 3*10*0.05 = 1.8
    // terminus of u3 = 1.8 + 2*0.05 = 1.9
    // identity spec expects 0.3 → holonomy ≈ |1.9 - 0.3| = 1.6
    let g       = three_cycle_graph();
    let records = simulate_cycle(10, 0.05);
    let report  = DynamicReport::compute(&g, &records, 0.5, 1e-6, 10);
    let mag     = report.violations.iter().map(|v| v.magnitude).fold(0.0_f64, f64::max);
    let expected = 1.6_f64;
    assert!(
        (mag - expected).abs() < 0.05,
        "holonomy magnitude: expected ≈{expected:.4}, got {mag:.4}"
    );
}

// ── R_dyn regime ──────────────────────────────────────────────────────────────

#[test]
fn r_dyn_phase_locked_when_synchronised() {
    let records: Vec<RuntimeRecord> = ["u1", "u2", "u3"].iter().map(|id| {
        let samples = (0..=20)
            .map(|k| Sample { t: k as f64 * 0.1, state: vec![0.0] })
            .collect();
        RuntimeRecord::from_trajectory(Trajectory { unit_id: UnitId::new(*id), samples })
    }).collect();
    let g      = three_cycle_graph();
    let report = DynamicReport::compute(&g, &records, 0.1, 1e-6, 10);
    let final_r = report.r_dyn_series.last().map(|(_, r)| *r).unwrap_or(0.0);
    assert_eq!(Regime::classify(final_r), Regime::PhaseLocked,
        "R_dyn = {final_r:.4}");
}

// ── Purpose phase ─────────────────────────────────────────────────────────────

#[test]
fn purposeless_unit_has_zero_contribution() {
    let centre = vec![0.0];
    let make   = |id: &str, val: f64| {
        RuntimeRecord::from_trajectory(Trajectory {
            unit_id: UnitId::new(id),
            samples: vec![Sample { t: 1.0, state: vec![val] }],
        })
    };
    // u3 outputs 0.0; ablating it leaves u1/u2 inside the cell too.
    let full    = vec![make("u1", 0.1), make("u2", 0.2), make("u3", 0.0)];
    let ablated = vec![make("u1", 0.1), make("u2", 0.2)];
    let score = wt_purpose::contribution_score(&full, &ablated, &centre, 0.5, 0.1, 0.7);
    assert!(score.abs() < 1e-9, "score = {score}");
}

#[test]
fn purposeful_unit_has_positive_contribution() {
    let centre = vec![0.0];
    let make   = |id: &str, val: f64| {
        RuntimeRecord::from_trajectory(Trajectory {
            unit_id: UnitId::new(id),
            samples: vec![Sample { t: 1.0, state: vec![val] }],
        })
    };
    // Without u3, u1/u2 are outside the cell.
    let full    = vec![make("u1", 0.1), make("u2", 0.2), make("u3", 0.3)];
    let ablated = vec![make("u1", 2.0), make("u2", 3.0)];
    let score = wt_purpose::contribution_score(&full, &ablated, &centre, 0.5, 0.1, 0.7);
    assert!(score > 0.0, "score = {score}");
}

// ── Full pipeline ─────────────────────────────────────────────────────────────

#[test]
fn pipeline_zero_delta_no_violations_phase_locked() {
    let g             = three_cycle_graph();
    let static_report = StaticReport::compute(&g, 10, 0.1);
    assert_eq!(static_report.regime, Regime::PhaseLocked);
    assert!(static_report.cycle_candidates.is_empty());

    let records        = simulate_cycle(10, 0.0);
    let dynamic_report = DynamicReport::compute(&g, &records, 0.5, 1e-6, 10);
    assert!(dynamic_report.violations.is_empty());

    let purpose_report = PurposeReport::compute(
        &records, &ablate(&records), &[0.3], 0.5, 0.1, 0.7, 0.05,
    );
    let metric  = WindTunnelMetric::from_reports(&static_report, &dynamic_report, &purpose_report);
    let json    = metric.to_json().unwrap();
    let metric2 = WindTunnelMetric::from_json(&json).unwrap();
    assert_eq!(metric2.regime, metric.regime);
    assert!((metric2.r_est - 1.0).abs() < 1e-9);
    assert!(metric2.holonomy_violations.is_empty());
}

#[test]
fn pipeline_large_delta_violations_in_output() {
    let g             = three_cycle_graph();
    let static_report = StaticReport::compute(&g, 10, 0.1);
    let records        = simulate_cycle(10, 0.15);
    let dynamic_report = DynamicReport::compute(&g, &records, 0.5, 1e-6, 10);
    assert!(!dynamic_report.violations.is_empty());

    let purpose_report = PurposeReport::compute(
        &records, &ablate(&records), &[0.3], 0.5, 0.1, 0.7, 0.05,
    );
    let metric = WindTunnelMetric::from_reports(&static_report, &dynamic_report, &purpose_report);
    assert!(!metric.holonomy_violations.is_empty());

    let json = metric.to_json().unwrap();
    let back = WindTunnelMetric::from_json(&json).unwrap();
    assert_eq!(back.holonomy_violations.len(), metric.holonomy_violations.len());
}
