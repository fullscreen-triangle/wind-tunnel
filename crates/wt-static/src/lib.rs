//! Phase 1 of the Wind Tunnel protocol — static analysis.
//!
//! Inputs:  a `SystemGraph` (no running system required).
//! Outputs: `StaticReport` containing regime, decoherence zones,
//!           K_c estimate, and holonomy-candidate cycles.

use std::collections::HashSet;
use std::f64::consts::PI;

use serde::{Deserialize, Serialize};
use wt_graph::{SystemGraph, UnitId};

// ── Regime ────────────────────────────────────────────────────────────────────

/// Five coordination regimes (Definition [Coordination Regimes]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Regime {
    /// R_ens < 0.3
    Turbulent,
    /// 0.3 ≤ R_ens < 0.5
    ApertureDominated,
    /// 0.5 ≤ R_ens < 0.8
    HierarchicalCascade,
    /// 0.8 ≤ R_ens < 0.95
    Coherent,
    /// R_ens ≥ 0.95
    PhaseLocked,
}

impl Regime {
    pub fn classify(r: f64) -> Self {
        if r < 0.30 { Regime::Turbulent }
        else if r < 0.50 { Regime::ApertureDominated }
        else if r < 0.80 { Regime::HierarchicalCascade }
        else if r < 0.95 { Regime::Coherent }
        else              { Regime::PhaseLocked }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Regime::Turbulent           => "Turbulent",
            Regime::ApertureDominated   => "Aperture-dominated",
            Regime::HierarchicalCascade => "Hierarchical cascade",
            Regime::Coherent            => "Coherent",
            Regime::PhaseLocked         => "Phase-locked",
        }
    }
}

// ── Aperture gap ──────────────────────────────────────────────────────────────

/// d_X(P_{u_i}, A_{u_j}): 0.0 if the sets share at least one type tag;
/// otherwise Jaccard complement (fraction of tags not in common).
///
/// Corresponds to the aperture-gap component of Definition [Sync Tension].
pub fn aperture_gap(production: &[String], aperture: &[String]) -> f64 {
    if production.is_empty() || aperture.is_empty() {
        // No type information — conservatively assume full gap.
        return 1.0;
    }
    let p_set: HashSet<&str> = production.iter().map(String::as_str).collect();
    let a_set: HashSet<&str> = aperture.iter().map(String::as_str).collect();
    let intersection = p_set.intersection(&a_set).count();
    if intersection > 0 {
        return 0.0;
    }
    let union = p_set.union(&a_set).count();
    // Jaccard distance: 1 - |P ∩ A| / |P ∪ A|
    1.0 - (intersection as f64 / union as f64)
}

// ── Synchronisation tension ───────────────────────────────────────────────────

/// ϑ(u_i, u_j) = d_X(P_{u_i}, A_{u_j}) + |ω_i - ω_j|
///
/// Definition [Synchronisation Tension].
pub fn tension(graph: &SystemGraph, from: &UnitId, to: &UnitId) -> f64 {
    let ui = graph.unit(from);
    let uj = graph.unit(to);
    match (ui, uj) {
        (Some(ui), Some(uj)) => {
            let gap  = aperture_gap(ui.production.tags(), uj.aperture.tags());
            let freq = (ui.freq_est - uj.freq_est).abs();
            gap + freq
        }
        _ => 1.0, // unknown units: maximum gap
    }
}

// ── Static order parameter ────────────────────────────────────────────────────

/// R_est = exp(−mean tension across all edges).
///
/// Definition [Static Order Parameter Estimate].
pub fn r_est(graph: &SystemGraph) -> f64 {
    if graph.edges.is_empty() {
        return 1.0; // no edges = no tension
    }
    let mean_tension: f64 = graph.edges.iter()
        .map(|e| tension(graph, &e.from, &e.to))
        .sum::<f64>()
        / graph.edges.len() as f64;
    (-mean_tension).exp()
}

// ── Critical coupling estimate ────────────────────────────────────────────────

/// K_c = 2 σ_ω / π  (Proposition [Critical Coupling]).
pub fn k_c_estimate(graph: &SystemGraph) -> f64 {
    if graph.units.is_empty() { return 0.0; }
    let freqs: Vec<f64> = graph.units.iter().map(|u| u.freq_est).collect();
    let n = freqs.len() as f64;
    let mean = freqs.iter().sum::<f64>() / n;
    let variance = freqs.iter().map(|f| (f - mean).powi(2)).sum::<f64>() / n;
    let sigma = variance.sqrt();
    2.0 * sigma / PI
}

// ── Decoherence zones ─────────────────────────────────────────────────────────

/// A connected subgraph whose R_est is below the global R_est.
/// Corresponds to Definition [Decoherence Zone].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecoherenceZone {
    pub units:    Vec<UnitId>,
    pub r_est:    f64,
    /// How much worse than global (global_r_est − zone_r_est).
    pub severity: f64,
}

/// Find all maximal connected subgraphs with R_est < global R_est,
/// ranked by severity (largest deficit first).
pub fn decoherence_zones(graph: &SystemGraph) -> Vec<DecoherenceZone> {
    let global = r_est(graph);
    let mut zones: Vec<DecoherenceZone> = graph
        .connected_components()
        .into_iter()
        .filter_map(|comp| {
            if comp.len() == graph.units.len() { return None; } // skip the whole graph
            let sub  = graph.subgraph(&comp);
            let sub_r = r_est(&sub);
            if sub_r < global {
                Some(DecoherenceZone {
                    units:    comp,
                    r_est:    sub_r,
                    severity: global - sub_r,
                })
            } else {
                None
            }
        })
        .collect();
    zones.sort_by(|a, b| b.severity.partial_cmp(&a.severity).unwrap());
    zones
}

// ── Cycle residual candidates ─────────────────────────────────────────────────

/// Sum of per-edge tensions around a cycle — a static proxy for holonomy.
pub fn static_residual(cycle: &[UnitId], graph: &SystemGraph) -> f64 {
    if cycle.len() < 2 { return 0.0; }
    cycle.windows(2)
        .map(|w| tension(graph, &w[0], &w[1]))
        .sum::<f64>()
        // closing edge: last → first
        + tension(graph, cycle.last().unwrap(), &cycle[0])
}

/// Cycles whose static residual exceeds `threshold` — likely holonomy violations.
pub fn cycle_candidates(
    graph:     &SystemGraph,
    max_len:   usize,
    threshold: f64,
) -> Vec<(Vec<UnitId>, f64)> {
    graph.cycles(max_len)
        .into_iter()
        .filter_map(|cycle| {
            let r = static_residual(&cycle, graph);
            if r > threshold { Some((cycle, r)) } else { None }
        })
        .collect()
}

// ── Static report ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaticReport {
    pub r_est:             f64,
    pub regime:            Regime,
    pub k_c:               f64,
    pub decoherence_zones: Vec<DecoherenceZone>,
    /// (cycle, residual) pairs exceeding the detection threshold.
    pub cycle_candidates:  Vec<(Vec<UnitId>, f64)>,
}

impl StaticReport {
    pub fn compute(graph: &SystemGraph, cycle_max_len: usize, residual_threshold: f64) -> Self {
        let r = r_est(graph);
        Self {
            r_est:             r,
            regime:            Regime::classify(r),
            k_c:               k_c_estimate(graph),
            decoherence_zones: decoherence_zones(graph),
            cycle_candidates:  cycle_candidates(graph, cycle_max_len, residual_threshold),
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
    use wt_graph::{Edge, Unit, SystemGraph};

    fn make_graph(units: Vec<(&str, Vec<&str>, Vec<&str>, f64)>, edges: Vec<(&str, &str)>) -> SystemGraph {
        SystemGraph::new(
            units.into_iter().map(|(id, ap, pr, w)| Unit::new(
                id,
                ap.into_iter().map(str::to_owned).collect(),
                pr.into_iter().map(str::to_owned).collect(),
                w,
            )).collect(),
            edges.into_iter().map(|(f, t)| Edge::new(f, t)).collect(),
        )
    }

    #[test]
    fn aperture_gap_zero_when_overlap() {
        let gap = aperture_gap(&["int".into(), "str".into()], &["str".into(), "bool".into()]);
        assert_eq!(gap, 0.0);
    }

    #[test]
    fn aperture_gap_nonzero_when_disjoint() {
        let gap = aperture_gap(&["int".into()], &["str".into()]);
        assert!(gap > 0.0);
    }

    #[test]
    fn r_est_perfect_homogeneous() {
        // Identical units, same type tags → tension = 0 → R_est = 1.
        let g = make_graph(
            vec![
                ("u1", vec!["T"], vec!["T"], 1.0),
                ("u2", vec!["T"], vec!["T"], 1.0),
            ],
            vec![("u1", "u2")],
        );
        let r = r_est(&g);
        assert!((r - 1.0).abs() < 1e-9, "r_est = {r}");
    }

    #[test]
    fn r_est_decreases_with_tension() {
        let g_low = make_graph(
            vec![("u1", vec!["T"], vec!["T"], 1.0), ("u2", vec!["T"], vec!["T"], 1.0)],
            vec![("u1", "u2")],
        );
        let g_high = make_graph(
            vec![("u1", vec!["A"], vec!["A"], 1.0), ("u2", vec!["B"], vec!["B"], 5.0)],
            vec![("u1", "u2")],
        );
        assert!(r_est(&g_low) > r_est(&g_high));
    }

    #[test]
    fn regime_classification() {
        assert_eq!(Regime::classify(0.10), Regime::Turbulent);
        assert_eq!(Regime::classify(0.40), Regime::ApertureDominated);
        assert_eq!(Regime::classify(0.65), Regime::HierarchicalCascade);
        assert_eq!(Regime::classify(0.90), Regime::Coherent);
        assert_eq!(Regime::classify(0.97), Regime::PhaseLocked);
    }

    #[test]
    fn k_c_homogeneous_is_zero() {
        let g = make_graph(
            vec![("u1", vec![], vec![], 2.0), ("u2", vec![], vec![], 2.0)],
            vec![],
        );
        assert!(k_c_estimate(&g) < 1e-9);
    }

    #[test]
    fn static_report_serialises() {
        let g = make_graph(
            vec![
                ("u1", vec!["T"], vec!["T"], 1.0),
                ("u2", vec!["T"], vec!["T"], 1.0),
                ("u3", vec!["T"], vec!["T"], 1.0),
            ],
            vec![("u1", "u2"), ("u2", "u3"), ("u3", "u1")],
        );
        let report = StaticReport::compute(&g, 10, 0.1);
        let json = report.to_json().unwrap();
        assert!(json.contains("r_est"));
    }
}
