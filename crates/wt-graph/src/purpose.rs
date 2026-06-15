//! Bridge to the `purpose` CLI tool.
//!
//! `purpose index` — builds `.purpose/index.json` inside the project directory.
//! `purpose ask "<question>"` — returns symbol locations matching the query.
//!
//! This module calls those commands and parses their output to populate
//! `ApertureSet` and `ProductionSet` for each unit, without reading source
//! files directly.

use std::path::{Path, PathBuf};
use std::process::Command;
use anyhow::{bail, Context};
use serde::Deserialize;

use crate::graph::{ApertureSet, Edge, ProductionSet, SystemGraph, Unit, UnitId};

// ── purpose index entry ───────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct IndexEntry {
    /// Symbol name (function, struct, etc.)
    pub name:   String,
    /// File path relative to the project root
    pub file:   String,
    /// Line number
    pub line:   u32,
    /// Kind: "function" | "struct" | "trait" | "impl" | …
    pub kind:   Option<String>,
    /// Tags the purpose tool extracted (input/output type names, etc.)
    #[serde(default)]
    pub tags:   Vec<String>,
}

/// Client that wraps the `purpose` binary.
pub struct PurposeClient {
    /// Absolute path to the `purpose` executable.
    exe:         PathBuf,
    /// Project root (where `.purpose/index.json` lives after `purpose index`).
    project_dir: PathBuf,
}

impl PurposeClient {
    pub fn new(exe: impl Into<PathBuf>, project_dir: impl Into<PathBuf>) -> Self {
        Self {
            exe:         exe.into(),
            project_dir: project_dir.into(),
        }
    }

    /// Run `purpose index` in the project directory to (re)build the index.
    pub fn build_index(&self) -> anyhow::Result<()> {
        let status = Command::new(&self.exe)
            .arg("index")
            .current_dir(&self.project_dir)
            .status()
            .with_context(|| format!("failed to run {:?}", self.exe))?;
        if !status.success() {
            bail!("`purpose index` exited with status {}", status);
        }
        Ok(())
    }

    /// Run `purpose ask "<query>"` and return the raw stdout.
    pub fn ask(&self, query: &str) -> anyhow::Result<String> {
        let out = Command::new(&self.exe)
            .args(["ask", query])
            .current_dir(&self.project_dir)
            .output()
            .with_context(|| format!("failed to run `purpose ask {query}`"))?;
        if !out.status.success() {
            let stderr = String::from_utf8_lossy(&out.stderr);
            bail!("`purpose ask` failed: {stderr}");
        }
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    }

    /// Extract aperture (input type) tags for the unit at `symbol_name`.
    pub fn aperture(&self, symbol_name: &str) -> anyhow::Result<ApertureSet> {
        let raw = self.ask(&format!("what types does {symbol_name} accept as input"))?;
        Ok(ApertureSet(parse_tags(&raw)))
    }

    /// Extract production (output type) tags for the unit at `symbol_name`.
    pub fn production(&self, symbol_name: &str) -> anyhow::Result<ProductionSet> {
        let raw = self.ask(&format!("what types does {symbol_name} return or produce"))?;
        Ok(ProductionSet(parse_tags(&raw)))
    }

    /// Estimate natural frequency ω for a symbol (proxy: mention count in index).
    pub fn freq_estimate(&self, symbol_name: &str) -> anyhow::Result<f64> {
        let raw = self.ask(&format!("how many times is {symbol_name} called or referenced"))?;
        // `purpose ask` returns lines like "file:line: …"
        // Count the number of result lines as a rough call-count proxy.
        let count = raw.lines().filter(|l| !l.trim().is_empty()).count();
        // Normalise to [0.1, 10.0] so no unit has exactly ω=0.
        Ok(0.1 + (count as f64).ln_1p())
    }

    /// Build a `SystemGraph` by reading `.purpose/index.json` and querying
    /// the purpose tool for each top-level symbol.
    ///
    /// Edges are inferred from callee relationships returned by the index.
    pub fn build_graph(&self) -> anyhow::Result<SystemGraph> {
        let index_path = self.project_dir.join(".purpose").join("index.json");
        if !index_path.exists() {
            bail!(
                "No purpose index found at {:?}. Run `purpose index` first.",
                index_path
            );
        }
        let raw = std::fs::read_to_string(&index_path)
            .with_context(|| format!("reading {:?}", index_path))?;

        // The index is either a JSON array of entries or a JSON object with
        // an "entries" / "symbols" key — handle both shapes.
        let entries: Vec<IndexEntry> = {
            let v: serde_json::Value = serde_json::from_str(&raw)
                .with_context(|| "parsing purpose index")?;
            if v.is_array() {
                serde_json::from_value(v)?
            } else if let Some(arr) = v.get("entries").or_else(|| v.get("symbols")) {
                serde_json::from_value(arr.clone())?
            } else {
                bail!("Unrecognised purpose index format");
            }
        };

        // Deduplicate by name — keep the first occurrence.
        let mut seen = std::collections::HashSet::new();
        let mut units = Vec::new();
        for entry in &entries {
            if seen.insert(entry.name.clone()) {
                let aperture   = self.aperture(&entry.name).unwrap_or_default();
                let production = self.production(&entry.name).unwrap_or_default();
                let freq_est   = self.freq_estimate(&entry.name).unwrap_or(1.0);
                units.push(Unit {
                    id: UnitId::new(&entry.name),
                    aperture,
                    production,
                    freq_est,
                });
            }
        }

        // Infer edges: for each unit, ask what it calls.
        let mut edges = Vec::new();
        for unit in &units {
            let raw = self.ask(&format!("what functions or symbols does {} call", unit.id.as_str()))
                .unwrap_or_default();
            for callee in parse_tags(&raw) {
                if units.iter().any(|u| u.id.as_str() == callee) {
                    edges.push(Edge {
                        from: unit.id.clone(),
                        to:   UnitId::new(callee),
                    });
                }
            }
        }

        Ok(SystemGraph::new(units, edges))
    }
}

/// Parse a `purpose ask` response into a list of type/symbol tags.
///
/// `purpose ask` returns lines of the form `file:line: symbol_or_type`.
/// We extract the part after the last colon+space and deduplicate.
fn parse_tags(raw: &str) -> Vec<String> {
    let mut tags: Vec<String> = raw
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() { return None; }
            // Take the text after the last ": " if the line looks like "file:line: text".
            if let Some(pos) = line.rfind(": ") {
                let tag = line[pos + 2..].trim().to_owned();
                if !tag.is_empty() { Some(tag) } else { None }
            } else {
                Some(line.to_owned())
            }
        })
        .collect();
    tags.dedup();
    tags
}


/// Convenience: build a `SystemGraph` from a project directory, running
/// `purpose index` first if requested.
pub fn graph_from_project(
    purpose_exe: &Path,
    project_dir: &Path,
    run_index: bool,
) -> anyhow::Result<SystemGraph> {
    let client = PurposeClient::new(purpose_exe, project_dir);
    if run_index {
        client.build_index()?;
    }
    client.build_graph()
}
