# Wind Tunnel

A global self-consistency framework for software correctness.

---

## The problem

Unit tests, integration tests, and property-based tests are all *local queries*. Each one addresses a bounded subsystem and asks whether that subsystem behaves correctly in isolation. This is structurally insufficient.

A system can satisfy every local test while failing globally. The failure mode is not unusual: individually correct data-transformation steps compose into a cycle that accumulates a semantic error invisible to any single observer. No component misbehaved. The failure resided in the cycle, not in any node.

This is not an engineering caveat — it is a theorem. A bounded observer with state capacity |K_R| cannot encode a complete correctness criterion for a system whose state complexity exceeds log₂|K_R| bits. Test suites are bounded observers. The No Template Theorem makes this precise.

---

## The approach

An aeronautical wind tunnel does not test individual rivets. It tests the assembled aircraft under operational load, measuring quantities — lift, drag, turbulence — that are properties of the whole. There is no expected output written on a card. The criterion is self-consistency under load.

This tool applies the same principle to software. Instead of a binary pass/fail oracle, it produces a *regime map*: a multi-dimensional characterisation of how coherently the system's units coordinate toward a shared purpose.

The theoretical foundation is developed in full in the companion paper:

> *Wind Tunnel: A Global Self-Consistency Framework for Software Correctness*
> `publications/windtunnel-code-testing/windtunnel-code-testing.tex`

---

## Core concepts

**Semantic entropy S ∈ [0, Σ]** — a scalar measuring the residual distance between the system's current state and purposeful operation. S has a positive floor S_flat > 0 (Floor Positivity axiom): a system that is working correctly still carries irreducible uncertainty; the floor is not zero.

**Action-cell C\*** — the positive-volume region in outcome space where S = S_flat. Correctness is not a point; it is a region. All states inside C\* are indistinguishable by S.

**Holonomy hol(c, x) = ‖T_c(x) − T_c^spec(x)‖** — the deviation of a directed cycle's actual transformation from its declared specification. Nonzero holonomy on any cycle implies global semantic incorrectness (Cycle Inconsistency Theorem). Zero holonomy is necessary but not sufficient.

**Ensemble order parameter R_ens ∈ [0, 1]** — phase coherence across the dependency graph, derived from Kuramoto coupling theory. Five regimes:

| R_ens | Regime |
|---|---|
| < 0.30 | Turbulent — no shared action-cell |
| 0.30 – 0.50 | Aperture-dominated — interface contracts without shared purpose |
| 0.50 – 0.80 | Hierarchical cascade — coherent sub-ensembles, incoherent interfaces |
| 0.80 – 0.95 | Coherent — converging toward a common action-cell |
| ≥ 0.95 | Phase-locked — partition extinction, coordination friction = 0 |

**Contribution score δS(u, E)** — the reduction in the ensemble's semantic floor when unit u is included. A unit with δS = 0 is *purposeless*: its removal does not change what the ensemble can achieve.

---

## Protocol

The tool runs in two phases.

**Static phase** (no running system required). Reads the dependency graph and unit type signatures via the `purpose` index. Computes synchronisation tension on each edge, estimates R_est, classifies the coordination regime, identifies decoherence zones (subgraphs whose local R_est falls below the global average), and enumerates directed cycles whose static residuals suggest likely holonomy violations.

**Dynamic phase** (requires runtime traces). Runs the system under representative load. Measures R_dyn(t) as a time series, detects live holonomy violations on candidate cycles, and checks for aperture drift (units whose runtime outputs exit their declared type sets). Computes contribution scores by ablation.

Primary output: `WT(E, Λ) = (R_dyn, S_flat_est, H, D, δS)` — a regime map, not a verdict.

---

## Dependencies

| Dependency | Purpose |
|---|---|
| Rust ≥ 1.75 | All five library crates and the `wt` binary |
| [`purpose`](https://github.com/fullscreen-triangle/purpose) | Builds a symbol index for the target project; `wt-graph` queries it to extract unit type signatures without reading source files |
| Python ≥ 3.10 + matplotlib + numpy | Validation suite and figure generation only — not required for `wt` itself |

---

## Installation

```sh
# 1. Install the purpose indexer
cargo install --git https://github.com/fullscreen-triangle/purpose

# 2. Clone and build wind-tunnel
git clone <this-repo>
cd wind-tunnel
cargo build --release

# The binary is at target/release/wt
# Optionally install it:
cargo install --path wt
```

---

## Usage

### Static analysis only (no traces needed)

```sh
# Index the target project first
cd /path/to/your/project
purpose index

# Run the static phase
wt static /path/to/your/project
```

Output:

```
Regime        : Hierarchical cascade
R_est         : 0.6143
K_c estimate  : 0.8821
Decoherence zones: 2
Cycle candidates : 1
  ["payments", "ledger", "reconciler"]  residual=2.3401
```

### Full protocol (with runtime traces)

Traces are JSONL files, one per unit, placed in a directory. Each line is a JSON object `{"t": <float>, "state": [<float>, ...]}`. The filename stem is the unit id.

```sh
# Run your system under load and collect traces into /tmp/traces/
# (see docs/trace-format.md for the schema)

wt run /path/to/your/project /tmp/traces/
```

Output: `wt-output.json` in the project directory, plus a coloured terminal regime map.

### Re-render a saved result

```sh
wt report wt-output.json
```

### Machine-readable output

All subcommands accept `--json`:

```sh
wt static /path/to/project --json | jq '.regime'
```

### Flags

| Flag | Default | Meaning |
|---|---|---|
| `--json` | off | Emit JSON instead of human-readable output |
| `--cycle-depth N` | 12 | Maximum cycle length for Johnson's algorithm |
| `--threshold T` | 0.5 | Static residual threshold for cycle candidates |
| `--tol T` | 1e-6 | Holonomy violation tolerance |
| `--dt T` | 0.1 | Time step for R_dyn time series |
| `--alpha A` | 0.05 | Significance level for purposelessness detection |

---

## Working with the graph directly

If `purpose index` has already been run on the target project, `wt-graph.json` can be written manually or generated and cached:

```sh
# Generate and cache the graph (skips purpose queries on subsequent runs)
wt static /path/to/project --json > /dev/null
# wt writes wt-graph.json to the project dir on first run
```

On subsequent runs, `wt` reads `wt-graph.json` directly without re-querying `purpose`.

---

## Repository layout

```
wind-tunnel/
├── Cargo.toml                          workspace root
├── crates/
│   ├── wt-graph/                       data model, cycle enumeration, purpose bridge
│   ├── wt-static/                      Phase 1: tension, R_est, decoherence zones
│   ├── wt-dynamic/                     Phase 2: traces, holonomy, R_dyn
│   ├── wt-purpose/                     Phase 3: contribution scores, purposelessness
│   └── wt-report/                      WindTunnelMetric assembly and rendering
├── wt/                                 CLI binary
├── src/
│   ├── validation.py                   25-experiment numerical validation suite
│   ├── panels.py                       publication figure generator
│   └── validation_results.json         25/25 PASS, max error 1.11e-16
├── docs/
│   ├── implementation.md               implementation plan (strike-through = done)
│   └── sources/                        foundational papers (read-only)
└── publications/
    └── windtunnel-code-testing/
        ├── windtunnel-code-testing.tex  companion paper
        ├── references.bib
        └── figures/                    panel_{1..5}.png + captions
```

---

## Validation

All theorems in the companion paper are numerically validated:

```sh
python src/validation.py
```

```
25/25 PASS
max relative error: 1.11e-16
```

Experiments cover: S-functional axioms, action-cell geometry, Local Invisibility witness, No Template pigeonhole bound, cycle holonomy accumulation, DAG boundary conditions, Kuramoto dynamics across all five regimes, Partition Extinction discontinuity, synchronisation tension decomposition, static R_est, decoherence zone detection, and contribution score computation.

---

## Running the test suite

```sh
cargo test --workspace
```

23 unit tests across all five library crates. No external services required.

---

## Theoretical background

The framework draws on:

- Kuramoto (1975, 1984) — coupled oscillator synchronisation and the critical coupling K_c = 2σ_ω/π
- Landau (1937) — second-order phase transitions; the Coherent → Phase-locked transition is discontinuous
- Banach (1922) — fixed-point theorem used in the Purpose Existence proof
- Kirchhoff (1847) — circuit analogy for constraint cycles
- Johnson (1975) — elementary cycle enumeration algorithm used in `wt-graph`

The full bibliography is in `publications/windtunnel-code-testing/references.bib`.

---

## Limitations

**Holonomy requires a cycle specification.** The dynamic phase measures `‖T_c(x) − T_c^spec(x)‖`. The specification function `T_c^spec` must be supplied by the user for each candidate cycle. Without it, the tool defaults to the identity (the cycle should return to its initial state), which is incorrect for stateful loops.

**Contribution scores require ablation runs.** Computing δS(u, E) requires re-running the system without unit u. For large ensembles this is O(n) runs. The static phase provides a regime map without this cost.

**Phase estimation is a proxy.** The `ScalarPhaseEstimator` maps the first state dimension to [0, 2π). For units whose state is not naturally periodic, the estimated R_dyn is a rough indicator, not a precise measurement. Custom estimators can be plugged in via the `PhaseEstimator` trait.

**The action-cell parameters (centre, radius, beta) are domain-specific.** The tool cannot infer them from traces alone. They must be provided by the caller or approximated from the centroid of terminus states.
