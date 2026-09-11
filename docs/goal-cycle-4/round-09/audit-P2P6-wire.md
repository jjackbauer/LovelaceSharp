# Audit P2+P6 — wire contract & capability honesty

Target: `C:\Users\ricar\dev\LovelaceSharp\out\aot\Lovelace.Run.exe`
(5,673,984 bytes, LastWriteTime 9/11/2026 12:41:36 PM — Native AOT, freshly built)
Contract under test: `docs/symbolics/dsh-protocol.md` (cited as `dsh-protocol.md:<line>`)
Auditor: adversarial falsifier, part 6 (capability honesty) first, then part 2 (envelope).

## Conventions used in every command below

* Working directory: `C:\Users\ricar\dev\LovelaceSharp` (repo root).
* All scripts were written to `%TEMP%\p2p6\<name>.ls` with `[IO.File]::WriteAllText` — verbatim, **no trailing newline**.
* Command form shown as `.\out\aot\Lovelace.Run.exe --file %T%\<name>.ls <flags>`; where `<flags>` is
  `--omit-functions --omit-variables` unless the row says otherwise. `%T%` = `C:\Users\ricar\AppData\Local\Temp\p2p6`.
* Exit codes were read from `$LASTEXITCODE`; stdout/stderr were separated with
  `Start-Process -RedirectStandardOutput/-RedirectStandardError` where byte counts matter.
* "Advertised entry N" = element N of `capabilities().result.structured.fields[unsupported_operations].elements`, captured with
  `.\out\aot\Lovelace.Run.exe --file %T%\caps.ls --omit-functions` where `caps.ls` = `capabilities();`.
* Nothing outside this deliverable was modified or created in the repo; the one probe artifact that
  `plot([1,2,3])` wrote to the repo root (`plot.svg`) was deleted immediately and `git status` confirms the tree is clean apart from pre-existing untracked paths.

### The advertised statement (verbatim, decoded from the envelope)

`capabilities()` returned `ok:true`, `result.structured.type = "CapabilitiesResult"` with fields
`supported_domains=[real,complex]`, `unsupported_domains=[integer,rational]`,
`exactness=Enum(CapabilitiesExactness,"BestEffort")`, and `unsupported_operations` =
(`shape:[4]`):

| # | operation_class | code | category | message | trigger |
|---|---|---|---|---|---|
| 0 | pow.non-integer-exponent | UnsupportedOperation | UnsupportedOperation | Non-integer exponents are not yet supported. | `2^(1/2)` |
| 1 | pow.negative-base-unrepresentable-exponent | UnsupportedOperation | UnsupportedOperation | Non-integer exponents are not yet supported. | `(-8)^(1/3)` |
| 2 | solve.unsupported-domain | InvalidOperation | DomainError | solve(): currently supports domains real and complex; got integer. | `x = symbol("x"); solve(x^2 - 2 == 0, x, integer)` |
| 3 | rootof.complex-algebraic | solve.unrepresented-roots | UnsupportedOperation | complex algebraic roots not supported (RootOf is real-only in v1). | `x = symbol("x"); solve_full(x^4 - x^2 - 1 == 0, x)` |

## Part 6(a) — every advertised entry tripped live

| probe | doc claim (file:line) | command | observed | verdict |
|---|---|---|---|---|
| P6a-0 advertised entry 0 | dsh-protocol.md:183-185 (category from the one `ErrorCategory` taxonomy); advertisement itself | `.\out\aot\Lovelace.Run.exe --file %T%\adv0.ls --omit-functions` with `2^(1/2);` | exit 1; `{"ok":false,"code":"UnsupportedOperation","category":"UnsupportedOperation","message":"Non-integer exponents are not yet supported.","recoverable":true,...}` — `code`/`category`/`message` identical to advertised entry 0 | HELD |
| P6a-1 advertised entry 1 | advertisement itself | `... --file %T%\adv1.ls` with `(-8)^(1/3);` | exit 1; `code=UnsupportedOperation`, `category=UnsupportedOperation`, `message="Non-integer exponents are not yet supported."` — identical to advertised entry 1 | HELD |
| P6a-2 advertised entry 2 | advertisement itself | `... --file %T%\adv2.ls` with `x = symbol("x"); solve(x^2 - 2 == 0, x, integer);` | exit 1; `code=InvalidOperation`, `category=DomainError`, `message="solve(): currently supports domains real and complex; got integer."` — identical to advertised entry 2 | HELD |
| P6a-3 advertised entry 3 | advertisement itself; dsh-protocol.md:61-78, 80-91 (Diagnostic form) | `... --file %T%\adv3.ls` with `x = symbol("x"); solve_full(x^4 - x^2 - 1 == 0, x);` | exit **0**, `ok:true`, `SolveResult` carrying `Diagnostic(code:"solve.unrepresented-roots", category:Enum(ErrorCategory,"UnsupportedOperation"), message:"complex algebraic roots not supported (RootOf is real-only in v1).")` — all three fields identical to advertised entry 3 | HELD (note: this class surfaces as a result-record diagnostic, not an error envelope; the advertisement does not claim which) |
| P6a-4 advertised `unsupported_domains` second member | advertisement `unsupported_domains:[integer,rational]` | `... --file %T%\b_dom1.ls` with `x = symbol("x"); solve(x^2 - 2 == 0, x, rational);` | exit 1; `code=InvalidOperation`, `category=DomainError`, `message="solve(): currently supports domains real and complex; got rational."` — class `solve.unsupported-domain` covers it | HELD |
| P6a-5 advertised class re-trip via the `Domain` value form | dsh-protocol.md:36 `{"kind":"Domain","domain":"real"}` | `... --file %T%\b_dom_solve3.ls` with `x = symbol("x"); solve(x^2 - 2 == 0, x, integer());` | exit 1; identical to P6a-2 (`integer()` is a `Domain` value, same message) | HELD |

## Part 6(b) — operations the kernel REFUSES that the statement does NOT list

All nine rows below are refusals (kernel declines or crashes on the operation) for which
`capabilities().unsupported_operations` (4 entries above) has **no entry**. The first four are the
falsification of exhaustiveness.

| probe | doc claim (file:line) | command | observed | verdict |
|---|---|---|---|---|
| P6b-1 symbolic limit that cannot be determined | advertisement claims `unsupported_operations` is the exhaustive list of refused operations; dsh-protocol.md:80-81 names `limit.unevaluated` as a stable code and dsh-protocol.md:183-185 gives the category taxonomy | `... --file %T%\b_lim6.ls` with `x = symbol("x"); limit_full(sin(x), x, inf);` | exit 0, `ok:true`; `LimitResult.status = Enum(LimitStatus,"Unevaluated")` and `diagnostics.elements[0] = Diagnostic(code:"limit.unevaluated", category:Enum(ErrorCategory,"UnsupportedOperation"), message:"...")`. The kernel itself files this refusal under category **UnsupportedOperation**, the very category the statement enumerates — and the statement lists no `limit.*` entry. Same for `limit_full(exp(x),x,inf)` and `limit_full(sin(1/x),x,0)` | **FINDING** |
| P6b-2 unsupported integration | same exhaustiveness claim; dsh-protocol.md:61-62 (`IntegrationResult` is a rich result record) | `... --file %T%\b_int5.ls` with `x = symbol("x"); integrate_full(exp(x^2), x);` | exit 0, `ok:true`; `IntegrationResult.status = Enum(IntegrationStatus,"Unevaluated")` with **empty** `diagnostics` (`shape:[0]`). Also `integrate_full(sin(x^2),x)` and `integrate_full(1/(x^5+1),x)`. No capability entry covers integration, and no diagnostic records the refusal | **FINDING** |
| P6b-3 refusal is invisible to the documented diagnostics check | dsh-protocol.md:93-94 "An agent can therefore answer **"did anything go wrong, and can I branch on it?"** from `diagnostics.elements[].fields[category].type` + `value`" | `... --file %T%\b_int5.ls` (same run as P6b-2) | `diagnostics = {"kind":"Array","type":"Vector","shape":[0],"elements":[]}` while `status = Unevaluated`: "anything went wrong" is **not** recoverable from `diagnostics`; the only signal is the status enum | **FINDING** |
| P6b-4 documented `plot` surface crashes | dsh-protocol.md:3-5 (the runner is the agent-facing surface); `--help` advertises `--plot-dir`/`--plot-file` | `.\out\aot\Lovelace.Run.exe --file %T%\b_plot2.ls --plot-dir %T%\plots --omit-functions --omit-variables` with `x = symbol("x"); plot(sin(x));` | exit 1; `{"ok":false,"code":"InternalError","category":"InternalInvariantFailure","message":"Specified cast is not valid.","recoverable":false,...}` — with **and** without `--plot-dir`; no file written. `plot([1,2,3])` (a plain vector) succeeds, so only the symbolic-plot path is dead. No capability entry mentions `plot` | **FINDING** |
| P6b-5 non-Real numeric type as the solve variable | task probe list ("non-Real numeric types"); exhaustiveness claim | `... --file %T%\b_nonreal1.ls` with `x = symbol("x"); solve(x^2 - 2 == 0, 1);` | exit 1; `code=InvalidOperation`, `category=DomainError`, `message="solve(): argument 2 must be a symbolic variable; got Natural."` Same for `diff`, `integrate`, `limit` with `1` as the variable, and `solve([x == 1], x)` → `"...argument 1 must be a symbolic expression; got Vector."` No capability entry | **FINDING** |
| P6b-6 symbolic elements refused by DSP builtins | exhaustiveness claim | `... --file %T%\b_dft2.ls` with `x = symbol("x"); dft([x, 1, 2, 3]);` | exit 1; `code=InvalidOperation`, `category=DomainError`, `message="DSP builtins expect numeric/complex elements, but got 'SymbolExpr'."` — an unlisted refusal that also leaks an internal CLR type name | **FINDING** |
| P6b-7 numeric matrix refused by linsolve | exhaustiveness claim | `... --file %T%\b_linsol3.ls` with `A = [[1,2],[2,4]]; b = [[1],[3]]; linsolve(A, b);` | exit 1; `code=InvalidOperation`, `category=DomainError`, `message="linsolve() requires a symbolic matrix A."` No capability entry (and no documented way to build a "symbolic matrix") | **FINDING** |
| P6b-8 FFT length constraint | exhaustiveness claim | `... --file %T%\b_dsp1.ls` with `x = symbol("x"); fft([1,2,3]);` | exit 1; `code=InvalidArgument`, `category=TypeMismatch`, `message="FFT length must be a power of two, but got 3. (Parameter 'x')"` — unlisted refusal carrying a .NET `ArgumentException` artifact | **FINDING** (low; arguably an FFT fact, but it is a refusal the statement omits) |
| P6b-9 degree ≥ 4 / degree ≥ 5 complex algebraic roots | advertisement entry 3 covers `rootof.complex-algebraic` | `... --file %T%\b_deg5.ls` with `x = symbol("x"); solve(x^5 - x - 1 == 0, x);` | exit 0; `result.kind=Text`, value `"partially representable (4 root(s) missing: complex algebraic roots not supported (RootOf is real-only in v1).); representable: [rootof(x^5 - x - 1, 0)]"` — the refusal *cause* is advertised, so the class is covered | HELD (but see F2 for the envelope form) |

## Part 2 — the envelope itself

| probe | doc claim (file:line) | command | observed | verdict |
|---|---|---|---|---|
| P2-1 versioned triple always present | dsh-protocol.md:16 "`protocolVersion`, `symbolicFormatVersion` and `mathIrVersion` are always present." | every run above | `{"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,...}` in all success envelopes | HELD |
| P2-2 Enum on `status`/`completeness`/`exactness`/`classification`/`category` | dsh-protocol.md:45-48 | `... --file %T%\b_ssys2.ls`, `b_trans1.ls`, `b_steps1.ls`, `b_adv3.ls` | `Enum/SolveStatus`, `Enum/Completeness`, `Enum/SolutionExactness`, `Enum/IntegrationStatus`, `Enum/LimitStatus`, `Enum/TransformStatus`, `Enum/RuleClassification` (`RewriteStep.classification="Universal"`), `Enum/ErrorCategory` — all Enum, none Text | HELD |
| P2-3 `type()` names the declared enum type | dsh-protocol.md:51-52 "`type(r.status)` is `SolveStatus`, not `Enum`." | `... --file %T%\b_type1.ls` with `x = symbol("x"); r = solve_full(x^2-2==0, x); type(r.status);` | `{"kind":"Text","value":"SolveStatus"}` and `type(r)` → `"SolveResult"` | HELD |
| P2-4 Booleans are the STRING "true"/"false" | dsh-protocol.md:37 `{"kind":"Boolean","value":"true"}` | all record dumps above | `{"kind":"Boolean","value":"false"}` (adv3 `complete`, `int4` `verified`), `{"kind":"Boolean","value":"true"}` (adv3 Diagnostic `recoverable`, ssys2 `complete`) — always the JSON string | HELD |
| P2-5 Diagnostic = exactly six fields in order, empty `details` Array, `location` Null | dsh-protocol.md:68-78 | `... --file %T%\b_adv3.ls` (raw envelope quoted in the P6a-3 row and F6) | `Diagnostic` fields in order `code(Text)`, `category(Enum/ErrorCategory)`, `message(Text)`, `recoverable(Boolean "true")`, `location({"kind":"Null"})`, `details(Array shape[0])` — matches the documented example byte-for-byte in structure | HELD |
| P2-6 empty `diagnostics` present, never Null/"" | dsh-protocol.md:63-65 "When there is nothing to report the array is **empty** (`{"kind":"Array","shape":[0],"elements":[]}`): never `Null` and never `""`" | `b_trans1.ls`, `b_optf1.ls`, `b_compf1.ls`, `b_ssys2.ls` | every rich record ends with `{"name":"diagnostics","value":{"kind":"Array","type":"Vector","shape":[0],"elements":[]}}` (the binary additionally emits `"type":"Vector"`, which the doc's own `Array` form at line 33 also carries) | HELD |
| P2-7 all seven rich records end with `diagnostics` Array | dsh-protocol.md:61-62 | `b_ssys2.ls` (SystemSolveResult), `b_trans1.ls`/`b_steps1.ls` (TransformResult), `b_adv3.ls`/`b_limfull2.ls`/`b_int4.ls` (Solve/Limit/Integration), `b_optf1.ls` (OptimizationResult), `b_compf1.ls` (CompilationResult) | `diagnostics` is the last field and an Array in all seven | HELD |
| P2-8 Array `shape` + row-major `elements` | dsh-protocol.md:33 | `... --file %T%\b_matrix_rowmajor.ls` with `A = [[1,2,3],[4,5,6]]; A;` | `{"kind":"Array","type":"Array","shape":[2,3],"elements":[1,2,3,4,5,6]}` — row-major confirmed by distinct values 1..6 | HELD |
| P2-9 absent optionals are `{"kind":"Null"}` | dsh-protocol.md:15 | `b_limfull2.ls` (`LimitResult.value` when the limit does not exist), `b_inspect1.ls` (`shape`,`rank`,`members`) | `{"name":"value","value":{"kind":"Null"}}`, `{"name":"shape","value":{"kind":"Null"}}`, `{"name":"rank","value":{"kind":"Null"}}`, `{"name":"members","value":{"kind":"Null"}}` | HELD |
| P2-10 `--print-budget` truncation triple and ` …` prefix | dsh-protocol.md:17-20 | `.\out\aot\Lovelace.Run.exe --file %T%\b_budget1.ls --print-budget 5 --omit-functions --omit-variables` with `x = symbol("x"); expand((x+1)^20);` | `{"pretty":"x^20 + 20*x^19 + 190*x^18 + 1140*x^17 + 4845*x^ …","canonical":"(add (rat 1 1) (pow (sym x) (rat 20 1)) (mul ( …","nodeCount":98,"truncated":true,"truncationReason":"node-budget","budget":5}` — space + U+2026 terminator, genuine prefix of the untruncated rendering, `nodeCount` unmodified | HELD |
| P2-11 `--print-budget` rejects non-positive/garbage | dsh-protocol.md:185 "Exit codes: 0 success, 1 script/diagnostic error, 2 usage error." | `... --print-budget 0` / `-1` / `abc` | exit **2**, stderr `Error: --print-budget requires a positive node count.` + usage; no envelope on stdout | HELD |
| P2-12 `timings[].position` is the zero-based statement offset; `resultKind`; `hasOutput` | dsh-protocol.md:137-140 | `... --file %T%\pf.ls` with `print("hello world"); x = symbol("x"); x^2 + 1;` | `positions 0,22,39` (= exact source offsets), `resultKind:"Void"` for the print statement, `hasOutput:true` only for it | HELD |
| P2-13 stdout purity with `print()` | dsh-protocol.md:9-10 "Anything the script prints with `print(...)` is captured and returned in the top-level `output` array." | `.\out\aot\Lovelace.Run.exe --file %T%\purity.ls --omit-functions 2>$null` with `print("LINE-ONE"); print("LINE-TWO"); x = symbol("x"); x+1;` | stdout is exactly one JSON document (985 chars, 1 newline) and `output:["LINE-ONE\r","LINE-TWO"]` — captured, not printed raw | HELD for purity; see F5 for the captured bytes |
| P2-14 `NoSolutions` row of the status table | dsh-protocol.md:161-162 (`NoSolutions` → `complete:true` → `Completeness.Complete`) | `... --file %T%\nosol.ls` with `x = symbol("x"); solve_full(x^2 + 1 == 0, x, real);` | `status=Enum(SolveStatus,"NoSolutions")`, `complete={"kind":"Boolean","value":"true"}`, `completeness=Enum(Completeness,"Complete")` | HELD |
| P2-15 error envelope fields + exit 1 | dsh-protocol.md:172-185 | `... --file %T%\adv0.ls --omit-functions` with `2^(1/2);` | exit 1; `{"ok":false,"code":"UnsupportedOperation","category":"UnsupportedOperation","message":"...","recoverable":true,"diagnostics":[{"message":"...","position":0,"line":1,"column":1}],"elapsed":"336.2 µs"}` — `code`, `category`, `recoverable`, `diagnostics` all present | HELD |
| P2-16 error-envelope diagnostics = parser's source-position form, position/line/column truthful | dsh-protocol.md:201-204 | `... --file %T%\b_parse1.ls` with `1 +;` | `message="Unexpected token ';' at position 3: ..."`, `diagnostics[0]={"message":..., "position":3,"line":1,"column":4}` — structured position agrees with the message; `b_parse2.ls` (`x = ;`) → position 4, column 5 | HELD |
| P2-17 usage error exit 2 with no envelope | dsh-protocol.md:185 | `.\out\aot\Lovelace.Run.exe` (no arguments) | exit **2**, stderr `Error: No script provided. Use --eval <script>, --file <path>, --stdin, or a bare file path.` + usage; nothing on stdout | HELD |
| P2-18 unreadable file | dsh-protocol.md:183-185 (`ParseError` is in the taxonomy, exit 1 for script errors) | `.\out\aot\Lovelace.Run.exe --file %T%\nonexistent.ls` | exit 1; `{"ok":false,"code":"FileReadError","category":"ParseError","message":"Cannot read script file '...': Could not find file '...'.","recoverable":true,"diagnostics":[],"elapsed":"0 ms"}` | HELD |
| P2-19 same script twice differs ONLY in volatile fields | dsh-protocol.md:96-140 (envelope shape) | script `%T%\b_ssys2.ls` run twice, both envelopes flattened to 99 leaf paths and compared | `DIFFERING PATHS: 1` → only `root.elapsed` (`'686.4 µs'` vs `'668.1 µs'`) and its sibling `elapsedTime`/`timings` values; `revision` (83), every `result` path, `diagnostics`, and `timings[].position/resultKind/hasOutput` identical | HELD |
| P2-20 bare-string degradation, main result | dsh-protocol.md:11-14 "**Every value is structural.** ... nothing degrades to text because the serializer lacked a case." | `... --file %T%\b_sys1.ls` with `x = symbol("x"); y = symbol("y"); solve_system([x + y == 2, x - y == 0], [x, y]);` | `result = {"kind":"Text","display":"x = 1, y = 1","typed":"x = 1, y = 1","structured":{"kind":"Text","value":"x = 1, y = 1"}}` — the whole system solution (variables *and* values) is one unparsable string although `solve_system_full` on the same input returns a structured `SystemSolveResult` | **FINDING** (F1) |
| P2-20a short-form `solve` degrades the partial case to Text | dsh-protocol.md:11-14 "nothing degrades to text because the serializer lacked a case"; dsh-protocol.md:45-48 (`status` is an enum); dsh-protocol.md:149-151 "`unrepresented_count`/`unrepresented_reason` say how much is missing and why" | `... --file %T%\deg4.ls --omit-functions --omit-variables` with `x = symbol("x"); solve(x^4 - x^2 - 1 == 0, x);` | `ok:true`; `result = {"kind":"Text","structured":{"kind":"Text","value":"partially representable (2 root(s) missing: complex algebraic roots not supported (RootOf is real-only in v1).); representable: [rootof(x^4 - x^2 - 1, 0), rootof(x^4 - x^2 - 1, 1)]"}}` — no `status`, no `unrepresented_count`, no `unrepresented_reason`; the same script via `solve_full` returns `SolveResult` with `Enum(SolveStatus,"Partial")`, `represented_count:2`, `unrepresented_count:2`. `solve(x^5 - x - 1 == 0, x)` behaves identically | **FINDING** (F2) |
| P2-20b short-form `limit` degrades the outcome to Text | dsh-protocol.md:11-14; dsh-protocol.md:45-48 (`LimitStatus` is an enum) | `... --file %T%\lim1.ls` `x = symbol("x"); limit(sin(x), x, inf);` and `... --file %T%\lim2.ls` `x = symbol("x"); limit(1/x, x, 0);` | `limit(sin(x),x,inf)` → `{"kind":"Text","value":"unevaluated: coefficient does not evaluate at the point"}`; `limit(1/x,x,0)` → `{"kind":"Text","value":"does not exist (left: -inf, right: +inf)"}`; `limit(x^2,x,inf)` (the success case) → `{"kind":"Symbolic",...}`. `limit_full` on the same inputs returns `Enum(LimitStatus,"Unevaluated")` / `Enum(LimitStatus,"DoesNotExist")`. `limit_full(sin(x),x,inf)`, `limit_full(exp(x),x,inf)`, `limit_full(sin(1/x),x,0)` all → `Unevaluated` + `limit.unevaluated` diagnostic | **FINDING** (F2) |
| P2-21 arity errors are `InvalidArgument`/`TypeMismatch` naming the builtin | dsh-protocol.md:187-195 | `b_arity_abs2.ls` `abs(1,2);`, `b_arity_det0.ls` `det();`, `b_arity_len2.ls` `len(1,2);`, `b_arity_min0.ls` `min();`, `b_arity_sum3.ls` `sum(1,2,3);` | `abs(1,2)` → `code=InvalidOperation`,`category=DomainError`,`message="abs() expects exactly 1 argument(s), but got 2."`; `det()` → `InvalidOperation`/`DomainError`; `len(1,2)` → `InvalidOperation`/`DomainError`; `min()` → `InvalidOperation`/`DomainError`; `sum(1,2,3)` → `InvalidOperation`/`DomainError`,`message="Expected 1 or 2 arguments, but got 3."` (**no builtin name**). Only some builtins follow the documented path (`sin(1,2)` → `InvalidArgument`/`TypeMismatch` `"sin(): expected 1 argument; got 2."`, `subs`, `solve`, `evalf`, `factor` all conformant) | **FINDING** (F3) |
| P2-22 argument errors never surface as an internal invariant failure | dsh-protocol.md:187-191 "...before the builtin body can index an argument that was never supplied. It crosses as a recoverable argument error ... **never as an internal invariant failure**" | `b_arity_mean2.ls` `mean(1,2);`, `b_mean_1.ls` `mean(1);`, `b_max_2.ls` `max(1,2);`, `b_nonreal10.ls` `x = symbol("x"); sum(x, 5);`, `b_plot2.ls` `x = symbol("x"); plot(sin(x));` | all five: exit 1; `{"ok":false,"code":"InternalError","category":"InternalInvariantFailure","message":"Specified cast is not valid.","recoverable":false,...}`. `mean`'s own registry metadata declares `parameters:["a","axis"]` (1–2 args), so `mean(1)` / `mean(1,2)` are **not** arity violations, yet the body cast-fails; `recoverable` is `false`, not the documented `true` | **FINDING** (F4) |
| P2-23 `output` carries what the script printed | dsh-protocol.md:9-10 | `... --file %T%\p3.ls --omit-functions --omit-variables` with `print("A"); print("B"); print("C");` | `output = ["A\r","B\r","C"]` — every element except the last has a trailing CR appended | **FINDING** (F5) |
| P2-24 `complete` and `completeness` are one mapping and the two records cannot drift apart | dsh-protocol.md:153-156 "`complete` and `completeness` are read off ONE mapping ... so the two fields cannot disagree and the two records cannot drift apart"; dsh-protocol.md:57 "`completeness` is the authoritative field" | `... --file %T%\b_ssys2.ls` with `x = symbol("x"); y = symbol("y"); solve_system_full([x + y == 2, x - y == 0], [x, y]);` | `SystemSolveResult` fields = `status, domain, complete, solutions, diagnostics` — **there is no `completeness` field at all** (`SolveResult` from `solve_full` does carry both), so the authoritative field is absent from one of the two records the doc says cannot drift | **FINDING** (F6) |
| P2-25 `variables`/`partialVariables` entries expose structured values | dsh-protocol.md:3-5 "designed so that an agent never has to parse a display string to recover mathematical meaning"; dsh-protocol.md:11-14 | `... --file %T%\pf.ls --omit-functions` (`variables`), `... --file %T%\b_heavy.ls --cancel-after 1 --omit-functions --omit-variables` (`partialVariables`) | `variables:[{"name":"_","kind":"Symbolic","display":"x^2 + 1"},{"name":"x","kind":"Symbolic","display":"x"}]` and `partialVariables:[{"name":"x","kind":"Symbolic","display":"x"}]` — only `name`/`kind`/`display`; no `structured`, `canonical` or `pretty`, so recovering the value's meaning requires parsing `display` | **FINDING** (F7) |
| P2-26 durations are structural (`elapsedTime` next to `elapsed`), so no unit-suffix parsing | dsh-protocol.md:21-23 "Durations are structural: `elapsedTime` is `{value, unit}` next to the human `elapsed` string, and `timings` carries one entry per top-level statement, so an agent never parses a unit suffix." | every error envelope, e.g. `... --file %T%\b_arity_mean2.ls`, `... --file %T%\adv0.ls`; and `... --file %T%\b_heavy.ls --cancel-after 1` | `{"ok":false,...,"elapsed":"181.2 µs"}` / `"elapsed":"336.2 µs"` / `{"ok":false,"code":"Cancelled",...,"elapsed":"39.29 ms","partialOutput":[],"partialVariables":[...]}` — **no `elapsedTime` object and no `timings` array on any error envelope**; the agent must parse the unit suffix it was promised it would never need | **FINDING** (F8) |
| P2-27 `Cancelled` refusal + `partialOutput`/`partialVariables` | dsh-protocol.md:172-185 (error envelope shape), dsh-protocol.md:16-23 | `.\out\aot\Lovelace.Run.exe --file %T%\b_heavy.ls --cancel-after 1 --omit-functions --omit-variables` with `x = symbol("x"); expand((x+1)^400);` | exit 1; `code=Cancelled`, `category=BudgetExceeded`, `message="the evaluation was cancelled by the caller."`, `recoverable=true`, `diagnostics=[]`, plus **undocumented** `partialOutput`/`partialVariables` fields and the missing `elapsedTime`/`timings` of P2-26 | INCONCLUSIVE (documents a `BudgetExceeded` category but never this code or these fields; no line contradicted) |
| P2-28 same concept, two encodings: free symbols | dsh-protocol.md:32 (`"freeSymbols": ["x"]` — bare JSON strings inside a `Symbolic` value) vs dsh-protocol.md:11-14 | `... --file %T%\b_inspect1.ls` with `x = symbol("x"); inspect(x^2 + 1);` | `Symbolic.freeSymbols = ["x"]` (bare strings) but `Inspection.free_symbols = {"kind":"Array","type":"Vector","shape":[1],"elements":[{"kind":"Text","value":"x"}]}` (Text records); `Inspection` itself is an undocumented record type and carries `canonical`/`pretty` as flat `Text` fields instead of a `Symbolic` value | **FINDING** (F9, low) |
| P2-29 declared enum-type list covers every enum the runner emits | dsh-protocol.md:47 "( `SolveStatus`, `Completeness`, `SolutionExactness`, `LimitStatus`, `IntegrationStatus`, `TransformStatus`, `RuleClassification`, `ErrorCategory` )" | `... --file %T%\caps.ls` (`CapabilitiesExactness`), `... --file %T%\b_trans3.ls` with `x = symbol("x"); cancel_full((x^2-1)/(x-1));` | `capabilities().exactness` is `{"kind":"Enum","type":"CapabilitiesExactness","value":"BestEffort"}` and `cancel_full(...)` returns record type `CancelResult` with `{"kind":"Enum","type":"CancelStatus","value":"Conditional"}` and **no `diagnostics` field** — two enum types and one rich record type absent from the doc's enumerations | **FINDING** (F10, low) |
| P2-30 `revision` is a version/revision marker | dsh-protocol.md:16 (versioned triple) and dsh-protocol.md:104 (`"revision": 77`) | `b_rev1.ls` `a = 1;` → 82; `b_t.ls` `1;` → 81; `b_pf.ls` (vars `_,x`) → 82; `b_ssys2.ls` (vars `_,x,y`) → 83; `b_rev4.ls` `a=1;b=2;c=3;c;` → 84; `b_rev6.ls` `a=1;b=2;c=3;d=4;e=5;e;` → 86 (= 80 + 6 variables) | `revision` = 80 + the number of live variable slots, unchanged by `--omit-variables` (still 86), stable across identical runs but different for different scripts on the same binary | INCONCLUSIVE (the doc never defines `revision`; recorded as a contract-honesty gap — `revision` is not a revision of anything) |
| P2-31 `--omit-functions` / `--omit-variables` | dsh-protocol.md says nothing about either flag (only `--print-budget` at line 17); `--help` says "omit the builtin registry from the envelope (agent loops)" / "omit the variables array from the envelope" | `... --file %T%\pf.ls` with each flag alone and together | the keys are **never removed**: `--omit-functions` → `functions:[]` (0 of 123), `variables` still 2; `--omit-variables` → `variables` key still present with count 0; both → both empty arrays. An agent cannot distinguish "omitted" from "genuinely empty" | INCONCLUSIVE (doc-silent; the help text's "omit ... from the envelope" reads either way) |
| P2-32 parallel input surfaces emit the same envelope | dsh-protocol.md:3-5 | `--eval "1+1;"`, bare path `%T%\t.ls`, `--stdin` (piped `x = symbol("x"); x^2;`) | all three emit a well-formed envelope, exit 0, same key set as `--file` | HELD |
| P2-33 `--text` | dsh-protocol.md:3-5 "Its contract is a versioned **JSON envelope on stdout**" | `.\out\aot\Lovelace.Run.exe --file %T%\t.ls --text` | exit 0; stdout is `= 1 (Natural)\n  _ = 1` — not JSON | INCONCLUSIVE (`--text` is a documented `--help` opt-out; dsh-protocol.md never mentions it) |
| P2-34 extreme but valid input still yields an envelope | dsh-protocol.md:3-5 (JSON envelope on stdout), dsh-protocol.md:185 "Exit codes: 0 success, 1 script/diagnostic error, 2 usage error." | `... --file %T%\d3000.ls` with `x = symbol("x"); sin(sin(...sin(x)...));` (3000 nested calls), stdout/stderr captured separately | exit **-1073741571** (`0xC00000FD`, `STATUS_STACK_OVERFLOW`), **stdout 0 bytes** (no envelope at all), stderr `Process is terminating due to StackOverflowException.` Depths 1000 and 2000 on the same generator return exit 0 with full envelopes (24,624 / 48,624 stdout bytes) | **FINDING** (F11) |

## FINDINGS

### F1 — `solve_system()` returns a bare human string instead of the documented `SystemSolveResult` (silent degradation)

Doc claim (dsh-protocol.md:11-14): "**Every value is structural.** Every field is one of
`scalar | symbolic | array | record | domain | enum | null`; nothing degrades to text because the
serializer lacked a case." Doc claim (dsh-protocol.md:61-62): `SystemSolveResult` is one of the
"rich result records" ending in `diagnostics`; (dsh-protocol.md:142-147): "An agent can answer, from
structure alone...".

Reproduction:

```
> [IO.File]::WriteAllText("$env:TEMP\p2p6\sys1.ls", 'x = symbol("x"); y = symbol("y"); solve_system([x + y == 2, x - y == 0], [x, y]);')
> .\out\aot\Lovelace.Run.exe --file $env:TEMP\p2p6\sys1.ls --omit-functions --omit-variables
{"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":true,"revision":83,
 "result":{"kind":"Text","display":"x = 1, y = 1","typed":"x = 1, y = 1",
 "structured":{"kind":"Text","value":"x = 1, y = 1"}}, ...}
```

The *same* input through the full form yields structure, proving the structured answer exists and is
withheld by the short form:

```
> .\out\aot\Lovelace.Run.exe --file $env:TEMP\p2p6\ssys2.ls --omit-functions --omit-variables   # x = symbol("x"); y = symbol("y"); solve_system_full([x + y == 2, x - y == 0], [x, y]);
... "structured":{"kind":"Record","type":"SystemSolveResult","fields":[{"name":"status","value":{"kind":"Enum","type":"SolveStatus","value":"Solved"}}, ...
```

`x = 1` and `y = 1` are recoverable only by parsing prose. `solve_system` is a registered builtin
(`functions[].name == "solve_system"`, `parameters:["equations","variables"]`).

### F2 — the partial-solve and unevaluated-limit outcomes degrade to Text as well

Doc claims contradicted: dsh-protocol.md:11-14 (every value structural; enum-valued fields never
Text), dsh-protocol.md:45-48 (`status` is an enum), dsh-protocol.md:149-151 "When the solver cannot
represent the whole set, `status` is `Partial` and `unrepresented_count`/`unrepresented_reason` say
how much is missing and why."

```
> .\out\aot\Lovelace.Run.exe --file $env:TEMP\p2p6\deg4.ls --omit-functions --omit-variables   # x = symbol("x"); solve(x^4 - x^2 - 1 == 0, x);
"result":{"kind":"Text","structured":{"kind":"Text","value":"partially representable (2 root(s) missing: complex algebraic roots not supported (RootOf is real-only in v1).); representable: [rootof(x^4 - x^2 - 1, 0), rootof(x^4 - x^2 - 1, 1)]"}}

> .\out\aot\Lovelace.Run.exe --file $env:TEMP\p2p6\lim1.ls --omit-functions --omit-variables   # x = symbol("x"); limit(sin(x), x, inf);
"result":{"kind":"Text","structured":{"kind":"Text","value":"unevaluated: coefficient does not evaluate at the point"}}

> .\out\aot\Lovelace.Run.exe --file $env:TEMP\p2p6\lim2.ls --omit-functions --omit-variables   # x = symbol("x"); limit(1/x, x, 0);
"result":{"kind":"Text","structured":{"kind":"Text","value":"does not exist (left: -inf, right: +inf)"}}
```

No `status`, no `unrepresented_count`, no `unrepresented_reason`, no `LimitStatus` — the same script
run with `solve_full` / `limit_full` returns the enums (`SolveStatus.Partial`,
`LimitStatus.DoesNotExist`, `LimitStatus.Unevaluated`). The kind of the result therefore *changes
from structured to text exactly in the failure case*, which is the case an agent must branch on.

### F3 — two different arity-error contracts; the documented one is not universal

Doc claim (dsh-protocol.md:187-195): "A call with the wrong number of arguments is rejected at the
call site... It crosses as a recoverable argument error (`code: InvalidArgument`,
`category: TypeMismatch`) naming the builtin and both counts".

```
> .\out\aot\Lovelace.Run.exe --file $env:TEMP\p2p6\abs2.ls --omit-functions    # abs(1,2);
{"ok":false,"code":"InvalidOperation","category":"DomainError","message":"abs() expects exactly 1 argument(s), but got 2.","recoverable":true,...}
> .\out\aot\Lovelace.Run.exe --file $env:TEMP\p2p6\sum3.ls --omit-functions    # sum(1,2,3);
{"ok":false,"code":"InvalidOperation","category":"DomainError","message":"Expected 1 or 2 arguments, but got 3.","recoverable":true,...}
> .\out\aot\Lovelace.Run.exe --file $env:TEMP\p2p6\sin2.ls --omit-functions    # sin(1,2);
{"ok":false,"code":"InvalidArgument","category":"TypeMismatch","message":"sin(): expected 1 argument; got 2.","recoverable":true,...}
```

Conformant (wrong argument *count* → documented envelope): `sin(1,2)`, `subs(x)` → `"subs(): expected
3 arguments; got 1."`, `solve(f)` → `"solve(): expected 2 to 3 arguments; got 1."`, `evalf(x^2)`,
`factor(x^2-2, real)`, `complex(1)` → `"complex(): expected 0 arguments; got 1."`, `powerseries(...)`
4 args. Non-conformant (wrong argument *count*, but wrong `code`, wrong `category`, and for
`sum`/`min`/`mean`/`plot` no builtin name in the message): `abs`, `det`, `len`, `min`, `sum`, `mean`,
`plot`. (`max(1,2)` is not an arity error at all — 2 arguments are legal per its own metadata — it
belongs to F4.)

### F4 — caller-side argument errors surface as `InternalInvariantFailure` with a raw CLR message

Doc claim (dsh-protocol.md:187-191): the argument error "crosses as a recoverable argument error
(...`recoverable: true`...), **never as an internal invariant failure**."

```
> .\out\aot\Lovelace.Run.exe --file $env:TEMP\p2p6\mean1.ls --omit-functions    # mean(1);
{"ok":false,"code":"InternalError","category":"InternalInvariantFailure","message":"Specified cast is not valid.","recoverable":false,
 "diagnostics":[{"message":"Specified cast is not valid.","position":0,"line":1,"column":1}],"elapsed":"181.2 µs"}
> .\out\aot\Lovelace.Run.exe --file $env:TEMP\p2p6\max2.ls --omit-functions     # max(1,2);      -> identical envelope
> .\out\aot\Lovelace.Run.exe --file $env:TEMP\p2p6\plot2.ls --omit-functions    # x = symbol("x"); plot(sin(x));  -> identical envelope
> .\out\aot\Lovelace.Run.exe --file $env:TEMP\p2p6\sx5.ls --omit-functions      # x = symbol("x"); sum(x, 5);     -> identical envelope
```

`mean(1)` and `mean(1,2)` are not arity violations by the binary's own registry metadata
(`{"name":"mean","parameters":["a","axis"],"builtin":true}` → 1–2 arguments accepted), so the failure
is inside the builtin body; `mean(1)` fails while `mean([1,2,3])` and `mean([[1,2],[3,4]], 0)`
succeed. The `message` is an unhandled .NET `InvalidCastException` string, `recoverable` is `false`,
and the code is a fourth vocabulary (`InternalError`) not in the doc's stable-code examples.

### F5 — `print()` capture appends a CR to every element but the last

Doc claim (dsh-protocol.md:9-10): "Anything the script prints with `print(...)` is captured and
returned in the top-level `output` array."

```
> [IO.File]::WriteAllText("$env:TEMP\p2p6\p3.ls", 'print("A"); print("B"); print("C");')
> .\out\aot\Lovelace.Run.exe --file $env:TEMP\p2p6\p3.ls --omit-functions --omit-variables
...,"output":["A\r","B\r","C"],...
```

Two prints: `["LINE-ONE\r","LINE-TWO"]`. Non-string arguments are equally affected:
`x = symbol("x"); print(x^2 + 1); print(42); print([1,2]);` →
`["x^2 + 1\r","42\r","[1, 2]"]`. An agent comparing a captured line to its expected value fails on
every element except the last, and the artifact is the Windows line separator leaking into the wire
contract. (The file was written with no trailing newline, so the CR is generated by the capture path,
not by the script.)

### F6 — `SystemSolveResult` has no `completeness` field

Doc claim (dsh-protocol.md:153-156): "`complete` and `completeness` are read off ONE mapping of the
effective status ... so the two fields cannot disagree and the two records cannot drift apart."
Doc claim (dsh-protocol.md:57): "`completeness` is the authoritative field."

```
> .\out\aot\Lovelace.Run.exe --file $env:TEMP\p2p6\ssys2.ls --omit-functions --omit-variables
"structured":{"kind":"Record","type":"SystemSolveResult","fields":[
 {"name":"status","value":{"kind":"Enum","type":"SolveStatus","value":"Solved"}},
 {"name":"domain","value":{"kind":"Domain","domain":"complex"}},
 {"name":"complete","value":{"kind":"Boolean","value":"true"}},
 {"name":"solutions", ...},
 {"name":"diagnostics","value":{"kind":"Array","type":"Vector","shape":[0],"elements":[]}}]}
```

`completeness` is absent (verified: field-name list is `status,domain,complete,solutions,diagnostics`).
`SolveResult` from `solve_full` carries both. A consumer that reads the authoritative `completeness`
field — as the doc directs — gets nothing from a `SystemSolveResult`, and `complete` is left with no
authoritative partner, i.e. the two records *have* drifted.

### F7 — `variables[]` (and `partialVariables[]`) carry display strings only

Doc claim (dsh-protocol.md:3-5): the surface is "designed so that an agent never has to parse a
display string to recover mathematical meaning"; dsh-protocol.md:11-14 (every value structural).

```
> .\out\aot\Lovelace.Run.exe --file $env:TEMP\p2p6\pf.ls --omit-functions
...,"variables":[{"name":"_","kind":"Symbolic","display":"x^2 + 1"},{"name":"x","kind":"Symbolic","display":"x"}],...
> .\out\aot\Lovelace.Run.exe --file $env:TEMP\p2p6\adv3.ls --omit-functions
...,"variables":[{"name":"_","kind":"Record","display":"SolveResult(status: Partial, variable: x, domain: complex, complete: False, completeness: Partial, solutions: [Solution(value: rootof(x^4 - x^2 - 1, 0), ...)]"}],...
```

There is no `structured`, `canonical` or `pretty` on these entries, so the value's mathematical
meaning is only in `display` — the one thing the doc says an agent never has to parse. The same
applies to `partialVariables` (F8's run). `--omit-functions` exists precisely for agent loops, which
makes this the loop-facing surface.

### F8 — every error envelope omits `elapsedTime` and `timings`, forcing unit-suffix parsing

Doc claim (dsh-protocol.md:21-23): "Durations are structural: `elapsedTime` is `{value, unit}` next
to the human `elapsed` string, and `timings` carries one entry per top-level statement, so an agent
never parses a unit suffix."

```
> .\out\aot\Lovelace.Run.exe --file $env:TEMP\p2p6\mean1.ls --omit-functions
{...,"ok":false,"code":"InternalError","category":"InternalInvariantFailure",...,"elapsed":"181.2 µs"}     <- no elapsedTime, no timings
> .\out\aot\Lovelace.Run.exe --file $env:TEMP\p2p6\adv0.ls --omit-functions
{...,"ok":false,"code":"UnsupportedOperation",...,"elapsed":"336.2 µs"}                                      <- no elapsedTime, no timings
> .\out\aot\Lovelace.Run.exe --file $env:TEMP\p2p6\heavy.ls --cancel-after 1 --omit-functions --omit-variables
{...,"ok":false,"code":"Cancelled","category":"BudgetExceeded",...,"elapsed":"39.29 ms","partialOutput":[],"partialVariables":[{"name":"x","kind":"Symbolic","display":"x"}]}
```

Success envelopes do carry `elapsedTime` and `timings`; every failure path drops both. An agent that
reports timing for failed runs must parse `"39.29 ms"` / `"336.2 µs"`. (The doc's own error example
at line 180 also shows only `elapsed`, so the invariant and the example disagree with each other —
the binary follows the example and breaks the invariant.)

### F9 — the same documented datum has two encodings (free symbols), and `Inspection` is undocumented

Doc claim (dsh-protocol.md:32): a `Symbolic` value carries `"freeSymbols": ["x"]` — bare JSON strings.

```
> .\out\aot\Lovelace.Run.exe --file $env:TEMP\p2p6\inspect1.ls --omit-functions --omit-variables   # x = symbol("x"); inspect(x^2 + 1);
"structured":{"kind":"Record","type":"Inspection","fields":[
 {"name":"type","value":{"kind":"Text","value":"Symbolic"}},
 {"name":"free_symbols","value":{"kind":"Array","type":"Vector","shape":[1],"elements":[{"kind":"Text","value":"x"}]}},
 {"name":"canonical","value":{"kind":"Text","value":"(add (rat 1 1) (pow (sym x) (rat 2 1)))"}},
 {"name":"pretty","value":{"kind":"Text","value":"x^2 + 1"}}, ...]}
```

The same information (`Symbolic.freeSymbols` vs `Inspection.free_symbols`) arrives once as bare
strings and once as `Text` records, and `Inspection` re-serialises the canonical/pretty forms as flat
`Text` instead of reusing the documented `Symbolic` value form. The record type `Inspection` is not
in the doc's value forms or record list.

### F10 — enum/record types absent from the doc's enumerations

Doc claim (dsh-protocol.md:47): "The declared enum type is part of the value: `type` names it
(`SolveStatus`, `Completeness`, `SolutionExactness`, `LimitStatus`, `IntegrationStatus`,
`TransformStatus`, `RuleClassification`, `ErrorCategory`)". Doc claim (dsh-protocol.md:61-62) lists
the rich result records.

```
> .\out\aot\Lovelace.Run.exe --file $env:TEMP\p2p6\caps.ls --omit-functions
... "exactness":{"kind":"Enum","type":"CapabilitiesExactness","value":"BestEffort"} ...
> .\out\aot\Lovelace.Run.exe --file $env:TEMP\p2p6\trans3.ls --omit-functions --omit-variables   # x = symbol("x"); cancel_full((x^2-1)/(x-1));
"structured":{"kind":"Record","type":"CancelResult","fields":[
 {"name":"status","value":{"kind":"Enum","type":"CancelStatus","value":"Conditional"}},
 {"name":"original",...},{"name":"expression",...},{"name":"changed",...},{"name":"conditions",...}]}
```

`CapabilitiesExactness` (the enum type of the exhaustive-statement's own `exactness` field) and
`CancelStatus` are ninth and tenth enum types outside the listed set, and `CancelResult` is a rich
result record that — unlike every record in the doc's list — has **no `diagnostics` field at all**.
Low severity (the lists read as illustrative), but a consumer generating a switch from the doc will
not compile/handle these.

### F11 — a valid script crashes the runner: native stack overflow, exit `0xC00000FD`, empty stdout

Doc claim (dsh-protocol.md:185): "Exit codes: 0 success, 1 script/diagnostic error, 2 usage error."
Doc claim (dsh-protocol.md:3-5): "Its contract is a versioned JSON envelope on stdout."

```
> # generate x = symbol("x"); sin(sin(...sin(x)...));   with 3000 nested calls
> Start-Process .\out\aot\Lovelace.Run.exe -ArgumentList '--file','%T%\d3000.ls','--omit-functions','--omit-variables' -Wait -PassThru -RedirectStandardOutput so.txt -RedirectStandardError se.txt
exit=0xC00000FD (-1073741571)   stdoutBytes=0   stderrBytes=55 ("Process is terminating due to StackOverflowException.")
# same generator, depth 1000 -> exit 0, stdout 24624 bytes ; depth 2000 -> exit 0, stdout 48624 bytes
```

There is no envelope, no `code`, no `category`, no `recoverable`, and the exit code is outside the
documented set. Any agent loop that pipes `stdout` into a JSON parser gets an empty document; the
depth that crashes is inside the range a generated script can reach.

### Secondary: `revision` is not a revision (measured, doc-silent)

`revision` = **80 + number of live variable slots**: `1;` → 81, `a = 1;` → 82, `pf.ls` (vars `_,x`)
→ 82, `ssys2.ls` (vars `_,x,y`) → 83, `a=1;b=2;c=3;c;` → 84, `a=1;b=2;c=3;d=4;e=5;e;` → 86, and
still 86 under `--omit-variables`. It is stable across identical runs but changes with script
content on one unchanged binary. dsh-protocol.md never defines it (only line 104's example `77`), so
no line is contradicted — classified INCONCLUSIVE above — but an agent that treats the field as the
build/revision marker its name and its place beside `protocolVersion` suggest will mis-detect
toolchain changes.

## What I could not test / could not establish

1. **`truncationReason: "depth-limit"`** (dsh-protocol.md:19) — never observed. `--print-budget 5`
   gives `node-budget`; a 400-deep and a 2000-deep nested expression with `--print-budget 100000`
   were returned in full, and deeper input crashes first (F11, 3000 deep). No input produced
   `depth-limit`, so that branch of the documented truncation contract is unverified.
2. **`transform.budget-exceeded` / `transform.unsatisfiable-conditions`** (dsh-protocol.md:81, 86-88)
   — never observed. `--print-budget` bounds renderings, not transforms; `simplify_full`'s only
   parameter is `f` and it rejected extra conditions (`simplify_full(): expected 1 argument; got 2`),
   `cancel_full` likewise. The nearest contradiction of assumptions surfaced as a *top-level error
   envelope* with a different vocabulary: `x = symbol("x"); assume(x > 0); assume(x < 0);
   simplify_full(sqrt(x^2));` → exit 1, `code=UnsatisfiableAssumptions`, `category=DomainError`,
   `message="Assumption x < 0 contradicts the existing assumptions (its negation x >= 0 is provable)."`
   I could not reach the documented `transform.unsatisfiable-conditions` diagnostic or its
   `recoverable: false` contract, so I cannot call this a contradiction.
3. **`NoSolution` as an error-envelope category** (dsh-protocol.md:184) — I only ever saw
   `NoSolutions` as a `SolveStatus` enum member (P2-14). Whether the category exists at the envelope
   level is untested.
4. **A non-empty `Diagnostic.details` array** (dsh-protocol.md:91) — every diagnostic I produced had
   `details: {"kind":"Array","shape":[0],"elements":[]}`. The element form of `details` is
   unverified; I never found an operation that populates it.
5. **`DiagnosticLocation`** (dsh-protocol.md:88-90) — never observed; every diagnostic's `location`
   was `{"kind":"Null"}`. The four documented span fields are unverified.
6. **`CompilationResult` and `OptimizationResult` failure paths** — both only ever appeared with an
   empty `diagnostics` array; I could not force a diagnostic into them.
7. **`--omit-functions`/`--omit-variables` as a contract** — dsh-protocol.md is silent, so the
   observed "key stays, array empties" behaviour is recorded as INCONCLUSIVE rather than a finding;
   the same applies to `--text` (non-JSON stdout) and to the `partialOutput`/`partialVariables`
   fields of the `Cancelled` envelope.
8. **Whether `plot()`'s internal cast failure** (F4/P6b-4) is "plot is unsupported" or "plot is
   broken" — `plot([1,2,3])` succeeds and writes `plot.svg` to the working directory (the file it
   created in the repo root during this audit was deleted immediately), so only the symbolic path is
   dead; I could not tell which was intended.
9. **Cross-platform behaviour** of the `print()` CR artifact (F5) — this is a Windows AOT binary; I
   cannot tell from here whether the captured CR is platform-conditional.

## Tally

51 probe rows total: **27 HELD / 20 FINDING / 4 INCONCLUSIVE**, consolidating into 11 numbered findings.

* **HELD: 27** — part 6(a): P6a-0, P6a-1, P6a-2, P6a-3, P6a-4, P6a-5 (6). Part 6(b): P6b-9 (1). Part 2: P2-1..P2-19 and P2-32 (20).
* **FINDING: 20** — part 6(b): P6b-1, P6b-2, P6b-3, P6b-4, P6b-5, P6b-6, P6b-7, P6b-8 (8). Part 2: P2-20 (F1), P2-20a and P2-20b (F2), P2-21 (F3), P2-22 (F4), P2-23 (F5), P2-24 (F6), P2-25 (F7), P2-26 (F8), P2-28 (F9), P2-29 (F10), P2-34 (F11) (12).
* **INCONCLUSIVE: 4** — P2-27 (`Cancelled` + `partialOutput`/`partialVariables`), P2-30 (`revision`), P2-31 (omit flags), P2-33 (`--text`), plus the nine untested items listed above.
