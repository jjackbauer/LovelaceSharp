//! Read-only graph traversal and reporting.

use crate::model::Graph;
use crate::model::Plane;

/// Renders the one-line graph summary the CLI publishes.
#[must_use]
pub fn summary(graph: &Graph) -> String {
    let mut text = format!(
        "planes={}, boundaries={}, frontiers={}, samples={}",
        graph.planes.len(),
        graph.boundaries.len(),
        graph.frontiers.len(),
        graph.samples.len()
    );
    if let Some(metrics) = &graph.metrics {
        text.push_str(&format!(
            ", converged={}, C1_newPlanes_lastK={} (rate {}), C2_stable={}/{}, C3_agreement={}, C4_covered={}",
            boolean_text(metrics.converged),
            metrics.c1_new_planes_last_k,
            format_rate(metrics.c1_new_plane_rate),
            metrics.c2_stable_boundaries,
            metrics.c2_total_boundaries,
            format_rate(metrics.c3_agreement),
            boolean_text(metrics.c4_covered)
        ));
    }
    text
}

/// The distinct typed values observed within a plane (e.g. periodic vs terminating reals).
#[must_use]
pub fn representative_values(plane: &Plane, graph: &Graph, max: usize) -> Vec<String> {
    let mut values: Vec<String> = Vec::new();
    for index in &plane.sample_indices {
        let Some(sample) = graph.samples.get(*index as usize) else {
            continue;
        };
        let Some(typed) = &sample.typed else {
            continue;
        };
        if !values.contains(typed) {
            values.push(typed.clone());
        }
    }
    values.sort();
    let mut head: Vec<String> = values.iter().take(max).cloned().collect();
    if values.len() > max {
        head.push(format!("… (+{} more)", values.len() - max));
    }
    head
}

/// Whether a typed rendering carries a repeating period.
#[must_use]
pub fn is_periodic(typed_value: &str) -> bool {
    typed_value.contains('(') && typed_value.contains(')')
}

fn boolean_text(value: bool) -> &'static str {
    if value { "True" } else { "False" }
}

/// Formats a ratio with the C# custom format "0.###".
#[must_use]
pub fn format_rate(value: f64) -> String {
    let rounded = if value.is_finite() {
        (value * 1000.0).round() / 1000.0
    } else {
        0.0
    };
    let mut text = format!("{rounded:.3}");
    while text.ends_with('0') {
        text.pop();
    }
    if text.ends_with('.') {
        text.pop();
    }
    if text == "-0" {
        text = "0".to_string();
    }
    text
}
