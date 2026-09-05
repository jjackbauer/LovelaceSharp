# Modus Plugin Design

> Status: design proposal (partially landed; contract corrected — see §2)
> Scope: LovelaceSharp — the extension ("Modus") boundary and the rules for authoring plugins.
> Traceability: MOD-001..006, KRN-001..005, ARR-001..003, STO-001..002.

---

## 1. Executive summary

A **Modus plugin** extends the Lovelace language by *composing Lovelace's own numeric
constructs*, never by reaching for machine types. Its base is:

- **`IField<T>`** — the element-arithmetic seam (`Add`, `Subtract`, `Multiply`, `Divide`,
  `Negate`, `IsZero`, `Compare`, `Sqrt`, `Zero`, `One`, `FromLong`) that already parameterizes
  the repo's generic array algorithms.
- **`ArrayValue` / `DenseArray<T>`** — the homogeneous N-dimensional container.
- **`Natural` / `Integer` / `Real`** — the arbitrary-precision scalars, used directly when a
  computation needs more than field arithmetic (e.g. `sin`, `exp`, `gcd`, factorial, digit work).

A plugin registers **builtins** (named functions callable from Lovelace source) and **kernels**
(optimized implementations of existing operations for a concrete scalar type), all through
`Lovelace.Abstractions`, and all **exact** unless the plugin explicitly opts into a machine-type
fast path (§8).

> **Golden rule.** The core owns the `Value` ↔ typed-data mapping. A plugin never sees
> `Value`, `ValueKind`, `Ast`, `Parser`, or `Interpreter`; it sees `IField<T>` + typed arrays +
> the exact scalar types.

---

## 2. Root-cause correction: why the proof plugin used `double`

The landed proof plugin (`Lovelace.Statistics/StatisticsPlugin.cs`) registers a `double` add
kernel. That is **wrong** for this project and must not be treated as the model. It happened
because of a contract defect, not a plugin decision:

- The kernel interface is declared `IArrayKernel<T> where T : unmanaged`.
- `Natural`, `Integer`, and `Real` are **reference types** (`Natural.cs` / `Integer.cs` /
  `Real.cs` declare `sealed class`/`class`). They are therefore **not** `unmanaged`.
- Result: the only element types the interface would accept are machine types (`double`,
  `float`, `int`, …). A proof plugin that wanted *any* element type was steered to `double`.

This document **removes the `unmanaged` constraint** and re-bases the kernel seam on
`IField<T>` (§5), so plugins operate on `Natural`/`Integer`/`Real` — the same scalars the
language computes with.

### 2.1 The two facts that make the correction possible

1. **The element seam already exists and is generic over arbitrary types.** `IField<T>`
   (`Lovelace.Array/IField.cs`) has no `unmanaged` constraint. `ArrayMath` is generic over it
   (`Sum`, `Prod`, `Mean`, `Min`, `Max`, `Norm`, `Dot`, `Cross`, `MatMul`, `Det`, `Inverse`,
   `Trace` — `Lovelace.Array/ArrayMath.cs`). This is the template for what "adding a
   computation on top of Lovelace's constructs" looks like.
2. **A typed exact field is one small step away.** `RealField : IField<Real>` already exists in
   `arraybench/Program.cs`; the production gap is only that `RealField` / `IntegerField` /
   `NaturalField` do not yet live in the numeric projects as reusable library types (§9.2).

### 2.2 The container seam

`ArrayValue` / `DenseArray<T>` (`Lovelace.Abstractions`) is already typed and
`IsAotCompatible`. `DenseArray<T>` is `T[]`-backed and exposes `AsSpan()`, so a plugin can
iterate `ReadOnlySpan<Real>` exactly as it would a `double[]` — without leaving exact
arithmetic.

---

## 3. Current state (landed vs not)

### 3.1 Landed

- `Lovelace.Abstractions`: `ArrayValue`, `DenseArray<T>`, `DType { Natural, Integer, Real }`,
  `Precision`, `Slice`/`IndexSpec`, and the *current* plugin contract
  (`ArrayOp`, `IArrayKernel<T> where T : unmanaged`, `IModusContext`, `IModusPlugin`).
- `Lovelace.Suite/ModusHost.cs`: the interpreter-aware adapter; `SuiteEngine.LoadPlugin` +
  `TryDispatchKernel`.
- `Lovelace.Statistics/StatisticsPlugin.cs` + `Lovelace.Suite.Tests/ModusTests.cs` (both
  `double`-based, to be replaced).

### 3.2 Not landed (the gaps this document specifies)

1. Kernel dispatch is **not wired** into the interpreter's elementwise path.
2. The kernel seam is `unmanaged`-only (wrong; §2).
3. `IField<T>` lives in `Lovelace.Array`, outside the contract assembly plugins target (§9.1).
4. No production `IField<Real>` / `IField<Integer>` / `IField<Natural>` (§9.2).
5. Arrays are still stored as `DenseArray<Value>` (boxed), not `DenseArray<Real>` etc. (§9.3).
6. No scalar / multi-argument builtin channel (`mean`, `dot`) — only unary array→array (§5.4).
7. No linear-algebra / reduction backend interface (§5.5).
8. No machine-type dtype distinct from `DType.Real` (§8).

---

## 4. The compute base: Lovelace's own constructs

The plugin's base is a three-layer stack. "Adding a computation" means composing the layers
below; you only drop to concrete scalars when the layer above can't express the operation.

```
layer 3  concrete scalars      Natural / Integer / Real      (sin, exp, gcd, factorial, digits)
layer 2  generic algorithms    Sum/Prod/Mean/Norm/Dot/MatMul/Det/Inv  (over IField<T> + DenseArray<T>)
layer 1  element arithmetic    IField<T>                      (Add/Subtract/Multiply/Divide/…)
layer 0  container + metadata  ArrayValue / DenseArray<T> / DType / Precision / Shape / Strides
```

- **layer 0** answers "what am I looking at" (`Shape`, `DType`, `Precision`, `IsContiguous`).
- **layer 1** answers "how do two elements combine" (`IField<T>`).
- **layer 2** answers "how do whole arrays reduce/transform" (the generic algorithms).
- **layer 3** answers "what if I need an operation `IField<T>` doesn't have" (use `Real`/`Integer`/`Natural` directly).

A statistics plugin (`mean`, `variance`, `std`, `sum`, `prod`, `min`, `max`) lives entirely in
**layers 1–2** — it never names a concrete type, so it is exact for `Natural`, `Integer`, and
`Real` simultaneously. A special-functions plugin (`sin`, `gamma`) lives in **layer 3**, using
`Real`'s own arithmetic under the interpreter's precision scope.

---

## 5. Corrected contract (`Lovelace.Abstractions`)

### 5.1 `IField<T>` becomes part of the contract (move it here)

`IField<T>` is currently in `Lovelace.Array` (`namespace Lovelace.Arrays`). It **must move
into `Lovelace.Abstractions`** (or be re-exported by it), because `MOD-006` makes it the
minimal scalar contract plugins target and `MOD-001` forbids plugins from depending on
anything else. Its shape stays:

```csharp
namespace Lovelace.Abstractions;

public interface IField<T>
{
    T Zero { get; }
    T One { get; }
    T FromLong(long value);
    T Add(T a, T b);
    T Subtract(T a, T b);
    T Multiply(T a, T b);
    T Divide(T a, T b);
    T Negate(T a);
    bool IsZero(T a);
    int Compare(T a, T b);
    T Sqrt(T a);
}
```

### 5.2 Kernel seam — over `IField<T>`, not `unmanaged`

```csharp
namespace Lovelace.Abstractions;

public enum ArrayOp { Add, Subtract, Multiply, Divide }

/// <summary>A pluggable elementwise kernel for a concrete scalar type.
/// Declines by returning false; the caller then runs the reference backend.</summary>
public interface IFieldKernel<T>
{
    DType DType { get; }
    bool TryElementwise(ArrayOp op, ReadOnlySpan<T> left, ReadOnlySpan<T> right, Span<T> result, IField<T> field);
}
```

Key differences from the landed `IArrayKernel<T>`:

- **No `unmanaged` constraint** — `T` may be `Natural`, `Integer`, `Real` (reference types).
- **The field is injected** by the core, so the plugin composes exact arithmetic
  (`field.Add`) instead of hard-coding a machine `+`. The core hands the *real* field
  (`RealField` etc.), which inherits the active precision scope.

### 5.3 Registration surface

```csharp
namespace Lovelace.Abstractions;

public interface IModusContext
{
    // landed, kept
    void RegisterArrayBuiltin(string name, Func<ArrayValue, ArrayValue> implementation);

    // corrected (was RegisterKernel<T>(IArrayKernel<T>) where T : unmanaged)
    void RegisterKernel<T>(IFieldKernel<T> kernel);

    // proposed — scalar and multi-argument builtins (§5.4)
    void RegisterScalarBuiltin(string name, Func<ArrayValue, ScalarResult> implementation);
    void RegisterBuiltin(string name, IReadOnlyList<string> parameters,
                         Func<IReadOnlyList<ArrayValue>, ScalarResult> implementation);
}

public interface IModusPlugin
{
    string Name { get; }
    void Register(IModusContext context);
}
```

### 5.4 `ScalarResult` — non-array results without importing `Value`

A plugin may need to return a scalar (a `mean` returns an exact `Real`). It must do so without
touching `Value`. A narrow opaque result type carries the numeric subset; the core converts it:

```csharp
namespace Lovelace.Abstractions;

public readonly struct ScalarResult
{
    // static factories: FromNatural(...), FromInteger(...), FromReal(...),
    //                   FromBoolean(bool), FromText(string), FromArray(ArrayValue)
    // payload is opaque; the core maps it onto a Value at the boundary.
}
```

### 5.5 Linear-algebra / reduction backend (Stage-5 decision, KRN-005)

```csharp
public interface ILinearAlgebraBackend<T>
{
    DType DType { get; }
    bool TryMatMul(ReadOnlySpan<T> a, ReadOnlySpan<long> aShape,
                   ReadOnlySpan<T> b, ReadOnlySpan<long> bShape,
                   Span<T> result, IField<T> field);
    // TrySolve / TryDet / TryInverse — each declining by returning false
}
```

The reference implementation is the generic layer of §4 (ported to `DenseArray<T>`), so the
"reference backend" and the "plugin backend" are literally the same algorithms.

---

## 6. Worked examples (exact, no `double`)

### 6.1 A statistics plugin, generic over `IField<T>` (layers 1–2)

```csharp
using Lovelace.Abstractions;
using Rl = Lovelace.Real.Real;

namespace Lovelace.Statistics;

public sealed class StatisticsPlugin : IModusPlugin
{
    public string Name => "Lovelace.Statistics";

    public void Register(IModusContext context)
    {
        // exact mean over whatever scalar type the array holds — the core injects IField<T>
        context.RegisterScalarBuiltin("mean", a => Mean(a));
        // an exact elementwise-add kernel for Real (layer 1), not a double fast path
        context.RegisterKernel(new RealAddKernel());
    }

    // layer 2: composed from IField<T> only. Works for Natural, Integer, and Real.
    internal static T Mean<T>(IField<T> f, ReadOnlySpan<T> data)
    {
        T sum = f.Zero;
        foreach (var x in data) sum = f.Add(sum, x);
        return f.Divide(sum, f.FromLong(data.Length));
    }

    // layer 1: exact Real add, inheriting the interpreter's precision scope via the injected field
    private sealed class RealAddKernel : IFieldKernel<Rl>
    {
        public DType DType => DType.Real;
        public bool TryElementwise(ArrayOp op, ReadOnlySpan<Rl> left, ReadOnlySpan<Rl> right,
                                   Span<Rl> result, IField<Rl> field)
        {
            if (op != ArrayOp.Add || left.Length != right.Length || right.Length != result.Length)
                return false;
            for (int i = 0; i < left.Length; i++)
                result[i] = field.Add(left[i], right[i]);
            return true;
        }
    }
}
```

### 6.2 A special-functions plugin over `Real` directly (layer 3)

`IField<T>` has no `Sin`/`Exp`/`Pow`. A plugin that needs them targets `Real`'s own arithmetic,
still under the engine's precision:

```csharp
using Lovelace.Abstractions;
using Rl = Lovelace.Real.Real;

namespace Lovelace.Special;

public sealed class SpecialPlugin : IModusPlugin
{
    public string Name => "Lovelace.Special";

    public void Register(IModusContext context)
    {
        context.RegisterScalarBuiltin("sin", a => Sin(a)); // a is DenseArray<Rl>
    }

    private static ScalarResult Sin(ArrayValue a)
    {
        // Rl arithmetic — exact to the active precision, computed with Real's own primitives
        // (x - x^3/3! + x^5/5! - ... via Rl multiplication/division, under WithPrecision).
        // Returns ScalarResult.FromReal(...).
        throw new NotImplementedException("illustrative");
    }
}
```

### 6.3 Script surface

```text
> mean(1..5)
= 3 (Real)

> sin([0, pi/2])
= [0, 1] (Array)
```

`mean` is exact (the injected `RealField`/`IntegerField`/`NaturalField` does the division); it
does not round through a `double`.

### 6.4 Host wiring

```csharp
var engine = new SuiteEngine();
engine.LoadPlugin(new StatisticsPlugin());
engine.LoadPlugin(new SpecialPlugin());
```

---

## 7. Dispatch semantics

1. **Fallible.** `TryElementwise`/`TryMatMul` return `false` to decline (wrong op, shape,
   dtype, precision, or length). The core then runs the reference backend — the generic
   `IField<T>` algorithms — which produce the canonical result (KRN-003, P3).
2. **Field injection.** The core owns the concrete fields (`RealField`, `IntegerField`,
   `NaturalField`) and passes the correct one per the array's `DType`. The plugin never
   constructs or selects a field, so it cannot accidentally use the wrong arithmetic.
3. **Precision inherits.** Evaluation already runs inside `Rl.WithPrecision`
   (`Interpreter.EvaluateAsync`). A plugin kernel over `Real` therefore inherits the active
   precision with no extra work; it must not set its own.
4. **Typed dispatch.** The core dispatches on the *stored* element type
   (`DenseArray<Real>` → `IFieldKernel<Real>`, `DenseArray<Integer>` → `IFieldKernel<Integer>`),
   so an exact kernel is never silently substituted for a machine one.
5. **Order.** Registration order is the selection order; a kernel must be defensive about
   what it accepts (same as the reference `TryDispatch` loop in `ModusHost`).

---

## 8. Machine types are an explicit, non-default opt-in

There is **one** legitimate place for `double`/`float` in this design: an *opt-in* fast path for
hot loops, exactly as `typed-array-benchmark-baseline.md` concluded ("`Real` stays the default;
machine types are the opt-in fast path"). It is never the plugin's primary base, and it is never
confused with `DType.Real`.

Rules for the machine path:

1. Introduce a **distinct dtype** (`DType.F64`, later `F32`) that does **not** overlap
   `DType.Real`. A `double` kernel declares `DType.F64`.
2. A machine kernel is selected **only** when the array is genuinely `DenseArray<double>`
   (or the user has explicitly requested a machine backend). It is never applied to
   `DenseArray<Real>`.
3. Crossing into a machine backend is a **documented precision sacrifice** the user opted into,
   surfaced in the language (e.g. an explicit `as f64` / `@machine` marker), not an implicit
   downgrade.

This is why the landed `DoubleAddKernel` claiming `DType.Real` is a bug: it silently replaced
arbitrary-precision results with 15-digit doubles. The correction is not "gate the double"; it
is "build on `Real`, and only offer `double` as a separately-typed, explicitly-requested backend."

---

## 9. Required code changes (to make §4–§8 real)

### 9.1 Move `IField<T>` into `Lovelace.Abstractions`

Relocate `Lovelace.Array/IField.cs` → `Lovelace.Abstractions` (namespace
`Lovelace.Abstractions`), and have `Lovelace.Array` reference `Lovelace.Abstractions`. This
makes `IField<T>` reachable by plugins without a second dependency (MOD-001/006).

### 9.2 Add `RealField` / `IntegerField` / `NaturalField` as library types

Promote the benchmark's `RealField : IField<Rl>` (`arraybench/Program.cs`) into
`Lovelace.Real`; add `IntegerField` in `Lovelace.Integer` and `NaturalField` in
`Lovelace.Natural`. These are the *exact* identities the core injects into plugins. Each is a
singleton and AOT-safe (no reflection).

### 9.3 Complete typed storage

Arrays must be stored as `DenseArray<Real>` / `DenseArray<Integer>` / `DenseArray<Natural>`
(homogeneous), not `DenseArray<Value>` (boxed) — this is the typed-array plan's Stage 3
(ARR-001). A kernel over `Real` only becomes meaningful once arrays actually hold `Real`.

### 9.4 Replace the kernel contract

- Delete `IArrayKernel<T> where T : unmanaged`; add `IFieldKernel<T>` (§5.2).
- `ModusHost.RegisterKernel<T>` / `TryDispatch<T>` drop the `unmanaged` constraint and pass an
  `IField<T>` argument.

### 9.5 Wire dispatch into the interpreter

Call `TryDispatch<T>` from the interpreter's elementwise path with the array's stored element
type and the matching field, falling back to the reference backend on `false`. This is the
change that turns a loaded plugin from *testable* into *actually accelerating the language*.

### 9.6 Add the scalar / multi-argument builtin channels (§5.4)

---

## 10. AOT and the build model

- Every plugin and `Lovelace.Abstractions` is `net10.0`, `<IsAotCompatible>true</IsAotCompatible>`
  (already true for the two landed projects).
- **No reflection, no `Assembly.Load`, no `dynamic`, no `MakeGenericType` over plugin types.**
  Plugin discovery is compile-time: explicit `LoadPlugin(...)` calls, or a source generator over a
  `[LovelacePlugin]` attribute emitting `PluginRegistry.LoadAll(engine)`.
- Serialization (config, process-boundary) uses `System.Text.Json` source generation
  (`StudioJsonContext`/`RunJsonContext` pattern).
- A plugin is a compile-time-linked library, shipped in the `PublishAot` binary — not a
  runtime-loaded DLL. Runtime IL loading would force a CoreCLR host, outside this repo's
  Native-AOT identity.

---

## 11. Guidelines

### 11.1 Plugin author

- [ ] Reference only `Lovelace.Abstractions` (+ a scalar project only when implementing
      layer-3 functions over `Real`/`Integer`/`Natural`).
- [ ] Compose `IField<T>` + `DenseArray<T>`/`ArrayValue` + the generic algorithms. Do not
      introduce `double`/`float` unless it is an explicitly-typed machine backend (§8).
- [ ] Kernels are pure, stateless, and **decline** (`return false`) rather than throw for
      unsupported op/dtype/shape/precision/length.
- [ ] Never construct a field; receive it. Never set a precision scope; inherit it.
- [ ] Array results are immutable and rank-tagged by the core, not the plugin.
- [ ] `net10.0`, `IsAotCompatible=true`, source-generated JSON, no reflection/`dynamic`.

### 11.2 Core maintainer

- [ ] Every channel goes through `IModusContext`/`ModusHost`; plugins never see
      `Interpreter`/`Value`/`Ast`/`Parser`.
- [ ] The core owns: unwrap (`Value.AsArrayValue()`), field selection, wrap
      (`Interpreter.WrapArrayValue`), arity, and error normalization.
- [ ] Dispatch is fallible; reference backend is the generic `IField<T>` layer.
- [ ] A new `DType` is ordered correctly for `Natural → Integer → Real` and documented.
- [ ] Machine dtypes (`F64`/`F32`) are kept visually and semantically distinct from `Real`.
- [ ] Contract is additive-only; bump `Lovelace.Abstractions` per semver.

### 11.3 AOT

- [ ] `PublishAot=true` build stays green (Makefile `build`/`runner`/`studio`/`knowledge`).
- [ ] Zero trim warnings (`IL2026`/`IL3050`/`IL2104`).
- [ ] No `MakeGenericType`/`Activator.CreateInstance` over plugin or field types at runtime.

---

## 12. Testing strategy

Replace the `double`-based `ModusTests` with exact-typed tests:

- **Exact kernel runs:** load `RealAddKernel`; dispatch `ArrayOp.Add` over `ReadOnlySpan<Real>`;
  assert the result equals `Real` addition byte-for-byte (no `double`).
- **Field injection:** `Integer`/`Natural` kernels receive the correct field and produce the
  exact `Integer`/`Natural` result.
- **Decline → fallback:** a kernel declining an op/dtype/length yields the reference result.
- **Precision inheritance:** `mean` over `Real` at high precision matches a hand-computed exact
  value; no rounding at 15 digits.
- **Builtin round-trip:** `EvaluateAsync("mean(1..5)")` returns `= 3 (Real)`.
- **Machine backend opt-in:** `DType.F64` kernel is selected only for `DenseArray<double>`,
  never for `DenseArray<Real>`.
- **Binary compat:** plugin compiled against `Abstractions` vN loads against vN+1.
- **AOT:** the app publishing target still builds with the plugin referenced.

---

## 13. Open decisions

1. `ScalarResult` exact shape and the scalar/multi-arg builtin API.
2. `IField<T>` relocation mechanics (move vs. re-export) and the `Lovelace.Array` split.
3. Machine-dtype naming/set (`F64`/`F32`) and the language-level opt-in syntax for it.
4. `ILinearAlgebraBackend<T>` vs descriptor+delegate registry (KRN-005).
5. Source-generated plugin discovery (`[LovelacePlugin]` + `PluginRegistry.LoadAll`) vs explicit
   `LoadPlugin` calls only.
6. Whether `IField<T>` should grow layer-3 operations (`Sin`/`Exp`/`Pow`) or stay field-only
   and defer those to direct `Real` use.

---

## 14. Rollout

1. **Move `IField<T>` into `Lovelace.Abstractions`** and add `RealField`/`IntegerField`/
   `NaturalField` to the numeric projects (§9.1–9.2). No behavior change.
2. **Replace `IArrayKernel<T> where T : unmanaged` with `IFieldKernel<T>`** and update
   `ModusHost`/`SuiteEngine` (§5.2, §9.4). Delete the `double` proof kernel.
3. **Complete typed storage** so arrays are `DenseArray<Real>` etc. (§9.3).
4. **Wire fallible dispatch** into the interpreter's elementwise path with field injection
   (§9.5) — the point where a plugin first accelerates real language arithmetic.
5. **Add the scalar builtin channel** so `mean` works end-to-end as the contract's proof
   (§5.4).
6. **Add the machine dtype** (`DType.F64`) as the explicit, opt-in fast path (§8).
