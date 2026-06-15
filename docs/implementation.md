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

- ~~Populate `Cargo.toml` as a Cargo workspace declaring all five member crates~~
- ~~Create directory tree (`crates/wt-graph/`, `crates/wt-static/`, `crates/wt-dynamic/`, `crates/wt-purpose/`, `crates/wt-report/`, `wt/`)~~
- ~~Add `Cargo.toml` and `src/lib.rs` (or `src/main.rs`) for each crate~~
- ~~Confirm `cargo check --workspace` passes~~

---

## Phase 2 — `wt-graph` (shared data model + `purpose` bridge)

### 2.1 Core types
- ~~`UnitId` — newtype over `String`~~
- ~~`ApertureSet` — `Vec<String>` (categorical type tags from `purpose`)~~
- ~~`ProductionSet` — `Vec<String>`~~
- ~~`Unit { id: UnitId, aperture: ApertureSet, production: ProductionSet, freq_est: f64 }`~~
- ~~`Edge { from: UnitId, to: UnitId }`~~
- ~~`SystemGraph { units: Vec<Unit>, edges: Vec<Edge> }` with adjacency helpers~~

### 2.2 Graph algorithms
- ~~`SystemGraph::neighbours(id)` — outgoing neighbours~~
- ~~`SystemGraph::cycles(max_depth: usize) -> Vec<Vec<UnitId>>` — Johnson's algorithm~~
- ~~`SystemGraph::connected_components() -> Vec<Vec<UnitId>>` — BFS components~~
- ~~`SystemGraph::subgraph(ids: &[UnitId]) -> SystemGraph` — induced subgraph~~

### 2.3 `purpose` bridge
- ~~`PurposeClient` — wraps `std::process::Command` calling `purpose ask`~~
- ~~`PurposeClient::aperture(symbol: &str) -> ApertureSet`~~
- ~~`PurposeClient::production(symbol: &str) -> ProductionSet`~~
- ~~`PurposeClient::freq_estimate(symbol: &str) -> f64`~~
- ~~`PurposeClient::build_graph()` — reads `.purpose/index.json`, calls client per symbol~~

### 2.4 Serialisation
- ~~`serde` derives on all types~~
- ~~`SystemGraph::to_json() / from_json()` round-trip~~
- ~~Unit tests: cycle detection on known graphs, subgraph extraction (7/7 pass)~~

---

## Phase 3 — `wt-static` (Phase 1 of the protocol)

### 3.1 Aperture gap
- ~~`aperture_gap(production: &[String], aperture: &[String]) -> f64` — 0.0 if overlap; Jaccard distance otherwise~~

### 3.2 Synchronisation tension
- ~~`tension(graph, from, to) -> f64` — `aperture_gap + |ω_i − ω_j|`~~

### 3.3 Static order parameter + regime
- ~~`r_est(graph) -> f64` — `exp(−mean tension)`~~
- ~~`Regime` enum: `Turbulent | ApertureDominated | HierarchicalCascade | Coherent | PhaseLocked`~~
- ~~`Regime::classify(r: f64) -> Regime`~~
- ~~`k_c_estimate(graph) -> f64` — `2 σ_ω / π`~~

### 3.4 Decoherence zones
- ~~`decoherence_zones(graph) -> Vec<DecoherenceZone>` — maximal subgraphs with R_est < global, ranked by severity~~

### 3.5 Cycle residual candidates
- ~~`static_residual(cycle, graph) -> f64` — sum of per-edge tensions~~
- ~~`cycle_candidates(graph, max_len, threshold) -> Vec<(Vec<UnitId>, f64)>`~~

### 3.6 Static report
- ~~`StaticReport::compute(graph, cycle_max_len, threshold) -> StaticReport`~~
- ~~Unit tests: aperture gap, R_est, regime classification, K_c, serialisation (7/7 pass)~~

---

## Phase 4 — `wt-dynamic` (Phase 2 of the protocol)

### 4.1 Trace ingestion
- ~~`Sample { t: f64, state: Vec<f64> }`~~
- ~~`Trajectory { unit_id: UnitId, samples: Vec<Sample> }`~~
- ~~`RuntimeRecord { unit_id, trajectory, terminus, memory }`~~
- ~~`load_traces(dir: &Path) -> Vec<RuntimeRecord>` — JSONL, one file per unit~~

### 4.2 Phase estimation
- ~~`PhaseEstimator` trait: `fn estimate(record, t) -> f64`~~
- ~~`ScalarPhaseEstimator` — default: interpolate first state dim, map to [0, 2π)~~

### 4.3 Dynamic order parameter
- ~~`r_dyn(records, t, estimator) -> f64`~~
- ~~`r_dyn_series(records, dt, estimator) -> Vec<(f64, f64)>`~~

### 4.4 Holonomy measurement
- ~~`holonomy_dyn(cycle, records, spec_fn) -> Option<f64>`~~
- ~~`HolonomyViolation { cycle: Vec<UnitId>, magnitude: f64 }`~~
- ~~`live_violations(graph, records, tol, max_len) -> Vec<HolonomyViolation>`~~

### 4.5 Aperture drift detection
- ~~`DriftEvent { unit_id, t, state }`~~
- ~~`aperture_drift_with(record, is_outside: F) -> Vec<DriftEvent>`~~

### 4.6 Dynamic report
- ~~`DynamicReport::compute(graph, records, dt, tol, max_len) -> DynamicReport`~~
- ~~Unit tests: R_dyn phase-locked, holonomy zero/nonzero (3/3 pass)~~

---

## Phase 5 — `wt-purpose` (Phase 3 of the protocol)

### 5.1 OR-success probability
- ~~`s_estimate(state, centre, radius, beta) -> f64` — same formula as `validation.py`~~
- ~~`or_success(records, centre, radius, beta, threshold) -> f64`~~

### 5.2 Contribution scores
- ~~`contribution_score(full_records, ablated_records, …) -> f64` — `P_OR(E) − P_OR(E \ {u})`~~
- ~~`purposeless_units(scores, alpha) -> Vec<UnitId>`~~

### 5.3 Purpose report
- ~~`PurposeReport::compute(full_records, ablated_by_unit, …) -> PurposeReport`~~
- ~~Unit tests: s_estimate inside/outside, or_success, contribution zero, purposeless filter (5/5 pass)~~

---

## Phase 6 — `wt-report` (output layer)

- ~~`WindTunnelMetric { r_dyn_series, s_flat_est, holonomy_violations, decoherence_zones, contribution_scores, purposeless, regime, r_est, k_c }`~~
- ~~`WindTunnelMetric::from_reports(static, dynamic, purpose) -> Self`~~
- ~~`WindTunnelMetric::to_json() / from_json()` — pretty-printed JSON round-trip~~
- ~~`WindTunnelMetric::print_regime_map()` — coloured terminal table (regime, R_est, R_dyn, K_c, zones, violations, contribution bars)~~
- ~~Integration test: JSON round-trip (1/1 pass)~~

---

## Phase 7 — `wt` binary (CLI)

- ~~`wt static  <project-dir>`~~
- ~~`wt dynamic <project-dir> <traces-dir>`~~
- ~~`wt purpose <project-dir> <traces-dir>`~~
- ~~`wt run     <project-dir> [<traces-dir>]` — all phases; writes `wt-output.json`~~
- ~~`wt report  <wt-output.json>`~~
- ~~`--json` flag on all subcommands~~
- ~~`--cycle-depth`, `--threshold`, `--tol`, `--dt`, `--alpha` flags~~
- ~~Exit code 0 = Coherent/Phase-locked; 1 = below Coherent; 2 = error~~

---

## Phase 8 — Integration + end-to-end test

- ~~`crates/wt-tests/tests/e2e.rs` — 12 integration tests over the synthetic 3-unit cycle~~
  - ~~Static: homogeneous cycle → Phase-locked; type mismatch → degraded regime; heterogeneous cycle → cycle candidate~~
  - ~~E05 witness: δ=0 → zero holonomy; δ=0.05 → violation; magnitude grows with traversal count; magnitude matches formula~~
  - ~~R_dyn: synchronised units → Phase-locked~~
  - ~~Purpose: purposeless unit δS=0; purposeful unit δS>0~~
  - ~~Full pipeline: δ=0 → no violations, JSON round-trip; δ=0.15 → violations in output~~
- ~~CI script `scripts/ci.sh`: `cargo fmt --check && cargo clippy && cargo test --workspace && python src/validation.py`~~

---

## Unblocking order

```
Phase 0 ✓ → Phase 1 ✓ → Phase 2 ✓ → Phase 3 ✓ → Phase 4 ✓ ─┐
                                                               ├→ Phase 6 ✓ → Phase 7 ✓ → Phase 8 ✓
                                                  Phase 5 ✓ ──┘
```

**All phases complete. 35/35 tests pass (23 unit + 12 integration). 25/25 numerical validation experiments pass.**
