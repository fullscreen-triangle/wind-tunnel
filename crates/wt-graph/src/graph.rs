use std::collections::{HashMap, HashSet, VecDeque};
use serde::{Deserialize, Serialize};

/// Newtype so unit identifiers are never confused with raw strings.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UnitId(pub String);

impl UnitId {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for UnitId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Categorical type tags describing what a unit accepts (input types).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ApertureSet(pub Vec<String>);

/// Categorical type tags describing what a unit produces (output types).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProductionSet(pub Vec<String>);

impl ApertureSet {
    pub fn tags(&self) -> &[String] { &self.0 }
}

impl ProductionSet {
    pub fn tags(&self) -> &[String] { &self.0 }
}

/// A single unit in the dependency graph.
/// Corresponds to Definition [Unit] in the paper.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Unit {
    pub id:         UnitId,
    /// Input type set A_u
    pub aperture:   ApertureSet,
    /// Output type set P_u
    pub production: ProductionSet,
    /// Estimated natural frequency ω_u (proxy: normalised call-count or LOC)
    pub freq_est:   f64,
}

impl Unit {
    pub fn new(
        id: impl Into<String>,
        aperture: Vec<String>,
        production: Vec<String>,
        freq_est: f64,
    ) -> Self {
        Self {
            id: UnitId::new(id),
            aperture: ApertureSet(aperture),
            production: ProductionSet(production),
            freq_est,
        }
    }
}

/// A directed dependency edge u_i → u_j.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub from: UnitId,
    pub to:   UnitId,
}

impl Edge {
    pub fn new(from: impl Into<String>, to: impl Into<String>) -> Self {
        Self {
            from: UnitId::new(from),
            to:   UnitId::new(to),
        }
    }
}

/// The system dependency graph G = (V, E).
/// Corresponds to Definition [System Graph] in the paper.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemGraph {
    pub units: Vec<Unit>,
    pub edges: Vec<Edge>,
}

impl SystemGraph {
    pub fn new(units: Vec<Unit>, edges: Vec<Edge>) -> Self {
        Self { units, edges }
    }

    /// Look up a unit by id, O(n).
    pub fn unit(&self, id: &UnitId) -> Option<&Unit> {
        self.units.iter().find(|u| &u.id == id)
    }

    /// All unit ids.
    pub fn ids(&self) -> Vec<&UnitId> {
        self.units.iter().map(|u| &u.id).collect()
    }

    /// Outgoing neighbours of `id`.
    pub fn neighbours(&self, id: &UnitId) -> Vec<&UnitId> {
        self.edges
            .iter()
            .filter(|e| &e.from == id)
            .map(|e| &e.to)
            .collect()
    }

    /// Incoming neighbours of `id`.
    pub fn predecessors(&self, id: &UnitId) -> Vec<&UnitId> {
        self.edges
            .iter()
            .filter(|e| &e.to == id)
            .map(|e| &e.from)
            .collect()
    }

    /// All directed cycles up to `max_len` nodes (Johnson's algorithm).
    /// Returns each cycle as an ordered Vec of UnitIds (first == last omitted).
    pub fn cycles(&self, max_len: usize) -> Vec<Vec<UnitId>> {
        crate::cycles::johnson_cycles(self, max_len)
    }

    /// BFS-based weakly-connected components.
    pub fn connected_components(&self) -> Vec<Vec<UnitId>> {
        let mut adj: HashMap<&UnitId, Vec<&UnitId>> = HashMap::new();
        for u in &self.units {
            adj.entry(&u.id).or_default();
        }
        for e in &self.edges {
            adj.entry(&e.from).or_default().push(&e.to);
            adj.entry(&e.to).or_default().push(&e.from);
        }

        let mut visited: HashSet<&UnitId> = HashSet::new();
        let mut components = Vec::new();

        for id in self.units.iter().map(|u| &u.id) {
            if visited.contains(id) { continue; }
            let mut queue = VecDeque::new();
            let mut comp = Vec::new();
            queue.push_back(id);
            visited.insert(id);
            while let Some(cur) = queue.pop_front() {
                comp.push(cur.clone());
                for nb in adj.get(cur).into_iter().flatten() {
                    if !visited.contains(nb) {
                        visited.insert(nb);
                        queue.push_back(nb);
                    }
                }
            }
            components.push(comp);
        }
        components
    }

    /// Induced subgraph over the given unit ids.
    pub fn subgraph(&self, ids: &[UnitId]) -> SystemGraph {
        let id_set: HashSet<&UnitId> = ids.iter().collect();
        let units = self.units.iter()
            .filter(|u| id_set.contains(&u.id))
            .cloned()
            .collect();
        let edges = self.edges.iter()
            .filter(|e| id_set.contains(&e.from) && id_set.contains(&e.to))
            .cloned()
            .collect();
        SystemGraph { units, edges }
    }

    /// Number of edges (|E|), used in tension / friction formulae.
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    pub fn to_json(&self) -> anyhow::Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    pub fn from_json(s: &str) -> anyhow::Result<Self> {
        Ok(serde_json::from_str(s)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn three_cycle() -> SystemGraph {
        SystemGraph::new(
            vec![
                Unit::new("u1", vec!["int".into()], vec!["int".into()], 1.0),
                Unit::new("u2", vec!["int".into()], vec!["int".into()], 1.0),
                Unit::new("u3", vec!["int".into()], vec!["int".into()], 1.0),
            ],
            vec![
                Edge::new("u1", "u2"),
                Edge::new("u2", "u3"),
                Edge::new("u3", "u1"),
            ],
        )
    }

    #[test]
    fn neighbours_correct() {
        let g = three_cycle();
        let u1 = UnitId::new("u1");
        let nb: Vec<_> = g.neighbours(&u1).into_iter().map(|id| id.as_str()).collect();
        assert_eq!(nb, vec!["u2"]);
    }

    #[test]
    fn one_component() {
        let g = three_cycle();
        let comps = g.connected_components();
        assert_eq!(comps.len(), 1);
        assert_eq!(comps[0].len(), 3);
    }

    #[test]
    fn subgraph_isolates_nodes() {
        let g = three_cycle();
        let sub = g.subgraph(&[UnitId::new("u1"), UnitId::new("u2")]);
        assert_eq!(sub.units.len(), 2);
        assert_eq!(sub.edges.len(), 1); // only u1→u2
    }

    #[test]
    fn json_roundtrip() {
        let g = three_cycle();
        let json = g.to_json().unwrap();
        let g2 = SystemGraph::from_json(&json).unwrap();
        assert_eq!(g2.units.len(), 3);
        assert_eq!(g2.edges.len(), 3);
    }
}
