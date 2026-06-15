//! `wt` — Wind Tunnel CLI
//!
//! Usage:
//!   wt static  <project-dir>                [--json] [--cycle-depth N] [--threshold T]
//!   wt dynamic <project-dir> <traces-dir>   [--json] [--dt T] [--tol T]
//!   wt purpose <project-dir> <traces-dir>   [--json] [--alpha A]
//!   wt run     <project-dir> [<traces-dir>] [--json]
//!   wt report  <wt-output.json>

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use anyhow::{bail, Context};

use wt_graph::SystemGraph;
use wt_static::StaticReport;
use wt_dynamic::{DynamicReport, RuntimeRecord};
use wt_purpose::PurposeReport;
use wt_report::WindTunnelMetric;

// ── Defaults ──────────────────────────────────────────────────────────────────

const DEFAULT_CYCLE_DEPTH: usize = 12;
const DEFAULT_THRESHOLD:   f64   = 0.5;
const DEFAULT_TOL:         f64   = 1e-6;
const DEFAULT_DT:          f64   = 0.1;
const DEFAULT_ALPHA:       f64   = 0.05;

// ── Arg parsing (manual, no external deps) ───────────────────────────────────

struct Args {
    subcommand:  String,
    project_dir: Option<PathBuf>,
    traces_dir:  Option<PathBuf>,
    json:        bool,
    cycle_depth: usize,
    threshold:   f64,
    tol:         f64,
    dt:          f64,
    alpha:       f64,
}

impl Args {
    fn parse() -> anyhow::Result<Self> {
        let mut raw: Vec<String> = std::env::args().skip(1).collect();
        if raw.is_empty() {
            print_usage();
            std::process::exit(0);
        }
        let subcommand = raw.remove(0);

        let mut json        = false;
        let mut cycle_depth = DEFAULT_CYCLE_DEPTH;
        let mut threshold   = DEFAULT_THRESHOLD;
        let mut tol         = DEFAULT_TOL;
        let mut dt          = DEFAULT_DT;
        let mut alpha       = DEFAULT_ALPHA;
        let mut positionals: Vec<PathBuf> = Vec::new();

        let mut i = 0;
        while i < raw.len() {
            match raw[i].as_str() {
                "--json"        => { json = true; }
                "--cycle-depth" => { i += 1; cycle_depth = raw[i].parse()?; }
                "--threshold"   => { i += 1; threshold   = raw[i].parse()?; }
                "--tol"         => { i += 1; tol         = raw[i].parse()?; }
                "--dt"          => { i += 1; dt          = raw[i].parse()?; }
                "--alpha"       => { i += 1; alpha       = raw[i].parse()?; }
                other           => positionals.push(PathBuf::from(other)),
            }
            i += 1;
        }

        let project_dir = positionals.first().cloned();
        let traces_dir  = positionals.get(1).cloned();

        Ok(Args { subcommand, project_dir, traces_dir, json, cycle_depth, threshold, tol, dt, alpha })
    }
}

fn print_usage() {
    eprintln!(
        "wt — Wind Tunnel

Usage:
  wt static  <project-dir>               [--json] [--cycle-depth N] [--threshold T]
  wt dynamic <project-dir> <traces-dir>  [--json] [--dt T] [--tol T]
  wt purpose <project-dir> <traces-dir>  [--json] [--alpha A]
  wt run     <project-dir> [<traces-dir>][--json]
  wt report  <wt-output.json>            [--json]
"
    );
}

// ── Graph loading ─────────────────────────────────────────────────────────────

fn load_graph(project_dir: &Path) -> anyhow::Result<SystemGraph> {
    // Try to read a cached graph.json first, then fall back to purpose bridge.
    let graph_path = project_dir.join("wt-graph.json");
    if graph_path.exists() {
        let raw = std::fs::read_to_string(&graph_path)?;
        return SystemGraph::from_json(&raw).context("parsing wt-graph.json");
    }

    // Fall back to purpose bridge.
    let purpose_exe = which_purpose()?;
    eprintln!("Building graph via `purpose` (this may take a moment)…");
    wt_graph::purpose::graph_from_project(&purpose_exe, project_dir, true)
}

fn which_purpose() -> anyhow::Result<PathBuf> {
    // Check common install location first.
    let candidate = PathBuf::from(r"C:\Users\kunda\.cargo\bin\purpose.exe");
    if candidate.exists() { return Ok(candidate); }
    // Walk PATH.
    if let Ok(p) = which_on_path("purpose") { return Ok(p); }
    bail!("Could not find `purpose` binary. Install it or add it to PATH.");
}

fn which_on_path(name: &str) -> anyhow::Result<PathBuf> {
    let path_var = std::env::var("PATH").unwrap_or_default();
    for dir in std::env::split_paths(&path_var) {
        let candidate = dir.join(name);
        if candidate.exists() { return Ok(candidate); }
        let with_exe = dir.join(format!("{name}.exe"));
        if with_exe.exists() { return Ok(with_exe); }
    }
    bail!("not found on PATH");
}

// ── Subcommands ───────────────────────────────────────────────────────────────

fn cmd_static(args: &Args) -> anyhow::Result<()> {
    let project_dir = args.project_dir.as_ref()
        .ok_or_else(|| anyhow::anyhow!("static requires <project-dir>"))?;
    let graph  = load_graph(project_dir)?;
    let report = StaticReport::compute(&graph, args.cycle_depth, args.threshold);
    if args.json {
        println!("{}", report.to_json()?);
    } else {
        println!("Regime        : {}", report.regime.as_str());
        println!("R_est         : {:.4}", report.r_est);
        println!("K_c estimate  : {:.4}", report.k_c);
        println!("Decoherence zones: {}", report.decoherence_zones.len());
        println!("Cycle candidates : {}", report.cycle_candidates.len());
        for (cycle, res) in &report.cycle_candidates {
            let names: Vec<_> = cycle.iter().map(|id| id.as_str()).collect();
            println!("  {:?}  residual={res:.4}", names);
        }
    }
    Ok(())
}

fn cmd_dynamic(args: &Args) -> anyhow::Result<()> {
    let project_dir = args.project_dir.as_ref()
        .ok_or_else(|| anyhow::anyhow!("dynamic requires <project-dir>"))?;
    let traces_dir  = args.traces_dir.as_ref()
        .ok_or_else(|| anyhow::anyhow!("dynamic requires <traces-dir>"))?;
    let graph   = load_graph(project_dir)?;
    let records = wt_dynamic::load_traces(traces_dir)?;
    let report  = DynamicReport::compute(&graph, &records, args.dt, args.tol, args.cycle_depth);
    if args.json {
        println!("{}", report.to_json()?);
    } else {
        println!("Regime (actual)   : {}", report.regime_actual);
        println!("R_dyn (final)     : {:.4}", report.r_dyn_series.last().map(|(_, r)| *r).unwrap_or(0.0));
        println!("Holonomy violations: {}", report.violations.len());
        println!("Aperture drift     : {}", report.drift_events.len());
    }
    Ok(())
}

fn cmd_purpose(args: &Args) -> anyhow::Result<()> {
    let _project_dir = args.project_dir.as_ref()
        .ok_or_else(|| anyhow::anyhow!("purpose requires <project-dir>"))?;
    let traces_dir  = args.traces_dir.as_ref()
        .ok_or_else(|| anyhow::anyhow!("purpose requires <traces-dir>"))?;
    let records: Vec<RuntimeRecord> = wt_dynamic::load_traces(traces_dir)?;

    // Action-cell parameters: use a unit sphere centred at the mean terminus
    // as a default when not specified by the user.
    let centre: Vec<f64> = mean_terminus(&records);
    let radius = 1.0;
    let beta   = 0.1;
    let s_threshold = beta + 1.0; // states within radius+1 are "successful"

    // Ablated ensembles: for each unit, drop its record.
    let ablated: HashMap<wt_graph::UnitId, Vec<RuntimeRecord>> = records.iter()
        .map(|r| {
            let ablated: Vec<_> = records.iter()
                .filter(|other| other.unit_id != r.unit_id)
                .cloned()
                .collect();
            (r.unit_id.clone(), ablated)
        })
        .collect();

    let report = PurposeReport::compute(
        &records, &ablated, &centre, radius, beta, s_threshold, args.alpha,
    );
    if args.json {
        println!("{}", report.to_json()?);
    } else {
        println!("P_OR (full)   : {:.4}", report.p_or_full);
        println!("Purposeless   : {:?}", report.purposeless);
        let mut scores: Vec<_> = report.scores.iter().collect();
        scores.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap());
        for (uid, score) in scores {
            println!("  {uid:20}  δS = {score:.4}");
        }
    }
    Ok(())
}

fn cmd_run(args: &Args) -> anyhow::Result<()> {
    let project_dir = args.project_dir.as_ref()
        .ok_or_else(|| anyhow::anyhow!("run requires <project-dir>"))?;

    eprintln!("=== Phase 1: static analysis ===");
    let graph         = load_graph(project_dir)?;
    let static_report = StaticReport::compute(&graph, args.cycle_depth, args.threshold);
    eprintln!("  Regime: {}   R_est={:.4}", static_report.regime.as_str(), static_report.r_est);

    let (dynamic_report, purpose_report) = if let Some(traces_dir) = &args.traces_dir {
        eprintln!("=== Phase 2: dynamic analysis ===");
        let records = wt_dynamic::load_traces(traces_dir)?;
        let dr = DynamicReport::compute(&graph, &records, args.dt, args.tol, args.cycle_depth);
        eprintln!("  R_dyn={:.4}  violations={}", dr.r_dyn_series.last().map(|(_, r)| *r).unwrap_or(0.0), dr.violations.len());

        eprintln!("=== Phase 3: purposelessness ===");
        let centre = mean_terminus(&records);
        let radius = 1.0; let beta = 0.1; let s_thr = beta + 1.0;
        let ablated: HashMap<wt_graph::UnitId, Vec<RuntimeRecord>> = records.iter()
            .map(|r| {
                let ab: Vec<_> = records.iter().filter(|o| o.unit_id != r.unit_id).cloned().collect();
                (r.unit_id.clone(), ab)
            }).collect();
        let pr = PurposeReport::compute(&records, &ablated, &centre, radius, beta, s_thr, args.alpha);
        (dr, pr)
    } else {
        eprintln!("  (no traces-dir — skipping Phases 2 & 3)");
        let dr = DynamicReport {
            r_dyn_series:  vec![],
            violations:    vec![],
            drift_events:  vec![],
            regime_actual: static_report.regime.as_str().to_owned(),
        };
        let pr = PurposeReport {
            p_or_full:  0.0,
            scores:     HashMap::new(),
            purposeless: vec![],
        };
        (dr, pr)
    };

    let metric = WindTunnelMetric::from_reports(&static_report, &dynamic_report, &purpose_report);

    if args.json {
        println!("{}", metric.to_json()?);
    } else {
        metric.print_regime_map();
    }

    // Write wt-output.json.
    let out_path = project_dir.join("wt-output.json");
    std::fs::write(&out_path, metric.to_json()?)?;
    eprintln!("Result written to {}", out_path.display());

    // Exit code: 0 = Coherent or Phase-locked, 1 = below Coherent.
    let final_r = dynamic_report.r_dyn_series.last().map(|(_, r)| *r).unwrap_or(static_report.r_est);
    if final_r < 0.80 { std::process::exit(1); }
    Ok(())
}

fn cmd_report(args: &Args) -> anyhow::Result<()> {
    let path = args.project_dir.as_ref()
        .ok_or_else(|| anyhow::anyhow!("report requires <wt-output.json>"))?;
    let raw    = std::fs::read_to_string(path)?;
    let metric = WindTunnelMetric::from_json(&raw)?;
    if args.json {
        println!("{}", metric.to_json()?);
    } else {
        metric.print_regime_map();
    }
    Ok(())
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn mean_terminus(records: &[RuntimeRecord]) -> Vec<f64> {
    if records.is_empty() { return vec![0.0]; }
    let dim = records[0].terminus.len().max(1);
    let mut acc = vec![0.0_f64; dim];
    for r in records {
        for (i, v) in r.terminus.iter().enumerate().take(dim) {
            acc[i] += v;
        }
    }
    let n = records.len() as f64;
    acc.iter_mut().for_each(|v| *v /= n);
    acc
}

// ── Entry point ───────────────────────────────────────────────────────────────

fn main() {
    let args = match Args::parse() {
        Ok(a)  => a,
        Err(e) => { eprintln!("Error: {e}"); print_usage(); std::process::exit(2); }
    };

    let result = match args.subcommand.as_str() {
        "static"  => cmd_static(&args),
        "dynamic" => cmd_dynamic(&args),
        "purpose" => cmd_purpose(&args),
        "run"     => cmd_run(&args),
        "report"  => cmd_report(&args),
        other     => { eprintln!("Unknown subcommand: {other}"); print_usage(); std::process::exit(2); }
    };

    if let Err(e) = result {
        eprintln!("Error: {e:#}");
        std::process::exit(2);
    }
}
