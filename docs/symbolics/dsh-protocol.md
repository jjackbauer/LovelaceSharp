# DSH structured protocol (v1)

The noninteractive runner (`Lovelace.Run`) is the agent-facing surface. Its contract is a
**versioned JSON envelope on stdout**, designed so that an agent never has to parse a display
string to recover mathematical meaning.

## Invariants

1. **stdout carries the envelope and nothing else.** Anything the script prints with
   `print(...)` is captured and returned in the top-level `output` array.
2. **Every value is structural.** Every field is one of `scalar | symbolic | array | record |
   domain | null`; nothing degrades to text because the serializer lacked a case.
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
{ "kind": "Null" }
{ "kind": "Record", "type": "SolveResult", "fields": [ { "name": "status", "value": { "kind": "Text", "value": "Solved" } } ] }
```

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
        { "name": "status", "value": { "kind": "Text", "value": "Solved" } },
        { "name": "domain", "value": { "kind": "Domain", "domain": "complex" } },
        { "name": "complete", "value": { "kind": "Boolean", "value": "true" } },
        { "name": "solutions", "value": { "kind": "Array", "shape": [2], "elements": [
          { "kind": "Record", "type": "Solution", "fields": [
            { "name": "value", "value": { "kind": "Symbolic", "pretty": "-2", "canonical": "(rat -2 1)" } },
            { "name": "conditions", "value": { "kind": "Array", "shape": [0], "elements": [] } },
            { "name": "multiplicity", "value": { "kind": "Integer", "value": "1" } },
            { "name": "exactness", "value": { "kind": "Text", "value": "Exact" } } ] } ] } }
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

An agent can answer, from structure alone: **Was it solved?** (`status`), **over what domain?**
(`domain`), **is it complete?** (`complete`), **how many solutions?** (`solutions.shape`),
**what is each solution?** (`solutions.elements[].fields[value].canonical`), **what conditions
apply to each?** (`...fields[conditions]`), **is it exact?** (`...fields[exactness]`).

When the solver cannot represent the whole set, `status` is `Partial` and
`unrepresented_count`/`unrepresented_reason` say how much is missing and why. When nothing is
representable the status is `Unevaluated` — never a complete-looking subset.

## Error envelope

```json
{ "protocolVersion": 1, "symbolicFormatVersion": "#!lovelace-sym 1", "mathIrVersion": 2,
  "ok": false, "code": "InvalidOperation", "category": "DomainError",
  "message": "solve(): currently supports domains real and complex; got integer.",
  "recoverable": true,
  "diagnostics": [ { "message": "...", "position": 0, "line": 1, "column": 1 } ],
  "elapsed": "12 ms" }
```

Categories: `ParseError`, `DomainError`, `UnsupportedOperation`, `TypeMismatch`,
`InternalInvariantFailure`. Exit codes: 0 success, 1 script/diagnostic error, 2 usage error.

## CI

`.github/workflows/ci.yml` publishes the runner as a Native AOT binary and then **executes it**
against the scenarios above, asserting the envelope's structure. A packaging regression that
breaks the protocol fails CI.
