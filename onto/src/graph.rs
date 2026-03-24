use crate::node::Node;
use serde::Serialize;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Serialize)]
pub struct GraphEdge {
    pub from: String,
    pub to: String,
    pub edge_type: EdgeType,
}

#[derive(Debug, Serialize)]
pub enum EdgeType {
    Ref,       // explicit refs in frontmatter
    Wikilink,  // inline [[wikilink]]
    Tag,       // shared tag connection
}

#[derive(Debug)]
pub struct Graph {
    /// node name -> node
    pub nodes: HashMap<String, Node>,
    /// node name -> set of connected node names
    pub adjacency: HashMap<String, HashSet<String>>,
    /// tag -> set of node names
    pub tag_index: HashMap<String, HashSet<String>>,
}

impl Graph {
    pub fn build(nodes: &[Node]) -> Self {
        let mut node_map = HashMap::new();
        let mut adjacency: HashMap<String, HashSet<String>> = HashMap::new();
        let mut tag_index: HashMap<String, HashSet<String>> = HashMap::new();

        // Index all nodes
        for node in nodes {
            node_map.insert(node.meta.name.clone(), node.clone());
            adjacency.entry(node.meta.name.clone()).or_default();

            for tag in &node.meta.tags {
                tag_index
                    .entry(tag.clone())
                    .or_default()
                    .insert(node.meta.name.clone());
            }
        }

        // Build adjacency from refs
        for node in nodes {
            for ref_name in node.all_refs() {
                if node_map.contains_key(&ref_name) {
                    adjacency
                        .entry(node.meta.name.clone())
                        .or_default()
                        .insert(ref_name.clone());
                    adjacency
                        .entry(ref_name)
                        .or_default()
                        .insert(node.meta.name.clone());
                }
            }
        }

        Graph {
            nodes: node_map,
            adjacency,
            tag_index,
        }
    }

    /// Get neighbors of a node up to `depth` hops
    pub fn neighbors(&self, name: &str, depth: usize) -> Vec<(&Node, usize)> {
        let mut visited: HashSet<&str> = HashSet::new();
        let mut result = Vec::new();
        let mut frontier: Vec<(&str, usize)> = vec![(name, 0)];

        while let Some((current, d)) = frontier.pop() {
            if visited.contains(current) || d > depth {
                continue;
            }
            visited.insert(current);
            if d > 0 {
                if let Some(node) = self.nodes.get(current) {
                    result.push((node, d));
                }
            }
            if d < depth {
                if let Some(adj) = self.adjacency.get(current) {
                    for neighbor in adj {
                        if !visited.contains(neighbor.as_str()) {
                            frontier.push((neighbor.as_str(), d + 1));
                        }
                    }
                }
            }
        }

        result.sort_by_key(|(_, d)| *d);
        result
    }

    /// Find nodes by tag
    pub fn by_tag(&self, tag: &str) -> Vec<&Node> {
        self.tag_index
            .get(tag)
            .map(|names| {
                names
                    .iter()
                    .filter_map(|n| self.nodes.get(n))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get all edges for visualization/export
    pub fn edges(&self) -> Vec<GraphEdge> {
        let mut edges = Vec::new();
        let mut seen: HashSet<(String, String)> = HashSet::new();

        for node in self.nodes.values() {
            for ref_name in &node.meta.refs {
                if self.nodes.contains_key(ref_name) {
                    let key = ordered_pair(&node.meta.name, ref_name);
                    if seen.insert(key) {
                        edges.push(GraphEdge {
                            from: node.meta.name.clone(),
                            to: ref_name.clone(),
                            edge_type: EdgeType::Ref,
                        });
                    }
                }
            }
            for ref_name in node.inline_refs() {
                if self.nodes.contains_key(&ref_name)
                    && !node.meta.refs.contains(&ref_name)
                {
                    let key = ordered_pair(&node.meta.name, &ref_name);
                    if seen.insert(key) {
                        edges.push(GraphEdge {
                            from: node.meta.name.clone(),
                            to: ref_name,
                            edge_type: EdgeType::Wikilink,
                        });
                    }
                }
            }
        }
        edges
    }

    /// Validate: find broken refs (pointing to non-existent nodes)
    pub fn broken_refs(&self) -> Vec<(String, String)> {
        let mut broken = Vec::new();
        for node in self.nodes.values() {
            for ref_name in node.all_refs() {
                if !self.nodes.contains_key(&ref_name) {
                    broken.push((node.meta.name.clone(), ref_name));
                }
            }
        }
        broken
    }
}

fn ordered_pair(a: &str, b: &str) -> (String, String) {
    if a < b {
        (a.to_string(), b.to_string())
    } else {
        (b.to_string(), a.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::NodeMeta;

    fn make_node(name: &str, tags: &[&str], refs: &[&str], body: &str) -> Node {
        Node {
            meta: NodeMeta {
                name: name.to_string(),
                category: "test".to_string(),
                tags: tags.iter().map(|s| s.to_string()).collect(),
                refs: refs.iter().map(|s| s.to_string()).collect(),
                created: None,
                updated: None,
            },
            body: body.to_string(),
            path: None,
        }
    }

    #[test]
    fn test_graph_build() {
        let nodes = vec![
            make_node("a", &["x"], &["b"], ""),
            make_node("b", &["x", "y"], &[], "see [[c]]"),
            make_node("c", &["y"], &[], ""),
        ];
        let g = Graph::build(&nodes);
        assert_eq!(g.adjacency["a"].len(), 1); // b
        assert_eq!(g.adjacency["b"].len(), 2); // a, c
        assert_eq!(g.adjacency["c"].len(), 1); // b
    }

    #[test]
    fn test_neighbors() {
        let nodes = vec![
            make_node("a", &[], &["b"], ""),
            make_node("b", &[], &["c"], ""),
            make_node("c", &[], &[], ""),
        ];
        let g = Graph::build(&nodes);
        let n1 = g.neighbors("a", 1);
        assert_eq!(n1.len(), 1);
        assert_eq!(n1[0].0.meta.name, "b");

        let n2 = g.neighbors("a", 2);
        assert_eq!(n2.len(), 2);
    }

    #[test]
    fn test_broken_refs() {
        let nodes = vec![make_node("a", &[], &["nonexistent"], "see [[also-missing]]")];
        let g = Graph::build(&nodes);
        let broken = g.broken_refs();
        assert_eq!(broken.len(), 2);
    }

    #[test]
    fn test_by_tag() {
        let nodes = vec![
            make_node("a", &["shared"], &[], ""),
            make_node("b", &["shared", "extra"], &[], ""),
            make_node("c", &["other"], &[], ""),
        ];
        let g = Graph::build(&nodes);
        assert_eq!(g.by_tag("shared").len(), 2);
        assert_eq!(g.by_tag("other").len(), 1);
        assert_eq!(g.by_tag("nope").len(), 0);
    }
}
