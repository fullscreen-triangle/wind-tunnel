# Wind Tunnel — Usage Guide

Wind tunnel analyses whether your software system is *globally self-consistent* — whether its units coordinate toward a shared purpose rather than merely passing local tests. This guide covers installation, configuration, and every subcommand.

---

## Prerequisites

| Requirement | Version | Notes |
|---|---|---|
| Rust + Cargo | ≥ 1.75 | Install via [rustup.rs](https://rustup.rs) |
| `purpose` CLI | latest | Intent indexer — see below |
| Python | ≥ 3.10 | Validation suite only — not required to run `wt` |

---

## Installation

### 1. Install Rust

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### 2. Install the `purpose` indexer

```sh
cargo install --git https://github.com/fullscreen-triangle/purpose
```

`purpose` builds a symbol index for any project. Wind tunnel's `wt-graph` crate
queries it to extract unit intent labels when AST parsing is unavailable or insufficient.

### 3. Build and install `wt`

```sh
git clone <this-repo>
cd wind-tunnel
cargo install --path wt
```

`wt` is now on your PATH. Verify:

```sh
wt --version
```

---

## HuggingFace extraction backend (optional)

Wind tunnel can use a code model via the HuggingFace Inference API to extract
type signatures from source files that tree-sitter cannot parse completely. To
enable it:

1. Copy the example env file — **do not commit it**:
   ```sh
   cp .env.local.example .env.local
   ```

2. `.env.local` already contains your token. Source it before running `wt`:
   ```sh
   source .env.local   # bash/zsh
   # or on Windows:
   $env:HF_TOKEN = "hf_..."
   ```

   `HUGGINGFACE_HUB_TOKEN` is also accepted as an alias.

3. Optionally override the model (default: `bigcode/starcoder2-7b`):
   ```sh
   export WT_HF_MODEL=bigcode/starcoder2-15b
   ```

Both `.env.local` and `.env.local.example` are git-ignored — they will never be pushed.

---

## Extraction backends

`wt` has three strategies for reading your codebase:

| Backend | Flag | How it works |
|---|---|---|
| `auto` | `--backend auto` (default) | Tree-sitter per file; falls back to HF if the parse returns nothing; falls back to `purpose` CLI if no recognised source files exist |
| `treesitter` | `--backend treesitter` | AST-based exact extraction. Supports `.rs`, `.py`, `.js`, `.mjs`, `.ts`, `.tsx`. Fast, no network, no token needed. |
| `hf` | `--backend hf` | HuggingFace Inference API. Language-agnostic, probabilistic. Requires `HF_TOKEN`. |

For most Rust / Python / JS / TS projects the default `auto` is all you need.
Use `--backend hf` explicitly when the project is heavily macro-heavy, dynamically
typed, or uses a language tree-sitter does not cover.

---

## Quick start

```sh
cd /path/to/your/project

# Static analysis — no traces required
wt static

# Full protocol — with runtime traces in /tmp/traces/
wt run . /tmp/traces/
```

Both commands default to the current directory if no path is given.

---

## Subcommands

### `wt static [<project-dir>]`

Reads the codebase, builds the dependency graph, and runs Phase 1 of the protocol.

**What it computes:**
- Synchronisation tension `ϑ(u_i, u_j)` on every edge
- Static order parameter `R_est = exp(−mean tension)`
- Coordination regime (Turbulent → Phase-locked)
- Critical coupling estimate `K_c = 2σ_ω / π`
- Decoherence zones — maximal subgraphs whose local `R_est` falls below the global value
- Cycle candidates — directed cycles whose static residual exceeds `--threshold`

**Example:**

```sh
wt static /path/to/project
```

```
Project : /path/to/project
Units   : 47   Edges: 83
Regime  : Hierarchical cascade
R_est   : 0.6143
K_c     : 0.8821
Zones   :
  R_est=0.4201  Δ=0.1942  ["payments", "ledger"]
Cycles  :
  residual=2.3401  ["payments", "ledger", "reconciler"]
```

The coloured regime label is the most important single output:

| Colour | Regime | What it means |
|---|---|---|
| Red | Turbulent | Units share no common action-cell. Expect semantic drift under any load. |
| Yellow | Aperture-dominated | Interface contracts exist but there is no shared purpose. |
| Bright yellow | Hierarchical cascade | Coherent clusters, incoherent interfaces between them. |
| Green | Coherent | Converging toward a shared action-cell. Holonomy checks are warranted. |
| Blue | Phase-locked | Partition extinction. Coordination friction is zero. |

---

### `wt dynamic [<project-dir>] <traces-dir>`

Runs Phase 2. Ingests runtime traces and measures dynamic coherence.

**What it computes:**
- `R_dyn(t)` as a time series — phase coherence as the system runs
- Live holonomy violations on candidate cycles
- Aperture drift events — units whose runtime outputs exit their declared type sets

**Traces** are JSONL files, one per unit, placed in a directory:

```
traces/
├── payments.jsonl
├── ledger.jsonl
└── reconciler.jsonl
```

Each line is one observation:

```json
{"t": 0.00, "state": [1.2, 0.4]}
{"t": 0.05, "state": [1.3, 0.4]}
```

`t` is a monotonically increasing timestamp (seconds). `state` is the unit's
observable output vector at that moment. The filename stem must match the unit's
id in the dependency graph. See [trace-format.md](trace-format.md) for the full schema.

**Example:**

```sh
wt dynamic . /tmp/traces/
```

```
Regime (actual)    : Coherent
R_dyn (final)      : 0.8712
Holonomy violations: 1
  |hol|=0.0341  ["payments", "ledger", "reconciler"]
Aperture drift     : 0
```

A holonomy violation means the cycle's actual transformation diverged from its
declared specification. See [trace-format.md](trace-format.md) for how to supply
a non-identity cycle specification.

---

### `wt purpose [<project-dir>] <traces-dir>`

Runs Phase 3. Computes contribution scores and identifies purposeless units.

**What it computes:**
- `P_OR(E)` — the ensemble's probability of reaching the action-cell under representative load
- `δS(u, E) = P_OR(E) − P_OR(E \ {u})` for each unit — how much the ensemble loses when u is removed
- Purposeless units — units where `δS ≈ 0` (within `--alpha`)

A unit is purposeless if removing it does not measurably change what the
ensemble can achieve. This is distinct from being unused: a unit may execute
on every request and still contribute nothing to global purpose.

**Example:**

```sh
wt purpose . /tmp/traces/
```

```
P_OR (full)  : 0.8431
  reconciler                      δS=0.2103
  payments                        δS=0.1847
  ledger                          δS=0.0912
  audit_log                       δS=0.0000  [purposeless]
```

**Note:** contribution scores require one ablation run per unit (the system is
re-run without each unit in turn). For large ensembles this is O(n) runs.
The static phase provides a regime map without this cost.

---

### `wt run [<project-dir>] [<traces-dir>]`

Runs all three phases in sequence and writes the result to `.wt/output.json`.

```sh
# Static only (no traces)
wt run /path/to/project

# All three phases
wt run /path/to/project /path/to/traces/
```

Phases 2 and 3 are skipped gracefully if no traces directory is provided.

**Exit codes:**

| Code | Meaning |
|---|---|
| 0 | Coherent or Phase-locked |
| 1 | Below Coherent (R < 0.80) |
| 2 | Error |

This makes `wt run` composable in CI pipelines:

```sh
wt run . && echo "coherent" || echo "degraded"
```

---

### `wt report [<wt-output.json>]`

Re-renders a saved result without re-running analysis.

```sh
# Use the result from the last `wt run` in the current directory
wt report

# Specify a file explicitly
wt report /path/to/output.json
```

---

### `wt init [<project-dir>]`

Forces a rebuild of the dependency graph and caches it to `.wt/graph.json`.
Use this after adding or removing units from the project.

```sh
wt init /path/to/project
```

Subsequent `wt` calls read the cached graph without re-indexing unless `--no-cache` is passed.

---

## All flags

| Flag | Default | Applies to | Meaning |
|---|---|---|---|
| `--json` | off | all | Emit machine-readable JSON instead of the coloured terminal output |
| `--no-cache` | off | static, dynamic, purpose, run, init | Ignore `.wt/graph.json`; rebuild the dependency graph from source |
| `--backend B` | `auto` | static, dynamic, purpose, run, init | Extraction backend: `treesitter`, `hf`, or `auto` |
| `--cycle-depth N` | 12 | static, run | Maximum cycle length for Johnson's enumeration algorithm |
| `--threshold T` | 0.5 | static, run | Static residual threshold for reporting cycle candidates |
| `--tol T` | 1e-6 | dynamic, run | Holonomy violation tolerance |
| `--dt T` | 0.1 | dynamic, run | Time step for the `R_dyn` time series |
| `--alpha A` | 0.05 | purpose, run | Significance level for purposelessness detection |

---

## Using `wt` from any repository

After `cargo install --path wt`, the binary is in `~/.cargo/bin/wt` and available
everywhere. From inside any project:

```sh
cd ~/projects/my-service

# Static scan of this project
wt static

# Full run with traces from a local directory
wt run . ./traces/

# Bare path — treated as `wt run <path>`
wt ~/projects/my-service
```

No configuration file is needed. Wind tunnel discovers source files by walking
the project directory and skips `target/`, `node_modules/`, `.git/`, `dist/`,
`__pycache__/`, and `.wt/` automatically.

---

## Machine-readable output

All subcommands accept `--json`. The schema is `WindTunnelMetric`:

```sh
wt run . --json | jq '.regime'
wt run . --json | jq '.r_est'
wt run . --json | jq '.holonomy_violations[] | select(.magnitude > 0.01)'
wt run . --json | jq '.contribution_scores | to_entries | sort_by(-.value)'
```

The JSON is also written to `.wt/output.json` after every `wt run`.

---

## VS Code integration

The `.vscode/` directory in this repository ships three files that wire `wt`
into VS Code with no manual setup:

### tasks.json — keyboard shortcut

`Ctrl+Shift+B` runs `wt static` on the current workspace immediately.

To pick a different task: `Ctrl+Shift+P` → **Tasks: Run Task** → choose from:

| Task label | What it runs |
|---|---|
| `wt: static` | `wt static <workspace>` — default build task |
| `wt: run (static only)` | `wt run <workspace>` — all phases, no traces |
| `wt: run (with traces)` | prompts for a traces directory, then runs all three phases |
| `wt: dynamic` | `wt dynamic <workspace> <traces>` |
| `wt: purpose` | `wt purpose <workspace> <traces>` |
| `wt: report` | re-renders `.wt/output.json` |
| `wt: init (rebuild graph)` | force-rebuilds the dependency graph |
| `wt: run --json` | full run, JSON output to the terminal |

Output always appears in the integrated terminal panel.

### settings.json — wt-shell terminal profile

`settings.json` defines a terminal profile called **wt-shell** (the beaker icon).
Open it with `Ctrl+Shift+\`` → dropdown arrow → **wt-shell**.

It automatically sources `.env.local` when the terminal starts, so `HF_TOKEN`
is set without you having to do anything. This works on Windows (PowerShell),
Linux, and macOS.

> `settings.json` is git-ignored — it lives only on your machine.
> `tasks.json` and `launch.json` are committed and travel with the repo.

### Using wt in your other VS Code projects

Copy `.vscode/tasks.json` into any other project:

```sh
cp /path/to/wind-tunnel/.vscode/tasks.json /path/to/other-project/.vscode/tasks.json
```

`${workspaceFolder}` expands to that project's root automatically — no edits needed.

For the HuggingFace token, copy `settings.json` too (or set `HF_TOKEN` in your
system environment once and it will be available everywhere):

```sh
# Windows — set permanently in user environment
[System.Environment]::SetEnvironmentVariable("HF_TOKEN", "hf_...", "User")

# Linux / macOS — add to ~/.bashrc or ~/.zshrc
export HF_TOKEN=hf_...
```

Once `HF_TOKEN` is in your system environment you never need to source
`.env.local` manually again.

---

## CI integration

```yaml
# GitHub Actions example
- name: Wind Tunnel
  run: |
    cargo install --path wt
    wt run .
  # exits 1 if R < 0.80, 0 if Coherent or Phase-locked
```

```sh
# Pre-commit hook
#!/usr/bin/env bash
wt static --threshold 1.0 || { echo "wt: new decoherence zones detected"; exit 1; }
```

---

## Output files

All output is written to `.wt/` inside the analysed project:

| File | Written by | Contents |
|---|---|---|
| `.wt/graph.json` | `wt init`, first run of any subcommand | Cached dependency graph |
| `.wt/output.json` | `wt run` | Full `WindTunnelMetric` for the last run |

Both files are git-ignored by the `.gitignore` in the wind-tunnel repository itself.
Add `.wt/` to the `.gitignore` of any project you analyse.

---

## Interpreting the regime map

The regime map is the primary output. It is not a pass/fail verdict.

**Turbulent or Aperture-dominated:** The units do not share a common action-cell.
Individual tests may pass. The system will produce semantically inconsistent
outputs under load. Start with the decoherence zones — they localise the worst
coupling mismatches. Fix interface contracts (aperture/production type alignment)
before worrying about holonomy.

**Hierarchical cascade:** There are coherent clusters but their interfaces are
lossy. The cycle candidates are the next thing to examine. Run the dynamic phase
on a representative workload to see whether the static candidates produce live
violations.

**Coherent:** The system is converging. Run the full protocol including Phase 3
(contribution scores). Look for purposeless units — they carry coupling cost
without contributing to the ensemble's action-cell.

**Phase-locked:** Partition extinction has occurred. Coordination friction is
zero. The system behaves as a single unit with respect to its declared purpose.
This is the target state.
