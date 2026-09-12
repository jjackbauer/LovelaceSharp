# DSH structured protocol (v1)

The noninteractive runner (`Lovelace.Run`) is the agent-facing surface. Its contract is a
**versioned JSON envelope on stdout**, designed so that an agent never has to parse a display
string to recover mathematical meaning.

## Invariants

1. **stdout carries the envelope and nothing else.** Anything the script prints with
   `print(...)` is captured and returned in the top-level `output` array.
2. **Every value is structural.** Every field is one of `scalar | symbolic | array | record |
   domain | enum | null`; nothing degrades to text because the serializer lacked a case. An
   enum-valued field is never `Text`: it carries the declared enum type, so `SolveStatus.Partial`
   and `Completeness.Partial` can never be confused for one another.
3. **An absent field is `{"kind":"Null"}`**, distinct from an empty string.
4. **Versioned.** `protocolVersion`, `symbolicFormatVersion` and `mathIrVersion` are always present.
   `--print-budget <nodes>` bounds structured renderings: beyond it, `pretty`/`canonical` carry a
   ` …`-terminated prefix of the real rendering (never a re-ordered one) and the value reports
   `truncated: true`, `truncationReason` (`node-budget`/`depth-limit`) and `budget`. Without the
   flag the full canonical form is always available.
   An abbreviation NEVER ends inside a token: the cut lands on the last token boundary at or before
   the allowance, and when the allowance falls inside the rendering's first token that whole token is
   kept (a cut that split it would publish an identifier the value does not contain).
   The same three fields carry the one bound the ENGINE applies to a value of its own accord:
   `evalf(f, digits)` publishes at most **1000** decimal places, so a request above 1000 answers a
   value carrying at most 1000 and reports `truncated: true`, `truncationReason: "digit-cap"` and
   `budget: 1000` — never a silent clamp. A value the cap did not cut (an exact answer, or a count
   at or below 1000) carries none of the three fields.
   Durations are structural: `elapsedTime` is `{value, unit}` next to the human `elapsed`
   string, and `timings` carries one entry per top-level statement, so an agent never parses a
   unit suffix.
5. **camelCase JSON keys; snake_case record field names** (record field names come from the
   kernel records and are part of the contract).
6. **Errors are structural**: `code`, `category`, `message`, `recoverable`, `diagnostics`.

**Payload control.** `--omit-functions` and `--omit-variables` are for agent loops that never read the
registry or the variable table: each flag replaces its array with an EMPTY one — the key stays present, so
a consumer written against the full envelope never has to handle a missing field, and an empty array is
what the flag produced rather than a claim that the registry itself is empty. They change no value, no
display and no `output[]` entry.

## Value forms

```json
{ "kind": "Symbolic", "pretty": "x^2 + 1", "canonical": "(add (rat 1 1) (pow (sym x) (rat 2 1)))",
  "domain": "complex", "exact": true, "nodeCount": 5, "freeSymbols": ["x"] }
{ "kind": "Array", "type": "Array", "shape": [2, 2], "elements": [ /* row-major */ ] }
{ "kind": "Complex", "re": "0", "im": "1", "exact": true }
{ "kind": "Integer", "value": "42", "exact": true }
{ "kind": "Domain", "domain": "real" }
{ "kind": "Boolean", "value": "true" }
{ "kind": "Enum", "type": "SolveStatus", "value": "Partial" }
{ "kind": "Null" }
{ "kind": "Record", "type": "SolveResult", "fields": [ { "name": "status", "value": { "kind": "Enum", "type": "SolveStatus", "value": "Solved" } } ] }
```

### Enum-valued fields

`status`, `completeness`, `exactness`, `classification` and a diagnostic's `category` are
**enums**, never text. The declared enum type is part of the value: `type` names it (`SolveStatus`,
`Completeness`, `SolutionExactness`, `LimitStatus`, `IntegrationStatus`, `TransformStatus`,
`RuleClassification`, `ErrorCategory`) and `value` is the member name, so a consumer switches on
`(type, value)` instead of recognising a spelling. `SolveStatus.Partial` and
`Completeness.Partial` are different answers with the same member name — that is exactly why the
type travels with it. `type()` answers the same declared name: `type(r.status)` is
`SolveStatus`, not `Enum`.

`complete` is a **derived convenience** for the `Completeness` enum: it is `true` exactly when
`completeness` is `Complete` (both are produced from one expression in the kernel, so they
cannot disagree), and it exists so a consumer that only needs the yes/no answer does not have to
compare enum members. `completeness` is the authoritative field.

### The Diagnostic form

Every rich result record (`SolveResult`, `SystemSolveResult`, `TransformResult`, `LimitResult`,
`IntegrationResult`, `OptimizationResult`, `CompilationResult`) **ends with a `diagnostics`
field that is an Array of `Diagnostic` records** — never a text value. When there is nothing to
report the array is **empty** (`{"kind":"Array","shape":[0],"elements":[]}`): never `Null` and
never `""`. The kernel's human-readable note is no longer a field of its own; it rides inside a
diagnostic's `message`.

A `Diagnostic` is a Record named `Diagnostic` with exactly these six fields, in this order:

```json
{ "kind": "Record", "type": "Diagnostic", "fields": [
  { "name": "code",        "value": { "kind": "Text", "value": "solve.unrepresented-roots" } },
  { "name": "category",    "value": { "kind": "Enum", "type": "ErrorCategory", "value": "UnsupportedOperation" } },
  { "name": "message",     "value": { "kind": "Text", "value": "complex algebraic roots not supported (RootOf is real-only in v1)." } },
  { "name": "recoverable", "value": { "kind": "Boolean", "value": "true" } },
  { "name": "location",    "value": { "kind": "Null" } },
  { "name": "details",     "value": { "kind": "Array", "type": "Vector", "shape": [0], "elements": [] } } ] }
```

* `code` is **stable text a consumer matches** (`solve.unrepresented-roots`,
  `transform.budget-exceeded`, `transform.unsatisfiable-conditions`, `limit.unevaluated`, …).
  Message text is for humans and is never the contract.
* `category` is an **Enum of type `ErrorCategory`**: `ParseError`, `DomainError`,
  `UnsupportedOperation`, `BudgetExceeded`, `NoSolution`, `TypeMismatch`,
  `InternalInvariantFailure`.
* `recoverable` says whether the caller can continue: a `transform.unsatisfiable-conditions`
  diagnostic is `false` (retrying cannot give that branch a model).
* `location` is `{"kind":"Null"}` for a kernel-level diagnostic — there is no source span at that
  layer — and otherwise a `DiagnosticLocation` record with `start_line`, `start_column`,
  `end_line`, `end_column`.
* `details` is always an Array (empty when there are none), never `Null`.

An agent can therefore answer **"did anything go wrong, and can I branch on it?"** from
`diagnostics.elements[].fields[category].type` + `value`, without reading prose.

## A real solve envelope (abridged)

`revision` identifies the ENGINE STATE the envelope was produced from. It is monotone within one
engine and opaque to a consumer — only a change matters — and it is **not** a statement count: this
two-statement script reports `82` in a fresh process (deterministically), while `1 + 1` three times
reports `81`. Compare revisions within one engine, never across processes or sessions.

The script is `x = symbol("x"); solve(x^2 - 4 == 0, x)`. `solve` and `solve_full` publish this
SAME record: the short form no longer answers a vector (or, worse, a prose sentence) for one shape of
answer and a record for another. Abridged below only in the nested symbolic renderings, which keep
`pretty`/`canonical` and drop `domain`/`exact`/`nodeCount`/`freeSymbols`.

```json
{
  "protocolVersion": 1,
  "symbolicFormatVersion": "#!lovelace-sym 1",
  "mathIrVersion": 2,
  "ok": true,
  "revision": 82,
  "result": {
    "kind": "Record",
    "display": "SolveResult(status: Solved, ...)",
    "typed": "SolveResult(...) (SolveResult)",
    "structured": {
      "kind": "Record",
      "type": "SolveResult",
      "fields": [
        { "name": "status", "value": { "kind": "Enum", "type": "SolveStatus", "value": "Solved" } },
        { "name": "variable", "value": { "kind": "Symbolic", "pretty": "x", "canonical": "(sym x)" } },
        { "name": "domain", "value": { "kind": "Domain", "domain": "complex" } },
        { "name": "complete", "value": { "kind": "Boolean", "value": "true" } },
        { "name": "completeness", "value": { "kind": "Enum", "type": "Completeness", "value": "Complete" } },
        { "name": "solutions", "value": { "kind": "Array", "type": "Vector", "shape": [2], "elements": [
          { "kind": "Record", "type": "Solution", "fields": [
            { "name": "value", "value": { "kind": "Symbolic", "pretty": "-2", "canonical": "(rat -2 1)" } },
            { "name": "conditions", "value": { "kind": "Array", "shape": [0], "elements": [] } },
            { "name": "multiplicity", "value": { "kind": "Integer", "value": "1" } },
            { "name": "exactness", "value": { "kind": "Enum", "type": "SolutionExactness", "value": "Exact" } } ] },
          { "kind": "Record", "type": "Solution", "fields": [
            { "name": "value", "value": { "kind": "Symbolic", "pretty": "2", "canonical": "(rat 2 1)" } },
            { "name": "conditions", "value": { "kind": "Array", "shape": [0], "elements": [] } },
            { "name": "multiplicity", "value": { "kind": "Integer", "value": "1" } },
            { "name": "exactness", "value": { "kind": "Enum", "type": "SolutionExactness", "value": "Exact" } } ] } ] } },
        { "name": "families", "value": { "kind": "Array", "type": "Vector", "shape": [0], "elements": [] } },
        { "name": "common_conditions", "value": { "kind": "Array", "type": "Vector", "shape": [0], "elements": [] } },
        { "name": "represented_count", "value": { "kind": "Integer", "value": "2" } },
        { "name": "unrepresented_count", "value": { "kind": "Integer", "value": "0" } },
        { "name": "unrepresented_reason", "value": { "kind": "Text", "value": "" } },
        { "name": "diagnostics", "value": { "kind": "Array", "type": "Vector", "shape": [0], "elements": [] } }
      ]
    }
  },
  "output": [],
  "elapsed": "1.2 ms",
  "elapsedTime": { "value": 1.2, "unit": "ms" },
  "timings": [
    { "position": 0, "elapsed": { "value": 1.1, "unit": "ms" }, "resultKind": "Record", "hasOutput": false }
  ]
}
```

`elapsedTime` and `timings[].elapsed` are produced by the same unit selector as the human
`elapsed` string, so the two forms can never disagree. `timings[].position` is the zero-based
offset of the statement in the text the caller supplied (see **Positions and locations**);
`resultKind` is the value kind it produced (`Void` for a statement with no value) and
`hasOutput` says whether it wrote anything with `print`.

An agent can answer, from structure alone: **Was it solved?** (`status`, an `Enum` of type
`SolveStatus`), **over what domain?** (`domain`), **is it complete?** (`completeness`, an `Enum`
of type `Completeness`; `complete` is the derived boolean), **how many solutions?**
(`solutions.shape`), **what is each solution?** (`solutions.elements[].fields[value].canonical`),
**what conditions apply to each?** (`...fields[conditions]`), **is it exact?**
(`...fields[exactness]`, an `Enum` of type `SolutionExactness`).

When the solver cannot represent the whole set, `status` is `Partial` and
`unrepresented_count`/`unrepresented_reason` say how much is missing and why. When nothing is
representable the status is `Unevaluated` — never a complete-looking subset.

`complete` and `completeness` are read off ONE mapping of the effective status
(`SolveCompletenessMapping.Of` in `Lovelace.Symbolics/SymbolicsPlugin.cs`, the file that builds
both `SolveResult` and `SystemSolveResult`), so the two fields cannot disagree and the two records
cannot drift apart:

| `status` | `complete` | `completeness` |
|---|---|---|
| `Solved` | `true` | `Complete` |
| `NoSolutions` | `true` | `Complete` |
| `Partial` | `false` | `Partial` |
| `Unevaluated` | `false` | `Unknown` |
| `BudgetExceeded` | `false` | the kernel's `Completeness` for the subset it stopped on |

`NoSolutions` means the solution set over the **requested domain is provably empty** (§D of the
alignment plan) — over the requested domain, including the case where a denominator or domain
condition removed every candidate. A provably empty set is a complete answer, so it reports
`complete: true` / `completeness: Complete`; nothing is missing from it. Only `Partial`,
`Unevaluated` and `BudgetExceeded` report `complete: false`.

## Positions and locations

Every position the protocol publishes — `timings[].position`, and the error envelope's
`diagnostics[].position` with its `line`/`column` — is a **zero-based offset into the text the
caller supplied**, exactly as that surface received it, counted in **UTF-16 code units** (the units of
the runtime string the script is held in, which is what `Substring` slices):

| surface | the text a position indexes |
|---|---|
| `--eval <script>` | the argument string, character for character |
| `--file <path>` | the file's decoded characters; a leading byte-order mark is the character U+FEFF at offset 0 and is **not** removed |
| `--stdin` | the characters read from standard input |

A non-BMP character (an emoji, say) occupies **two** code units, so a consumer that slices by Unicode
code points rather than by UTF-16 units must map the offset before slicing; for text that is entirely
BMP — every script in this document — the two are the same number.

Byte-identical input therefore reports identical positions on every surface. A position is something
a consumer can **use**: the text it handed over (or, for `--file`, that file's own characters) sliced
at the reported offset starts at the statement the envelope is talking about — `text.Substring(position)`
names it, on every surface, for the same bytes.

A leading byte-order mark is one character and a CRLF is two, because both are part of that text:
`a = 1` / `b = 2` / `det(a)` on three lines reports its third statement at 12 with LF separators,
at 14 with CRLF separators, and at 15 when that CRLF file is saved as "UTF-8 with BOM". The engine
itself drops a leading U+FEFF (the tokenizer does not accept it as whitespace), which is why a
"UTF-8 with BOM" file still evaluates: that drop is the engine's, and it never changes what a
published position indexes.

`line` and `column` are 1-based and read off that same text, with CRLF, CR and LF each ending a
line once — so a Windows script, a Unix script and a classic-Mac script that hold the same statements
report the same line and column.

## Error envelope

```json
{ "protocolVersion": 1, "symbolicFormatVersion": "#!lovelace-sym 1", "mathIrVersion": 2,
  "ok": false, "code": "InvalidOperation", "category": "DomainError",
  "message": "solve(): currently supports domains real and complex; got integer.",
  "recoverable": true,
  "diagnostics": [ { "message": "...", "position": 17, "line": 1, "column": 18 } ],
  "elapsed": "46.22 ms",
  "elapsedTime": { "value": 46.22, "unit": "ms" },
  "timings": [
    { "position": 0,  "elapsed": { "value": 8.89,  "unit": "ms" }, "resultKind": "Symbolic", "hasOutput": false },
    { "position": 17, "elapsed": { "value": 16.41, "unit": "ms" }, "resultKind": "Void",     "hasOutput": false } ] }
```

Invariant 4 is **not scoped to success**: the two structural durations are present on the error path
too, including for an error raised before any engine exists (a parse error, an unreadable script
file), where `timings` is present and empty. A consumer never parses a unit suffix on either path.
The statement that FAILED is still a timed statement: it is reported with `resultKind` `Void`.

Categories are spelled from the one taxonomy, `ErrorCategory`: `ParseError`, `DomainError`,
`UnsupportedOperation`, `BudgetExceeded`, `NoSolution`, `TypeMismatch`,
`InternalInvariantFailure`. Exit codes: 0 success, 1 script/diagnostic error, 2 usage error.

A call with the wrong number of arguments is rejected at the call site, before the builtin body can
index an argument that was never supplied. It crosses as a recoverable argument error
(`code: InvalidArgument`, `category: TypeMismatch`) naming the builtin and both counts, never as an
internal invariant failure:

```json
{ "ok": false, "code": "InvalidArgument", "category": "TypeMismatch", "recoverable": true,
  "message": "compile_full(): expected 2 arguments; got 1." }
```

The counts come from the builtin's declared metadata (`Parameters`, `MinArity`, `Variadic`), so a
builtin that legitimately accepts a shorter form declares it (`symbol(name [, domain])`,
`solve(f, x [, domain])`, `dft(x [, n])`) instead of the validator being widened.

The error envelope's `diagnostics` array is the **parser's source-position form**
(`message`/`position`/`line`/`column`) — a different record from the `Diagnostic` above, and also
an array. Its `position`/`line`/`column` index the caller's text by the same rule as
`timings[].position` (see **Positions and locations**), so the two forms of one envelope can never
disagree about where a failure is. The two are never mixed: a result record carries `Diagnostic`
records, the error envelope carries parse-site diagnostics.

## CI

`.github/workflows/ci.yml` publishes the runner as a Native AOT binary and then **executes it**
against the scenarios above, asserting the envelope's structure. A packaging regression that
breaks the protocol fails CI.
