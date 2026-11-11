//! The record-schema registry: the Rust counterpart of Lovelace.Suite.RecordSchemas.
//!
//! The table is data, not reflection: it names the well-known result records the kernel emits and
//! declares each record's fields in wire order, with the payload kind a consumer should expect.
//! Field names are snake_case because they are part of the wire contract; the kind strings are the
//! reference's own vocabulary (Enum, Symbolic, Boolean|Null, Record|Null, ...), where a "|Null"
//! alternative records that the field may carry a Null value.
//!
//! The registry validates the structured value the runner publishes:
//! {"kind":"Record","type":...,"fields":[{"name":...,"value":...}]}. As in the reference,
//! validate() checks only the SHAPE - every declared field present and no undeclared field
//! carried - and returns the violations as a value rather than raising. A record whose type is not
//! registered is simply unknown: for_type() returns None and validate() claims nothing. Whether a
//! value of a declared kind is actually carried is a consumer's concern; the reference does not
//! enforce it either.

use lovelace_json::Json;

/// One declared field of a well-known record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecordFieldSchema {
    /// The field name exactly as the kernel emits it (snake_case, part of the contract).
    pub name: &'static str,
    /// The payload kind a consumer should expect, for docs and tooling. A "T|Null" alternative
    /// marks a field whose value may be Null rather than an absent field.
    pub kind: &'static str,
}

impl RecordFieldSchema {
    /// Declares one field.
    pub const fn new(name: &'static str, kind: &'static str) -> Self {
        Self { name, kind }
    }
}

/// The declared shape of a well-known result record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecordSchema {
    /// The record type name (SolveResult, Diagnostic, ...).
    pub type_name: &'static str,
    /// The declared fields, in wire order.
    pub fields: &'static [RecordFieldSchema],
}

impl RecordSchema {
    /// Declares one record schema.
    pub const fn new(type_name: &'static str, fields: &'static [RecordFieldSchema]) -> Self {
        Self { type_name, fields }
    }

    /// Field names this schema requires, in order.
    pub fn field_names(&self) -> Vec<&'static str> {
        self.fields.iter().map(|f| f.name).collect()
    }

    /// Checks a record against this schema: every declared field present, and no undeclared field
    /// carried. Returns the violations (empty means conformant) - a value, not an exception, so a
    /// host can report schema drift as a diagnostic.
    pub fn validate(&self, record: &Json) -> Vec<String> {
        let mut violations = Vec::new();
        let actual = record_type(record).unwrap_or("");
        if actual != self.type_name {
            violations.push(format!(
                "expected record '{}', got '{}'",
                self.type_name, actual
            ));
        }

        let present: Vec<&str> = record_fields(record)
            .iter()
            .filter_map(field_name)
            .collect();
        for field in self.fields {
            if !present.contains(&field.name) {
                violations.push(format!(
                    "{}: missing declared field '{}'",
                    self.type_name, field.name
                ));
            }
        }
        let declared = self.field_names();
        for name in &present {
            if !declared.contains(name) {
                violations.push(format!("{}: undeclared field '{}'", self.type_name, name));
            }
        }
        violations
    }
}

/// Statically registered schemas for the well-known result records. No reflection and no scanning:
/// the table is data, so it can drive validation, docs, Studio rendering and the DSH schema from
/// one source.
pub struct RecordSchemas;

impl RecordSchemas {
    /// Every registered schema, in the reference's declaration order.
    pub const ALL: &'static [RecordSchema] = &[
        RecordSchema::new(
            "SolveResult",
            &[
                RecordFieldSchema::new("status", "Enum"),
                RecordFieldSchema::new("variable", "Symbolic"),
                RecordFieldSchema::new("domain", "Domain"),
                RecordFieldSchema::new("complete", "Boolean"),
                RecordFieldSchema::new("completeness", "Enum"),
                RecordFieldSchema::new("solutions", "Array"),
                RecordFieldSchema::new("families", "Array"),
                RecordFieldSchema::new("common_conditions", "Array"),
                RecordFieldSchema::new("represented_count", "Integer"),
                RecordFieldSchema::new("unrepresented_count", "Integer"),
                RecordFieldSchema::new("unrepresented_reason", "Text"),
                RecordFieldSchema::new("diagnostics", "Array"),
            ],
        ),
        RecordSchema::new(
            "Solution",
            &[
                RecordFieldSchema::new("value", "Symbolic"),
                RecordFieldSchema::new("conditions", "Array"),
                RecordFieldSchema::new("multiplicity", "Integer"),
                RecordFieldSchema::new("exactness", "Enum"),
            ],
        ),
        RecordSchema::new(
            "SolutionFamily",
            &[
                RecordFieldSchema::new("template", "Symbolic"),
                RecordFieldSchema::new("parameter", "Symbolic"),
                RecordFieldSchema::new("period", "Symbolic"),
                RecordFieldSchema::new("parameter_domain", "Domain"),
                RecordFieldSchema::new("conditions", "Array"),
                RecordFieldSchema::new("exactness", "Enum"),
            ],
        ),
        // The SAME solve vocabulary as SolveResult: the Enum status, the derived complete Boolean
        // and the Completeness Enum, then the payload and diagnostics LAST.
        RecordSchema::new(
            "SystemSolveResult",
            &[
                RecordFieldSchema::new("status", "Enum"),
                RecordFieldSchema::new("domain", "Domain"),
                RecordFieldSchema::new("complete", "Boolean"),
                RecordFieldSchema::new("completeness", "Enum"),
                RecordFieldSchema::new("solutions", "Array"),
                RecordFieldSchema::new("diagnostics", "Array"),
            ],
        ),
        // The two matrix result records (inv_full / linsolve_full): the same solve vocabulary,
        // the payload in the middle, the Diagnostic array LAST.
        RecordSchema::new(
            "MatrixInverseResult",
            &[
                RecordFieldSchema::new("status", "Enum"),
                RecordFieldSchema::new("complete", "Boolean"),
                RecordFieldSchema::new("completeness", "Enum"),
                RecordFieldSchema::new("inverse", "Array"),
                RecordFieldSchema::new("conditions", "Array"),
                RecordFieldSchema::new("diagnostics", "Array"),
            ],
        ),
        RecordSchema::new(
            "MatrixSolveResult",
            &[
                RecordFieldSchema::new("status", "Enum"),
                RecordFieldSchema::new("complete", "Boolean"),
                RecordFieldSchema::new("completeness", "Enum"),
                RecordFieldSchema::new("solutions", "Array"),
                RecordFieldSchema::new("conditions", "Array"),
                RecordFieldSchema::new("diagnostics", "Array"),
            ],
        ),
        RecordSchema::new(
            "SystemSolution",
            &[
                RecordFieldSchema::new("bindings", "Array"),
                RecordFieldSchema::new("conditions", "Array"),
                RecordFieldSchema::new("exactness", "Enum"),
            ],
        ),
        RecordSchema::new(
            "Binding",
            &[
                RecordFieldSchema::new("name", "Symbolic"),
                RecordFieldSchema::new("value", "Symbolic"),
            ],
        ),
        RecordSchema::new(
            "TransformResult",
            &[
                RecordFieldSchema::new("status", "Enum"),
                RecordFieldSchema::new("original", "Symbolic"),
                RecordFieldSchema::new("expression", "Symbolic"),
                RecordFieldSchema::new("changed", "Boolean"),
                RecordFieldSchema::new("conditions", "Array"),
                RecordFieldSchema::new("steps", "Array"),
                RecordFieldSchema::new("budget_exceeded", "Boolean"),
                RecordFieldSchema::new("budget_kind", "Text"),
                RecordFieldSchema::new("diagnostics", "Array"),
            ],
        ),
        RecordSchema::new(
            "RewriteStep",
            &[
                RecordFieldSchema::new("rule_id", "Text"),
                RecordFieldSchema::new("classification", "Enum"),
                RecordFieldSchema::new("before", "Symbolic"),
                RecordFieldSchema::new("after", "Symbolic"),
                RecordFieldSchema::new("required_conditions", "Array"),
            ],
        ),
        // "exists" is three-valued (N19): Boolean true/false when determined or proven, Null when
        // the engine did not determine it - never false for "unknown".
        RecordSchema::new(
            "LimitResult",
            &[
                RecordFieldSchema::new("status", "Enum"),
                RecordFieldSchema::new("exists", "Boolean|Null"),
                RecordFieldSchema::new("value", "Symbolic|Null"),
                RecordFieldSchema::new("left", "Symbolic|Null"),
                RecordFieldSchema::new("left_conditions", "Array"),
                RecordFieldSchema::new("right", "Symbolic|Null"),
                RecordFieldSchema::new("right_conditions", "Array"),
                RecordFieldSchema::new("conditions", "Array"),
                RecordFieldSchema::new("exactness", "Enum"),
                RecordFieldSchema::new("diagnostics", "Array"),
            ],
        ),
        RecordSchema::new(
            "IntegrationResult",
            &[
                RecordFieldSchema::new("status", "Enum"),
                RecordFieldSchema::new("expression", "Symbolic"),
                RecordFieldSchema::new("conditions", "Array"),
                RecordFieldSchema::new("verified", "Boolean"),
                RecordFieldSchema::new("method", "Text"),
                RecordFieldSchema::new("verification_method", "Text"),
                RecordFieldSchema::new("exactness", "Enum"),
                RecordFieldSchema::new("diagnostics", "Array"),
            ],
        ),
        RecordSchema::new(
            "OptimizationResult",
            &[
                RecordFieldSchema::new("original", "Symbolic"),
                RecordFieldSchema::new("optimized", "Symbolic"),
                RecordFieldSchema::new("estimated_cost_before", "Integer"),
                RecordFieldSchema::new("estimated_cost_after", "Integer"),
                RecordFieldSchema::new("shared_subtrees", "Integer"),
                RecordFieldSchema::new("horner_rewrites", "Integer"),
                RecordFieldSchema::new("transformations", "Array"),
                RecordFieldSchema::new("target", "Text"),
                RecordFieldSchema::new("policy", "Text"),
                RecordFieldSchema::new("diagnostics", "Array"),
            ],
        ),
        RecordSchema::new(
            "CompilationResult",
            &[
                RecordFieldSchema::new("ir", "Text"),
                RecordFieldSchema::new("parameters", "Array"),
                RecordFieldSchema::new("result_domain", "Domain"),
                RecordFieldSchema::new("result_type", "Text"),
                RecordFieldSchema::new("precision_policy", "Text"),
                RecordFieldSchema::new("optimization_policy", "Text"),
                RecordFieldSchema::new("target", "Text"),
                RecordFieldSchema::new("mathir_version", "Integer"),
                RecordFieldSchema::new("exact", "Boolean"),
                RecordFieldSchema::new("diagnostics", "Array"),
            ],
        ),
        RecordSchema::new(
            "ParameterInfo",
            &[
                RecordFieldSchema::new("name", "Text"),
                RecordFieldSchema::new("domain", "Domain"),
            ],
        ),
        // The diagnostic vocabulary every rich record's diagnostics array carries: the category is
        // an Enum, the location is a wire record or Null, details is an Array.
        RecordSchema::new(
            "Diagnostic",
            &[
                RecordFieldSchema::new("code", "Text"),
                RecordFieldSchema::new("category", "Enum"),
                RecordFieldSchema::new("message", "Text"),
                RecordFieldSchema::new("recoverable", "Boolean"),
                RecordFieldSchema::new("location", "Record|Null"),
                RecordFieldSchema::new("details", "Array"),
            ],
        ),
        RecordSchema::new(
            "Inspection",
            &[
                RecordFieldSchema::new("type", "Text"),
                RecordFieldSchema::new("domain", "Domain|Null"),
                RecordFieldSchema::new("exact", "Boolean|Null"),
                RecordFieldSchema::new("free_symbols", "Array"),
                RecordFieldSchema::new("node_count", "Integer|Null"),
                RecordFieldSchema::new("canonical", "Text|Null"),
                RecordFieldSchema::new("pretty", "Text|Null"),
                RecordFieldSchema::new("shape", "Array|Null"),
                RecordFieldSchema::new("rank", "Integer|Null"),
                RecordFieldSchema::new("element_domain", "Domain|Null"),
                RecordFieldSchema::new("assumptions", "Array"),
                RecordFieldSchema::new("members", "Array|Null"),
            ],
        ),
        RecordSchema::new(
            "DomainCondition",
            &[
                RecordFieldSchema::new("variable", "Symbolic"),
                RecordFieldSchema::new("domain", "Domain"),
            ],
        ),
        RecordSchema::new(
            "FiniteCondition",
            &[RecordFieldSchema::new("expression", "Symbolic")],
        ),
        RecordSchema::new(
            "PredicateCondition",
            &[
                RecordFieldSchema::new("predicate", "Text"),
                RecordFieldSchema::new("expression", "Symbolic"),
            ],
        ),
        RecordSchema::new(
            "IntervalCondition",
            &[
                RecordFieldSchema::new("expression", "Symbolic"),
                RecordFieldSchema::new("lower", "Symbolic|Null"),
                RecordFieldSchema::new("lower_open", "Boolean"),
                RecordFieldSchema::new("upper", "Symbolic|Null"),
                RecordFieldSchema::new("upper_open", "Boolean"),
            ],
        ),
    ];

    /// Every registered schema, in declaration order.
    pub fn all() -> &'static [RecordSchema] {
        Self::ALL
    }

    /// The schema for a record type name, or None when the registry does not know it.
    pub fn for_type(type_name: &str) -> Option<&'static RecordSchema> {
        Self::ALL.iter().find(|s| s.type_name == type_name)
    }

    /// Schema violations for a record of a known type; empty for an unknown type.
    pub fn validate(record: &Json) -> Vec<String> {
        match record_type(record).and_then(Self::for_type) {
            Some(schema) => schema.validate(record),
            None => Vec::new(),
        }
    }
}

/// The declared type name of a structured record value, when it carries one.
fn record_type(record: &Json) -> Option<&str> {
    record.get("type").and_then(Json::as_str)
}

/// The fields array of a structured record value (empty when the shape is malformed).
fn record_fields(record: &Json) -> &[Json] {
    record.get("fields").and_then(Json::as_arr).unwrap_or(&[])
}

/// The declared name of one {"name":...,"value":...} field entry.
fn field_name(field: &Json) -> Option<&str> {
    field.get("name").and_then(Json::as_str)
}
