//! Persistence tests: the graph file is the C# document, field for field.

mod common;

use lovelace_know::graph;
use lovelace_know::model::{Graph, KnowledgeConfig};

fn reference_text() -> String {
    std::fs::read_to_string(common::reference_graph_path())
        .expect("the frozen reference is present")
}

fn normalize(text: &str) -> String {
    text.replace("\r\n", "\n")
}

#[test]
fn frozen_reference_round_trips_byte_for_byte() {
    let original = reference_text();
    let document = lovelace_json::Json::parse(&original).expect("the reference parses");
    let graph = Graph::from_json(&document).expect("the reference deserializes");
    let rendered = graph.to_json().pretty();
    assert_eq!(normalize(&rendered), normalize(&original));
}

#[test]
fn save_and_load_round_trip() {
    let original = reference_text();
    let document = lovelace_json::Json::parse(&original).expect("parses");
    let graph = Graph::from_json(&document).expect("deserializes");
    let path = common::temp_graph_path("round-trip");
    graph::save(&graph, &path).expect("the graph is written");
    let loaded = graph::load(&path).expect("the graph is read");
    assert_eq!(normalize(&loaded.to_json().pretty()), normalize(&original));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn new_graph_is_empty_and_versioned() {
    let config = KnowledgeConfig::default_config();
    let graph = Graph::new(config);
    assert_eq!(graph.seed, 20_240_617);
    assert_eq!(graph.version, Graph::VERSION);
    assert!(graph.samples.is_empty());
    assert!(graph.planes.is_empty());
    assert!(graph.boundaries.is_empty());
    assert!(graph.frontiers.is_empty());
    assert!(graph.metrics.is_none());
}
