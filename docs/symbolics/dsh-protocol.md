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
   Durations are structural: `elapsedTime` is `{value, unit}` next to the human `elapsed`
   string, and `timings` carries one entry per top-level statement, so an agent never parses a
   unit suffix.
5. **camelCase JSON keys; snake_case record field names** (record field names come from the
   kernel records and are part of the contract).
6. **Errors are structural**: `code`, `category`, `message`, `recoverable`, `diagnostics`.

## Value forms

```json
{ "kind": "Symbolic", "pretty": "x^2 + 1", "canonical": "(add (pow (sym x) (int 2)) (int 1))",
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
  { "name": "details",     "value": { "kind": "Array", "shape": [0], "elements": [] } } ] }
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

```json
{
  "protocolVersion": 1,
  "symbolicFormatVersion": "#!lovelace-sym 1",
  "mathIrVersion": 2,
  "ok": true,
  "revision": 77,
  "result": {
    "kind": "Record",
    "display": "SolveResult(status: Solved, ...)",
    "typed": "SolveResult(...) (SolveResult)",
    "structured": {
      "kind": "Record",
      "type": "SolveResult",
      "fields": [
        { "name": "status", "value": { "kind": "Enum", "type": "SolveStatus", "value": "Solved" } },
        { "name": "domain", "value": { "kind": "Domain", "domain": "complex" } },
        { "name": "complete", "value": { "kind": "Boolean", "value": "true" } },
        { "name": "completeness", "value": { "kind": "Enum", "type": "Completeness", "value": "Complete" } },
        { "name": "solutions", "value": { "kind": "Array", "shape": [2], "elements": [
          { "kind": "Record", "type": "Solution", "fields": [
            { "name": "value", "value": { "kind": "Symbolic", "pretty": "-2", "canonical": "(rat -2 1)" } },
            { "name": "conditions", "value": { "kind": "Array", "shape": [0], "elements": [] } },
            { "name": "multiplicity", "value": { "kind": "Integer", "value": "1" } },
            { "name": "exactness", "value": { "kind": "Enum", "type": "SolutionExactness", "value": "Exact" } } ] } ] } },
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
source offset of the statement; `resultKind` is the value kind it produced (`Void` for a
statement with no value) and `hasOutput` says whether it wrote anything with `print`.

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

## Error envelope

```json
{ "protocolVersion": 1, "symbolicFormatVersion": "#!lovelace-sym 1", "mathIrVersion": 2,
  "ok": false, "code": "InvalidOperation", "category": "DomainError",
  "message": "solve(): currently supports domains real and complex; got integer.",
  "recoverable": true,
  "diagnostics": [ { "message": "...", "position": 0, "line": 1, "column": 1 } ],
  "elapsed": "12 ms" }
```

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
an array. The two are never mixed: a result record carries `Diagnostic` records, the error
envelope carries parse-site diagnostics.

## CI

`.github/workflows/ci.yml` publishes the runner as a Native AOT binary and then **executes it**
against the scenarios above, asserting the envelope's structure. A packaging regression that
breaks the protocol fails CI.
