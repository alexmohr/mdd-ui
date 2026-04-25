/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 Alexander Mohr
 */

//! MCP (Model Context Protocol) server for browsing and diffing MDD diagnostic databases.
//!
//! Exposes tools over stdio that allow LLMs to load MDD files, browse the tree
//! structure, search nodes, view details, and compare two databases.

use std::{collections::HashMap, fmt::Write as _, sync::Mutex};

use anyhow::{Context, Result};
use mdd_core::{
    database, diff,
    tree::{self, DetailContent, DiffStatus, NodeType, TreeNode},
};
use rmcp::{handler::server::wrapper::Parameters, schemars, tool, tool_router};
use serde::Deserialize;

// Cached data: we load the MDD, build the tree, and store only Send-safe data.

struct CachedDatabase {
    nodes: Vec<TreeNode>,
    ecu_name: String,
}

// Tool parameter types

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct LoadMddParams {
    /// Absolute path to the MDD file to load
    path: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct BrowseTreeParams {
    /// Absolute path to the MDD file (must be loaded first via `load_mdd`)
    path: String,
    /// Maximum tree depth to display (default: all)
    max_depth: Option<usize>,
    /// Index of the parent node to start browsing from (default: root)
    start_index: Option<usize>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct GetNodeDetailsParams {
    /// Absolute path to the MDD file (must be loaded first via `load_mdd`)
    path: String,
    /// Index of the tree node (as returned by `browse_tree` or `search_nodes`)
    node_index: usize,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct SearchNodesParams {
    /// Absolute path to the MDD file (must be loaded first via `load_mdd`)
    path: String,
    /// Case-insensitive search query to match against node text
    query: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct DiffMddParams {
    /// Absolute path to the old/reference MDD file
    old_path: String,
    /// Absolute path to the new MDD file
    new_path: String,
    /// Maximum tree depth to display (default: all)
    max_depth: Option<usize>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct ExportDiffParams {
    /// Absolute path to the old/reference MDD file
    old_path: String,
    /// Absolute path to the new MDD file
    new_path: String,
}

// MCP Server

#[derive(Clone)]
pub struct MddMcpServer {
    databases: std::sync::Arc<Mutex<HashMap<String, CachedDatabase>>>,
}

impl MddMcpServer {
    fn new() -> Self {
        Self {
            databases: std::sync::Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Load a database if not already cached, returning `Ok(())` or an error message.
    fn ensure_loaded(&self, path: &str) -> std::result::Result<(), String> {
        let mut cache = self
            .databases
            .lock()
            .map_err(|e| format!("Lock poisoned: {e}"))?;
        if cache.contains_key(path) {
            return Ok(());
        }
        let db = database::load_mdd(path).map_err(|e| format!("Failed to load {path}: {e:#}"))?;
        let (nodes, ecu_name) = tree::build_tree(&db, path);
        cache.insert(path.to_owned(), CachedDatabase { nodes, ecu_name });
        Ok(())
    }

    /// Get a reference to cached nodes via a closure (avoids holding the lock).
    fn with_cache<F, R>(&self, path: &str, f: F) -> std::result::Result<R, String>
    where
        F: FnOnce(&CachedDatabase) -> R,
    {
        let cache = self
            .databases
            .lock()
            .map_err(|e| format!("Lock poisoned: {e}"))?;
        let db = cache
            .get(path)
            .ok_or_else(|| format!("Database not loaded: {path}. Call load_mdd first."))?;
        Ok(f(db))
    }
}

// Tool implementations

#[tool_router(server_handler)]
impl MddMcpServer {
    /// Load an MDD diagnostic database file and return a summary of its contents.
    /// This must be called before using `browse_tree`, `get_node_details`, or `search_nodes`.
    #[tool(
        description = "Load an MDD diagnostic database file and return a summary. Must be called \
                       before browse_tree, get_node_details, or search_nodes."
    )]
    fn load_mdd(&self, Parameters(params): Parameters<LoadMddParams>) -> String {
        if let Err(e) = self.ensure_loaded(&params.path) {
            return format!("Error: {e}");
        }
        self.with_cache(&params.path, |db| {
            let node_count = db.nodes.len();
            let mut summary = format!(
                "Loaded: {}\nECU: {}\nTotal nodes: {node_count}\n\nTop-level sections:",
                params.path, db.ecu_name
            );
            for (i, node) in db.nodes.iter().enumerate() {
                if node.depth == 0 {
                    let _ = write!(summary, "\n  [{i}] {}", node.text);
                }
            }
            summary
        })
        .unwrap_or_else(|e| format!("Error: {e}"))
    }

    /// Browse the tree structure of a loaded MDD database.
    /// Returns an indented text representation of the node hierarchy with node indices.
    #[tool(
        description = "Browse the tree hierarchy of a loaded MDD database. Returns indented node \
                       list with indices that can be used with get_node_details."
    )]
    fn browse_tree(&self, Parameters(params): Parameters<BrowseTreeParams>) -> String {
        if let Err(e) = self.ensure_loaded(&params.path) {
            return format!("Error: {e}");
        }
        self.with_cache(&params.path, |db| {
            let max_depth = params.max_depth.unwrap_or(usize::MAX);
            let start = params.start_index.unwrap_or(0);
            let start_depth = db.nodes.get(start).map_or(0, |n| n.depth);
            let mut output = String::new();
            let mut skip_depth: Option<usize> = None;

            for (i, node) in db.nodes.iter().enumerate().skip(start) {
                // If we started from a specific node, stop when we
                // return to the same or lower depth
                if i > start && node.depth <= start_depth {
                    break;
                }
                let relative_depth = node.depth.saturating_sub(start_depth);
                if relative_depth > max_depth {
                    continue;
                }
                // Skip collapsed subtrees for readability
                if let Some(sd) = skip_depth {
                    if node.depth > sd {
                        continue;
                    }
                    skip_depth = None;
                }
                let indent = "  ".repeat(relative_depth);
                let type_tag = node_type_tag(node.node_type);
                let children_marker = if node.has_children { " ..." } else { "" };
                let _ = writeln!(
                    output,
                    "{indent}{type_tag} [{i}] {}{children_marker}",
                    node.text
                );
            }
            if output.is_empty() {
                "No nodes found at the specified location.".to_owned()
            } else {
                output
            }
        })
        .unwrap_or_else(|e| format!("Error: {e}"))
    }

    /// Get detailed information for a specific tree node by its index.
    /// Returns all detail sections (overview tables, parameters, etc.) as formatted text.
    #[tool(
        description = "Get detailed information for a specific tree node by index. Returns \
                       formatted detail sections including overview tables, parameters, and \
                       related data."
    )]
    fn get_node_details(&self, Parameters(params): Parameters<GetNodeDetailsParams>) -> String {
        if let Err(e) = self.ensure_loaded(&params.path) {
            return format!("Error: {e}");
        }
        self.with_cache(&params.path, |db| {
            let Some(node) = db.nodes.get(params.node_index) else {
                return format!(
                    "Error: Node index {} out of range (0..{})",
                    params.node_index,
                    db.nodes.len()
                );
            };
            let mut output = format!(
                "Node: {}\nType: {:?}\nDepth: {}\n",
                node.text, node.node_type, node.depth
            );

            if let Some(ref status) = node.diff_status {
                let _ = writeln!(output, "Diff: {status:?}");
            }

            if node.detail_sections.is_empty() {
                output.push_str("\n(No detail sections for this node)\n");
                return output;
            }

            for section in node.detail_sections.iter() {
                let _ = write!(output, "\n--- {} ---\n", section.title);
                format_detail_content(&section.content, &mut output, 0);
            }
            output
        })
        .unwrap_or_else(|e| format!("Error: {e}"))
    }

    /// Search for tree nodes matching a query string (case-insensitive).
    /// Returns matching nodes with their indices for use with `get_node_details`.
    #[tool(
        description = "Search tree nodes by text (case-insensitive). Returns matching nodes with \
                       indices that can be used with get_node_details or browse_tree start_index."
    )]
    fn search_nodes(&self, Parameters(params): Parameters<SearchNodesParams>) -> String {
        if let Err(e) = self.ensure_loaded(&params.path) {
            return format!("Error: {e}");
        }
        let query_lower = params.query.to_lowercase();
        self.with_cache(&params.path, |db| {
            let mut results = Vec::new();
            for (i, node) in db.nodes.iter().enumerate() {
                if node.text.to_lowercase().contains(&query_lower) {
                    results.push(format!(
                        "[{i}] {} (depth:{}, type:{:?})",
                        node.text, node.depth, node.node_type
                    ));
                }
            }
            if results.is_empty() {
                format!("No nodes matching \"{}\"", params.query)
            } else {
                format!("Found {} matches:\n{}", results.len(), results.join("\n"))
            }
        })
        .unwrap_or_else(|e| format!("Error: {e}"))
    }

    /// Compare two MDD databases and return a diff tree showing additions,
    /// removals, and modifications.
    #[allow(clippy::unused_self)]
    #[tool(
        description = "Compare two MDD databases and return a diff tree with change annotations \
                       (+added, -removed, ~modified)."
    )]
    fn diff_mdd(&self, Parameters(params): Parameters<DiffMddParams>) -> String {
        let db_old = match database::load_mdd(&params.old_path) {
            Ok(db) => db,
            Err(e) => return format!("Error loading {}: {e:#}", params.old_path),
        };
        let db_new = match database::load_mdd(&params.new_path) {
            Ok(db) => db,
            Err(e) => return format!("Error loading {}: {e:#}", params.new_path),
        };

        let (nodes, ecu_name) =
            diff::diff_tree::build_diff_tree(&db_old, &db_new, &params.old_path, &params.new_path);

        let max_depth = params.max_depth.unwrap_or(usize::MAX);
        let mut output = format!(
            "Diff: {} (old) vs {} (new)\nECU: {ecu_name}\n\n",
            params.old_path, params.new_path
        );

        for (i, node) in nodes.iter().enumerate() {
            if node.depth > max_depth {
                continue;
            }
            let diff_marker = match node.diff_status {
                Some(DiffStatus::Added) => "+ ",
                Some(DiffStatus::Removed) => "- ",
                Some(DiffStatus::Modified) => "~ ",
                Some(DiffStatus::Unchanged) | None => "  ",
            };
            let indent = "  ".repeat(node.depth);
            let _ = writeln!(output, "{diff_marker}{indent}[{i}] {}", node.text);
        }
        output
    }

    /// Export a full text diff report comparing two MDD databases.
    #[allow(clippy::unused_self)]
    #[tool(
        description = "Export a detailed text diff report between two MDD databases, showing all \
                       property changes across variants, services, parameters, etc."
    )]
    fn export_diff(&self, Parameters(params): Parameters<ExportDiffParams>) -> String {
        let db_old = match database::load_mdd(&params.old_path) {
            Ok(db) => db,
            Err(e) => return format!("Error loading {}: {e:#}", params.old_path),
        };
        let db_new = match database::load_mdd(&params.new_path) {
            Ok(db) => db,
            Err(e) => return format!("Error loading {}: {e:#}", params.new_path),
        };

        let snap_old = match diff::snapshot::EcuSnapshot::from_database(&db_old) {
            Ok(s) => s,
            Err(e) => return format!("Error extracting old snapshot: {e:#}"),
        };
        let snap_new = match diff::snapshot::EcuSnapshot::from_database(&db_new) {
            Ok(s) => s,
            Err(e) => return format!("Error extracting new snapshot: {e:#}"),
        };

        let diff_result = diff::compare::compare(&snap_old, &snap_new);

        let mut buf = Vec::new();
        if let Err(e) = diff::export::write_text_report(&mut buf, &diff_result) {
            return format!("Error writing report: {e}");
        }
        String::from_utf8_lossy(&buf).into_owned()
    }
}

// Helpers

/// Short tag for a node type, used in tree display.
const fn node_type_tag(nt: NodeType) -> &'static str {
    match nt {
        NodeType::Container => "[+]",
        NodeType::SectionHeader => "[#]",
        NodeType::Service | NodeType::ParentRefService => "[S]",
        NodeType::Request => "[Rq]",
        NodeType::PosResponse => "[R+]",
        NodeType::NegResponse => "[R-]",
        NodeType::Dop => "[D]",
        NodeType::Job => "[J]",
        NodeType::FunctionalClass => "[FC]",
        NodeType::Sdg => "[SDG]",
        NodeType::ParentRefs => "[PR]",
        NodeType::Default => "[-]",
    }
}

/// Format detail content as human-readable text.
fn format_detail_content(content: &DetailContent, output: &mut String, indent: usize) {
    let prefix = "  ".repeat(indent);
    match content {
        DetailContent::PlainText(lines) => {
            for line in lines {
                let _ = writeln!(output, "{prefix}{line}");
            }
        }
        DetailContent::Table { header, rows, .. } => {
            if !header.cells.is_empty() {
                let _ = writeln!(
                    output,
                    "{prefix}{}",
                    header
                        .cells
                        .iter()
                        .map(|c| c.text.as_str())
                        .collect::<Vec<_>>()
                        .join(" | ")
                );
                let separator: Vec<String> = header
                    .cells
                    .iter()
                    .map(|c| "-".repeat(c.text.len().max(3)))
                    .collect();
                let _ = writeln!(output, "{prefix}{}", separator.join("-+-"));
            }
            for row in rows {
                let row_indent = "  ".repeat(row.indent);
                let diff_prefix = match row.diff_status {
                    Some(DiffStatus::Added) => "+ ",
                    Some(DiffStatus::Removed) => "- ",
                    Some(DiffStatus::Modified) => "~ ",
                    _ => "",
                };
                let _ = writeln!(
                    output,
                    "{prefix}{diff_prefix}{row_indent}{}",
                    row.cells
                        .iter()
                        .map(|c| c.text.as_str())
                        .collect::<Vec<_>>()
                        .join(" | ")
                );
            }
        }
        DetailContent::Composite(sections) => {
            for section in sections {
                let _ = writeln!(output, "{prefix}[{}]", section.title);
                format_detail_content(&section.content, output, indent.saturating_add(1));
            }
        }
    }
}

// Entry point

/// Start the MCP server over stdio.
pub fn run_mcp() -> Result<()> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("Failed to build tokio runtime")?;

    rt.block_on(async {
        let server = MddMcpServer::new();
        let service = rmcp::ServiceExt::serve(server, rmcp::transport::io::stdio())
            .await
            .context("Failed to start MCP service")?;
        service.waiting().await.context("MCP service error")?;
        Ok(())
    })
}
