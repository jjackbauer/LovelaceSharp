# Round 10 — `capabilities()` made a truthful, exhaustive statement of what the kernel refuses

Cycle 4, round 10. Ground truth for the defect: `docs/goal-cycle-4/round-09/audit-P2P6-wire.md`,
part **6(b)** ("operations the kernel REFUSES that the statement does NOT list"). The pre-change
behaviour of the eight classes is also captured independently by the orchestrator in
`docs/goal-cycle-4/round-10/pre-change-baseline.txt` (published AOT binary of `8b47175`).

**Result: 8 of 8 audit classes are now covered and honest.** Seven were advertised directly from
live output; two of those (P6b-2/P6b-3 and P6b-4) first had to be made *advertisable* — the
integration refusal carried no code and an empty `diagnostics` array, and the symbolic plot path
died as an internal invariant failure. `exactness` stays **`BestEffort`**, on evidence, with the
base of what is enumerated and the exact list of what is not written into the record's own comment
(§7).

Everything below was observed on the **published Native AOT binary**
`out/aot/Lovelace.Run.exe`, rebuilt in this round from the working tree
(`dotnet publish Lovelace.Run/Lovelace.Run.csproj -c Release -p:PublishAot=true
-p:InvariantGlobalization=true -o out/aot`, exit 0). The golden fixture was regenerated from that
same binary, never hand-edited.

## 1. Files changed (all inside the round's scope)

| file | change |
|---|---|
| `Lovelace.Symbolics/Calculus/Integrate.cs` | a refused integration now always carries a reason (typed refusal), distinguishing "no tier produced a candidate" from "the candidate failed verification" |
| `Lovelace.Symbolics/SymbolicsPlugin.cs` | the integration refusal crosses as a structured `integration.unevaluated` diagnostic with class-level message + `details`; the capability record grew from 4 to 16 entries; the `exactness` basis comment was rewritten |
| `Lovelace.Suite/Interpreter.cs` | `plot()` checks its argument kinds before casting them (the refusal is produced there); the symbolic plot path is a typed `InvalidOperation`/`DomainError` |
| `Lovelace.Symbolics.Tests/CapabilitiesBuiltinTests.cs` | every advertised entry is now live-verified together with the field that CARRIES the refusal; the enumeration is pinned as a set; `BestEffort` is backed by live unlisted refusals; two focused tests for the classes that had to be repaired |
| `Lovelace.Run.Tests/fixtures/capabilities.json` | regenerated from the AOT binary (§5) |

No file outside the scope was touched. `Lovelace.Real/**`, `dsh-protocol.md` and every other test
file are untouched (`git status` shows only the five files above plus the two round-10 artefacts
owned by the orchestrator).

## 2. The two refusals that had to be repaired before they could be advertised

### 2.1 P6b-2 / P6b-3 — unsupported integration (was: `Unevaluated` with NO diagnostic)

Audit: `integrate_full(exp(x^2), x)` returned `status = Unevaluated` with
`diagnostics = {"kind":"Array","shape":[0],"elements":[]}` — nothing to transcribe, and "did
anything go wrong?" was not answerable from `diagnostics`, contradicting `dsh-protocol.md:93-94`.

Before (orchestrator baseline, old AOT binary):

```
=== P6b-2 integrate   exit=0
  script: x = symbol("x"); integrate_full(exp(x^2), x)
  ok kind=Record type=IntegrationResult
  field status = {"kind":"Enum","type":"IntegrationStatus","value":"Unevaluated"}
  field diagnostics = {"kind":"Array","type":"Vector","shape":[0],"elements":[]}
```

After (this round, AOT binary). The kernel now always supplies a reason for an unevaluated
integral, and the plugin projects it as a structured refusal whose **outer message is class-level**
(one sentence, identical for every refused integrand — so one advertised message is true for the
whole class) with the **input-specific reason in `details`**:

```json
{"name":"diagnostics","value":{"kind":"Array","type":"Vector","shape":[1],"elements":[
 {"kind":"Record","type":"Diagnostic","fields":[
  {"name":"code",     "value":{"kind":"Text","value":"integration.unevaluated"}},
  {"name":"category", "value":{"kind":"Enum","type":"ErrorCategory","value":"UnsupportedOperation"}},
  {"name":"message",  "value":{"kind":"Text","value":"No closed form was found, so the integral is returned unevaluated; the reason for the refusal is carried in this diagnostic's details."}},
  {"name":"recoverable","value":{"kind":"Boolean","value":"true"}},
  {"name":"location", "value":{"kind":"Null"}},
  {"name":"details",  "value":{"kind":"Array","type":"Vector","shape":[1],"elements":[
   {"kind":"Record","type":"Diagnostic","fields":[
    {"name":"code",     "value":{"kind":"Text","value":"integration.no-closed-form"}},
    {"name":"category", "value":{"kind":"Enum","type":"ErrorCategory","value":"UnsupportedOperation"}},
    {"name":"message",  "value":{"kind":"Text","value":"no integration tier produced a candidate closed form for integrate(exp(x^2), x)"}},
    {"name":"recoverable","value":{"kind":"Boolean","value":"true"}},
    {"name":"location", "value":{"kind":"Null"}},
    {"name":"details",  "value":{"kind":"Array","type":"Vector","shape":[0],"elements":[]}}]}]}}]}]}}
```

Code choice: `integration.unevaluated` — the string the kernel already intended (`IntegrationDiagnostic`
carried it since round 4, unreachable because `IntegrationResult.Unevaluated` never set a `Note`).
It matches the sibling vocabulary (`solve.unevaluated`, `limit.unevaluated`,
`system-solve.unevaluated`); the brief's `integrate.unevaluated` was explicitly an example ("e.g.").
The message is a shared constant (`SymbolicsPlugin.IntegrationUnevaluatedMessage`) used by both the
diagnostic and the advertisement, so the two cannot drift.

This is also the first non-empty `Diagnostic.details` array the runtime emits — the audit had
recorded "a non-empty `details` array … never observed" as untestable. The element form is the same
six-field `Diagnostic`, which is what `DiagnosticProjection` can produce; no protocol shape changed.

Two guards against regression: the message is one shared constant
(`SymbolicsPlugin.IntegrationUnevaluatedMessage`) used by both the diagnostic and the advertisement,
so they cannot drift; and `IntegrationResult.Unevaluated(Expr, string note)` now **requires** the
reason (the parameter is not optional), so a silent refusal cannot come back by omission.

### 2.2 P6b-4 — symbolic `plot` (was: `InternalInvariantFailure`, `recoverable:false`)

Audit: `plot(sin(x))` → exit 1, `InternalError` / `InternalInvariantFailure`, "Specified cast is not
valid.", no file written. Root cause: `BuiltinPlot`'s one-argument branch called
`BuildIndexVector(ys)` — which does `ys.AsVector()` — **before** the vector-kind check that sat
after the `switch`. A `Symbolic` argument therefore threw `InvalidCastException`. `plot(1)` had the
same defect (never probed by the audit; found while fixing it).

Decision: **not** to invent a sampling range to "make it work" (a silently wrong plot is worse than
a refusal, and no range is specified anywhere), but to make it fail like every other argument error.
The kind check now happens first, for every argument position:

```
plot() argument 1 must be a vector, but got 'Symbolic'.
```

A second, distinct refusal in the same family was found while probing: a vector whose **elements**
are symbolic (`plot([x, 1, 2])`) is refused by the plot value conversion with
`Cannot convert value of kind 'Symbolic' to a number for plotting.` — already typed, merely
unlisted, so it is advertised too (class `plot.symbolic-element`).

## 3. The eight classes: live output of the AOT binary next to the live advertised entry

Every row below was produced with
`out\aot\Lovelace.Run.exe --file <script> --omit-functions --omit-variables`.
"Advertised" is the element of `capabilities().result.structured.fields[unsupported_operations]`
(see §5 for the complete list).

### P6b-1 — symbolic limit that cannot be determined → **ADVERTISED** (record-carried)

```
live: x = symbol("x"); limit_full(sin(x), x, inf);
exit 0, ok=true, result.structured.type=LimitResult, status=Unevaluated, diagnostics.shape=[1]
  diagnostic.code = limit.unevaluated
  diagnostic.category = UnsupportedOperation
  diagnostic.message = coefficient does not evaluate at the point
  diagnostic.recoverable = true
  diagnostic.location =            (Null)
  diagnostic.details = shape=[0]

advertised: operation_class=limit.unevaluated-in-record-diagnostics
            code=limit.unevaluated  category=UnsupportedOperation
            message="coefficient does not evaluate at the point"
            trigger="x = symbol(\"x\"); limit_full(sin(x), x, inf)"
```

The refusal rides in the `LimitResult`'s `diagnostics` (the call itself succeeds), so the class name
says so, and the honesty test asserts exactly that carrier: exit 0, a Record result, a `status` enum
that is a **refusal** member (`Unevaluated`), and `diagnostics[0]` equal to the advertisement.
`limit.unevaluated` has more than one message — `limit_full(a/x, x, 0)` produces
"leading coefficient is symbolic" (already pinned by `DiagnosticContractTests.KernelNotes_CrossAsDiagnosticMessages`)
— so the code/category are class-level and the message is the one the advertised trigger produces;
the record's comment states this explicitly rather than pretending one message covers all inputs.

### P6b-2 / P6b-3 — unsupported integration → **ADVERTISED** (record-carried, after the kernel fix)

```
live: x = symbol("x"); integrate_full(exp(x^2), x);
exit 0, ok=true, result.structured.type=IntegrationResult, status=Unevaluated, diagnostics.shape=[1]
  diagnostic.code = integration.unevaluated
  diagnostic.category = UnsupportedOperation
  diagnostic.message = No closed form was found, so the integral is returned unevaluated; the reason for the refusal is carried in this diagnostic's details.
  diagnostic.details = shape=[1]
    details[0].code = integration.no-closed-form
    details[0].category = UnsupportedOperation
    details[0].message = no integration tier produced a candidate closed form for integrate(exp(x^2), x)

advertised: operation_class=integration.unevaluated-in-record-diagnostics
            code=integration.unevaluated  category=UnsupportedOperation
            message="No closed form was found, so the integral is returned unevaluated; the reason for the refusal is carried in this diagnostic's details."
            trigger="x = symbol(\"x\"); integrate_full(exp(x^2), x)"
```

### P6b-4 — symbolic `plot` → **ADVERTISED** (after the kernel fix)

```
live: x = symbol("x"); plot(sin(x));
exit 1, ok=false, code=InvalidOperation, category=DomainError, recoverable=true
  message = plot() argument 1 must be a vector, but got 'Symbolic'.

advertised: operation_class=plot.symbolic-expression
            code=InvalidOperation  category=DomainError
            message="plot() argument 1 must be a vector, but got 'Symbolic'."
            trigger="x = symbol(\"x\"); plot(sin(x))"

live (same family, also advertised): x = symbol("x"); plot([x, 1, 2]);
exit 1, ok=false, code=InvalidOperation, category=DomainError, recoverable=true
  message = Cannot convert value of kind 'Symbolic' to a number for plotting.
```

### P6b-5 — non-symbolic solve variable → **ADVERTISED** (plus the four sibling surfaces the audit named)

```
live: x = symbol("x"); solve(x^2 - 2 == 0, 1);
exit 1, ok=false, code=InvalidOperation, category=DomainError, recoverable=true
  message = solve(): argument 2 must be a symbolic variable; got Natural.

advertised: solve.non-symbolic-variable | InvalidOperation | DomainError | "solve(): argument 2 must be a symbolic variable; got Natural."
```

The audit's row also names `diff`, `integrate`, `limit` with `1` as the variable and
`solve([x == 1], x)`; all four were probed live and each is advertised as its own class, because the
message names the builtin and the payload kind:

```
diff(x^2, 1)        → InvalidOperation/DomainError  "diff(): argument 2 must be a symbolic variable; got Natural."
integrate(x^2, 1)   → InvalidOperation/DomainError  "integrate(): argument 2 must be a symbolic variable; got Natural."
limit(sin(x), 1, 0) → InvalidOperation/DomainError  "limit(): argument 2 must be a symbolic variable; got Natural."
solve([x == 1], x)  → InvalidOperation/DomainError  "solve(): argument 1 must be a symbolic expression; got Vector."
```

### P6b-6 — symbolic elements refused by DSP builtins → **ADVERTISED**

```
live: x = symbol("x"); dft([x, 1, 2, 3]);
exit 1, ok=false, code=InvalidOperation, category=DomainError, recoverable=true
  message = DSP builtins expect numeric/complex elements, but got 'SymbolExpr'.

advertised: dsp.symbolic-element | InvalidOperation | DomainError
            "DSP builtins expect numeric/complex elements, but got 'SymbolExpr'."
```

The message leaks the kernel's internal CLR type name (`SymbolExpr`). `Lovelace.Dsp` is outside this
round's scope, so it is transcribed as observed rather than reworded — the same treatment the FFT
artefact below gets.

### P6b-7 — numeric matrix refused by `linsolve` → **ADVERTISED**

```
live: A = [[1,2],[2,4]]; b = [[1],[3]]; linsolve(A, b);
exit 1, ok=false, code=InvalidOperation, category=DomainError, recoverable=true
  message = linsolve() requires a symbolic matrix A.

advertised: linsolve.non-symbolic-matrix | InvalidOperation | DomainError
            "linsolve() requires a symbolic matrix A."
```

### P6b-8 — FFT length constraint → **ADVERTISED**

```
live: x = symbol("x"); fft([1,2,3]);
exit 1, ok=false, code=InvalidArgument, category=TypeMismatch, recoverable=true
  message = FFT length must be a power of two, but got 3. (Parameter 'x')

advertised: fft.non-power-of-two-length | InvalidArgument | TypeMismatch
            "FFT length must be a power of two, but got 3. (Parameter 'x')"
```

The message carries the .NET `ArgumentException` artifact `(Parameter 'x')` and the offending
length, both transcribed as observed (`Lovelace.Dsp` again outside scope). Because the length is in
the message, this class is trigger-level in its message, like the argument guards.

## 4. Machine check over the whole enumeration

Run against the AOT binary (script: run `capabilities()`, then each advertised `trigger`, compare
the advertised `code`/`category`/`message` with the field that actually carries the refusal):

```
AOT capabilities(): exactness=BestEffort entries=16
OK  pow.non-integer-exponent                             error envelope
OK  pow.negative-base-unrepresentable-exponent           error envelope
OK  solve.unsupported-domain                             error envelope
OK  rootof.complex-algebraic                             record SolveResult status=Partial diagnostics[0]
OK  limit.unevaluated-in-record-diagnostics              record LimitResult status=Unevaluated diagnostics[0]
OK  integration.unevaluated-in-record-diagnostics        record IntegrationResult status=Unevaluated diagnostics[0]
OK  plot.symbolic-expression                             error envelope
OK  plot.symbolic-element                                error envelope
OK  solve.non-symbolic-variable                          error envelope
OK  solve.non-symbolic-expression                        error envelope
OK  diff.non-symbolic-variable                           error envelope
OK  integrate.non-symbolic-variable                      error envelope
OK  limit.non-symbolic-variable                          error envelope
OK  dsp.symbolic-element                                 error envelope
OK  linsolve.non-symbolic-matrix                         error envelope
OK  fft.non-power-of-two-length                          error envelope
AOT MISMATCHES=0
```

`rootof.complex-algebraic` is the one record-carried class that does **not** carry the
`-in-record-diagnostics` marker: it predates this round and its published `operation_class` id is
deliberately not renamed (consumers enumerate on it; audit P6a-3 recorded that the advertisement
"does not claim which" carrier). The marker is therefore a **guarantee the marked classes make**,
not a partition of the whole list — and the honesty test reports the carrier it observed for every
entry, marked or not. Asserting the carrier rather than assuming it is what the test does for all 16.

## 5. The final capability list, verbatim

From `out\aot\Lovelace.Run.exe --file caps.ls --omit-functions --omit-variables` (`caps.ls` is
`capabilities();`). Scalars: `supported_domains = [real, complex]`,
`unsupported_domains = [integer, rational]`,
`exactness = Enum(CapabilitiesExactness, "BestEffort")`, `unsupported_operations.shape = [16]`.

The record's own verbatim rendering (`result.display`, 3989 chars):

```text
CapabilitiesResult(supported_domains: [real, complex], unsupported_domains: [integer, rational], unsupported_operations: [UnsupportedCapability(operation_class: pow.non-integer-exponent, code: UnsupportedOperation, category: UnsupportedOperation, message: Non-integer exponents are not yet supported., trigger: 2^(1/2)), UnsupportedCapability(operation_class: pow.negative-base-unrepresentable-exponent, code: UnsupportedOperation, category: UnsupportedOperation, message: Non-integer exponents are not yet supported., trigger: (-8)^(1/3)), UnsupportedCapability(operation_class: solve.unsupported-domain, code: InvalidOperation, category: DomainError, message: solve(): currently supports domains real and complex; got integer., trigger: x = symbol("x"); solve(x^2 - 2 == 0, x, integer)), UnsupportedCapability(operation_class: rootof.complex-algebraic, code: solve.unrepresented-roots, category: UnsupportedOperation, message: complex algebraic roots not supported (RootOf is real-only in v1)., trigger: x = symbol("x"); solve_full(x^4 - x^2 - 1 == 0, x)), UnsupportedCapability(operation_class: limit.unevaluated-in-record-diagnostics, code: limit.unevaluated, category: UnsupportedOperation, message: coefficient does not evaluate at the point, trigger: x = symbol("x"); limit_full(sin(x), x, inf)), UnsupportedCapability(operation_class: integration.unevaluated-in-record-diagnostics, code: integration.unevaluated, category: UnsupportedOperation, message: No closed form was found, so the integral is returned unevaluated; the reason for the refusal is carried in this diagnostic's details., trigger: x = symbol("x"); integrate_full(exp(x^2), x)), UnsupportedCapability(operation_class: plot.symbolic-expression, code: InvalidOperation, category: DomainError, message: plot() argument 1 must be a vector, but got 'Symbolic'., trigger: x = symbol("x"); plot(sin(x))), UnsupportedCapability(operation_class: plot.symbolic-element, code: InvalidOperation, category: DomainError, message: Cannot convert value of kind 'Symbolic' to a number for plotting., trigger: x = symbol("x"); plot([x, 1, 2])), UnsupportedCapability(operation_class: solve.non-symbolic-variable, code: InvalidOperation, category: DomainError, message: solve(): argument 2 must be a symbolic variable; got Natural., trigger: x = symbol("x"); solve(x^2 - 2 == 0, 1)), UnsupportedCapability(operation_class: solve.non-symbolic-expression, code: InvalidOperation, category: DomainError, message: solve(): argument 1 must be a symbolic expression; got Vector., trigger: x = symbol("x"); solve([x == 1], x)), UnsupportedCapability(operation_class: diff.non-symbolic-variable, code: InvalidOperation, category: DomainError, message: diff(): argument 2 must be a symbolic variable; got Natural., trigger: x = symbol("x"); diff(x^2, 1)), UnsupportedCapability(operation_class: integrate.non-symbolic-variable, code: InvalidOperation, category: DomainError, message: integrate(): argument 2 must be a symbolic variable; got Natural., trigger: x = symbol("x"); integrate(x^2, 1)), UnsupportedCapability(operation_class: limit.non-symbolic-variable, code: InvalidOperation, category: DomainError, message: limit(): argument 2 must be a symbolic variable; got Natural., trigger: x = symbol("x"); limit(sin(x), 1, 0)), UnsupportedCapability(operation_class: dsp.symbolic-element, code: InvalidOperation, category: DomainError, message: DSP builtins expect numeric/complex elements, but got 'SymbolExpr'., trigger: x = symbol("x"); dft([x, 1, 2, 3])), UnsupportedCapability(operation_class: linsolve.non-symbolic-matrix, code: InvalidOperation, category: DomainError, message: linsolve() requires a symbolic matrix A., trigger: A = [[1,2],[2,4]]; b = [[1],[3]]; linsolve(A, b)), UnsupportedCapability(operation_class: fft.non-power-of-two-length, code: InvalidArgument, category: TypeMismatch, message: FFT length must be a power of two, but got 3. (Parameter 'x'), trigger: x = symbol("x"); fft([1,2,3]))], exactness: BestEffort)
```

The 16 entries, field by field, exactly as the binary emits them
(`operation_class | code | category | message | trigger`):

| # | operation_class | code | category | message | trigger |
|---|---|---|---|---|---|
| 0 | `pow.non-integer-exponent` | `UnsupportedOperation` | `UnsupportedOperation` | `Non-integer exponents are not yet supported.` | `2^(1/2)` |
| 1 | `pow.negative-base-unrepresentable-exponent` | `UnsupportedOperation` | `UnsupportedOperation` | `Non-integer exponents are not yet supported.` | `(-8)^(1/3)` |
| 2 | `solve.unsupported-domain` | `InvalidOperation` | `DomainError` | `solve(): currently supports domains real and complex; got integer.` | `x = symbol("x"); solve(x^2 - 2 == 0, x, integer)` |
| 3 | `rootof.complex-algebraic` | `solve.unrepresented-roots` | `UnsupportedOperation` | `complex algebraic roots not supported (RootOf is real-only in v1).` | `x = symbol("x"); solve_full(x^4 - x^2 - 1 == 0, x)` |
| 4 | `limit.unevaluated-in-record-diagnostics` | `limit.unevaluated` | `UnsupportedOperation` | `coefficient does not evaluate at the point` | `x = symbol("x"); limit_full(sin(x), x, inf)` |
| 5 | `integration.unevaluated-in-record-diagnostics` | `integration.unevaluated` | `UnsupportedOperation` | `No closed form was found, so the integral is returned unevaluated; the reason for the refusal is carried in this diagnostic's details.` | `x = symbol("x"); integrate_full(exp(x^2), x)` |
| 6 | `plot.symbolic-expression` | `InvalidOperation` | `DomainError` | `plot() argument 1 must be a vector, but got 'Symbolic'.` | `x = symbol("x"); plot(sin(x))` |
| 7 | `plot.symbolic-element` | `InvalidOperation` | `DomainError` | `Cannot convert value of kind 'Symbolic' to a number for plotting.` | `x = symbol("x"); plot([x, 1, 2])` |
| 8 | `solve.non-symbolic-variable` | `InvalidOperation` | `DomainError` | `solve(): argument 2 must be a symbolic variable; got Natural.` | `x = symbol("x"); solve(x^2 - 2 == 0, 1)` |
| 9 | `solve.non-symbolic-expression` | `InvalidOperation` | `DomainError` | `solve(): argument 1 must be a symbolic expression; got Vector.` | `x = symbol("x"); solve([x == 1], x)` |
| 10 | `diff.non-symbolic-variable` | `InvalidOperation` | `DomainError` | `diff(): argument 2 must be a symbolic variable; got Natural.` | `x = symbol("x"); diff(x^2, 1)` |
| 11 | `integrate.non-symbolic-variable` | `InvalidOperation` | `DomainError` | `integrate(): argument 2 must be a symbolic variable; got Natural.` | `x = symbol("x"); integrate(x^2, 1)` |
| 12 | `limit.non-symbolic-variable` | `InvalidOperation` | `DomainError` | `limit(): argument 2 must be a symbolic variable; got Natural.` | `x = symbol("x"); limit(sin(x), 1, 0)` |
| 13 | `dsp.symbolic-element` | `InvalidOperation` | `DomainError` | `DSP builtins expect numeric/complex elements, but got 'SymbolExpr'.` | `x = symbol("x"); dft([x, 1, 2, 3])` |
| 14 | `linsolve.non-symbolic-matrix` | `InvalidOperation` | `DomainError` | `linsolve() requires a symbolic matrix A.` | `A = [[1,2],[2,4]]; b = [[1],[3]]; linsolve(A, b)` |
| 15 | `fft.non-power-of-two-length` | `InvalidArgument` | `TypeMismatch` | `FFT length must be a power of two, but got 3. (Parameter 'x')` | `x = symbol("x"); fft([1,2,3])` |

The byte-exact wire form of all 16 entries is `Lovelace.Run.Tests/fixtures/capabilities.json`
(`structured.fields[unsupported_operations]`, 504 lines), regenerated from this binary (§6).

## 6. The golden fixture: before/after diff

The golden is the envelope with `revision`, `elapsed`, `elapsedTime` and `timings` replaced by
`"<volatile>"` at any depth (the contract `TestSupport` compares modulo). It was regenerated **from
the binary**, not edited:

```powershell
$exe = (Resolve-Path "out\aot\Lovelace.Run.exe").Path
Start-Process $exe -ArgumentList '--file','Lovelace.Run.Tests\fixtures\capabilities.ls','--omit-functions' `
  -Wait -RedirectStandardOutput $env:TEMP\caps.stdout -NoNewWindow
# then: parse the envelope, replace the four volatile keys at any depth, pretty-print with the
# golden's own format (2-space indent, LF, trailing newline) and write fixtures/capabilities.json
```

Size: **10727 → 36304 bytes**. `git diff --numstat`: `508 insertions(+), 4 deletions(-)`, in five
hunks:

```
diff --git a/Lovelace.Run.Tests/fixtures/capabilities.json b/Lovelace.Run.Tests/fixtures/capabilities.json
index d3b5854..b117c2f 100644
--- a/Lovelace.Run.Tests/fixtures/capabilities.json
+++ b/Lovelace.Run.Tests/fixtures/capabilities.json
@@ -9,2 +9,2 @@
-    "display": "CapabilitiesResult(supported_domains: [real, complex], unsupported_domains: [integer, rational], unsuppo…[line is 1105 chars; abridged]
-    "typed": "CapabilitiesResult(supported_domains: [real, complex], unsupported_domains: [integer, rational], unsupport…[line is 1124 chars; abridged]
+    "display": "CapabilitiesResult(supported_domains: [real, complex], unsupported_domains: [integer, rational], unsuppo…[line is 4034 chars; abridged]
+    "typed": "CapabilitiesResult(supported_domains: [real, complex], unsupported_domains: [integer, rational], unsupport…[line is 4053 chars; abridged]
@@ -61 +61 @@
-              4
+              16
@@ -230,0 +231,504 @@
+              },
+              {
+                "kind": "Record",
+                "type": "UnsupportedCapability",
+                "fields": [
+                  {
+                    "name": "operation_class",
+                    "value": {
+                      "kind": "Text",
+                      "value": "limit.unevaluated-in-record-diagnostics"
+                    }
+                  },
   [... 483 lines elided: the wire form of entries 4..15, reproduced field by field in §5 and in
        full in the fixture itself; elided here only because it is 483 lines of the same six-field
        record shape ...]
+                  },
+                  {
+                    "name": "trigger",
+                    "value": {
+                      "kind": "Text",
+                      "value": "x = symbol(\"x\"); fft([1,2,3])"
+                    }
+                  }
+                ]
@@ -251 +755 @@
-      "display": "CapabilitiesResult(supported_domains: [real, complex], unsupported_domains: [integer, rational], unsup…[line is 1106 chars; abridged]
+      "display": "CapabilitiesResult(supported_domains: [real, complex], unsupported_domains: [integer, rational], unsup…[line is 4035 chars; abridged]
```

(The three `display`/`typed` lines above are the record's own rendering of the entry list, abridged
in this document at 120 characters with their true length marked; their full text is exactly the
rendering of the entries listed verbatim in §5. The 504-line hunk adds the twelve new entries to
`structured.fields[unsupported_operations]`: `shape` goes 4 → 16, which is the `@@ -61 +61 @@`
hunk. Nothing else in the envelope changed — not one byte of the unchanged entries' wire form.)

Reproduce the complete, unabridged diff with:

```powershell
git diff -- Lovelace.Run.Tests/fixtures/capabilities.json
```

## 7. `exactness`: verdict and its basis

**`BestEffort`, unchanged — and now falsifiable in both directions.**

*Enumerated (basis):* **every refusal class the round-09 adversarial audit found unlisted**
(part 6(b): P6b-1…P6b-8, i.e. the limit, integration, plot, solve/diff/integrate/limit argument
guards, DSP element guard, linsolve guard and FFT length), plus the four classes earlier rounds
advertised — 16 entries, each live-verified against the published binary by
`CapabilitiesBuiltinTests.AdvertisedCodesAndCategories_MatchTheLiveEnvelope`, and the whole set
pinned by `UnsupportedOperations_AreRecordsCarryingCodeCategoryAndMessage`.

*Not enumerated, stated in the record's own comment and therefore not overclaimed:*
the remaining kernel-note diagnostics whose message is the input-specific reason
(`solve.unevaluated`, `solve.partial`, `solve.budget-exceeded`, `solve.no-solutions`,
`system-solve.unevaluated`, `system-solve.no-solutions`, `matrix.singular`);
`transform.budget-exceeded` / `transform.unsatisfiable-conditions` (budget stop and
contradictory-branch domain error, not unsupported operations); the host-level taxonomy
(`ParseError`, `FileReadError`, the arity errors, `Cancelled`/`BudgetExceeded`,
`UnsatisfiableAssumptions`); and the caller-side argument errors that are still **defects**, not
capabilities (`mean(1)`, `max(1,2)`, `sum(x, 5)` → `InternalError`/`InternalInvariantFailure` with a
raw CLR cast message, audit finding F4; the 3000-deep native stack overflow, F11).

`CapabilitiesBuiltinTests.ExactnessBestEffort_IsBackedByLiveRefusalsTheListDoesNotCarry` drives five
of those refusals live, asserts each is refused and that **no advertised entry describes it**, and
asserts the `InternalError` code is claimed by no entry. If a later round enumerates them all, that
test fails and the verdict must be re-decided on the record — `BestEffort` cannot silently become an
underclaim, and nothing here lets it silently become an overclaim.

## 8. Tests: what was extended, what was replaced, and why

**No test was weakened, deleted or skipped.** Three assertions were replaced; each replacement is
strictly stronger, and each is listed here.

1. `CapabilitiesBuiltinTests.AdvertisedCodesAndCategories_MatchTheLiveEnvelope` — **same test name**,
   same property (advertised code/category/message == observed), extended:
   * it now runs over all **16** entries instead of 4 (the loop was already generic; the entries are
     new);
   * it now asserts the **carrier** as well: for a class marked `-in-record-diagnostics` the live
     call must SUCCEED with a Record whose `status` enum is a refusal member and whose first
     diagnostic equals the advertisement; for every other class the carrier actually observed is
     used and reported;
   * it reports the observed carrier for every entry, so the log shows *which* field was checked.
2. `CapabilitiesBuiltinTests.UnsupportedOperations_AreRecordsCarryingCodeCategoryAndMessage` —
   the four `Assert.Contains(...)` class checks were replaced by a **complete set comparison**
   (`Assert.Equal(16, classes.Length)` plus set equality against the pinned list). The four old
   names are still required (they are in the pinned set, which also still asserts
   `DoesNotContain("complex.sqrt-negative")`); the replacement additionally fails if an entry is
   added or removed without a deliberate edit here.
3. `AdvertisedByOperationClass` (test helper) — now **throws** if two entries share an
   `operation_class`, because the honesty loop is keyed by that name and would otherwise silently
   verify one entry and skip the other.

Unchanged but verified unaffected (each was checked before the change and re-run after):

* `HyperbolicBuiltinsAndNegativeIntegerPowersTests` — **not touched and not broken**: no builtin was
  added, so `BuiltinCount_IncludesTheThreeHyperbolicFunctions` still observes exactly 107 builtins,
  and the two power controls still throw with their exact messages. (The brief anticipated that this
  file would need a replacement; it did not, and the reason is that the repairs changed *how* an
  existing builtin refuses, not the exported surface.)
* `HyperbolicBuiltinsAndNegativeIntegerPowersTests.NegativeSquareRootsAndHalfPowers_NowAnswerExactly_WhileOtherNonIntegerPowersStillThrow`
  — the two advertised power classes are still advertised and still throw identically.
* `DiagnosticContractTests.PartialSolve_CarriesADiagnosticWithAStableCodeAndAnErrorCategoryMember`
  and `DiagnosticContractTests.KernelNotes_CrossAsDiagnosticMessages` — these pin an EMPTY `details`
  for the solve/limit diagnostics and the exact solve/limit messages. Only the **integration**
  diagnostic gained `details`; solve and limit are untouched, and both tests pass.
* `GoldenEnvelopeTests.WireDiagnostic_CarriesTheSixFrozenFieldsInOrder` (empty `details` on the
  `solve_result` fixture) and `Lovelace.Suite.Tests/RecordSchemaTests.SingularMatrix_…` (empty
  `details` on the matrix-singular diagnostic) — unaffected; the integration fixture
  (`integrate_full(x^2, x)`) is a *solved* integration and its golden did not change.
* `RelationsAndFamiliesTests` — asserts `IntegrationKind.Unevaluated` for a hard integral; still
  true. The kernel change *adds* a `Note`; no test asserted that the note was null.
* **No test asserted the plot failure.** The audit's P6b-4 was reproduced by hand, and nothing in
  `Lovelace.Suite.Tests` (PlotTests, SuiteEngineOutputPlotTests, LanguageDocumentationTests,
  ScriptSourceTests) or `Lovelace.Studio.Tests` drives a symbolic plot: every plot example is
  vector-based. So there was no assertion to replace — instead two new tests were added.

Added tests:

* `EveryAdvertisedEntry…` (rule) is folded into test 1 above; the two new focused facts are
  `IntegrationRefusal_IsTypedStructuredAndClassLevel` (three different integrands: one class-level
  message, three different `details` reasons, each naming the input) and
  `SymbolicPlot_IsATypedRefusal_NeverAnInternalInvariantFailure` (`plot(sin(x))` and `plot(1)` are
  `InvalidOperation`/`DomainError` and recoverable, never `InternalInvariantFailure`; the advertised
  entry equals the live message).
* `ExactnessBestEffort_IsBackedByLiveRefusalsTheListDoesNotCarry` (§7).

## 9. Acceptance evidence — observed output

```
> dotnet test Lovelace.Symbolics.Tests -c Release --nologo
Passed!  - Failed:     0, Passed:   816, Skipped:     6, Total:   822, Duration: 17 s - Lovelace.Symbolics.Tests.dll (net10.0)
     (the 6 skipped are the DifferentialOracle tests, which skip when the SymPy oracle is not on PATH)

> $env:PATH = "C:\Users\ricar\dev\.lovelace-tools\python;" + $env:PATH; $env:LOVELACE_REQUIRE_SYMPY = "1"
> python -c "import sympy; print('sympy', sympy.__version__)"
sympy 1.14.0
> dotnet test Lovelace.Symbolics.Tests -c Release --nologo
Passed!  - Failed:     0, Passed:   822, Skipped:     0, Total:   822, Duration: 27 s - Lovelace.Symbolics.Tests.dll (net10.0)
     (with the oracle on PATH every differential test RUNS: 0 skipped)

> dotnet test Lovelace.Suite.Tests -c Release --nologo
Passed!  - Failed:     0, Passed:   642, Skipped:     0, Total:   642, Duration: 12 s - Lovelace.Suite.Tests.dll (net10.0)

> dotnet test Lovelace.Run.Tests -c Release --nologo
Passed!  - Failed:     0, Passed:    39, Skipped:     0, Total:    39, Duration: 782 ms - Lovelace.Run.Tests.dll (net10.0)

> dotnet test Lovelace.Symbolics.Tests -c Release --nologo --filter "FullyQualifiedName~DifferentialOracle"
Passed!  - Failed:     0, Passed:     6, Skipped:     0, Total:     6, Duration: 20 s - Lovelace.Symbolics.Tests.dll (net10.0)
```

The `capabilities` golden comparison is inside the 39 passing `Lovelace.Run.Tests` (regenerated
fixture, §6). `dotnet publish … -p:PublishAot=true` for the AOT binary used throughout exited 0.

## 10. What I could NOT make honest, and what remains

1. **The script-level parse refusal is not advertisable as a class.** `1 +;` is refused with
   `code=InvalidOperation, category=DomainError` — the *same* code/category pair as the argument
   guards, so an agent cannot branch between "malformed script" and "bad argument" by code alone.
   Giving it its own code would be a change to the host's exception classification
   (`Lovelace.Run.Runner.Classify` / the parser), outside this round's scope; advertising it under
   the shared code would make the enumeration ambiguous rather than more truthful. Recorded as a
   residual, and it is one of the reasons `exactness` stays `BestEffort`.
2. **`mean(1)`, `max(1,2)`, `sum(x, 5)` remain `InternalError`/`InternalInvariantFailure`** with the
   raw CLR cast message (audit F4). They are defects, not capability boundaries, so they are
   deliberately unadvertised — but that also means the enumeration cannot claim exhaustiveness.
3. **The remaining message-varying kernel-note diagnostics** (`solve.unevaluated`, `solve.partial`,
   `system-solve.unevaluated`, …) are still unadvertised. The integration fix shows the shape that
   would make them advertisable (class-level message + `details` reason), but applying it to solve
   would change messages that other tests pin as contract
   (`DiagnosticContractTests.KernelNotes_CrossAsDiagnosticMessages` asserts
   `"0 = 0: every value is a solution."` *as the message*), and that is a decision for the solve
   contract, not a side effect of this round.
4. **Two transcribed messages leak internals**: `dft`/`fft` expose the CLR type name `SymbolExpr`
   and the .NET `(Parameter 'x')` artifact. They are honest transcriptions of live output;
   rewording them means touching `Lovelace.Dsp`, which is outside the round's scope.
5. **`limit.unevaluated` is one class with several messages** (the advertised trigger's message is
   transcribed; `limit_full(a/x, x, 0)` produces a different one). The record documents that the
   code/category are class-level while the message is trigger-level, and the honesty test asserts
   exactly that pair, so nothing is overstated — but a consumer that matches on `message` rather than
   `code` will see more than one string for the class.
6. **`revision`** in the envelope is not a revision marker (audit "Secondary"); unrelated to this
   round's scope and untouched.

## 11. Reproduce everything

```powershell
# 1. build + publish the binary the evidence used
dotnet publish Lovelace.Run/Lovelace.Run.csproj -c Release -p:PublishAot=true -p:InvariantGlobalization=true -o out/aot

# 2. the live statement and every advertised trigger, checked field by field (no shell quoting issues)
#    -> see §4; each trigger is run with --file, --omit-functions --omit-variables

# 3. regenerate the golden from that binary (§6), then:
dotnet test Lovelace.Run.Tests      -c Release --nologo
dotnet test Lovelace.Symbolics.Tests -c Release --nologo
dotnet test Lovelace.Suite.Tests    -c Release --nologo

# 4. the differential oracle must RUN, not skip
$env:PATH = "C:\Users\ricar\dev\.lovelace-tools\python;" + $env:PATH
$env:LOVELACE_REQUIRE_SYMPY = "1"
dotnet test Lovelace.Symbolics.Tests -c Release --nologo --filter "FullyQualifiedName~DifferentialOracle"
```
