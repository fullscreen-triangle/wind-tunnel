//! `wt` — Wind Tunnel CLI
//!
//! When called with no project-dir argument, the current working directory
//! is used as the project root.  This lets you run `wt run` from inside any
//! repository without specifying a path.
//!
//! Usage:
//!   wt [static]  [<project-dir>]               [--json] [--cycle-depth N] [--threshold T]
//!   wt dynamic   [<project-dir>] <traces-dir>   [--json] [--dt T] [--tol T]
//!   wt purpose   [<project-dir>] <traces-dir>   [--json] [--alpha A]
//!   wt run       [<project-dir>] [<traces-dir>]  [--json]
//!   wt report    [<wt-output.json>]
//!   wt init      [<project-dir>]                 — (re)build the purpose index only
//!   wt version

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
const VERSION: &str = env!("CARGO_PKG_VERSION");

// ── Arg parsing ───────────────────────────────────────────────────────────────

struct Args {
    subcommand:  String,
    project_dir: PathBuf,
    traces_dir:  Option<PathBuf>,
    report_file: Option<PathBuf>,
    json:        bool,
    cycle_depth: usize,
    threshold:   f64,
    tol:         f64,
    dt:          f64,
    alpha:       f64,
    no_cache:    bool,
}

impl Args {
    fn parse() -> anyhow::Result<Self> {
        let mut raw: Vec<String> = std::env::args().skip(1).collect();

        // No arguments: default to `wt static` in CWD.
        if raw.is_empty() {
            raw.push("static".to_owned());
        }

        // Treat bare `wt .` or `wt /some/path` (no subcommand keyword) as `wt run <path>`.
        let first = &raw[0];
        let known_subcommands = ["static","dynamic","purpose","run","report","init","version","help"];
        if !known_subcommands.contains(&first.as_str()) {
            raw.insert(0, "run".to_owned());
        }

        let subcommand = raw.remove(0);

        if subcommand == "help" || subcommand == "--help" || subcommand == "-h" {
            print_usage();
            std::process::exit(0);
        }
        if subcommand == "version" || subcommand == "--version" {
            println!("wt {VERSION}");
            std::process::exit(0);
        }

        let mut json        = false;
        let mut no_cache    = false;
        let mut cycle_depth = DEFAULT_CYCLE_DEPTH;
        let mut threshold   = DEFAULT_THRESHOLD;
        let mut tol         = DEFAULT_TOL;
        let mut dt          = DEFAULT_DT;
        let mut alpha       = DEFAULT_ALPHA;
        let mut positionals: Vec<PathBuf> = Vec::new();

        let mut i = 0;
        while i < raw.len() {
            match raw[i].as_str() {
                "--json"        => json       = true,
                "--no-cache"    => no_cache   = true,
                "--cycle-depth" => { i += 1; cycle_depth = raw[i].parse()?; }
                "--threshold"   => { i += 1; threshold   = raw[i].parse()?; }
                "--tol"         => { i += 1; tol         = raw[i].parse()?; }
                "--dt"          => { i += 1; dt          = raw[i].parse()?; }
                "--alpha"       => { i += 1; alpha       = raw[i].parse()?; }
                other           => positionals.push(PathBuf::from(other)),
            }
            i += 1;
        }

        // Resolve project_dir: first positional if it looks like a directory,
        // otherwise CWD.  For `report`, first positional is a JSON file.
        let (project_dir, traces_dir, report_file) = match subcommand.as_str() {
            "report" => {
                let f = positionals.into_iter().next();
                (std::env::current_dir()?, None, f)
            }
            "dynamic" | "purpose" => {
                // project-dir (optional), traces-dir (required)
                let (pd, td) = resolve_project_and_traces(&positionals)?;
                (pd, Some(td), None)
            }
            _ => {
                let pd = positionals.first()
                    .map(|p| canonicalise(p))
                    .transpose()?
                    .unwrap_or_else(|| std::env::current_dir().unwrap());
                let td = positionals.get(1).cloned();
                (pd, td, None)
            }
        };

        Ok(Args { subcommand, project_dir, traces_dir, report_file,
                  json, no_cache, cycle_depth, threshold, tol, dt, alpha })
    }
}

fn canonicalise(p: &Path) -> anyhow::Result<PathBuf> {
    p.canonicalize().with_context(|| format!("path not found: {}", p.display()))
}

/// For commands that take an optional project-dir followed by a required
/// traces-dir: if only one positional is given and it is an existing directory,
/// treat it as the traces-dir and use CWD as the project-dir.
fn resolve_project_and_traces(positionals: &[PathBuf]) -> anyhow::Result<(PathBuf, PathBuf)> {
    match positionals.len() {
        0 => bail!("traces-dir is required"),
        1 => {
            let cwd = std::env::current_dir()?;
            let td  = canonicalise(&positionals[0])?;
            Ok((cwd, td))
        }
        _ => {
            let pd = canonicalise(&positionals[0])?;
            let td = canonicalise(&positionals[1])?;
            Ok((pd, td))
        }
    }
}

fn print_usage() {
    eprintln!(
"wt {VERSION} — Wind Tunnel: global self-consistency analysis for software

Usage (all <project-dir> default to the current directory):

  wt                                  static analysis of the current project
  wt [static]  [<project-dir>]        static analysis (regime map, cycle candidates)
  wt dynamic   [<project-dir>] <traces-dir>   dynamic analysis from runtime traces
  wt purpose   [<project-dir>] <traces-dir>   contribution scores, purposeless units
  wt run       [<project-dir>] [<traces-dir>] full protocol (phases 1–3)
  wt report    [<wt-output.json>]     re-render a saved result
  wt init      [<project-dir>]        (re)build the purpose index

Flags:
  --json             emit JSON instead of human-readable output
  --no-cache         ignore cached wt-graph.json, rebuild from purpose
  --cycle-depth N    max cycle length for Johnson's algorithm (default: {DEFAULT_CYCLE_DEPTH})
  --threshold T      static residual threshold for cycle candidates   (default: {DEFAULT_THRESHOLD})
  --tol T            holonomy violation tolerance                       (default: {DEFAULT_TOL})
  --dt T             R_dyn time-series step                             (default: {DEFAULT_DT})
  --alpha A          purposelessness significance level                 (default: {DEFAULT_ALPHA})

Output files (written to <project-dir>/.wt/):
  graph.json         cached dependency graph
  output.json        full WindTunnelMetric for the last run
"
    );
}

// ── Purpose binary location ───────────────────────────────────────────────────

fn which_purpose() -> anyhow::Result<PathBuf> {
    // 1. CARGO_HOME / bin / purpose[.exe]
    if let Ok(home) = std::env::var("CARGO_HOME") {
        for name in ["purpose", "purpose.exe"] {
            let p = PathBuf::from(&home).join("bin").join(name);
            if p.exists() { return Ok(p); }
        }
    }
    // 2. ~/.cargo/bin/purpose[.exe]
    if let Some(home) = dirs_home() {
        for name in ["purpose", "purpose.exe"] {
            let p = home.join(".cargo").join("bin").join(name);
            if p.exists() { return Ok(p); }
        }
    }
    // 3. PATH
    let path_var = std::env::var("PATH").unwrap_or_default();
    for dir in std::env::split_paths(&path_var) {
        for name in ["purpose", "purpose.exe"] {
            let p = dir.join(name);
            if p.exists() { return Ok(p); }
        }
    }
    bail!(
        "`purpose` not found. Install it with:\n  \
         cargo install --git https://github.com/fullscreen-triangle/purpose"
    );
}

fn dirs_home() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
}

// ── .wt/ output directory ────────────────────────────────────────────────────

/// Returns (and creates) the `.wt/` directory inside the project root.
fn wt_dir(project_dir: &Path) -> anyhow::Result<PathBuf> {
    let d = project_dir.join(".wt");
    std::fs::create_dir_all(&d)
        .with_context(|| format!("creating {}", d.display()))?;
    Ok(d)
}

// ── Graph loading ─────────────────────────────────────────────────────────────

fn load_graph(project_dir: &Path, no_cache: bool) -> anyhow::Result<SystemGraph> {
    let cache = wt_dir(project_dir)?.join("graph.json");

    if !no_cache && cache.exists() {
        eprintln!("  Using cached graph ({}).", cache.display());
        let raw = std::fs::read_to_string(&cache)?;
        return SystemGraph::from_json(&raw).context("parsing .wt/graph.json");
    }

    let purpose_exe = which_purpose()?;
    eprintln!("  Indexing project with `purpose`…");
    let graph = wt_graph::purpose::graph_from_project(&purpose_exe, project_dir, true)?;

    // Cache for subsequent calls.
    std::fs::write(&cache, graph.to_json()?)
        .with_context(|| format!("writing {}", cache.display()))?;
    eprintln!("  Graph cached at {}.", cache.display());
    Ok(graph)
}

// ── Subcommands ───────────────────────────────────────────────────────────────

fn cmd_init(args: &Args) -> anyhow::Result<()> {
    let purpose_exe = which_purpose()?;
    eprintln!("Indexing {} …", args.project_dir.display());
    let graph = wt_graph::purpose::graph_from_project(&purpose_exe, &args.project_dir, true)?;
    let cache = wt_dir(&args.project_dir)?.join("graph.json");
    std::fs::write(&cache, graph.to_json()?)?;
    eprintln!("Index built: {} units, {} edges.", graph.units.len(), graph.edges.len());
    eprintln!("Cached at {}.", cache.display());
    Ok(())
}

fn cmd_static(args: &Args) -> anyhow::Result<()> {
    let graph  = load_graph(&args.project_dir, args.no_cache)?;
    let report = StaticReport::compute(&graph, args.cycle_depth, args.threshold);

    if args.json {
        println!("{}", report.to_json()?);
        return Ok(());
    }

    let col = regime_colour(report.regime.as_str());
    let rst = "\x1b[0m";
    println!("\nProject : {}", args.project_dir.display());
    println!("Units   : {}   Edges: {}", graph.units.len(), graph.edges.len());
    println!("Regime  : {col}{}{rst}", report.regime.as_str());
    println!("R_est   : {:.4}", report.r_est);
    println!("K_c     : {:.4}", report.k_c);
    if report.decoherence_zones.is_empty() {
        println!("Zones   : (none)");
    } else {
        println!("Zones   :");
        for z in &report.decoherence_zones {
            let ids: Vec<_> = z.units.iter().map(|id| id.as_str()).collect();
            println!("  R_est={:.4}  Δ={:.4}  {:?}", z.r_est, z.severity, ids);
        }
    }
    if !report.cycle_candidates.is_empty() {
        println!("Cycles  :");
        for (cycle, res) in &report.cycle_candidates {
            let names: Vec<_> = cycle.iter().map(|id| id.as_str()).collect();
            println!("  residual={res:.4}  {:?}", names);
        }
    }
    Ok(())
}

fn cmd_dynamic(args: &Args) -> anyhow::Result<()> {
    let traces_dir = args.traces_dir.as_ref()
        .ok_or_else(|| anyhow::anyhow!("dynamic requires <traces-dir>"))?;
    let graph   = load_graph(&args.project_dir, args.no_cache)?;
    let records = wt_dynamic::load_traces(traces_dir)?;
    let report  = DynamicReport::compute(&graph, &records, args.dt, args.tol, args.cycle_depth);

    if args.json {
        println!("{}", report.to_json()?);
        return Ok(());
    }

    let final_r = report.r_dyn_series.last().map(|(_, r)| *r).unwrap_or(0.0);
    let col = regime_colour(&report.regime_actual);
    let rst = "\x1b[0m";
    println!("Regime (actual)    : {col}{}{rst}", report.regime_actual);
    println!("R_dyn (final)      : {:.4}", final_r);
    println!("Holonomy violations: {}", report.violations.len());
    for v in &report.violations {
        let names: Vec<_> = v.cycle.iter().map(|id| id.as_str()).collect();
        println!("  |hol|={:.4}  {:?}", v.magnitude, names);
    }
    println!("Aperture drift     : {}", report.drift_events.len());
    Ok(())
}

fn cmd_purpose(args: &Args) -> anyhow::Result<()> {
    let traces_dir = args.traces_dir.as_ref()
        .ok_or_else(|| anyhow::anyhow!("purpose requires <traces-dir>"))?;
    let records: Vec<RuntimeRecord> = wt_dynamic::load_traces(traces_dir)?;
    let centre      = mean_terminus(&records);
    let radius      = 1.0;
    let beta        = 0.1;
    let s_threshold = beta + 1.0;
    let ablated     = ablate_map(&records);
    let report = PurposeReport::compute(
        &records, &ablated, &centre, radius, beta, s_threshold, args.alpha,
    );

    if args.json {
        println!("{}", report.to_json()?);
        return Ok(());
    }

    println!("P_OR (full)  : {:.4}", report.p_or_full);
    let mut scores: Vec<_> = report.scores.iter().collect();
    scores.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap());
    for (uid, score) in &scores {
        let mark = if report.purposeless.contains(uid) { "  [purposeless]" } else { "" };
        println!("  {uid:30}  δS={score:.4}{mark}");
    }
    Ok(())
}

fn cmd_run(args: &Args) -> anyhow::Result<()> {
    eprintln!("\n=== Wind Tunnel: {} ===\n", args.project_dir.display());

    eprintln!("[1/3] Static analysis");
    let graph         = load_graph(&args.project_dir, args.no_cache)?;
    let static_report = StaticReport::compute(&graph, args.cycle_depth, args.threshold);
    eprintln!("      Regime: {}   R_est={:.4}   units={}   edges={}",
        static_report.regime.as_str(), static_report.r_est,
        graph.units.len(), graph.edges.len());

    let (dynamic_report, purpose_report) = if let Some(traces_dir) = &args.traces_dir {
        eprintln!("[2/3] Dynamic analysis");
        let records   = wt_dynamic::load_traces(traces_dir)?;
        let dr        = DynamicReport::compute(&graph, &records, args.dt, args.tol, args.cycle_depth);
        let final_r   = dr.r_dyn_series.last().map(|(_, r)| *r).unwrap_or(0.0);
        eprintln!("      R_dyn={:.4}   violations={}", final_r, dr.violations.len());

        eprintln!("[3/3] Purposelessness");
        let centre  = mean_terminus(&records);
        let ablated = ablate_map(&records);
        let pr = PurposeReport::compute(
            &records, &ablated, &centre, 1.0, 0.1, 1.1, args.alpha,
        );
        eprintln!("      P_OR={:.4}   purposeless={}", pr.p_or_full, pr.purposeless.len());
        (dr, pr)
    } else {
        eprintln!("[2/3] Dynamic analysis  (skipped — no traces-dir)");
        eprintln!("[3/3] Purposelessness   (skipped)");
        let dr = DynamicReport {
            r_dyn_series:  vec![],
            violations:    vec![],
            drift_events:  vec![],
            regime_actual: static_report.regime.as_str().to_owned(),
        };
        let pr = PurposeReport { p_or_full: 0.0, scores: HashMap::new(), purposeless: vec![] };
        (dr, pr)
    };

    let metric = WindTunnelMetric::from_reports(&static_report, &dynamic_report, &purpose_report);

    if args.json {
        println!("{}", metric.to_json()?);
    } else {
        eprintln!();
        metric.print_regime_map();
    }

    // Persist result.
    let out_path = wt_dir(&args.project_dir)?.join("output.json");
    std::fs::write(&out_path, metric.to_json()?)
        .with_context(|| format!("writing {}", out_path.display()))?;
    eprintln!("Result saved to {}.", out_path.display());

    // Exit code: 0 = Coherent or Phase-locked, 1 = below Coherent.
    let final_r = dynamic_report.r_dyn_series.last()
        .map(|(_, r)| *r)
        .unwrap_or(static_report.r_est);
    if final_r < 0.80 { std::process::exit(1); }
    Ok(())
}

fn cmd_report(args: &Args) -> anyhow::Result<()> {
    // If an explicit path was given use it; otherwise look in .wt/output.json.
    let path = match &args.report_file {
        Some(f) => f.clone(),
        None    => {
            let p = wt_dir(&args.project_dir)?.join("output.json");
            if !p.exists() {
                bail!("No saved result found at {}. Run `wt run` first.", p.display());
            }
            p
        }
    };
    let raw    = std::fs::read_to_string(&path)
        .with_context(|| format!("reading {}", path.display()))?;
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

fn ablate_map(records: &[RuntimeRecord]) -> HashMap<wt_graph::UnitId, Vec<RuntimeRecord>> {
    records.iter().map(|r| {
        let ab = records.iter().filter(|o| o.unit_id != r.unit_id).cloned().collect();
        (r.unit_id.clone(), ab)
    }).collect()
}

fn regime_colour(regime: &str) -> &'static str {
    if regime.contains("Turbulent")    { return "\x1b[31m"; }
    if regime.contains("Aperture")     { return "\x1b[33m"; }
    if regime.contains("Hierarchical") { return "\x1b[93m"; }
    if regime.contains("Coherent")     { return "\x1b[32m"; }
    if regime.contains("Phase")        { return "\x1b[34m"; }
    "\x1b[0m"
}

// ── Entry point ───────────────────────────────────────────────────────────────

fn main() {
    let args = match Args::parse() {
        Ok(a)  => a,
        Err(e) => { eprintln!("Error: {e:#}"); print_usage(); std::process::exit(2); }
    };

    let result = match args.subcommand.as_str() {
        "static"  => cmd_static(&args),
        "dynamic" => cmd_dynamic(&args),
        "purpose" => cmd_purpose(&args),
        "run"     => cmd_run(&args),
        "report"  => cmd_report(&args),
        "init"    => cmd_init(&args),
        other     => {
            eprintln!("Unknown subcommand: {other}");
            print_usage();
            std::process::exit(2);
        }
    };

    if let Err(e) = result {
        eprintln!("Error: {e:#}");
        std::process::exit(2);
    }
}
