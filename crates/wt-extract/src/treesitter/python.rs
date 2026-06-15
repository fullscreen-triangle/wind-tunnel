use std::path::Path;

use anyhow::Result;
use tree_sitter::{Node, Parser};
use wt_graph::{ApertureSet, Edge, ProductionSet, UnitId};

use crate::UnitData;

pub fn extract_file(path: &Path) -> Result<Vec<UnitData>> {
    let src = std::fs::read_to_string(path)?;
    let mut parser = Parser::new();
    parser.set_language(&tree_sitter_python::LANGUAGE.into())?;
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
    if node.kind() == "function_definition" {
        if let Some(unit) = parse_fn(node, src) {
            out.push(unit);
        }
        // also descend for nested functions
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_functions(&child, src, out);
    }
}

fn parse_fn(node: &Node, src: &[u8]) -> Option<UnitData> {
    let name_node = node.child_by_field_name("name")?;
    let name = name_node.utf8_text(src).ok()?.to_string();

    // parameters
    let params_node = node.child_by_field_name("parameters");
    let aperture = ApertureSet(
        params_node
            .map(|p| collect_param_types(&p, src))
            .unwrap_or_default(),
    );

    // return type annotation (-> Type)
    let ret_node = node.child_by_field_name("return_type");
    let production = ProductionSet(
        ret_node
            .map(|r| vec![r.utf8_text(src).unwrap_or("Any").to_string()])
            .unwrap_or_else(|| vec!["Any".to_string()]),
    );

    // body → calls
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

fn collect_param_types(params: &Node, src: &[u8]) -> Vec<String> {
    let mut types = Vec::new();
    let mut cursor = params.walk();
    for child in params.children(&mut cursor) {
        match child.kind() {
            // typed_parameter: `x: int`
            "typed_parameter" => {
                if let Some(ty) = child.child_by_field_name("type") {
                    if let Ok(t) = ty.utf8_text(src) {
                        types.push(t.to_string());
                    }
                }
            }
            // default_parameter with type: rare but handle
            "typed_default_parameter" => {
                if let Some(ty) = child.child_by_field_name("type") {
                    if let Ok(t) = ty.utf8_text(src) {
                        types.push(t.to_string());
                    }
                }
            }
            // untyped parameter — record as "Any"
            "identifier" => {
                let txt = child.utf8_text(src).unwrap_or("_");
                if txt != "self" && txt != "cls" {
                    types.push("Any".to_string());
                }
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
    if node.kind() == "call" {
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
        "attribute" => {
            // `obj.method` — take the attribute name
            if let Some(attr) = node.child_by_field_name("attribute") {
                return leaf_name(&attr, src);
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
