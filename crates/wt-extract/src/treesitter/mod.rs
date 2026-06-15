pub mod js;
pub mod python;
pub mod rust;

use std::path::Path;

use anyhow::{bail, Result};

use crate::{Extractor, UnitData};

/// Dispatches to the correct language grammar by file extension.
pub struct TreeSitterExtractor;

impl Extractor for TreeSitterExtractor {
    fn extract(&self, path: &Path) -> Result<Vec<UnitData>> {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        match ext.as_str() {
            "rs" => rust::extract_file(path),
            "py" => python::extract_file(path),
            "js" | "mjs" | "cjs" => js::extract_file(path, false),
            "ts" | "mts" | "cts" => js::extract_file(path, true),
            "tsx" => js::extract_file(path, true),
            other => bail!("tree-sitter: unsupported extension .{other}"),
        }
    }
}
