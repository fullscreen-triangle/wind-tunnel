//! Hugging Face Inference API backend.
//!
//! Uses the text-generation endpoint of a code model (default: `bigcode/starcoder2-7b`)
//! to extract behavioural unit data from source files when tree-sitter cannot produce
//! an accurate parse (e.g. heavily dynamic Python, template metaprogramming, or macro-heavy Rust).
//!
//! Set `HF_TOKEN` in the environment.  Set `WT_HF_MODEL` to override the model slug.

use std::path::Path;

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use wt_graph::{ApertureSet, Edge, ProductionSet, UnitId};

use crate::UnitData;

const DEFAULT_MODEL: &str = "bigcode/starcoder2-7b";
const API_BASE: &str = "https://api-inference.huggingface.co/models";

pub struct HfExtractor {
    token: Option<String>,
    model: String,
}

impl HfExtractor {
    /// Reads `HF_TOKEN` and `WT_HF_MODEL` from the environment.
    pub fn from_env() -> Self {
        let token = std::env::var("HF_TOKEN").ok();
        let model = std::env::var("WT_HF_MODEL").unwrap_or_else(|_| DEFAULT_MODEL.to_string());
        Self { token, model }
    }

    fn url(&self) -> String {
        format!("{API_BASE}/{}", self.model)
    }

    fn bearer(&self) -> Result<String> {
        self.token
            .clone()
            .ok_or_else(|| anyhow::anyhow!("HF_TOKEN is not set; cannot use the HF backend"))
    }
}

/// The structured response we ask the model to produce (JSON mode via prompt engineering).
#[derive(Debug, Deserialize, Serialize)]
struct HfUnit {
    name: String,
    aperture: Vec<String>,
    production: Vec<String>,
    calls: Vec<String>,
}

/// Minimal HF text-generation response envelope.
#[derive(Debug, Deserialize)]
struct HfResponse {
    generated_text: String,
}

impl crate::Extractor for HfExtractor {
    fn extract(&self, path: &Path) -> Result<Vec<UnitData>> {
        let src = std::fs::read_to_string(path)?;
        let token = self.bearer()?;

        let prompt = build_prompt(&src, path);

        let body = serde_json::json!({
            "inputs": prompt,
            "parameters": {
                "max_new_tokens": 2048,
                "temperature": 0.0,
                "return_full_text": false
            }
        });

        let resp = ureq::post(&self.url())
            .header("Authorization", &format!("Bearer {token}"))
            .header("Content-Type", "application/json")
            .send_json(&body)
            .context("HF API request failed")?;

        let status = resp.status();
        if status != 200 {
            let body_text = resp
                .into_body()
                .read_to_string()
                .unwrap_or_else(|_| "<unreadable>".to_string());
            bail!("HF API returned HTTP {status}: {body_text}");
        }

        let responses: Vec<HfResponse> = resp.into_body().read_json()?;
        let raw = responses
            .into_iter()
            .next()
            .map(|r| r.generated_text)
            .unwrap_or_default();

        let hf_units: Vec<HfUnit> = parse_json_array(&raw)?;

        let n = hf_units.len().max(1) as f64;
        let units = hf_units
            .into_iter()
            .enumerate()
            .map(|(_i, u)| {
                let from = UnitId::new(&u.name);
                let calls = u
                    .calls
                    .iter()
                    .map(|callee| Edge {
                        from: from.clone(),
                        to: UnitId::new(callee),
                    })
                    .collect();
                UnitData {
                    id: from,
                    aperture: ApertureSet(u.aperture),
                    production: ProductionSet(u.production),
                    freq_est: 1.0 / n,
                    calls,
                }
            })
            .collect();

        Ok(units)
    }
}

/// Build a prompt that asks the model for a JSON array of units.
fn build_prompt(src: &str, path: &Path) -> String {
    let lang = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("unknown");

    // Truncate large files — model context is finite.
    const MAX_CHARS: usize = 6_000;
    let snippet = if src.len() > MAX_CHARS {
        &src[..MAX_CHARS]
    } else {
        &src
    };

    format!(
        r#"You are a code analysis assistant. Analyse the following {lang} source file and
extract every top-level function or method. Return ONLY a valid JSON array where each
element has exactly these fields:
  "name"       — string, the function/method name
  "aperture"   — array of strings, the concrete input parameter types (use language
                 notation; "Any" if unannotated)
  "production" — array of strings, the return type(s) ("void" if none)
  "calls"      — array of strings, names of other functions called in the body

Return nothing outside the JSON array.

```{lang}
{snippet}
```
JSON:
"#
    )
}

/// Find and deserialize the first JSON array in the model output.
fn parse_json_array(text: &str) -> Result<Vec<HfUnit>> {
    // The model may emit preamble text before the array.
    if let Some(start) = text.find('[') {
        let slice = &text[start..];
        // find matching close bracket (naïve; works for well-formed JSON)
        if let Some(end) = find_array_end(slice) {
            let json_str = &slice[..=end];
            let units: Vec<HfUnit> = serde_json::from_str(json_str)
                .context("failed to parse HF model output as JSON array of units")?;
            return Ok(units);
        }
    }
    // If no array found, return empty rather than error — caller decides fallback.
    Ok(vec![])
}

fn find_array_end(s: &str) -> Option<usize> {
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escape_next = false;
    for (i, ch) in s.char_indices() {
        if escape_next {
            escape_next = false;
            continue;
        }
        if in_string {
            match ch {
                '\\' => escape_next = true,
                '"' => in_string = false,
                _ => {}
            }
        } else {
            match ch {
                '"' => in_string = true,
                '[' | '{' => depth += 1,
                ']' | '}' => {
                    if depth == 0 {
                        return None;
                    }
                    depth -= 1;
                    if depth == 0 {
                        return Some(i);
                    }
                }
                _ => {}
            }
        }
    }
    None
}
