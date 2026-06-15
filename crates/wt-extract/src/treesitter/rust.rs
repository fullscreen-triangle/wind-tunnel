use std::path::Path;

use anyhow::Result;
use tree_sitter::{Node, Parser};
use wt_graph::{ApertureSet, Edge, ProductionSet, UnitId};

use crate::UnitData;

pub fn extract_file(path: &Path) -> Result<Vec<UnitData>> {
    let src = std::fs::read_to_string(path)?;
    let mut parser = Parser::new();
    parser.set_language(&tree_sitter_rust::LANGUAGE.into())?;
    let tree = parser
        .parse(src.as_bytes(), None)
        .ok_or_else(|| anyhow::anyhow!("tree-sitter failed to parse {}", path.display()))?;

    let root = tree.root_node();
    let mut units: Vec<UnitData> = Vec::new();
    collect_functions(&root, src.as_bytes(), &mut units);

    // freq_est: uniform across functions in the file.
    let n = units.len().max(1) as f64;
    for u in &mut units {
        u.freq_est = 1.0 / n;
    }

    // Resolve call edges: for each unit whose callee name matches another unit in the file.
    let ids: Vec<UnitId> = units.iter().map(|u| u.id.clone()).collect();
    for u in &mut units {
        u.calls.retain(|e| ids.contains(&e.to));
    }

    Ok(units)
}

fn collect_functions(node: &Node, src: &[u8], out: &mut Vec<UnitData>) {
    match node.kind() {
        "function_item" | "impl_item" => {
            if node.kind() == "function_item" {
                if let Some(unit) = parse_fn(node, src) {
                    out.push(unit);
                }
            }
            // descend into impl blocks to find their methods
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                collect_functions(&child, src, out);
            }
        }
        _ => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                collect_functions(&child, src, out);
            }
        }
    }
}

fn parse_fn(node: &Node, src: &[u8]) -> Option<UnitData> {
    // name
    let name_node = node.child_by_field_name("name")?;
    let name = name_node.utf8_text(src).ok()?.to_string();

    // parameters → aperture
    let params_node = node.child_by_field_name("parameters");
    let aperture = ApertureSet(
        params_node
            .map(|p| collect_types(&p, src, "parameter"))
            .unwrap_or_default(),
    );

    // return_type → production
    let ret_node = node.child_by_field_name("return_type");
    let production = ProductionSet(
        ret_node
            .map(|r| vec![r.utf8_text(src).unwrap_or("()").to_string()])
            .unwrap_or_else(|| vec!["()".to_string()]),
    );

    // body → call_expression sites
    let body = node.child_by_field_name("body");
    let calls: Vec<Edge> = body
        .map(|b| collect_calls(&b, src, &UnitId::new(&name)))
        .unwrap_or_default();

    Some(UnitData {
        id: UnitId::new(&name),
        aperture,
        production,
        freq_est: 0.0, // set by caller
        calls,
    })
}

/// Walk a `parameters` node and extract the type annotation of each parameter.
fn collect_types(params: &Node, src: &[u8], _kind_hint: &str) -> Vec<String> {
    let mut types = Vec::new();
    let mut cursor = params.walk();
    for child in params.children(&mut cursor) {
        if child.kind() == "parameter" || child.kind() == "self_parameter" {
            if let Some(ty) = child.child_by_field_name("type") {
                if let Ok(t) = ty.utf8_text(src) {
                    types.push(t.to_string());
                }
            }
        }
    }
    types
}

/// Walk a function body and collect every call_expression callee name.
fn collect_calls(body: &Node, src: &[u8], from: &UnitId) -> Vec<Edge> {
    let mut edges = Vec::new();
    visit_calls(body, src, from, &mut edges);
    edges
}

fn visit_calls(node: &Node, src: &[u8], from: &UnitId, out: &mut Vec<Edge>) {
    if node.kind() == "call_expression" {
        if let Some(func) = node.child_by_field_name("function") {
            // e.g. `foo(...)` or `self.foo(...)` or `Foo::bar(...)`
            let callee = leaf_name(&func, src);
            if let Some(name) = callee {
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

/// Extract the rightmost identifier in a possibly-qualified path.
fn leaf_name(node: &Node, src: &[u8]) -> Option<String> {
    match node.kind() {
        "identifier" => Some(node.utf8_text(src).ok()?.to_string()),
        "scoped_identifier" | "field_expression" | "method_call_expression" => {
            // take the `name` field if present, otherwise last child identifier
            if let Some(n) = node.child_by_field_name("name") {
                return leaf_name(&n, src);
            }
            let mut cursor = node.walk();
            node.children(&mut cursor)
                .filter_map(|c| leaf_name(&c, src))
                .last()
        }
        _ => {
            let mut cursor = node.walk();
            node.children(&mut cursor)
                .filter_map(|c| leaf_name(&c, src))
                .last()
        }
    }
}
