//! The JSON-over-stdio command protocol.
//!
//! One request in, one response out, with the C# exit-code contract: 0 success, 1 command or
//! execution error, 2 usage error. Commands are config, sample, reduce, converge, query, and
//! help; every response carries the full field set with nulls for unused members, exactly as
//! the C# CliResponse record does.

use std::collections::BTreeSet;
use std::path::Path;

use lovelace_json::Json as In;

use crate::converge;
use crate::convergence;
use crate::graph;
use crate::json::Out;
use crate::model::{BoundaryEdge, Frontier, Graph, KnowledgeConfig, Metrics, Plane, SampleRecord};
use crate::proposal::Proposal;
use crate::query;
use crate::reducer;
use crate::runner::{self, ProcessRunner};

/// A JSON-over-stdio request to the CLI.
#[derive(Clone, Debug, Default)]
pub struct CliRequest {
    /// The command to run.
    pub command: String,
    /// A full configuration override.
    pub config: Option<KnowledgeConfig>,
    /// The graph file path.
    pub graph_path: Option<String>,
    /// The runner executable path.
    pub runner: Option<String>,
    /// A seed override.
    pub seed: Option<i64>,
    /// A batch-size override.
    pub batch_size: Option<i32>,
    /// A sample-budget override.
    pub max_samples: Option<i32>,
    /// The query to run.
    pub query: Option<String>,
}

impl CliRequest {
    /// Parses a request document.
    pub fn parse(json: &str) -> Result<Self, String> {
        let document = lovelace_json::Json::parse(json).map_err(|error| error.to_string())?;
        if matches!(document, In::Null) {
            return Err("empty request".to_string());
        }
        let command = document
            .get("command")
            .and_then(In::as_str)
            .unwrap_or("")
            .to_string();
        Ok(Self {
            command,
            config: match document.get("config") {
                None | Some(In::Null) => None,
                Some(value) => Some(KnowledgeConfig::from_json(value)?),
            },
            graph_path: optional_string(&document, "graphPath"),
            runner: optional_string(&document, "runner"),
            seed: optional_integer(&document, "seed"),
            batch_size: optional_small_integer(&document, "batchSize"),
            max_samples: optional_small_integer(&document, "maxSamples"),
            query: optional_string(&document, "query"),
        })
    }
}

fn optional_string(document: &In, key: &str) -> Option<String> {
    document.get(key).and_then(In::as_str).map(str::to_string)
}

fn optional_integer(document: &In, key: &str) -> Option<i64> {
    match document.get(key) {
        Some(In::Num(text)) => text.parse::<i64>().ok(),
        _ => None,
    }
}

fn optional_small_integer(document: &In, key: &str) -> Option<i32> {
    match document.get(key) {
        Some(In::Num(text)) => text.parse::<i32>().ok(),
        _ => None,
    }
}

/// A JSON-over-stdio response from the CLI; unused fields are null per command.
#[derive(Clone, Debug, Default)]
pub struct CliResponse {
    /// Whether the command succeeded.
    pub ok: bool,
    /// The command that produced the response.
    pub command: String,
    /// The one-line human-readable result.
    pub summary: Option<String>,
    /// The resolved configuration.
    pub config: Option<KnowledgeConfig>,
    /// The whole graph, when the command returns one.
    pub graph: Option<Graph>,
    /// The planes, when the command returns them.
    pub planes: Option<Vec<Plane>>,
    /// The boundaries, when the command returns them.
    pub boundaries: Option<Vec<BoundaryEdge>>,
    /// The frontiers, when the command returns them.
    pub frontiers: Option<Vec<Frontier>>,
    /// The metrics, when the command returns them.
    pub metrics: Option<Metrics>,
    /// The samples, when the command returns them.
    pub samples: Option<Vec<SampleRecord>>,
    /// The error message, when the command failed.
    pub error: Option<String>,
}

impl CliResponse {
    /// Builds a failed response.
    #[must_use]
    pub fn error(command: &str, message: impl Into<String>) -> Self {
        Self {
            ok: false,
            command: command.to_string(),
            error: Some(message.into()),
            ..Self::default()
        }
    }

    /// Renders the response as the C# indented document.
    #[must_use]
    pub fn to_json(&self) -> Out {
        Out::obj([
            ("ok", Out::Bool(self.ok)),
            ("command", Out::str(self.command.clone())),
            ("summary", Out::opt_str(self.summary.as_deref())),
            (
                "config",
                match &self.config {
                    Some(config) => config.to_json(),
                    None => Out::Null,
                },
            ),
            (
                "graph",
                match &self.graph {
                    Some(graph) => graph.to_json(),
                    None => Out::Null,
                },
            ),
            (
                "planes",
                match &self.planes {
                    Some(planes) => Out::Arr(planes.iter().map(Plane::to_json).collect()),
                    None => Out::Null,
                },
            ),
            (
                "boundaries",
                match &self.boundaries {
                    Some(boundaries) => {
                        Out::Arr(boundaries.iter().map(BoundaryEdge::to_json).collect())
                    }
                    None => Out::Null,
                },
            ),
            (
                "frontiers",
                match &self.frontiers {
                    Some(frontiers) => Out::Arr(frontiers.iter().map(Frontier::to_json).collect()),
                    None => Out::Null,
                },
            ),
            (
                "metrics",
                match &self.metrics {
                    Some(metrics) => metrics.to_json(),
                    None => Out::Null,
                },
            ),
            (
                "samples",
                match &self.samples {
                    Some(samples) => Out::Arr(samples.iter().map(SampleRecord::to_json).collect()),
                    None => Out::Null,
                },
            ),
            ("error", Out::opt_str(self.error.as_deref())),
        ])
    }

    /// Renders the response with the trailing newline the CLI writes.
    #[must_use]
    pub fn render(&self) -> String {
        let mut text = self.to_json().pretty();
        text.push('\n');
        text
    }
}

/// The help text returned by the help command.
#[must_use]
pub fn help_text() -> String {
    "Commands: config (resolve defaults), sample (draw+execute a random batch), \
reduce (re-derive planes/boundaries/frontiers from a persisted graph), \
converge (autonomous loop to C1-C4 thresholds), \
query (summary|planes|boundaries|frontiers|metrics|graph)."
        .to_string()
}

/// Dispatches one request.
pub fn dispatch(request: &CliRequest) -> Result<CliResponse, String> {
    match request.command.as_str() {
        "config" => Ok(config_command(request)),
        "sample" => sample_command(request),
        "reduce" => reduce_command(request),
        "converge" => converge_command(request),
        "query" => query_command(request),
        "help" => Ok(CliResponse {
            ok: true,
            command: "help".to_string(),
            summary: Some(help_text()),
            ..CliResponse::default()
        }),
        other => Ok(CliResponse::error(
            other,
            format!("unknown command '{other}'"),
        )),
    }
}

/// Parses and dispatches one request document, returning the response text and exit code.
#[must_use]
pub fn run_request(request_json: &str) -> (String, i32) {
    let request = match CliRequest::parse(request_json) {
        Ok(request) => request,
        Err(message) => {
            let response = CliResponse::error("", format!("invalid request JSON: {message}"));
            return (response.render(), 1);
        }
    };
    match dispatch(&request) {
        Ok(response) => {
            let exit_code = if response.ok { 0 } else { 1 };
            (response.render(), exit_code)
        }
        Err(message) => (CliResponse::error(&request.command, message).render(), 1),
    }
}

fn resolve_config(request: &CliRequest) -> KnowledgeConfig {
    let mut config = request
        .config
        .clone()
        .unwrap_or_else(KnowledgeConfig::default_config);
    if let Some(seed) = request.seed {
        config.seed = seed;
    }
    config
}

fn resolve_graph_path(path: Option<&str>) -> String {
    match path {
        Some(path) if !path.is_empty() => path.to_string(),
        _ => "knowledge-graph.json".to_string(),
    }
}

fn load_graph(request: &CliRequest) -> Result<Graph, String> {
    let path = resolve_graph_path(request.graph_path.as_deref());
    if !Path::new(&path).exists() {
        return Err(format!("no graph at '{path}'; run 'converge' first"));
    }
    graph::load(Path::new(&path))
}

fn config_command(request: &CliRequest) -> CliResponse {
    let config = resolve_config(request);
    let summary = format!(
        "config resolved: seed={}, batch={}, maxSamples={}, ops={}, naturals={}, integers={}, reals={}",
        config.seed,
        config.batch_size,
        config.max_samples,
        config.operations.len(),
        config.natural_values.len(),
        config.integer_values.len(),
        config.real_values.len()
    );
    CliResponse {
        ok: true,
        command: "config".to_string(),
        summary: Some(summary),
        config: Some(config),
        ..CliResponse::default()
    }
}

fn sample_command(request: &CliRequest) -> Result<CliResponse, String> {
    let config = resolve_config(request);
    let count = request.batch_size.unwrap_or(config.batch_size);
    let count = usize::try_from(count).unwrap_or(0);
    let runner_path = runner::resolve_runner(request.runner.as_deref())?;
    let runner = ProcessRunner::new(runner_path);
    let specs = Proposal::random_samples(config.seed, &config, count);
    let records = runner::execute_batch(&runner, &specs, 0);
    let ok_count = records.iter().filter(|record| record.success).count();
    let errors = records.len() - ok_count;
    let distinct = records
        .iter()
        .map(|record| record.sigma.as_str())
        .collect::<BTreeSet<&str>>()
        .len();
    let summary = format!(
        "executed {} samples: {} ok, {} errors, {} distinct plane classes",
        records.len(),
        ok_count,
        errors,
        distinct
    );
    Ok(CliResponse {
        ok: true,
        command: "sample".to_string(),
        summary: Some(summary),
        config: Some(config),
        samples: Some(records),
        ..CliResponse::default()
    })
}

fn reduce_command(request: &CliRequest) -> Result<CliResponse, String> {
    let loaded = load_graph(request)?;
    let reduction = reducer::reduce(&loaded.samples, &loaded.config);
    let metrics = match &loaded.metrics {
        Some(metrics) => metrics.clone(),
        None => convergence::measure(&reduction, &loaded.samples, &loaded.config, &[], &[]),
    };
    let reduced = Graph {
        planes: reduction.planes,
        boundaries: reduction.boundaries,
        frontiers: reduction.frontiers,
        metrics: Some(metrics.clone()),
        ..loaded
    };
    let summary = query::summary(&reduced);
    Ok(CliResponse {
        ok: true,
        command: "reduce".to_string(),
        summary: Some(summary),
        config: Some(reduced.config.clone()),
        planes: Some(reduced.planes.clone()),
        boundaries: Some(reduced.boundaries.clone()),
        frontiers: Some(reduced.frontiers.clone()),
        metrics: Some(metrics),
        ..CliResponse::default()
    })
}

fn converge_command(request: &CliRequest) -> Result<CliResponse, String> {
    let mut config = resolve_config(request);
    if let Some(batch_size) = request.batch_size {
        config.batch_size = batch_size;
    }
    if let Some(max_samples) = request.max_samples {
        config.max_samples = max_samples;
    }
    let path = resolve_graph_path(request.graph_path.as_deref());
    let runner_path = runner::resolve_runner(request.runner.as_deref())?;
    let runner = ProcessRunner::new(runner_path);

    let existing = if Path::new(&path).exists() {
        Some(load_graph(request)?)
    } else {
        None
    };
    let graph = converge::run(&config, &runner, existing);
    graph::save(&graph, Path::new(&path))?;

    let absolute = std::path::absolute(&path)
        .map(|resolved| resolved.display().to_string())
        .unwrap_or_else(|_| path.clone());
    let summary = format!("{}; graph={}", query::summary(&graph), absolute);
    Ok(CliResponse {
        ok: true,
        command: "converge".to_string(),
        summary: Some(summary),
        config: Some(graph.config.clone()),
        graph: Some(graph.clone()),
        metrics: graph.metrics.clone(),
        ..CliResponse::default()
    })
}

fn query_command(request: &CliRequest) -> Result<CliResponse, String> {
    let graph = load_graph(request)?;
    let query_name = request
        .query
        .clone()
        .unwrap_or_else(|| "summary".to_string());
    let base = CliResponse {
        ok: true,
        command: "query".to_string(),
        summary: Some(query::summary(&graph)),
        config: Some(graph.config.clone()),
        metrics: graph.metrics.clone(),
        ..CliResponse::default()
    };
    match query_name.as_str() {
        "summary" | "metrics" => Ok(base),
        "planes" => Ok(CliResponse {
            summary: Some(format!("{} planes", graph.planes.len())),
            planes: Some(graph.planes.clone()),
            ..base
        }),
        "boundaries" => Ok(CliResponse {
            summary: Some(format!("{} boundaries", graph.boundaries.len())),
            boundaries: Some(graph.boundaries.clone()),
            ..base
        }),
        "frontiers" => Ok(CliResponse {
            summary: Some(format!("{} frontiers", graph.frontiers.len())),
            frontiers: Some(graph.frontiers.clone()),
            ..base
        }),
        "graph" => Ok(CliResponse {
            graph: Some(graph.clone()),
            ..base
        }),
        other => Ok(CliResponse::error(
            "query",
            format!(
                "unknown query '{other}' (expected summary|planes|boundaries|frontiers|metrics|graph)"
            ),
        )),
    }
}
