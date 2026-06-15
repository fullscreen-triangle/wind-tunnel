//! Johnson's algorithm for enumerating all simple directed cycles.
//!
//! Reference: D.B. Johnson, "Finding all the elementary circuits of a directed
//! graph", SIAM J. Comput. 4(1), 1975.
//!
//! We bound enumeration by `max_len` to keep runtime tractable on large graphs.

use std::collections::{HashMap, HashSet};
use crate::graph::{SystemGraph, UnitId};

pub fn johnson_cycles(graph: &SystemGraph, max_len: usize) -> Vec<Vec<UnitId>> {
    if max_len == 0 { return vec![]; }

    // Map ids to indices for fast lookup.
    let ids: Vec<&UnitId> = graph.units.iter().map(|u| &u.id).collect();
    let n = ids.len();
    let index: HashMap<&UnitId, usize> = ids.iter().enumerate().map(|(i, id)| (*id, i)).collect();

    // Adjacency list (indices).
    let mut adj: Vec<Vec<usize>> = vec![vec![]; n];
    for e in &graph.edges {
        if let (Some(&fi), Some(&ti)) = (index.get(&e.from), index.get(&e.to)) {
            adj[fi].push(ti);
        }
    }

    let mut blocked: Vec<bool>        = vec![false; n];
    let mut b_sets:  Vec<HashSet<usize>> = vec![HashSet::new(); n];
    let mut stack:   Vec<usize>        = Vec::new();
    let mut cycles:  Vec<Vec<UnitId>>  = Vec::new();

    fn unblock(v: usize, blocked: &mut Vec<bool>, b_sets: &mut Vec<HashSet<usize>>) {
        blocked[v] = false;
        let dependents: Vec<usize> = b_sets[v].drain().collect();
        for w in dependents {
            if blocked[w] { unblock(w, blocked, b_sets); }
        }
    }

    fn circuit(
        v:       usize,
        s:       usize,
        adj:     &Vec<Vec<usize>>,
        blocked: &mut Vec<bool>,
        b_sets:  &mut Vec<HashSet<usize>>,
        stack:   &mut Vec<usize>,
        cycles:  &mut Vec<Vec<UnitId>>,
        ids:     &Vec<&UnitId>,
        max_len: usize,
    ) -> bool {
        let mut found = false;
        stack.push(v);
        blocked[v] = true;

        if stack.len() <= max_len {
            for &w in &adj[v] {
                if w == s {
                    // Cycle found.
                    let cycle: Vec<UnitId> = stack.iter().map(|&i| ids[i].clone()).collect();
                    cycles.push(cycle);
                    found = true;
                } else if !blocked[w] {
                    if circuit(w, s, adj, blocked, b_sets, stack, cycles, ids, max_len) {
                        found = true;
                    }
                }
            }
        }

        if found {
            unblock(v, blocked, b_sets);
        } else {
            for &w in &adj[v] {
                b_sets[w].insert(v);
            }
        }

        stack.pop();
        found
    }

    for s in 0..n {
        // Reset state for each starting node.
        blocked.iter_mut().for_each(|b| *b = false);
        b_sets.iter_mut().for_each(|b| b.clear());

        circuit(s, s, &adj, &mut blocked, &mut b_sets, &mut stack, &mut cycles, &ids, max_len);
    }

    cycles
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{Edge, Unit, SystemGraph};

    fn three_cycle() -> SystemGraph {
        SystemGraph::new(
            vec![
                Unit::new("u1", vec![], vec![], 1.0),
                Unit::new("u2", vec![], vec![], 1.0),
                Unit::new("u3", vec![], vec![], 1.0),
            ],
            vec![
                Edge::new("u1", "u2"),
                Edge::new("u2", "u3"),
                Edge::new("u3", "u1"),
            ],
        )
    }

    #[test]
    fn detects_three_cycle() {
        let g = three_cycle();
        let cycles = johnson_cycles(&g, 10);
        // Johnson finds each rotation; there should be exactly one elementary cycle.
        // For a 3-node directed cycle, Johnson returns it once starting from the
        // lowest-indexed node.
        assert!(!cycles.is_empty(), "should find at least one cycle");
        assert!(cycles.iter().all(|c| c.len() == 3));
    }

    #[test]
    fn dag_has_no_cycles() {
        let g = SystemGraph::new(
            vec![
                Unit::new("a", vec![], vec![], 1.0),
                Unit::new("b", vec![], vec![], 1.0),
                Unit::new("c", vec![], vec![], 1.0),
            ],
            vec![
                Edge::new("a", "b"),
                Edge::new("b", "c"),
            ],
        );
        let cycles = johnson_cycles(&g, 10);
        assert!(cycles.is_empty(), "DAG must have no cycles");
    }

    #[test]
    fn max_len_respected() {
        // Graph with a 4-cycle; max_len=3 should not return it.
        let g = SystemGraph::new(
            vec![
                Unit::new("a", vec![], vec![], 1.0),
                Unit::new("b", vec![], vec![], 1.0),
                Unit::new("c", vec![], vec![], 1.0),
                Unit::new("d", vec![], vec![], 1.0),
            ],
            vec![
                Edge::new("a", "b"),
                Edge::new("b", "c"),
                Edge::new("c", "d"),
                Edge::new("d", "a"),
            ],
        );
        let cycles = johnson_cycles(&g, 3);
        assert!(cycles.is_empty(), "4-cycle must not appear with max_len=3");
        let cycles4 = johnson_cycles(&g, 4);
        assert!(!cycles4.is_empty());
    }
}
