//! Handles both JavaScript and TypeScript via the respective grammars.
//!
//! When `typescript = true`, tree-sitter-typescript is used, which parses `.ts`/`.tsx` files
//! and exposes `: Type` parameter annotations and `: ReturnType` return annotations.
//! For plain JS those fields are absent but the visitor degrades gracefully.

use std::path::Path;

use anyhow::Result;
use tree_sitter::{Node, Parser};
use wt_graph::{ApertureSet, Edge, ProductionSet, UnitId};

use crate::UnitData;

pub fn extract_file(path: &Path, typescript: bool) -> Result<Vec<UnitData>> {
    let src = std::fs::read_to_string(path)?;
    let mut parser = Parser::new();

    if typescript {
        parser.set_language(&tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())?;
    } else {
        parser.set_language(&tree_sitter_javascript::LANGUAGE.into())?;
    }

    let tree = parser
        .parse(src.as_bytes(), None)
        .ok_or_else(|| anyhow::anyhow!("tree-sitter failed to parse {}", path.display()))?;

    let root = tree.root_node();
    let mut units: Vec<UnitData> = Vec::new();
    collect_functions(&root, src.as_bytes(), &mut units);

    let n = units.len().max(1) as f64;
    for u in &mut units {
        u.freq_est = 1.0 / n;
    }

    let ids: Vec<UnitId> = units.iter().map(|u| u.id.clone()).collect();
    for u in &mut units {
        u.calls.retain(|e| ids.contains(&e.to));
    }

    Ok(units)
}

fn collect_functions(node: &Node, src: &[u8], out: &mut Vec<UnitData>) {
    match node.kind() {
        // `function foo() {}`
        "function_declaration" => {
            if let Some(unit) = parse_named_fn(node, src) {
                out.push(unit);
            }
        }
        // `const foo = function() {}` or `const foo = () => {}`
        "lexical_declaration" | "variable_declaration" => {
            let mut cursor = node.walk();
            for declarator in node.children(&mut cursor) {
                if declarator.kind() == "variable_declarator" {
                    if let Some(unit) = parse_variable_declarator(&declarator, src) {
                        out.push(unit);
                    }
                }
            }
        }
        // `method_definition` inside class body
        "method_definition" => {
            if let Some(unit) = parse_method(node, src) {
                out.push(unit);
            }
        }
        _ => {}
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_functions(&child, src, out);
    }
}

fn parse_named_fn(node: &Node, src: &[u8]) -> Option<UnitData> {
    let name_node = node.child_by_field_name("name")?;
    let name = name_node.utf8_text(src).ok()?.to_string();
    build_unit(name, node, src)
}

fn parse_variable_declarator(node: &Node, src: &[u8]) -> Option<UnitData> {
    let name_node = node.child_by_field_name("name")?;
    let name = name_node.utf8_text(src).ok()?.to_string();

    let value = node.child_by_field_name("value")?;
    match value.kind() {
        "function" | "arrow_function" => build_unit(name, &value, src),
        _ => None,
    }
}

fn parse_method(node: &Node, src: &[u8]) -> Option<UnitData> {
    let name_node = node.child_by_field_name("name")?;
    let name = name_node.utf8_text(src).ok()?.to_string();
    build_unit(name, node, src)
}

/// Common builder: extract params, return type, and call edges from a function-like node.
fn build_unit(name: String, node: &Node, src: &[u8]) -> Option<UnitData> {
    let params_node = node.child_by_field_name("parameters");
    let aperture = ApertureSet(
        params_node
            .map(|p| collect_param_types(&p, src))
            .unwrap_or_default(),
    );

    // TypeScript `: ReturnType` — lives in `return_type` field
    let ret_node = node.child_by_field_name("return_type");
    let production = ProductionSet(
        ret_node
            .map(|r| vec![strip_colon(r.utf8_text(src).unwrap_or("any")).to_string()])
            .unwrap_or_else(|| vec!["any".to_string()]),
    );

    let body = node.child_by_field_name("body");
    let calls: Vec<Edge> = body
        .map(|b| collect_calls(&b, src, &UnitId::new(&name)))
        .unwrap_or_default();

    Some(UnitData {
        id: UnitId::new(&name),
        aperture,
        production,
        freq_est: 0.0,
        calls,
    })
}

/// `": number"` → `"number"` (tree-sitter includes the colon in the return_type text).
fn strip_colon(s: &str) -> &str {
    let s = s.trim();
    s.strip_prefix(':').map(|t| t.trim()).unwrap_or(s)
}

fn collect_param_types(params: &Node, src: &[u8]) -> Vec<String> {
    let mut types = Vec::new();
    let mut cursor = params.walk();
    for child in params.children(&mut cursor) {
        match child.kind() {
            // TS: `required_parameter` has a `type` field  `x: number`
            "required_parameter" | "optional_parameter" => {
                if let Some(ty) = child.child_by_field_name("type") {
                    let t = strip_colon(ty.utf8_text(src).unwrap_or("any"));
                    types.push(t.to_string());
                } else {
                    types.push("any".to_string());
                }
            }
            // Plain JS identifier parameter
            "identifier" => {
                types.push("any".to_string());
            }
            // assignment_pattern: `x = default`
            "assignment_pattern" => {
                types.push("any".to_string());
            }
            // rest_pattern: `...args`
            "rest_pattern" => {
                types.push("...any".to_string());
            }
            _ => {}
        }
    }
    types
}

fn collect_calls(body: &Node, src: &[u8], from: &UnitId) -> Vec<Edge> {
    let mut edges = Vec::new();
    visit_calls(body, src, from, &mut edges);
    edges
}

fn visit_calls(node: &Node, src: &[u8], from: &UnitId, out: &mut Vec<Edge>) {
    if node.kind() == "call_expression" {
        if let Some(func) = node.child_by_field_name("function") {
            if let Some(name) = leaf_name(&func, src) {
                out.push(Edge {
                    from: from.clone(),
                    to: UnitId::new(&name),
                });
            }
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        visit_calls(&child, src, from, out);
    }
}

fn leaf_name(node: &Node, src: &[u8]) -> Option<String> {
    match node.kind() {
        "identifier" => Some(node.utf8_text(src).ok()?.to_string()),
        "member_expression" => {
            // `obj.method` — take the property
            if let Some(prop) = node.child_by_field_name("property") {
                return leaf_name(&prop, src);
            }
            None
        }
        _ => {
            let mut cursor = node.walk();
            node.children(&mut cursor)
                .filter_map(|c| leaf_name(&c, src))
                .last()
        }
    }
}
