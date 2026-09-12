# Cycle 6 · round 5 — the fourteen residual bounds of section O.2, re-probed against the current tree

**Probed revision.** `git rev-parse HEAD` → `1b70a320a313b2374167ee8ced179aa6dd79350a`; `git status --porcelain` at probe time →
`M docs/goal-cycle-6/journal.md`, `M docs/goal-cycle-6/state.md`, `?? Lovelace.Real.Tests/ZzRound4ProbeTests.cs`, `?? docs/goal-cycle-5/waiter6.ps1`, `?? docs/goal-cycle-6/round-4/` — no product source or fixture was modified by this round.

**The tree moved while this round ran.** At re-check time `git status` also showed `M Lovelace.Real/Real.cs` (file written 9/11/2026 9:37:41 PM, after this round's runner build at 9:28:36 PM); the copy of `Lovelace.Real.dll` in the runner's output directory is 9:37:52 PM. The Real-sensitive probes were re-run under that newer dependency — `1/2*sqrt(8)`, `1/(3*10^1000)`, `setprecision(18); 1/1009`, `sin(x)`, `2^(1/2)` — and returned byte-identical deciding fields to the first pass.

**Runner, rebuilt from that tree.** `dotnet build Lovelace.Run/Lovelace.Run.csproj --configuration Release --nologo` → `Build succeeded. 0 Warning(s) 0 Error(s)` (4.51 s). Binary used everywhere below, called `$RUN`:

```
C:\Users\ricar\dev\LovelaceSharp\Lovelace.Run\bin\Release\net10.0\Lovelace.Run.dll
```

**Scratch scripts.** `%T% = C:\Users\ricar\AppData\Local\Temp\lov6` (outside the repo); each row quotes the script text it ran.
Why `--file` rather than `--eval`: `dotnet $RUN --eval 'x = symbol("x"); x^2' --json --omit-functions --omit-variables` exits 1 with `InvalidOperation/DomainError "Undefined variable 'x'."`, while the identical text in a `--file` succeeds (❓ the cause was not investigated), so every probe whose script contains an assignment or double quotes used `--file`.

**Ground truth used for every value claim.** SymPy 1.14.0 / mpmath, Python 3.12.14 at `C:\Users\ricar\dev\.lovelace-tools\python` (prepended to `$env:PATH`; `python --version` → `Python 3.12.14`), script `%T%\gt.py`.

**Row O-B13 was not re-probed** (it is CLOSED in the document). Its current CI state was looked up and is recorded below.

---

## O-B1 — general rational exponents of a positive base

| Bound | One-line statement | Exact command | Observed result (verbatim) | Still true today? |
|---|---|---|---|---|
| O-B1 | `2^(1/2)` and `2^(1/3)` are refused, and a symbolic result the printer renders as `2^(1/3)` cannot be re-evaluated | `dotnet $RUN --file %T%\b1_a.ls --json --omit-functions --omit-variables` (b1_a.ls = `2^(1/2)`) | `{"ok":false,"code":"UnsupportedOperation","category":"UnsupportedOperation","message":"Non-integer exponents are not yet supported.","recoverable":true}`, exit 1 | STILL TRUE |
| | second shape | `dotnet $RUN --file %T%\b1_b.ls --json --omit-functions --omit-variables` (b1_b.ls = `2^(1/3)`) | identical envelope: `code=UnsupportedOperation, category=UnsupportedOperation, message=Non-integer exponents are not yet supported.`, exit 1 | STILL TRUE |
| | advertisement | `dotnet $RUN --file %T%\b14.ls --json --omit-functions --omit-variables` (b14.ls = `capabilities()`) | `{"operation_class":"pow.non-integer-exponent","code":"UnsupportedOperation","category":"UnsupportedOperation","message":"Non-integer exponents are not yet supported.","trigger":"2^(1/2)"}` | STILL TRUE |
| | the printed shape exists | `dotnet $RUN --file %T%\cubic.ls --json --omit-functions --omit-variables` (cubic.ls = `x = symbol("x"); solve_full(x^3 - 2 == 0, x)`) | `SolveResult(status: Solved, ... completeness: Complete, solutions: [Solution(value: 2^(1/3)*(-1/2*i*sqrt(3) - 1/2), ...), Solution(value: 2^(1/3)*(1/2*i*sqrt(3) - 1/2), ...), Solution(value: 2^(1/3), ...)], represented_count: 3, unrepresented_count: 0 ...)` | STILL TRUE |

## O-B2 — exponents with denominator ≥ 3 over a negative base

| Bound | One-line statement | Exact command | Observed result (verbatim) | Still true today? |
|---|---|---|---|---|
| O-B2 | `(-8)^(1/3)` is a typed refusal, advertised | `dotnet $RUN --file %T%\b2.ls --json --omit-functions --omit-variables` (b2.ls = `(-8)^(1/3)`) | `{"ok":false,"code":"UnsupportedOperation","category":"UnsupportedOperation","message":"Non-integer exponents are not yet supported.","recoverable":true}`, exit 1 | STILL TRUE |
| | advertisement | `dotnet $RUN --file %T%\b14.ls --json --omit-functions --omit-variables` | `{"operation_class":"pow.negative-base-unrepresentable-exponent","code":"UnsupportedOperation","category":"UnsupportedOperation","message":"Non-integer exponents are not yet supported.","trigger":"(-8)^(1/3)"}` | STILL TRUE |

## O-B3 — complex algebraic roots of degree ≥ 4

| Bound | One-line statement | Exact command | Observed result (verbatim) | Still true today? |
|---|---|---|---|---|
| O-B3 | `RootOf` stays real-only, so a quartic/quintic solve is `Partial` with `unrepresented_count` and a diagnostic | `dotnet $RUN --file %T%\b3.ls --json --omit-functions --omit-variables` (b3.ls = `x = symbol("x"); solve_full(x^4 - x^2 - 1 == 0, x)`) | `{"name":"status","value":{"kind":"Enum","type":"SolveStatus","value":"Partial"}}`, `complete: false`, `represented_count: 2`, `unrepresented_count: 2`, `unrepresented_reason: complex algebraic roots not supported (RootOf is real-only in v1).`, `diagnostics[0] = solve.unrepresented-roots / UnsupportedOperation`; each solution is `rootof(x^4 - x^2 - 1, 0/1)` with `"domain":"real"` | STILL TRUE |
| | second shape | `dotnet $RUN --file %T%\b3c.ls --json --omit-functions --omit-variables` (b3c.ls = `x = symbol("x"); solve_full(x^5 - x + 1 == 0, x)`) | `represented_count: 1, unrepresented_count: 4, unrepresented_reason: complex algebraic roots not supported (RootOf is real-only in v1).` | STILL TRUE |
| | independent ground truth | `python %T%\gt.py` (SymPy 1.14.0) | `SYMPY solve x^4-x^2-1: [-I*sqrt(-1/2 + sqrt(5)/2), I*sqrt(-1/2 + sqrt(5)/2), -sqrt(1/2 + sqrt(5)/2), sqrt(1/2 + sqrt(5)/2)]`; `SYMPY count real: 2`; `SYMPY solve x^5-x+1 count: 5 real: 1` — 4 (2 real) and 5 (1 real) roots, i.e. the runner represents exactly the real ones | STILL TRUE |

## O-B4 — complex `log` beyond the principal branch

| Bound | One-line statement | Exact command | Observed result (verbatim) | Still true today? |
|---|---|---|---|---|
| O-B4 | only the principal value of a complex `log` is produced | `dotnet $RUN --file %T%\log_n1.ls --json --omit-functions --omit-variables` (log_n1.ls = `evalf(log(-1), 30)`) | `{"kind":"Complex","display":"3.141592653589793238462643383279i","structured":{"kind":"Complex","exact":false,"re":"0","im":"3.141592653589793238462643383279"}}` | STILL TRUE |
| | second and third shape | `dotnet $RUN --file %T%\log_n1b.ls ...` (`evalf(log(-1/2), 30)`) and `%T%\log_n4.ls` (`evalf(log(-4), 30)`) | `"-0.693147180559945309417232121458 + 3.141592653589793238462643383279i"`; `"1.386294361119890618834464242916 + 3.141592653589793238462643383279i"` | STILL TRUE |
| | independent ground truth | `python %T%\gt.py` (mpmath 130 dps) | `MPMATH log(-1): (0.0 + 3.14159265358979323846264338327950288...j)`; `MPMATH log(-1/2): (-0.6931471805599453094172321214581765... + 3.14159265358979323846264338327950288...j)`; `MPMATH log(-4): (1.3862943611198906188344642429163531... + ...j)` — all 30 printed digits agree with the principal branch | STILL TRUE |
| | ❓ no branch argument is reachable in any probed form; every complex `log` returned the principal value only | | | STILL TRUE |

## O-B5 — `rootof(poly, k)` prints but does not re-parse

| Bound | One-line statement | Exact command | Observed result (verbatim) | Still true today? |
|---|---|---|---|---|
| O-B5 | the printer emits `rootof(...)` but the parser has no `rootof` | printed from `dotnet $RUN --file %T%\b5a.ls ...` (b5a.ls = `x = symbol("x"); solve_full(x^4 - x^2 - 1 == 0, x)`) | solution value pretty: `rootof(x^4 - x^2 - 1, 0)`, canonical `(rootof (add (rat -1 1) (pow (sym x) (rat 4 1)) (mul (rat -1 1) (pow (sym x) (rat 2 1)))) 0)` | STILL TRUE |
| | re-feed the printed text | `dotnet $RUN --file %T%\b5b.ls --json --omit-functions --omit-variables` (b5b.ls = `rootof(x^4 - x^2 - 1, 0)`) | `{"ok":false,"code":"InvalidOperation","category":"DomainError","message":"Undefined variable 'x'.","recoverable":true}`, exit 1 | STILL TRUE (message differs from the amendment's `Unknown function 'rootof'.`) |
| | isolate the name | `%T%\rootof_one.ls` (`rootof(1)`), `%T%\rootof_bare.ls` (`rootof`) | `Unknown function 'rootof'.` (exit 1) and `Undefined variable 'rootof'.` (exit 1) | STILL TRUE |

## O-B6 — an exact algebraic value computed numerically loses exactness

| Bound | One-line statement | Exact command | Observed result (verbatim) | Still true today? |
|---|---|---|---|---|
| O-B6 | `1/2*sqrt(8)` comes back as a numeric truncation labelled `exact:false` | `dotnet $RUN --file %T%\b6.ls --json --omit-functions --omit-variables` (b6.ls = `1/2*sqrt(8)`) | `{"kind":"Real","display":"1.4142135623730950488016887242096980785696718753769480731766797379907324784621070388503875343276415727","structured":{"kind":"Real","value":"1.4142135623730950488016887242096980785696718753769480731766797379907324784621070388503875343276415727","exact":false}}` | STILL TRUE |
| | second surface | `dotnet $RUN --file %T%\insp_rat.ls --json --omit-functions --omit-variables` (insp_rat.ls = `inspect(1/2*sqrt(8))`) | `Inspection(type: Real, domain: , exact: False, free_symbols: [], node_count: , canonical: , ...)` | STILL TRUE |
| | independent ground truth | `python %T%\gt.py` (mpmath 130 dps) | `MPMATH sqrt2 105: 1.41421356237309504880168872420969807856967187537694807317667973799073247846210703885038753432764157273501` — the runner's 100 printed digits agree exactly (the numeral is right; only the exactness label is lost) | STILL TRUE |

## O-B7 — `IsExact` is too narrow in the kernel

| Bound | One-line statement | Exact command | Observed result (verbatim) | Still true today? |
|---|---|---|---|---|
| O-B7 | `sin(x)`, `sqrt(x)` and `diff(sin(x),x)` report `exact:false` although they are exact | `dotnet $RUN --file %T%\b7_sin.ls --json --omit-functions --omit-variables` (b7_sin.ls = `x = symbol("x"); sin(x)`) | `{"kind":"Symbolic","pretty":"sin(x)","canonical":"(fn sin (sym x))","domain":"complex","exact":true,"nodeCount":2,"freeSymbols":["x"]}` | **NO LONGER TRUE** |
| | | `dotnet $RUN --file %T%\b7_sqrt.ls ...` (`x = symbol("x"); sqrt(x)`) | `"pretty":"sqrt(x)","canonical":"(pow (sym x) (rat 1 2))","domain":"complex","exact":true` | **NO LONGER TRUE** |
| | | `dotnet $RUN --file %T%\b7_diff.ls ...` (`x = symbol("x"); diff(sin(x), x)`) | `"pretty":"cos(x)","canonical":"(fn cos (sym x))","domain":"complex","exact":true` | **NO LONGER TRUE** |
| | second surface | `dotnet $RUN --file %T%\insp_sin.ls ...` (`inspect(sin(x))`) and `%T%\insp_sqrt.ls` (`inspect(sqrt(x))`) | `Inspection(type: Symbolic, domain: complex, exact: True, free_symbols: [x], node_count: 2, canonical: (fn sin (sym x)), ...)` and `... exact: True ... node_count: 3, canonical: (pow (sym x) (rat 1 2)) ...` | **NO LONGER TRUE** |

## O-B8 — the wire-contract cluster (nine sub-items, re-probed one by one)

| Bound | One-line statement | Exact command | Observed result (verbatim) | Still true today? |
|---|---|---|---|---|
| O-B8a | error envelopes carry no `elapsedTime`/`timings` | `dotnet $RUN --file %T%\b1_a.ls ...` (`2^(1/2)`), `%T%\err_div.ls` (`1/0`), `%T%\err_parse.ls` (`1 +`), `%T%\err_name.ls` (`nope`) | every one carries `"elapsed":"18.39 ms","elapsedTime":{"value":18.39,"unit":"ms"},"timings":[...]`; the three runtime errors carry a populated `timings` array, the parse error carries `"timings":[]` | **NO LONGER TRUE** |
| O-B8b | 39 of 123 builtins answer a wrong-arity call with `InvalidOperation/DomainError` instead of `InvalidArgument/TypeMismatch` | `powershell -NoProfile -ExecutionPolicy Bypass -File %T%\sweep.ps1` — for each of the 123 registered builtins, `dotnet $RUN --eval "<name>(<k x '1'>)" --json --omit-functions --omit-variables` with k=0 and k=9, one process per call (246 calls) | `ROWS=246`, `ZERO_ARG: InvalidArgument, TypeMismatch=112; OK, Text=1; OK, Record=2; OK, Domain=4; OK, Real=2; OK, Symbolic=1; =1`, `NINE_ARG: InvalidArgument, TypeMismatch=119; OK, Array=2; =1; InternalError, InternalInvariantFailure=1` — **zero rows with code `InvalidOperation` or category `DomainError`**. The single residue: `reshape(1,1,1,1,1,1,1,1,1)` → `{"ok":false,"code":"InternalError","category":"InternalInvariantFailure","message":"Unable to cast object of type 'Lovelace.Natural.Natural' to type 'Lovelace.Abstractions.ArrayValue'."}` (reshape has minArity 2 and variadic true, so this is a shape complaint, not an arity one) | **NO LONGER TRUE** for the stated 39/123 shape; one `InternalError` residue observed |
| O-B8c | `print` output keeps a trailing CR | `dotnet $RUN --file %T%\b8_print.ls --json --omit-functions --omit-variables > %T%\print-json.txt` (`print(1)`), `%T%\print_str.ls` (`print("a")`), then `Format-Hex %T%\print-json.txt` and an index scan for `[char]13` | `"output":["1"]` and `"output":["a"]` (no escape in the value); `CRCOUNT=1 CRPOS=314 LEN=316 TAIL=[]` — the only CR is the terminal CRLF; `--text` bytes are `31 00 0D 00 0A 00` = `1` + CRLF | **NO LONGER TRUE** |
| O-B8d | `variables[]` is display-only | `dotnet $RUN --file %T%\b8_vars2.ls --json --omit-functions` (b8_vars2.ls = `x = 5; y = x + 1`) | `"variables":[{"name":"_","kind":"Natural","display":"6","structured":{"kind":"Natural","value":"6","exact":true}},{"name":"x","kind":"Natural","display":"5","structured":{...}},{"name":"y","kind":"Natural","display":"6","structured":{...}}]` | **NO LONGER TRUE** |
| O-B8e | `divrem` returns prose | `dotnet $RUN --file %T%\b8_divrem.ls --json --omit-functions --omit-variables` (`divrem(7, 2)`) | `{"kind":"Record","display":"DivRemResult(quotient: 3, remainder: 1)","structured":{"kind":"Record","type":"DivRemResult","fields":[{"name":"quotient",...},{"name":"remainder",...}]}}` | **NO LONGER TRUE** |
| O-B8f | `--print-budget` does not bound values already over budget | `dotnet $RUN --file %T%\pb_x2.ls --json --omit-functions --omit-variables --print-budget 1` (pb_x2.ls = `x = symbol("x"); x^2`, nodeCount 3) | `"structured":{"kind":"Symbolic","pretty":"x …","canonical":"(pow ( …","domain":"complex","exact":true,"nodeCount":3,"freeSymbols":["x"],"truncated":true,"truncationReason":"node-budget","budget":1}` | **NO LONGER TRUE** for the small-Symbolic shape the audit cited (A2-F19: nodeCount 2/3/5 never truncate) |
| | same flag, a second shape | `dotnet $RUN --file %T%\b8_pb.ls --json --omit-functions --omit-variables --print-budget 3` (b8_pb.ls = `[1,2,3,4,5,6,7,8,9,10]`) | all ten elements returned in `structured.elements`, `"display":"[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]"`, and **no** `truncated`/`budget` field anywhere in the envelope | ❓ a vector over the budget is not truncated, but whether this shape is the bound's subject was not established |
| O-B8g | `evalf(f,digits)` ignores `digits` for an already-numeric argument | `dotnet $RUN --file %T%\ev_sqrt5.ls ...` / `%T%\ev_sqrt30.ls` (`evalf(sqrt(2), 5)` / `evalf(sqrt(2), 30)`) and `%T%\b8_evalf1.ls` / `%T%\b8_evalf2.ls` (`evalf(1/3, 5)` / `evalf(1/3, 30)`) | `1.41421` (5 digits) and `1.414213562373095048801688724209` (30); `0.33333` and `0.333333333333333333333333333333`; ground truth `MPMATH sqrt2 105` and 1/3 agree to the printed digits | **NO LONGER TRUE** |
| O-B8h | `assumptions()` is Text-only | `dotnet $RUN --file %T%\b8_assum.ls --json --omit-functions --omit-variables` (`assumptions()`) | `{"kind":"Record","display":"AssumptionSet(display: , assumptions: [])","structured":{"kind":"Record","type":"AssumptionSet","fields":[{"name":"display","value":{"kind":"Text","value":""}},{"name":"assumptions","value":{"kind":"Array","type":"Vector","shape":[0],"elements":[]}}]}}` | **NO LONGER TRUE** |
| O-B8i | `functions[]` omits `MinArity`/`Variadic` | `dotnet $RUN --eval "1" --json --omit-variables` (registry not omitted) | `COUNT=123`, `FIELDS=name,parameters,minArity,parameterCount,variadic,builtin`, `ENTRY0={"name":"abs","parameters":["x"],"minArity":1,"parameterCount":1,"variadic":false,"builtin":true}` | **NO LONGER TRUE** |

## O-B9 — complex results do not round-trip end to end

| Bound | One-line statement | Exact command | Observed result (verbatim) | Still true today? |
|---|---|---|---|---|
| O-B9 | the printer emits `i` and the parser does not know the name | `dotnet $RUN --file %T%\b9.ls --json --omit-functions --omit-variables` (b9.ls = `sqrt(-1)`) | `{"kind":"Symbolic","display":"i","structured":{"kind":"Symbolic","pretty":"i","canonical":"(i)","domain":"complex","exact":true,"nodeCount":1}}` | STILL TRUE |
| | re-feed | `dotnet $RUN --eval "i" --json --omit-functions --omit-variables` | `{"ok":false,"code":"InvalidOperation","category":"DomainError","message":"Undefined variable 'i'.","recoverable":true}`, exit 1 | STILL TRUE |
| | other complex producers | `%T%\neg4_pow.ls` (`(-4)^(1/2)`), `%T%\b4b.ls` (`log(1 + i)`) | `"display":"2*i","canonical":"(mul (rat 2 1) (i))"`; `Undefined variable 'i'.` | STILL TRUE |
| | independent ground truth | `python %T%\gt.py` | `MPMATH (-4)^(1/2): (0.0 + 2.0j)` — the value behind `2*i` is the principal square root | STILL TRUE |

## O-B10 — `Real.Divide` returns 0, and the wire still labels some truncations exact

| Bound | One-line statement | Exact command | Observed result (verbatim) | Still true today? |
|---|---|---|---|---|
| O-B10 (first half) | `1/(3*10^1000)` returns 0 for a nonzero quotient | `dotnet $RUN --file %T%\b10a.ls --json --omit-functions --omit-variables` (b10a.ls = `1/(3*10^1000)`) | `{"kind":"Real","display":"0","typed":"0 (Real)","structured":{"kind":"Real","value":"0","exact":false}}`, exit 0 | STILL TRUE |
| | independent ground truth | `python %T%\gt.py` (mpmath) | `MPMATH 1/(3*10^1000): 3.333333333e-1001` — nonzero | STILL TRUE |
| O-B10 (second half) | the wire labels some truncations exact (`setprecision(18); 1/1009`) | `dotnet $RUN --file %T%\b10b.ls --json --omit-functions --omit-variables` (b10b.ls = `setprecision(18); 1/1009`) | `{"kind":"Real","display":"0.000991080277502477","structured":{"kind":"Real","value":"0.000991080277502477","exact":false}}` — the label is `exact:false`, not exact | **NO LONGER TRUE** for the probed shape |
| | independent ground truth | `python %T%\gt.py` (mpmath, 22 digits) | `SYMPY Rational(1,1009): 0.0009910802775024777006938` — the runner's 18 significant digits are correct | — |

## O-B11 — finite user-function recursion is not bounded

| Bound | One-line statement | Exact command | Observed result (verbatim) | Still true today? |
|---|---|---|---|---|
| O-B11 | `f(N)` is accepted at N=384 and kills the process at N=448 | script `%T%\rec.ls` = `func f(n) { if (n == 0) { return 0 }; return f(n - 1) }` + newline + `f(N)`, run as `dotnet $RUN --file %T%\rec.ls --json --omit-functions --omit-variables`; N driven by `powershell -NoProfile -ExecutionPolicy Bypass -File %T%\recursion2.ps1` | `N=432 exit=0 hex=0x00000000 stdoutBytes=535 stderr=[]` with `ok:true ... "kind":"Natural","display":"0"`; `N=436 exit=-1073741571 hex=0xC00000FD stdoutBytes=0 stderr=[Stack overflow. \|\| at Lovelace.Suite.Interpreter+<ExecuteReturnAsync>d__123.MoveNext()]`; `N=438/440/444/448/512` all `exit=-1073741571 hex=0xC00000FD stdoutBytes=0 stderr=[Stack overflow. ...]`; `N=128/256/384/400/416` all exit 0 with the same `ok:true` envelope | STILL TRUE (accepted up to 432, dead from 436 — the amendment records 384 accepted / 448 dead) |

## O-B12 — benchmark numbers come from a non-idle machine; no across-run spread

| Bound | One-line statement | Exact command | Observed result (verbatim) | Still true today? |
|---|---|---|---|---|
| O-B12 | the recorded benchmark numbers come from a machine that was not idle, with no across-run spread | `dotnet run -c Release --project C:\Users\ricar\dev\LovelaceSharp\symbench -- --filter *Calculus* --job short` (run twice; also from the project directory and as `dotnet ...\symbench\bin\Release\net10.0\symbench.dll --filter *Calculus* --job short`) | both runs: `\| Method \| Mean \| Error \|` / `\| Diff_TenthOrder \| NA \| NA \|`, `Benchmarks with issues:`, `Run time: 00:00:00 (0.22 sec), executed benchmarks: 0`, preceded by `// Generate Exception: System.NotSupportedException: Found more than one matching project file for symbench in C:\Users\ricar\dev\LovelaceSharp and its subfolders: 'C:\Users\ricar\dev\LovelaceSharp\symbench\symbench.csproj', 'C:\Users\ricar\dev\LovelaceSharp\.worktrees\c5-cancel\symbench\symbench.csproj', ... (53 paths) ... Benchmark project names needs to be unique.` | **COULD NOT BE MEASURED HERE** for the project's rows (the BenchmarkDotNet harness cannot generate its project in this workspace) |
| | machine state at probe time | `Get-Process \| Where-Object { $_.ProcessName -match 'dotnet\|MSBuild\|VBCSCompiler\|testhost' } \| Format-Table` | `dotnet` × 12 live processes, `VBCSCompiler` CPU 561.56 s — this machine is not idle | ❓ corroborates the bound's premise for this machine; it does not re-measure the committed documents |
| | surrogate across-run spread (different instrument) | `dotnet $RUN --file %T%\spread.ls --json --omit-functions --omit-variables` (`expand((x+1)^12)`) repeated 12 times in one PowerShell loop | `VALUES=14.63,17.08,14.78,21.39,14.58,15.95,14.82,18.85,14.7,14.68,14.83,14.52 MIN=14.52 MAX=21.39 MEAN=15.9 RANGE=6.87` (runner `elapsedTime` in ms) | ❓ measures the runner, not the symbench rows |

## O-B13 — CI had never executed on a GitHub runner (CLOSED; not re-probed)

| Bound | One-line statement | Exact command | Observed result (verbatim) | Still true today? |
|---|---|---|---|---|
| O-B13 | closed in the amendment; not re-probed this round | `web_fetch https://api.github.com/repos/jjackbauer/LovelaceSharp/actions/runs?per_page=4` | `#30 e8638c0 main status=in_progress conclusion=null created=2026-09-12T00:27:24Z`; `#29 300f6bb main status=completed conclusion=success`; `#28 70241dc main status=completed conclusion=success`; `#27 31d3e4a main status=completed conclusion=failure`. Jobs of #29: `Native AOT publish + runner smoke: success`, `Differential oracle (SymPy installed): success`, `Fast accuracy test suites: success` | CLOSED (not re-probed); ❓ run #30 was still in progress when looked up |

## O-B14 — `capabilities()`'s `exactness` is `BestEffort`, and one statement under-claims

| Bound | One-line statement | Exact command | Observed result (verbatim) | Still true today? |
|---|---|---|---|---|
| O-B14 | `exactness` is `BestEffort` and the advertisement under-claims `(-4)^(1/2)` | `dotnet $RUN --file %T%\b14.ls --json --omit-functions --omit-variables` (`capabilities()`) | `"exactness","value":{"kind":"Enum","type":"CapabilitiesExactness","value":"BestEffort"}`; the `unsupported_operations` vector has `"shape":[16]` | STILL TRUE |
| | re-run every advertised trigger | `powershell -NoProfile -ExecutionPolicy Bypass -File %T%\capverify.ps1` (writes each `trigger` to a `.ls` file and runs `dotnet $RUN --file <trigger>.ls --json --omit-functions --omit-variables`) | `ENTRIES=16`, `EXACTNESS=BestEffort`, `MATCHES=13/16`; the three envelope-level mismatches — `rootof.complex-algebraic`, `limit.unevaluated-in-record-diagnostics`, `integration.unevaluated-in-record-diagnostics` — all return `ok=True kind=Record`, and their records' diagnostics carry the advertised pair (`solve.unrepresented-roots / UnsupportedOperation`, `limit.unevaluated / UnsupportedOperation`, `integration.unevaluated / UnsupportedOperation`) | STILL TRUE; the amendment's `16/16 matching` reproduces only when record diagnostics count as the live call — 13/16 at envelope level, 16/16 at diagnostic level |
| | the under-claim | `dotnet $RUN --file %T%\neg4_pow.ls --json --omit-functions --omit-variables` (`(-4)^(1/2)`) | `{"kind":"Symbolic","display":"2*i","structured":{"kind":"Symbolic","pretty":"2*i","canonical":"(mul (rat 2 1) (i))","domain":"complex","exact":true}}` while `pow.non-integer-exponent` is advertised with trigger `2^(1/2)` | STILL TRUE (ground truth: `MPMATH (-4)^(1/2): (0.0 + 2.0j)`) |

---

## 1. Bounds that are NO LONGER TRUE (candidates to close)

| Bound | What today shows instead | Deciding command |
|---|---|---|
| **O-B7** | `sin(x)`, `sqrt(x)`, `diff(sin(x),x)` all report `exact:true` on the wire and `exact: True` through `inspect` | `--file %T%\b7_sin.ls`, `%T%\insp_sin.ls` |
| **O-B8a** | every error envelope carries `elapsedTime` and `timings` | `%T%\err_div.ls`, `%T%\err_parse.ls`, `%T%\err_name.ls`, `%T%\b1_a.ls` |
| **O-B8b** | the 246-call sweep produced zero `InvalidOperation/DomainError` rows (112+119 `InvalidArgument/TypeMismatch`); one `InternalError/InternalInvariantFailure` remains on `reshape(1,1,1,1,1,1,1,1,1)` | `%T%\sweep.ps1` |
| **O-B8c** | `output` values are clean; the only CR is the terminal CRLF | `%T%\b8_print.ls` + `Format-Hex` |
| **O-B8d** | `variables[]` entries carry `kind`, `display` and `structured` | `%T%\b8_vars2.ls` |
| **O-B8e** | `divrem` returns a `DivRemResult` record | `%T%\b8_divrem.ls` |
| **O-B8f** | `x^2` at `--print-budget 1` is truncated with `truncationReason:node-budget` | `%T%\pb_x2.ls --print-budget 1` |
| **O-B8g** | `evalf(sqrt(2), 5)` → `1.41421`; `evalf(sqrt(2), 30)` → 30 digits | `%T%\ev_sqrt5.ls`, `%T%\ev_sqrt30.ls` |
| **O-B8h** | `assumptions()` returns an `AssumptionSet` record | `%T%\b8_assum.ls` |
| **O-B8i** | the 123-entry registry publishes `minArity` and `variadic` per entry | `--eval "1" --json --omit-variables` |
| **O-B10 (second half)** | `setprecision(18); 1/1009` comes back `exact:false` (ground truth: the digits are right) | `%T%\b10b.ls` |

## 2. Bounds that are STILL TRUE (candidates for the maintainer to accept)

| Bound | One line of evidence |
|---|---|
| O-B1 | `2^(1/2)` / `2^(1/3)` → `UnsupportedOperation`; capability `pow.non-integer-exponent` still advertised; `solve_full(x^3-2==0,x)` still prints `2^(1/3)` |
| O-B2 | `(-8)^(1/3)` → the same typed refusal; capability `pow.negative-base-unrepresentable-exponent` still advertised |
| O-B3 | `solve_full(x^4-x^2-1==0,x)` → `Partial`, `unrepresented_count 2`, `solve.unrepresented-roots`; SymPy confirms 2 of the 4 roots are real |
| O-B4 | complex `log` returns the principal value only, matching mpmath to all 30 printed digits |
| O-B5 | the printed `rootof(...)` text still fails to re-parse (message today: `Undefined variable 'x'.`) |
| O-B6 | `1/2*sqrt(8)` is a numeric truncation with `exact:false` (the digits match mpmath) |
| O-B8f (vector shape) | `[1..10]` at `--print-budget 3` is returned in full with no truncation fields ❓ |
| O-B9 | `sqrt(-1)` prints `i`; re-feeding `i` gives `Undefined variable 'i'.`; `(-4)^(1/2)` prints `2*i` |
| O-B10 (first half) | `1/(3*10^1000)` returns `"value":"0"` where mpmath gives `3.333333333e-1001` |
| O-B11 | recursion accepted through N=432 and fatal (`0xC00000FD`, 0 bytes on stdout) from N=436 |
| O-B12 (partially) | the machine is not idle during the probe (12 `dotnet` processes; `VBCSCompiler` 561.56 s CPU) ❓ |
| O-B13 | not re-probed; CI state looked up: #29 `success` with all three jobs green |
| O-B14 | `exactness` is `BestEffort`; 16 advertised entries; `(-4)^(1/2)` still works while non-integer exponents are advertised unsupported |

## 3. Bounds that could NOT be measured (with the reason)

| Bound | Reason (observed) |
|---|---|
| **O-B12** (the project's benchmark rows themselves) | the BenchmarkDotNet harness cannot generate its project in this workspace: `Found more than one matching project file for symbench in C:\Users\ricar\dev\LovelaceSharp and its subfolders` — 53 `symbench.csproj` copies exist under `.worktrees\`. Four attempts (repo root, project directory, `dotnet run --project`, `symbench.dll` directly) all ended `executed benchmarks: 0` with `NA` means. The surrogate 12-run runner timing spread (`RANGE=6.87 ms`) is a different instrument ❓. |
| **O-B13** | deliberately not re-probed (the document records it CLOSED); only the CI state was looked up. ❓ Whether run #30 completed green after the fetch. |
| **O-B8f (vector shape)** | measured (full value, no truncation fields) but whether that shape is what the bound means by "values already over budget" was not established ❓. |

*Every "Still true today?" cell above is a statement about the probe that precedes it in the same row; ❓ marks an inference rather than an observation.*
