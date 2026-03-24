use crate::graph::Graph;
use crate::store::Store;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Serialize)]
pub struct RecallResult {
    pub nodes: Vec<RecalledNode>,
    pub total_candidates: usize,
}

#[derive(Debug, Serialize)]
pub struct RecalledNode {
    pub name: String,
    pub category: String,
    pub tags: Vec<String>,
    pub body: String,
    pub score: f64,
    pub reason: String,
}

/// Recall relevant nodes given a context string.
/// This is the core "associative memory" function.
///
/// Scoring strategy:
/// 1. Tokenize context into keywords
/// 2. For each node, compute relevance score:
///    - Name match: high weight
///    - Tag match: medium weight
///    - Ref match: medium weight (the node references or is referenced by matched nodes)
///    - Body keyword overlap: proportional to unique keyword hits
/// 3. Graph proximity boost: nodes connected to high-scoring nodes get a bonus
/// 4. Return top-N sorted by score
pub fn recall(
    store: &Store,
    context: &str,
    max_nodes: usize,
) -> Result<RecallResult, crate::store::StoreError> {
    let nodes = store.load_all()?;
    let graph = Graph::build(&nodes);
    let keywords = tokenize(context);

    if keywords.is_empty() || nodes.is_empty() {
        return Ok(RecallResult {
            nodes: vec![],
            total_candidates: nodes.len(),
        });
    }

    // Phase 1: Direct scoring
    let mut scores: HashMap<String, (f64, Vec<String>)> = HashMap::new();

    for node in &nodes {
        let mut score = 0.0f64;
        let mut reasons = Vec::new();

        // Name match
        let name_lower = node.meta.name.to_lowercase();
        let name_hits: usize = keywords
            .iter()
            .filter(|kw| name_lower.contains(kw.as_str()))
            .count();
        if name_hits > 0 {
            score += 5.0 * name_hits as f64;
            reasons.push(format!("name({})", name_hits));
        }

        // Tag match
        let tag_hits: usize = node
            .meta
            .tags
            .iter()
            .filter(|t| {
                let tl = t.to_lowercase();
                keywords.iter().any(|kw| tl.contains(kw.as_str()))
            })
            .count();
        if tag_hits > 0 {
            score += 3.0 * tag_hits as f64;
            reasons.push(format!("tag({})", tag_hits));
        }

        // Body keyword overlap
        let body_lower = node.body.to_lowercase();
        let body_hits: usize = keywords
            .iter()
            .filter(|kw| body_lower.contains(kw.as_str()))
            .count();
        if body_hits > 0 {
            let ratio = body_hits as f64 / keywords.len() as f64;
            score += 2.0 * ratio;
            reasons.push(format!("body({}/{})", body_hits, keywords.len()));
        }

        if score > 0.0 {
            scores.insert(node.meta.name.clone(), (score, reasons));
        }
    }

    // Phase 2: Graph proximity boost
    let high_scorers: Vec<String> = scores
        .iter()
        .filter(|(_, (s, _))| *s >= 3.0)
        .map(|(name, _)| name.clone())
        .collect();

    for name in &high_scorers {
        let neighbors = graph.neighbors(name, 2);
        for (neighbor, depth) in neighbors {
            let boost = 1.5 / depth as f64;
            let entry = scores
                .entry(neighbor.meta.name.clone())
                .or_insert((0.0, vec![]));
            entry.0 += boost;
            entry
                .1
                .push(format!("proximity({}→{},d={})", name, neighbor.meta.name, depth));
        }
    }

    // Sort and take top-N
    let mut ranked: Vec<_> = scores.into_iter().collect();
    ranked.sort_by(|a, b| b.1 .0.partial_cmp(&a.1 .0).unwrap());
    ranked.truncate(max_nodes);

    let recalled: Vec<RecalledNode> = ranked
        .into_iter()
        .filter_map(|(name, (score, reasons))| {
            graph.nodes.get(&name).map(|node| RecalledNode {
                name: node.meta.name.clone(),
                category: node.meta.category.clone(),
                tags: node.meta.tags.clone(),
                body: node.body.clone(),
                score,
                reason: reasons.join(", "),
            })
        })
        .collect();

    Ok(RecallResult {
        total_candidates: nodes.len(),
        nodes: recalled,
    })
}

fn tokenize(text: &str) -> Vec<String> {
    let stop_words: &[&str] = &[
        "the", "a", "an", "is", "are", "was", "were", "be", "been", "being", "have", "has",
        "had", "do", "does", "did", "will", "would", "could", "should", "may", "might", "can",
        "shall", "to", "of", "in", "for", "on", "with", "at", "by", "from", "as", "into",
        "about", "and", "or", "not", "this", "that", "it", "its",
        // Korean particles
        "의", "에", "를", "을", "이", "가", "은", "는", "로", "으로", "와", "과", "도",
        "에서", "까지", "부터",
    ];
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric() && c != '-' && c != '_')
        .filter(|w| w.len() >= 2 && !stop_words.contains(w))
        .map(|w| w.to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::{Node, NodeMeta};
    use tempfile::TempDir;

    fn setup() -> (TempDir, Store) {
        let dir = TempDir::new().unwrap();
        let store = Store::new(dir.path());

        let nodes = vec![
            Node {
                meta: NodeMeta {
                    name: "auth-flow".to_string(),
                    category: "domain".to_string(),
                    tags: vec!["auth".to_string(), "security".to_string()],
                    refs: vec!["user-model".to_string()],
                    created: None,
                    updated: None,
                },
                body: "JWT-based authentication flow. Handles login, logout, token refresh."
                    .to_string(),
                path: None,
            },
            Node {
                meta: NodeMeta {
                    name: "user-model".to_string(),
                    category: "domain".to_string(),
                    tags: vec!["model".to_string(), "core".to_string()],
                    refs: vec![],
                    created: None,
                    updated: None,
                },
                body: "User entity with email, roles, permissions. Central to auth system."
                    .to_string(),
                path: None,
            },
            Node {
                meta: NodeMeta {
                    name: "deploy-pipeline".to_string(),
                    category: "workflow".to_string(),
                    tags: vec!["ci".to_string(), "deploy".to_string()],
                    refs: vec![],
                    created: None,
                    updated: None,
                },
                body: "GitHub Actions pipeline. Build, test, deploy to staging then production."
                    .to_string(),
                path: None,
            },
        ];

        for n in &nodes {
            store.upsert(n).unwrap();
        }
        (dir, store)
    }

    #[test]
    fn test_recall_auth_context() {
        let (_dir, store) = setup();
        let result = recall(&store, "authentication login security", 5).unwrap();
        assert!(!result.nodes.is_empty());
        assert_eq!(result.nodes[0].name, "auth-flow");
    }

    #[test]
    fn test_recall_proximity_boost() {
        let (_dir, store) = setup();
        // "auth" should match auth-flow directly, and user-model should get proximity boost
        let result = recall(&store, "auth token", 5).unwrap();
        let names: Vec<&str> = result.nodes.iter().map(|n| n.name.as_str()).collect();
        assert!(names.contains(&"auth-flow"));
        assert!(names.contains(&"user-model")); // proximity boost
    }

    #[test]
    fn test_recall_empty_context() {
        let (_dir, store) = setup();
        let result = recall(&store, "", 5).unwrap();
        assert!(result.nodes.is_empty());
    }

    #[test]
    fn test_tokenize() {
        let tokens = tokenize("JWT authentication for the user login");
        assert!(tokens.contains(&"jwt".to_string()));
        assert!(tokens.contains(&"authentication".to_string()));
        assert!(tokens.contains(&"user".to_string()));
        assert!(tokens.contains(&"login".to_string()));
        assert!(!tokens.contains(&"the".to_string()));
        assert!(!tokens.contains(&"for".to_string()));
    }
}
