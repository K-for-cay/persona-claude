use crate::index::write_index;
use crate::node::{Node, NodeMeta};
use crate::recall;
use crate::search::{self, SearchBy};
use crate::store::Store;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Value,
}

#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
struct JsonRpcError {
    code: i64,
    message: String,
}

struct McpServer {
    persona_store: Store,
    project_store: Option<Store>,
}

impl McpServer {
    fn new(persona_dir: PathBuf, project_dir: Option<PathBuf>) -> Self {
        McpServer {
            persona_store: Store::new(persona_dir),
            project_store: project_dir.map(Store::new),
        }
    }

    fn get_store(&self, scope: &str) -> Result<&Store, String> {
        match scope {
            "persona" => Ok(&self.persona_store),
            "project" => self
                .project_store
                .as_ref()
                .ok_or_else(|| "no project ontology configured".to_string()),
            _ => Err(format!("invalid scope: {}, use 'persona' or 'project'", scope)),
        }
    }

    fn handle_request(&self, req: &JsonRpcRequest) -> JsonRpcResponse {
        let id = req.id.clone().unwrap_or(Value::Null);

        let result = match req.method.as_str() {
            "initialize" => Ok(json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": { "listChanged": false }
                },
                "serverInfo": {
                    "name": "onto",
                    "version": env!("CARGO_PKG_VERSION")
                }
            })),
            "notifications/initialized" => return JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(Value::Null),
                error: None,
            },
            "tools/list" => Ok(self.tools_list()),
            "tools/call" => self.tools_call(&req.params),
            _ => Err(format!("unknown method: {}", req.method)),
        };

        match result {
            Ok(val) => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(val),
                error: None,
            },
            Err(msg) => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: None,
                error: Some(JsonRpcError {
                    code: -32603,
                    message: msg,
                }),
            },
        }
    }

    fn tools_list(&self) -> Value {
        json!({
            "tools": [
                {
                    "name": "recall",
                    "description": "Associative recall: find ontology nodes relevant to a context. Returns scored results with connection reasons.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "scope": { "type": "string", "enum": ["persona", "project"], "description": "Which ontology to search" },
                            "context": { "type": "string", "description": "Context string to find relevant nodes for" },
                            "max_nodes": { "type": "integer", "default": 5, "description": "Maximum nodes to return" }
                        },
                        "required": ["scope", "context"]
                    }
                },
                {
                    "name": "upsert",
                    "description": "Create or update an ontology node. Auto-reindexes after change.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "scope": { "type": "string", "enum": ["persona", "project"] },
                            "name": { "type": "string", "description": "Node name (unique identifier)" },
                            "category": { "type": "string", "description": "Category directory (e.g., domain, identity, style)" },
                            "tags": { "type": "array", "items": { "type": "string" }, "default": [] },
                            "refs": { "type": "array", "items": { "type": "string" }, "default": [], "description": "References to other node names" },
                            "body": { "type": "string", "description": "Node content (markdown). Use [[name]] for inline refs." }
                        },
                        "required": ["scope", "name", "category", "body"]
                    }
                },
                {
                    "name": "delete",
                    "description": "Delete an ontology node. Returns list of nodes with dangling references.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "scope": { "type": "string", "enum": ["persona", "project"] },
                            "name": { "type": "string", "description": "Node name to delete" }
                        },
                        "required": ["scope", "name"]
                    }
                },
                {
                    "name": "search",
                    "description": "Search ontology nodes by tag, ref, content, or all.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "scope": { "type": "string", "enum": ["persona", "project"] },
                            "query": { "type": "string" },
                            "by": { "type": "string", "enum": ["tag", "ref", "content", "all"], "default": "all" }
                        },
                        "required": ["scope", "query"]
                    }
                },
                {
                    "name": "list",
                    "description": "List ontology nodes, optionally filtered by category and/or tags.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "scope": { "type": "string", "enum": ["persona", "project"] },
                            "category": { "type": "string", "description": "Filter by category" },
                            "tags": { "type": "array", "items": { "type": "string" }, "description": "Filter by tags (OR)" }
                        },
                        "required": ["scope"]
                    }
                },
                {
                    "name": "graph",
                    "description": "Get the connection graph around a node (neighbors up to N hops).",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "scope": { "type": "string", "enum": ["persona", "project"] },
                            "node": { "type": "string", "description": "Center node name" },
                            "depth": { "type": "integer", "default": 2, "description": "Max hops" }
                        },
                        "required": ["scope", "node"]
                    }
                },
                {
                    "name": "reindex",
                    "description": "Regenerate _index.md for the specified ontology.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "scope": { "type": "string", "enum": ["persona", "project"] }
                        },
                        "required": ["scope"]
                    }
                },
                {
                    "name": "validate",
                    "description": "Find broken references in the ontology.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "scope": { "type": "string", "enum": ["persona", "project"] }
                        },
                        "required": ["scope"]
                    }
                }
            ]
        })
    }

    fn tools_call(&self, params: &Value) -> Result<Value, String> {
        let tool_name = params["name"]
            .as_str()
            .ok_or("missing tool name")?;
        let args = &params["arguments"];

        match tool_name {
            "recall" => self.tool_recall(args),
            "upsert" => self.tool_upsert(args),
            "delete" => self.tool_delete(args),
            "search" => self.tool_search(args),
            "list" => self.tool_list(args),
            "graph" => self.tool_graph(args),
            "reindex" => self.tool_reindex(args),
            "validate" => self.tool_validate(args),
            _ => Err(format!("unknown tool: {}", tool_name)),
        }
    }

    fn tool_recall(&self, args: &Value) -> Result<Value, String> {
        let scope = args["scope"].as_str().ok_or("missing scope")?;
        let context = args["context"].as_str().ok_or("missing context")?;
        let max = args["max_nodes"].as_u64().unwrap_or(5) as usize;
        let store = self.get_store(scope)?;
        let result = recall::recall(store, context, max).map_err(|e| e.to_string())?;
        Ok(json!({
            "content": [{
                "type": "text",
                "text": serde_json::to_string_pretty(&result).unwrap()
            }]
        }))
    }

    fn tool_upsert(&self, args: &Value) -> Result<Value, String> {
        let scope = args["scope"].as_str().ok_or("missing scope")?;
        let store = self.get_store(scope)?;

        let node = Node {
            meta: NodeMeta {
                name: args["name"]
                    .as_str()
                    .ok_or("missing name")?
                    .to_string(),
                category: args["category"]
                    .as_str()
                    .unwrap_or("")
                    .to_string(),
                tags: args["tags"]
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default(),
                refs: args["refs"]
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default(),
                created: None,
                updated: Some(chrono::Local::now().naive_local().into()),
            },
            body: args["body"]
                .as_str()
                .ok_or("missing body")?
                .to_string(),
            path: None,
        };

        let path = store.upsert(&node).map_err(|e| e.to_string())?;
        write_index(store).map_err(|e| e.to_string())?;

        Ok(json!({
            "content": [{
                "type": "text",
                "text": format!("Node '{}' saved to {}", node.meta.name, path.display())
            }]
        }))
    }

    fn tool_delete(&self, args: &Value) -> Result<Value, String> {
        let scope = args["scope"].as_str().ok_or("missing scope")?;
        let name = args["name"].as_str().ok_or("missing name")?;
        let store = self.get_store(scope)?;
        let dangling = store.delete(name).map_err(|e| e.to_string())?;
        write_index(store).map_err(|e| e.to_string())?;

        Ok(json!({
            "content": [{
                "type": "text",
                "text": if dangling.is_empty() {
                    format!("Node '{}' deleted. No dangling references.", name)
                } else {
                    format!("Node '{}' deleted. Dangling refs in: {}", name, dangling.join(", "))
                }
            }]
        }))
    }

    fn tool_search(&self, args: &Value) -> Result<Value, String> {
        let scope = args["scope"].as_str().ok_or("missing scope")?;
        let query = args["query"].as_str().ok_or("missing query")?;
        let by = args["by"].as_str().unwrap_or("all");
        let store = self.get_store(scope)?;
        let results = search::search(store, query, SearchBy::from_str(by))
            .map_err(|e| e.to_string())?;
        Ok(json!({
            "content": [{
                "type": "text",
                "text": serde_json::to_string_pretty(&results).unwrap()
            }]
        }))
    }

    fn tool_list(&self, args: &Value) -> Result<Value, String> {
        let scope = args["scope"].as_str().ok_or("missing scope")?;
        let category = args["category"].as_str();
        let tags: Option<Vec<String>> = args["tags"].as_array().map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        });
        let store = self.get_store(scope)?;
        let nodes = store
            .list(category, tags.as_deref())
            .map_err(|e| e.to_string())?;
        let summaries: Vec<Value> = nodes
            .iter()
            .map(|n| {
                json!({
                    "name": n.meta.name,
                    "category": n.meta.category,
                    "tags": n.meta.tags,
                    "refs": n.all_refs(),
                })
            })
            .collect();
        Ok(json!({
            "content": [{
                "type": "text",
                "text": serde_json::to_string_pretty(&summaries).unwrap()
            }]
        }))
    }

    fn tool_graph(&self, args: &Value) -> Result<Value, String> {
        let scope = args["scope"].as_str().ok_or("missing scope")?;
        let node_name = args["node"].as_str().ok_or("missing node")?;
        let depth = args["depth"].as_u64().unwrap_or(2) as usize;
        let store = self.get_store(scope)?;
        let nodes = store.load_all().map_err(|e| e.to_string())?;
        let graph = crate::graph::Graph::build(&nodes);

        let neighbors = graph.neighbors(node_name, depth);
        let result: Vec<Value> = neighbors
            .iter()
            .map(|(n, d)| {
                json!({
                    "name": n.meta.name,
                    "category": n.meta.category,
                    "distance": d,
                    "tags": n.meta.tags,
                })
            })
            .collect();

        Ok(json!({
            "content": [{
                "type": "text",
                "text": serde_json::to_string_pretty(&result).unwrap()
            }]
        }))
    }

    fn tool_reindex(&self, args: &Value) -> Result<Value, String> {
        let scope = args["scope"].as_str().ok_or("missing scope")?;
        let store = self.get_store(scope)?;
        write_index(store).map_err(|e| e.to_string())?;
        Ok(json!({
            "content": [{
                "type": "text",
                "text": format!("{} ontology reindexed.", scope)
            }]
        }))
    }

    fn tool_validate(&self, args: &Value) -> Result<Value, String> {
        let scope = args["scope"].as_str().ok_or("missing scope")?;
        let store = self.get_store(scope)?;
        let nodes = store.load_all().map_err(|e| e.to_string())?;
        let graph = crate::graph::Graph::build(&nodes);
        let broken = graph.broken_refs();

        Ok(json!({
            "content": [{
                "type": "text",
                "text": if broken.is_empty() {
                    "No broken references found.".to_string()
                } else {
                    let items: Vec<String> = broken
                        .iter()
                        .map(|(from, to)| format!("{} → {} (missing)", from, to))
                        .collect();
                    format!("Broken references:\n{}", items.join("\n"))
                }
            }]
        }))
    }
}

pub fn run_server(persona_dir: PathBuf, project_dir: Option<PathBuf>) {
    let server = McpServer::new(persona_dir, project_dir);
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let req: JsonRpcRequest = match serde_json::from_str(line) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("JSON parse error: {}", e);
                continue;
            }
        };

        let resp = server.handle_request(&req);
        let output = serde_json::to_string(&resp).unwrap();
        let _ = writeln!(stdout, "{}", output);
        let _ = stdout.flush();
    }
}
