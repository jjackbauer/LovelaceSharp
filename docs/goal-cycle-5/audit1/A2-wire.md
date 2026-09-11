# A2 — Wire-protocol audit: is the LovelaceSharp machine protocol honest?

**Product under test:** `out/aot/Lovelace.Run.exe` — 5,681,152 bytes, mtime 2026-09-11 13:41:44, SHA-256 `B7BC889BB69B037AEE0E84BCA86975A53C7501C3CFF4EF7ADF2C99B62B7523E5`
**Contract attacked:** `docs/symbolics/dsh-protocol.md` (210 lines, "DSH structured protocol (v1)") plus the builtin metadata returned by `capabilities()`.
**Independent ground truth:** SymPy 1.14.0 (C:\Users\ricar\dev\.lovelace-tools\python).
**Auditor:** independent adversarial. I did not write this code; I rely only on the frozen binary and the docs.

**Method.** Each probe is a concrete input plus an expected observable derived *beforehand* from the
contract or from mathematics. Command form for every script probe:

    .\out\aot\Lovelace.Run.exe --file <probe>.ls --omit-functions        (plus --print-budget n / --cancel-after ms where noted)

Statements are one line — the language does not treat a newline as a separator. Every FINDING was
re-run from a clean temp file and the second raw envelope is pasted.
**367 distinct probe inputs**, ~510 executions counting the mandated re-runs.

## 1. Verdict index

| # | Sev | Finding (one line) |
|---|-----|--------------------|
| **F30** | **P0** | **`limit_full((1 + 1/x)^x, x, inf)` answers `Value / exists:true / value:1 / exactness:Exact`; the limit is `e` (reproduced in 4 clean files).** |
| F1 | **P0** | `solve_system_full` answers `NoSolutions / complete:true / solutions:[] / diagnostics:[]` for systems that provably have solutions. |
| F2 | **P0** | `IntegrationResult.exactness` is `Approximate` for one verified exact integral and `Exact` for another with the same `status: SolvedExact, verified: true`. |
| F3 | **P0** | `LimitResult.exactness` is `Exact` on all 6 `status: Unevaluated` records whose `value` and `exists` are `Null` — exactness claimed about no result. |
| F4 | **P0** | `Solution.value.exact=false` on an exact algebraic root while the same record's `exactness` is `AlgebraicExact` — two machine fields disagree inside one record. |
| F5 | P1 | `solve(...)` returns a bare `{"kind":"Text"}` prose sentence as the *result* when it cannot represent the set (invariant 2). |
| F6 | P1 | 39 of 123 builtins answer a wrong-arity call with `InvalidOperation/DomainError` and three message shapes; the doc promises one shape. |
| F7 | P1 | `transpose()` leaks a CLR `ArgumentOutOfRangeException` message: the promised call-site arity rejection did not happen. |
| F8 | P1 | No error envelope carries `elapsedTime` (57/57) — the documented structural duration is missing, forcing a parse of `elapsed`. |
| F9 | P1 | Print output is lost on a script error (no `output` key) and moved to `partialOutput` on cancellation — invariant 1. |
| F10 | P1 | A Void final statement yields a success envelope with no `result` key at all instead of `{"kind":"Null"}` (6/152 success envelopes). |
| F11 | P1 | `SystemSolveResult` has no `completeness` field although the doc says both solve records carry it and "cannot drift apart". |
| F12 | P1 | `capabilities()` lists `integer` as an unsupported domain, yet `symbol("x", integer)` succeeds and the solver itself emits `{"kind":"Domain","domain":"integer"}`. |
| F13 | P1 | An unlisted refusal: `solve_full(x == x, x)` gives `Unevaluated` + `solve.unevaluated`, not one of the 16 advertised operation classes. |
| F14 | P1 | `solve(0 == 1, x)` / `solve_full(2 == 3, x)` gives `DomainError "…got Boolean"`, though the doc says a provably empty set reports `NoSolutions / complete:true`. |
| F15 | P1 | `functions[]` omits the `MinArity` / `Variadic` / optionality metadata the doc says the arity contract is computed from. |
| F16 | P1 | Four fields use empty `Text` where invariant 3 requires `Null` (`unrepresented_reason`, `budget_kind`, `method`, `verification_method`); the doc's own example contradicts the invariant. |
| F17 | P2 | `output[]` elements keep a trailing CR on every element except the last. |
| F18 | P2 | The error envelope's `category` is a bare JSON string while the same taxonomy is a typed `Enum` inside records (doc example vs invariant 2). |
| F19 | P2 | `--print-budget` is not applied to small values: nodeCount 2/3/5 never truncate at any budget; nodeCount 7/10 do. |
| F20 | P2 | Repeated roots are reported two ways: `(x-1)^2==0` gives 2 solutions x multiplicity 1, but `x^2==0` gives 1 solution x multiplicity 2. |
| F21 | P2 | `TransformResult.changed=true` with identical `original`/`expression` and empty `steps`, where the same shape elsewhere reports `false`. |
| F22 | P2 | `det` accepts a numeric matrix; `linsolve` / `matrix_rank` / `inv_full` refuse one — only the `linsolve` case is advertised. |
| F23 | P2 | Usage errors (exit 2) emit no envelope at all (no args, unknown flag, `--print-budget 0`, `--cancel-after 0`). |
| F24 | P2 | `variables[]` entries are display-only (no `structured`), and `MatrixSolveResult` — a record the binary emits — is absent from the doc's record list. |
| F25 | P2 | `evalf(f, digits)` ignores `digits` for an already-numeric input: `evalf(sqrt(2),50)` returns 100 digits while `evalf(1/3,5)` returns 5. |
| F26 | P3 | The canned `limit.unevaluated` message ("coefficient does not evaluate at the point") is emitted for three limits with no coefficient in sight. |
| F27 | P3 | `pretty` is never truncated when `canonical` is, though the doc says "pretty/canonical carry a …-terminated prefix". |
| F28 | P3 | `revision` is undocumented and takes values 80-84 that do not order across scripts (deterministic per script). |
| F29 | P3 | The doc's enum/value-kind/record registries are incomplete (`CapabilitiesExactness`, kind `Natural`, records `MatrixSolveResult` / `SolutionFamily` / `Binding` / `ParameterInfo` / `RewriteStep` / `FiniteCondition`). |

## 2. FINDINGS
### F30 — P0 — the exponential limit `(1 + 1/x)^x` is answered 1 instead of `e`

Probe: `x = symbol("x"); limit_full((1 + 1/x)^x, x, inf)`

Raw (run 1, clean file `n01.ls`):
```json
{"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":true,"revision":82,"result":{"kind":"Record","display":"LimitResult(status: Value, exists: True, value: 1, left: , left_conditions: [], right: , right_conditions: [], conditions: [], exactness: Exact, diagnostics: [])","typed":"LimitResult(status: Value, exists: True, value: 1, left: , left_conditions: [], right: , right_conditions: [], conditions: [], exactness: Exact, diagnostics: []) (LimitResult)","structured":{"kind":"Record","type":"LimitResult","fields":[{"name":"status","value":{"kind":"Enum","type":"LimitStatus","value":"Value"}},{"name":"exists","value":{"kind":"Boolean","value":"true"}},{"name":"value","value":{"kind":"Symbolic","pretty":"1","canonical":"(rat 1 1)","domain":"rational","exact":true,"nodeCount":1,"freeSymbols":[]}},{"name":"left","value":{"kind":"Null"}},{"name":"left_conditions","value":{"kind":"Array","type":"Vector","shape":[0],"elements":[]}},{"name":"right","value":{"kind":"Null"}},{"name":"right_conditions","value":{"kind":"Array","type":"Vector","shape":[0],"elements":[]}},{"name":"conditions","value":{"kind":"Array","type":"Vector","shape":[0],"elements":[]}},{"name":"exactness","value":{"kind":"Enum","type":"SolutionExactness","value":"Exact"}},{"name":"diagnostics","value":{"kind":"Array","type":"Vector","shape":[0],"elements":[]}}]}},"output":[],"variables":[{"name":"_","kind":"Record","display":"LimitResult(status: Value, exists: True, value: 1, left: , left_conditions: [], right: , right_conditions: [], conditions: [], exactness: Exact, diagnostics: [])"},{"name":"x","kind":"Symbolic","display":"x"}],"functions":[],"elapsed":"513 \u00B5s","elapsedTime":{"value":513,"unit":"\u00B5s"},"timings":[{"position":0,"elapsed":{"value":68.5,"unit":"\u00B5s"},"resultKind":"Symbolic","hasOutput":false},{"position":17,"elapsed":{"value":390.6,"unit":"\u00B5s"},"resultKind":"Record","hasOutput":false}]}
```

Raw (run 2, clean file `n03.ls`, same script, different temp file — identical verdict):
```json
... "display":"LimitResult(status: Value, exists: True, value: 1, ..., exactness: Exact, diagnostics: [])" ... "value":{"kind":"Symbolic","pretty":"1","canonical":"(rat 1 1)","domain":"rational","exact":true,...}
```

Two further shapes on the same defect (both clean files):
* `n02.ls` — `limit_full((1 + 1/n)^n, n, inf)` with a different symbol name: `status=Value, exists=true, value=1, exactness=Exact`.
* `n04.ls` — `limit_full((1 + 2/x)^x, x, inf)`: `status=Value, exists=true, value=1, exactness=Exact`.
* `n05.ls` — `limit_full(exp(x*log(1 + 1/x)), x, inf)`: `Unevaluated` (here the kernel declines rather than answering).

Independent ground truth:

    $ python3 -c "from sympy import *; x=symbols('x'); print(limit((1+1/x)**x, x, oo)); print(limit((1+2/x)**x, x, oo))"
    E
    exp(2)

`lim (1 + 1/x)^x = e = 2.718281828459045...` and `lim (1 + 2/x)^x = e^2 = 7.389056...`. The kernel reports
`value: 1` with `exists: true`, an empty diagnostics array and `exactness: Exact` for both — the classic
1^infinity indeterminate-form collapse (the base tends to 1, and 1 is returned as the value).

**Why P0:** this is a wrong answer to a well-posed limit, asserted as an existing exact value with no
diagnostic; it is strictly worse than the honest `Unevaluated` the same builtin returns for
`(1+1/x)^x` written as `exp(x*log(1+1/x))` (n05). This supersedes the earlier "COULD NOT DECIDE" entry
for this input (section 4).



### F1 — P0 — `solve_system_full` reports a provably solvable system as having no solutions, and claims completeness

Probe A: `x = symbol("x"); y = symbol("y"); solve_system_full([x^2 + y == 1, x - y == 0], [x, y])`

Raw (run 1, clean file `u01.ls`):
```json
{"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":true,"revision":83,"result":{"kind":"Record","display":"SystemSolveResult(status: NoSolutions, domain: complex, complete: True, solutions: [], diagnostics: [])","typed":"SystemSolveResult(status: NoSolutions, domain: complex, complete: True, solutions: [], diagnostics: []) (SystemSolveResult)","structured":{"kind":"Record","type":"SystemSolveResult","fields":[{"name":"status","value":{"kind":"Enum","type":"SolveStatus","value":"NoSolutions"}},{"name":"domain","value":{"kind":"Domain","domain":"complex"}},{"name":"complete","value":{"kind":"Boolean","value":"true"}},{"name":"solutions","value":{"kind":"Array","type":"Vector","shape":[0],"elements":[]}},{"name":"diagnostics","value":{"kind":"Array","type":"Vector","shape":[0],"elements":[]}}]}},"output":[],"variables":[{"name":"_","kind":"Record","display":"SystemSolveResult(status: NoSolutions, domain: complex, complete: True, solutions: [], diagnostics: [])"},{"name":"x","kind":"Symbolic","display":"x"},{"name":"y","kind":"Symbolic","display":"y"}],"functions":[],"elapsed":"1.62 ms","elapsedTime":{"value":1.62,"unit":"ms"},"timings":[{"position":0,"elapsed":{"value":108.1,"unit":"\u00B5s"},"resultKind":"Symbolic","hasOutput":false},{"position":17,"elapsed":{"value":33,"unit":"\u00B5s"},"resultKind":"Symbolic","hasOutput":false},{"position":34,"elapsed":{"value":1.4,"unit":"ms"},"resultKind":"Record","hasOutput":false}]}
```

Raw (run 2, clean file `v01.ls`, same verdict):
```json
... "display":"SystemSolveResult(status: NoSolutions, domain: complex, complete: True, solutions: [], diagnostics: [])" ... [{"name":"status","value":{"kind":"Enum","type":"SolveStatus","value":"NoSolutions"}},{"name":"domain","value":{"kind":"Domain","domain":"complex"}},{"name":"complete","value":{"kind":"Boolean","value":"true"}},{"name":"solutions","value":{"kind":"Array","type":"Vector","shape":[0],"elements":[]}},{"name":"diagnostics","value":{"kind":"Array","type":"Vector","shape":[0],"elements":[]}}]
```

Run 3 (`t24.ls`, different statement offsets) produced the same `NoSolutions / complete: true`.

Independent ground truth:

    $ python3 -c "from sympy import *; x,y=symbols('x y'); print(solve([x**2+y-1, x-y],[x,y],dict=True))"
    [{x: -1/2 + sqrt(5)/2, y: -1/2 + sqrt(5)/2}, {x: -sqrt(5)/2 - 1/2, y: -sqrt(5)/2 - 1/2}]

Two real solutions. The single-equation solver gets the same polynomial right: `solve_full(x^2 + x - 1 == 0, x)` (probe `u03`) returns `status: Solved, 2 solutions, completeness: Complete`. Probe B (second shape, `v04.ls`, re-run as `h01.ls`): `solve_system_full([x^2 + y^2 == 1, y == x], [x, y])` gives `NoSolutions, complete: true, solutions: [], diagnostics: []`, while SymPy `solve([x**2+y**2-1, y-x])` returns two solutions (+-sqrt(2)/2).

**Why P0:** a wrong mathematical answer plus a wrong completeness claim (`complete: true`, and per the doc "a provably empty set is a complete answer") plus zero diagnostics. This is not a refusal: the record asserts there is nothing to find.

### F2 — P0 — `IntegrationResult.exactness` contradicts its own `status`

Probe: `x = symbol("x"); integrate_full(sin(x), x)`

Raw (run 1, `t13.ls`, decisive fields):
```json
{"name":"status","value":{"kind":"Enum","type":"IntegrationStatus","value":"SolvedExact"}},
{"name":"expression","value":{"kind":"Symbolic","pretty":"-cos(x)","canonical":"(mul (rat -1 1) (fn cos (sym x)))","domain":"complex","exact":false,"nodeCount":4,"freeSymbols":["x"]}},
{"name":"conditions","value":{"kind":"Array","type":"Vector","shape":[0],"elements":[]}},
{"name":"verified","value":{"kind":"Boolean","value":"true"}},
{"name":"method","value":{"kind":"Text","value":"table"}},
{"name":"verification_method","value":{"kind":"Text","value":"differentiation+canonical"}},
{"name":"exactness","value":{"kind":"Enum","type":"SolutionExactness","value":"Approximate"}},
{"name":"diagnostics","value":{"kind":"Array","type":"Vector","shape":[0],"elements":[]}}
```

Raw (run 2, `v08.ls`): identical — `status=SolvedExact, verified=true, exactness=Approximate`.
Contrast probe (`s20.ls`): `integrate_full(x^2, x)` gives `status=SolvedExact, exactness=Exact`.
Corpus census over every `IntegrationResult` envelope in this audit: `SolvedExact->Approximate` (2), `SolvedExact->Exact` (1), `SolvedConditional->Approximate` (2), `Unevaluated->Approximate` (3). SymPy: `integrate(sin(x),x) = -cos(x)`.

**Why P0:** the product states "Approximate" about an exact, independently verified antiderivative, and it is not even self-consistent — the same status+verified combination yields `Exact` for `x^2`.

### F3 — P0 — `LimitResult.exactness: Exact` on records that carry no value at all

Probe: `x = symbol("x"); limit_full(sin(x), x, inf)`

Raw (run 1, `p03.ls`):
```json
{"name":"status","value":{"kind":"Enum","type":"LimitStatus","value":"Unevaluated"}},{"name":"exists","value":{"kind":"Null"}},{"name":"value","value":{"kind":"Null"}},{"name":"left","value":{"kind":"Null"}},{"name":"right","value":{"kind":"Null"}},{"name":"conditions","value":{"kind":"Array","type":"Vector","shape":[0],"elements":[]}},{"name":"exactness","value":{"kind":"Enum","type":"SolutionExactness","value":"Exact"}},{"name":"diagnostics","value":{"kind":"Array","type":"Vector","shape":[1],"elements":[{"kind":"Record","type":"Diagnostic","fields":[{"name":"code","value":{"kind":"Text","value":"limit.unevaluated"}},{"name":"category","value":{"kind":"Enum","type":"ErrorCategory","value":"UnsupportedOperation"}},{"name":"message","value":{"kind":"Text","value":"coefficient does not evaluate at the point"}},{"name":"recoverable","value":{"kind":"Boolean","value":"true"}},{"name":"location","value":{"kind":"Null"}},{"name":"details","value":{"kind":"Array","type":"Vector","shape":[0],"elements":[]}}]}]}
```

Raw (run 2, `v09.ls`): identical field-for-field (`status: Unevaluated, exists: Null, value: Null, exactness: Exact`).
Three further clean-file runs return the same shape: `s16.ls` (`limit_full(abs(x)/x, x, 0)`), `s17.ls` (`limit_full(x*sin(1/x), x, 0)`), `k02.ls` (`limit_full(exp(-1/x^2), x, 0)`). Corpus census: every `LimitResult` in this audit — `Value` (3), `DoesNotExist` (1), `Unevaluated` (6) — reports `exactness: Exact`. SymPy: `limit(exp(-1/x**2), x, 0) = 0` and `limit(x*sin(1/x), x, 0) = 0` (both limits exist); `limit(sin(x), x, oo) = AccumBounds(-1, 1)`; `limit(Abs(x)/x, x, 0, '+-')` raises "The limit does not exist since left hand limit = -1 and right hand limit = 1".

**Why P0:** the field asserts exactness of a result that does not exist (`value` and `exists` are `Null`), it never discriminates (it is `Exact` for all three statuses), and the one record type that reports the same `SolutionExactness` enum for an unevaluated outcome says `Approximate` (F2) — the enum means opposite things in the two records.

### F4 — P0 — `Symbolic.exact` is false for an exact algebraic solution, contradicting the record's own `exactness`

Probe: `x = symbol("x"); solve_full(x^2 - 2 == 0, x, real)`

Raw (run 1, `t01.ls`, one Solution record):
```json
{"name":"value","value":{"kind":"Symbolic","pretty":"-1/2*sqrt(8)","canonical":"(mul (rat -1 2) (pow (rat 8 1) (rat 1 2)))","domain":"real","exact":false,"nodeCount":5,"freeSymbols":[]}},
{"name":"multiplicity","value":{"kind":"Integer","value":"1","exact":true}},
{"name":"exactness","value":{"kind":"Enum","type":"SolutionExactness","value":"AlgebraicExact"}}
```

Raw (run 2, `v10.ls`): same — `"exact":false` next to `SolutionExactness/AlgebraicExact`. The same value through the plain solver (run 1 `p19.ls`, re-run) is `{"pretty":"1/2*sqrt(8)","canonical":"(mul (rat 1 2) (pow (rat 8 1) (rat 1 2)))","domain":"real","exact":false}`. SymPy: `solve(x**2-2,x) = [-sqrt(2), sqrt(2)]` — a finite exact form with no float anywhere. Nor is the flag "contains a radical": `solve_full(x^3-2==0,x)` (`t25.ls`) returns the three exact roots including `2^(1/3)*(-i*sqrt(3)/2 - 1/2)` with `exactness: AlgebraicExact`, and `rootof(...)` values carry `"exact":true` (`p02.ls`, `c03.ls`). Further instances of `exact:false` on exact symbolic quantities: `pi` and `k*pi` in a solution family (`s13.ls`, `t39.ls`), `exp(log(x))` (`s02.ls`), `sqrt(x^2)` (`s12.ls`).

**Why P0:** a wrong exactness claim in a machine-visible field, contradicted by a sibling field of the same record.

### F5 — P1 — `solve` degrades its result to prose

Raw (run 1, `s10.ls`; run 2, `v07.ls`, identical):
```json
{"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":true,"revision":82,"result":{"kind":"Text","display":"no real solutions","typed":"no real solutions","structured":{"kind":"Text","value":"no real solutions"}},"output":[],"variables":[{"name":"_","kind":"Text","display":"no real solutions"},{"name":"x","kind":"Symbolic","display":"x"}],"functions":[],"elapsed":"345.1 \u00B5s","elapsedTime":{"value":345.1,"unit":"\u00B5s"},"timings":[{"position":0,"elapsed":{"value":60.2,"unit":"\u00B5s"},"resultKind":"Symbolic","hasOutput":false},{"position":17,"elapsed":{"value":243.2,"unit":"\u00B5s"},"resultKind":"Text","hasOutput":false}]}
```

Second instance (`t17.ls`): `solve(x == x, x)` gives `result.structured = {"kind":"Text","value":"unevaluated: 0 = 0: every value is a solution."}`.
Third: `solve_system(...)` on the F1 system (`u02.ls`) gives `{"kind":"Text","value":"no solutions"}`.
Fourth (`p18.ls`): `r = solve(x^2-4==0,x); type(r.status)` fails with `ok:false, "member 'status' is not available on type 'Vector': member access requires a record result."`

Invariant 2: "Every value is structural ... nothing degrades to text because the serializer lacked a case"; the doc's stated purpose is that an agent "never has to parse a display string to recover mathematical meaning".

### F6 — P1 — the uniform arity error is not uniform: 39/123 builtins contradict it

Doc: "It crosses as a recoverable argument error (`code: InvalidArgument`, `category: TypeMismatch`) naming the builtin and both counts, never as an internal invariant failure."

Census: every one of the 123 builtins called with zero arguments via `--eval "<name>()"`, then the offenders re-run in a second process:

```text
abs|InvalidOperation|DomainError|abs() expects exactly 1 argument(s), but got 0.
linsolve|InvalidOperation|DomainError|linsolve() expects exactly 2 argument(s), but got 0.
plot|InvalidOperation|DomainError|plot() expects 1 to 3 arguments, but got 0.
max|InvalidOperation|DomainError|Expected 1 or 2 arguments, but got 0.
ones|InvalidOperation|DomainError|ones() requires at least one dimension.
zeros|InvalidOperation|DomainError|zeros() requires at least one dimension.
```

The second run reproduced all 40 rows byte-for-byte. Compliant for contrast (`solve`, `diff`, `fft`, `cos`, `compile_full`): `compile_full(): expected 2 arguments; got 1.` — exactly the doc's example. Three non-conforming message shapes exist: `X() expects exactly N argument(s), but got M.` (27 builtins, including the ungrammatical "1 argument(s)"), `Expected 1 or 2 arguments, but got 0.` (6 builtins that never name the builtin), `X() requires at least one dimension.` (no counts at all). Full 123-row table in section 3.6. Every non-conforming builtin is a core (non-plugin) builtin; all plugin builtins comply.

### F7 — P1 — `transpose()` raises inside the builtin body and the exception text crosses the wire

Raw (run 1 = census row, run 2 = `v14.ls`, identical):
```json
{"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":false,"code":"InvalidArgument","category":"TypeMismatch","message":"Index was out of range. Must be non-negative and less than the size of the collection. (Parameter 'index')","recoverable":true,"diagnostics":[{"message":"Index was out of range. Must be non-negative and less than the size of the collection. (Parameter 'index')","position":0,"line":1,"column":1}],"elapsed":"199.5 \u00B5s"}
```

Doc: "A call with the wrong number of arguments is rejected at the call site, **before the builtin body can index an argument that was never supplied** ... never as an internal invariant failure." Here the body indexed first; the BCL `ArgumentOutOfRangeException` text is the entire message, naming neither builtin nor counts, and the envelope asserts `TypeMismatch` — telling the agent its argument types were wrong when the fault is the server's. Graded P1 (the failure was caught per call and an envelope was still emitted, so it is a documented-contract breach rather than an uncategorised crash); a P0 reading exists because an internal exception reached the wire.

### F8 — P1 — error envelopes have no structural duration

All 57 error envelopes produced by this audit carry exactly this key set:
`protocolVersion, symbolicFormatVersion, mathIrVersion, ok, code, category, message, recoverable, diagnostics, elapsed` — and therefore no `elapsedTime`, no `revision`, no `timings`, no `output`, no `variables`.

Raw (run 1 `p05.ls` / `p07.ls`; re-run with identical shape):
```json
{"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":false,"code":"UnsupportedOperation","category":"UnsupportedOperation","message":"Non-integer exponents are not yet supported.","recoverable":true,"diagnostics":[{"message":"Non-integer exponents are not yet supported.","position":0,"line":1,"column":1}],"elapsed":"235.7 \u00B5s"}
```

Invariant 4 is unconditional: "Durations are structural: `elapsedTime` is `{value, unit}` next to the human `elapsed` string, so an agent never parses a unit suffix." On the error path the agent must parse "235.7 us". The doc contradicts itself here: its own error-envelope example (line 180) shows only `elapsed`. On the success path `elapsedTime` was present and consistent in all 152 success envelopes.

### F9 — P1 — print output is lost on error and relocated on cancellation

Doc invariant 1: "Anything the script prints with `print(...)` is captured and returned in the top-level `output` array."

Raw (run 1 `s42.ls`, run 2 `v06.ls` — `print("before"); 2^(1/2)`):
```json
{"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":false,"code":"UnsupportedOperation","category":"UnsupportedOperation","message":"Non-integer exponents are not yet supported.","recoverable":true,"diagnostics":[{"message":"Non-integer exponents are not yet supported.","position":0,"line":1,"column":1}],"elapsed":"245.7 \u00B5s"}
```

No `output` key exists — "before" is gone. On cancellation the same information moves to an undocumented key (`c02.ls`, `--cancel-after 1`):
```json
{"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":false,"code":"Cancelled","category":"BudgetExceeded","message":"the evaluation was cancelled by the caller.","recoverable":true,"diagnostics":[],"elapsed":"1.22 ms","partialOutput":[],"partialVariables":[{"name":"x","kind":"Symbolic","display":"x"}]}
```

So `output` is not where print output always lands; `partialOutput` / `partialVariables` appear nowhere in dsh-protocol.md.

### F10 — P1 — a Void statement produces a success envelope with no `result` key

Raw (run 1 `v16.ls` — `print("hi")`):
```json
{"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":true,"revision":80,"output":["hi"],"variables":[],"functions":[],"elapsed":"61 \u00B5s","elapsedTime":{"value":61,"unit":"\u00B5s"},"timings":[{"position":0,"elapsed":{"value":27.3,"unit":"\u00B5s"},"resultKind":"Void","hasOutput":true}]}
```

Run 2 (`p43.ls`, three prints) is the same shape with no `result` key. Census: 6 of 152 success envelopes. Invariant 3: "An absent field is `{"kind":"Null"}`, distinct from an empty string" — the key is not Null-shaped, it is absent, so a consumer with a fixed key set cannot distinguish "void" from a protocol change.

### F11 — P1 — `SystemSolveResult` carries `complete` but not `completeness`

Doc: "complete and completeness are read off ONE mapping ... so the two fields cannot disagree and the two records cannot drift apart", and "completeness is the authoritative field".

Raw (`v01.ls`, re-run `u04.ls` and `s14.ls`): the SystemSolveResult field list is exactly `status, domain, complete, solutions, diagnostics` — no `completeness`. Contrast `SolveResult` (`v10.ls`) which carries `complete` and `completeness`, and `MatrixSolveResult` (`g04.ls`) which also carries both. The derived convenience exists; the authoritative field it is derived from does not, for exactly one of the three solve records.

### F12 — P1 — `capabilities()` says `integer` is unsupported, then accepts and emits it

Raw (run 1 `t05.ls`, run 2 `v11.ls` — `symbol("x", integer)`):
```json
{"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":true,"revision":81,"result":{"kind":"Symbolic","display":"x","typed":"x (Symbolic)","structured":{"kind":"Symbolic","pretty":"x","canonical":"(sym x)","domain":"complex","exact":true,"nodeCount":1,"freeSymbols":["x"]}},"output":[],"variables":[{"name":"_","kind":"Symbolic","display":"x"}],"functions":[],"elapsed":"157 \u00B5s","elapsedTime":{"value":157,"unit":"\u00B5s"},"timings":[{"position":0,"elapsed":{"value":122.8,"unit":"\u00B5s"},"resultKind":"Symbolic","hasOutput":false}]}
```

`symbol("x", rational)` behaves the same (`t06.ls`). The advertised statement declares `unsupported_domains: [integer, rational]` and the advertised refusal `solve.unsupported-domain` fires with "solve(): currently supports domains real and complex; got integer." (`p07.ls`). Meanwhile the solver's own records hand back `integer` as a live Domain: `solve_full(sin(x)==0,x)` produces the family field `{"name":"parameter_domain","value":{"kind":"Domain","domain":"integer"}}` (run 1 `s13.ls`, run 2 `t39.ls`). Two defects in one: the self-description is contradicted, and the requested symbol domain is silently coerced to `complex` with no diagnostic.

### F13 — P1 — a refusal that `capabilities()` does not list

Raw (run 1 `t16.ls`, run 2 `v15.ls`): `x = symbol("x"); solve_full(x == x, x)`
```json
{"name":"status","value":{"kind":"Enum","type":"SolveStatus","value":"Unevaluated"}},{"name":"complete","value":{"kind":"Boolean","value":"false"}},{"name":"completeness","value":{"kind":"Enum","type":"Completeness","value":"Unknown"}}, ... "diagnostics": {"kind":"Array","type":"Vector","shape":[1], "elements":[{"kind":"Record","type":"Diagnostic","fields":[{"name":"code","value":{"kind":"Text","value":"solve.unevaluated"}},{"name":"category","value":{"kind":"Enum","type":"ErrorCategory","value":"UnsupportedOperation"}},{"name":"message","value":{"kind":"Text","value":"0 = 0: every value is a solution."}},{"name":"recoverable","value":{"kind":"Boolean","value":"true"}},{"name":"location","value":{"kind":"Null"}},{"name":"details","value":{"kind":"Array","type":"Vector","shape":[0],"elements":[]}}]}]}
```

All 16 advertised `operation_class` entries were executed from their own `trigger` strings; every one reproduced its advertised `code` and `category` exactly (16/16 HELD, section 3.1). But `solve.unevaluated` is a refused operation class (status Unevaluated plus an `UnsupportedOperation` diagnostic) with no entry in the statement. Same family, softer: `inv_full([[1,2],[3,4]])` (`t22.ls`) and `matrix_rank([[1,2],[2,4]])` (`t23.ls`, `s37.ls`) refuse a numeric matrix with "requires a symbolic matrix" while only `linsolve.non-symbolic-matrix` is advertised.

### F14 — P1 — a trivially inconsistent equation is refused instead of answered `NoSolutions`

Raw (run 1 `p20.ls` — `solve(0 == 1, x)`; run 2 `t18.ls` — `solve_full(2 == 3, x)`):
```json
{"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":false,"code":"InvalidOperation","category":"DomainError","message":"solve(): argument 1 must be a symbolic expression; got Boolean.","recoverable":true,"diagnostics":[{"message":"solve(): argument 1 must be a symbolic expression; got Boolean.","position":0,"line":1,"column":1}],"elapsed":"302.7 \u00B5s"}
```

Doc: "NoSolutions means the solution set over the requested domain is provably empty ... A provably empty set is a complete answer, so it reports `complete: true` / `completeness: Complete`". The equation folds to a Boolean at parse time and the single-equation solver rejects it. The system path does it correctly: `solve_system_full([x+y==1, x+y==2],[x,y])` (`t49.ls`) returns `NoSolutions, complete: true` — so the two paths disagree about the same mathematics.

### F15 — P1 — the registry that is supposed to define arity does not carry arity

Doc: "The counts come from the builtin's declared metadata (`Parameters`, `MinArity`, `Variadic`) ... a builtin that legitimately accepts a shorter form declares it." Raw registry rows (run 1 and run 2 of `fn.ls`, `--omit-functions` off):
```json
{"name":"solve","parameters":["f","x","domain"],"builtin":true,"plugin":"Lovelace.Symbolics"}
{"name":"symbol","parameters":["name","domain"],"builtin":true,"plugin":"Lovelace.Symbolics"}
{"name":"concat","parameters":["a","b","axis"],"builtin":true}
```

`MinArity`, `Variadic` and any optional marker are absent. The consequence is observable: `concat([1],[2])` (`p41.ls`) is accepted and returns `[1,2]` (axis defaulted) while `eye(2,2,2)` (`p40.ls`) is rejected, and an agent cannot tell from the envelope which of a builtin's declared parameters are optional. With `--omit-functions` (the agent-loop flag) it gets no registry at all.

### F16 — P1 — empty `Text` where invariant 3 requires `Null`

Invariant 3: "An absent field is `{"kind":"Null"}`, distinct from an empty string." Observed empties: `unrepresented_reason` on a fully solved record — `{"name":"unrepresented_reason","value":{"kind":"Text","value":""}}` (`s05.ls`, `v10.ls`, `t16.ls`, `t25.ls`, `c03.ls`); `budget_kind` on every TransformResult (`s01.ls`, `s12.ls`, `v18.ls`, `t08.ls`); `method` and `verification_method` on an unevaluated IntegrationResult (`p04.ls`, `t43.ls`). The doc contradicts itself: its own "real solve envelope" example (line 123) shows `{"name":"unrepresented_reason","value":{"kind":"Text","value":""}}`. As shipped, a consumer cannot distinguish "no reason" from "a reason whose text is empty".

### F17-F29 — P2 and P3, compact

* **F17 (P2)** `print("hi"); print(42); print([1,2])` (`p43.ls`) gives `"output":["hi\r","42\r","[1, 2]"]`; single `print("hi")` (`v16.ls`) gives `["hi"]`. Every element but the last keeps a CR, so `output[0] == "hi"` is false for the multi-print case and true for the single-print case. Reproduced twice.
* **F18 (P2)** `p05.ls`: `"category":"UnsupportedOperation"` (bare string) versus the same taxonomy inside a record (`t16.ls`): `"category":{"kind":"Enum","type":"ErrorCategory","value":"UnsupportedOperation"}`. The error `diagnostics[]` elements are also untyped `{message,position,line,column}` objects — the doc documents that carve-out, so the fault is the invariant-2 tension, not the shipped bytes. 57/57 error envelopes.
* **F19 (P2)** `--print-budget 1` on `x^2` (nodeCount 3, `f01.ls`) returns the full `(pow (sym x) (rat 2 1))` with no `truncated` / `truncationReason` / `budget`; the same holds for nodeCount 5 at budgets 4, 3, 2, 1 (`d04`, `d05`, `b01`, `b02`, `f03`, `f04`, `f05`, `f06`, `f07`). nodeCount 7 truncates at budgets 5-6 and nodeCount 10 at budgets 1-9, with `truncated:true, truncationReason:"node-budget", budget:n` (`b07`, `e10_*`, `e07_*`, `f08`). No information is lost (the full rendering is returned), hence P2 rather than P1.
* **F20 (P2)** `solve_full((x-1)^2 == 0, x)` (`k01.ls`) gives `solutions.shape=[2]` with both `multiplicity=1`; `solve_full(x^2 == 0, x)` (`m01.ls`) gives `shape=[1]`, `multiplicity=2`; `solve_full((x-1)^3==0,x)` (`m02.ls`) gives `shape=[3]`, `1,1,1` while the algebraically identical expanded `x^3-3x^2+3x-1==0` (`m03.ls`) gives `shape=[1]`, `multiplicity=3`. SymPy: `roots((x-1)**2) = {1: 2}`, `roots((x-1)**3) = {1: 3}`. The multiset is preserved, but the doc's own question "how many solutions? (`solutions.shape`)" gets 2 and 3 where the answer is one root of multiplicity 2 and 3.
* **F21 (P2)** `simplify_full(sqrt(x^2))` (run 1 `s12.ls`, run 2 `v18.ls`) gives `changed: true` with `original` and `expression` byte-identical (`(pow (pow (sym x) (rat 2 1)) (rat 1 2))`), `conditions: []`, `steps: []`. `simplify_full(x + 0)` (`t08.ls`) — the same identical-in/out shape — reports `changed: false`.
* **F22 (P2)** `det([[1,2],[3,4]])` = `Integer -2` (correct; `s36`, `g06`) but `matrix_rank([[1,2],[2,4]])` (`s37`, `t23`), `inv_full([[1,2],[3,4]])` (`t22`) and `linsolve([[1,2],[2,4]],[[1],[3]])` (`v20`, `p16`) all refuse with `InvalidOperation/DomainError` "requires a symbolic matrix"; only the linsolve one is advertised.
* **F23 (P2)** No args, `--nope`, `--print-budget 0`, `--print-budget -1`, `--cancel-after 0`: exit 2, empty stdout, human usage text on stderr, no envelope — invariant 6 ("Errors are structural: code, category, message, recoverable, diagnostics") holds only for script errors. `--help` exits 0 with no envelope, which is fine.
* **F24 (P2)** `variables[]` elements are `{name, kind, display}` only (`p01.ls`: `[{"name":"_","kind":"Vector","display":"[-2, 2]"},{"name":"x","kind":"Symbolic","display":"x"}]`): the same value that is fully structural in `result.structured` is display-only here. `MatrixSolveResult` (`g04.ls`) is emitted by the binary although the doc's record list names only seven records.
* **F25 (P2)** `evalf(sqrt(2), 50)` gives a 102-char Real (`t19`), `evalf(pi(), 5)` gives 102 chars (`t20`), while `evalf(1/3, 5)` gives 5 digits (`v12`), `evalf(1/3, 300)` gives 300 digits (`v13`) and `setprecision(20); evalf(1/3,20)` gives 20 digits (`v19`). The `digits` parameter is inert whenever the input is already numeric.
* **F26 (P3)** `limit.unevaluated` always carries the message "coefficient does not evaluate at the point" (`p03`, `s16`, `s17`, `k02`) — none of those limits has a coefficient. Message text is explicitly not the contract, so cosmetic.
* **F27 (P3)** In `f08.ls` (`--print-budget 1`, nodeCount 10) `canonical` is abbreviated but `pretty` is the full `(x + 1)*(x + 2)*(x + 3)`; the doc's "pretty/canonical carry a ...-terminated prefix" is true only of `canonical`.
* **F28 (P3)** `revision` is never defined in the doc; observed values 80-84 across scripts (3 prints -> 80, `concat` -> 81, `symbol+solve` -> 82, `symbol+solve_full` -> 83, three assignments -> 84) and deterministic for a given script (three runs of the same file -> 82, 82, 82). An agent cannot compare revisions across runs.
* **F29 (P3)** Undeclared names appear in the envelope: enum type `CapabilitiesExactness` (`capabilities()`, value `BestEffort`), value kinds `Natural` and `Real` (the doc lists `Integer` / `Complex`), array types `Vector` / `Matrix` (the doc's example says `"type":"Array"`), records `CapabilitiesResult`, `MatrixSolveResult`, `SolutionFamily`, `Binding`, `SystemSolution`, `ParameterInfo`, `RewriteStep`, `FiniteCondition`; and `CompilationResult.result_type` carries the value-kind name as free Text (`"Integer"`, `s04`, re-run `h02`) alongside `precision_policy` / `optimization_policy` / `policy` / `target` / `method` / `verification_method`, all Text spellings. No documented enum travelled as Text in this audit.

## 3. Probe ledger

The command for every row is the method's command form with `<probe>.ls` set to the script shown; the rows that used an option name it (`--print-budget n`, `--cancel-after ms`), and the census rows in 3.6 used `--eval "<name>()"`. Batch groupings in 3.2-3.7 are per temp directory, so ids repeat across batches only as deliberate re-runs.

Raw output is abridged to the decisive fields (long digitisations/arrays elided with "..."); nothing
that decides a verdict is elided. "V" = verdict, "F" = finding id from section 2.

### 3.1 capabilities() self-check — every advertised entry executed from its own `trigger`

All 16 entries of `unsupported_operations` were run from the advertised `trigger` string. Advertised vs live:

| operation_class | advertised code / category | live code / category | V |
|---|---|---|---|
| pow.non-integer-exponent (`2^(1/2)`) | UnsupportedOperation / UnsupportedOperation | UnsupportedOperation / UnsupportedOperation | HELD |
| pow.negative-base-unrepresentable-exponent (`(-8)^(1/3)`) | UnsupportedOperation / UnsupportedOperation | UnsupportedOperation / UnsupportedOperation | HELD |
| solve.unsupported-domain | InvalidOperation / DomainError | InvalidOperation / DomainError | HELD |
| rootof.complex-algebraic | solve.unrepresented-roots / UnsupportedOperation | solve.unrepresented-roots / UnsupportedOperation | HELD |
| limit.unevaluated-in-record-diagnostics | limit.unevaluated / UnsupportedOperation | limit.unevaluated / UnsupportedOperation | HELD |
| integration.unevaluated-in-record-diagnostics | integration.unevaluated / UnsupportedOperation | integration.unevaluated / UnsupportedOperation | HELD |
| plot.symbolic-expression / plot.symbolic-element | InvalidOperation / DomainError | InvalidOperation / DomainError | HELD |
| solve.non-symbolic-variable / solve.non-symbolic-expression | InvalidOperation / DomainError | InvalidOperation / DomainError | HELD |
| diff / integrate / limit .non-symbolic-variable | InvalidOperation / DomainError | InvalidOperation / DomainError | HELD |
| dsp.symbolic-element | InvalidOperation / DomainError | InvalidOperation / DomainError | HELD |
| linsolve.non-symbolic-matrix | InvalidOperation / DomainError | InvalidOperation / DomainError | HELD |
| fft.non-power-of-two-length | InvalidArgument / TypeMismatch | InvalidArgument / TypeMismatch | HELD |

16/16 triggers reproduce their advertised code and category. The statement is nonetheless wrong on
`unsupported_domains` (F12) and incomplete on refusals (F13). `supported_domains` was exercised too:
`solve(...,real)`, `solve(...,complex)` and symbol defaults all work (HELD).

### 3.2 Batch P (50 probes) — protocol surface, capability triggers, arity spot-checks, boundaries

| id | probe (script) | decisive raw fragment | V | F |
|---|---|---|---|---|
| p01 | `x=symbol("x"); solve(x^2-4==0,x)` | `{"kind":"Array","type":"Vector","shape":[2],"elements":[{...,"pretty":"-2","canonical":"(rat -2 1)","domain":"rational","exact":true},...{...,"pretty":"2"...}]}` — SymPy: {+-2} | HELD | |
| p02 | `solve_full(x^4-x^2-1==0,x)` | `status Partial, complete false, completeness Partial, represented_count 2, unrepresented_count 2`; `rootof(x^4-x^2-1,0)= -1.2720196495140689643`, index 1 = +1.2720196... (SymPy: two real roots +-1.27202, two complex unrepresented) | HELD | |
| p03 | `limit_full(sin(x),x,inf)` | `status Unevaluated, exists Null, value Null, exactness Exact` | FINDING | F3 |
| p04 | `integrate_full(exp(x^2),x)` | `status Unevaluated, diagnostic code integration.unevaluated, details:[integration.no-closed-form]` (SymPy: no elementary antiderivative) | HELD | F16 note |
| p05 | `2^(1/2)` | `ok:false, code UnsupportedOperation, category UnsupportedOperation` | HELD | |
| p06 | `(-8)^(1/3)` | same envelope as p05 | HELD | |
| p07 | `solve(x^2-2==0,x,integer)` | `InvalidOperation / DomainError, "got integer."` | HELD | |
| p08 | `plot(sin(x))` | `InvalidOperation / DomainError` as advertised | HELD | |
| p09 | `plot([x,1,2])` | `InvalidOperation / DomainError` as advertised | HELD | |
| p10 | `solve(x^2-2==0,1)` | `"argument 2 must be a symbolic variable; got Natural."` | HELD | |
| p11 | `solve([x==1],x)` | `"argument 1 must be a symbolic expression; got Vector."` | HELD | |
| p12 | `diff(x^2,1)` | `InvalidOperation / DomainError` | HELD | |
| p13 | `integrate(x^2,1)` | `InvalidOperation / DomainError` | HELD | |
| p14 | `limit(sin(x),1,0)` | `InvalidOperation / DomainError` | HELD | |
| p15 | `dft([x,1,2,3])` | `InvalidOperation / DomainError` | HELD | |
| p16 | `linsolve([[1,2],[2,4]],[[1],[3]])` | `InvalidOperation / DomainError` | HELD | |
| p17 | `fft([1,2,3])` | `InvalidArgument / TypeMismatch "FFT length must be a power of two"` | HELD | |
| p18 | `r=solve(x^2-4==0,x); type(r.status)` | `ok:false "member 'status' is not available on type 'Vector'"` | FINDING | F5 |
| p19 | `solve(x^2-2==0,x,real)` | `[-1/2*sqrt(8), 1/2*sqrt(8)]`, both `"exact":false` | FINDING | F4 |
| p20 | `solve(0==1,x)` | `InvalidOperation "got Boolean."` | FINDING | F14 |
| p21 | `solve((x-1)^2==0,x)` | `[1, 1]` | HELD | |
| p22 | `solve(x^2-2==0,x,real,complex)` | `InvalidArgument / TypeMismatch "expected 2 to 3 arguments; got 4."` | HELD | |
| p23 | `solve(x^2-4==0)` | `InvalidArgument / TypeMismatch "expected 2 to 3 arguments; got 1."` | HELD | |
| p24–p26 | `diff(x^2)`, `integrate(x^2)`, `limit(sin(x),x)` | `InvalidArgument / TypeMismatch, "expected 2/2/3 arguments; got 1/1/2."` | HELD | |
| p27–p30 | `dft()`, `dft([1,2,3,4],2,3)`, `symbol()`, `symbol("x",real,complex)` | `InvalidArgument / TypeMismatch "expected 1 to 2 arguments; got 0/3"`, `"expected 1 to 2 arguments; got 0/3"` | HELD | |
| p31 | `capabilities(1)` | `InvalidArgument / TypeMismatch "capabilities(): expected 0 arguments; got 1."` | HELD | |
| p32 | `fft()` | `"fft(): expected 1 argument; got 0."` | HELD | |
| p33 | `A=[[1,2],[2,4]]; b=[[1],[3]]; linsolve(A)` | `InvalidOperation / DomainError "linsolve() expects exactly 2 argument(s), but got 1."` | FINDING | F6 |
| p34 | `plot()` | `InvalidOperation / DomainError "plot() expects 1 to 3 arguments, but got 0."` | FINDING | F6 |
| p35 | `compile_full(x^2)` | `InvalidArgument / TypeMismatch "compile_full(): expected 2 arguments; got 1."` | HELD | |
| p36 | `exp()` | `InvalidArgument / TypeMismatch` | HELD | |
| p37–p39 | `det()`, `inv()`, `len()` | `InvalidOperation / DomainError "det() expects exactly 1 argument(s), but got 0."` etc. | FINDING | F6 |
| p40 | `eye(2,2,2)` | `InvalidOperation / DomainError "eye() expects 1 or 2 arguments, but got 3."` | FINDING | F6 |
| p41 | `concat([1],[2])` | `ok:true, {"kind":"Array","type":"Vector","shape":[2]...}` = `[1, 2]` (axis silently defaulted) | FINDING | F15 |
| p42 | `diff(x^2,"x")` | `2*x` — a Text argument accepted for a documented symbolic variable | INCONCLUSIVE | |
| p43 | `print("hi"); print(42); print([1,2])` | `"output":["hi\r","42\r","[1, 2]"]`; no `result` key | FINDING | F17, F10 |
| p44 | `x=symbol("x"); a=x^2; b=a+1` | `timings positions 0,17,26` = exact source offsets; `variables` entries carry only name/kind/display | HELD | F24 note |
| p45 | `2^100` | `{"kind":"Natural","value":"1267650600228229401496703205376"}` — SymPy `2**100` identical | HELD | |
| p46 | `0.1+0.2` | `{"kind":"Real","value":"0.3","exact":true,"numerator":"3","denominator":"10"}` — SymPy `Rational(1,10)+Rational(2,10)=3/10` | HELD | |
| p47 | same script as p01 | identical envelope; `revision 82` both times | HELD | |
| p48 | `fft([])` | `[]` (length 0 accepted although the advertised rule says power of two) | HELD | F19-note |
| p49 | `dft([])` | `[]` | HELD | |
| p50 | `dft([1,2,3,4],-1)` | `InvalidOperation "expects a non-negative sample count, but got -1."` | HELD | |

### 3.3 Batch S (50)

| id | probe | decisive fragment | V | F |
|---|---|---|---|---|
| s01 | `simplify_full(x/x)` | `status Satisfied, expression 1, conditions [x != 0], steps[rat.cancel-x-over-x / Conditional]`; `budget_kind ""` | FINDING | F16 |
| s02 | `simplify_full(exp(log(x)))` | `expression x, conditions [FiniteCondition(log(x))]` | HELD | |
| s03 | `optimize_full(x^2+2*x+1,[x])` | `estimated_cost_before 8, after 7, transformations ["cse","horner"]` | HELD | |
| s04 | `compile_full(x^2+1,[x])` | `result_type {"kind":"Text","value":"Integer"}, mathir_version Integer 2` | FINDING | F29 |
| s05 | `solve_full(x^2-4==0,x)` | `status Solved, complete true, completeness Complete, unrepresented_reason {"kind":"Text","value":""}` | FINDING | F16 |
| s06 | `type(r.status)` | `{"kind":"Text","value":"SolveStatus"}` | HELD | |
| s07 | `type(r.completeness)` | `{"kind":"Text","value":"Completeness"}` | HELD | |
| s08 | `r.complete` | `{"kind":"Boolean","value":"true"}` (consistent with completeness Complete) | HELD | |
| s09 | `solve_full(x^2+1==0,x,real)` | `status NoSolutions, complete true, completeness Complete, solutions shape [0], diagnostic solve.no-solutions/NoSolution` | HELD | |
| s10 | `solve(x^2+1==0,x,real)` | `result {"kind":"Text","value":"no real solutions"}` | FINDING | F5 |
| s11 | `solve_full(x^2+1==0,x,complex)` | `solutions [i, -i], exactness AlgebraicExact` | HELD | |
| s12 | `simplify_full(sqrt(x^2))` | identical original/expression, `changed true, steps []`, `budget_kind ""` | FINDING | F21, F16 |
| s13 | `solve_full(sin(x)==0,x)` | `status Solved, complete true, families [template k*pi, parameter_domain {"kind":"Domain","domain":"integer"}]` | FINDING | F12 |
| s14 | `solve_system_full([x+y==1,x-y==3],[x,y])` | `status Solved, complete true, bindings x=2,y=-1, exactness Exact`; fields = status, domain, complete, solutions, diagnostics | FINDING | F11 |
| s15 | `limit_full(1/x,x,0)` | `status DoesNotExist, exists false, left -inf, right inf, exactness Exact` | HELD | |
| s16 | `limit_full(abs(x)/x,x,0)` | `Unevaluated + exactness Exact + limit.unevaluated` (SymPy: two-sided limit does not exist) | FINDING | F3 |
| s17 | `limit_full(x*sin(1/x),x,0)` | `Unevaluated + exactness Exact` (SymPy: the limit is 0) | FINDING | F3 |
| s18 | `limit_full((1+1/x)^x,x,inf)` | `status Value, exists true, value 1` — SymPy: this limit is e (2.71828...); the kernel answers 1 | FINDING | F30 |
| s19 | `integrate_full(sin(x)/x,x)` | `Unevaluated, integration.unevaluated + details` (SymPy: Si(x), not elementary) | HELD | |
| s20 | `integrate_full(x^2,x)` | `status SolvedExact, exactness Exact` | HELD | |
| s21 | `solve(x^2-2==0,x,complex)` | same two roots as the real run, `exact:false` | FINDING | F4 |
| s22 | `sqrt(2)` | 100-digit Real, `exact:false` — matches SymPy N(sqrt(2),100) to 100 decimals | HELD | |
| s23/s24 | `-5`, `5-7` | `{"kind":"Integer","value":"-5"/"-2","exact":true}` | HELD | |
| s25 | `print("a")` | `"output":["a"]` | HELD | |
| s26 | `print("a"); print("b")` | `"output":["a\r","b"]` | FINDING | F17 |
| s27 | `print(1); 2` | timings: Void/hasOutput true then Natural/hasOutput false | HELD | |
| s28 | `evalf(pi(),100)` | 102-char Real = 100 decimals of pi, matches SymPy N(pi,101) | HELD | |
| s29 | `2^1000` | `Natural "10715086071862673209484250490600018105614048117055336074437503883703510511249361224931983788156958581275946729175531468251871452856923140435984577574698574803934567774824230985421074605062371141877954182153046474983581941267398767559165543946077062914571196477686542167660429831652624386837205668069376"` | HELD (SymPy agrees) | |
| s30/s31 | `1/0`, `0/0` | `DivisionByZero / DomainError "Cannot divide by zero."` | HELD | |
| s32 | `x^2-2==0` | `{"kind":"Boolean","value":"false"}` (relation evaluated) | HELD | |
| s33 | `subs(x^2,x,3)` | `9` | HELD | |
| s34 | `solve_full(x^5-x+1==0,x)` | `status Partial, complete false, completeness Partial, represented 1, unrepresented 4` | HELD | |
| s35/s36 | `sum([1,2,3])`, `det([[1,2],[3,4]])` | `6`, `Integer -2` (SymPy: 6, -2) | HELD | |
| s37 | `matrix_rank([[1,2],[2,4]])` | `InvalidOperation / DomainError "requires a symbolic matrix."` | FINDING | F22 |
| s38 | `linsolve_full(A,b)` (numeric A) | `InvalidOperation / DomainError` | FINDING | F22 |
| s39/s40/s41 | `fft([1])`, `fft([1,2,3,4])`, `dft([1,2,3,4],0)` | `[1]`, 4-element vector, `[]` | HELD | |
| s42 | `print("before"); 2^(1/2)` | error envelope, no `output` key | FINDING | F9 |
| s43 | `print("before"); diff(x^2)` | error envelope (arity), no `output` key | FINDING | F9 |
| s44/s45 | `inspect(capabilities())`, `type(capabilities())` | `CapabilitiesResult` / `"CapabilitiesResult"` | HELD | |
| s46 | `assume_positive(x); simplify_full(sqrt(x^2))` | returns x with conditions | HELD | |
| s47 | `solve(x^2-2==0,"x")` | accepted, returns the two roots | INCONCLUSIVE | |
| s48 | `diff(x^2,"y")` | accepted, returns `0` | INCONCLUSIVE | |
| s49 | `r.unrepresented_count` | `Integer 2` (record member access works on solve_full results) | HELD | |
| s50 | `evalf(r.solutions[0].value,20)` | `-1.2720196495140689642524` (SymPy real root -1.2720196495140689643) | HELD | |

**Note on s18** (`limit_full((1+1/x)^x, x, inf)` → `value 1`): the mathematically correct limit is
`e = 2.718281828459045`. I did **not** classify this as a finding because the probe was run once and the
binary was published mid-session; the value deserves an immediate re-run before anyone acts on it. It is
listed in section 4 as COULD NOT DECIDE with the missing evidence named.

### 3.4 Batch T (48)

| id | probe | decisive fragment | V | F |
|---|---|---|---|---|
| t01 | `solve_full(x^2-2==0,x,real)` | `"exact":false` beside `SolutionExactness/AlgebraicExact` | FINDING | F4 |
| t02 | `pi()` | 100-decimal Real, `exact:false`, matches SymPy | HELD | |
| t03 | `e()` | 100-decimal Real, matches SymPy | HELD | |
| t04 | `sin(pi())` | `Symbolic 0, canonical (real 0), exact:false` | HELD | |
| t05/t06 | `symbol("x",integer)`, `symbol("x",rational)` | `ok:true, domain "complex"` | FINDING | F12 |
| t08 | `simplify_full(x+0)` | identical in/out, `changed false` | HELD | |
| t09 | `series(exp(x),x,0,5)` | `1 + x + 1/2*x^2 + 1/6*x^3 + 1/24*x^4 + O(x^5)` (SymPy coefficients agree) | HELD | |
| t10 | `powerseries(exp(x),0,5)` | domain error demanding a sequence — the probe did not exercise the builtin's contract | INCONCLUSIVE | |
| t11 | `compile(x^2+1,[x])` | `{"kind":"Text","value":"#!mathir 2\nparam x..."}` | HELD | |
| t12 | `optimize(x^2+2*x+1,[x])` | `1 + x*(x + 2)` | HELD | |
| t13 | `integrate_full(sin(x),x)` | `SolvedExact + exactness Approximate` | FINDING | F2 |
| t14/t15 | `limit_full(x^2,x,2)`, `limit_full(sin(x)/x,x,0)` | `Value 4`, `Value 1` (SymPy agrees) | HELD | |
| t16 | `solve_full(x==x,x)` | `Unevaluated + solve.unevaluated/UnsupportedOperation` | FINDING | F13 |
| t17 | `solve(x==x,x)` | `{"kind":"Text","value":"unevaluated: 0 = 0: every value is a solution."}` | FINDING | F5 |
| t18 | `solve_full(2==3,x)` | `DomainError "got Boolean."` | FINDING | F14 |
| t19/t20 | `evalf(sqrt(2),50)`, `evalf(pi(),5)` | 102-char Reals (100 digits) | FINDING | F25 |
| t21 | `det([[x,1],[1,x]])` | `x^2 - 1` | HELD | |
| t22/t23 | `inv_full([[1,2],[3,4]])`, `matrix_rank(...)` | `"requires a symbolic matrix."` | FINDING | F22 |
| t24 | F1 system (second shape of the same defect class) | `NoSolutions, complete true` | FINDING | F1, F11 |
| t25 | `solve_full(x^3-2==0,x)` | three exact roots `2^(1/3)*(-i*sqrt(3)/2 - 1/2)`, `2^(1/3)*(i*sqrt(3)/2 - 1/2)`, `2^(1/3)` (SymPy identical) | HELD | |
| t26 | `abs(x)` | Symbolic abs | HELD | |
| t27–t33 | `abs(1,2)`, `det([1],2)`, `len([1],2)`, `linsolve(A,b,1)`, `type(1,2)`, `sqrt(4,1)`, `max([1],0,9)` | all `InvalidOperation / DomainError` | FINDING | F6 |
| t34–t36 | `zeros(2,3)`, `ones(2)`, `reshape([1,2,3,4],2,2)` | correct arrays | HELD | |
| t37/t38 | `print()`, `print("x","y")` | ok:true with no `result` key / `"output":["x y"]` | FINDING | F10 |
| t39 | `solve_full(sin(x)==0,x,real)` | family `parameter_domain Domain integer` | FINDING | F12 |
| t41 | `limit_full(exp(-1/x^2),x,0)` | `Unevaluated + exactness Exact` (SymPy: 0) | FINDING | F3 |
| t42 | `integrate_full(1/x,x)` | `SolvedConditional, log(x), conditions [x > 0], verified true, exactness Approximate` | FINDING | F2 |
| t43 | `integrate_full(exp(-x^2),x)` | `Unevaluated + details`, `method ""` | HELD | F16 note |
| t44/t45 | `factor(x^2-1)`, `expand((x+1)^3)` | `(x - 1)*(x + 1)`, `1 + 3*x + 3*x^2 + x^3` | HELD | |
| t46 | `evalir(compile(x^2+1,[x]),[3],20)` | `Integer 10` = 3^2+1 | HELD | |
| t47/t48 | `hessian(x^2*y,[x,y])`, `jacobian([x^2,x^3],[x])` | correct matrix shapes | HELD | |
| t49 | `solve_system_full([x+y==1,x+y==2],[x,y])` | `NoSolutions, complete true` (correct, and it contrasts with F14) | HELD | |
| t50 | `solve_full(x^2-4==0,x,"real")` | `ok:false` domain error for a Text domain | HELD | |

### 3.5 Batches U and V (27) — P0 re-runs and error-envelope probes

| id | probe | decisive fragment | V | F |
|---|---|---|---|---|
| u01 | F1 system (clean re-run) | `NoSolutions, complete true, solutions [], diagnostics []` | FINDING | F1 |
| u02 | `solve_system(...)` same system | `{"kind":"Text","value":"no solutions"}` — wrong answer as prose | FINDING | F1, F5 |
| u03 | `solve_full(x^2+x-1==0,x)` | two correct roots `1/2*(-sqrt(5) - 1)/…` (SymPy agrees) | HELD | |
| u04 | linear system re-run | `Solved, x=2, y=-1, complete true` | FINDING | F11 |
| u05 | `solve_system_full([x*y==1,x-y==0],[x,y])` | `Solved`, (−1,−1) and (1,1) — correct | HELD | |
| u06 | F1 system, variables reversed | `NoSolutions` | FINDING | F1 |
| u07 | F1 system, equations reversed | `NoSolutions` | FINDING | F1 |
| v01 | F1 system, third clean file | `NoSolutions, complete true` | FINDING | F1 |
| v02 | `[x^2==1, y-x==0]` | `Solved, 2 solutions` | HELD | |
| v03 | `[y==x^2, y==1]` | `Solved, 2 solutions` | HELD | |
| v04 | `[x^2+y^2==1, y==x]` | `NoSolutions, complete true` (SymPy: +-sqrt(2)/2) | FINDING | F1 |
| v05 | `[x^2-2==0, y==x]` | `Solved, 2 solutions` | HELD | |
| v06 | `print("before"); 2^(1/2)` | error envelope without `elapsedTime` or `output` | FINDING | F9, F8 |
| v07 | `solve(x^2+1==0,x,real)` | `result Text "no real solutions"` | FINDING | F5 |
| v08 | `integrate_full(sin(x),x)` | `SolvedExact + Approximate` | FINDING | F2 |
| v09 | `limit_full(sin(x),x,inf)` | `Unevaluated + Exact` | FINDING | F3 |
| v10 | `solve_full(x^2-2==0,x,real)` | `exact:false` + `AlgebraicExact` | FINDING | F4 |
| v11 | `symbol("x",integer)` | accepted, domain complex | FINDING | F12 |
| v12/v13 | `evalf(1/3,5)`, `evalf(1/3,300)` | 5-digit / 300-digit Reals — digits honoured for exact input | HELD | |
| v14 | `transpose()` | CLR index exception as the message | FINDING | F7 |
| v15 | `solve_full(x==x,x)` | `Unevaluated + solve.unevaluated` | FINDING | F13 |
| v16 | `print("hi")` | no `result` key; no CR on the single output element | FINDING | F10, F17 |
| v17 | `evalf(rootof(...,0),20)` | `-1.2720196495140689642524` | HELD | |
| v18 | `simplify_full(sqrt(x^2))` | `changed true, steps []` | FINDING | F21 |
| v19 | `setprecision(20); evalf(1/3,20)` | 20-digit Real | HELD | |
| v20 | `linsolve([[1,2],[2,4]],[[1],[3]])` | DomainError (advertised class) | FINDING | F22 |

### 3.6 Builtin arity census (123 probes, each executed twice)

Zero-argument call of every builtin in the registry. `V` column: HELD = `InvalidArgument/TypeMismatch`
(the documented shape), FINDING = anything else.

| builtin | code | category | V | builtin | code | category | V |
|---|---|---|---|---|---|---|---|
| abs | InvalidOperation | DomainError | FINDING | acos | InvalidArgument | TypeMismatch | HELD |
| acosh | InvalidArgument | TypeMismatch | HELD | and | InvalidArgument | TypeMismatch | HELD |
| apart | InvalidArgument | TypeMismatch | HELD | append | InvalidOperation | DomainError | FINDING |
| asin | InvalidArgument | TypeMismatch | HELD | asinh | InvalidArgument | TypeMismatch | HELD |
| assume | InvalidArgument | TypeMismatch | HELD | assume_clear | ok:true | Text | HELD |
| assume_integer | InvalidArgument | TypeMismatch | HELD | assume_negative | InvalidArgument | TypeMismatch | HELD |
| assume_nonnegative | InvalidArgument | TypeMismatch | HELD | assume_positive | InvalidArgument | TypeMismatch | HELD |
| assume_real | InvalidArgument | TypeMismatch | HELD | assumptions | ok:true | Text | HELD |
| atan | InvalidArgument | TypeMismatch | HELD | atanh | InvalidArgument | TypeMismatch | HELD |
| cancel | InvalidArgument | TypeMismatch | HELD | cancel_full | InvalidArgument | TypeMismatch | HELD |
| capabilities | ok:true | Record | HELD | collect | InvalidArgument | TypeMismatch | HELD |
| compile | InvalidArgument | TypeMismatch | HELD | compile_full | InvalidArgument | TypeMismatch | HELD |
| complex | ok:true | Domain | HELD | concat | InvalidOperation | DomainError | FINDING |
| conj | InvalidArgument | TypeMismatch | HELD | conv | InvalidArgument | TypeMismatch | HELD |
| cos | InvalidArgument | TypeMismatch | HELD | cosh | InvalidArgument | TypeMismatch | HELD |
| cosine | InvalidArgument | TypeMismatch | HELD | cross | InvalidOperation | DomainError | FINDING |
| delay | InvalidArgument | TypeMismatch | HELD | det | InvalidOperation | DomainError | FINDING |
| dft | InvalidArgument | TypeMismatch | HELD | diff | InvalidArgument | TypeMismatch | HELD |
| divrem | InvalidOperation | DomainError | FINDING | dot | InvalidOperation | DomainError | FINDING |
| e | ok:true | Real | HELD | evalf | InvalidArgument | TypeMismatch | HELD |
| evalir | InvalidArgument | TypeMismatch | HELD | evalir_batch | InvalidArgument | TypeMismatch | HELD |
| exp | InvalidArgument | TypeMismatch | HELD | expand | InvalidArgument | TypeMismatch | HELD |
| exponential | InvalidArgument | TypeMismatch | HELD | eye | InvalidOperation | DomainError | FINDING |
| factor | InvalidArgument | TypeMismatch | HELD | fft | InvalidArgument | TypeMismatch | HELD |
| filter | InvalidArgument | TypeMismatch | HELD | flatten | InvalidOperation | DomainError | FINDING |
| hessian | InvalidArgument | TypeMismatch | HELD | im | InvalidArgument | TypeMismatch | HELD |
| impulse | InvalidArgument | TypeMismatch | HELD | inf | ok:true | Symbolic | HELD |
| inspect | InvalidOperation | DomainError | FINDING | integer | ok:true | Domain | HELD |
| integrate | InvalidArgument | TypeMismatch | HELD | integrate_full | InvalidArgument | TypeMismatch | HELD |
| inv | InvalidOperation | DomainError | FINDING | inv_full | InvalidOperation | DomainError | FINDING |
| is_even | InvalidOperation | DomainError | FINDING | is_odd | InvalidOperation | DomainError | FINDING |
| jacobian | InvalidArgument | TypeMismatch | HELD | latex | InvalidArgument | TypeMismatch | HELD |
| len | InvalidOperation | DomainError | FINDING | limit | InvalidArgument | TypeMismatch | HELD |
| limit_full | InvalidArgument | TypeMismatch | HELD | limit_left | InvalidArgument | TypeMismatch | HELD |
| limit_right | InvalidArgument | TypeMismatch | HELD | linsolve | InvalidOperation | DomainError | FINDING |
| linsolve_full | InvalidOperation | DomainError | FINDING | log | InvalidArgument | TypeMismatch | HELD |
| lower | InvalidArgument | TypeMismatch | HELD | matmul | InvalidOperation | DomainError | FINDING |
| matrix_rank | InvalidOperation | DomainError | FINDING | max | InvalidOperation | DomainError | FINDING |
| mean | InvalidOperation | DomainError | FINDING | min | InvalidOperation | DomainError | FINDING |
| movingavg | InvalidArgument | TypeMismatch | HELD | ndims | InvalidOperation | DomainError | FINDING |
| noise | InvalidArgument | TypeMismatch | HELD | norm | InvalidOperation | DomainError | FINDING |
| not | InvalidArgument | TypeMismatch | HELD | numel | InvalidOperation | DomainError | FINDING |
| ones | InvalidOperation | DomainError | FINDING | optimize | InvalidArgument | TypeMismatch | HELD |
| optimize_full | InvalidArgument | TypeMismatch | HELD | or | InvalidArgument | TypeMismatch | HELD |
| pi | ok:true | Real | HELD | plot | InvalidOperation | DomainError | FINDING |
| powerseries | InvalidArgument | TypeMismatch | HELD | print | ok:true | (void) | HELD |
| prod | InvalidOperation | DomainError | FINDING | rank | InvalidOperation | DomainError | FINDING |
| rational | ok:true | Domain | HELD | re | InvalidArgument | TypeMismatch | HELD |
| real | ok:true | Domain | HELD | reshape | InvalidOperation | DomainError | FINDING |
| scale | InvalidArgument | TypeMismatch | HELD | series | InvalidArgument | TypeMismatch | HELD |
| setprecision | InvalidOperation | DomainError | FINDING | shape | InvalidOperation | DomainError | FINDING |
| sign | InvalidOperation | DomainError | FINDING | simplify | InvalidArgument | TypeMismatch | HELD |
| simplify_full | InvalidArgument | TypeMismatch | HELD | sin | InvalidArgument | TypeMismatch | HELD |
| sinh | InvalidArgument | TypeMismatch | HELD | solve | InvalidArgument | TypeMismatch | HELD |
| solve_full | InvalidArgument | TypeMismatch | HELD | solve_system | InvalidArgument | TypeMismatch | HELD |
| solve_system_full | InvalidArgument | TypeMismatch | HELD | sqrt | InvalidOperation | DomainError | FINDING |
| squeeze | InvalidOperation | DomainError | FINDING | step | InvalidArgument | TypeMismatch | HELD |
| subs | InvalidArgument | TypeMismatch | HELD | sum | InvalidOperation | DomainError | FINDING |
| symbol | InvalidArgument | TypeMismatch | HELD | tan | InvalidArgument | TypeMismatch | HELD |
| tanh | InvalidArgument | TypeMismatch | HELD | trace | InvalidOperation | DomainError | FINDING |
| transpose | InvalidArgument | TypeMismatch | FINDING (CLR exception text) | type | InvalidOperation | DomainError | FINDING |
| zeros | InvalidOperation | DomainError | FINDING | | | | |

Totals: 83 HELD, 39 FINDING (F6), 1 FINDING (F7).

### 3.7 Batches B/C/D/E/F/G/H/K/M/N (66) — CLI options, boundaries, re-runs

| id | probe | decisive fragment | V | F |
|---|---|---|---|---|
| b01/b02 | `x^2+1` with `--print-budget 3` / `1` | full canonical, no `truncated`/`budget` | FINDING | F19 |
| b03/b04 | `--print-budget 0` / `-1` | exit 2, no envelope, "requires a positive node count" | FINDING | F23 |
| b05/b06 | `--print-budget 5` / no flag | full canonical (5 is not beyond nodeCount 5) | HELD | |
| b07 | `(x+1)(x+2)(x+3)` `--print-budget 2` | `"truncated":true,"truncationReason":"node-budget","budget":2` | HELD | |
| b08 | same, `--print-budget 100000` | full canonical, no truncation fields | HELD | |
| c01/c04 | `--cancel-after 0` | exit 2, no envelope | FINDING | F23 |
| c02 | `--cancel-after 1` | `code Cancelled, category BudgetExceeded, extra keys partialOutput/partialVariables` | FINDING | F9, F23-adjacent |
| c03 | `--cancel-after 100000` | normal Partial SolveResult (decic, 2 represented / 8 unrepresented, RootOf real roots) | HELD | |
| d01–d03 | 10-node value, budgets 9/8/5 | truncated + budget reported at every budget < 10 | HELD | |
| d04/d05 | 5-node value, budgets 4/2 | no truncation fields at all | FINDING | F19 |
| d06 | 1-node value, budget 1 | no truncation (correct) | HELD | |
| d07 | 13-node value, budget 6 | truncated | HELD | |
| e10_1..e10_7, e07_5..e07_8, e07_none | threshold scan | truncation iff budget < nodeCount for nodeCount 7 and 10 | HELD | |
| f01–f07 | nodeCount 2/3/5 values at budgets 1–3 | full renderings, no truncation fields | FINDING | F19 |
| f08 | 10-node value, budget 1 | `canonical` truncated, `pretty` full | FINDING | F27 |
| g01/g02 | `4^(1/2)`, `8^(1/3)` | refused ("Non-integer exponents are not yet supported") although the answers are 2 | HELD (advertised class) | F26 note |
| g03 | `sqrt(2)` | 100-digit Real matching SymPy | HELD | |
| g04 | `linsolve_full([[x,1],[1,x]],[[1],[2]])` | `MatrixSolveResult Solved/true/Complete, a=(x-2)/(x^2-1), b=(2x-1)/(x^2-1), condition x^2-1 != 0` — Cramer cross-check agrees | HELD | F24 note |
| g05 | `evalf(rootof(...,1),20)` | `+1.2720196495140689642524` | HELD | |
| g06 | `det([[1,2],[3,4]])` | `-2` | HELD | |
| g07 | `0^0` | `Natural 1` (SymPy convention) | HELD | |
| g08 | `1/0` | DivisionByZero | HELD | |
| h01 | `[x^2+y^2==1, y==x]` re-run | `NoSolutions, complete true` | FINDING | F1 |
| h02 | `compile_full(x^2+1,[x])` re-run | `result_type = Text:Integer` | FINDING | F29 |
| k01 | `solve_full((x-1)^2==0,x)` | `shape [2], multiplicity 1,1` (SymPy roots: {1: 2}) | FINDING | F20 |
| k02 | `limit_full(exp(-1/x^2),x,0)` | `Unevaluated + Exact` | FINDING | F3 |
| k03 | `integrate_full(1/x,x)` | `SolvedConditional, log(x), exactness Approximate` | FINDING | F2 |
| k04/k05/k06 | `-5`, `5-7`, `print("x","y")` | `Integer -5`, `Integer -2`, `"output":["x y"]` | HELD | |
| m01 | `solve_full(x^2==0,x)` | `shape [1], multiplicity 2` | HELD | |
| m02 | `solve_full((x-1)^3==0,x)` | `shape [3], multiplicity 1,1,1` | FINDING | F20 |
| m03 | `solve_full(x^3-3x^2+3x-1==0,x)` | `shape [1], multiplicity 3` | HELD | |
| n01 | `limit_full((1+1/x)^x,x,inf)` | `status Value, exists true, value 1, exactness Exact` | FINDING | F30 |
| n02 | `limit_full((1+1/n)^n,n,inf)` | same wrong value 1 (SymPy: E) | FINDING | F30 |
| n03 | n01 re-run, clean file | `status Value, exists true, value 1` | FINDING | F30 |
| n04 | `limit_full((1+2/x)^x,x,inf)` | value 1 (SymPy: exp(2)) | FINDING | F30 |
| n05 | `limit_full(exp(x*log(1+1/x)),x,inf)` | `Unevaluated + exactness Exact` | FINDING | F3 |

### 3.8 CLI and registry probes (7)

| id | command | decisive output | V | F |
|---|---|---|---|---|
| cli-1 | `Lovelace.Run.exe` (no args) | exit 2, stderr "No script provided...", stdout empty | FINDING | F23 |
| cli-2 | `--nope` | exit 2, stderr "Unknown argument '--nope'.", stdout empty | FINDING | F23 |
| cli-3 | `--file missing.ls --omit-functions` | exit 1, `ok:false, code FileReadError, category ParseError, elapsed "0 ms"` (no `elapsedTime`; a file-read failure categorised as a parse error) | FINDING | F8 |
| cli-4 | `--help` | exit 0, human usage text (no envelope) | HELD | |
| cap-1 | `print(capabilities())` | display string in `output`, envelope otherwise normal | HELD | |
| cap-2 | `capabilities()` | 16 unsupported operations; `unsupported_domains [integer, rational]`; `exactness Enum CapabilitiesExactness/BestEffort` | FINDING | F12, F29 |
| reg-1 | registry dump (twice) | 123 builtins, `{name, parameters, builtin, plugin}` only | FINDING | F15 |

## 4. COULD NOT DECIDE

1. ~~s18 — exponential limit~~ **RESOLVED during this audit: promoted to finding F30** (reproduced in four clean files; see section 2). The correct limit is `e = 2.718281828459045`. The mandated re-run was performed and the wrong value 1 reproduced (n01, n03), together with two further shapes (n02 with a different symbol; n04 `(1+2/x)^x`, whose true limit is e^2 and which is also answered 1). This was the only item on this list that closed during the audit; the remaining items stand.
2. **The truncation accounting.** `--print-budget n` truncates a nodeCount-7 value at budgets 5-6 and a nodeCount-10 value at budgets 1-9, but never truncates nodeCount 2/3/5 values at budgets 1-4. I could not decide whether the kernel compares a *different* node measure than the `nodeCount` it publishes, because dsh-protocol.md never defines the measure and the published binary carries no symbols to read. **Missing evidence:** a documented definition of the budgeted unit.
3. **`revision`.** The doc never defines it. It is deterministic for a fixed script (three runs of the same file → 82, 82, 82) but not monotone across scripts (80–84). I cannot decide whether that is intended semantics or a defect.
4. **Text arguments where a symbolic variable is documented** (`diff(x^2,"x")`, `diff(x^2,"y")`, `solve(x^2-2==0,"x")`): all accepted and answered sensibly. The doc does not say whether a name-carrying Text is a legal stand-in for a symbol, so no verdict.
5. **`powerseries`** (`t10`): the builtin takes a coefficient sequence, so my probe (a function) only exercised the argument-type check. Its documented contract was not tested.
6. **Trustworthiness of the `elapsed` numbers themselves** (they are self-reported by the process and I have no independent clock inside it); only their *structural form* was audited.
7. **String-level validity of `canonical`** beyond prefix-preservation under truncation: only `nodeCount`/`freeSymbols`/`domain`/`exact` were cross-checked against SymPy, not a general canonical-form round trip.

## 5. Summary

Of **367 distinct probe inputs** (~510 executions including the mandated second runs), **222 HELD**
(the product matched the contract or the mathematics), **141 produced evidence of a defect**, and
**4 were INCONCLUSIVE**; the 141 collapse into **30 findings: 5 x P0, 12 x P1, 8 x P2, 5 x P3**. The
protocol is honest about *mechanism* — the version triple is on every envelope, enums are typed
everywhere a documented enum is involved, the six-field Diagnostic is exact and in order, timings
positions are true source offsets, the error taxonomy is used as published, and all 16 advertised
capability triggers reproduce their advertised code and category byte-for-byte — but it is not honest
about *state*: the two places where an agent asks "did you actually answer?" (the completeness pair and
the exactness field) are the two places that lie. One finding is worse than a misleading field: `limit_full((1 + 1/x)^x, x, inf)` returns `1` with `exists: true` where the answer is `e` (F30). `solve_system_full` returns a confident
"no solutions, complete" for a system with two solutions (F1); an exact verified integral is labelled
`Approximate` while an identical-status sibling is labelled `Exact` (F2); an unevaluated limit is labelled
`Exact` (F3); a symbolic value inside an `AlgebraicExact` solution is labelled `exact:false` (F4);
and the one time the solver cannot represent an answer at all it puts a human sentence in the result
slot (F5). Around those, the envelope has a second, structural failure mode: the error path drops the
structural duration and the captured print output (F8, F9), the void path drops `result` entirely (F10),
the system record drops the authoritative completeness field (F11), the arity contract is implemented
twice with different codes and messages and one CLR exception escapes (F6, F7), and the self-description
contradicts the solver's own records about the `integer` domain while omitting a refusal it performs
(F12, F13). Everything above is reproducible from the command lines in section 3.



