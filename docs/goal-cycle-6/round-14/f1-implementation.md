# F1 — a wrong-SHAPED builtin argument crosses as the documented argument error, never as an internal invariant failure

Round 14 of goal cycle 6. Finding under repair: **F1 (P1, NEW)** in
`docs/goal-cycle-6/round-13/audit-D-workflow.md:31-60`, whose measured extent was **52 of 369 swept
calls, across 26 builtins** (the 52 reproducing byte-identically on the published AOT binary).

| | before | after |
|---|---|---|
| 345 wrong-shape probes (123 builtins x 3 shapes, every declared position filled) | **52 crossed as `InternalError`/`InternalInvariantFailure`** (`recoverable: false`), across 26 builtins | **0** |
| the same probes refused in the documented family (`InvalidArgument`/`TypeMismatch`) | 27 | 79 |
| the five audit reproductions on the published AOT binary | `InternalError`/`InternalInvariantFailure`/`"Specified cast is not valid."`/`recoverable:false` | `InvalidArgument`/`TypeMismatch`, naming the builtin, the 1-based position and the kind that arrived, `recoverable:true` |
| whole solution `dotnet test LovelaceSharp.slnx -c Release` | (not run on this finding) | **15 projects, 0 failed, 5391 passed, 8 skipped** |
| the new tests on a pristine HEAD tree | — | **9 failed** (6 in Run.Tests, 3 in Suite.Tests) |

## 0. Trees and hygiene

* **Scratch tree (all work):** `.worktrees/c6-f1`, `git worktree add .worktrees/c6-f1 HEAD`,
  `HEAD = 141f34e2c145cb2ca40c7e0fd3cad9366e2e4db3` (`git -C .worktrees/c6-f1 rev-parse HEAD`).
* **Control tree:** `.worktrees/c6-f1-ctl`, same command, same `141f34e`; it contains **only the two
  new test files** and nothing else (`git -C .worktrees/c6-f1-ctl status --short` printed exactly
  `?? Lovelace.Run.Tests/BuiltinArgumentShapeSweepTests.cs` and
  `?? Lovelace.Suite.Tests/BuiltinArgumentShapeGuardTests.cs`; the `.txt` transcripts below are run
  artifacts, not sources).
* **The main tree** `C:/Users/ricar/dev/LovelaceSharp` received **only this document and its evidence
  files** under `docs/goal-cycle-6/round-14/`. No source file outside the scratch tree was touched.
* **No `git add`, no `git commit`, no `git push` was run** in any tree; the scratch tree reports the
  four modified sources as unstaged (` M`) and the two tests as untracked (`??`).
* The oracle environment was prepared for every run as instructed:
  `$env:PATH = "C:/Users/ricar/dev/.lovelace-tools/python;" + $env:PATH` and `$env:LOVELACE_REQUIRE_SYMPY="1"`.

## 1. Pre-fix behaviour, re-observed on the published binary

Binary: `C:/Users/ricar/dev/LovelaceSharp/out/aot/Lovelace.Run.exe`, **5 776 896 bytes,
2026-09-12T15:06:37-03:00**. Transcript: `aot-five-calls-before.txt`.

```powershell
$exe = "C:/Users/ricar/dev/LovelaceSharp/out/aot/Lovelace.Run.exe"
foreach ($s in @("det(1)","matmul(1,1)","symbol(1)","assume(1)","transpose(1,1)")) {
  & $exe --eval $s --omit-functions --omit-variables
  "EXIT=" + $LASTEXITCODE
}
```

| call | `ok` | `code` | `category` | `message` | `recoverable` | exit |
|---|---|---|---|---|---|---|
| `det(1)` | false | `InternalError` | `InternalInvariantFailure` | `Specified cast is not valid.` | false | 1 |
| `matmul(1,1)` | false | `InternalError` | `InternalInvariantFailure` | `Specified cast is not valid.` | false | 1 |
| `symbol(1)` | false | `InternalError` | `InternalInvariantFailure` | `Specified cast is not valid.` | false | 1 |
| `assume(1)` | false | `InternalError` | `InternalInvariantFailure` | `Specified cast is not valid.` | false | 1 |
| `transpose(1,1)` | false | `InternalError` | `InternalInvariantFailure` | `Specified cast is not valid.` | false | 1 |

Verbatim envelope for `det(1)` (`EXIT=1`):

```json
{"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":false,
 "code":"InternalError","category":"InternalInvariantFailure","message":"Specified cast is not valid.",
 "recoverable":false,"diagnostics":[{"message":"Specified cast is not valid.","position":0,"line":1,"column":1}],
 "elapsed":"608.8 us", ...}
```

> The message names neither the builtin, nor the argument position, nor the expectation — exactly the
> F1 defect. (The CLR gives the richer `Unable to cast object of type ... to type ...` on CoreCLR; the
> published AOT runtime collapses it to `Specified cast is not valid.`, which is why the in-process
> test transcripts below show the richer text and this one does not.)

## 2. The failing test, written and observed first

Two new files, **no production change at the time they were first run**:

* `Lovelace.Run.Tests/BuiltinArgumentShapeSweepTests.cs` — drives the **live registry**
  (`SuiteEngine` + `DspPlugin` + `SymbolicsPlugin` + `MathIRPlugin`, i.e. the wiring of
  `Lovelace.Run/Program.cs`) through the **published runner entry point** (`Runner.RunAsync`, the same
  one the AOT binary calls) and parses the one JSON envelope per call. There is **no list of the 26
  builtins anywhere in it**: every builtin the engine exposes is swept.
  * `EveryWrongShapedArgument_IsNeverAnInternalFailure_AndEveryArgumentRefusalNamesTheBuiltinAndThePosition`
    — 123 builtins x 3 wrong shapes (a bare scalar `1`, a four-element vector `[1, 2, 3, 4]`, a symbolic
    value `x` with `x = symbol("x")`), every declared position filled from the descriptor, one call per
    (builtin, shape) = **345 probes**. For each probe: an accepted call is recorded; a refusal must not be
    `InternalError`/`InternalInvariantFailure`, must be `recoverable`, must not carry raw framework text
    (`cast is not valid`, `Unable to cast`, `Object reference not set`, ...); and a refusal in the
    documented family (`code == InvalidArgument`) must be `category == TypeMismatch` with a message
    matching `^(?<builtin>[A-Za-z_]\w*)\(\): argument (?<position>[1-9][0-9]*)( \([A-Za-z_]\w*\))? must be .+; got .+\.$`
    that names **the probed builtin** and a position **inside the call's own arity**. The test also
    refuses to pass vacuously: `>= 300` probes, `>= 100` accepted, `>= 1` refusal, `>= 1` documented-family
    refusal.
  * `EveryWrongShapedArgument_OverAWiderShapeLattice_IsStillNeverAnInternalFailure` — the audit recorded
    52/369 as a **lower bound** ("3 shapes per builtin, not the full type lattice",
    `audit-D-workflow.md:312`). This test widens the lattice to 8 shapes (adding a Record `inspect(1)`, a
    2x2 matrix, text, a Boolean, an empty vector) = **920 probes**, and asserts the unconditional part:
    no internal failure, every refusal recoverable, no raw framework text.
  * `TheAuditsFiveReproductions_CrossAsTheDocumentedArgumentError` — pins the five audit calls to
    `InvalidArgument`/`TypeMismatch`/`recoverable:true` and to a message that starts with
    `"<builtin>(): argument <n> must be "` and ends with `"; got Natural."`. **This is the assertion that
    forbids the cheap "reclassify the cast as a domain error" repair**: it demands the documented code,
    category and attribution, not merely the absence of `InternalError`.
  * `TheDeclaredForm_StaysCallable` — nine positive controls (`det([[1,2],[3,4]])`,
    `matmul([[1,2],[3,4]],[[5],[6]])`, `transpose([[1,2],[3,4]])`, `trace(...)`, `shape(...)`,
    `symbol("x")`, `assume(x > 5)`, `assume_positive(x)`, `assume_real("y")`), so the contract is
    "the wrong shape is refused", not "the builtin became unreachable".
* `Lovelace.Suite.Tests/BuiltinArgumentShapeGuardTests.cs` — pins the **mechanism** on a builtin that
  exists only in the test (`engine.RegisterBuiltin("probe", ...)`), proving the guard is central rather
  than 26 patched call sites, plus the rule that a coercion failure on an engine-internal value stays an
  internal failure. Its assertions read the exception the engine raises and never name the guard's own
  type, so the file **also compiles against the pre-fix tree** (the discipline of
  `Lovelace.Run.Tests/ArityPropertyTests.cs:56-86`).

### 2.1 Failing-first transcript

```
PS> dotnet test Lovelace.Run.Tests\Lovelace.Run.Tests.csproj -c Release --nologo --filter "FullyQualifiedName~BuiltinArgumentShapeSweepTests"
  Failed ...EveryWrongShapedArgument_IsNeverAnInternalFailure_AndEveryArgumentRefusalNamesTheBuiltinAndThePosition
  58 of 232 refusals (345 probes over 123 builtins) violate the frozen argument-shape contract
  (52 crossed as internal invariant failures):
append(1, 1) [a bare scalar]: a caller-side mistake crossed as an INTERNAL failure (code=InternalError,
  category=InternalInvariantFailure, recoverable=False, message='Unable to cast object of type
  'Lovelace.Natural.Natural' to type 'Lovelace.Abstractions.ArrayValue'.')
assume(1) [a bare scalar]: ... ('Lovelace.Natural.Natural' to type 'Lovelace.Symbolics.Expr')
assume_integer(1) [a bare scalar]: ... ('Lovelace.Natural.Natural' to type 'System.String')
... (49 more)
  Failed ...TheAuditsFiveReproductions_CrossAsTheDocumentedArgumentError(call: "det(1)", ...)
    Assert.Equal() Failure: Strings differ   Expected: "InvalidArgument"   Actual: "InternalError"
  (the same for matmul(1, 1), symbol(1), assume(1), transpose(1, 1))
Failed!  - Failed:     6, Passed:     9, Skipped:     0, Total:    15
```

Full transcript: `sweep-before-test-output.txt` (19 327 bytes). The 52 internal failures group as: 37 x
`Value.AsArrayValue()` on a Natural/SymbolExpr, 12 x `(string)args[0]!` in `symbol`/`assume*`, 2 x
`(Expr)args[0]!` in `assume`, 1 x `(string)` for `symbol(x)`.

**One test correction, stated openly.** The first version of the grammar assertion above did not allow the
declared parameter name in parentheses, so the two `evalf` probes were reported as violations
(`evalf(): argument 2 (digits) must be a Natural or Integer digit count between 1 and 2147483647; got
Vector.`) — that is why the first transcript says 58 and the control run says 56. That refusal already
names the builtin, the 1-based position, the expectation and the arrived kind, and the parenthetical form is
itself frozen by other tests (`Lovelace.Run.Tests/BuiltinSurfaceContractTests.cs:117`,
`Lovelace.Symbolics.Tests/EvalfDigitCountContractTests.cs:59,145`). The regex was widened to allow it; **no
assertion was removed or weakened**, and the 52 internal failures were already zero from the source change
alone (the source diff was written before this correction).

## 3. Root cause

The argument-coercion helpers on the path every builtin uses cast the payload directly:

* `Lovelace.Suite/Value.cs:212` (pre-fix): `public ArrayValue AsArrayValue() => (ArrayValue)_inner;` — and
  the same shape for `AsText`, `AsReal`, `AsSymbolic`, ...
* `Lovelace.Symbolics/SymbolicsPlugin.cs:194,246,1502,1514` (pre-fix): `(string)args[0]!` for the symbol
  name and the `assume*` name form, `(Expr)args[0]!` for `assume`.

The failed cast throws `InvalidCastException`, which no layer recognises, so
`Lovelace.Run/Runner.cs:273-296` falls into its last arm — `_ => ("InternalError", "InternalInvariantFailure",
false)` — and the caller is told the kernel broke.

## 4. The fix — one guard in the shared helper plus one at the call site

| file:line (after) | what it does |
|---|---|
| `Lovelace.Suite/Value.cs:233` `private T Require<T>(string expected)` | the ONE coercion guard behind every `As...` accessor (`Value.cs:186-232`); raises `ValueShapeException` carrying the expectation and the offending value instead of a raw CLR cast |
| `Lovelace.Suite/Value.cs:340` `ValueShapeException` | derives from `InvalidCastException`, so the escape hatch (and every existing host that catches the framework type) behaves exactly as before |
| `Lovelace.Suite/Functions.cs:178` `BuiltinShapeException` | the documented refusal, an `ArgumentException` (hence `InvalidArgument`/`TypeMismatch`, recoverable): `<builtin>(): argument <n> must be <expected>; got <Kind>.` |
| `Lovelace.Suite/Interpreter.cs:797-830` the ONE call site (`fn.Builtin!(...)`) | passes a `BuiltinArguments` list (`Interpreter.cs:838`) that records the position read last, catches `ValueShapeException` and re-raises `BuiltinShapeException` **only when the failing value IS one of the call's arguments** (`PositionOf`, `Interpreter.cs:855`); a raw `InvalidCastException` from a body that casts its own payload is attributed to the argument it read last (`Interpreter.cs:821`) |
| `Lovelace.Suite/Interpreter.cs:2023,2035,2047` `RequireSquare` / `RequireLength3` / `RequirePermutation` | call-site shape guards where the expectation is not a `Value` accessor, so `det`/`trace`/`cross`/`transpose` refusals also carry the position and the arrived kind |
| `Lovelace.Symbolics/SymbolicsPlugin.cs:1698,1703,1710` `AsTextName` / `AsSymbolName` / `AsRelation` | the three Symbolics payload casts become the same `BuiltinShapeError` route the file already had (`SymbolicsPlugin.cs:176-183`), used at `:191`, `:246`, `:1502`, `:1514` |

No builtin body was patched one by one: the 43 array/matrix cases flow through the single coercion guard, and
the 12 Symbolics cases flow through three shared helpers.

### 4.1 The five calls after the fix (fixed Native AOT binary, re-published from the scratch tree)

```
C:/Users/ricar/dev/LovelaceSharp/.worktrees/c6-f1/out-aot-f1/Lovelace.Run.exe
5 793 792 bytes, 2026-09-12T15:46:21-03:00, publish exit 0
```

| call | `code` | `category` | `message` | `recoverable` | exit |
|---|---|---|---|---|---|
| `det(1)` | `InvalidArgument` | `TypeMismatch` | `det(): argument 1 must be a square matrix; got Natural.` | true | 1 |
| `matmul(1,1)` | `InvalidArgument` | `TypeMismatch` | `matmul(): argument 1 must be an array or vector; got Natural.` | true | 1 |
| `symbol(1)` | `InvalidArgument` | `TypeMismatch` | `symbol(): argument 1 must be a text name such as "x"; got Natural.` | true | 1 |
| `assume(1)` | `InvalidArgument` | `TypeMismatch` | `assume(): argument 1 must be a relation such as x > 5; got Natural.` | true | 1 |
| `transpose(1,1)` | `InvalidArgument` | `TypeMismatch` | `transpose(): argument 1 must be an array or vector; got Natural.` | true | 1 |

Full envelopes: `aot-five-calls-after.txt`. The same five were also observed through the in-tree Release
runner (`dotnet .worktrees/c6-f1/Lovelace.Run/bin/Release/net10.0/Lovelace.Run.dll --eval ...`), identical
deciding fields.

### 4.2 The source diff

```diff
diff --git a/Lovelace.Suite/Functions.cs b/Lovelace.Suite/Functions.cs
index 4ca1510..0d32572 100644
--- a/Lovelace.Suite/Functions.cs
+++ b/Lovelace.Suite/Functions.cs
@@ -159,3 +159,45 @@ public sealed class BuiltinArityException : ArgumentException
 
     private static string Argument(int count) => count == 1 ? "argument" : "arguments";
 }
+
+/// <summary>
+/// A call-site SHAPE failure: the argument the builtin's declared parameter names is not the KIND
+/// of thing the body requires — a bare scalar where a matrix is required, a symbolic value where a
+/// text name is required, a non-permutation where an axis order is required. Like
+/// <see cref="BuiltinArityException"/> it derives from <see cref="ArgumentException"/>, so the
+/// runner's taxonomy classifies the call as the documented RECOVERABLE argument error
+/// (<c>code: InvalidArgument</c>, <c>category: TypeMismatch</c>) — never as a domain refusal, and
+/// never, as the direct cast it replaced did, as an internal invariant failure carrying the raw CLR
+/// message (audit D, finding F1).
+/// <para>
+/// The message is the one grammar every argument-shape refusal shares, so a caller reads the
+/// builtin, the 1-based position, what was required and what actually arrived without parsing
+/// prose: <c>det(): argument 1 must be an array or vector; got Natural.</c>
+/// </para>
+/// </summary>
+public sealed class BuiltinShapeException : ArgumentException
+{
+    public BuiltinShapeException(string builtin, int position, string expected, ValueKind arrived)
+        : base(Describe(builtin, position, expected, arrived))
+    {
+        Builtin = builtin;
+        Position = position;
+        Expected = expected;
+        Arrived = arrived;
+    }
+
+    /// <summary>The builtin whose argument shape the call violated.</summary>
+    public string Builtin { get; }
+
+    /// <summary>The 1-based position of the offending argument.</summary>
+    public int Position { get; }
+
+    /// <summary>What that position requires, as prose.</summary>
+    public string Expected { get; }
+
+    /// <summary>The kind that actually arrived in that position.</summary>
+    public ValueKind Arrived { get; }
+
+    private static string Describe(string builtin, int position, string expected, ValueKind arrived) =>
+        $"{builtin}(): argument {position} must be {expected}; got {arrived}.";
+}
diff --git a/Lovelace.Suite/Interpreter.cs b/Lovelace.Suite/Interpreter.cs
index 134a5fb..62fd03f 100644
--- a/Lovelace.Suite/Interpreter.cs
+++ b/Lovelace.Suite/Interpreter.cs
@@ -798,12 +798,74 @@ public sealed class Interpreter
         {
             // ONE validator, computed from the builtin's own declared metadata, before the body runs
             ValidateArity(fn, args.Count);
-            return await fn.Builtin!(args);
+            // ... and ONE guard for the argument SHAPE. The body reads its arguments through this
+            // list, so a coercion failure is attributed to the argument that caused it and crosses
+            // as the documented recoverable argument error naming the builtin, the position and the
+            // kind that arrived — instead of escaping as a raw CLR cast and being reported as an
+            // internal invariant failure (audit D, finding F1).
+            var tracked = new BuiltinArguments(args);
+            try
+            {
+                return await fn.Builtin!(tracked);
+            }
+            catch (ValueShapeException ex)
+            {
+                // an exact attribution to one of the call's arguments is the documented argument
+                // error; a coercion that failed on an engine-internal value is NOT a caller mistake
+                // and keeps crossing as an internal invariant failure
+                if (tracked.PositionOf(ex.Offender) is not int position)
+                    throw;
+
+                throw new BuiltinShapeException(fn.Name, position + 1, ex.Expected, tracked[position].Kind);
+            }
+            catch (InvalidCastException) when (tracked.LastIndex >= 0)
+            {
+                // a body that casts its payload itself rather than through a Value accessor: the
+                // argument it read last is the one it was working on
+                throw new BuiltinShapeException(fn.Name, tracked.LastIndex + 1,
+                    "a value this builtin can use", tracked[tracked.LastIndex].Kind);
+            }
         }
 
         return await CallUserFunctionAsync(fn, args);
     }
 
+    /// <summary>
+    /// The argument list a builtin body reads. It records the position read last, so the call-site
+    /// shape guard can name the argument a failed coercion came from even when the failure is
+    /// raised deep inside the array kernel and carries no <see cref="Value"/> of its own.
+    /// </summary>
+    private sealed class BuiltinArguments(IReadOnlyList<Value> inner) : IReadOnlyList<Value>
+    {
+        /// <summary>0-based index of the argument read last, or -1 when the body has read none.</summary>
+        public int LastIndex { get; private set; } = -1;
+
+        public Value this[int index]
+        {
+            get { LastIndex = index; return inner[index]; }
+        }
+
+        public int Count => inner.Count;
+        public IEnumerator<Value> GetEnumerator() => inner.GetEnumerator();
+        System.Collections.IEnumerator System.Collections.IEnumerable.GetEnumerator() => GetEnumerator();
+
+        /// <summary>The 0-based position of <paramref name="value"/> among these arguments; the
+        /// position read last when the failing value is not identifiable; <see langword="null"/> when
+        /// neither is known — an engine-internal value, which must stay an internal failure.</summary>
+        public int? PositionOf(Value? value)
+        {
+            if (value is not null)
+            {
+                for (int i = 0; i < inner.Count; i++)
+                    if (ReferenceEquals(inner[i], value))
+                        return i;
+                return null;
+            }
+
+            return LastIndex >= 0 ? LastIndex : null;
+        }
+    }
+
     private async Task<Value> CallUserFunctionAsync(FunctionDefinition fn, IReadOnlyList<Value> args)
     {
         if (args.Count != fn.Parameters.Count)
@@ -1953,6 +2015,49 @@ public sealed class Interpreter
         return v.AsVector().Select(ToLong).ToArray();
     }
 
+    /// <summary>A call-site SHAPE guard for a builtin body whose expectation the <see cref="Value"/>
+    /// accessors cannot name (a square matrix, a length-3 vector, an axis permutation). It reports
+    /// the builtin, the 1-based position, the expectation and the kind that arrived — the same
+    /// grammar as the coercion guard — so a shape mistake never surfaces as the CLR text of a
+    /// refusal raised deep inside the array kernel (audit D, finding F1).</summary>
+    private static ArrayValue RequireSquare(string builtin, IReadOnlyList<Value> args, int index)
+    {
+        var value = args[index];
+        if (value.Kind is ValueKind.Vector or ValueKind.Array
+            && value.AsArrayValue() is { Rank: 2 } matrix
+            && matrix.Shape.ToArray()[0] == matrix.Shape.ToArray()[1])
+            return matrix;
+
+        throw new BuiltinShapeException(builtin, index + 1, "a square matrix", value.Kind);
+    }
+
+    /// <summary>The length-3 operand of <c>cross</c>: the same call-site shape guard.</summary>
+    private static ArrayValue RequireLength3(string builtin, IReadOnlyList<Value> args, int index)
+    {
+        var value = args[index];
+        if (value.Kind is ValueKind.Vector or ValueKind.Array
+            && value.AsArrayValue() is { Rank: 1, Numel: 3 } vector)
+            return vector;
+
+        throw new BuiltinShapeException(builtin, index + 1, "a vector of length 3", value.Kind);
+    }
+
+    /// <summary>The axis order of <c>transpose</c>: the same call-site shape guard, applied before
+    /// the array kernel sees a permutation that is not one.</summary>
+    private static long[] RequirePermutation(string builtin, IReadOnlyList<Value> args, int index, int rank)
+    {
+        var value = args[index];
+        if (value.Kind is ValueKind.Vector or ValueKind.Array && value.AsArrayValue().Rank == 1)
+        {
+            long[] permutation = ToLongArray(value);
+            if (permutation.Length == rank && permutation.All(axis => axis >= 0 && axis < rank)
+                && permutation.Distinct().Count() == rank)
+                return permutation;
+        }
+
+        throw new BuiltinShapeException(builtin, index + 1, $"a permutation of the {rank} axes", value.Kind);
+    }
+
     /// <summary>Shared dispatcher for reduce-all (1 arg) vs reduce-along-axis (2 args) built-ins.
     /// The argument COUNT is already guaranteed by the call-site validator (the axis is the declared
     /// optional tail); the argument SHAPE is checked here, because a scalar handed to a reduction is
@@ -1967,8 +2072,7 @@ public sealed class Interpreter
     {
         var input = args[0];
         if (input.Kind is not (ValueKind.Vector or ValueKind.Array))
-            throw new ArgumentException(
-                $"{name}(): argument 1 must be an array or vector; got {input.Kind}.");
+            throw new BuiltinShapeException(name, 1, "an array or vector", input.Kind);
 
         return args.Count == 1
             ? Task.FromResult(ReduceAllOrEmpty(input, empty, all))
@@ -2067,7 +2171,7 @@ public sealed class Interpreter
             var av = args[0].AsArrayValue();
             return args.Count == 1
                 ? Task.FromResult(WrapArrayValue(av.Transpose(null)))
-                : Task.FromResult(WrapArrayValue(av.Transpose(ToLongArray(args[1]))));
+                : Task.FromResult(WrapArrayValue(av.Transpose(RequirePermutation("transpose", args, 1, av.Rank))));
         }, minArity: 1);
 
         // squeeze(a)
@@ -2094,7 +2198,8 @@ public sealed class Interpreter
         // cross(a, b)
         Register("cross", ["a", "b"], args =>
         {
-            return Task.FromResult(WrapArrayValue(TypedArrayOps.Cross(args[0].AsArrayValue(), args[1].AsArrayValue())));
+            return Task.FromResult(WrapArrayValue(TypedArrayOps.Cross(
+                RequireLength3("cross", args, 0), RequireLength3("cross", args, 1))));
         });
 
         // matmul(a, b)
@@ -2110,7 +2215,7 @@ public sealed class Interpreter
         // det(m)
         Register("det", ["m"], args =>
         {
-            var av = args[0].AsArrayValue();
+            var av = RequireSquare("det", args, 0);
             var (elements, shape) = PayloadElements(av);
             var bridge = SymbolicMatrixBridge;
             if (bridge is not null && bridge.IsSymbolicMatrix(elements, shape))
@@ -2125,7 +2230,7 @@ public sealed class Interpreter
         // trace(m)
         Register("trace", ["m"], args =>
         {
-            return Task.FromResult<Value>(TypedArrayOps.Trace(args[0].AsArrayValue()));
+            return Task.FromResult<Value>(TypedArrayOps.Trace(RequireSquare("trace", args, 0)));
         });
 
         // concat(a, b) / concat(a, b, axis) — the axis is optional
diff --git a/Lovelace.Suite/Value.cs b/Lovelace.Suite/Value.cs
index 7998a98..4de98b0 100644
--- a/Lovelace.Suite/Value.cs
+++ b/Lovelace.Suite/Value.cs
@@ -184,23 +184,23 @@ public sealed class Value
     // -----------------------------------------------------------------
 
     /// <summary>Returns the stored value cast to <see cref="Nat"/>.</summary>
-    public Nat AsNatural() => (Nat)_inner;
+    public Nat AsNatural() => Require<Nat>("a Natural");
 
     /// <summary>Returns the stored value cast to <see cref="Int"/>.</summary>
-    public Int AsInteger() => (Int)_inner;
+    public Int AsInteger() => Require<Int>("an Integer");
 
     /// <summary>Returns the stored value cast to <see cref="Rl"/>.</summary>
-    public Rl AsReal() => (Rl)_inner;
+    public Rl AsReal() => Require<Rl>("a Real");
 
-    public Cplx AsComplex() => (Cplx)_inner;
+    public Cplx AsComplex() => Require<Cplx>("a Complex");
 
-    public Lovelace.Symbolics.Expr AsSymbolic() => (Lovelace.Symbolics.Expr)_inner;
+    public Lovelace.Symbolics.Expr AsSymbolic() => Require<Lovelace.Symbolics.Expr>("a symbolic expression");
 
     /// <summary>Returns the stored value cast to <see cref="bool"/>.</summary>
-    public bool AsBoolean() => (bool)_inner;
+    public bool AsBoolean() => _inner is bool value ? value : throw new ValueShapeException("a Boolean", this);
 
     /// <summary>Returns the stored value cast to <see cref="string"/>.</summary>
-    public string AsText() => (string)_inner;
+    public string AsText() => Require<string>("text");
 
     /// <summary>Returns the stored value cast to a read-only list of values.</summary>
     public IReadOnlyList<Value> AsVector() => TypedArrayAdapter.ToElements(AsArrayValue());
@@ -209,19 +209,29 @@ public sealed class Value
     public NdArray<Value> AsArray() => TypedArrayAdapter.ToNdArray(AsArrayValue());
 
     /// <summary>Returns the stored value cast to an <see cref="ArrayValue"/>.</summary>
-    public ArrayValue AsArrayValue() => (ArrayValue)_inner;
+    public ArrayValue AsArrayValue() => Require<ArrayValue>("an array or vector");
 
     /// <summary>Returns the stored value cast to a <see cref="FunctionDefinition"/>.</summary>
-    public FunctionDefinition AsFunction() => (FunctionDefinition)_inner;
+    public FunctionDefinition AsFunction() => Require<FunctionDefinition>("a function");
 
     /// <summary>Returns the stored value cast to a <see cref="RecordValue"/>.</summary>
-    public RecordValue AsRecord() => (RecordValue)_inner;
+    public RecordValue AsRecord() => Require<RecordValue>("a record");
 
     /// <summary>Returns the stored value cast to a <see cref="MathDomain"/>.</summary>
-    public MathDomain AsDomain() => (MathDomain)_inner;
+    public MathDomain AsDomain() =>
+        _inner is MathDomain domain ? domain : throw new ValueShapeException("a domain value such as real or complex", this);
 
     /// <summary>Returns the stored value cast to an <see cref="EnumValue"/>.</summary>
-    public EnumValue AsEnum() => (EnumValue)_inner;
+    public EnumValue AsEnum() => Require<EnumValue>("an enumerated value");
+
+    /// <summary>The ONE coercion guard behind every <c>As…</c> accessor: the value is not the KIND
+    /// the caller requires. It raises <see cref="ValueShapeException"/> — carrying the expectation
+    /// and this value — instead of letting a raw CLR cast escape as <c>InvalidCastException</c>,
+    /// which the runner can only report as an internal invariant failure (audit D, finding F1). The
+    /// call-site guard in <see cref="Interpreter"/> turns it into the documented recoverable
+    /// argument error naming the builtin, the argument position and the kind that arrived.</summary>
+    private T Require<T>(string expected) where T : class =>
+        _inner as T ?? throw new ValueShapeException(expected, this);
 
     // -----------------------------------------------------------------
     // Widening
@@ -310,3 +320,32 @@ public sealed class Value
         _                 => throw new InvalidOperationException($"Unknown kind: {Kind}"),
     };
 }
+
+/// <summary>
+/// A coercion failure inside the engine: the value is not the KIND the caller requires, so the
+/// <c>As…</c> accessors on <see cref="Value"/> raise this instead of a raw CLR cast — with the
+/// expectation in the message rather than the framework's "Specified cast is not valid.".
+/// <para>
+/// The call-site guard in <see cref="Interpreter"/> catches it for a builtin body and re-raises
+/// <see cref="BuiltinShapeException"/> (an <see cref="ArgumentException"/>, so the wire carries the
+/// documented <c>InvalidArgument</c>/<c>TypeMismatch</c>), naming the builtin, the 1-based argument
+/// position and the kind that arrived. <see cref="Offender"/> is the value that failed the
+/// coercion: when it is one of the call's arguments the attribution is exact. A failure on an
+/// ENGINE-INTERNAL value is not a caller mistake and keeps crossing as it did before — an internal
+/// invariant failure — so a genuine engine bug is never disguised as a user error. Deriving from
+/// <see cref="InvalidCastException"/> keeps that escape hatch (and every existing host that catches
+/// the framework type) behaving exactly as it did.
+/// </para>
+/// </summary>
+internal sealed class ValueShapeException(string expected, Value? offender = null) : InvalidCastException(
+    offender is null
+        ? $"a value is not {expected}."
+        : $"a value of kind {offender.Kind} is not {expected}.")
+{
+    /// <summary>The kind the caller required, as prose: <c>an array or vector</c>, <c>a Real</c>.</summary>
+    public string Expected { get; } = expected;
+
+    /// <summary>The value that failed the coercion, or <see langword="null"/> when the failing value
+    /// is not a single <see cref="Value"/> (a shape check inside the array kernel, for example).</summary>
+    public Value? Offender { get; } = offender;
+}
diff --git a/Lovelace.Symbolics/SymbolicsPlugin.cs b/Lovelace.Symbolics/SymbolicsPlugin.cs
index c3097f1..aa46a63 100644
--- a/Lovelace.Symbolics/SymbolicsPlugin.cs
+++ b/Lovelace.Symbolics/SymbolicsPlugin.cs
@@ -189,9 +189,12 @@ public sealed class SymbolicsPlugin : IModusPlugin, ISymbolicMatrixBridge, ISymb
         }
 
         Add("symbol", new[] { "name", "domain" }, args =>
-            args.Count >= 2 && args[1] is MathDomain md
-                ? SymbolWithDomain((string)args[0]!, md)
-                : Exprs.Symbol((string)args[0]!),
+        {
+            var name = AsTextName(args[0]);
+            return args.Count >= 2 && args[1] is MathDomain md
+                ? SymbolWithDomain(name, md)
+                : Exprs.Symbol(name);
+        },
             new BuiltinDescriptor("symbol", new[] { "name", "domain" }, BuiltinCategories.Symbolics,
                 "Creates a symbolic variable; an optional domain (integer/rational/real/complex) is assumed for it.",
                 ["symbol(\"x\")", "symbol(\"x\", real)"], "Symbolic", ["assume", "real", "complex"], MinArity: 1));
@@ -243,7 +246,7 @@ public sealed class SymbolicsPlugin : IModusPlugin, ISymbolicMatrixBridge, ISymb
         }
         Add("assume", new[] { "relation" }, args =>
         {
-            var rel = (Expr)args[0]!;
+            var rel = AsRelation(args[0]);
             if (!AssumeRecursive(rel))
                 throw new InvalidOperationException("assume() accepts relations and their conjunctions/negations with constant bounds.");
             return rel;
@@ -1499,7 +1502,7 @@ public sealed class SymbolicsPlugin : IModusPlugin, ISymbolicMatrixBridge, ISymb
             _assumptions = _assumptions.Add(new ExpressionPropertyAssumption(e, pred));
             return e;
         }
-        var s = Context.Symbol((string)args[0]!);
+        var s = Context.Symbol(AsSymbolName(args[0], AssumeExpectation));
         _assumptions = _assumptions.Add(new SymbolPropertyAssumption(s, pred));
         return Exprs.Symbol(s);
     }
@@ -1511,7 +1514,7 @@ public sealed class SymbolicsPlugin : IModusPlugin, ISymbolicMatrixBridge, ISymb
             _assumptions = _assumptions.Add(new SymbolDomainAssumption(sx.Symbol, domain));
             return e;
         }
-        var s = Context.Symbol((string)args[0]!);
+        var s = Context.Symbol(AsSymbolName(args[0], AssumeExpectation));
         _assumptions = _assumptions.Add(new SymbolDomainAssumption(s, domain));
         return Exprs.Symbol(s);
     }
@@ -1681,6 +1684,32 @@ public sealed class SymbolicsPlugin : IModusPlugin, ISymbolicMatrixBridge, ISymb
         throw new BuiltinArgumentError("a symbolic variable");
     }
 
+    /// <summary>What an assume* builtin requires when the payload is not already symbolic: a text
+    /// name (or a symbolic expression). Shared so the expectation text and the refusal variant
+    /// cannot drift between the six assume* surfaces.</summary>
+    private const string AssumeExpectation = "a symbolic expression or the text name of one";
+
+    /// <summary>The text NAME an argument must carry where the builtin declares one
+    /// (<c>symbol("x")</c>). A payload of another kind is a wrong-SHAPED argument, so it is refused
+    /// through <see cref="BuiltinShapeError"/> — the documented recoverable argument error naming
+    /// the builtin, the position and the kind that arrived — instead of the raw
+    /// <c>(string)args[0]!</c> cast that used to escape as an internal invariant failure carrying
+    /// "Specified cast is not valid." (audit D, finding F1).</summary>
+    private static string AsTextName(object? o) =>
+        o as string ?? throw new BuiltinShapeError("a text name such as \"x\"");
+
+    /// <summary>The same guard for the name form of the assume* builtins, which also accept an
+    /// already-symbolic argument (handled before this helper is reached).</summary>
+    private static string AsSymbolName(object? o, string expected) =>
+        o as string ?? throw new BuiltinShapeError(expected);
+
+    /// <summary>The relation an <c>assume</c> argument must carry. A non-expression payload is a
+    /// wrong SHAPE, not a domain refusal: it crosses as the documented recoverable argument error
+    /// naming the builtin, the position and the kind that arrived — never as the raw
+    /// <c>(Expr)args[0]!</c> cast failure it used to cross as (audit D, finding F1).</summary>
+    private static Expr AsRelation(object? o) =>
+        o as Expr ?? throw new BuiltinShapeError("a relation such as x > 5");
+
     private static Expr AsExpr(object? o) => o switch
     {
         Expr e => e,
```

The two new test files are not repeated here; the complete patch including them (836 lines;
SHA-256 `BE1D5C8F0E2EFFFA58A1684F9D06AF7DD169C665D99C2A3D6CBB63B56BA44A8D`, regenerated from the scratch
tree and hash-compared after the last edit) is `f1-diff.patch`.

## 5. Sweep totals, before and after

Both tables come from the same evidence harness (a temporary test that drives every builtin x the three
audit shapes through `Runner.RunAsync` and dumps one TSV row per call): `shape-sweep-before.tsv` (pristine
tree) and `shape-sweep-after-final.tsv` (final tree). Rows = 123 builtins (8 of them, e.g. `pi`, `e`, declare
no argument position and are recorded as such).

| metric | before | after |
|---|---|---|
| builtins swept | 123 | 123 |
| probe calls (3 wrong shapes x every declared position) | 345 | 345 |
| accepted calls | 113 | 113 |
| refusals | 232 | 232 |
| **refusals that were `InternalError`/`InternalInvariantFailure`** | **52** | **0** |
| ... spanning how many builtins | **26** | **0** |
| refusals in the documented family (`InvalidArgument`/`TypeMismatch`) | 27 | **79** |
| refusals in `InvalidOperation`/`DomainError` (frozen, see section 8) | 153 | 153 |

The 26 builtins, before (from `shape-sweep-before.tsv`): `append, assume, assume_integer, assume_negative,
assume_nonnegative, assume_positive, assume_real, concat, cross, det, dot, flatten, inv_full, linsolve,
linsolve_full, matmul, matrix_rank, ndims, numel, rank, reshape, shape, squeeze, symbol, trace, transpose` —
the audit's list, exactly.

The wider delivered sweep is 8 shapes x 115 positioned builtins = **920 probes** and normally reports the same
result (the test asserts it, and refuses to pass below 800 probes). An exploratory dump of only the 5 ADDED
shapes (a Record, a 2x2 matrix, text, a Boolean, an empty vector) is `shape-sweep-wide.tsv` with
`shape-sweep-wide-summary.txt` reading `probes=575 accepted=117 refused=458 internal=0`: **0 internal
failures, 0 unrecoverable refusals, 0 raw-framework messages** across those 575 calls as well.

## 6. Control: the new tests on a pristine HEAD tree

```powershell
git -C C:/Users/ricar/dev/LovelaceSharp worktree add .worktrees/c6-f1-ctl HEAD
Copy-Item .worktrees/c6-f1/Lovelace.Run.Tests/BuiltinArgumentShapeSweepTests.cs   .worktrees/c6-f1-ctl/Lovelace.Run.Tests/
Copy-Item .worktrees/c6-f1/Lovelace.Suite.Tests/BuiltinArgumentShapeGuardTests.cs .worktrees/c6-f1-ctl/Lovelace.Suite.Tests/
cd .worktrees/c6-f1-ctl
dotnet test Lovelace.Run.Tests/Lovelace.Run.Tests.csproj     -c Release --nologo --filter "FullyQualifiedName~BuiltinArgumentShapeSweepTests"
dotnet test Lovelace.Suite.Tests/Lovelace.Suite.Tests.csproj -c Release --nologo --filter "FullyQualifiedName~BuiltinArgumentShapeGuardTests"
```

| control run | result |
|---|---|
| `Lovelace.Run.Tests` on pristine `141f34e` + only the new sweep test | **Failed: 6, Passed: 9** — `56 of 232 refusals (345 probes over 123 builtins) violate the frozen argument-shape contract (52 crossed as internal invariant failures)`; the five audit calls report `Expected: "InvalidArgument"  Actual: "InternalError"` (`control-run-run-tests.txt`) |
| `Lovelace.Suite.Tests` on pristine `141f34e` + only the mechanism test | **Failed: 3, Passed: 0** — the message is `Unable to cast object of type ...` instead of `probe(): argument 2 must be ...`, `Assert.IsAssignableFrom() Failure: Value is an incompatible type`, and the engine-internal case raises `NullReferenceException` instead of the `InvalidCastException` family (`control-run-suite-tests.txt`) |

Both files therefore **fail on the unfixed tree and pass on the fixed one** — the tests are falsifiable and
the fix is what makes them pass.

## 7. Whole solution and the Timing subsets

`cd .worktrees/c6-f1; dotnet test LovelaceSharp.slnx --configuration Release --nologo -m:1` — transcript
`full-suite-after.txt`. Every project: **0 failed**.

| project | passed | failed | skipped |
|---|---|---|---|
| Lovelace.Abstractions.Tests | 20 | 0 | 0 |
| Lovelace.Array.Tests | 19 | 0 | 0 |
| Lovelace.Complex.Tests | 115 | 0 | 0 |
| Lovelace.Console.Tests | 15 | 0 | 0 |
| Lovelace.Dsp.Tests | 61 | 0 | 0 |
| Lovelace.Integer.Tests | 148 | 0 | 0 |
| Lovelace.Knowledge.Tests | 28 | 0 | 0 |
| Lovelace.Natural.Tests | 195 | 0 | 0 |
| Lovelace.Real.Tests | 2495 | 0 | 0 |
| Lovelace.Representation.Tests | 91 | 0 | 0 |
| Lovelace.Run.Tests | 222 | 0 | 0 |
| Lovelace.Studio.Tests | 22 | 0 | 0 |
| Lovelace.Suite.Tests | 828 | 0 | 0 |
| Lovelace.Symbolics.Tests | 1119 | 0 | 8 |
| precbench.Tests | 13 | 0 | 0 |
| **total** | **5391** | **0** | **8** |

Timing subsets (`[Trait("Category","Timing")]`; filter `--filter "Category=Timing"`):

| project | result |
|---|---|
| Lovelace.Suite.Tests | Passed 8, Failed 0 |
| Lovelace.Run.Tests | Passed 5, Failed 0 |
| Lovelace.Real.Tests | Passed 1, Failed 0 |
| Lovelace.Symbolics.Tests | Passed 1, Failed 0 |

(A `--filter "FullyQualifiedName~Timing"` run reports 16/2/0/0 because the Real and Symbolics timing cases
are named elsewhere; the trait filter above is the one that selects them — `RealTrigFastPathTests.cs:269`,
`AssumptionAddScalingTests.cs:104`, `ArrayKernelCancellationTests.cs:23`, `CancellationObservationTests.cs:19`,
`CancellationBudgetTests.cs:24`.)

## 8. Could not verify, or deliberately not changed

1. **The 153 `InvalidOperation`/`DomainError` refusals of the same sweep are untouched.** They are a
   *frozen* family, not an oversight: `Lovelace.Symbolics.Tests/CapabilitiesBuiltinTests.cs:69-73` advertises
   `solve.non-symbolic-variable`, `solve.non-symbolic-expression`, `diff.non-symbolic-variable`,
   `integrate.non-symbolic-variable` and `limit.non-symbolic-variable` with exactly
   `InvalidOperation`/`DomainError`, and `CapabilitiesBuiltinTests.AdvertisedCodesAndCategories_MatchTheLiveEnvelope`
   compares them against the live envelope. I measured the alternative: routing the shared
   `BuiltinArgumentError` conversion through `ArgumentException` instead of `InvalidOperationException`
   (`SymbolicsPlugin.cs:184-187`) turns 63 more refusals into `TypeMismatch` and fails that test with 5
   mismatched entries — so it would require weakening a frozen assertion. It is therefore **not** part of
   this repair, and the delivered sweep asserts the unconditional invariants for those refusals (never
   internal, always recoverable, never raw framework text) rather than a code they are advertised not to
   carry. `limit*` was left untouched entirely, as instructed.
2. **Two message-grammar outliers remain in the WIDER lattice** (visible in `shape-sweep-wide.tsv`, and why
   the wider delivered test asserts no grammar): `dot() operands must be rank-1 vectors.`
   (`Lovelace.Suite/TypedArrayOps.cs:57`, reached by `dot([[1,2],[3,4]], ...)`; in scope but outside the 52)
   and `filter(...)`'s `The denominator coefficient vector 'a' must not be empty. (Parameter 'a')`
   (`Lovelace.Dsp` — **outside the allowed scope**). Neither crosses as an internal failure.
3. **Anonymous refusals from plugins that do not name their builtin** (11 x `DSP builtins expect an array
   argument, but got a scalar.` from `Lovelace.Dsp/DspPlugin.cs:423`, 6 x `Expected a symbolic expression.`
   from the MathIR plugin) remain as they were: they are `DomainError`, recoverable, and their prose is
   owned by `Lovelace.Dsp` / `Lovelace.MathIR`, which are **outside the allowed scope**. They are the
   reason the sweep scopes its grammar assertion to the `InvalidArgument` family.
4. **The guard converts a coercion failure only when the failing value IS one of the call's arguments.**
   A builtin that coerces a `Value` it built itself still crosses as an internal failure — deliberately, so
   a genuine engine bug is not disguised as a user error; pinned by
   `BuiltinArgumentShapeGuardTests.ACoercionFailureOnAnEngineInternalValue_IsNotDisguisedAsAnArgumentError`.
   No probe of the 3-shape (345) or 8-shape (920) sweep reaches that path, but the space is not exhausted.
5. **Not the full type lattice.** 8 shapes out of the cross-product of every kind x every position; the
   claim is "0 of 345 (and 0 of the 5 extra shape families)", not "0 for every possible wrong shape".
6. **The main tree's `out/aot/Lovelace.Run.exe` was left as the pre-fix artifact** (5 776 896 bytes,
   15:06:37) so the audit's binary stays reproducible; the fixed AOT binary lives in the scratch tree at
   `.worktrees/c6-f1/out-aot-f1/`. Re-publishing the main tree's `out/aot` is a one-command follow-up:
   `dotnet publish Lovelace.Run/Lovelace.Run.csproj -c Release -p:PublishAot=true -p:InvariantGlobalization=true -o out/aot`.
7. **`Lovelace.Array` / `Lovelace.Abstractions` were not touched**, although `transpose`'s permutation
   validation lives there (`Lovelace.Array/NdArray.cs:229-243`, `Lovelace.Abstractions/DenseArray.cs:105-116`):
   the call-site guard in `Interpreter.cs:2047` now runs first for the `transpose` builtin, so that path is
   covered without changing the array kernel. Other callers of `ArrayValue.Transpose` (the Dsp/MathIR
   plugins, Studio) still get the kernel's own `ArgumentException`, unchanged.
8. **`Lovelace.Suite.Tests` line numbers cite the fixed tree**; the same code is at the cited `git diff`
   hunk positions.

## 9. Evidence index (all in `docs/goal-cycle-6/round-14/`)

| file | what it is |
|---|---|
| `aot-five-calls-before.txt` | the five calls on the published pre-fix AOT binary (5 776 896 bytes, 15:06:37) |
| `aot-five-calls-after.txt` | the five calls on the fixed AOT binary (5 793 792 bytes, 15:46:21) |
| `sweep-before-test-output.txt` | the failing-first transcript (58 violations, 52 internal, 6 failed / 9 passed) |
| `shape-sweep-before.tsv` / `shape-sweep-after-final.tsv` | the per-call sweep tables (353 rows each) |
| `shape-sweep-wide.tsv` / `shape-sweep-wide-summary.txt` | the 5 extra shape families (575 probes, 0 internal) |
| `control-run-run-tests.txt` / `control-run-suite-tests.txt` | the two control runs on pristine `141f34e` |
| `full-suite-after.txt` | the whole-solution run (15 projects, 0 failed) |
| `f1-diff.patch` | the complete patch, production code + the two new test files |
