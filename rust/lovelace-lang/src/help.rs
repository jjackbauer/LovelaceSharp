//! The plugin-aware help/introspection surface: the Rust mirror of the reference
//! `Lovelace.Suite.HelpService`.
//!
//! The reference derives every help rendering from the live function registry plus the builtin
//! descriptors; this crate's registry (`crate::registry::BUILTINS`) is the same DATA, so the
//! service here is a renderer, not a second source of truth. The category-alias table is the one
//! user-facing dataset the reference hard-codes, transcribed below.

use crate::registry::{self, BuiltinInfo};

/// The fixed help/funcs categories, in the reference's display order.
pub const CATEGORY_ORDER: &[&str] = &[
    "Language",
    "Arrays",
    "Numerics",
    "Symbolics",
    "Calculus",
    "Solving",
    "Linear Algebra",
    "Optimization",
    "Compilation",
    "DSP",
    "Introspection",
    "Core",
];

/// Reasonable synonyms so a user can find a category without knowing its exact registered
/// spelling (reference `HelpService.CategoryAliases`, case-insensitive).
const CATEGORY_ALIASES: &[(&str, &str)] = &[
    ("symbolic", "Symbolics"),
    ("symbolics", "Symbolics"),
    ("algebra", "Symbolics"),
    ("cas", "Symbolics"),
    ("calculus", "Calculus"),
    ("matrices", "Linear Algebra"),
    ("matrix", "Linear Algebra"),
    ("linearalgebra", "Linear Algebra"),
    ("linear algebra", "Linear Algebra"),
    ("mathir", "Compilation"),
    ("ir", "Compilation"),
    ("compilation", "Compilation"),
    ("solver", "Solving"),
    ("solvers", "Solving"),
    ("solving", "Solving"),
    ("optimisation", "Optimization"),
    ("dsp", "DSP"),
    ("signal", "DSP"),
    ("arrays", "Arrays"),
    ("numeric", "Numerics"),
    ("numbers", "Numerics"),
    ("types", "Introspection"),
    ("introspection", "Introspection"),
    ("language", "Language"),
    ("core", "Core"),
];

/// Every builtin descriptor, in name order (the reference catalog's deterministic order).
fn catalog() -> Vec<&'static BuiltinInfo> {
    let mut entries: Vec<&'static BuiltinInfo> = registry::BUILTINS.iter().collect();
    entries.sort_by(|a, b| a.name.cmp(b.name));
    entries
}

fn resolve_category(category: &str) -> Option<&'static str> {
    let trimmed = category.trim();
    CATEGORY_ALIASES
        .iter()
        .find(|(alias, _)| alias.eq_ignore_ascii_case(trimmed))
        .map(|(_, canonical)| *canonical)
}

fn canonical_category(category: &str) -> &str {
    for known in CATEGORY_ORDER {
        if known.eq_ignore_ascii_case(category) {
            return known;
        }
    }
    category
}

fn builtins_in(category: &str) -> Vec<&'static BuiltinInfo> {
    catalog()
        .into_iter()
        .filter(|info| info.category.eq_ignore_ascii_case(category))
        .collect()
}

/// Renders a signature that distinguishes required, optional and variadic parameters from the
/// descriptor's arity metadata: `solve(f, x [, domain])`, `diff(f, x)`.
pub fn signature(info: &BuiltinInfo) -> String {
    let min = info.min_arity.min(info.parameters.len());
    let required = &info.parameters[..min];
    let optional = &info.parameters[min..];
    let mut text = format!("{}({}", info.name, required.join(", "));
    if !optional.is_empty() {
        let rendered: Vec<String> = optional
            .iter()
            .enumerate()
            .map(|(index, parameter)| {
                if info.variadic && index + 1 == optional.len() {
                    format!("{parameter}...")
                } else {
                    (*parameter).to_string()
                }
            })
            .collect();
        if !required.is_empty() {
            text.push(' ');
        }
        text.push_str("[, ");
        text.push_str(&rendered.join(", "));
        text.push(']');
    }
    text.push(')');
    text
}

/// Full help for one builtin: signature, summary, examples, return kind, see-also and plugin.
/// `None` when the name is not a builtin (the reference returns null there).
pub fn function(name: &str) -> Option<String> {
    let info = registry::find(name)?;
    let mut text = String::new();
    text.push_str(&signature(info));
    text.push_str("\n\n");
    match info.summary {
        Some(summary) => text.push_str(summary),
        None => text.push_str("(no further help registered)"),
    }
    if !info.examples.is_empty() {
        text.push_str("\n\nExamples:");
        for example in info.examples {
            text.push_str("\n    ");
            text.push_str(example);
        }
    }
    text.push_str("\n\nReturns: ");
    text.push_str(info.return_kind.unwrap_or("Value"));
    if !info.related.is_empty() {
        text.push_str("\nSee also: ");
        text.push_str(&info.related.join(", "));
    }
    if let Some(plugin) = info.plugin {
        text.push_str("\nPlugin: ");
        text.push_str(plugin);
    }
    Some(text)
}

/// The one-line-per-function listing of a category (or all categories when `None`).
pub fn funcs(category: Option<&str>) -> String {
    match category {
        Some(category) => {
            let resolved = resolve_category(category).unwrap_or(category);
            let functions = builtins_in(resolved);
            if !functions.is_empty() {
                let mut out = String::new();
                out.push_str(canonical_category(functions[0].category));
                for info in functions {
                    out.push_str("\n  ");
                    out.push_str(&signature(info));
                }
                return out;
            }
            // Not a category: fall back to a name search so `funcs solve` finds the solvers.
            let matches: Vec<&'static BuiltinInfo> = catalog()
                .into_iter()
                .filter(|info| info.name.to_lowercase().contains(&category.to_lowercase()))
                .collect();
            if !matches.is_empty() {
                let mut out = format!("Matching '{category}':");
                for info in matches {
                    out.push_str("\n  ");
                    out.push_str(&signature(info));
                    out.push_str("  - ");
                    out.push_str(info.summary.unwrap_or(""));
                }
                return out;
            }
            format!(
                "No category or function matching '{category}'. Known categories: {}.",
                CATEGORY_ORDER.join(", ")
            )
        }
        None => {
            let mut out = String::new();
            for category in CATEGORY_ORDER {
                let functions = builtins_in(category);
                if functions.is_empty() {
                    continue;
                }
                out.push_str(category);
                for info in functions {
                    out.push_str("\n  ");
                    out.push_str(&signature(info));
                }
                out.push('\n');
            }
            out.trim_end().to_string()
        }
    }
}

/// Category listing (names + signatures); the reference's "No category" sentence otherwise.
pub fn category(category: &str) -> String {
    let resolved = resolve_category(category);
    let functions = builtins_in(resolved.unwrap_or(category));
    if functions.is_empty() {
        return format!(
            "No category '{category}'. Known categories: {}.",
            CATEGORY_ORDER.join(", ")
        );
    }
    let mut out = String::new();
    out.push_str(canonical_category(functions[0].category));
    for info in functions {
        out.push_str("\n  ");
        out.push_str(&signature(info));
    }
    out
}

/// Resolves a help argument as a function or a category (function wins, then a category alias,
/// then the registered category spelling). `None` when neither resolves (the reference's Lookup
/// returns the category's "No category" text; here the caller can tell the difference).
pub fn lookup(argument: &str) -> Option<String> {
    if let Some(function) = function(argument) {
        return Some(function);
    }
    if resolve_category(argument).is_some() {
        return Some(category(argument));
    }
    let text = category(argument);
    if text.starts_with("No category") {
        None
    } else {
        Some(text)
    }
}

/// Top-level help view: the categories that carry functions, with their counts.
pub fn overview() -> String {
    let mut text = String::from("LovelaceSharp — help\n");
    text.push_str("─────────────────────────────────────────────────\n");
    text.push_str("Categories:");
    for category in CATEGORY_ORDER {
        let count = builtins_in(category).len();
        if count == 0 {
            continue;
        }
        text.push_str(&format!("\n  {category:<16} {count} function(s)"));
    }
    // The reference's overview continues past the counts with the commands an agent can use and
    // the prelude every descriptor example runs under (HelpService.Overview). The census row
    // asserts the `help <function>` line; the whole block is reproduced so the rendered views
    // cannot drift.
    text.push_str("\n\nhelp <category>       list a category's functions");
    text.push_str("\nhelp <function>       signature, summary, examples");
    text.push_str("\nfuncs [category]      categorized function listing");
    text.push_str("\nvars / clear / delete / run <file> / set precision|display|pretty");
    text.push_str("\nLanguage statements & operators: docs/Language.md");
    text.push_str("\n\nEvery function example runs after this prelude:");
    text.push_str("\n  x = symbol(\"x\"); y = symbol(\"y\"); t = symbol(\"t\")");
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    /// HelpMetadataCompletenessTests.EveryBuiltin_HasCompleteMetadata — every shipped builtin has
    /// a summary, a declared return kind and no placeholder text.
    #[test]
    fn every_builtin_has_complete_metadata() {
        let mut holes: Vec<String> = Vec::new();
        for info in catalog() {
            let Some(help) = function(info.name) else {
                holes.push(format!("{}: no help entry", info.name));
                continue;
            };
            if help.contains("(no summary registered)")
                || help.contains("(no further help registered)")
            {
                holes.push(format!("{}: no summary", info.name));
            } else if help.contains("Returns: Value") {
                holes.push(format!("{}: return kind not declared", info.name));
            }
        }
        assert!(
            holes.is_empty(),
            "builtins with incomplete metadata: {}",
            holes.join("; ")
        );
    }

    /// HelpMetadataCompletenessTests.Signature_DistinguishesOptionalParameters.
    #[test]
    fn signature_distinguishes_optional_parameters() {
        let solve = function("solve").expect("solve has help");
        assert!(solve.contains("solve(f, x [, domain])"), "{solve}");
        let solve_full = function("solve_full").expect("solve_full has help");
        assert!(
            solve_full.contains("solve_full(f, x [, domain])"),
            "{solve_full}"
        );
        let diff = function("diff").expect("diff has help");
        assert!(diff.contains("diff(f, x)"), "{diff}");
    }

    /// HelpMetadataCompletenessTests.CategoryAliases_Resolve (the 13 InlineData rows).
    #[test]
    fn category_aliases_resolve() {
        for alias in [
            "symbolic",
            "symbolics",
            "calculus",
            "matrices",
            "matrix",
            "linear algebra",
            "mathir",
            "ir",
            "compilation",
            "solving",
            "optimization",
            "dsp",
            "introspection",
        ] {
            let text = lookup(alias).unwrap_or_else(|| panic!("alias '{alias}' did not resolve"));
            assert!(!text.contains("No category"), "alias '{alias}': {text}");
        }
    }

    /// HelpMetadataCompletenessTests.Funcs_FallsBackToNameSearch.
    #[test]
    fn funcs_falls_back_to_name_search() {
        let text = funcs(Some("solve"));
        assert!(text.contains("solve("), "{text}");
        assert!(text.contains("solve_full("), "{text}");
        assert!(!text.contains("No category"), "{text}");
    }

    /// HelpMetadataCompletenessTests.Funcs_ListsEveryBuiltinUnderACategory.
    #[test]
    fn funcs_lists_every_builtin_under_a_category() {
        let all = funcs(None);
        assert!(all.contains("Solving"), "{all}");
        assert!(all.contains("Symbolics"), "{all}");
        assert!(all.contains("DSP"), "{all}");
    }
}
