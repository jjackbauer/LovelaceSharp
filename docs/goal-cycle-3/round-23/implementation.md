# Round 23 — `abs()` accepts a symbolic argument (round-trip closure)

**Goal.** `x = symbol("x"); abs(x)` and `diff(abs(x), x)` failed with
`InvalidOperation/DomainError "abs() is not supported for values of kind 'Symbolic'."` while the
rewriter itself *produces* `abs(x)` for `simplify_full(sqrt(x^2))` under `x` real. A value the
engine creates must be expressible back into it.

**Status:** fix implemented, all suites run, runner probes run (evidence below).

---

## 0. What node the rewriter actually builds (this decides the fix)

There is **no `Absolute` node kind** — `Lovelace.Symbolics/Expr.cs:8-13` lists
`Symbol, IntegerConstant, RationalConstant, RealConstant, ComplexConstant, NamedConstant, Add,
Multiply, Power, Function, Relation, Piecewise, Derivative, Integral, RootOf, And, Or, Not, Order`.

The rewriter builds a **plain function application** — a `FunctionExpr` over the registered kernel
function `"abs"`:

| site | code |
| --- | --- |
| `Lovelace.Symbolics/Simplify.cs:251` | `(m, c) => Exprs.Function(c.Function("abs"), m.Get("w2"))` — the `pow.sqrt-square-real` rule |
| `Lovelace.Symbolics/Simplify.cs:320-321` | rule `abs^2`, pattern `FunPat("abs", ...)` |
| `Lovelace.Symbolics/Simplify.cs:332-335` | rule `abs(-u) -> abs(u)` |
| `Lovelace.Symbolics/Functions.cs:107-108` | the same definition is registered in the kernel: `Add("abs", 1, null, (args, c) => NumOps.Abs(args[0], c))` |

Confirmed end-to-end by probe **p5** (structured payload of the TransformResult):

```json
{"name":"expression","value":{"kind":"Symbolic","pretty":"abs(x)","canonical":"(fn abs (sym x))",
 "domain":"real","exact":true,"nodeCount":2,"freeSymbols":["x"]}}
```

So the fix is **"stop rejecting the symbolic payload and build the same node the rewriter builds"** —
not "add a new node kind". No `NodeKind` was added.

---

## 1. Step A — failing output BEFORE the fix (test-first)

New tests: `Lovelace.Symbolics.Tests/AbsSymbolicRoundTripTests.cs` (language level, through
`SuiteEngine` + `SymbolicsPlugin`, never the kernel API — the defect is at the language boundary).

```
dotnet test Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.csproj -c Release --nologo --filter FullyQualifiedName~AbsSymbolicRoundTripTests
```

```
  Failed Lovelace.Symbolics.Tests.AbsSymbolicRoundTripTests.Abs_Of_Symbol_Matches_The_Rewriters_Own_Abs_Node [1 ms]
  Error Message:
   System.InvalidOperationException : abs() is not supported for values of kind 'Symbolic'.
  Failed Lovelace.Symbolics.Tests.AbsSymbolicRoundTripTests.Diff_Of_Abs_Symbolic_Does_Not_Throw [< 1 ms]
  Error Message:
   System.InvalidOperationException : abs() is not supported for values of kind 'Symbolic'.
  Failed Lovelace.Symbolics.Tests.AbsSymbolicRoundTripTests.Subs_Abs_Symbolic_At_Negative_Value_Gives_Three [< 1 ms]
  Error Message:
   System.InvalidOperationException : abs() is not supported for values of kind 'Symbolic'.
Failed!  - Failed:     3, Passed:     2, Skipped:     0, Total:     5, Duration: 61 ms - Lovelace.Symbolics.Tests.dll (net10.0)
```

The two that already passed are exactly the asymmetry the defect describes:
`Abs_Of_Integer_Still_Returns_The_Magnitude` (numeric path fine) and
`Simplify_Sqrt_Square_Under_Real_Is_Still_Abs_X` (the rewriter already produces `abs(x)`).

Stack of the rejection (first run, before the fix):

```
System.InvalidOperationException : abs() is not supported for values of kind 'Symbolic'.
   at Lovelace.Suite.Interpreter.<>c.<RegisterBuiltins>b__140_2(IReadOnlyList`1 args) in C:\Users\ricar\dev\LovelaceSharp\Lovelace.Suite\Interpreter.cs:line 1275
   at Lovelace.Suite.Interpreter.EvaluateCallAsync(CallExpr call, Scope scope) in ...\Lovelace.Suite\Interpreter.cs:line 615
```

### Correction to the brief: the rejection is NOT in the plugin

The brief said "the rejection is in the plugin's symbolic path". It is not.
`grep -n "\babs\b" Lovelace.Symbolics/SymbolicsPlugin.cs` returns **nothing**: the plugin never
registers `abs` (its elementary list is `exp, log, sin, cos, tan, asin, acos, atan, sinh, cosh,
tanh, asinh, acosh, atanh` — `SymbolicsPlugin.cs:56-58`). The throwing switch is the core builtin
`Register("abs", ...)` in `Lovelace.Suite/Interpreter.cs:1264-1277`, and the runner's own function
table confirms it — `{"name":"abs","parameters":["x"],"builtin":true}` with **no** `"plugin"` field,
while every symbolic builtin carries `"plugin":"Lovelace.Symbolics"`.

Plugin-side shadowing is not a viable fix: `c.RegisterBuiltin` overwrites `_functions[name]`
(`Interpreter.cs:196-203`), so a plugin `abs` would replace the numeric one for *every* kind.
`Lovelace.Symbolics` does not reference `Lovelace.Suite` (the dependency runs the other way), so the
plugin cannot rebuild the array/Vector result of `AbsArray` (`Interpreter.cs:1235-1245`, the
`abs(fft(...))` magnitude idiom), and plugin builtins additionally run inside
`ModusHost.PluginPrecisionScope()` (30/15 digits) which would silently change numeric `abs` for
reals. See "Scope deviation" below.

---

## 2. The fix (one bounded change)

`Lovelace.Suite/Interpreter.cs`, core `abs` builtin — one arm added, mirroring the in-file
precedent `sqrt` (`Interpreter.cs:1443-1448`, which already takes a `ValueKind.Symbolic` arm):

```csharp
                ValueKind.Vector or ValueKind.Array => AbsArray(arg.AsArrayValue()),
                // Round 23: a value the engine CREATES must be expressible back into it. The
                // rewriter emits abs(x) as a FunctionExpr over the registered "abs" kernel
                // function (Lovelace.Symbolics/Simplify.cs:251, :321, :335 — there is no
                // Absolute node kind), so the symbolic payload builds that very node instead of
                // being rejected. Mirrors the sqrt() symbolic arm below (Exprs.Power): the
                // kernel's own constructor, no wrapper node, no new NodeKind.
                ValueKind.Symbolic => new Value(Lovelace.Symbolics.Exprs.Function(
                    Lovelace.Symbolics.Exprs.Current.Function("abs"), arg.AsSymbolic())),
                _ => throw new InvalidOperationException($"abs() is not supported for values of kind '{arg.Kind}'."),
```

Why this is exactly the rewriter's node: `Exprs.Function(FunctionId, args)` is the single public
constructor the rewriter itself calls; `FunctionId` equality is **by canonical name**
(`Context.cs:24-37`) and `FunctionExpr.Equals` compares function name + arguments
(`Expr.cs:312-314`), so the node built here is structurally identical to
`ctx.Function("abs")`-built nodes from any context.

**No change to `Lovelace.Symbolics` was needed at all** — not to `SymbolicsPlugin.cs` and not to
`Calculus/Diff.cs`. `Diff.cs:84-93` already differentiates `abs` correctly; the only reason
`diff(abs(x), x)` failed was that the argument expression could never be built.

---

## 3. What `diff(abs(x), x)` returns now, and why that is honest

Probe **p3** (`x = symbol("x"); diff(abs(x), x)`, runner result):

```json
{"kind":"Symbolic",
 "display":"piecewise(sign(x) if x != 0, diff(abs(x), x))",
 "structured":{"kind":"Symbolic","pretty":"piecewise(sign(x) if x != 0, diff(abs(x), x))",
 "canonical":"(pw ((ne (sym x) (rat 0 1)) (fn sign (sym x))) (der (x) (fn abs (sym x))))",
 "domain":"complex","exact":true,"nodeCount":10,"freeSymbols":["x"]}}
```

That node is built by the kernel's own `Calculus/Diff.cs:84-93`:
`piecewise(x != 0 -> sign(x), Derivative(abs(x), x))` times `d(x)/dx = 1` (folded away by
canonical multiply).

Why this is the mathematically honest answer, not a dodge:

* `d|x|/dx = sign(x)` holds **only for x ≠ 0**; at 0 the derivative does not exist (the left and
  right difference quotients are −1 and +1). Returning a bare `sign(x)` would claim a value at 0
  that does not exist; returning `1` or `0` would be wrong on a whole half-line; `x/abs(x)` is the
  same function as `sign(x)` and equally undefined at 0.
* The kernel therefore keeps the guard explicit (`x != 0`) and, on the branch where the derivative
  is undefined, returns the **unevaluated derivative node** rather than inventing a value — the same
  policy `Functions.cs:103-106` documents for `sign/floor/ceil`.
* It is not a wrong closed form: it evaluates to `sign(x)` where the derivative exists, and to
  `diff(abs(x), x)` (honest "not known here") exactly where it does not.
* The test asserts the exact pretty and canonical strings, and explicitly asserts the result is
  neither `1` nor a bare `sign(x)` (`AbsSymbolicRoundTripTests.Diff_Of_Abs_Symbolic_Does_Not_Throw`).

---

## 4. Step C — passing output AFTER the fix

```
dotnet test Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.csproj -c Release --nologo --filter FullyQualifiedName~AbsSymbolicRoundTripTests
```

```
Passed!  - Failed:     0, Passed:     5, Skipped:     0, Total:     5, Duration: 65 ms - Lovelace.Symbolics.Tests.dll (net10.0)
```

Full verification (raw log: `docs/goal-cycle-3/round-23/probes/verify.log`) — pasted in section 5.

---

## 5. Verification commands and observed output

Raw log of all three commands: `docs/goal-cycle-3/round-23/probes/verify.log` (UTF-8, full output).

```
dotnet build LovelaceSharp.slnx -c Release --nologo
```

```
Build succeeded.
    0 Warning(s)
    0 Error(s)
Time Elapsed 00:00:02.34
```

```
dotnet test Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.csproj -c Release --nologo
```

```
Passed!  - Failed:     0, Passed:   461, Skipped:     6, Total:   467, Duration: 13 s - Lovelace.Symbolics.Tests.dll (net10.0)
```

(The 6 skips are pre-existing skip attributes in that suite — none of them is in
`AbsSymbolicRoundTripTests.cs`, which reports 5 passed / 0 skipped.)

```
dotnet test Lovelace.Suite.Tests/Lovelace.Suite.Tests.csproj -c Release --nologo
```

```
Passed!  - Failed:     0, Passed:   637, Skipped:     0, Total:   637, Duration: 8 s - Lovelace.Suite.Tests.dll (net10.0)
```

All suites end 0 failed.

---

## 6. Runner probes (built Release binary, `Lovelace.Run`)

Probe scripts: `docs/goal-cycle-3/round-23/probes/p1..p6*.ls`.
Command shape: `dotnet Lovelace.Run/bin/Release/net10.0/Lovelace.Run.dll <probe>.ls`.

```
### p1_abs_integer.ls              abs(-3)
ok=true kind=Integer display=3

### p2_abs_symbol.ls               x = symbol("x"); abs(x)
ok=true kind=Symbolic pretty=abs(x) canonical=(fn abs (sym x))

### p3_diff_abs_symbol.ls          x = symbol("x"); diff(abs(x), x)
ok=true kind=Symbolic pretty=piecewise(sign(x) if x != 0, diff(abs(x), x)) canonical=(pw ((ne (sym x) (rat 0 1)) (fn sign (sym x))) (der (x) (fn abs (sym x))))

### p4_subs_abs_symbol.ls          x = symbol("x"); subs(abs(x), x, -3)
ok=true kind=Symbolic pretty=3 canonical=(rat 3 1)

### p5_simplify_sqrt_square_real.ls   x = symbol("x", real); simplify_full(sqrt(x^2))
ok=true kind=Record display=TransformResult(status: Satisfied, original: sqrt(x^2), expression: abs(x), changed: True,
  conditions: [DomainCondition(variable: x, domain: real)],
  steps: [RewriteStep(rule_id: pow.sqrt-square-real, classification: DomainSpecific, before: sqrt(x^2), after: abs(x),
  required_conditions: [DomainCondition(variable: x, domain: real)])], budget_exceeded: False, budget_kind: , diagnostics: [])

### p6_roundtrip_equality.ls        (the round-trip itself, in-language)
output:
Inspection(type: Symbolic, domain: real, exact: True, free_symbols: [x], node_count: 2, canonical: (fn abs (sym x)), pretty: abs(x), shape: , rank: , element_domain: real, assumptions: [DomainCondition(variable: x, domain: real)], members: )
Inspection(type: Symbolic, domain: real, exact: True, free_symbols: [x], node_count: 2, canonical: (fn abs (sym x)), pretty: abs(x), shape: , rank: , element_domain: real, assumptions: [DomainCondition(variable: x, domain: real)], members: )
3
3
piecewise(sign(x) if x != 0, diff(abs(x), x))
```

p6 is the acceptance evidence for requirement 1: `inspect(abs(x))` and
`inspect(simplify_full(sqrt(x^2)).expression)` agree on **every** field —
`canonical: (fn abs (sym x))`, `node_count: 2`, `domain: real`, `assumptions: [DomainCondition(variable: x, domain: real)]`.
Requirement 5 is visible in the same probe: the rewrite still fires `pow.sqrt-square-real` and still
yields `abs(x)`.

Test-level round-trip assertion (`Abs_Of_Symbol_Matches_The_Rewriters_Own_Abs_Node`) asserts
three things, not the printed string alone: pretty form `abs(x)`; canonical form of `abs(x)` equals
the canonical form of the rewriter's output; and structural equality with
`Exprs.Function(Exprs.Current.Function("abs"), Exprs.Current.Symbol("x"))`.

---

## 7. Scope deviation (explicit)

The brief scoped the change to `Lovelace.Symbolics/SymbolicsPlugin.cs` "(the abs builtin path)".
That premise does not hold: the plugin does not register `abs` at all (`SymbolicsPlugin.cs:56-58`
lists the elementary functions; `abs` is not among them and there is no second registration path
for it), and the rejecting code is the core builtin at `Lovelace.Suite/Interpreter.cs:1264-1277`,
whose stack frame the failing tests print. The one bounded change therefore lands there, directly
beside the `sqrt` symbolic arm (`Interpreter.cs:1443-1448`) that already solves the identical
problem for `sqrt`. Every argument for why the plugin cannot carry this fix is in section 1.
Nothing else was touched: no new `NodeKind`, no change to `Lovelace.Symbolics`, no test deleted or
weakened, no `git commit`.

## 8. Not done / open

* `subs(abs(x), x, -3)` returns an **exact Symbolic 3** (`pretty "3"`, `canonical "(rat 3 1)"`,
  `freeSymbols []`), not an `Integer` 3. The value is 3 — printing, canonical form and structure all
  agree — but it stays on the symbolic side of the payload boundary. Not changed: converting back to
  a numeric kind is a separate (type-coercion) question and is out of scope here. The test asserts
  the exact symbolic value rather than pretending the kind is `Integer`.
* Complex `abs` on a symbolic complex expression (e.g. `abs(symbol("z", complex))`) builds the same
  function node — the kernel has no complex-modulus rewrite for it, so it stays unevaluated; no
  behaviour was claimed for it (out of scope per the brief).
* The literal instruction "edit `SymbolicsPlugin.cs`" was not followed because the defect is not
  there (sections 1 and 7); the equivalent, non-regressing fix is in the core builtin.
