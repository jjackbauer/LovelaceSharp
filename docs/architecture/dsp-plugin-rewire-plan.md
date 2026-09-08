# DSP Plugin Rewire Plan — make DSP a true Modus plugin (suite knows nothing)

> **Status: implemented and verified.** Full solution builds; `Lovelace.Suite.Tests` 397/397 green;
> `fft`/`conv`/`re`/`im` smoke-tested under both JIT and Native AOT; grep gates confirm
> `Lovelace.Suite` holds zero DSP references and `Lovelace.Abstractions` remains dependency-free.
> **Trigger:** the first rewire attempt put `DspPlugin` *inside* `Lovelace.Suite` and defined a
> Suite-level registration channel (`ISuiteModusContext`). That is not pluggable: any extension
> needing multi-argument builtins would have to touch the core. This plan re-does it so the suite
> knows only the `IModusPlugin`/`IModusContext` contract and the extension defines its own plugin.

---

## 1. Problem

- `Lovelace.Abstractions/Modus.cs` offers only `RegisterArrayBuiltin` (1-arg `ArrayValue→ArrayValue`)
  and `RegisterKernel<T: unmanaged>` — too narrow for `conv(x,h)`, `filter(a,b,n)`,
  `cosine(freq,phase,n)`, `re(z)`, …
- The DSP bridge needs multi-arg builtins with scalar args and scalar/array results.
- `Lovelace.Abstractions` references nothing and must stay `Value`-free, so the channel cannot
  name `Value` — and it cannot name `Natural`/`Integer`/`Real`/`Complex` either (those projects
  sit above it in the layering).

## 2. Design decision — a Value-free, object-payload channel on `IModusContext`

```csharp
// Lovelace.Abstractions/Modus.cs — IModusContext gains:
void RegisterBuiltin(string name, IReadOnlyList<string> parameters,
                     Func<IReadOnlyList<object?>, object?> implementation);
```

Payload contract (documented in XML docs):

- **Core → plugin (args):** scalar args arrive as `Natural`/`Integer`/`Real`/`Complex`;
  vector/array args as a flat `IReadOnlyList<object?>` of those scalars.
- **Plugin → core (result):** one of those scalars, or an array of them; the core wraps to
  `Vector`/`Array` with inferred dtype (same behavior as today's `FromComplexArray`).
- No `Value`/`Interpreter`/`Ast` ever crosses the boundary. Runtime type switches are
  AOT-safe (no reflection). A named opaque wrapper (`ScalarResult`-style) could replace raw
  `object` later without changing the channel's shape.

## 3. Steps

1. **Contract** — `Lovelace.Abstractions/Modus.cs`: add `RegisterBuiltin` to `IModusContext`,
   payload contract documented.
2. **Bridge in the host** — `Lovelace.Suite/ModusHost.cs`: implement `RegisterBuiltin`
   (delegate to `Interpreter.RegisterBuiltin` after unwrap/wrap). ModusHost owns the
   `Value ↔ payload` mapping (golden rule). Delete `ISuiteModusContext`.
3. **Plugin moves to the extension** — new `Lovelace.Dsp/DspPlugin.cs`:
   `DspPlugin : IModusPlugin` (`Name = "Lovelace.Dsp"`) with all 16 builtins
   (`conv`, `dft`, `fft`, `filter`, `movingavg`, `impulse`, `step`, `cosine`,
   `exponential`, `powerseries`, `noise`, `delay`, `scale`, `re`, `im`, `conj`)
   ported onto the payload types; Real↔Complex coercion stays in the plugin.
   `Lovelace.Dsp.csproj` gains refs to `Abstractions`, `Natural`, `Integer`, `Real`.
4. **Suite becomes DSP-free** — delete `Lovelace.Suite/DspPlugin.cs`; drop the
   `Suite → Lovelace.Dsp` project reference (probe confirmed nothing else in Suite uses it).
   Suite knows only `IModusPlugin`.
5. **Hosts opt in** — `Lovelace.Run`, `Lovelace.Studio`, `Lovelace.Console` add a
   `Lovelace.Dsp` reference + `using Lovelace.Dsp;` (their `LoadPlugin(new DspPlugin())`
   calls already exist; `dspbench` already references Dsp). Compile-time-linked plugins are
   the repo's AOT model (`modus-plugin-design.md` §10).
6. **Tests** — `Lovelace.Suite.Tests` adds a `Lovelace.Dsp` reference;
   `DspPluginTests`/`ModusTests`/`LanguageDocumentationTests` add `using Lovelace.Dsp;`.
   All expectations unchanged (same error messages, same `DType.Complex` output).
7. **Docs** — update `modus-plugin-design.md`, `dspbench-coherence-plan.md`,
   `dsp-plugin-plan.md` to record the landed shape (and correct the earlier
   "Suite-level contract" notes).
8. **Verify** — full solution build; `Lovelace.Suite.Tests` green;
   `Lovelace.Run --eval "fft([1,0,0,0])"` under JIT **and** Native AOT; grep gates:
   zero `Lovelace.Dsp`/`DspPlugin` references inside `Lovelace.Suite`,
   `Abstractions` dependency set unchanged.

## 4. Open choice for the reviewer — resolved

The channel payload is `object`-typed — the only option that keeps `Lovelace.Abstractions`
dependency-free. **Resolved by the remediation pass** (`dsp-plugin-remediation-plan.md`):
`ScalarResult` landed as an additive typed wrapper (factories `From`/`FromBoolean`/
`FromText`/`FromArray`, opaque payload) with a default-implemented `RegisterBuiltin`
overload; `DspPlugin` migrated onto it, and the raw-`object` overload remains supported.
Novel *element* types still require a core bridge (documented in the contract docs).