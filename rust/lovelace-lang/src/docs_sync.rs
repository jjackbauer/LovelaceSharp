//! The usage-guide sync checker: extract the runnable examples and C# snippets from a usage
//! guide, and prove the snippets are present in the compiled examples file.
//!
//! This is the reference's `UsageDocParser` + `DocsSyncTests` contract as a library utility:
//!
//! * every fenced `lovelace` block must be followed by a `result` block (the documented
//!   expected result), and
//! * every fenced `csharp` block must appear verbatim in the examples source once whitespace is
//!   removed, so the guide cannot show code that does not compile.
//!
//! The parser mirrors the C# line rules: a fence is the exact tag line (```lovelace`,
//! ```result`, ```csharp`), the block runs to the next ``` line, and the block text is
//! trimmed. The caller supplies the documents; no path is embedded here.

/// One documented lovelace/result pair.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocExample {
    /// The script exactly as documented.
    pub script: String,
    /// The documented expected result (may start with `error: ` or `prints: `).
    pub expected: String,
}

/// One documented C# snippet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocSnippet {
    /// The snippet's position-derived name (`snippet-0`, `snippet-1`, …).
    pub name: String,
    /// The snippet's code exactly as documented.
    pub code: String,
}

/// Extracts every `lovelace` block that is immediately followed by a `result` block.
pub fn parse_examples(document: &str) -> Vec<DocExample> {
    let lines: Vec<&str> = document
        .split('\n')
        .map(|line| line.trim_end_matches('\r'))
        .collect();
    let mut examples = Vec::new();
    let mut index = 0usize;
    while index < lines.len() {
        if lines[index] == "```lovelace" {
            let script = read_block(&lines, &mut index);
            // A markdown reader separates the two fences with a blank line; the documented pair
            // is still one example. Skip blank separator lines before the result fence.
            let mut result_fence = index + 1;
            while result_fence < lines.len() && lines[result_fence].trim().is_empty() {
                result_fence += 1;
            }
            if result_fence < lines.len() && lines[result_fence] == "```result" {
                index = result_fence;
                let expected = read_block(&lines, &mut index);
                examples.push(DocExample { script, expected });
            }
        }
        index += 1;
    }
    examples
}

/// Extracts every `csharp` block, naming them in document order.
pub fn parse_csharp_snippets(document: &str) -> Vec<DocSnippet> {
    let lines: Vec<&str> = document
        .split('\n')
        .map(|line| line.trim_end_matches('\r'))
        .collect();
    let mut snippets = Vec::new();
    let mut index = 0usize;
    while index < lines.len() {
        if lines[index] == "```csharp" {
            let code = read_block(&lines, &mut index);
            snippets.push(DocSnippet {
                name: format!("snippet-{}", snippets.len()),
                code,
            });
        }
        index += 1;
    }
    snippets
}

/// Whitespace-normalizes text: the C# checker removes every whitespace character before
/// comparing, so indentation and line wrapping cannot hide a drift.
pub fn normalize(text: &str) -> String {
    text.chars().filter(|c| !c.is_whitespace()).collect()
}

/// Checks that every C# snippet appears in `examples_source` after whitespace normalization.
///
/// Returns the number of snippets checked, or every snippet that is missing.
pub fn check_csharp_snippets_appear(
    document: &str,
    examples_source: &str,
) -> Result<usize, Vec<String>> {
    let snippets = parse_csharp_snippets(document);
    let source = normalize(examples_source);
    let missing: Vec<String> = snippets
        .iter()
        .filter(|snippet| !source.contains(&normalize(&snippet.code)))
        .map(|snippet| {
            format!(
                "The C# snippet {} in the usage guide does not appear in the examples source:\n{}",
                snippet.name, snippet.code
            )
        })
        .collect();
    if missing.is_empty() {
        Ok(snippets.len())
    } else {
        Err(missing)
    }
}

/// A block's lines, read from just after the opening fence to just before the closing fence.
/// `index` is left ON the closing fence (or the last line), matching the C# `ref i` contract.
fn read_block(lines: &[&str], index: &mut usize) -> String {
    let mut block: Vec<&str> = Vec::new();
    *index += 1;
    while *index < lines.len() && lines[*index] != "```" {
        block.push(lines[*index]);
        *index += 1;
    }
    block.join("\n").trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::{check_csharp_snippets_appear, normalize, parse_csharp_snippets, parse_examples};

    #[test]
    fn parser_reads_examples_and_snippets_like_the_reference() {
        let document = "# Guide\n\n```lovelace\nx = 1; x + 1\n```\n\n```result\n2 (Natural)\n```\n\n```csharp\nvar x = 1;\n```\n";
        let examples = parse_examples(document);
        assert_eq!(examples.len(), 1);
        assert_eq!(examples[0].script, "x = 1; x + 1");
        assert_eq!(examples[0].expected, "2 (Natural)");
        let snippets = parse_csharp_snippets(document);
        assert_eq!(snippets.len(), 1);
        assert_eq!(snippets[0].name, "snippet-0");
        assert_eq!(snippets[0].code, "var x = 1;");
        assert_eq!(normalize(" a \n b "), "ab");
        assert_eq!(
            check_csharp_snippets_appear(document, "ignored\n var   x =\n1;\n").unwrap(),
            1
        );
    }
}
