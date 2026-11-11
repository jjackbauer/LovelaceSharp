# Round 51 — the dispatcher gap: registered builtins that answered "Unknown function"

**Status: fixed and verified.** Four builtins — `filter`, `scale`, `ndims`,
`cancel_full` — were present in the builtin registry (and therefore in
`--functions`, name suggestions and `--function <name>`) but had no arm in the
dispatcher's `match`. A call answered `UnknownFunction / UnsupportedOperation` with
the absurd suggestion of the same name. The four arms are now implemented against
the pinned C# oracle's own behaviour, and a structural test enumerates the registry
and asserts every entry dispatches. The 11 census rows that were blocked on this
class now run and match the oracle on the wire.

Pinned oracle: `dotnet exec Lovelace.Run/bin/Release/net10.0/Lovelace.Run.dll`
(`1cc28f255eca6029849f880949fc7be1d40ab8af`, as pinned by round 50).
Rust runner: `rust/target/release/lovelace-run.exe`, built from this change.
Measured 2026-11-11 (America/Sao_Paulo).

## 1. What the gap was

The registry (`rust/lovelace-lang/src/registry.rs`, `pub static BUILTINS`) is DATA:
123 `BuiltinInfo` descriptors that feed the envelope's functions array and the
`--functions` / `--function` discovery surface. Dispatch is a separate,
hand-maintained `match name` in `builtins::call` (`rust/lovelace-lang/src/builtins.rs`).
Nothing linked the two lists, and no test compared them: every registry/help test
passed while four published builtins could not execute.

This was **not** name resolution, an arity table or an alias map:

* the error's own suggestions contained the called name (`Did you mean 'filter'?`),
  which only happens when `registry::find`/suggest sees the entry;
* the registry entries carry correct `min_arity`, `parameter_count` and parameter
  names, and `arity_failure` reads them — an arity-only call reached the fallback
  before any arity check because the arm did not exist;
* `--functions --function-limit 1000` published 123 names in both runners, including
  all four.

A zero-argument call through the dispatcher for every registered name isolated the
population exactly:

```text
$ for n in $(--functions names); lovelace-run --eval "$n()"
before: registered=123 total=123 unknown=4 [cancel_full,filter,ndims,scale]
after : registered=123 total=123 unknown=0 []
```

## 2. Before — exact current error

All four exited 1 with the same shape (and the same message after the documented
prelude `x = symbol("x"); y = symbol("y"); t = symbol("t")`):

| Call | Exit | Code / category | Message |
|---|---:|---|---|
| `cancel_full(x/x)` | 1 | `UnknownFunction` / `UnsupportedOperation` | `Unknown function 'cancel_full'. Did you mean one of: 'cancel_full', 'cancel'?` |
| `filter([1],[1,2,3],[1,1,1,1])` | 1 | `UnknownFunction` / `UnsupportedOperation` | `Unknown function 'filter'. Did you mean 'filter'?` |
| `ndims([[1,2],[3,4]])` | 1 | `UnknownFunction` / `UnsupportedOperation` | `Unknown function 'ndims'. Did you mean 'ndims'?` |
| `scale([1,2,3],2)` | 1 | `UnknownFunction` / `UnsupportedOperation` | `Unknown function 'scale'. Did you mean one of: 'scale', 'shape'?` |

Envelope fields for each: `ok:false`, `recoverable:true`, `registrySize:123`, and
`suggestions` naming the called builtin. The 11 census scripts all failed on the
corresponding one of these four calls; the six descriptor examples failed with exit 1
and the `Unknown function` envelope, and the five DSP/cancel_full rows failed at
`$.exitCode` exactly as round 50 measured.

## 3. The fix

Production only, `rust/lovelace-lang/src/builtins.rs`; the registry itself was
already correct and is unchanged.

| Builtin | Dispatch arm and behaviour |
|---|---|
| `filter` | `dsp_filter`: `filter(a,b,x)` is `DspMath.Filter`'s direct-form difference equation with zero initial state — `y[n] = (Σ b[j]·x[n−j] − Σ a[j]·y[n−j]) / a[0]`, one output per input sample; `filter(a,b,n)` is the impulse response of length `n` (the third argument read through the shared exact-integer contract). Empty `a` reproduces the reference's `The denominator coefficient vector 'a' must not be empty. (Parameter 'a')`. Complex coefficients use the exact component arithmetic of `Lovelace.Complex`. |
| `scale` | `dsp_scale`: `Signal.Sample(new Scalar(c, Sequence(x)), 0, len−1)` — every sample multiplied by the complex scalar; an empty signal is the reference's `end must be ≥ begin. (Parameter 'end')`. |
| `ndims` | rank read of `shape_of`: vector → 1, N-D array → its rank; a scalar is the reference's `ndims(): argument 1 must be an array or vector; got <kind>.` (`InvalidArgument`/`TypeMismatch`). |
| `cancel_full` | `cancel_full_result`: the reference's `CancelResult` record (`status`, `original`, `expression`, `changed`, `conditions`). Each additive term is split into numerator/positive-power denominator, both read as multivariate polynomials over the expression's symbols (with the reference's one expand-and-retry when a power of a sum is not directly a polynomial), reduced by the Euclidean GCD, and the removed factor is reported as the `!= 0` condition. Exact reductions keep the original term and report an empty condition vector. |

Supporting code: sparse multivariate polynomial terms (`exponents, Rat`), Lex long
division and monic Euclidean GCD over the rationals, plus complex helper arithmetic.
The multivariate remainder sequence can cycle when two polynomials are relatively
prime (the C# oracle itself never returns for `cancel_full(y/x)`); the Rust GCD
detects a repeated state or a 10,000-reduction cap and answers the unit polynomial,
so the call returns the unchanged `Exact` result instead of hanging.

No `.cs`, fixture, `parity/` file or other document was edited. The only change
outside `rust/lovelace-lang/src` is a rustfmt-only normalization of
`rust/lovelace-run/src/round50_resweep.rs`, which was committed unformatted by
round 50 and was the only reason `cargo fmt --check` failed at baseline; no
statement or assertion in it changed.

## 4. The structural guard

New module `rust/lovelace-lang/src/dispatcher_guard_round51.rs` (registered under
`#[cfg(test)]` in `lib.rs`), four tests:

1. `every_registered_builtin_dispatches_instead_of_answering_unknown_function` —
   iterates `registry::BUILTINS` and sends a zero-argument call through the real
   `builtins::call`. Success and every ordinary arity/shape refusal are accepted;
   only `error.code == "UnknownFunction"` fails the test with the offender list.
   This is the structural test for the class: it fails on exactly the four names on
   the pre-fix tree (`[cancel_full, filter, ndims, scale]`) and passes only when
   registry and dispatcher agree.
2. `the_four_gap_builtins_run_their_published_examples` — the six descriptor
   examples, exact typed results.
3. `filter_signal_and_impulse_forms_match_the_reference_arithmetic` — FIR, IIR and
   impulse-response forms.
4. `cancel_full_publishes_the_conditions_the_bare_value_cannot_carry` — the
   Conditional/Exact record fields, including `x != 0` and `x - 1 != 0`.

## 5. After — the 11 census rows, both runners

Scripts use the documented prelude where the row is a published descriptor example.
Rust and oracle display/typed/result envelope are identical (the differential's
normalisation: `elapsed`, `elapsedTime`, `revision`, `timings` ignored; key
presence structural).

| # | Census row (short id) | Script | Rust | Oracle | Match |
|---:|---|---|---|---|---|
| 1 | `DescriptorExampleTests/Example_Executes#21` | `cancel_full(x/x)` | 0 — `CancelResult(status: Conditional, original: x/x, expression: 1, changed: True, conditions: [x != 0])` | same | SAME |
| 2 | `DescriptorExampleTests/Example_Executes#22` | `cancel_full((x^2 - 1)/(x - 1))` | 0 — `CancelResult(status: Conditional, original: (x^2 - 1)/(x - 1), expression: x + 1, changed: True, conditions: [x - 1 != 0])` | same | SAME |
| 3 | `DescriptorExampleTests/Example_Executes#53` | `filter([1], [1], [1, 2, 3])` | 0 — `[1, 2, 3] (Vector)` | same | SAME |
| 4 | `DescriptorExampleTests/Example_Executes#86` | `ndims([[1, 2], [3, 4]])` | 0 — `2 (Natural)` | same | SAME |
| 5 | `DescriptorExampleTests/Example_Executes#107` | `scale([1, 2, 3], 2)` | 0 — `[2, 4, 6] (Vector)` | same | SAME |
| 6 | `DescriptorExampleTests/Example_Executes#108` | `scale([1, 2, 3], 0.5)` | 0 — `[0.5, 1, 1.5] (Vector)` | same | SAME |
| 7 | `DspTests/Filter_GivenFir_MatchesConvolution#0` | `filter([1],[1,2,3],[1,1,1,1])` | 0 — `[1, 3, 6, 6]` | 0 — `[1, 3, 6, 6]` | SAME |
| 8 | `DspTests/Filter_GivenIir_MatchesImpulseResponseConvolution#0` | `filter([1,-0.5],[1],[1,1,1,1])` | 0 — `[1, 1.5, 1.75, 1.875]` | 0 — `[1, 1.5, 1.75, 1.875]` | SAME |
| 9 | `DspPluginTests/Filter_GivenLength_ReturnsImpulseResponse#0` | `filter([1,-0.5],[1],4)` | 0 — `[1, 0.5, 0.25, 0.125]` | 0 — `[1, 0.5, 0.25, 0.125]` | SAME |
| 10 | `DspPluginTests/Filter_GivenSignal_MatchesConvolution#0` | `filter([1,-0.5],[1],[1,1,1,1])` | 0 — `[1, 1.5, 1.75, 1.875]` | 0 — `[1, 1.5, 1.75, 1.875]` | SAME |
| 11 | `MetamorphicSetClosureTests/CancelFull_ReportsTheConditionsTheBareValueCannotCarry#0` | `[cancel_full(x/x).status, .original, .expression, .changed, .conditions[0], cancel_full((x^2+1)/(x-1)).status/.expression, cancel_full((x-1)^2/(x-1)).expression/.conditions[0]]` | 0 — `[Conditional, x/x, 1, True, x != 0, Exact, (x^2 + 1)/(x - 1), x - 1, x - 1 != 0]` | same | SAME |

The C# assertions of the four DSP rows are prefix equalities against
`DspMath.Convolve`. The runner-observable translations agree byte-for-byte with the
oracle:

* `conv([1,1,1,1],[1,2,3]) = [1, 3, 6, 6, 5, 3]`; row 7's filter is its first four samples.
* `conv([1,1,1,1],[1,0.5,0.25,0.125]) = [1, 1.5, 1.75, 1.875, 0.875, 0.375, 0.125]`; row 8's filter is its first four samples.
* `filter([1,-0.5],[1],[1,0,0,0]) = [1, 0.5, 0.25, 0.125] = filter([1,-0.5],[1],4)`, which is row 9's impulse response.

Refusal parity was also checked while implementing (both runners identical exit,
code, category and message): `filter([],[1],[1])` (empty `a`), `filter([1],[1],-1)`,
`filter([1],[1],2.5)`, `filter(1,[1],[1])`, `scale([],2)`, `scale(1,2)`,
`scale([1,2,3],[1,2,3])`, `ndims(5)`.

## 6. The 11 ids (expected to become runnable/mappable)

```text
Lovelace.Suite.Tests/Lovelace.Suite.Tests.DescriptorExampleTests/Example_Executes#21
Lovelace.Suite.Tests/Lovelace.Suite.Tests.DescriptorExampleTests/Example_Executes#22
Lovelace.Suite.Tests/Lovelace.Suite.Tests.DescriptorExampleTests/Example_Executes#53
Lovelace.Suite.Tests/Lovelace.Suite.Tests.DescriptorExampleTests/Example_Executes#86
Lovelace.Suite.Tests/Lovelace.Suite.Tests.DescriptorExampleTests/Example_Executes#107
Lovelace.Suite.Tests/Lovelace.Suite.Tests.DescriptorExampleTests/Example_Executes#108
Lovelace.Dsp.Tests/Lovelace.Dsp.Tests.DspTests/Filter_GivenFir_MatchesConvolution#0
Lovelace.Dsp.Tests/Lovelace.Dsp.Tests.DspTests/Filter_GivenIir_MatchesImpulseResponseConvolution#0
Lovelace.Suite.Tests/Lovelace.Suite.Tests.DspPluginTests/Filter_GivenLength_ReturnsImpulseResponse#0
Lovelace.Suite.Tests/Lovelace.Suite.Tests.DspPluginTests/Filter_GivenSignal_MatchesConvolution#0
Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.MetamorphicSetClosureTests/CancelFull_ReportsTheConditionsTheBareValueCannotCarry#0
```

Candidate Rust test ids for the manifest mapping (all resolve in the new module):
`dispatcher_guard_round51::the_four_gap_builtins_run_their_published_examples`,
`dispatcher_guard_round51::filter_signal_and_impulse_forms_match_the_reference_arithmetic`,
`dispatcher_guard_round51::cancel_full_publishes_the_conditions_the_bare_value_cannot_carry`.
Per scope, `parity/manifest-map.json` was not edited; the rows remain
`disposition = "deferred"` until the parent's mapping lane flips them.

## 7. Gates (observed this round)

| Step | Command | Observed | Exit |
|---|---|---|---:|
| Workspace suite | `cargo test --workspace` | `TOTAL passed=4971 failed=0` (4,967 before + 4 new guard tests) | 0 |
| Corpus | `cargo run --release -p lovelace-corpus` | `corpus: 214 passed, 0 failed, 214 total` | 0 |
| Protocol fixtures | `node parity/tools/gate-fixtures.mjs` | `oracle: 20/20`, `rust: 20/20`, `PASS` | 0 |
| CLI matrix | `node parity/tools/gate-cli.mjs` | `CLI matrix: 16/16`, `PASS` | 0 |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` | clean | 0 |
| Format | `cargo fmt --check` | clean | 0 |
| Registry sweep | every registered name called once | `registered=123 unknown=0 []` | 0 |

## 8. Residual

* `parity/manifest-map.json` still lists the 11 rows as deferred and round 50's
  measured reasons still describe the pre-fix tree; mapping them is the parent's
  parity step, not touched here.
* The bare `cancel` builtin keeps its existing measured divergence
  (`cancel((x^2+1)/(x-1))` returns `x + 1` instead of the unchanged quotient,
  because `lovelace_symbolics::poly_div` discards its remainder). It dispatches and
  was outside this gap; its two census rows remain deferred under the `cancel`
  family.
* `cancel_full(y/x)` and `cancel_full(x/y)` make the pinned C# oracle loop without
  returning; the Rust implementation answers `Exact`, unchanged, with no conditions.
  Neither form is in the 11 rows, and returning is preferable to reproducing an
  oracle hang.
* The registry/help surface itself did not move: 123 entries before and after.
