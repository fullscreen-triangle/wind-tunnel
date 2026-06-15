# Trace Format

The dynamic phase reads runtime traces from a directory of JSONL files.

## File layout

```
traces/
├── payments.jsonl
├── ledger.jsonl
└── reconciler.jsonl
```

One file per unit. The filename stem is the unit id — it must match the id used when the `SystemGraph` was built from the `purpose` index.

## Record schema

Each line in a JSONL file is a JSON object:

```json
{"t": 0.00, "state": [1.2, 0.4]}
{"t": 0.05, "state": [1.3, 0.4]}
{"t": 0.10, "state": [1.1, 0.5]}
```

| Field   | Type          | Meaning |
|---------|---------------|---------|
| `t`     | float (seconds) | Observation timestamp. Must be monotonically non-decreasing within a file. |
| `state` | array of float  | The unit's observable state vector at time `t`. Dimensionality must be consistent within a file; it may differ across units. |

The first state dimension is used by the default `ScalarPhaseEstimator` to compute the phase angle θ̂. If the unit's state is multi-dimensional and the first dimension is not meaningful as a phase proxy, supply a custom `PhaseEstimator`.

## Minimum viable trace

A file with a single sample is valid. `r_dyn_series` will return a single point; holonomy is measured from `terminus` (the last sample's state).

```json
{"t": 1.0, "state": [0.83]}
```

## Collecting traces

Wind tunnel does not prescribe how traces are collected. Common approaches:

- **Middleware / interceptor**: wrap each service's output channel; emit one JSONL line per message.
- **Sidecar**: a lightweight process that samples the unit's output queue at fixed intervals.
- **Log extraction**: if the unit already emits structured logs, parse them into JSONL.

The key invariant: `state` must represent the unit's *output* at time `t`, not its internal state. The wind tunnel measures what a unit *produces*, not how it produces it.

## Holonomy cycle specifications

For candidate cycles identified by the static phase, a specification function maps the initial state of the first unit to the intended terminal state after one full cycle traversal:

```
spec_fn: &dyn Fn(&[f64]) -> Vec<f64>
```

The default is the identity (`|x| x.to_vec()`), which is correct only for cycles that are expected to return to their starting state. For stateful cycles (e.g., a counter that should increment by exactly 3 per traversal), provide a custom spec:

```rust
let spec: Box<dyn Fn(&[f64]) -> Vec<f64]> = Box::new(|x| vec![x[0] + 3.0]);
```

Without a correct spec, holonomy measurements are unreliable.
