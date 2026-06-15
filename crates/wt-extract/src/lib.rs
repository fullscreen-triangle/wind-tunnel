pub mod hf;
pub mod treesitter;

use std::path::Path;

use anyhow::Result;
use wt_graph::{ApertureSet, Edge, ProductionSet, UnitId};

/// Concrete behavioural data extracted from a single source unit.
#[derive(Debug, Clone)]
pub struct UnitData {
    pub id: UnitId,
    /// Concrete input types (aperture).
    pub aperture: ApertureSet,
    /// Concrete return/output types (production).
    pub production: ProductionSet,
    /// Estimated call frequency relative to total calls in file (0..1).
    pub freq_est: f64,
    /// Outgoing call edges discovered inside this unit's body.
    pub calls: Vec<Edge>,
}

/// Backend-agnostic extraction interface.
pub trait Extractor {
    /// Extract all units from the file at `path`.
    fn extract(&self, path: &Path) -> Result<Vec<UnitData>>;
}

/// Which extraction backend to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    /// AST-based, exact.
    TreeSitter,
    /// Model-based, probabilistic.
    Hf,
    /// Try TreeSitter first; fall back to Hf on parse failure or empty result.
    Auto,
}

impl Default for Backend {
    fn default() -> Self {
        Backend::Auto
    }
}

impl std::str::FromStr for Backend {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self> {
        match s {
            "treesitter" | "tree-sitter" | "ts" => Ok(Backend::TreeSitter),
            "hf" | "huggingface" => Ok(Backend::Hf),
            "auto" => Ok(Backend::Auto),
            other => anyhow::bail!("unknown backend: {other}; expected treesitter | hf | auto"),
        }
    }
}

/// Run extraction against a file using the chosen backend.
pub fn extract(path: &Path, backend: Backend) -> Result<Vec<UnitData>> {
    let ts = treesitter::TreeSitterExtractor;
    let hf_client = hf::HfExtractor::from_env();

    match backend {
        Backend::TreeSitter => ts.extract(path),
        Backend::Hf => hf_client.extract(path),
        Backend::Auto => {
            let result = ts.extract(path);
            match result {
                Ok(units) if !units.is_empty() => Ok(units),
                _ => hf_client.extract(path),
            }
        }
    }
}
