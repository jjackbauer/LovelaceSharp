//! The wire and persistence model: enums, observations, graph records, and metrics.
//!
//! Field names and value spellings match the C# records one for one, so a graph written by
//! this crate is readable by the C# tool and the other way round.

use lovelace_json::Json as In;

use crate::json::Out;

// ---------------------------------------------------------------------------
// Enumerations (serialized as strings, like the C# JsonStringEnumConverter)
// ---------------------------------------------------------------------------

/// The numeric domain of an operand.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum NumberDomain {
    /// Non-negative whole numbers.
    Natural,
    /// Signed whole numbers.
    Integer,
    /// Exact decimals and periodic fractions.
    Real,
}

impl NumberDomain {
    /// The wire spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            NumberDomain::Natural => "Natural",
            NumberDomain::Integer => "Integer",
            NumberDomain::Real => "Real",
        }
    }

    /// Parses a wire spelling.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "Natural" => Some(NumberDomain::Natural),
            "Integer" => Some(NumberDomain::Integer),
            "Real" => Some(NumberDomain::Real),
            _ => None,
        }
    }

    /// The three domains in their canonical enumeration order.
    #[must_use]
    pub fn all() -> [NumberDomain; 3] {
        [
            NumberDomain::Natural,
            NumberDomain::Integer,
            NumberDomain::Real,
        ]
    }

    /// The number of sampled literal values available for this domain.
    #[must_use]
    pub fn value_count(self, config: &KnowledgeConfig) -> usize {
        match self {
            NumberDomain::Natural => config.natural_values.len(),
            NumberDomain::Integer => config.integer_values.len(),
            NumberDomain::Real => config.real_values.len(),
        }
    }
}

/// The twelve observed operations: six arithmetic and six relations.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum Operation {
    /// Addition.
    Add,
    /// Subtraction.
    Subtract,
    /// Multiplication.
    Multiply,
    /// Division.
    Divide,
    /// Remainder.
    Modulo,
    /// Exponentiation.
    Power,
    /// Equality.
    Equal,
    /// Inequality.
    NotEqual,
    /// Strict greater-than.
    Greater,
    /// Strict less-than.
    Less,
    /// Greater-or-equal.
    GreaterEqual,
    /// Less-or-equal.
    LessEqual,
}

impl Operation {
    /// The wire spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Operation::Add => "Add",
            Operation::Subtract => "Subtract",
            Operation::Multiply => "Multiply",
            Operation::Divide => "Divide",
            Operation::Modulo => "Modulo",
            Operation::Power => "Power",
            Operation::Equal => "Equal",
            Operation::NotEqual => "NotEqual",
            Operation::Greater => "Greater",
            Operation::Less => "Less",
            Operation::GreaterEqual => "GreaterEqual",
            Operation::LessEqual => "LessEqual",
        }
    }

    /// Parses a wire spelling.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        Some(match text {
            "Add" => Operation::Add,
            "Subtract" => Operation::Subtract,
            "Multiply" => Operation::Multiply,
            "Divide" => Operation::Divide,
            "Modulo" => Operation::Modulo,
            "Power" => Operation::Power,
            "Equal" => Operation::Equal,
            "NotEqual" => Operation::NotEqual,
            "Greater" => Operation::Greater,
            "Less" => Operation::Less,
            "GreaterEqual" => Operation::GreaterEqual,
            "LessEqual" => Operation::LessEqual,
            _ => return None,
        })
    }

    /// The infix symbol used inside a sample script.
    #[must_use]
    pub fn symbol(self) -> &'static str {
        match self {
            Operation::Add => "+",
            Operation::Subtract => "-",
            Operation::Multiply => "*",
            Operation::Divide => "/",
            Operation::Modulo => "%",
            Operation::Power => "^",
            Operation::Equal => "==",
            Operation::NotEqual => "!=",
            Operation::Greater => ">",
            Operation::Less => "<",
            Operation::GreaterEqual => ">=",
            Operation::LessEqual => "<=",
        }
    }

    /// Whether the operation belongs to the arithmetic family used for coverage cells.
    #[must_use]
    pub fn is_arithmetic(self) -> bool {
        matches!(
            self,
            Operation::Add
                | Operation::Subtract
                | Operation::Multiply
                | Operation::Divide
                | Operation::Modulo
                | Operation::Power
        )
    }
}

/// How a sample entered the batch.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum SamplingKind {
    /// A seeded Monte Carlo draw.
    Random,
    /// A deterministic one-dimensional sweep.
    Sweep,
    /// A frontier-guided bisection step.
    Refine,
    /// A held-out validation probe.
    Validate,
}

impl SamplingKind {
    /// The wire spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            SamplingKind::Random => "Random",
            SamplingKind::Sweep => "Sweep",
            SamplingKind::Refine => "Refine",
            SamplingKind::Validate => "Validate",
        }
    }

    /// Parses a wire spelling.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "Random" => Some(SamplingKind::Random),
            "Sweep" => Some(SamplingKind::Sweep),
            "Refine" => Some(SamplingKind::Refine),
            "Validate" => Some(SamplingKind::Validate),
            _ => None,
        }
    }
}

/// Confidence levels from the white paper, ordered weakest to strongest.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum Confidence {
    /// Stated but unobserved.
    Hypothesized,
    /// Seen once.
    Observed,
    /// Seen more than once.
    Repeated,
    /// Localized between two probes.
    Bounded,
    /// Localized twice at two step sizes.
    Conformant,
    /// Proven.
    Proven,
}

impl Confidence {
    /// The wire spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Confidence::Hypothesized => "Hypothesized",
            Confidence::Observed => "Observed",
            Confidence::Repeated => "Repeated",
            Confidence::Bounded => "Bounded",
            Confidence::Conformant => "Conformant",
            Confidence::Proven => "Proven",
        }
    }

    /// Parses a wire spelling.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        Some(match text {
            "Hypothesized" => Confidence::Hypothesized,
            "Observed" => Confidence::Observed,
            "Repeated" => Confidence::Repeated,
            "Bounded" => Confidence::Bounded,
            "Conformant" => Confidence::Conformant,
            "Proven" => Confidence::Proven,
            _ => return None,
        })
    }
}

/// Frontier kind tags for the open-world rule.
pub mod frontier_kinds {
    /// A boundary whose two sides are further apart than one integer step.
    pub const UNRESOLVED_BOUNDARY: &str = "unresolved-boundary";
    /// A plane with fewer samples than C4 requires.
    pub const LOW_SUPPORT: &str = "low-support";
    /// An arithmetic domain pair that was never sampled.
    pub const WEAK_DIMENSION: &str = "weak-dimension";
    /// An interval no probe reached.
    pub const UNSAMPLED_INTERVAL: &str = "unsampled-interval";
}

// ---------------------------------------------------------------------------
// Operand / sample
// ---------------------------------------------------------------------------

/// An operand: its numeric domain and its Lovelace literal text.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Operand {
    /// The operand's numeric domain.
    pub domain: NumberDomain,
    /// The operand's Lovelace literal text.
    pub literal: String,
}

impl Operand {
    /// A natural operand.
    #[must_use]
    pub fn natural(literal: impl Into<String>) -> Self {
        Self {
            domain: NumberDomain::Natural,
            literal: literal.into(),
        }
    }

    /// An integer operand.
    #[must_use]
    pub fn integer(literal: impl Into<String>) -> Self {
        Self {
            domain: NumberDomain::Integer,
            literal: literal.into(),
        }
    }

    /// A real operand.
    #[must_use]
    pub fn real(literal: impl Into<String>) -> Self {
        Self {
            domain: NumberDomain::Real,
            literal: literal.into(),
        }
    }

    /// Renders the record in the wire format.
    #[must_use]
    pub fn to_json(&self) -> Out {
        Out::obj([
            ("domain", Out::str(self.domain.as_str())),
            ("literal", Out::str(self.literal.clone())),
        ])
    }

    /// Parses the record from the wire format.
    pub fn from_json(value: &In) -> Result<Self, String> {
        Ok(Self {
            domain: NumberDomain::parse(&str_member(value, "domain")?)
                .ok_or_else(|| "unknown operand domain".to_string())?,
            literal: str_member(value, "literal")?,
        })
    }
}

/// One executed sample: the coordinate, its observation sigma, and its provenance.
#[derive(Clone, Debug)]
pub struct SampleRecord {
    /// Zero-based position in the graph's sample list.
    pub index: i32,
    /// The executed script text.
    pub script: String,
    /// The operation under observation.
    pub op: Operation,
    /// The left operand.
    pub left: Operand,
    /// The right operand.
    pub right: Operand,
    /// The sweep family this sample belongs to, if any.
    pub sweep_id: Option<String>,
    /// Which side of the expression was swept.
    pub swept_side: Option<String>,
    /// The swept operand's literal, used as the axis coordinate.
    pub axis_pos: Option<String>,
    /// The behavioural plane class the observation belongs to.
    pub sigma: String,
    /// Whether the runner reported a successful result.
    pub success: bool,
    /// The result kind reported by the runner.
    pub kind: Option<String>,
    /// The result's typed rendering reported by the runner.
    pub typed: Option<String>,
    /// The error message when the script failed.
    pub error_message: Option<String>,
    /// How the sample entered the batch.
    pub sampling_kind: SamplingKind,
    /// The proposal density weight of the coordinate.
    pub weight: f64,
}

impl SampleRecord {
    /// Renders the record in the wire format.
    #[must_use]
    pub fn to_json(&self) -> Out {
        Out::obj([
            ("index", Out::Int(i64::from(self.index))),
            ("script", Out::str(self.script.clone())),
            ("op", Out::str(self.op.as_str())),
            ("left", self.left.to_json()),
            ("right", self.right.to_json()),
            ("sweepId", Out::opt_str(self.sweep_id.as_deref())),
            ("sweptSide", Out::opt_str(self.swept_side.as_deref())),
            ("axisPos", Out::opt_str(self.axis_pos.as_deref())),
            ("sigma", Out::str(self.sigma.clone())),
            ("success", Out::Bool(self.success)),
            ("kind", Out::opt_str(self.kind.as_deref())),
            ("typed", Out::opt_str(self.typed.as_deref())),
            ("errorMessage", Out::opt_str(self.error_message.as_deref())),
            ("samplingKind", Out::str(self.sampling_kind.as_str())),
            ("weight", Out::Float(self.weight)),
        ])
    }

    /// Parses the record from the wire format.
    pub fn from_json(value: &In) -> Result<Self, String> {
        Ok(Self {
            index: i32_member(value, "index")?,
            script: str_member(value, "script")?,
            op: Operation::parse(&str_member(value, "op")?)
                .ok_or_else(|| "unknown operation".to_string())?,
            left: Operand::from_json(member(value, "left")?)?,
            right: Operand::from_json(member(value, "right")?)?,
            sweep_id: opt_str_member(value, "sweepId"),
            swept_side: opt_str_member(value, "sweptSide"),
            axis_pos: opt_str_member(value, "axisPos"),
            sigma: str_member(value, "sigma")?,
            success: bool_member(value, "success")?,
            kind: opt_str_member(value, "kind"),
            typed: opt_str_member(value, "typed"),
            error_message: opt_str_member(value, "errorMessage"),
            sampling_kind: SamplingKind::parse(&str_member(value, "samplingKind")?)
                .ok_or_else(|| "unknown sampling kind".to_string())?,
            weight: float_member(value, "weight")?,
        })
    }
}

// ---------------------------------------------------------------------------
// Graph model
// ---------------------------------------------------------------------------

/// A behaviour plane: one canonical observation class and its support.
#[derive(Clone, Debug)]
pub struct Plane {
    /// The canonical observation class.
    pub sigma: String,
    /// The result kind of the plane's first sample.
    pub kind: Option<String>,
    /// The error message of the plane's first sample.
    pub error_message: Option<String>,
    /// The number of samples in the plane.
    pub support: i32,
    /// The confidence the support justifies.
    pub confidence: Confidence,
    /// The sample indices in the plane, ascending.
    pub sample_indices: Vec<i32>,
}

impl Plane {
    /// Renders the record in the wire format.
    #[must_use]
    pub fn to_json(&self) -> Out {
        Out::obj([
            ("sigma", Out::str(self.sigma.clone())),
            ("kind", Out::opt_str(self.kind.as_deref())),
            ("errorMessage", Out::opt_str(self.error_message.as_deref())),
            ("support", Out::Int(i64::from(self.support))),
            ("confidence", Out::str(self.confidence.as_str())),
            (
                "sampleIndices",
                Out::Arr(
                    self.sample_indices
                        .iter()
                        .map(|index| Out::Int(i64::from(*index)))
                        .collect(),
                ),
            ),
        ])
    }

    /// Parses the record from the wire format.
    pub fn from_json(value: &In) -> Result<Self, String> {
        Ok(Self {
            sigma: str_member(value, "sigma")?,
            kind: opt_str_member(value, "kind"),
            error_message: opt_str_member(value, "errorMessage"),
            support: i32_member(value, "support")?,
            confidence: Confidence::parse(&str_member(value, "confidence")?)
                .ok_or_else(|| "unknown confidence".to_string())?,
            sample_indices: array_member(value, "sampleIndices")?
                .iter()
                .map(|item| match item {
                    In::Num(text) => text
                        .parse::<i32>()
                        .map_err(|_| "sample index is not an integer".to_string()),
                    _ => Err("sample index is not a number".to_string()),
                })
                .collect::<Result<Vec<i32>, String>>()?,
        })
    }
}

/// A fitted guard on a swept operand variable.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Guard {
    /// The swept variable.
    pub variable: String,
    /// The relation of the guard: one of ">", "==", "?".
    pub relop: String,
    /// The threshold literal.
    pub threshold: String,
    /// The guard rendered as an expression.
    pub expression: String,
    /// The guard family: "threshold", "equality", or "composite".
    pub kind: String,
}

impl Guard {
    /// Renders the record in the wire format.
    #[must_use]
    pub fn to_json(&self) -> Out {
        Out::obj([
            ("variable", Out::str(self.variable.clone())),
            ("relop", Out::str(self.relop.clone())),
            ("threshold", Out::str(self.threshold.clone())),
            ("expression", Out::str(self.expression.clone())),
            ("kind", Out::str(self.kind.clone())),
        ])
    }

    /// Parses the record from the wire format.
    pub fn from_json(value: &In) -> Result<Self, String> {
        Ok(Self {
            variable: str_member(value, "variable")?,
            relop: str_member(value, "relop")?,
            threshold: str_member(value, "threshold")?,
            expression: str_member(value, "expression")?,
            kind: str_member(value, "kind")?,
        })
    }
}

/// A single bounding sample on one side of a boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BoundEvidence {
    /// The side of the boundary: "low" or "high".
    pub side: String,
    /// The index of the bounding sample.
    pub sample_index: i32,
    /// The swept position of the bounding sample.
    pub axis_pos: String,
    /// The plane class of the bounding sample.
    pub sigma: String,
}

impl BoundEvidence {
    /// Renders the record in the wire format.
    #[must_use]
    pub fn to_json(&self) -> Out {
        Out::obj([
            ("side", Out::str(self.side.clone())),
            ("sampleIndex", Out::Int(i64::from(self.sample_index))),
            ("axisPos", Out::str(self.axis_pos.clone())),
            ("sigma", Out::str(self.sigma.clone())),
        ])
    }

    /// Parses the record from the wire format.
    pub fn from_json(value: &In) -> Result<Self, String> {
        Ok(Self {
            side: str_member(value, "side")?,
            sample_index: i32_member(value, "sampleIndex")?,
            axis_pos: str_member(value, "axisPos")?,
            sigma: str_member(value, "sigma")?,
        })
    }
}

/// A boundary between two adjacent behaviour planes.
#[derive(Clone, Debug)]
pub struct BoundaryEdge {
    /// The stable identifier.
    pub id: String,
    /// The plane on the low side.
    pub from_plane: String,
    /// The plane on the high side.
    pub to_plane: String,
    /// The operation name.
    pub operation: String,
    /// The swept side.
    pub swept_side: String,
    /// The fitted guard.
    pub guard: Guard,
    /// The confidence the confirmations justify.
    pub confidence: Confidence,
    /// The bounding samples.
    pub evidence: Vec<BoundEvidence>,
}

impl BoundaryEdge {
    /// Renders the record in the wire format.
    #[must_use]
    pub fn to_json(&self) -> Out {
        Out::obj([
            ("id", Out::str(self.id.clone())),
            ("fromPlane", Out::str(self.from_plane.clone())),
            ("toPlane", Out::str(self.to_plane.clone())),
            ("operation", Out::str(self.operation.clone())),
            ("sweptSide", Out::str(self.swept_side.clone())),
            ("guard", self.guard.to_json()),
            ("confidence", Out::str(self.confidence.as_str())),
            (
                "evidence",
                Out::Arr(self.evidence.iter().map(BoundEvidence::to_json).collect()),
            ),
        ])
    }

    /// Parses the record from the wire format.
    pub fn from_json(value: &In) -> Result<Self, String> {
        Ok(Self {
            id: str_member(value, "id")?,
            from_plane: str_member(value, "fromPlane")?,
            to_plane: str_member(value, "toPlane")?,
            operation: str_member(value, "operation")?,
            swept_side: str_member(value, "sweptSide")?,
            guard: Guard::from_json(member(value, "guard")?)?,
            confidence: Confidence::parse(&str_member(value, "confidence")?)
                .ok_or_else(|| "unknown confidence".to_string())?,
            evidence: array_member(value, "evidence")?
                .iter()
                .map(BoundEvidence::from_json)
                .collect::<Result<Vec<_>, String>>()?,
        })
    }
}

/// An explicitly unresolved region.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Frontier {
    /// The stable identifier.
    pub id: String,
    /// The frontier kind.
    pub kind: String,
    /// The human-readable description.
    pub description: String,
    /// The operation, when the frontier is axis-local.
    pub operation: Option<String>,
    /// The swept side, when the frontier is axis-local.
    pub swept_side: Option<String>,
    /// The anchor literal, when the frontier is axis-local.
    pub anchor: Option<String>,
    /// The low end of the unresolved interval.
    pub low: Option<String>,
    /// The high end of the unresolved interval.
    pub high: Option<String>,
}

impl Frontier {
    /// Renders the record in the wire format.
    #[must_use]
    pub fn to_json(&self) -> Out {
        Out::obj([
            ("id", Out::str(self.id.clone())),
            ("kind", Out::str(self.kind.clone())),
            ("description", Out::str(self.description.clone())),
            ("operation", Out::opt_str(self.operation.as_deref())),
            ("sweptSide", Out::opt_str(self.swept_side.as_deref())),
            ("anchor", Out::opt_str(self.anchor.as_deref())),
            ("low", Out::opt_str(self.low.as_deref())),
            ("high", Out::opt_str(self.high.as_deref())),
        ])
    }

    /// Parses the record from the wire format.
    pub fn from_json(value: &In) -> Result<Self, String> {
        Ok(Self {
            id: str_member(value, "id")?,
            kind: str_member(value, "kind")?,
            description: str_member(value, "description")?,
            operation: opt_str_member(value, "operation"),
            swept_side: opt_str_member(value, "sweptSide"),
            anchor: opt_str_member(value, "anchor"),
            low: opt_str_member(value, "low"),
            high: opt_str_member(value, "high"),
        })
    }
}

/// Per (operation, domain-pair) sample coverage for C4.
#[derive(Clone, Debug)]
pub struct CoverageCell {
    /// The coverage key.
    pub key: String,
    /// The operation name.
    pub operation: String,
    /// The left operand's domain.
    pub left_domain: String,
    /// The right operand's domain.
    pub right_domain: String,
    /// How many samples landed in the cell.
    pub samples: i32,
    /// The summed proposal weight of those samples.
    pub weight: f64,
}

impl CoverageCell {
    /// Renders the record in the wire format.
    #[must_use]
    pub fn to_json(&self) -> Out {
        Out::obj([
            ("key", Out::str(self.key.clone())),
            ("operation", Out::str(self.operation.clone())),
            ("leftDomain", Out::str(self.left_domain.clone())),
            ("rightDomain", Out::str(self.right_domain.clone())),
            ("samples", Out::Int(i64::from(self.samples))),
            ("weight", Out::Float(self.weight)),
        ])
    }

    /// Parses the record from the wire format.
    pub fn from_json(value: &In) -> Result<Self, String> {
        Ok(Self {
            key: str_member(value, "key")?,
            operation: str_member(value, "operation")?,
            left_domain: str_member(value, "leftDomain")?,
            right_domain: str_member(value, "rightDomain")?,
            samples: i32_member(value, "samples")?,
            weight: float_member(value, "weight")?,
        })
    }
}

/// Convergence metrics C1-C4 plus the stopping result.
#[derive(Clone, Debug)]
pub struct Metrics {
    /// Whether all four criteria hold.
    pub converged: bool,
    /// The explicit stopping reason.
    pub stop_reason: Option<String>,
    /// The number of samples measured.
    pub total_samples: i32,
    /// The number of planes.
    pub plane_count: i32,
    /// C1: new planes in the last K batches.
    pub c1_new_planes_last_k: i32,
    /// C1: new-plane rate over the last K batches.
    pub c1_new_plane_rate: f64,
    /// C1 verdict.
    pub c1_saturated: bool,
    /// C2: total boundaries.
    pub c2_total_boundaries: i32,
    /// C2: boundaries with at least the minimum confirmations.
    pub c2_stable_boundaries: i32,
    /// C2 verdict.
    pub c2_stable: bool,
    /// C3: held-out probes matched to a localized boundary.
    pub c3_held_out_count: i32,
    /// C3: probes the guard predicted correctly.
    pub c3_agreed_count: i32,
    /// C3: agreement ratio.
    pub c3_agreement: f64,
    /// C3 verdict.
    pub c3_agreed: bool,
    /// C4 verdict.
    pub c4_covered: bool,
    /// Per-cell coverage.
    pub coverage: Vec<CoverageCell>,
}

impl Metrics {
    /// Renders the record in the wire format.
    #[must_use]
    pub fn to_json(&self) -> Out {
        Out::obj([
            ("converged", Out::Bool(self.converged)),
            ("stopReason", Out::opt_str(self.stop_reason.as_deref())),
            ("totalSamples", Out::Int(i64::from(self.total_samples))),
            ("planeCount", Out::Int(i64::from(self.plane_count))),
            (
                "c1NewPlanesLastK",
                Out::Int(i64::from(self.c1_new_planes_last_k)),
            ),
            ("c1NewPlaneRate", Out::Float(self.c1_new_plane_rate)),
            ("c1Saturated", Out::Bool(self.c1_saturated)),
            (
                "c2TotalBoundaries",
                Out::Int(i64::from(self.c2_total_boundaries)),
            ),
            (
                "c2StableBoundaries",
                Out::Int(i64::from(self.c2_stable_boundaries)),
            ),
            ("c2Stable", Out::Bool(self.c2_stable)),
            (
                "c3HeldOutCount",
                Out::Int(i64::from(self.c3_held_out_count)),
            ),
            ("c3AgreedCount", Out::Int(i64::from(self.c3_agreed_count))),
            ("c3Agreement", Out::Float(self.c3_agreement)),
            ("c3Agreed", Out::Bool(self.c3_agreed)),
            ("c4Covered", Out::Bool(self.c4_covered)),
            (
                "coverage",
                Out::Arr(self.coverage.iter().map(CoverageCell::to_json).collect()),
            ),
        ])
    }

    /// Parses the record from the wire format.
    pub fn from_json(value: &In) -> Result<Self, String> {
        Ok(Self {
            converged: bool_member(value, "converged")?,
            stop_reason: opt_str_member(value, "stopReason"),
            total_samples: i32_member(value, "totalSamples")?,
            plane_count: i32_member(value, "planeCount")?,
            c1_new_planes_last_k: i32_member(value, "c1NewPlanesLastK")?,
            c1_new_plane_rate: float_member(value, "c1NewPlaneRate")?,
            c1_saturated: bool_member(value, "c1Saturated")?,
            c2_total_boundaries: i32_member(value, "c2TotalBoundaries")?,
            c2_stable_boundaries: i32_member(value, "c2StableBoundaries")?,
            c2_stable: bool_member(value, "c2Stable")?,
            c3_held_out_count: i32_member(value, "c3HeldOutCount")?,
            c3_agreed_count: i32_member(value, "c3AgreedCount")?,
            c3_agreement: float_member(value, "c3Agreement")?,
            c3_agreed: bool_member(value, "c3Agreed")?,
            c4_covered: bool_member(value, "c4Covered")?,
            coverage: array_member(value, "coverage")?
                .iter()
                .map(CoverageCell::from_json)
                .collect::<Result<Vec<_>, String>>()?,
        })
    }
}

/// The full domain/proposal/threshold configuration: the only agent input.
#[derive(Clone, Debug)]
pub struct KnowledgeConfig {
    /// The determinism seed.
    pub seed: i64,
    /// How many specs one batch carries.
    pub batch_size: i32,
    /// The sample budget.
    pub max_samples: i32,
    /// C1: the new-plane-rate ceiling.
    pub c1_new_plane_rate_threshold: f64,
    /// C1: how many recent batches the rate covers.
    pub c1_window_batches: i32,
    /// C2: confirmations a boundary needs to count as stable.
    pub c2_min_confirmations: i32,
    /// C3: the agreement ratio C3 demands.
    pub c3_agreement_threshold: f64,
    /// C4: the minimum support per boundary-adjacent plane.
    pub c4_min_support_per_plane: i32,
    /// How many random draws the breadth phase includes.
    pub min_random_samples: i32,
    /// The natural literals available to random draws.
    pub natural_values: Vec<String>,
    /// The integer literals available to random draws.
    pub integer_values: Vec<String>,
    /// The real literals available to random draws.
    pub real_values: Vec<String>,
    /// The operations under observation.
    pub operations: Vec<Operation>,
    /// The operations that get deterministic sweeps.
    pub sweep_operations: Vec<Operation>,
}

impl KnowledgeConfig {
    /// The default configuration for the first convergence run (C# DefaultConfig).
    #[must_use]
    pub fn default_config() -> Self {
        let naturals = (0..=12).map(|value| value.to_string()).collect();
        let integers = (-6..=6).map(|value| value.to_string()).collect();
        let reals = [
            "-1.5", "-0.5", "0.25", "0.5", "0.(3)", "0.1(6)", "1.5", "2.5",
        ]
        .iter()
        .map(|value| (*value).to_string())
        .collect();
        Self {
            seed: 20_240_617,
            batch_size: 64,
            max_samples: 700,
            c1_new_plane_rate_threshold: 0.01,
            c1_window_batches: 3,
            c2_min_confirmations: 2,
            c3_agreement_threshold: 1.0,
            c4_min_support_per_plane: 2,
            min_random_samples: 100,
            natural_values: naturals,
            integer_values: integers,
            real_values: reals,
            operations: vec![
                Operation::Add,
                Operation::Subtract,
                Operation::Multiply,
                Operation::Divide,
                Operation::Modulo,
                Operation::Power,
                Operation::Equal,
                Operation::NotEqual,
                Operation::Greater,
                Operation::Less,
                Operation::GreaterEqual,
                Operation::LessEqual,
            ],
            sweep_operations: vec![
                Operation::Subtract,
                Operation::Divide,
                Operation::Modulo,
                Operation::Greater,
                Operation::Less,
            ],
        }
    }

    /// Returns a copy with the default seed replaced.
    #[must_use]
    pub fn with_seed(mut self, seed: i64) -> Self {
        self.seed = seed;
        self
    }

    /// Renders the record in the wire format.
    #[must_use]
    pub fn to_json(&self) -> Out {
        Out::obj([
            ("seed", Out::Int(self.seed)),
            ("batchSize", Out::Int(i64::from(self.batch_size))),
            ("maxSamples", Out::Int(i64::from(self.max_samples))),
            (
                "c1NewPlaneRateThreshold",
                Out::Float(self.c1_new_plane_rate_threshold),
            ),
            (
                "c1WindowBatches",
                Out::Int(i64::from(self.c1_window_batches)),
            ),
            (
                "c2MinConfirmations",
                Out::Int(i64::from(self.c2_min_confirmations)),
            ),
            (
                "c3AgreementThreshold",
                Out::Float(self.c3_agreement_threshold),
            ),
            (
                "c4MinSupportPerPlane",
                Out::Int(i64::from(self.c4_min_support_per_plane)),
            ),
            (
                "minRandomSamples",
                Out::Int(i64::from(self.min_random_samples)),
            ),
            ("naturalValues", string_array(&self.natural_values)),
            ("integerValues", string_array(&self.integer_values)),
            ("realValues", string_array(&self.real_values)),
            (
                "operations",
                Out::Arr(
                    self.operations
                        .iter()
                        .map(|operation| Out::str(operation.as_str()))
                        .collect(),
                ),
            ),
            (
                "sweepOperations",
                Out::Arr(
                    self.sweep_operations
                        .iter()
                        .map(|operation| Out::str(operation.as_str()))
                        .collect(),
                ),
            ),
        ])
    }

    /// Parses the record from the wire format.
    pub fn from_json(value: &In) -> Result<Self, String> {
        Ok(Self {
            seed: i64_member(value, "seed")?,
            batch_size: i32_member(value, "batchSize")?,
            max_samples: i32_member(value, "maxSamples")?,
            c1_new_plane_rate_threshold: float_member(value, "c1NewPlaneRateThreshold")?,
            c1_window_batches: i32_member(value, "c1WindowBatches")?,
            c2_min_confirmations: i32_member(value, "c2MinConfirmations")?,
            c3_agreement_threshold: float_member(value, "c3AgreementThreshold")?,
            c4_min_support_per_plane: i32_member(value, "c4MinSupportPerPlane")?,
            min_random_samples: i32_member(value, "minRandomSamples")?,
            natural_values: string_array_member(value, "naturalValues")?,
            integer_values: string_array_member(value, "integerValues")?,
            real_values: string_array_member(value, "realValues")?,
            operations: operation_array_member(value, "operations")?,
            sweep_operations: operation_array_member(value, "sweepOperations")?,
        })
    }
}

/// The persisted graph: samples (provenance) plus the reduced structure.
#[derive(Clone, Debug)]
pub struct Graph {
    /// The determinism seed.
    pub seed: i64,
    /// The persistence format version.
    pub version: i32,
    /// The configuration the graph was produced under.
    pub config: KnowledgeConfig,
    /// Every executed sample.
    pub samples: Vec<SampleRecord>,
    /// The clustered behaviour planes.
    pub planes: Vec<Plane>,
    /// The localized boundaries.
    pub boundaries: Vec<BoundaryEdge>,
    /// The unresolved regions.
    pub frontiers: Vec<Frontier>,
    /// The convergence metrics, once measured.
    pub metrics: Option<Metrics>,
}

impl Graph {
    /// The persistence format version this crate writes.
    pub const VERSION: i32 = 1;

    /// Builds an empty graph for a configuration.
    #[must_use]
    pub fn new(config: KnowledgeConfig) -> Self {
        Self {
            seed: config.seed,
            version: Self::VERSION,
            config,
            samples: Vec::new(),
            planes: Vec::new(),
            boundaries: Vec::new(),
            frontiers: Vec::new(),
            metrics: None,
        }
    }

    /// Renders the graph as the C# WriteIndented document.
    #[must_use]
    pub fn to_json(&self) -> Out {
        Out::obj([
            ("seed", Out::Int(self.seed)),
            ("version", Out::Int(i64::from(self.version))),
            ("config", self.config.to_json()),
            (
                "samples",
                Out::Arr(self.samples.iter().map(SampleRecord::to_json).collect()),
            ),
            (
                "planes",
                Out::Arr(self.planes.iter().map(Plane::to_json).collect()),
            ),
            (
                "boundaries",
                Out::Arr(self.boundaries.iter().map(BoundaryEdge::to_json).collect()),
            ),
            (
                "frontiers",
                Out::Arr(self.frontiers.iter().map(Frontier::to_json).collect()),
            ),
            (
                "metrics",
                match &self.metrics {
                    Some(metrics) => metrics.to_json(),
                    None => Out::Null,
                },
            ),
        ])
    }

    /// Parses a persisted graph document.
    pub fn from_json(value: &In) -> Result<Self, String> {
        Ok(Self {
            seed: i64_member(value, "seed")?,
            version: i32_member(value, "version")?,
            config: KnowledgeConfig::from_json(member(value, "config")?)?,
            samples: array_member(value, "samples")?
                .iter()
                .map(SampleRecord::from_json)
                .collect::<Result<Vec<_>, String>>()?,
            planes: array_member(value, "planes")?
                .iter()
                .map(Plane::from_json)
                .collect::<Result<Vec<_>, String>>()?,
            boundaries: array_member(value, "boundaries")?
                .iter()
                .map(BoundaryEdge::from_json)
                .collect::<Result<Vec<_>, String>>()?,
            frontiers: array_member(value, "frontiers")?
                .iter()
                .map(Frontier::from_json)
                .collect::<Result<Vec<_>, String>>()?,
            metrics: match value.get("metrics") {
                None | Some(In::Null) => None,
                Some(metrics) => Some(Metrics::from_json(metrics)?),
            },
        })
    }
}

// ---------------------------------------------------------------------------
// Input helpers
// ---------------------------------------------------------------------------

fn member<'a>(value: &'a In, key: &str) -> Result<&'a In, String> {
    value
        .get(key)
        .ok_or_else(|| format!("missing member '{key}'"))
}

fn str_member(value: &In, key: &str) -> Result<String, String> {
    member(value, key)?
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| format!("member '{key}' is not a string"))
}

fn opt_str_member(value: &In, key: &str) -> Option<String> {
    match value.get(key) {
        Some(In::Str(text)) => Some(text.clone()),
        _ => None,
    }
}

fn number_text<'a>(value: &'a In, key: &str) -> Result<&'a str, String> {
    match member(value, key)? {
        In::Num(text) => Ok(text.as_str()),
        _ => Err(format!("member '{key}' is not a number")),
    }
}

fn i64_member(value: &In, key: &str) -> Result<i64, String> {
    number_text(value, key)?
        .parse::<i64>()
        .map_err(|_| format!("member '{key}' is not an integer"))
}

fn i32_member(value: &In, key: &str) -> Result<i32, String> {
    number_text(value, key)?
        .parse::<i32>()
        .map_err(|_| format!("member '{key}' is not a small integer"))
}

fn float_member(value: &In, key: &str) -> Result<f64, String> {
    number_text(value, key)?
        .parse::<f64>()
        .map_err(|_| format!("member '{key}' is not a double"))
}

fn bool_member(value: &In, key: &str) -> Result<bool, String> {
    match member(value, key)? {
        In::Bool(flag) => Ok(*flag),
        _ => Err(format!("member '{key}' is not a boolean")),
    }
}

fn array_member<'a>(value: &'a In, key: &str) -> Result<&'a [In], String> {
    member(value, key)?
        .as_arr()
        .ok_or_else(|| format!("member '{key}' is not an array"))
}

fn string_array_member(value: &In, key: &str) -> Result<Vec<String>, String> {
    array_member(value, key)?
        .iter()
        .map(|item| {
            item.as_str()
                .map(str::to_string)
                .ok_or_else(|| format!("member '{key}' has a non-string element"))
        })
        .collect()
}

fn operation_array_member(value: &In, key: &str) -> Result<Vec<Operation>, String> {
    array_member(value, key)?
        .iter()
        .map(|item| {
            let text = item
                .as_str()
                .ok_or_else(|| format!("member '{key}' has a non-string element"))?;
            Operation::parse(text).ok_or_else(|| format!("unknown operation '{text}'"))
        })
        .collect()
}

fn string_array(values: &[String]) -> Out {
    Out::Arr(values.iter().map(|value| Out::str(value.clone())).collect())
}
