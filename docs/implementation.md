# Wind Tunnel — Implementation Plan

Strike-through (`~~item~~`) marks completed tasks.

---

## Phase 0 — Foundations (paper + validation)

- ~~Write `publications/windtunnel-code-testing/windtunnel-code-testing.tex` (1 202-line two-column paper)~~
- ~~Write `publications/windtunnel-code-testing/references.bib` (21 entries)~~
- ~~Apply peer-review fixes (holonomy definition, Cycle Inconsistency one-directionality, strip Gödel/Shannon/Turing decoration)~~
- ~~Write `src/validation.py` (25 experiments, all theorems)~~
- ~~Confirm 25/25 PASS, max relative error 1.11e-16~~
- ~~Write `.gitignore`~~
- ~~Generate `publications/windtunnel-code-testing/figures/panel_{1..5}.png` (5 panels, 4 charts each, ≥1 3D per panel)~~
- ~~Write `publications/windtunnel-code-testing/figures/code-captions.tex` (LaTeX captions cross-referenced to theorem labels)~~

---

## Phase 1 — Workspace skeleton

- [ ] Populate `Cargo.toml` as a Cargo workspace declaring all five member crates
- [ ] Create directory tree:
  ```
  crates/wt-graph/
  crates/wt-static/
  crates/wt-dynamic/
  crates/wt-purpose/
  crates/wt-report/
  wt/
  ```
- [ ] Add stub `Cargo.toml` and `src/lib.rs` (or `src/main.rs`) for each crate
- [ ] Confirm `cargo check --workspace` passes on the empty stubs

---

## Phase 2 — `wt-graph` (shared data model + `purpose` bridge)

### 2.1 Core types
- [ ] `UnitId` — newtype over `String`
- [ ] `ApertureSet` — `Vec<String>` (categorical type tags from `purpose`)
- [ ] `ProductionSet` — `Vec<String>`
- [ ] `Unit { id: UnitId, aperture: ApertureSet, production: ProductionSet, freq_est: f64 }`
- [ ] `Edge { from: UnitId, to: UnitId }`
- [ ] `SystemGraph { units: Vec<Unit>, edges: Vec<Edge> }` with adjacency helpers

### 2.2 Graph algorithms
- [ ] `SystemGraph::neighbours(id)` — outgoing neighbours
- [ ] `SystemGraph::cycles(max_depth: usize) -> Vec<Vec<UnitId>>` — Johnson's algorithm
- [ ] `SystemGraph::connected_subgraphs() -> Vec<Vec<UnitId>>` — BFS/DFS components
- [ ] `SystemGraph::subgraph(ids: &[UnitId]) -> SystemGraph` — induced subgraph

### 2.3 `purpose` bridge
- [ ] `PurposeClient` — wraps `std::process::Command` calling `purpose ask`
- [ ] `PurposeClient::aperture(unit_path: &str) -> ApertureSet` — query input types
- [ ] `PurposeClient::production(unit_path: &str) -> ProductionSet` — query output types
- [ ] `PurposeClient::freq_estimate(unit_path: &str) -> f64` — proxy: LOC or call-count
- [ ] `SystemGraph::from_purpose_index(index_path: &Path) -> Result<SystemGraph>`
  - Reads `.purpose/index.json`, calls `PurposeClient` per symbol, builds graph

### 2.4 Serialisation
- [ ] `serde` derives on all types
- [ ] `SystemGraph::to_json() / from_json()` round-trip
- [ ] Unit tests: cycle detection on known graphs, subgraph extraction

---

## Phase 3 — `wt-static` (Phase 1 of the protocol)

### 3.1 Aperture gap
- [ ] `aperture_gap(a: &ApertureSet, p: &ProductionSet) -> f64`
  - 0.0 if intersection non-empty; else normalised type-distance (Jaccard complement)

### 3.2 Synchronisation tension
- [ ] `tension(u_i: &Unit, u_j: &Unit) -> f64`
  - `aperture_gap(P_{u_i}, A_{u_j}) + (ω_i - ω_j).abs()`

### 3.3 Static order parameter + regime
- [ ] `r_est(graph: &SystemGraph) -> f64` — `exp(-mean tension across edges)`
- [ ] `Regime` enum: `Turbulent | ApertureDominated | HierarchicalCascade | Coherent | PhaseLocked`
- [ ] `classify(r: f64) -> Regime`
- [ ] `k_c_estimate(graph: &SystemGraph) -> f64` — `2 * σ_ω / π`

### 3.4 Decoherence zones
- [ ] `decoherence_zones(graph: &SystemGraph) -> Vec<SystemGraph>`
  - All maximal connected subgraphs where `r_est(sub) < r_est(graph)`
  - Ranked by severity (`r_est(global) - r_est(sub)`)

### 3.5 Cycle residual candidates
- [ ] `static_residual(cycle: &[UnitId], graph: &SystemGraph) -> f64`
  - Sum of per-edge tensions around the cycle
- [ ] `cycle_candidates(graph: &SystemGraph, threshold: f64) -> Vec<(Vec<UnitId>, f64)>`
  - Cycles whose static residual exceeds `threshold`

### 3.6 Static report
- [ ] `StaticReport { r_est, regime, k_c, zones, cycle_candidates }`
- [ ] Unit tests against the 25-experiment validation numbers

---

## Phase 4 — `wt-dynamic` (Phase 2 of the protocol)

### 4.1 Trace ingestion
- [ ] `Sample { t: f64, state: Vec<f64> }`
- [ ] `Trajectory { unit_id: UnitId, samples: Vec<Sample> }`
- [ ] `RuntimeRecord { unit_id, trajectory, terminus: Vec<f64>, memory: Vec<Vec<f64>> }`
- [ ] `TraceLoader::from_jsonl(path: &Path) -> Vec<RuntimeRecord>` — JSONL format, one record per line

### 4.2 Phase estimation
- [ ] `PhaseEstimator` trait: `fn estimate(record: &RuntimeRecord, t: f64) -> f64`
- [ ] `FourierPhaseEstimator` — default: dominant frequency of output time series via FFT
- [ ] `ArgPhaseEstimator` — for scalar outputs: `θ̂ = arg(z)` where `z` is state as complex

### 4.3 Dynamic order parameter
- [ ] `r_dyn(records: &[RuntimeRecord], t: f64, estimator: &dyn PhaseEstimator) -> f64`
- [ ] `r_dyn_series(records, dt, estimator) -> Vec<(f64, f64)>` — time series

### 4.4 Holonomy measurement
- [ ] `holonomy_dyn(cycle, records, spec_fn) -> f64`
  - `‖Γ_{i_k} - T_c^spec(γ_{i_1}(0))‖`
  - `spec_fn: Box<dyn Fn(&[f64]) -> Vec<f64>>` — user-supplied cycle specification
- [ ] `HolonomyViolation { cycle: Vec<UnitId>, magnitude: f64 }`
- [ ] `live_violations(graph, records, specs, tol) -> Vec<HolonomyViolation>`

### 4.5 Aperture drift detection
- [ ] `DriftEvent { unit_id: UnitId, t: f64, state: Vec<f64> }`
- [ ] `aperture_drift(unit: &Unit, record: &RuntimeRecord) -> Vec<DriftEvent>`
  - Detects samples where state exits declared production set

### 4.6 Dynamic report
- [ ] `DynamicReport { r_dyn_series, violations, drift_events, regime_actual }`
- [ ] Unit tests: known holonomy witnesses from E05/E09 validation experiments

---

## Phase 5 — `wt-purpose` (Phase 3 of the protocol)

### 5.1 OR-success probability
- [ ] `or_success(records: &[RuntimeRecord], threshold: f64) -> f64`
  - Fraction of terminus states with estimated `S ≤ threshold`
- [ ] `s_estimate(terminus: &[f64], centre: &[f64], radius: f64, beta: f64) -> f64`
  - Re-uses the same formula as `src/validation.py`

### 5.2 Contribution scores
- [ ] `contribution_score(unit_id: &UnitId, full_records: &[RuntimeRecord], ablated_records: &[RuntimeRecord], threshold: f64) -> f64`
  - `P_OR(E, Λ) - P_OR(E \ {u}, Λ)`
- [ ] `purposeless_units(scores: &HashMap<UnitId, f64>, alpha: f64) -> Vec<UnitId>`
  - Units where `score < alpha`

### 5.3 Purpose report
- [ ] `PurposeReport { scores: HashMap<UnitId, f64>, purposeless: Vec<UnitId>, p_or_full: f64 }`
- [ ] Unit tests against E21–E25 validation experiments

---

## Phase 6 — `wt-report` (output layer)

- [ ] `WindTunnelMetric { r_dyn_series, s_flat_est, holonomy_violations, decoherence_zones, contribution_scores }`
- [ ] `WindTunnelMetric::from_reports(static, dynamic, purpose) -> Self`
- [ ] `WindTunnelMetric::to_json(&self) -> String` (pretty-printed)
- [ ] `WindTunnelMetric::print_regime_map(&self)` — coloured terminal table
  - Regime name + colour (red/orange/yellow/green/blue)
  - Per-unit contribution score bar
  - Decoherence zone list ranked by severity
  - Holonomy violations list
- [ ] Integration test: feed known traces → check JSON round-trip

---

## Phase 7 — `wt` binary (CLI)

- [ ] `wt static  <project-dir>`              — Phase 1 only; prints `StaticReport`
- [ ] `wt dynamic <project-dir> <traces-dir>` — Phase 2; prints `DynamicReport`
- [ ] `wt purpose <project-dir> <traces-dir>` — Phase 3; prints `PurposeReport`
- [ ] `wt run     <project-dir> [<traces-dir>]` — all phases in sequence; writes `wt-output.json`
- [ ] `wt report  <wt-output.json>`           — re-render saved result
- [ ] `--json` flag on all subcommands for machine-readable output
- [ ] `--threshold` flag (default 0.05) for purposelessness significance level
- [ ] `--tol` flag (default 1e-6) for holonomy violation tolerance
- [ ] Exit code 0 = Phase-locked or Coherent; 1 = below Coherent; 2 = error

---

## Phase 8 — Integration + end-to-end test

- [ ] Write `tests/e2e/` with a minimal synthetic project (3-unit cycle)
- [ ] `wt run tests/e2e/synthetic` reproduces the E05 witness (cycle residual accumulation)
- [ ] `wt run tests/e2e/synthetic` regime = Turbulent when δ > 0, Phase-locked when δ = 0
- [ ] CI script (`scripts/ci.sh`): `cargo test --workspace && python src/validation.py`

---

## Unblocking order

```
Phase 0 ✓ → Phase 1 → Phase 2 → Phase 3 → Phase 4 ─┐
                                                      ├→ Phase 6 → Phase 7 → Phase 8
                                           Phase 5 ──┘
```

Phases 4 and 5 can proceed in parallel once Phase 2 is complete.
