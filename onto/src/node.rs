use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum NodeError {
    #[error("invalid frontmatter: {0}")]
    InvalidFrontmatter(String),
    #[error("missing frontmatter delimiters")]
    MissingFrontmatter,
    #[error("yaml parse error: {0}")]
    YamlError(#[from] serde_yaml::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NodeMeta {
    pub name: String,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub refs: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created: Option<NaiveDate>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated: Option<NaiveDate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub meta: NodeMeta,
    pub body: String,
    /// Relative path from ontology root (e.g., "domain/user-auth.md")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<PathBuf>,
}

impl Node {
    pub fn parse(content: &str) -> Result<Self, NodeError> {
        let (meta, body) = split_frontmatter(content)?;
        let meta: NodeMeta =
            serde_yaml::from_str(&meta).map_err(NodeError::YamlError)?;
        Ok(Node {
            meta,
            body: body.trim().to_string(),
            path: None,
        })
    }

    pub fn render(&self) -> String {
        let yaml = serde_yaml::to_string(&self.meta).unwrap_or_default();
        format!("---\n{}---\n\n{}\n", yaml, self.body)
    }

    /// Extract inline [[wikilink]] references from body
    pub fn inline_refs(&self) -> Vec<String> {
        let re = regex::Regex::new(r"\[\[([^\]]+)\]\]").unwrap();
        re.captures_iter(&self.body)
            .map(|c| c[1].to_string())
            .collect()
    }

    /// All references: explicit refs + inline wikilinks
    pub fn all_refs(&self) -> Vec<String> {
        let mut all = self.meta.refs.clone();
        for r in self.inline_refs() {
            if !all.contains(&r) {
                all.push(r);
            }
        }
        all
    }
}

fn split_frontmatter(content: &str) -> Result<(String, String), NodeError> {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return Err(NodeError::MissingFrontmatter);
    }
    let after_first = &trimmed[3..];
    let end = after_first
        .find("\n---")
        .ok_or(NodeError::MissingFrontmatter)?;
    let frontmatter = after_first[..end].trim().to_string();
    let body = after_first[end + 4..].to_string();
    Ok((frontmatter, body))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_node() {
        let content = r#"---
name: "test-node"
category: "domain"
tags: ["auth", "user"]
refs: ["other-node"]
created: "2026-03-24"
---

This is the body with a [[wikilink]] reference.
"#;
        let node = Node::parse(content).unwrap();
        assert_eq!(node.meta.name, "test-node");
        assert_eq!(node.meta.category, "domain");
        assert_eq!(node.meta.tags, vec!["auth", "user"]);
        assert_eq!(node.meta.refs, vec!["other-node"]);
        assert!(node.body.contains("wikilink"));
    }

    #[test]
    fn test_inline_refs() {
        let content = r#"---
name: "test"
---

See [[node-a]] and [[node-b]] for details.
"#;
        let node = Node::parse(content).unwrap();
        assert_eq!(node.inline_refs(), vec!["node-a", "node-b"]);
    }

    #[test]
    fn test_all_refs_dedup() {
        let content = r#"---
name: "test"
refs: ["node-a"]
---

See [[node-a]] and [[node-b]].
"#;
        let node = Node::parse(content).unwrap();
        let refs = node.all_refs();
        assert_eq!(refs, vec!["node-a", "node-b"]);
    }

    #[test]
    fn test_render_roundtrip() {
        let content = r#"---
name: "test-node"
category: "domain"
tags:
- auth
refs: []
---

Body text here.
"#;
        let node = Node::parse(content).unwrap();
        let rendered = node.render();
        let reparsed = Node::parse(&rendered).unwrap();
        assert_eq!(node.meta.name, reparsed.meta.name);
        assert_eq!(node.body, reparsed.body);
    }

    #[test]
    fn test_missing_frontmatter() {
        let content = "no frontmatter here";
        assert!(Node::parse(content).is_err());
    }
}
