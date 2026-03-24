use crate::graph::Graph;
use crate::store::Store;
use std::path::Path;

/// Generate _index.md content from the current graph state
pub fn generate_index(store: &Store) -> Result<String, crate::store::StoreError> {
    let nodes = store.load_all()?;
    let graph = Graph::build(&nodes);

    let mut out = String::new();
    out.push_str("---\n");
    out.push_str("name: \"Index\"\n");
    out.push_str(&format!(
        "description: \"Auto-generated ontology index ({} nodes)\"\n",
        nodes.len()
    ));
    out.push_str(&format!(
        "updated: \"{}\"\n",
        chrono::Local::now().format("%Y-%m-%d")
    ));
    out.push_str("---\n\n");
    out.push_str("# Ontology Index\n\n");

    // Group by category
    let mut categories: std::collections::BTreeMap<String, Vec<&crate::node::Node>> =
        std::collections::BTreeMap::new();
    for node in &nodes {
        let cat = if node.meta.category.is_empty() {
            "uncategorized"
        } else {
            &node.meta.category
        };
        categories.entry(cat.to_string()).or_default().push(node);
    }

    for (cat, cat_nodes) in &categories {
        out.push_str(&format!("## {}/\n\n", cat));
        for node in cat_nodes {
            let tags = if node.meta.tags.is_empty() {
                String::new()
            } else {
                format!(" [{}]", node.meta.tags.join(", "))
            };
            let refs = node.all_refs();
            let ref_str = if refs.is_empty() {
                String::new()
            } else {
                format!(
                    " → {}",
                    refs.iter()
                        .map(|r| format!("[[{}]]", r))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            };
            out.push_str(&format!("- **{}**{}{}\n", node.meta.name, tags, ref_str));
        }
        out.push('\n');
    }

    // Broken refs warning
    let broken = graph.broken_refs();
    if !broken.is_empty() {
        out.push_str("## ⚠ Broken References\n\n");
        for (from, to) in &broken {
            out.push_str(&format!("- {} → {} (not found)\n", from, to));
        }
        out.push('\n');
    }

    // Tag cloud
    if !graph.tag_index.is_empty() {
        out.push_str("## Tags\n\n");
        let mut tags: Vec<_> = graph.tag_index.iter().collect();
        tags.sort_by(|a, b| b.1.len().cmp(&a.1.len()));
        for (tag, nodes) in tags {
            out.push_str(&format!("- `{}` ({})\n", tag, nodes.len()));
        }
    }

    Ok(out)
}

/// Write _index.md to store root
pub fn write_index(store: &Store) -> Result<(), crate::store::StoreError> {
    let content = generate_index(store)?;
    let path = store.root().join("_index.md");
    std::fs::write(path, content)?;
    Ok(())
}

/// Reindex only if the given path is within the ontology root
pub fn reindex_if_ontology_path(
    path: &str,
    persona_root: &Path,
    project_root: Option<&Path>,
) -> Result<bool, crate::store::StoreError> {
    let path = Path::new(path);

    if path.starts_with(persona_root) {
        let store = Store::new(persona_root);
        write_index(&store)?;
        return Ok(true);
    }

    if let Some(proj) = project_root {
        if path.starts_with(proj) {
            let store = Store::new(proj);
            write_index(&store)?;
            return Ok(true);
        }
    }

    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::{Node, NodeMeta};
    use tempfile::TempDir;

    #[test]
    fn test_generate_index() {
        let dir = TempDir::new().unwrap();
        let store = Store::new(dir.path());

        let node = Node {
            meta: NodeMeta {
                name: "test-concept".to_string(),
                category: "domain".to_string(),
                tags: vec!["core".to_string()],
                refs: vec![],
                created: None,
                updated: None,
            },
            body: "A test concept.".to_string(),
            path: None,
        };
        store.upsert(&node).unwrap();

        let index = generate_index(&store).unwrap();
        assert!(index.contains("test-concept"));
        assert!(index.contains("domain/"));
        assert!(index.contains("`core`"));
    }

    #[test]
    fn test_write_index() {
        let dir = TempDir::new().unwrap();
        let store = Store::new(dir.path());
        write_index(&store).unwrap();
        assert!(dir.path().join("_index.md").exists());
    }
}
