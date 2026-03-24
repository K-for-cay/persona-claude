mod graph;
mod index;
mod mcp;
mod node;
mod recall;
mod search;
mod store;

use clap::{Parser, Subcommand};
use node::{Node, NodeMeta};
use std::path::PathBuf;
use store::Store;

#[derive(Parser)]
#[command(name = "onto", version, about = "Ontology manager for Claude Code")]
struct Cli {
    /// Persona ontology directory
    #[arg(long, env = "ONTO_PERSONA_DIR")]
    persona_dir: Option<PathBuf>,

    /// Project ontology directory
    #[arg(long, env = "ONTO_PROJECT_DIR")]
    project_dir: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run as MCP stdio server
    Serve,

    /// Node operations
    Node {
        #[command(subcommand)]
        action: NodeAction,
    },

    /// Search nodes
    Search {
        /// Scope: persona or project
        #[arg(short, long, default_value = "project")]
        scope: String,
        /// Search query
        query: String,
        /// Search by: tag, ref, content, all
        #[arg(short, long, default_value = "all")]
        by: String,
    },

    /// Associative recall
    Recall {
        #[arg(short, long, default_value = "project")]
        scope: String,
        /// Context to recall from
        context: String,
        /// Max nodes to return
        #[arg(short, long, default_value = "5")]
        max: usize,
    },

    /// Regenerate _index.md
    Reindex {
        #[arg(short, long, default_value = "project")]
        scope: String,
    },

    /// Validate references
    Validate {
        #[arg(short, long, default_value = "project")]
        scope: String,
    },

    /// Show graph around a node
    Graph {
        #[arg(short, long, default_value = "project")]
        scope: String,
        /// Center node
        node: String,
        /// Max hops
        #[arg(short, long, default_value = "2")]
        depth: usize,
    },

    /// Reindex if path is in ontology (used by hooks)
    ReindexIfPath {
        /// File path to check
        path: String,
    },
}

#[derive(Subcommand)]
enum NodeAction {
    /// Create or update a node
    Upsert {
        #[arg(short, long, default_value = "project")]
        scope: String,
        /// Node name
        #[arg(long)]
        name: String,
        /// Category
        #[arg(long)]
        category: String,
        /// Tags (comma-separated)
        #[arg(long, default_value = "")]
        tags: String,
        /// Refs (comma-separated)
        #[arg(long, default_value = "")]
        refs: String,
        /// Body content
        #[arg(long)]
        body: String,
    },
    /// Delete a node
    Delete {
        #[arg(short, long, default_value = "project")]
        scope: String,
        /// Node name
        name: String,
    },
    /// Get a node by name
    Get {
        #[arg(short, long, default_value = "project")]
        scope: String,
        /// Node name
        name: String,
    },
    /// List nodes
    List {
        #[arg(short, long, default_value = "project")]
        scope: String,
        /// Filter by category
        #[arg(long)]
        category: Option<String>,
        /// Filter by tags (comma-separated)
        #[arg(long)]
        tags: Option<String>,
    },
}

fn main() {
    let cli = Cli::parse();

    let default_persona = dirs_home().join(".claude/personas");
    let persona_dir = cli.persona_dir.unwrap_or(default_persona);

    match cli.command {
        Commands::Serve => {
            mcp::run_server(persona_dir, cli.project_dir);
        }
        Commands::Node { action } => handle_node(action, &persona_dir, cli.project_dir.as_deref()),
        Commands::Search { scope, query, by } => {
            let store = get_store(&scope, &persona_dir, cli.project_dir.as_deref());
            match search::search(&store, &query, search::SearchBy::from_str(&by)) {
                Ok(results) => println!("{}", serde_json::to_string_pretty(&results).unwrap()),
                Err(e) => eprintln!("error: {}", e),
            }
        }
        Commands::Recall {
            scope,
            context,
            max,
        } => {
            let store = get_store(&scope, &persona_dir, cli.project_dir.as_deref());
            match recall::recall(&store, &context, max) {
                Ok(result) => println!("{}", serde_json::to_string_pretty(&result).unwrap()),
                Err(e) => eprintln!("error: {}", e),
            }
        }
        Commands::Reindex { scope } => {
            let store = get_store(&scope, &persona_dir, cli.project_dir.as_deref());
            match index::write_index(&store) {
                Ok(()) => println!("{} ontology reindexed.", scope),
                Err(e) => eprintln!("error: {}", e),
            }
        }
        Commands::Validate { scope } => {
            let store = get_store(&scope, &persona_dir, cli.project_dir.as_deref());
            match store.load_all() {
                Ok(nodes) => {
                    let g = graph::Graph::build(&nodes);
                    let broken = g.broken_refs();
                    if broken.is_empty() {
                        println!("No broken references.");
                    } else {
                        for (from, to) in &broken {
                            println!("{} → {} (missing)", from, to);
                        }
                    }
                }
                Err(e) => eprintln!("error: {}", e),
            }
        }
        Commands::Graph {
            scope,
            node: node_name,
            depth,
        } => {
            let store = get_store(&scope, &persona_dir, cli.project_dir.as_deref());
            match store.load_all() {
                Ok(nodes) => {
                    let g = graph::Graph::build(&nodes);
                    let neighbors = g.neighbors(&node_name, depth);
                    for (n, d) in &neighbors {
                        println!(
                            "[d={}] {} ({}) [{}]",
                            d,
                            n.meta.name,
                            n.meta.category,
                            n.meta.tags.join(", ")
                        );
                    }
                    if neighbors.is_empty() {
                        println!("No neighbors found for '{}'", node_name);
                    }
                }
                Err(e) => eprintln!("error: {}", e),
            }
        }
        Commands::ReindexIfPath { path } => {
            match index::reindex_if_ontology_path(
                &path,
                &persona_dir,
                cli.project_dir.as_deref(),
            ) {
                Ok(true) => println!("reindexed"),
                Ok(false) => {} // not an ontology path, silent
                Err(e) => eprintln!("error: {}", e),
            }
        }
    }
}

fn handle_node(action: NodeAction, persona_dir: &PathBuf, project_dir: Option<&std::path::Path>) {
    match action {
        NodeAction::Upsert {
            scope,
            name,
            category,
            tags,
            refs,
            body,
        } => {
            let store = get_store(&scope, persona_dir, project_dir);
            let node = Node {
                meta: NodeMeta {
                    name,
                    category,
                    tags: parse_csv(&tags),
                    refs: parse_csv(&refs),
                    created: None,
                    updated: Some(chrono::Local::now().naive_local().into()),
                },
                body,
                path: None,
            };
            match store.upsert(&node) {
                Ok(path) => {
                    let _ = index::write_index(&store);
                    println!("Saved: {}", path.display());
                }
                Err(e) => eprintln!("error: {}", e),
            }
        }
        NodeAction::Delete { scope, name } => {
            let store = get_store(&scope, persona_dir, project_dir);
            match store.delete(&name) {
                Ok(dangling) => {
                    let _ = index::write_index(&store);
                    println!("Deleted: {}", name);
                    if !dangling.is_empty() {
                        println!("Dangling refs in: {}", dangling.join(", "));
                    }
                }
                Err(e) => eprintln!("error: {}", e),
            }
        }
        NodeAction::Get { scope, name } => {
            let store = get_store(&scope, persona_dir, project_dir);
            match store.get(&name) {
                Ok(node) => println!("{}", serde_json::to_string_pretty(&node).unwrap()),
                Err(e) => eprintln!("error: {}", e),
            }
        }
        NodeAction::List {
            scope,
            category,
            tags,
        } => {
            let store = get_store(&scope, persona_dir, project_dir);
            let tag_vec = tags.map(|t| parse_csv(&t));
            match store.list(category.as_deref(), tag_vec.as_deref()) {
                Ok(nodes) => {
                    for n in &nodes {
                        println!(
                            "{} ({}) [{}]",
                            n.meta.name,
                            n.meta.category,
                            n.meta.tags.join(", ")
                        );
                    }
                    if nodes.is_empty() {
                        println!("No nodes found.");
                    }
                }
                Err(e) => eprintln!("error: {}", e),
            }
        }
    }
}

fn get_store(
    scope: &str,
    persona_dir: &PathBuf,
    project_dir: Option<&std::path::Path>,
) -> Store {
    match scope {
        "persona" => Store::new(persona_dir),
        "project" => {
            Store::new(project_dir.expect("--project-dir or ONTO_PROJECT_DIR required"))
        }
        _ => {
            eprintln!("invalid scope: {}, use 'persona' or 'project'", scope);
            std::process::exit(1);
        }
    }
}

fn parse_csv(s: &str) -> Vec<String> {
    s.split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

fn dirs_home() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"))
}
