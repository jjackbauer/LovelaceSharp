# Round 37 — numeric regressions: the Real division boundary and the DSP trig publication

Status: fixed in production, tests untouched. The two failing cross-crate numeric tests named by the
brief now pass, the workspace is green, and the corpus is at its 214/214 baseline.

| File | Change |
|---|---|
| `rust/lovelace-real/src/lib.rs` | `Real::period_start()` / `Real::period_length()` accessors; `Real::truncated_frac(digits)` (the reference's display-cap truncation for a terminating value) |
| `rust/lovelace-lang/src/builtins.rs` | `div` publishes a boundary-periodic quotient as inexact; `dsp_real_cos` / `dsp_real_sin` publish their Taylor result at the engine display width |

No test, fixture, corpus file, `parity/` artifact or C# source was changed.

## 1. `robustness_round13::round13_terminating_quotient_longer_than_the_budget_stays_exact`

Probe (the test drives `Engine::set_precision(1000)` before evaluating the script):

```text
setprecision(1000); 1/(3*10^1000)
```

| | value (short form) | exact |
|---|---|---|
| before | `0.<1000 zeros>(3)`, period_start 1000, period_length 1 | **true** (assertion failed) |
| after | the same value and period metadata | **false** |
| control, unchanged | `1/(2^4000)` = numerator 1 over 2^4000 (a 4000-digit terminating expansion) | true |

The value is kept: the leading zero run is scale, so the 1000 zeros stay in the exponent and the
repeating block stays stored. Only the provenance says the non-terminating expansion did not fit the
budget.

## 2. `dsp_round21::cos_given_pi_over_6_is_sqrt3_over_2`

Probe: `setprecision(100); re(cosine(1/12, 0, 2)[1])`, compared with
`setprecision(100); sqrt(3)/2` (the test's `real_cos_at_100("1/12")`).

| | typed value (short form) |
|---|---|
| before | `0.866025403784438646763723170752936183471402626905190314027903489725966508454400018540573093378624287844658744…` — a **9,519-character** display (9,526 with ` (Real)`); the first 100 digits are correct, the rest are the Taylor value's stored tail |
| after | `0.8660254037844386467637231707529361834714026269051903140279034897259665084544000185405730933786242878 (Real)` — exactly the 100-digit `sqrt(3)/2` publication |
| expected | the same 100-digit `sqrt(3)/2` string (the assertion's right side) |

## 3. The causing change

No `precision.max(64)` floor is present in this tree: `builtins::div` divides at
`precision.max(1)`, so the round-34 floor removal is intact. The two failures have different
proximate causes.

**Division — round 34's digit-loop rewrite (`ee8f7de`, "inexact-division semantics").** That
commit replaced the round-13 budget-boundary period peek with the reference's top-of-loop budget
stop and added the freed leading-zero accounting (`freed`/`skipping_zeros`). The freed walk can
reach past a leading run that alone fills the budget, so `1/(3*10^1000)` — whose repeating block
begins exactly at the 1000-place cap — now closes its period and came back exact. The round-13
control pins that non-terminating quotient as the one that must say inexact. Reproduction against
the committed tree confirms it: the test also fails at `ee8f7de` (identical assertion at line 248),
so the change is at or before that commit, not in round 35 or the uncommitted round-36 work.

The pinned oracle (`Lovelace.Run`, sha256 `f292249…`) and the C# `RealDivideLeadingZeroRunTests`
row both return this class-level quotient as an **exact** periodic value, because the leading zero
run is scale and the period is representable. The round-13 language control is stricter: it pins the
language surface's truncation rule. The fix therefore sits at the language boundary
(`builtins::div`), and the `Real` crate keeps the oracle's exact representation
(`r15_divide_leading_zero_run_keeps_the_value_and_its_scale` still passes).

**DSP cosine — the angle construction vs. the table candidate, plus the missing display cap.** The
DSP builtin builds the angle the way the reference DSP signal does:
`pi*2*frequency*n`. `1/12` is periodic, so `Real::mul` takes its exact-fraction path. The stored
`pi(100)` magnitude happens to be divisible by 3, so `pi*2*(1/12)` is the exact terminating value
at **101** fractional digits, while `dsp_special_angle_candidate` builds the table's pi/6 as the
**100**-place `div` truncation (the reference's `SpecialAngleCandidate` integer division at pi's
own scale). The 101st digit differs, the table lookup misses, and the general-angle Taylor result is
returned with its full stored width (9,517 fractional digits). The reference's `Real.ToString`
publishes a non-periodic value at `Real.DisplayDecimalPlaces` (100 here); the Rust language's typed
rendering has no separate display step, so the port published every stored digit.

This is a pre-existing gap, not a round-35/round-36 regression: the identical failure reproduces at
`ee8f7de` (9,519-character display), and round 35 did not touch `div`, the DSP builtins or the
digit loop. The round-34 document's claim that the DSP row was fixed by the floor removal is not
reproducible on the committed tree.

## 4. The fixes

1. `Real::truncated_frac(digits)` truncates a terminating value to `digits` fractional places
   through one integer division and a `Normalize`; a periodic value is returned unchanged, exactly
   as the reference renders a repeating block in full and applies its display limit only to a finite
   expansion. `period_start()` / `period_length()` expose the metadata the language rule needs.
2. `builtins::div` marks a quotient inexact when a period was discovered whose block ends past the
   computation cap (`period_start + period_length > places`), mirroring the boundary rule
   `fraction_real_exact` already applies. The value, period and exponent are untouched.
3. `dsp_real_cos` and `dsp_real_sin` publish their series result with
   `truncated_frac(engine.display_precision())`; special-angle table hits are returned untouched.
   This also keeps the 100-digit prefix assertion of
   `dsp_round20::cosine_given_general_frequency_matches_the_oracle_digits` (default scope: 30
   computation digits, 100 published digits).

## 5. Gates (measured after the final source state)

```text
$ cargo test --workspace --no-fail-fast
array 58, desktop 67, int 125, json 7, lang 1321, nat 226, real 2438,
run 350, run integration 1, symbolics 100  =  4,693 tests, 0 failed, exit 0
```

```text
$ cargo run --release -p lovelace-corpus
corpus: 214 passed, 0 failed, 214 total
```

The full workspace run above was measured on the final code; the only later edit was a comment in
`builtins::div`, and the final source was re-measured with `cargo test -p lovelace-lang`
(1,321 passed, 0 failed, exit 0).

The two target tests in isolation:

```text
robustness_round13::round13_terminating_quotient_longer_than_the_budget_stays_exact ... ok
dsp_round21::cos_given_pi_over_6_is_sqrt3_over_2 ... ok
```

## 6. What remains

- The round-13 language control and the pinned oracle/C# class row disagree on `1/(3*10^1000)`'s
  `IsExact`. This round keeps both contracts: the class/Real surface stays exact, the language
  surface publishes the boundary quotient as inexact. If the language contract is ever reconciled
  with the runner, the `builtins::div` rule is the single line to revisit.
- The DSP proxy still relies on the reference's displayed width rather than on a table hit. A
  faithful `SpecialAngleCandidate` comparison cannot match the `pi*2*(1/12)` angle (the C# class
  probe confirms `pi*2*(1/12) != pi/6` and its `Cos` also falls through to Taylor), so the
  publication width is the correct observable to reproduce.
