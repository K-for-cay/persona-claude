use crate::node::Node;
use crate::store::Store;
use regex::Regex;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct SearchResult {
    pub name: String,
    pub category: String,
    pub tags: Vec<String>,
    pub snippet: String,
    pub match_type: String,
}

pub enum SearchBy {
    Tag,
    Ref,
    Content,
    All,
}

impl SearchBy {
    pub fn from_str(s: &str) -> Self {
        match s {
            "tag" => SearchBy::Tag,
            "ref" => SearchBy::Ref,
            "content" => SearchBy::Content,
            _ => SearchBy::All,
        }
    }
}

pub fn search(
    store: &Store,
    query: &str,
    by: SearchBy,
) -> Result<Vec<SearchResult>, crate::store::StoreError> {
    let nodes = store.load_all()?;
    let query_lower = query.to_lowercase();
    let re = Regex::new(&regex::escape(&query_lower)).ok();

    let mut results = Vec::new();

    for node in &nodes {
        let matched = match by {
            SearchBy::Tag => match_tags(node, &query_lower),
            SearchBy::Ref => match_refs(node, &query_lower),
            SearchBy::Content => match_content(node, &query_lower, &re),
            SearchBy::All => {
                match_tags(node, &query_lower)
                    .or(match_refs(node, &query_lower))
                    .or(match_content(node, &query_lower, &re))
                    .or(match_name(node, &query_lower))
            }
        };

        if let Some(match_type) = matched {
            let snippet = make_snippet(&node.body, &query_lower);
            results.push(SearchResult {
                name: node.meta.name.clone(),
                category: node.meta.category.clone(),
                tags: node.meta.tags.clone(),
                snippet,
                match_type,
            });
        }
    }

    Ok(results)
}

fn match_tags(node: &Node, query: &str) -> Option<String> {
    if node
        .meta
        .tags
        .iter()
        .any(|t| t.to_lowercase().contains(query))
    {
        Some("tag".to_string())
    } else {
        None
    }
}

fn match_refs(node: &Node, query: &str) -> Option<String> {
    if node
        .all_refs()
        .iter()
        .any(|r| r.to_lowercase().contains(query))
    {
        Some("ref".to_string())
    } else {
        None
    }
}

fn match_name(node: &Node, query: &str) -> Option<String> {
    if node.meta.name.to_lowercase().contains(query) {
        Some("name".to_string())
    } else {
        None
    }
}

fn match_content(node: &Node, query: &str, _re: &Option<Regex>) -> Option<String> {
    if node.body.to_lowercase().contains(query) {
        Some("content".to_string())
    } else {
        None
    }
}

fn make_snippet(body: &str, query: &str) -> String {
    let lower = body.to_lowercase();
    if let Some(pos) = lower.find(query) {
        let start = pos.saturating_sub(40);
        let end = (pos + query.len() + 40).min(body.len());
        // Adjust to char boundaries
        let start = body
            .char_indices()
            .map(|(i, _)| i)
            .find(|&i| i >= start)
            .unwrap_or(0);
        let end = body
            .char_indices()
            .map(|(i, _)| i)
            .rfind(|&i| i <= end)
            .unwrap_or(body.len());
        let mut snippet = body[start..end].to_string();
        if start > 0 {
            snippet = format!("...{}", snippet);
        }
        if end < body.len() {
            snippet = format!("{}...", snippet);
        }
        snippet
    } else {
        body.chars().take(80).collect::<String>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::NodeMeta;
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
                body: "Authentication flow using JWT tokens.".to_string(),
                path: None,
            },
            Node {
                meta: NodeMeta {
                    name: "user-model".to_string(),
                    category: "domain".to_string(),
                    tags: vec!["model".to_string()],
                    refs: vec![],
                    created: None,
                    updated: None,
                },
                body: "User entity with roles and permissions.".to_string(),
                path: None,
            },
        ];
        for n in &nodes {
            store.upsert(n).unwrap();
        }
        (dir, store)
    }

    #[test]
    fn test_search_by_tag() {
        let (_dir, store) = setup();
        let results = search(&store, "auth", SearchBy::Tag).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "auth-flow");
    }

    #[test]
    fn test_search_by_content() {
        let (_dir, store) = setup();
        let results = search(&store, "JWT", SearchBy::Content).unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_search_all() {
        let (_dir, store) = setup();
        let results = search(&store, "user", SearchBy::All).unwrap();
        assert_eq!(results.len(), 2); // user-model by name, auth-flow by ref
    }
}
