//! Canonicalization of the runner's JSON envelope into an observation (req 6).
//!
//! Success becomes ok|kind|typed; failure becomes err|message. Everything the protocol
//! marks as noise (revision, timings, artifacts, variables, functions) is discarded, so two
//! executions that differ only in timing canonicalize to the same observation.

use lovelace_json::Json as In;

/// The canonical observation of one executed sample.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Observation {
    /// Whether the runner reported success.
    pub success: bool,
    /// The full canonical sigma: ok|kind|typed, or err|message.
    pub sigma: String,
    /// The result kind, for a successful execution.
    pub kind: Option<String>,
    /// The typed rendering, for a successful execution.
    pub typed: Option<String>,
    /// The error message, for a failed execution.
    pub error_message: Option<String>,
}

impl Observation {
    /// Builds the void-success observation the C# canonicalizer falls back to.
    #[must_use]
    pub fn success_void() -> Self {
        Self {
            success: true,
            sigma: "ok|Void|".to_string(),
            kind: Some("Void".to_string()),
            typed: Some(String::new()),
            error_message: None,
        }
    }

    /// Builds a synthesized failure observation.
    #[must_use]
    pub fn failure(message: impl Into<String>) -> Self {
        let message = message.into();
        Self {
            success: false,
            sigma: format!("err|{message}"),
            kind: None,
            typed: None,
            error_message: Some(message),
        }
    }
}

/// Builds sigma from the raw JSON stdout of lovelace-run.
pub fn from_runner_output(stdout: &str) -> Result<Observation, String> {
    let document = lovelace_json::Json::parse(stdout).map_err(|error| error.to_string())?;
    let ok = matches!(document.get("ok"), Some(In::Bool(true)));
    if !ok {
        let message = document
            .get("message")
            .and_then(In::as_str)
            .unwrap_or("(no message)")
            .to_string();
        return Ok(Observation {
            success: false,
            sigma: format!("err|{message}"),
            kind: None,
            typed: None,
            error_message: Some(message),
        });
    }
    let result = document.get("result");
    let is_object = matches!(result, Some(In::Obj(_)));
    if !is_object {
        return Ok(Observation::success_void());
    }
    let result = result.expect("checked above");
    let kind = result
        .get("kind")
        .and_then(In::as_str)
        .unwrap_or("Unknown")
        .to_string();
    let typed = result
        .get("typed")
        .and_then(In::as_str)
        .unwrap_or("")
        .to_string();
    Ok(Observation {
        success: true,
        sigma: format!("ok|{kind}|{typed}"),
        kind: Some(kind),
        typed: Some(typed),
        error_message: None,
    })
}

/// Whether sigma represents an error class.
#[must_use]
pub fn is_error(sigma: &str) -> bool {
    sigma.starts_with("err|")
}

/// The purpose-relative behaviour class (the plane key): the result kind, tagged for
/// Boolean, or the error class for failures.
#[must_use]
pub fn plane_sigma(observation: &Observation) -> String {
    if !observation.success {
        return format!(
            "err|{}",
            observation
                .error_message
                .as_deref()
                .unwrap_or("(no message)")
        );
    }
    match observation.kind.as_deref() {
        Some("Natural") => "Natural".to_string(),
        Some("Integer") => "Integer".to_string(),
        Some("Real") => "Real".to_string(),
        Some("Boolean") => {
            let typed = observation.typed.as_deref().unwrap_or("");
            let label = if typed.starts_with("True") {
                "True"
            } else {
                "False"
            };
            format!("Boolean:{label}")
        }
        Some("Text") => "Text".to_string(),
        Some("Vector") => "Vector".to_string(),
        Some("Array") => "Array".to_string(),
        Some("Function") => "Function".to_string(),
        Some("Void") => "Void".to_string(),
        Some(other) => other.to_string(),
        None => "Void".to_string(),
    }
}
