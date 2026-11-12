//! Live, context-aware completion: catalogue, caret context and the insertion rule.
//!
//! Everything the popup offers is assembled at run time; the feature has no list of its
//! own. [`build_catalogue`] takes the four sources the window hands it:
//!
//! - `keywords`: [`crate::language::KEYWORDS`], the same table the syntax highlighter paints;
//! - `functions`: every function the active engine's own snapshot reports. The real engine's
//!   `Engine::capture_state` publishes its whole registry (builtins with their declared
//!   parameter names, host/plugin registrations and the user's `func` definitions), and the
//!   desktop adapter copies it verbatim. Completion never re-implements that registry;
//! - `variables`: the active document's variables from its last committed workspace
//!   snapshot, with the kind the engine reported (`Natural`, `Text`, `Record`, ...);
//! - the builtin/non-builtin flag that tells user-defined functions apart from builtins.
//!
//! The module is deliberately free of egui: `ui::editor` owns the popup, the keys and the
//! painter, and calls into the pure pieces below. The unit tests cover the catalogue (dedup,
//! sort, exact-prefix first), the caret context (strings/comments), the prefix rule and the
//! insertion (replace exactly the prefix, move the caret, leave the text alone on a
//! dismissal).

use std::collections::BTreeSet;

use crate::engine::{EngineFunction, EngineVariable};

/// What one catalogue entry is, which is also how the popup marks it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CompletionKind {
    /// A variable from the active document's last committed snapshot.
    Variable,
    /// A function the engine resolves that is not a builtin (a user `func`, a host or a
    /// plugin registration).
    Function,
    /// A builtin from the engine's registry.
    Builtin,
    /// A keyword of the language.
    Keyword,
}

impl CompletionKind {
    /// Short, stable name used by the pane header and the tests.
    pub fn tag(self) -> &'static str {
        match self {
            CompletionKind::Variable => "variable",
            CompletionKind::Function => "function",
            CompletionKind::Builtin => "builtin",
            CompletionKind::Keyword => "keyword",
        }
    }

    /// Deduplication precedence: a variable shadows a function of the same name, a user
    /// function shadows the builtin it replaces, and none of them is hidden by a keyword.
    fn precedence(self) -> u8 {
        match self {
            CompletionKind::Variable => 0,
            CompletionKind::Function => 1,
            CompletionKind::Builtin => 2,
            CompletionKind::Keyword => 3,
        }
    }
}

/// One entry in the catalogue.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompletionItem {
    /// The word shown in the left column and inserted into the buffer.
    pub label: String,
    /// The second column: `name(params)` for functions and `name (Kind)` for variables.
    pub detail: String,
    /// The text acceptance inserts; always the label, so the popup cannot silently expand a
    /// prefix into something the user did not choose.
    pub insert: String,
    /// The entry's source, used for the popup's colour and marker.
    pub kind: CompletionKind,
}

impl CompletionItem {
    fn new(label: &str, detail: String, kind: CompletionKind) -> CompletionItem {
        CompletionItem {
            label: label.to_string(),
            detail,
            insert: label.to_string(),
            kind,
        }
    }
}

/// A keyword of the language.
pub fn keyword_item(keyword: &str) -> CompletionItem {
    CompletionItem::new(keyword, "keyword".to_string(), CompletionKind::Keyword)
}

/// A function with its declared parameter names.
pub fn function_item(function: &EngineFunction) -> CompletionItem {
    let kind = if function.is_builtin {
        CompletionKind::Builtin
    } else {
        CompletionKind::Function
    };
    CompletionItem::new(
        &function.name,
        format!("{}({})", function.name, function.parameters.join(", ")),
        kind,
    )
}

/// A variable with the kind the engine reported.
pub fn variable_item(variable: &EngineVariable) -> CompletionItem {
    CompletionItem::new(
        &variable.name,
        format!("{} ({})", variable.name, variable.kind),
        CompletionKind::Variable,
    )
}

/// Builds the catalogue from the live sources, deduplicated by label and sorted by name.
///
/// The four sources are merged in precedence order ([`CompletionKind::precedence`]), so a
/// variable named like a builtin is offered once, as the variable. The result is sorted
/// case-insensitively by label so the popup's order does not depend on the order the engine
/// happened to walk its maps in.
pub fn build_catalogue(
    keywords: &[&str],
    functions: &[EngineFunction],
    variables: &[EngineVariable],
) -> Vec<CompletionItem> {
    let mut candidates: Vec<CompletionItem> = Vec::new();
    candidates.extend(variables.iter().map(variable_item));
    candidates.extend(functions.iter().map(function_item));
    candidates.extend(keywords.iter().map(|keyword| keyword_item(keyword)));
    // Stable sort by precedence first: the first entry for a label is the one that wins.
    candidates.sort_by_key(|item| item.kind.precedence());

    let mut seen: BTreeSet<&str> = BTreeSet::new();
    let mut items: Vec<CompletionItem> = Vec::new();
    for item in &candidates {
        if seen.insert(item.label.as_str()) {
            items.push(item.clone());
        }
    }
    sort_items(&mut items);
    items
}

fn sort_items(items: &mut [CompletionItem]) {
    items.sort_by(|a, b| {
        a.label
            .to_ascii_lowercase()
            .cmp(&b.label.to_ascii_lowercase())
            .then_with(|| a.label.cmp(&b.label))
            .then_with(|| a.kind.cmp(&b.kind))
    });
}

/// Case-insensitive prefix filter, with an exact (case-insensitive) label first.
///
/// A prefix that matches no label returns an empty list; the caller then keeps the popup
/// closed instead of painting an empty box.
pub fn filter(catalogue: &[CompletionItem], prefix: &str) -> Vec<CompletionItem> {
    let needle = prefix.to_ascii_lowercase();
    let mut hits: Vec<CompletionItem> = catalogue
        .iter()
        .filter(|item| item.label.to_ascii_lowercase().starts_with(&needle))
        .cloned()
        .collect();
    hits.sort_by(|a, b| {
        let a_exact = a.label.eq_ignore_ascii_case(prefix);
        let b_exact = b.label.eq_ignore_ascii_case(prefix);
        b_exact
            .cmp(&a_exact)
            .then_with(|| {
                a.label
                    .to_ascii_lowercase()
                    .cmp(&b.label.to_ascii_lowercase())
            })
            .then_with(|| a.label.cmp(&b.label))
            .then_with(|| a.kind.cmp(&b.kind))
    });
    hits
}

fn is_ident_start(ch: char) -> bool {
    ch.is_ascii_alphabetic() || ch == '_'
}

fn is_ident_continue(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

/// The identifier prefix immediately before the caret: its text and its 0-based start column.
///
/// The scan runs back over `[A-Za-z0-9_]`; a run that starts with a digit is a number, not an
/// identifier, so `1sq` offers nothing. That rule also keeps the popup out of numeric
/// literals without a second lexer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IdentifierPrefix {
    /// The identifier characters before the caret.
    pub text: String,
    /// 0-based character column of the first character of `text`.
    pub start_col: usize,
}

/// Extracts the identifier prefix immediately before `caret_col` on `line`.
pub fn identifier_prefix(line: &str, caret_col: usize) -> Option<IdentifierPrefix> {
    let chars: Vec<char> = line.chars().collect();
    let caret = caret_col.min(chars.len());
    let mut start = caret;
    while start > 0 && is_ident_continue(chars[start - 1]) {
        start -= 1;
    }
    if start == caret || !is_ident_start(chars[start]) {
        return None;
    }
    Some(IdentifierPrefix {
        text: chars[start..caret].iter().collect(),
        start_col: start,
    })
}

/// What the caret sits in on its line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaretContext {
    /// Ordinary code: completion is offered.
    Code,
    /// Inside a quoted string literal: no completion.
    StringLiteral,
    /// Inside a `#` comment: no completion.
    Comment,
}

/// Classifies the caret context by scanning the line up to the caret.
///
/// This is the cheap, line-local rule the objective allows: the language's strings and
/// comments are line-local (`syntax::job` scans them the same way), so a plain state machine
/// over the characters before the caret is exact for this grammar. A `#` inside a string is
/// not a comment and an escaped quote does not end the string.
pub fn caret_context(line: &str, caret_col: usize) -> CaretContext {
    let mut in_string = false;
    let mut escaped = false;
    for ch in line.chars().take(caret_col) {
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        match ch {
            '#' => return CaretContext::Comment,
            '"' => in_string = true,
            _ => {}
        }
    }
    if in_string {
        CaretContext::StringLiteral
    } else {
        CaretContext::Code
    }
}

/// The popup's pure state: the filtered entries, the highlighted one and the prefix they
/// replace. The editor owns one of these; it never paints without an entry.
#[derive(Clone, Debug, Default)]
pub struct CompletionSession {
    open: bool,
    items: Vec<CompletionItem>,
    selected: usize,
    prefix: String,
    prefix_start: usize,
}

impl CompletionSession {
    /// True while the popup has entries to show.
    pub fn is_open(&self) -> bool {
        self.open && !self.items.is_empty()
    }

    /// The filtered entries, best match first.
    pub fn items(&self) -> &[CompletionItem] {
        &self.items
    }

    /// The highlighted entry's index into [`CompletionSession::items`].
    pub fn selected_index(&self) -> usize {
        self.selected
    }

    /// The highlighted entry.
    pub fn selected(&self) -> Option<&CompletionItem> {
        if self.is_open() {
            self.items.get(self.selected)
        } else {
            None
        }
    }

    /// The prefix the popup was opened/refreshed for.
    pub fn prefix(&self) -> &str {
        &self.prefix
    }

    /// Opens or refreshes the popup for `prefix`.
    ///
    /// Returns false (and closes) when the filter has no hits, so an empty popup is never
    /// shown. Refreshing keeps the highlighted label when it survives the narrower filter.
    pub fn show(&mut self, catalogue: &[CompletionItem], prefix: &str, prefix_start: usize) -> bool {
        let items = filter(catalogue, prefix);
        if items.is_empty() {
            self.dismiss();
            return false;
        }
        let previous = self.selected().map(|item| item.label.clone());
        let selected = previous
            .and_then(|label| items.iter().position(|item| item.label == label))
            .unwrap_or(0);
        self.items = items;
        self.selected = selected;
        self.prefix = prefix.to_string();
        self.prefix_start = prefix_start;
        self.open = true;
        true
    }

    /// Closes the popup without touching the buffer.
    pub fn dismiss(&mut self) {
        self.open = false;
        self.items.clear();
        self.selected = 0;
        self.prefix.clear();
        self.prefix_start = 0;
    }

    /// Moves the highlight by `delta`, clamped to the filtered entries.
    pub fn move_selection(&mut self, delta: isize) {
        if !self.is_open() {
            return;
        }
        let last = self.items.len() - 1;
        let next = (self.selected as isize + delta).clamp(0, last as isize);
        self.selected = next as usize;
    }

    /// Accepts the highlighted entry: replaces exactly the prefix characters and returns the
    /// caret position after the inserted text.
    ///
    /// The edit is refused (and the popup closed) when the buffer no longer contains the
    /// prefix at the recorded columns, so a stale popup can never rewrite unrelated text.
    pub fn accept(
        &mut self,
        lines: &mut [String],
        caret_row: usize,
        caret_col: usize,
    ) -> Option<(usize, usize)> {
        if !self.is_open() {
            return None;
        }
        let item = self.items.get(self.selected).cloned()?;
        let line = lines.get_mut(caret_row)?;
        let chars: Vec<char> = line.chars().collect();
        if caret_col > chars.len() || self.prefix_start > caret_col {
            self.dismiss();
            return None;
        }
        let reachable: String = chars[self.prefix_start..caret_col].iter().collect();
        if !reachable.eq_ignore_ascii_case(&self.prefix) {
            self.dismiss();
            return None;
        }
        let replacement: Vec<char> = item.insert.chars().collect();
        let mut rebuilt: String = chars[..self.prefix_start].iter().collect();
        rebuilt.extend(replacement.iter());
        rebuilt.extend(chars[caret_col..].iter());
        *line = rebuilt;
        let caret = self.prefix_start + replacement.len();
        self.dismiss();
        Some((caret_row, caret))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn builtin(name: &str, parameters: &[&str]) -> EngineFunction {
        EngineFunction {
            name: name.to_string(),
            parameters: parameters.iter().map(|p| p.to_string()).collect(),
            is_builtin: true,
            plugin: None,
            summary: None,
        }
    }

    fn user_function(name: &str, parameters: &[&str]) -> EngineFunction {
        EngineFunction {
            name: name.to_string(),
            parameters: parameters.iter().map(|p| p.to_string()).collect(),
            is_builtin: false,
            plugin: None,
            summary: None,
        }
    }

    fn variable(name: &str, kind: &str) -> EngineVariable {
        EngineVariable {
            name: name.to_string(),
            kind: kind.to_string(),
            display: "0".to_string(),
        }
    }

    fn labels(items: &[CompletionItem]) -> Vec<String> {
        items.iter().map(|item| item.label.clone()).collect()
    }

    #[test]
    fn catalogue_combines_keywords_functions_and_variables_without_duplicates() {
        let keywords = ["if", "else", "while"];
        let functions = [
            builtin("sqrt", &["x"]),
            builtin("sin", &["angle"]),
            user_function("norm", &["a", "b"]),
            // A user function that replaces a builtin of the same name must win.
            user_function("sin", &["x", "unit"]),
        ];
        let variables = [
            variable("x", "Natural"),
            variable("name", "Text"),
            // A variable shadows the builtin it is named after.
            variable("sqrt", "Real"),
        ];
        let catalogue = build_catalogue(&keywords, &functions, &variables);

        assert_eq!(
            labels(&catalogue),
            vec!["else", "if", "name", "norm", "sin", "sqrt", "while", "x"],
            "the catalogue is deduplicated and sorted by name"
        );
        let find = |label: &str| {
            catalogue
                .iter()
                .find(|item| item.label == label)
                .unwrap_or_else(|| panic!("{label} is missing from {:?}", labels(&catalogue)))
        };

        assert_eq!(find("if").kind, CompletionKind::Keyword);
        assert_eq!(find("if").detail, "keyword");
        assert_eq!(find("sqrt").kind, CompletionKind::Variable);
        assert_eq!(find("sqrt").detail, "sqrt (Real)");
        assert_eq!(find("name").detail, "name (Text)");
        assert_eq!(find("norm").kind, CompletionKind::Function);
        assert_eq!(find("norm").detail, "norm(a, b)");
        assert_eq!(
            find("sin").kind,
            CompletionKind::Function,
            "a user function overrides the builtin of the same name"
        );
        assert_eq!(find("sin").detail, "sin(x, unit)");
        for item in &catalogue {
            assert_eq!(item.insert, item.label, "acceptance inserts exactly the label");
        }
    }

    #[test]
    fn every_catalogue_entry_comes_from_one_of_the_live_sources() {
        // Nothing is invented: every label is a keyword, a function name or a variable name
        // from the arguments.
        let keywords = ["return", "break"];
        let functions = [builtin("sqrt", &["x"]), user_function("f", &[])];
        let variables = [variable("t", "Natural")];
        let catalogue = build_catalogue(&keywords, &functions, &variables);
        let allowed: BTreeSet<&str> = keywords
            .iter()
            .copied()
            .chain(functions.iter().map(|f| f.name.as_str()))
            .chain(variables.iter().map(|v| v.name.as_str()))
            .collect();
        for item in &catalogue {
            assert!(
                allowed.contains(item.label.as_str()),
                "{} is not from a live source",
                item.label
            );
        }
        assert_eq!(catalogue.len(), 5);
    }

    #[test]
    fn filter_is_case_insensitive_and_puts_the_exact_label_first() {
        let catalogue = build_catalogue(
            &["for"],
            &[
                builtin("sqrt", &["x"]),
                builtin("sq", &["x"]),
                builtin("Sine", &["x"]),
            ],
            &[],
        );
        assert_eq!(
            labels(&filter(&catalogue, "sq")),
            vec!["sq", "sqrt"],
            "the exact label is first, then the longer prefix match"
        );
        assert_eq!(
            labels(&filter(&catalogue, "SQ")),
            vec!["sq", "sqrt"],
            "filtering is case-insensitive"
        );
        assert_eq!(
            labels(&filter(&catalogue, "s")),
            vec!["Sine", "sq", "sqrt"],
            "case-insensitive alphabetical order for the common prefix"
        );
        assert!(filter(&catalogue, "zz").is_empty());
    }

    #[test]
    fn identifier_prefix_scans_only_the_identifier_run_before_the_caret() {
        assert_eq!(
            identifier_prefix("x = sq", 6),
            Some(IdentifierPrefix {
                text: "sq".to_string(),
                start_col: 4
            })
        );
        assert_eq!(
            identifier_prefix("  _a1", 5),
            Some(IdentifierPrefix {
                text: "_a1".to_string(),
                start_col: 2
            })
        );
        assert_eq!(identifier_prefix("x = ", 4), None, "no identifier run");
        assert_eq!(identifier_prefix("", 0), None, "empty line");
        assert!(
            identifier_prefix("x = 1sq", 7).is_none(),
            "a run that starts with a digit is a number, not an identifier"
        );
        assert_eq!(
            identifier_prefix("a.b", 3),
            Some(IdentifierPrefix {
                text: "b".to_string(),
                start_col: 2
            }),
            "a dot ends the identifier"
        );
        assert_eq!(
            identifier_prefix("abc", 99),
            Some(IdentifierPrefix {
                text: "abc".to_string(),
                start_col: 0
            }),
            "a caret past end-of-line clamps to the end of the identifier"
        );
        // A caret inside a word sees only the typed prefix, not the whole word.
        assert_eq!(
            identifier_prefix("sqx", 2),
            Some(IdentifierPrefix {
                text: "sq".to_string(),
                start_col: 0
            })
        );
    }

    #[test]
    fn caret_context_suppresses_strings_and_comments() {
        assert_eq!(caret_context("x = sq", 6), CaretContext::Code);
        assert_eq!(caret_context("x = \"sq", 7), CaretContext::StringLiteral);
        assert_eq!(
            caret_context("x = \"sqrt(4)\"", 13),
            CaretContext::Code,
            "the caret after the closing quote is code again"
        );
        assert_eq!(caret_context("x = 1 # sq", 10), CaretContext::Comment);
        assert_eq!(
            caret_context("x = \"# not a comment\"", 10),
            CaretContext::StringLiteral,
            "a hash inside a string does not start a comment"
        );
        assert_eq!(
            caret_context("x = \"a\\\"b\", y = 1", 8),
            CaretContext::StringLiteral,
            "an escaped quote does not end the string"
        );
        assert_eq!(
            caret_context("x = \"a\\\"b\"", 10),
            CaretContext::Code,
            "the string ends after the escaped quote and the closing quote"
        );
    }

    #[test]
    fn accepting_replaces_only_the_prefix_and_moves_the_caret() {
        let catalogue = build_catalogue(&[], &[builtin("sqrt", &["x"])], &[]);
        let mut session = CompletionSession::default();
        assert!(session.show(&catalogue, "sq", 4));

        let mut lines = vec!["x = sq".to_string()];
        assert_eq!(session.accept(&mut lines, 0, 6), Some((0, 8)));
        assert_eq!(lines, vec!["x = sqrt".to_string()], "only the prefix changes");
        assert_eq!(
            session.selected().map(|item| item.label.as_str()),
            None,
            "accepting dismisses the popup"
        );
        assert!(!session.is_open());

        // A caret in the middle of a longer word replaces only the typed prefix.
        let mut session = CompletionSession::default();
        assert!(session.show(&catalogue, "sq", 0));
        let mut lines = vec!["sqx".to_string()];
        assert_eq!(session.accept(&mut lines, 0, 2), Some((0, 4)));
        assert_eq!(lines, vec!["sqrtx".to_string()]);

        // The selection is what gets inserted.
        let mut session = CompletionSession::default();
        let catalogue = build_catalogue(&[], &[builtin("sqrt", &["x"]), builtin("sq", &["x"])], &[]);
        assert!(session.show(&catalogue, "sq", 0));
        assert_eq!(session.selected().unwrap().label, "sq");
        session.move_selection(1);
        assert_eq!(session.selected().unwrap().label, "sqrt");
        let mut lines = vec!["sq".to_string()];
        assert_eq!(session.accept(&mut lines, 0, 2), Some((0, 4)));
        assert_eq!(lines, vec!["sqrt".to_string()]);
    }

    #[test]
    fn escape_dismisses_without_changing_the_buffer() {
        let catalogue = build_catalogue(&["while"], &[builtin("sqrt", &["x"])], &[]);
        let mut session = CompletionSession::default();
        assert!(session.show(&catalogue, "s", 0));
        let before = vec!["x = s".to_string()];

        session.dismiss();

        assert!(!session.is_open());
        assert!(session.selected().is_none());
        assert_eq!(before, vec!["x = s".to_string()], "dismissal never edits");
        // A closed session refuses to accept, so a later Enter cannot insert anything.
        let mut lines = before.clone();
        assert_eq!(session.accept(&mut lines, 0, 5), None);
        assert_eq!(lines, before);
    }

    #[test]
    fn arrow_navigation_is_clamped_and_a_stale_prefix_is_refused() {
        let catalogue = build_catalogue(&[], &[builtin("sqrt", &["x"]), builtin("sq", &["x"])], &[]);
        let mut session = CompletionSession::default();
        assert!(session.show(&catalogue, "sq", 2));
        session.move_selection(-1);
        assert_eq!(session.selected_index(), 0, "Up on the first entry stays");
        session.move_selection(9);
        assert_eq!(
            session.selected_index(),
            session.items().len() - 1,
            "Down on the last entry stays"
        );

        // The buffer changed under the popup: the recorded prefix is gone, so acceptance
        // refuses instead of replacing unrelated characters.
        let mut lines = vec!["x = ta".to_string()];
        assert_eq!(session.accept(&mut lines, 0, 6), None);
        assert_eq!(lines, vec!["x = ta".to_string()]);
        assert!(!session.is_open());
    }
}
