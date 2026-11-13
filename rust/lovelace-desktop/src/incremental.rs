//! The per-document incremental computation cache (INC-001..INC-010).
//!
//! This is the desktop port of the web Studio's `IncrementalRunner`: a script is sliced into
//! top-level statements, and a statement whose normalized source and typed inputs are unchanged
//! (and which has no observable effect) is served from a per-document cache instead of being
//! evaluated again. The cached *typed value* is applied to the run's fresh engine, so later
//! statements see exactly the workspace a fresh run would have produced.
//!
//! # The key scheme
//!
//! One [`StatementKey`] holds, all as full canonical strings (never digests, so a hash collision
//! can never reuse a different statement):
//!
//! 1. the **normalized statement source** — comments removed, whitespace runs collapsed, and
//!    whitespace dropped next to punctuation (`normalize_statement`). String literals are copied
//!    verbatim, so `"a;b"` is never disturbed;
//! 2. the **document precision** submitted for the run and the **effective precision** the engine
//!    reported when the statement was planned. INC-005: a precision-control change reuses
//!    nothing, and a `setprecision(...)` inside the script changes the effective half;
//! 3. **ordered dependency keys**: for each global the statement reads, `name -> canonical typed
//!    value key` (or `UNDEF` when the engine has no such global). The canonical key comes from
//!    the engine ([`Evaluator::variable_key`]) and is derived from the *typed value*, never from
//!    its display form (INC-006);
//! 4. **function keys**: for every function the statement calls, directly or transitively,
//!    `builtin` or `rev:<n>` where `n` is that function's revision in this document's cache. A
//!    `func` definition bumps the revision only when its normalized source changes, so editing a
//!    function body invalidates its call sites (and their callers) while re-running an unchanged
//!    definition leaves them reusable (INC-003).
//!
//! # Dependency extraction (an over-approximation)
//!
//! `lovelace-lang`'s parser is not reachable from the desktop crate, so reads are recovered with
//! a token scan of the statement text: every identifier mentioned outside strings and comments is
//! a read **except** (a) language keywords, (b) the statement's own simple assignment target, and
//! (c) parameters of the `func` whose body is being scanned. An identifier immediately followed
//! by `(` is a call, not a variable read. This is deliberately conservative:
//!
//! - field names after `.`, record field labels, index operands and loop variables are all
//!   treated as global reads;
//! - a global read only from inside a called user function's body is added to the caller's read
//!   set (transitively), so changing that global invalidates the caller;
//! - a call to any callable this run cannot classify (not a source `func` and not an engine
//!   builtin) makes the statement effectful; it is never reused.
//!
//! Over-approximation can only cost a reuse, never serve a stale value.
//!
//! # Effects
//!
//! A statement is effectful — always re-evaluated, never cached, never served from the cache —
//! when it mentions a call to a side-effecting builtin, when it calls a user function that
//! (transitively) does, or when it calls a callable of unknown purity. The side-effecting
//! builtins are [`SIDE_EFFECT_BUILTINS`]: `print`, `plot`, `setprecision`, the `assume*` family
//! and `assumptions` (they mutate/read the engine's global symbolic assumption context), and the
//! fake engine's `clear` (it removes every global). Function definitions and control-flow
//! statements (`if`, `while`, `for`, `return`, blocks) are not memoizable at all, exactly like the
//! reference's `IsMemoizable`.
//!
//! # Apply-on-reuse
//!
//! A cache hit is reported as reused only when the cached [`CachedValue`] was actually applied to
//! the engine: the statement's write target (a simple assignment) and, for a non-`Void` value,
//! the engine's `_` result binding — mirroring `lovelace_lang::Engine::eval`, which stores `_`
//! after every statement. If the engine refuses ([`Evaluator::apply_cached_value`] returns
//! false), the statement is evaluated and reported as *computed*: a reuse that skipped the
//! statement without applying its value would corrupt the workspace, so that path is never taken
//! and never claimed.

use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};

use lovelace_json::Json;

use crate::engine::{CachedValue, Evaluator};
use crate::statements::Statement;

/// Builtins whose call is an observable effect, so a statement that calls one always re-runs.
///
/// `print` writes output, `plot` writes a plot capture and file, `setprecision` changes the
/// engine's display precision (which every later rendering depends on), the `assume*` family
/// mutates the engine's global assumption context (and `assumptions` reads it), and `clear` (the
/// deterministic fake's own builtin) removes every global.
pub const SIDE_EFFECT_BUILTINS: [&str; 12] = [
    "print",
    "plot",
    "setprecision",
    "assume",
    "assume_clear",
    "assume_integer",
    "assume_negative",
    "assume_nonnegative",
    "assume_positive",
    "assume_real",
    "assumptions",
    "clear",
];

/// Language keywords. They are never global reads.
const KEYWORDS: [&str; 14] = [
    "and", "break", "continue", "else", "false", "for", "func", "if", "in", "not", "or", "return",
    "true", "while",
];

/// Statement-leading keywords that make a statement non-memoizable (control flow or a definition).
const CONTROL_KEYWORDS: [&str; 7] = ["if", "while", "for", "return", "break", "continue", "else"];

/// A deterministic upper bound on one document's entries.
///
/// A full cache is cleared before the next insertion. The cache is a memo table, so eviction can
/// only cause recomputation: it can never change a result. INC-011 (documented memory limits and
/// deterministic eviction) is beyond this round's `INC-001..INC-010` scope; this bound keeps a
/// long editing session from growing without limit.
pub const ENTRY_LIMIT: usize = 4096;

// ---------------------------------------------------------------------------------------------
// Token scan and normalization
// ---------------------------------------------------------------------------------------------

/// What a scanned token is; only identifiers matter for dependency extraction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TokenKind {
    /// A name (including `_`).
    Ident,
    /// A numeric literal; it can never be an identifier.
    Number,
    /// A whole string literal, kept as one opaque token.
    Text,
    /// One punctuation character.
    Punct,
}

#[derive(Clone, Copy, Debug)]
struct Token<'a> {
    text: &'a str,
    kind: TokenKind,
}

/// Splits a statement into tokens, dropping comments and hiding string literals.
///
/// This is **not** the engine's lexer: it only has to be conservative. A `#` outside a string
/// starts a comment (to the end of the line), a quote starts a string (escapes copied), an
/// identifier is a letter-or-underscore followed by letters/digits/underscores, a leading digit
/// starts a number, and every other character is one punctuation token.
fn tokenize(source: &str) -> Vec<Token<'_>> {
    let mut tokens = Vec::new();
    let mut index = 0usize;
    while index < source.len() {
        let rest = &source[index..];
        let ch = rest.chars().next().unwrap_or_default();
        if ch == '#' {
            // A comment runs to the end of its line; the newline itself is whitespace.
            index += rest.find('\n').unwrap_or(rest.len());
            continue;
        }
        if ch == '"' || ch == '\'' {
            let start = index;
            index += ch.len_utf8();
            while index < source.len() {
                let current = source[index..].chars().next().unwrap_or_default();
                if current == '\\' {
                    index += current.len_utf8();
                    if index < source.len() {
                        let escaped = source[index..].chars().next().unwrap_or_default();
                        index += escaped.len_utf8();
                    }
                    continue;
                }
                index += current.len_utf8();
                if current == ch {
                    break;
                }
            }
            tokens.push(Token {
                text: &source[start..index],
                kind: TokenKind::Text,
            });
            continue;
        }
        if ch.is_whitespace() {
            index += ch.len_utf8();
            continue;
        }
        if is_ident_start(ch) {
            let start = index;
            index += ch.len_utf8();
            while index < source.len() {
                let current = source[index..].chars().next().unwrap_or_default();
                if is_ident_continue(current) {
                    index += current.len_utf8();
                } else {
                    break;
                }
            }
            tokens.push(Token {
                text: &source[start..index],
                kind: TokenKind::Ident,
            });
            continue;
        }
        if ch.is_ascii_digit() {
            let start = index;
            while index < source.len() {
                let current = source[index..].chars().next().unwrap_or_default();
                if current.is_ascii_alphanumeric() || current == '.' || current == '_' {
                    index += current.len_utf8();
                } else {
                    break;
                }
            }
            tokens.push(Token {
                text: &source[start..index],
                kind: TokenKind::Number,
            });
            continue;
        }
        let start = index;
        index += ch.len_utf8();
        tokens.push(Token {
            text: &source[start..index],
            kind: TokenKind::Punct,
        });
    }
    tokens
}

fn is_ident_start(ch: char) -> bool {
    ch.is_alphabetic() || ch == '_'
}

fn is_ident_continue(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

fn is_keyword(text: &str) -> bool {
    KEYWORDS.contains(&text)
}

/// True when the identifier token at `index` is called: the next token is `(`.
fn is_call(tokens: &[Token<'_>], index: usize) -> bool {
    matches!(
        tokens.get(index + 1),
        Some(token) if token.kind == TokenKind::Punct && token.text == "("
    )
}

/// The normalized statement key text.
///
/// Comments are dropped; whitespace runs become one space, except next to ASCII punctuation where
/// the space is dropped entirely, so `a = 1`, `a=1` and a re-indented form share one key. String
/// literals (including their internal whitespace and any `#`) are copied verbatim, because they
/// are data, not formatting.
pub fn normalize_statement(source: &str) -> String {
    let mut out = String::new();
    let mut pending_space = false;
    let mut index = 0usize;
    while index < source.len() {
        let rest = &source[index..];
        let ch = rest.chars().next().unwrap_or_default();
        if ch == '#' {
            index += rest.find('\n').unwrap_or(rest.len());
            continue;
        }
        if ch.is_whitespace() {
            pending_space = true;
            index += ch.len_utf8();
            continue;
        }
        if ch == '"' || ch == '\'' {
            emit_space(&mut out, &mut pending_space, false);
            let start = index;
            index += ch.len_utf8();
            while index < source.len() {
                let current = source[index..].chars().next().unwrap_or_default();
                if current == '\\' {
                    index += current.len_utf8();
                    if index < source.len() {
                        let escaped = source[index..].chars().next().unwrap_or_default();
                        index += escaped.len_utf8();
                    }
                    continue;
                }
                index += current.len_utf8();
                if current == ch {
                    break;
                }
            }
            out.push_str(&source[start..index]);
            continue;
        }
        emit_space(&mut out, &mut pending_space, ch.is_ascii_punctuation());
        out.push(ch);
        index += ch.len_utf8();
    }
    out
}

/// Emits the pending inter-token space unless it touches punctuation on either side.
fn emit_space(out: &mut String, pending: &mut bool, next_is_punct: bool) {
    if *pending && !out.is_empty() {
        let previous_is_punct = out
            .chars()
            .next_back()
            .is_some_and(|previous| previous.is_ascii_punctuation());
        if !previous_is_punct && !next_is_punct {
            out.push(' ');
        }
    }
    *pending = false;
}

// ---------------------------------------------------------------------------------------------
// Statement facts and program analysis
// ---------------------------------------------------------------------------------------------

/// What the token scan recovered about one sliced statement.
#[derive(Clone, Debug, Default)]
pub struct StatementFacts {
    /// The normalized source (the source half of the cache key).
    normalized: String,
    /// The simple assignment target, when the statement is one.
    write: Option<String>,
    /// The function name when the statement is a `func` definition.
    definition: Option<String>,
    /// True when the statement is a single expression/assignment, not control flow.
    memoizable: bool,
    /// Directly mentioned global names (keyword-filtered, assignment target excluded).
    reads: Vec<String>,
    /// Directly called names.
    calls: Vec<String>,
}

/// One user function's body recovered from its definition statement.
#[derive(Clone, Debug, Default)]
struct FunctionShape {
    reads: Vec<String>,
    calls: Vec<String>,
    /// True when the body contains an assignment. A call cannot be reused then: applying the
    /// cached result would not reproduce the global the body wrote.
    mutates: bool,
}

/// The source-level facts one run needs: per-statement reads/calls and the function table.
///
/// Built once per run from the sliced statements (plus the engine's builtin names). It is
/// independent of the engine instances, so it never holds engine state.
pub struct ProgramAnalysis {
    facts: Vec<StatementFacts>,
    functions: HashMap<String, FunctionShape>,
    /// Functions whose body (transitively) performs an observable effect.
    effectful: HashSet<String>,
    /// The normalized definition text of every user function, in source order, joined by `\n`.
    ///
    /// A function revision moves only when this combined text changes, so re-running an unchanged
    /// definition (which is never memoized) does not invalidate its call sites.
    definition_signatures: HashMap<String, String>,
    builtins: HashSet<String>,
}

impl ProgramAnalysis {
    /// Analyses every statement of a run.
    pub fn new(statements: &[Statement], builtins: HashSet<String>) -> ProgramAnalysis {
        let mut facts = Vec::with_capacity(statements.len());
        let mut functions: HashMap<String, FunctionShape> = HashMap::new();
        let mut definition_signatures: HashMap<String, String> = HashMap::new();

        for statement in statements {
            let tokens = tokenize(statement.text());
            let analysed = analyze_statement(statement.text(), &tokens);
            if let Some(name) = &analysed.definition {
                let (_name, shape) = function_details(&tokens).unwrap_or_default();
                let entry = functions.entry(name.clone()).or_default();
                // Over-approximate across redefinitions of one name: every body's reads and
                // calls are merged, so a call site never misses a dependency that one of the
                // definitions had.
                for read in shape.reads {
                    if !entry.reads.contains(&read) {
                        entry.reads.push(read);
                    }
                }
                for call in shape.calls {
                    if !entry.calls.contains(&call) {
                        entry.calls.push(call);
                    }
                }
                entry.mutates |= shape.mutates;
                let signature = definition_signatures.entry(name.clone()).or_default();
                if !signature.is_empty() {
                    signature.push('\n');
                }
                signature.push_str(&analysed.normalized);
            }
            facts.push(analysed);
        }

        let effectful = compute_effectful(&functions, &builtins);
        ProgramAnalysis {
            facts,
            functions,
            effectful,
            definition_signatures,
            builtins,
        }
    }

    /// The number of analysed statements.
    #[allow(dead_code)] // introspection accessor used by the cache tests
    pub fn len(&self) -> usize {
        self.facts.len()
    }

    /// True when no statement was analysed.
    #[allow(dead_code)] // introspection accessor used by the cache tests
    pub fn is_empty(&self) -> bool {
        self.facts.is_empty()
    }

    fn normalized(&self, index: usize) -> Option<&str> {
        self.facts.get(index).map(|facts| facts.normalized.as_str())
    }

    fn write(&self, index: usize) -> Option<String> {
        self.facts.get(index).and_then(|facts| facts.write.clone())
    }

    /// The function name a statement defines, when it defines one.
    pub fn definition_name(&self, index: usize) -> Option<&str> {
        self.facts
            .get(index)
            .and_then(|facts| facts.definition.as_deref())
    }

    /// The combined normalized definition text of a user function, for revision bookkeeping.
    pub fn definition_signature(&self, name: &str) -> Option<&str> {
        self.definition_signatures.get(name).map(String::as_str)
    }

    /// True when the engine lists `name` as one of its builtins.
    pub fn is_builtin(&self, name: &str) -> bool {
        self.builtins.contains(name)
    }

    /// Expands one statement's direct calls into its transitive read and call sets.
    ///
    /// Returns `None` when the statement is not memoizable. The bool is true when the statement
    /// is effectful and must be re-evaluated.
    pub fn expand(&self, index: usize) -> Option<(BTreeSet<String>, BTreeSet<String>, bool)> {
        let facts = self.facts.get(index)?;
        if !facts.memoizable {
            return None;
        }
        let mut reads: BTreeSet<String> = facts.reads.iter().cloned().collect();
        let mut functions: BTreeSet<String> = BTreeSet::new();
        let mut effectful = false;
        let mut queue: VecDeque<String> = facts.calls.iter().cloned().collect();
        let mut visited: HashSet<String> = HashSet::new();
        while let Some(name) = queue.pop_front() {
            if !visited.insert(name.clone()) {
                continue;
            }
            functions.insert(name.clone());
            if let Some(shape) = self.functions.get(&name) {
                if self.effectful.contains(&name) {
                    effectful = true;
                }
                for read in &shape.reads {
                    reads.insert(read.clone());
                }
                for call in &shape.calls {
                    queue.push_back(call.clone());
                }
            } else if SIDE_EFFECT_BUILTINS.contains(&name.as_str()) {
                effectful = true;
            } else if !self.builtins.contains(&name) {
                // A callable this run cannot classify. Re-evaluating is always safe; guessing
                // that it is pure is not.
                effectful = true;
            }
        }
        Some((reads, functions, effectful))
    }
}

/// Least fixed point of "this user function performs an observable effect".
fn compute_effectful(
    functions: &HashMap<String, FunctionShape>,
    builtins: &HashSet<String>,
) -> HashSet<String> {
    let mut effectful: HashSet<String> = HashSet::new();
    loop {
        let mut changed = false;
        for (name, shape) in functions {
            if effectful.contains(name) {
                continue;
            }
            let tainted = shape.mutates
                || shape.calls.iter().any(|call| {
                    SIDE_EFFECT_BUILTINS.contains(&call.as_str())
                        || effectful.contains(call)
                        || (!functions.contains_key(call) && !builtins.contains(call))
                });
            if tainted {
                effectful.insert(name.clone());
                changed = true;
            }
        }
        if !changed {
            return effectful;
        }
    }
}

/// Recovers the statement's shape, write target, direct reads and direct calls.
fn analyze_statement(source: &str, tokens: &[Token<'_>]) -> StatementFacts {
    let normalized = normalize_statement(source);
    let mut facts = StatementFacts {
        normalized,
        ..StatementFacts::default()
    };
    if tokens.is_empty() {
        return facts;
    }
    let first = tokens[0];
    if first.kind == TokenKind::Ident && first.text == "func" {
        // A definition is never memoized; its body feeds the transitive dependency tables.
        facts.definition = function_details(tokens).map(|(name, _)| name);
        return facts;
    }
    if first.kind == TokenKind::Punct && first.text == "{" {
        return facts;
    }
    if first.kind == TokenKind::Ident && CONTROL_KEYWORDS.contains(&first.text) {
        return facts;
    }

    let write_index = match assignment_index(tokens) {
        AssignmentScan::None => None,
        AssignmentScan::Target(index) => Some(index),
        AssignmentScan::Unsupported => return facts,
    };

    let mut reads = Vec::new();
    let mut calls = Vec::new();
    for (index, token) in tokens.iter().enumerate() {
        if token.kind != TokenKind::Ident || is_keyword(token.text) {
            continue;
        }
        if Some(index) == write_index {
            continue;
        }
        if is_call(tokens, index) {
            calls.push(token.text.to_string());
        } else {
            reads.push(token.text.to_string());
        }
    }

    facts.memoizable = true;
    facts.write = write_index.map(|index| tokens[index].text.to_string());
    facts.reads = reads;
    facts.calls = calls;
    facts
}

/// The result of looking for a simple assignment in one statement.
enum AssignmentScan {
    /// No assignment operator: a pure expression statement.
    None,
    /// A simple `name = ...` assignment; the token index of the target.
    Target(usize),
    /// An assignment form this module will not key: nested assignment, `f(y = 1)`, `x = y = 1`,
    /// or an assignment whose left side is not a single identifier.
    Unsupported,
}

/// True when the `=` token at `index` assigns, rather than forming `==`, `!=`, `<=` or `>=`.
fn is_assignment_operator(tokens: &[Token<'_>], index: usize) -> bool {
    match tokens.get(index) {
        Some(token) if token.kind == TokenKind::Punct && token.text == "=" => {}
        _ => return false,
    }
    let previous = index.checked_sub(1).map(|previous| &tokens[previous]);
    let next = tokens.get(index + 1);
    let comparison = matches!(next, Some(token) if token.kind == TokenKind::Punct && token.text == "=")
        || matches!(previous, Some(token) if token.kind == TokenKind::Punct && matches!(token.text, "=" | "!" | "<" | ">"));
    !comparison
}

fn assignment_index(tokens: &[Token<'_>]) -> AssignmentScan {
    let mut found: Option<usize> = None;
    for index in 0..tokens.len() {
        if !is_assignment_operator(tokens, index) {
            continue;
        }
        if found.is_some() {
            return AssignmentScan::Unsupported;
        }
        found = Some(index);
    }
    match found {
        None => AssignmentScan::None,
        Some(index) => {
            let simple = index == 1
                && tokens[0].kind == TokenKind::Ident
                && !is_keyword(tokens[0].text)
                && !is_call(tokens, 0);
            if simple {
                AssignmentScan::Target(0)
            } else {
                AssignmentScan::Unsupported
            }
        }
    }
}

/// Parses `func name(params) = body` or `func name(params) { body }` and recovers its shape.
///
/// Returns the function name and the read/call sets of its body, with the declared parameters
/// excluded from the reads.
fn function_details(tokens: &[Token<'_>]) -> Option<(String, FunctionShape)> {
    if tokens.len() < 5 {
        return None;
    }
    if tokens[0].kind != TokenKind::Ident || tokens[0].text != "func" {
        return None;
    }
    if tokens[1].kind != TokenKind::Ident {
        return None;
    }
    if !(tokens[2].kind == TokenKind::Punct && tokens[2].text == "(") {
        return None;
    }
    let mut index = 3usize;
    let mut parameters: Vec<String> = Vec::new();
    if tokens
        .get(index)
        .is_some_and(|token| token.kind == TokenKind::Punct && token.text == ")")
    {
        index += 1;
    } else {
        loop {
            let token = tokens.get(index)?;
            if token.kind != TokenKind::Ident {
                return None;
            }
            parameters.push(token.text.to_string());
            index += 1;
            match tokens.get(index) {
                Some(token) if token.kind == TokenKind::Punct && token.text == "," => index += 1,
                Some(token) if token.kind == TokenKind::Punct && token.text == ")" => {
                    index += 1;
                    break;
                }
                _ => return None,
            }
        }
    }
    let body_start = match tokens.get(index) {
        Some(token) if token.kind == TokenKind::Punct && token.text == "=" => index + 1,
        Some(token) if token.kind == TokenKind::Punct && token.text == "{" => index + 1,
        _ => return None,
    };

    let mut shape = FunctionShape::default();
    for position in body_start..tokens.len() {
        let token = tokens[position];
        if token.kind == TokenKind::Punct && token.text == "=" {
            if is_assignment_operator(tokens, position) {
                // The body writes something. A call cannot be reused: replaying only the return
                // value would leave the written global (or local) at its old value.
                shape.mutates = true;
            }
            continue;
        }
        if token.kind != TokenKind::Ident || is_keyword(token.text) {
            continue;
        }
        if is_call(tokens, position) {
            shape.calls.push(token.text.to_string());
        } else if !parameters.iter().any(|parameter| parameter == token.text) {
            shape.reads.push(token.text.to_string());
        }
    }
    Some((tokens[1].text.to_string(), shape))
}

// ---------------------------------------------------------------------------------------------
// The cache
// ---------------------------------------------------------------------------------------------

/// The full cache key of one statement (see the module documentation for the scheme).
///
/// The fields are the canonical strings themselves rather than digests: equality is then exact,
/// so no hash collision can ever reuse a different statement.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct StatementKey {
    /// The normalized statement source.
    pub source: String,
    /// The precision the run was submitted with.
    pub document_precision: usize,
    /// The precision the engine reported when the statement was planned.
    pub effective_precision: usize,
    /// Ordered `(global name, canonical typed value key)` pairs, sorted by name.
    pub dependencies: Vec<(String, String)>,
    /// Ordered `(function name, "builtin" | "rev:<n>")` pairs, sorted by name.
    pub functions: Vec<(String, String)>,
}

/// One cached statement outcome.
///
/// `display`, `structured` and `inspection` are carried because a reuse must reproduce the exact
/// committed snapshot a fresh evaluation would have published (RES-001/SYM-001), and they were
/// produced together with `value` at the commit point.
#[derive(Clone, Debug)]
pub struct CachedEntry {
    /// The simple assignment target, when the statement had one.
    pub write: Option<String>,
    /// The display form the evaluation published.
    pub display: String,
    /// The engine's structured projection of the value, when it published one.
    pub structured: Option<Json>,
    /// The engine's symbolic inspection of the value, when it published one.
    pub inspection: Option<Json>,
    /// The engine's own typed value, re-applied on reuse.
    pub value: CachedValue,
}

/// What the cache says about one statement of the next run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StepDecision {
    /// Not memoizable, effectful, or the engine cannot re-apply a cached value: evaluate only.
    Evaluate,
    /// Memoizable and pure. Reuse the entry when it hits; otherwise evaluate and store under
    /// this key.
    Keyed {
        /// The key built from the statement's normalized source and current typed inputs.
        key: StatementKey,
        /// The simple assignment target to apply a cached value to.
        write: Option<String>,
    },
}

#[derive(Clone, Debug)]
struct FunctionRecord {
    revision: u64,
    source: String,
}

/// One document's computation cache: cached statement outcomes plus function revisions.
#[derive(Default)]
pub struct DocumentCache {
    entries: HashMap<StatementKey, CachedEntry>,
    functions: HashMap<String, FunctionRecord>,
    next_revision: u64,
}

impl DocumentCache {
    /// Creates an empty cache.
    #[allow(dead_code)] // the host builds one with Default; kept as the explicit constructor
    pub fn new() -> DocumentCache {
        DocumentCache::default()
    }

    /// The number of cached statements.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// True when nothing is cached.
    #[allow(dead_code)] // introspection accessor used by the cache tests
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// One cached entry, when the key hits.
    pub fn get(&self, key: &StatementKey) -> Option<&CachedEntry> {
        self.entries.get(key)
    }

    /// Stores one outcome. A full cache is cleared first; that can only cause recomputation.
    pub fn insert(&mut self, key: StatementKey, entry: CachedEntry) {
        if self.entries.len() >= ENTRY_LIMIT && !self.entries.contains_key(&key) {
            self.entries.clear();
        }
        self.entries.insert(key, entry);
    }

    /// Records a function definition, returning true when its revision moved.
    ///
    /// INC-003: the revision moves only when the normalized definition text changes. Re-running an
    /// unchanged `func` statement (definitions are never memoized) therefore keeps its call sites
    /// reusable, while editing the body invalidates them — and their callers, whose keys include
    /// this revision transitively.
    pub fn note_definition(&mut self, name: &str, source: &str) -> bool {
        match self.functions.get(name) {
            Some(record) if record.source == source => false,
            _ => {
                self.next_revision += 1;
                let revision = self.next_revision;
                self.functions.insert(
                    name.to_string(),
                    FunctionRecord {
                        revision,
                        source: source.to_string(),
                    },
                );
                true
            }
        }
    }

    /// The current revision of a user function, if it has ever been defined in this document.
    pub fn function_revision(&self, name: &str) -> Option<u64> {
        self.functions.get(name).map(|record| record.revision)
    }

    /// Drops every cached statement, keeping the function revisions.
    ///
    /// INC-009: the next run evaluates every statement ("Run All Fresh") but the same values and
    /// output. Function revisions are kept so the fresh run's keys are identical to the ones an
    /// incremental run would build.
    pub fn clear_entries(&mut self) {
        self.entries.clear();
    }

    /// Decides whether one statement may be served from the cache and, if so, builds its key.
    ///
    /// Building the key reads the *current* typed value of every global the statement reads, so
    /// it has to run immediately before the statement itself. The key is returned even on a miss
    /// so the caller can store the evaluated outcome under it.
    pub fn plan(
        &self,
        analysis: &ProgramAnalysis,
        index: usize,
        document_precision: usize,
        effective_precision: usize,
        engine: &dyn Evaluator,
    ) -> StepDecision {
        if !engine.supports_cached_values() {
            return StepDecision::Evaluate;
        }
        let Some((reads, functions, effectful)) = analysis.expand(index) else {
            return StepDecision::Evaluate;
        };
        if effectful {
            return StepDecision::Evaluate;
        }
        let Some(normalized) = analysis.normalized(index) else {
            return StepDecision::Evaluate;
        };

        let mut dependencies = Vec::with_capacity(reads.len());
        for name in reads {
            // INC-006: the canonical typed value, or the explicit `UNDEF` marker when the engine
            // has no global of that name. The display form is never consulted.
            let value = engine
                .variable_key(&name)
                .unwrap_or_else(|| "UNDEF".to_string());
            dependencies.push((name, value));
        }

        let mut function_keys = Vec::with_capacity(functions.len());
        for name in functions {
            let key = match self.function_revision(&name) {
                Some(revision) => format!("rev:{revision}"),
                None if analysis.is_builtin(&name) => "builtin".to_string(),
                // A callable of unknown purity was already classified effectful; keep the
                // conservative answer if it somehow reaches here.
                None => return StepDecision::Evaluate,
            };
            function_keys.push((name, key));
        }

        StepDecision::Keyed {
            key: StatementKey {
                source: normalized.to_string(),
                document_precision,
                effective_precision,
                dependencies,
                functions: function_keys,
            },
            write: analysis.write(index),
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Unit tests (no engine): the scan, the normalization and the key scheme
// ---------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn facts_of(source: &str) -> StatementFacts {
        let tokens = tokenize(source);
        analyze_statement(source, &tokens)
    }

    fn analysis_of(source: &str, builtins: &[&str]) -> ProgramAnalysis {
        let statements = crate::statements::slice(source);
        ProgramAnalysis::new(
            &statements,
            builtins.iter().map(|name| name.to_string()).collect(),
        )
    }

    #[test]
    fn normalization_drops_comments_and_collapses_whitespace_but_keeps_strings() {
        assert_eq!(normalize_statement("  a   =  1  "), "a=1");
        assert_eq!(normalize_statement("a = 1 # trailing"), "a=1");
        assert_eq!(
            normalize_statement("s = \"a  b # c\"  # note"),
            "s=\"a  b # c\""
        );
        assert_eq!(normalize_statement("f ( x )"), "f(x)");
        assert_eq!(normalize_statement("a = 1\r\nb = 2"), "a=1 b=2");
        assert_eq!(
            normalize_statement("# only a comment"),
            "",
            "a comment-only statement normalizes to nothing"
        );
        // Re-indenting an unchanged statement keeps its key.
        assert_eq!(
            normalize_statement("b = a +\n    1"),
            normalize_statement("b=a+1")
        );
    }

    #[test]
    fn the_scan_ignores_strings_comments_and_numbers() {
        let facts = facts_of("s = \"x # not a comment\" # a comment");
        assert_eq!(facts.write.as_deref(), Some("s"));
        assert_eq!(facts.reads, Vec::<String>::new());
        assert_eq!(facts.calls, Vec::<String>::new());

        let facts = facts_of("x = 1..10");
        assert!(facts.reads.is_empty(), "range endpoints are numbers");
    }

    #[test]
    fn a_simple_assignment_excludes_only_the_target_occurrence() {
        let facts = facts_of("x = x + 1");
        assert_eq!(facts.write.as_deref(), Some("x"));
        assert_eq!(
            facts.reads,
            vec!["x".to_string()],
            "the right-hand x is still a read"
        );
        assert!(facts.memoizable);

        let facts = facts_of("b = a + 1");
        assert_eq!(facts.write.as_deref(), Some("b"));
        assert_eq!(facts.reads, vec!["a".to_string()]);
    }

    #[test]
    fn comparisons_are_not_assignments_and_nested_assignments_are_not_memoized() {
        for source in ["a == 1", "a != 1", "a <= 1", "a >= 1"] {
            let facts = facts_of(source);
            assert!(facts.memoizable, "{source} is an expression statement");
            assert_eq!(facts.write, None, "{source} is not a write");
        }
        for source in ["x = y = 1", "f(y = 1)", "x[0] = 1"] {
            let facts = facts_of(source);
            assert!(
                !facts.memoizable,
                "{source} has an assignment form the key cannot describe"
            );
        }
    }

    #[test]
    fn control_flow_and_definitions_are_not_memoizable() {
        for source in [
            "if (a) { b = 1 }",
            "while (a) { b = 1 }",
            "for i in 0..3 { b = i }",
            "return a",
            "{ a = 1 }",
            "func f(x) = x + 1",
        ] {
            assert!(
                !facts_of(source).memoizable,
                "{source} must always be evaluated"
            );
        }
        let definition = facts_of("func f(x) = x + g");
        assert_eq!(definition.definition.as_deref(), Some("f"));
    }

    #[test]
    fn effects_are_detected_directly_and_through_user_functions() {
        let builtins = ["sqrt", "abs"];
        for source in ["print(\"x\")", "plot(1..3, [1, 4, 9])", "setprecision(20)"] {
            let analysis = analysis_of(source, &builtins);
            let (_, _, effectful) = analysis.expand(0).expect("a memoizable statement");
            assert!(effectful, "{source} must be effectful");
        }

        // A user function that prints makes its caller effectful, transitively.
        let source = "func p(x) = print(x)\nfunc q(x) = p(x)\ny = q(1)";
        let analysis = analysis_of(source, &builtins);
        let (_, _, effectful) = analysis.expand(2).expect("the call statement");
        assert!(effectful, "a call to a printing function must re-evaluate");

        // A pure user function stays reusable.
        let source = "func twice(x) = x * 2\ny = twice(3)";
        let analysis = analysis_of(source, &builtins);
        let (_, _, effectful) = analysis.expand(1).expect("the call statement");
        assert!(!effectful);
    }

    #[test]
    fn a_call_to_an_unknown_callable_is_treated_as_effectful() {
        let analysis = analysis_of("y = mystery(1)", &["sqrt"]);
        let (_, _, effectful) = analysis.expand(0).expect("the call statement");
        assert!(
            effectful,
            "the desktop cannot prove an unknown callable is pure"
        );
    }

    #[test]
    fn a_user_function_that_assigns_inside_its_body_is_effectful() {
        // `func f(x) = (g = x)` writes a global while returning a value. Reusing a call would
        // apply only the return value and leave `g` stale, so the call must re-evaluate.
        let analysis = analysis_of("func f(x) = (g = x)\ny = f(1)", &["sqrt"]);
        let (_, _, effectful) = analysis.expand(1).expect("the call statement");
        assert!(
            effectful,
            "a call that writes a global must never be served from the cache"
        );
    }

    #[test]
    fn a_user_function_body_contributes_its_global_reads_transitively() {
        let source = "func f(x) = x + g\nfunc h(x) = f(x)\ny = h(2)";
        let analysis = analysis_of(source, &["sqrt"]);
        let (reads, functions, effectful) = analysis.expand(2).expect("the call statement");
        assert!(!effectful);
        assert!(
            reads.contains("g"),
            "the body's global read must reach the caller: {reads:?}"
        );
        assert!(functions.contains("f") && functions.contains("h"));
        assert!(
            !reads.contains("x"),
            "a declared parameter is not a global read: {reads:?}"
        );
    }

    #[test]
    fn the_definition_signature_is_the_combined_text_of_every_redefinition() {
        let analysis = analysis_of("func f(x) = x + 1\nfunc f(x) = x + 2", &[]);
        let signature = analysis.definition_signature("f").expect("a signature");
        assert_eq!(signature, "func f(x)=x+1\nfunc f(x)=x+2");
    }

    #[test]
    fn the_key_changes_with_every_input_the_reference_hashes() {
        let mut engine = StubEngine::default();
        engine.values.insert("a".to_string(), "N:2".to_string());
        let statements = crate::statements::slice("b = a + 1");
        let analysis = ProgramAnalysis::new(&statements, HashSet::new());
        let cache = DocumentCache::new();

        let StepDecision::Keyed { key, write } = cache.plan(&analysis, 0, 1000, 1000, &engine)
        else {
            panic!("a pure assignment must be keyed");
        };
        assert_eq!(write.as_deref(), Some("b"));
        assert_eq!(key.source, "b=a+1");
        assert_eq!(key.dependencies, vec![("a".to_string(), "N:2".to_string())]);

        // Same inputs, same key.
        let StepDecision::Keyed { key: again, .. } = cache.plan(&analysis, 0, 1000, 1000, &engine)
        else {
            panic!("keyed");
        };
        assert_eq!(key, again);

        // A typed input change moves the key.
        engine.values.insert("a".to_string(), "N:3".to_string());
        let StepDecision::Keyed { key: changed, .. } =
            cache.plan(&analysis, 0, 1000, 1000, &engine)
        else {
            panic!("keyed");
        };
        assert_ne!(key, changed);

        // A precision change moves the key twice over.
        let StepDecision::Keyed { key: precision, .. } =
            cache.plan(&analysis, 0, 1000, 20, &engine)
        else {
            panic!("keyed");
        };
        assert_ne!(changed, precision);
        assert_eq!(precision.document_precision, 1000);
        assert_eq!(precision.effective_precision, 20);

        // An undefined read is keyed as UNDEF, never as some other value.
        engine.values.clear();
        let StepDecision::Keyed { key: undefined, .. } =
            cache.plan(&analysis, 0, 1000, 1000, &engine)
        else {
            panic!("keyed");
        };
        assert_eq!(undefined.dependencies[0].1, "UNDEF");
    }

    #[test]
    fn a_function_revision_moves_only_when_the_definition_text_changes() {
        let mut cache = DocumentCache::new();
        assert!(cache.note_definition("f", "func f(x)=x+1"));
        let first = cache.function_revision("f").expect("a revision");
        assert!(
            !cache.note_definition("f", "func f(x)=x+1"),
            "the same definition must not invalidate its call sites"
        );
        assert_eq!(cache.function_revision("f"), Some(first));
        assert!(cache.note_definition("f", "func f(x)=x+10"));
        assert!(cache.function_revision("f").unwrap() > first);
    }

    #[test]
    fn the_entry_limit_evicts_deterministically_and_never_changes_results() {
        let mut cache = DocumentCache::new();
        let entry = |index: usize| CachedEntry {
            write: Some("x".to_string()),
            display: index.to_string(),
            structured: None,
            inspection: None,
            value: CachedValue::Fake(crate::engine::FakeValue::Int(index as i64)),
        };
        let key = |index: usize| StatementKey {
            source: format!("x = {index}"),
            document_precision: 1000,
            effective_precision: 1000,
            dependencies: Vec::new(),
            functions: Vec::new(),
        };
        for index in 0..ENTRY_LIMIT {
            cache.insert(key(index), entry(index));
        }
        assert_eq!(cache.len(), ENTRY_LIMIT);
        cache.insert(key(ENTRY_LIMIT), entry(ENTRY_LIMIT));
        assert_eq!(
            cache.len(),
            1,
            "a full cache is cleared before the new entry, never grown without bound"
        );
        cache.clear_entries();
        assert!(cache.is_empty());
    }

    /// A minimal evaluator with no engine behind it: canonical values are a map, there is no
    /// typed value to re-apply, and the cache is therefore planned but never applied here.
    #[derive(Default)]
    struct StubEngine {
        values: HashMap<String, String>,
    }

    impl Evaluator for StubEngine {
        fn eval(&mut self, _src: &str) -> Result<String, String> {
            Ok("ok".to_string())
        }

        fn supports_cached_values(&self) -> bool {
            true
        }

        fn variable_key(&self, name: &str) -> Option<String> {
            self.values.get(name).cloned()
        }
    }

    #[test]
    fn a_plan_without_engine_support_is_never_keyed() {
        struct Bare;
        impl Evaluator for Bare {
            fn eval(&mut self, _src: &str) -> Result<String, String> {
                Ok("ok".to_string())
            }
        }
        let statements = crate::statements::slice("x = 1");
        let analysis = ProgramAnalysis::new(&statements, HashSet::new());
        let cache = DocumentCache::new();
        assert_eq!(
            cache.plan(&analysis, 0, 1000, 1000, &Bare),
            StepDecision::Evaluate,
            "an engine that cannot re-apply a value must never be offered a reuse"
        );
    }
}

// ---------------------------------------------------------------------------------------------
// End-to-end tests against the real engine
// ---------------------------------------------------------------------------------------------

#[cfg(all(test, feature = "real-engine"))]
mod engine_tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    use lovelace_json::Json;

    use crate::engine::{
        CachedValue as EngineValue, EngineState, Evaluator, PlotCapture, RealEngine,
    };
    use crate::execution::{
        DesktopEvent, DocumentId, EvaluatorFactory, ExecutionHost, SourceRevision,
        WorkspaceSnapshot,
    };
    use crate::log_kind::LogKind;
    use crate::workbench::Workbench;

    /// The temp plot directory the effect test's engines write into.
    ///
    /// A plot statement needs a writable directory. The shipped workbench derives it from
    /// `%APPDATA%`, which this sandbox does not let the test process write to; the engine's own
    /// `set_plot_dir` seam points it at the redirected temp directory instead (the harness sets
    /// `TMP`/`TEMP` to `rust\target-desktop\tmp`), so the plot statement itself is exercised
    /// rather than skipped.
    fn plot_directory() -> String {
        let directory = std::env::temp_dir().join("lovelace-desktop-incremental-plots");
        std::fs::create_dir_all(&directory).expect("a writable temp plot directory");
        directory.to_string_lossy().to_string()
    }

    /// A real engine that counts its `eval` calls and can refuse cached values.
    ///
    /// The counter is the falsifier for INC-008: a reuse claim is only honest when the statement
    /// was not evaluated, so the tests assert the exact number of `eval` calls each run made.
    struct HarnessEngine {
        inner: RealEngine,
        evaluations: Arc<AtomicUsize>,
        accept: bool,
    }

    impl Evaluator for HarnessEngine {
        fn eval(&mut self, src: &str) -> Result<String, String> {
            self.evaluations.fetch_add(1, Ordering::SeqCst);
            self.inner.eval(src)
        }

        fn set_precision(&mut self, decimals: usize) {
            self.inner.set_precision(decimals);
        }

        fn set_cancel_flag(&mut self, flag: Option<Arc<std::sync::atomic::AtomicBool>>) {
            self.inner.set_cancel_flag(flag);
        }

        fn set_plot_dir(&mut self, dir: &str) {
            self.inner.set_plot_dir(dir);
        }

        fn precision(&self) -> usize {
            self.inner.precision()
        }

        fn variables(&self) -> Vec<(String, String)> {
            self.inner.variables()
        }

        fn functions(&self) -> Vec<(String, Vec<String>)> {
            self.inner.functions()
        }

        fn state(&self) -> Option<EngineState> {
            self.inner.state()
        }

        fn last_structured(&self) -> Option<Json> {
            self.inner.last_structured()
        }

        fn last_inspection(&self) -> Option<Json> {
            self.inner.last_inspection()
        }

        fn take_output(&mut self) -> Vec<String> {
            self.inner.take_output()
        }

        fn last_plot(&self) -> Option<PlotCapture> {
            self.inner.last_plot()
        }

        fn supports_cached_values(&self) -> bool {
            self.inner.supports_cached_values()
        }

        fn variable_key(&self, name: &str) -> Option<String> {
            self.inner.variable_key(name)
        }

        fn last_value(&self) -> Option<EngineValue> {
            self.inner.last_value()
        }

        fn apply_cached_value(&mut self, name: &str, value: &EngineValue) -> bool {
            if !self.accept {
                return false;
            }
            self.inner.apply_cached_value(name, value)
        }

        fn tracks_result_variable(&self) -> bool {
            self.inner.tracks_result_variable()
        }
    }

    fn factory(evaluations: Arc<AtomicUsize>, accept: bool) -> EvaluatorFactory {
        let plot_dir = plot_directory();
        Box::new(move || {
            let mut engine = RealEngine::new();
            engine.set_plot_dir(&plot_dir);
            Box::new(HarnessEngine {
                inner: engine,
                evaluations: Arc::clone(&evaluations),
                accept,
            })
        })
    }

    /// One run's observed outcome, read from the typed event stream.
    struct RunObservation {
        snapshot: WorkspaceSnapshot,
        reused: Vec<bool>,
        results: Vec<String>,
        evaluations: usize,
    }

    impl RunObservation {
        fn reused_count(&self) -> usize {
            self.reused.iter().filter(|flag| **flag).count()
        }

        fn variables(&self) -> Vec<(String, String, String)> {
            self.snapshot
                .state
                .variables
                .iter()
                .map(|variable| {
                    (
                        variable.name.clone(),
                        variable.kind.clone(),
                        variable.display.clone(),
                    )
                })
                .collect()
        }

        fn display(&self, name: &str) -> String {
            self.snapshot
                .state
                .variables
                .iter()
                .find(|variable| variable.name == name)
                .map(|variable| variable.display.clone())
                .unwrap_or_else(|| "(missing)".to_string())
        }
    }

    struct Harness {
        host: ExecutionHost,
        document: DocumentId,
        evaluations: Arc<AtomicUsize>,
    }

    impl Harness {
        fn new() -> Harness {
            Harness::with_acceptance(true)
        }

        /// A harness whose engine can key and publish values but refuses to apply them.
        fn refusing() -> Harness {
            Harness::with_acceptance(false)
        }

        fn with_acceptance(accept: bool) -> Harness {
            let evaluations = Arc::new(AtomicUsize::new(0));
            let host = ExecutionHost::new(factory(Arc::clone(&evaluations), accept));
            Harness {
                host,
                document: DocumentId(1),
                evaluations,
            }
        }

        fn run(&self, revision: u64, source: &str) -> RunObservation {
            self.run_at(revision, source, 1000)
        }

        fn run_at(&self, revision: u64, source: &str, precision: usize) -> RunObservation {
            let before = self.evaluations.load(Ordering::SeqCst);
            let _handle = self
                .host
                .submit(
                    self.document,
                    SourceRevision(revision),
                    source.to_string(),
                    precision,
                )
                .expect("an idle document accepts the run");
            let mut reused = Vec::new();
            let mut results = Vec::new();
            loop {
                let event = self
                    .host
                    .next_event(Duration::from_secs(120))
                    .expect("an event before the terminal one");
                match &event.event {
                    DesktopEvent::StepCommitted {
                        index,
                        reused: flag,
                        snapshot,
                        ..
                    } => {
                        assert_eq!(*index, reused.len(), "commits arrive in statement order");
                        reused.push(*flag);
                        results.push(snapshot.result.clone());
                    }
                    DesktopEvent::Finished { snapshot, .. } => {
                        return RunObservation {
                            snapshot: snapshot.clone(),
                            reused,
                            results,
                            evaluations: self.evaluations.load(Ordering::SeqCst) - before,
                        };
                    }
                    DesktopEvent::Failed { diagnostic, .. } => {
                        panic!("the run failed: {}", diagnostic.render())
                    }
                    DesktopEvent::Cancelled { .. } => panic!("the run was cancelled"),
                    _ => {}
                }
            }
        }
    }

    fn fractional_digits(text: &str) -> usize {
        text.split_once('.')
            .map(|(_, fraction)| fraction.len())
            .unwrap_or(0)
    }

    #[test]
    fn three_statements_run_twice_reuse_zero_then_three_with_identical_values() {
        let harness = Harness::new();
        let source = "a = 2\nb = a + 1\nc = b * 3\n";

        let first = harness.run(1, source);
        assert_eq!(first.reused, vec![false, false, false]);
        assert_eq!(first.reused_count(), 0);
        assert_eq!(
            first.evaluations, 3,
            "the first run evaluates every statement"
        );
        assert_eq!(first.results, vec!["2", "3", "9"]);
        let first_variables = first.variables();

        let second = harness.run(2, source);
        assert_eq!(
            second.reused,
            vec![true, true, true],
            "the unchanged script must be reused statement by statement"
        );
        assert_eq!(second.reused_count(), 3);
        assert_eq!(
            second.evaluations, 0,
            "a truthful reuse claim means the statements were not evaluated"
        );
        assert_eq!(
            second.results, first.results,
            "the values must be identical"
        );
        assert_eq!(
            second.variables(),
            first_variables,
            "the workspace must be rebuilt from the applied cached values"
        );
        assert_eq!(second.display("c"), "9");
    }

    #[test]
    fn editing_the_last_of_three_independent_statements_reuses_two() {
        let harness = Harness::new();
        let first = harness.run(1, "a = 2\nb = 5\nc = 7\n");
        assert_eq!(first.reused_count(), 0);

        let second = harness.run(2, "a = 2\nb = 5\nc = 9\n");
        assert_eq!(second.reused, vec![true, true, false]);
        assert_eq!(second.reused_count(), 2);
        assert_eq!(
            second.evaluations, 1,
            "only the edited statement recomputes"
        );
        assert_eq!(second.results, vec!["2", "5", "9"]);
    }

    #[test]
    fn changing_a_recomputes_b_transitively_and_leaves_c_reused() {
        let harness = Harness::new();
        let first = harness.run(1, "a = 2\nb = a + 1\nc = pi(100)\n");
        assert_eq!(first.reused_count(), 0);
        let original_c = first.display("c");

        let second = harness.run(2, "a = 5\nb = a + 1\nc = pi(100)\n");
        assert_eq!(
            second.reused,
            vec![false, false, true],
            "a and its dependent b recompute; the independent c is reused"
        );
        assert_eq!(second.reused_count(), 1);
        assert_eq!(second.evaluations, 2);
        assert_eq!(second.display("a"), "5");
        assert_eq!(second.display("b"), "6", "b must see the new a");
        assert_eq!(
            second.display("c"),
            original_c,
            "c must be re-applied from the cache with its exact value"
        );
        assert_eq!(
            fractional_digits(&second.display("c")),
            100,
            "pi(100) publishes 100 places"
        );
    }

    #[test]
    fn redefining_a_user_function_invalidates_call_sites_and_their_callers() {
        let harness = Harness::new();
        let base = "func f(x) = x + 1\nfunc g(x) = f(x) * 2\ny = g(3)\nq = f(1)\nz = 10\n";
        let first = harness.run(1, base);
        assert_eq!(first.reused_count(), 0);
        assert_eq!(first.display("y"), "8");
        assert_eq!(first.display("q"), "2");

        // The identical source re-runs: the definitions evaluate again (they are never memoized)
        // but their text did not change, so the call sites remain reusable.
        let second = harness.run(2, base);
        assert_eq!(
            second.reused,
            vec![false, false, true, true, true],
            "an unchanged definition must not invalidate its call sites"
        );
        assert_eq!(second.evaluations, 2, "only the two definitions evaluate");

        // Redefining f invalidates its direct call site q, y (which calls g which calls f) and
        // leaves the independent z reusable.
        let changed = "func f(x) = x + 10\nfunc g(x) = f(x) * 2\ny = g(3)\nq = f(1)\nz = 10\n";
        let third = harness.run(3, changed);
        assert_eq!(third.reused, vec![false, false, false, false, true]);
        assert_eq!(third.reused_count(), 1);
        assert_eq!(third.evaluations, 4, "two definitions, y and q recompute");
        assert_eq!(third.display("y"), "26", "y is (3 + 10) * 2");
        assert_eq!(third.display("q"), "11", "q is 1 + 10");
        assert_eq!(third.display("z"), "10");
    }

    #[test]
    fn print_plot_and_setprecision_never_reuse() {
        let harness = Harness::new();
        let source = "print(\"one\")\nplot(1..3, (1..3)^2)\nsetprecision(20)\n";
        for run in 1..=2u64 {
            let observed = harness.run(run, source);
            assert_eq!(
                observed.reused,
                vec![false, false, false],
                "run {run}: an effectful statement must re-evaluate every run"
            );
            assert_eq!(observed.reused_count(), 0);
            assert_eq!(
                observed.evaluations, 3,
                "run {run}: print, plot and setprecision all evaluate"
            );
            assert_eq!(
                observed.snapshot.output,
                vec!["one".to_string()],
                "run {run}: print ran once, not replayed from a cache"
            );
            assert!(
                observed.snapshot.plot.is_some(),
                "run {run}: the plot statement actually executed"
            );
        }
    }

    #[test]
    fn a_precision_change_reuses_nothing() {
        let harness = Harness::new();
        let at_1000 = harness.run_at(1, "sqrt(2)\n", 1000);
        assert_eq!(at_1000.reused, vec![false]);
        assert_eq!(at_1000.evaluations, 1);
        assert_eq!(fractional_digits(&at_1000.snapshot.result), 1000);

        let at_20 = harness.run_at(2, "sqrt(2)\n", 20);
        assert_eq!(
            at_20.reused,
            vec![false],
            "a precision change must not reuse the 1000-place result"
        );
        assert_eq!(at_20.evaluations, 1);
        assert_eq!(at_20.snapshot.result, "1.4142135623730950488");
        assert_eq!(fractional_digits(&at_20.snapshot.result), 19);

        // Back at the same precision, the first run's entry is still valid.
        let again = harness.run_at(3, "sqrt(2)\n", 1000);
        assert_eq!(again.reused, vec![true]);
        assert_eq!(again.evaluations, 0);
        assert_eq!(again.snapshot.result, at_1000.snapshot.result);
    }

    #[test]
    fn reuse_counts_are_exactly_the_cache_hits() {
        let harness = Harness::new();
        let source = "a = 2\nb = a + 1\nc = pi(100)\nd = b + c\n";

        let first = harness.run(1, source);
        assert_eq!(first.reused_count(), 0);
        assert_eq!(first.evaluations, 4);
        assert_eq!(
            harness.host.cached_entries(harness.document),
            4,
            "one entry per committed statement"
        );

        let second = harness.run(2, source);
        assert_eq!(second.reused_count(), 4);
        assert_eq!(
            second.reused_count(),
            second.reused.iter().filter(|flag| **flag).count(),
            "the reported total is exactly the number of reused commits"
        );
        assert_eq!(second.evaluations, 0);

        let third = harness.run(3, "a = 2\nb = a + 1\nc = pi(100)\nd = b + c + 1\n");
        assert_eq!(third.reused, vec![true, true, true, false]);
        assert_eq!(third.reused_count(), 3);
        assert_eq!(third.evaluations, 1);
        assert_eq!(
            third.reused_count(),
            third.reused.iter().filter(|flag| **flag).count()
        );
        assert_eq!(
            harness.host.cached_entries(harness.document),
            5,
            "the edited statement adds its own key instead of replacing another"
        );
    }

    #[test]
    fn a_symbolic_result_reuses_with_the_same_projection_and_inspection() {
        let harness = Harness::new();
        let source = "x = symbol(\"x\")\ny = diff(x^3, x)\n";
        let first = harness.run(1, source);
        assert_eq!(first.reused_count(), 0);
        assert!(first.snapshot.result_structured.is_some());
        assert!(first.snapshot.result_inspection.is_some());

        let second = harness.run(2, source);
        assert_eq!(second.reused, vec![true, true]);
        assert_eq!(second.evaluations, 0);
        assert_eq!(second.snapshot.result, first.snapshot.result);
        assert_eq!(
            second.snapshot.result_structured, first.snapshot.result_structured,
            "the commit must carry the engine's projection the cache stored"
        );
        assert_eq!(
            second.snapshot.result_inspection, first.snapshot.result_inspection,
            "the commit must carry the inspection payload the cache stored"
        );
        assert_eq!(second.variables(), first.variables());
    }

    #[test]
    fn an_engine_that_cannot_apply_a_cached_value_never_reports_reuse() {
        let harness = Harness::refusing();
        let source = "a = 2\nb = a + 1\nc = b * 3\n";
        let first = harness.run(1, source);
        assert_eq!(first.reused_count(), 0);

        let second = harness.run(2, source);
        assert_eq!(
            second.reused,
            vec![false, false, false],
            "a cached value that could not be applied must be evaluated, not claimed"
        );
        assert_eq!(
            second.evaluations, 3,
            "every statement is evaluated when the apply seam refuses"
        );
        assert_eq!(second.results, first.results);
    }

    #[test]
    fn run_all_fresh_clears_the_cache_and_yields_the_same_values() {
        let harness = Harness::new();
        let source = "a = 2\nb = a + 1\nc = b * 3\n";
        let first = harness.run(1, source);
        let second = harness.run(2, source);
        assert_eq!(second.reused_count(), 3);

        harness.host.clear_cache(harness.document);
        let fresh = harness.run(3, source);
        assert_eq!(
            fresh.reused_count(),
            0,
            "Run All Fresh evaluates everything"
        );
        assert_eq!(fresh.evaluations, 3);
        assert_eq!(
            fresh.snapshot.variables, first.snapshot.variables,
            "the fresh run must produce the same workspace"
        );
        assert_eq!(fresh.results, first.results);
    }

    // -----------------------------------------------------------------------------------------
    // The metamorphic test: 20 random edits, incremental final snapshot vs a fresh run
    // -----------------------------------------------------------------------------------------

    /// A deterministic xorshift PRNG; the test must be reproducible, so there is no `rand`.
    struct Rng(u64);

    impl Rng {
        fn next(&mut self) -> u64 {
            let mut value = self.0;
            value ^= value << 13;
            value ^= value >> 7;
            value ^= value << 17;
            self.0 = value;
            value
        }

        fn pick(&mut self, bound: usize) -> usize {
            if bound == 0 {
                0
            } else {
                (self.next() % bound as u64) as usize
            }
        }
    }

    /// The statements random edits draw from: pure definitions, dependent arithmetic, effects
    /// and a user-function call chain, so a random script exercises every dependency shape.
    const CATALOGUE: [&str; 14] = [
        "a = 2",
        "a = 7",
        "b = a + 1",
        "b = a * 3",
        "c = pi(60)",
        "d = b + c",
        "func f(x) = x + 1",
        "func f(x) = x + 10",
        "func g(x) = f(x) * 2",
        "y = g(a)",
        "z = sqrt(2)",
        "print(\"tick\")",
        "w = b - 1",
        "s = \"text\"",
    ];

    fn edit_script(mut script: Vec<String>, rng: &mut Rng, step: usize) -> Vec<String> {
        if step == 0 {
            // J-07's shape: the first edit changes only the last statement, so the run that
            // follows it observes reuse before the random edits begin.
            if let Some(last) = script.last_mut() {
                *last = "d = b + c".to_string();
            }
            return script;
        }
        match rng.pick(3) {
            0 => {
                let index = rng.pick(script.len());
                script[index] = CATALOGUE[rng.pick(CATALOGUE.len())].to_string();
            }
            1 => {
                let index = rng.pick(script.len() + 1);
                script.insert(index, CATALOGUE[rng.pick(CATALOGUE.len())].to_string());
            }
            _ => {
                if script.len() > 1 {
                    let index = rng.pick(script.len());
                    script.remove(index);
                } else {
                    script.push(CATALOGUE[rng.pick(CATALOGUE.len())].to_string());
                }
            }
        }
        script
    }

    /// One run's observable state: the final snapshot plus the log lines of that run.
    #[derive(Debug, PartialEq, Eq)]
    struct Observation {
        result: String,
        result_structured: Option<Json>,
        variables: Vec<(String, String, String)>,
        variables_legacy: Vec<(String, String)>,
        committed: usize,
        total: usize,
        revision: u64,
        diagnostic: Option<String>,
        logs: Vec<(LogKind, Option<usize>, String)>,
    }

    /// Wall-clock milliseconds and the run id are not state; every other character is compared.
    fn normalize_log_text(text: &str) -> String {
        let normalized = match text.find(" in ") {
            Some(index) => format!("{} in <elapsed>", &text[..index]),
            None => text.to_string(),
        };
        // The run id is an identity, not state: the incremental document's final run is its
        // twentieth, while the fresh document's single run is its first.
        match normalized.find(", run ") {
            Some(index) => {
                let marker = ", run ".len();
                let tail = &normalized[index + marker..];
                let end = tail.find(')').unwrap_or(tail.len());
                format!("{}<run>{}", &normalized[..index + marker], &tail[end..])
            }
            None => normalized,
        }
    }

    fn observe(workbench: &Workbench, log_start: usize) -> Observation {
        let document = &workbench.documents()[0];
        let snapshot = document
            .snapshot
            .as_ref()
            .expect("a terminal snapshot was published");
        Observation {
            result: snapshot.result.clone(),
            result_structured: snapshot.result_structured.clone(),
            variables: snapshot
                .state
                .variables
                .iter()
                .map(|variable| {
                    (
                        variable.name.clone(),
                        variable.kind.clone(),
                        variable.display.clone(),
                    )
                })
                .collect(),
            variables_legacy: snapshot.variables.clone(),
            committed: snapshot.committed_steps,
            total: snapshot.total,
            revision: snapshot.revision.0,
            diagnostic: snapshot.diagnostic.clone(),
            logs: document.logs[log_start..]
                .iter()
                .map(|line| (line.kind, line.statement, normalize_log_text(&line.text)))
                .collect(),
        }
    }

    fn real_workbench() -> Workbench {
        Workbench::with_factory(Box::new(|| Box::new(RealEngine::new())))
    }

    #[test]
    fn twenty_random_edits_match_a_fresh_run() {
        let mut script: Vec<String> = ["a = 2", "b = a + 1", "c = pi(60)", "y = b + c"]
            .iter()
            .map(|statement| statement.to_string())
            .collect();
        let mut rng = Rng(0x5eed_1234_abcd_0001);
        let mut previous = script.join("\n");
        let mut sources: Vec<String> = Vec::new();
        for step in 0..20usize {
            script = edit_script(script, &mut rng, step);
            let mut source = script.join("\n");
            if source == previous {
                script.push(format!("marker = {step}"));
                source = script.join("\n");
            }
            assert_ne!(
                source, previous,
                "every edit must change the source so the revision advances"
            );
            previous = source.clone();
            sources.push(source);
        }

        // The incremental document: run after every edit, so each run starts from the cache the
        // previous one left behind.
        let mut incremental = real_workbench();
        let mut total_reused = 0usize;
        let mut second_run_reused = 0usize;
        let mut final_log_start = 0usize;
        for (step, source) in sources.iter().enumerate() {
            incremental.set_source(0, source.clone());
            let log_start = incremental.documents()[0].logs.len();
            assert!(
                incremental.run_active(),
                "incremental run {step} was accepted"
            );
            assert!(
                incremental.wait_until_idle(Duration::from_secs(120)),
                "incremental run {step} reached a terminal state"
            );
            let document = &incremental.documents()[0];
            let snapshot = document.snapshot.as_ref().expect("a terminal snapshot");
            assert_eq!(
                snapshot.revision.0, document.revision,
                "the snapshot names the document's current revision"
            );
            total_reused += document.reused;
            if step == 1 {
                second_run_reused = document.reused;
            }
            if step == 19 {
                final_log_start = log_start;
            }
        }
        assert!(
            second_run_reused >= 1,
            "the second run must actually have reused statements, or this test compares two \
             fresh runs (second run reused {second_run_reused})"
        );
        assert!(
            total_reused > 0,
            "no statement was reused across the 20 incremental runs"
        );

        // The fresh document: the same 20 edits (so the same revision), no runs, then one run.
        let mut fresh = real_workbench();
        for source in &sources {
            fresh.set_source(0, source.clone());
        }
        let fresh_log_start = fresh.documents()[0].logs.len();
        assert!(fresh.run_active(), "the fresh run was accepted");
        assert!(
            fresh.wait_until_idle(Duration::from_secs(120)),
            "the fresh run reached a terminal state"
        );

        let incremental_observation = observe(&incremental, final_log_start);
        let fresh_observation = observe(&fresh, fresh_log_start);
        assert_eq!(
            incremental_observation.committed, fresh_observation.committed,
            "the two runs must commit the same prefix"
        );
        assert_eq!(
            incremental_observation, fresh_observation,
            "the incremental final snapshot (variables with kinds, revision, log lines) must \
             equal a fresh run's for the same source"
        );
        assert_eq!(
            incremental_observation.revision, 20,
            "the final source is the twentieth edit in both documents"
        );
        eprintln!(
            "metamorphic: 20 edits, {total_reused} reused statements, final revision {}",
            incremental_observation.revision
        );
    }
}
