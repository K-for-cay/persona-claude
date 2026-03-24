use crate::node::{Node, NodeError};
use std::path::{Path, PathBuf};
use thiserror::Error;
use walkdir::WalkDir;

#[derive(Error, Debug)]
pub enum StoreError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("node error: {0}")]
    Node(#[from] NodeError),
    #[error("node not found: {0}")]
    NotFound(String),
    #[error("node already exists: {0}")]
    AlreadyExists(String),
}

#[derive(Debug, Clone)]
pub struct Store {
    pub root: PathBuf,
}

impl Store {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Store { root: root.into() }
    }

    /// Load all nodes from the store
    pub fn load_all(&self) -> Result<Vec<Node>, StoreError> {
        let mut nodes = Vec::new();
        if !self.root.exists() {
            return Ok(nodes);
        }
        for entry in WalkDir::new(&self.root)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "md")
                && path.file_name().is_some_and(|f| f != "_index.md")
            {
                let content = std::fs::read_to_string(path)?;
                match Node::parse(&content) {
                    Ok(mut node) => {
                        node.path = Some(
                            path.strip_prefix(&self.root)
                                .unwrap_or(path)
                                .to_path_buf(),
                        );
                        nodes.push(node);
                    }
                    Err(_) => continue, // skip non-ontology markdown files
                }
            }
        }
        Ok(nodes)
    }

    /// Get a single node by name
    pub fn get(&self, name: &str) -> Result<Node, StoreError> {
        let nodes = self.load_all()?;
        nodes
            .into_iter()
            .find(|n| n.meta.name == name)
            .ok_or_else(|| StoreError::NotFound(name.to_string()))
    }

    /// Upsert a node: create or update
    pub fn upsert(&self, node: &Node) -> Result<PathBuf, StoreError> {
        let category = if node.meta.category.is_empty() {
            "."
        } else {
            &node.meta.category
        };
        let dir = self.root.join(category);
        std::fs::create_dir_all(&dir)?;

        // Check if node exists by name (might be in different category)
        if let Ok(existing) = self.get(&node.meta.name) {
            if let Some(old_path) = &existing.path {
                let full_old = self.root.join(old_path);
                if full_old.exists() {
                    std::fs::remove_file(&full_old)?;
                }
            }
        }

        let filename = slug(&node.meta.name);
        let path = dir.join(format!("{}.md", filename));
        std::fs::write(&path, node.render())?;
        Ok(path)
    }

    /// Delete a node by name, returns list of nodes that referenced it
    pub fn delete(&self, name: &str) -> Result<Vec<String>, StoreError> {
        let node = self.get(name)?;
        if let Some(path) = &node.path {
            let full = self.root.join(path);
            if full.exists() {
                std::fs::remove_file(&full)?;
            }
        }

        // Find dangling refs
        let all = self.load_all()?;
        let dangling: Vec<String> = all
            .iter()
            .filter(|n| n.all_refs().contains(&name.to_string()))
            .map(|n| n.meta.name.clone())
            .collect();

        Ok(dangling)
    }

    /// List nodes, optionally filtered by category and/or tags
    pub fn list(
        &self,
        category: Option<&str>,
        tags: Option<&[String]>,
    ) -> Result<Vec<Node>, StoreError> {
        let all = self.load_all()?;
        Ok(all
            .into_iter()
            .filter(|n| {
                if let Some(cat) = category {
                    if n.meta.category != cat {
                        return false;
                    }
                }
                if let Some(tags) = tags {
                    if !tags.iter().any(|t| n.meta.tags.contains(t)) {
                        return false;
                    }
                }
                true
            })
            .collect())
    }

    pub fn root(&self) -> &Path {
        &self.root
    }
}

fn slug(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' { c } else { '-' })
        .collect::<String>()
        .to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::NodeMeta;
    use tempfile::TempDir;

    fn test_store() -> (TempDir, Store) {
        let dir = TempDir::new().unwrap();
        let store = Store::new(dir.path());
        (dir, store)
    }

    fn make_node(name: &str, category: &str, tags: &[&str], refs: &[&str]) -> Node {
        Node {
            meta: NodeMeta {
                name: name.to_string(),
                category: category.to_string(),
                tags: tags.iter().map(|s| s.to_string()).collect(),
                refs: refs.iter().map(|s| s.to_string()).collect(),
                created: None,
                updated: None,
            },
            body: format!("Body of {}", name),
            path: None,
        }
    }

    #[test]
    fn test_upsert_and_get() {
        let (_dir, store) = test_store();
        let node = make_node("test-node", "domain", &["auth"], &[]);
        store.upsert(&node).unwrap();

        let loaded = store.get("test-node").unwrap();
        assert_eq!(loaded.meta.name, "test-node");
        assert_eq!(loaded.meta.category, "domain");
    }

    #[test]
    fn test_delete() {
        let (_dir, store) = test_store();
        let n1 = make_node("node-a", "domain", &[], &[]);
        let n2 = make_node("node-b", "domain", &[], &["node-a"]);
        store.upsert(&n1).unwrap();
        store.upsert(&n2).unwrap();

        let dangling = store.delete("node-a").unwrap();
        assert!(dangling.contains(&"node-b".to_string()));
        assert!(store.get("node-a").is_err());
    }

    #[test]
    fn test_list_filter() {
        let (_dir, store) = test_store();
        store.upsert(&make_node("a", "domain", &["x"], &[])).unwrap();
        store.upsert(&make_node("b", "workflow", &["y"], &[])).unwrap();
        store.upsert(&make_node("c", "domain", &["y"], &[])).unwrap();

        let domain = store.list(Some("domain"), None).unwrap();
        assert_eq!(domain.len(), 2);

        let tagged = store.list(None, Some(&["y".to_string()])).unwrap();
        assert_eq!(tagged.len(), 2);
    }

    #[test]
    fn test_upsert_overwrites() {
        let (_dir, store) = test_store();
        let mut node = make_node("test", "domain", &["v1"], &[]);
        store.upsert(&node).unwrap();

        node.meta.tags = vec!["v2".to_string()];
        node.body = "Updated body".to_string();
        store.upsert(&node).unwrap();

        let loaded = store.get("test").unwrap();
        assert_eq!(loaded.meta.tags, vec!["v2"]);
        assert_eq!(loaded.body, "Updated body");
    }
}
