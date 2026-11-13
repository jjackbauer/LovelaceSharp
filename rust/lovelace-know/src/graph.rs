//! Graph persistence: the durable product of a convergence run.
//!
//! The file is the C# WriteIndented document, so the same bytes describe the same graph in
//! either implementation (field order, string escaping, and double notation all match).

use std::path::Path;

use crate::model::{Graph, KnowledgeConfig};

/// Builds an empty graph for a configuration.
#[must_use]
pub fn new(config: KnowledgeConfig) -> Graph {
    Graph::new(config)
}

/// Writes a graph to a path.
pub fn save(graph: &Graph, path: &Path) -> Result<(), String> {
    let text = graph.to_json().pretty();
    std::fs::write(path, text)
        .map_err(|error| format!("cannot write '{}': {error}", path.display()))
}

/// Reads a graph from a path.
pub fn load(path: &Path) -> Result<Graph, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("cannot read '{}': {error}", path.display()))?;
    // .NET's File.ReadAllText strips a UTF-8 BOM before parsing; match it.
    let text = text.strip_prefix('\u{feff}').unwrap_or(&text);
    let document =
        lovelace_json::Json::parse(text).map_err(|error| format!("invalid graph JSON: {error}"))?;
    Graph::from_json(&document)
}
