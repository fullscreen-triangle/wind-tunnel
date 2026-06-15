//! Output layer — assembles WindTunnelMetric and renders it.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use wt_static::StaticReport;
use wt_dynamic::DynamicReport;
use wt_purpose::PurposeReport;

// ── Wind Tunnel Metric ────────────────────────────────────────────────────────

/// WT(E, Λ) = (R_dyn, S_flat_est, H, D, δS)
///
/// Definition [Wind Tunnel Metric].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindTunnelMetric {
    /// R_dyn time series from the dynamic phase.
    pub r_dyn_series:          Vec<(f64, f64)>,
    /// Estimated S_flat (OR-success complement: 1 − P_OR).
    pub s_flat_est:            f64,
    /// Live holonomy violations.
    pub holonomy_violations:   Vec<HolonomyEntry>,
    /// Decoherence zones, ranked by severity.
    pub decoherence_zones:     Vec<ZoneEntry>,
    /// Per-unit contribution scores.
    pub contribution_scores:   HashMap<String, f64>,
    /// Purposeless units.
    pub purposeless:           Vec<String>,
    /// Final coordination regime.
    pub regime:                String,
    /// Static R_est (pre-run estimate).
    pub r_est:                 f64,
    /// Estimated critical coupling K_c.
    pub k_c:                   f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HolonomyEntry {
    pub cycle:     Vec<String>,
    pub magnitude: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZoneEntry {
    pub units:    Vec<String>,
    pub r_est:    f64,
    pub severity: f64,
}

impl WindTunnelMetric {
    pub fn from_reports(
        static_report:  &StaticReport,
        dynamic_report: &DynamicReport,
        purpose_report: &PurposeReport,
    ) -> Self {
        let s_flat_est = 1.0 - purpose_report.p_or_full;

        let holonomy_violations = dynamic_report.violations.iter().map(|v| HolonomyEntry {
            cycle:     v.cycle.iter().map(|id| id.0.clone()).collect(),
            magnitude: v.magnitude,
        }).collect();

        let decoherence_zones = static_report.decoherence_zones.iter().map(|z| ZoneEntry {
            units:    z.units.iter().map(|id| id.0.clone()).collect(),
            r_est:    z.r_est,
            severity: z.severity,
        }).collect();

        let regime = dynamic_report.regime_actual.clone();

        WindTunnelMetric {
            r_dyn_series:        dynamic_report.r_dyn_series.clone(),
            s_flat_est,
            holonomy_violations,
            decoherence_zones,
            contribution_scores: purpose_report.scores.clone(),
            purposeless:         purpose_report.purposeless.clone(),
            regime,
            r_est:               static_report.r_est,
            k_c:                 static_report.k_c,
        }
    }

    pub fn to_json(&self) -> anyhow::Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    pub fn from_json(s: &str) -> anyhow::Result<Self> {
        Ok(serde_json::from_str(s)?)
    }

    /// Print a coloured regime map to stdout.
    pub fn print_regime_map(&self) {
        let colour = regime_colour(&self.regime);
        let reset  = "\x1b[0m";

        println!("\n╔══ Wind Tunnel Report ══════════════════════════════╗");
        println!("║  Regime       : {colour}{}{reset}", self.regime);
        println!("║  R_est        : {:.4}", self.r_est);
        println!("║  R_dyn (final): {:.4}", self.r_dyn_series.last().map(|(_, r)| *r).unwrap_or(0.0));
        println!("║  S_flat_est   : {:.4}", self.s_flat_est);
        println!("║  K_c estimate : {:.4}", self.k_c);
        println!("╠══ Decoherence Zones (sorted by severity) ═════════════╣");
        if self.decoherence_zones.is_empty() {
            println!("║  (none)");
        } else {
            for z in &self.decoherence_zones {
                println!("║  {:?}  R_est={:.3}  Δ={:.3}", z.units, z.r_est, z.severity);
            }
        }
        println!("╠══ Holonomy Violations ═════════════════════════════╣");
        if self.holonomy_violations.is_empty() {
            println!("║  (none)");
        } else {
            for v in &self.holonomy_violations {
                println!("║  cycle {:?}  |hol|={:.4}", v.cycle, v.magnitude);
            }
        }
        println!("╠══ Contribution Scores ═════════════════════════════╣");
        let mut scores: Vec<_> = self.contribution_scores.iter().collect();
        scores.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap());
        for (uid, score) in &scores {
            let bar = bar_chart(**score, 20);
            let mark = if self.purposeless.contains(uid) { " [purposeless]" } else { "" };
            println!("║  {uid:12}  {bar}  {score:.4}{mark}");
        }
        println!("╠══ Purposeless Units ═══════════════════════════════╣");
        if self.purposeless.is_empty() {
            println!("║  (none)");
        } else {
            println!("║  {:?}", self.purposeless);
        }
        println!("╚════════════════════════════════════════════════════╝\n");
    }
}

fn regime_colour(regime: &str) -> &'static str {
    match regime {
        r if r.contains("Turbulent")            => "\x1b[31m",  // red
        r if r.contains("Aperture")             => "\x1b[33m",  // yellow
        r if r.contains("Hierarchical")         => "\x1b[93m",  // bright yellow
        r if r.contains("Coherent")             => "\x1b[32m",  // green
        r if r.contains("Phase")                => "\x1b[34m",  // blue
        _                                        => "\x1b[0m",
    }
}

fn bar_chart(value: f64, width: usize) -> String {
    let filled = ((value.clamp(0.0, 1.0)) * width as f64).round() as usize;
    format!("[{}{}]", "█".repeat(filled), "░".repeat(width - filled))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_roundtrip() {
        let metric = WindTunnelMetric {
            r_dyn_series:        vec![(0.0, 0.5), (1.0, 0.8)],
            s_flat_est:          0.1,
            holonomy_violations: vec![],
            decoherence_zones:   vec![],
            contribution_scores: HashMap::from([("u1".into(), 0.3)]),
            purposeless:         vec![],
            regime:              "Coherent".into(),
            r_est:               0.85,
            k_c:                 0.42,
        };
        let json  = metric.to_json().unwrap();
        let back  = WindTunnelMetric::from_json(&json).unwrap();
        assert_eq!(back.regime, "Coherent");
        assert!((back.r_est - 0.85).abs() < 1e-9);
    }
}
