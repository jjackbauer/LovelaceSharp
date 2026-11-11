# Round 25 — the arbitrary-precision transcendental kernel

Status: **implemented and verified against the frozen oracle; gates re-run**. Scope: `rust/lovelace-real/src`
(the kernel), `rust/lovelace-lang/src` (the numeric `evalf` evaluator and the complex surface) and
tests. No `.cs` source, fixture, corpus case, `parity/` source or configuration was edited, and no
documentation outside this report.

## 1. What was implemented

| File | Change |
|---|---|
| `rust/lovelace-real/src/transcendental.rs` (new) | `Real::exp`, `ln`, `sin`, `cos`, `tan`, `atan`, `atan2`, `asin`, `acos`, `sinh`, `cosh`, `tanh`, `asinh`, `acosh`, `atanh`, `pow_real` and the `e` constant. The algorithms are the reference kernel's (`Lovelace.Complex/ComplexMath.cs`): exp splits `x = n + f` and evaluates `e^f` by Taylor series times `e^n` by binary exponentiation; ln decomposes `x = m·10^L` with `m ∈ [1,10)`, takes 16 square roots and a reduced atanh series, and rebuilds `ln x = ln m + L·ln 10` with `ln 10 = 3·ln 2 + ln 1.25`; sin/cos reduce modulo π/2 against a π resolved to the argument's own places, fold to [0, π/4] and sum the Taylor series; atan uses three half-angle reductions and an alternating series; the hyperbolic and inverse functions are the reference identities. |
| `rust/lovelace-real/src/lib.rs` | registers the module. |
| `rust/lovelace-lang/src/numeric.rs` (new) | the reference `EvaluateToNum` tiers over the canonical symbolic tree: exact rationals stay rational, `Real` is promoted only when a transcendental forces it, complex values use the principal branch, and the exact special values fold exactly (`exp 0 = 1`, `sin 0 = 0`, `log 1 = 0`, …). |
| `rust/lovelace-lang/src/complex_math.rs` (new) | complex `Exp`, `Sin`, `Cos`, `Tan`, `Sinh`, `Cosh`, `Sqrt`, `Log`, `Pow` over the round-24 `Complex` and `Real` kernel, with the reference component truncation and provenance rules. |
| `rust/lovelace-lang/src/builtins.rs` | `evalf` evaluates a symbolic argument: working precision = requested digits + 20, re-evaluated once at digits + integer digits + 20 when the result carries at least 20 integer digits, then bounded back to the request. |
| `rust/lovelace-real/src/transcendental_round25.rs`, `rust/lovelace-lang/src/transcendental_round25.rs` (new, 16 tests) | the probes, exactness, domains, identities, and the complex values pinned against the C# assembly. |

### Precision policy

The reference policy is followed exactly:

* each entry point computes at the requested fractional budget **plus 20 guard places**;
* series divisions are the reference's single fixed-point truncating division (no period
  detection, remainder tracked into the provenance); composed identities use the reference's
  ambient-precision division;
* every returned value is truncated toward zero to the requested budget;
* a value that dropped a digit is marked **inexact**; a special value that is exactly
  representable and was reached from an exact argument stays exact (`sin(0)=0`, `cos(0)=1`,
  `exp(0)=1`), and an inexact argument marks even those values inexact.

The constants follow the reference too: π is the existing Machin implementation (verified equal to
the C# `PiTo` at 20/50/100/140 places), e is the factorial rational divided once at
digits+10 (equal to the C# `ETo`), and √ is the existing integer-square-root implementation
(verified equal to the C# `Sqrt` at 20/50/100/140 places).

## 2. The twelve probes, digit for digit

Method: both runners were invoked as
`--eval 'evalf(EXPRESSION, N)' --json --omit-functions`; the `structured.kind`,
`structured.value` and `structured.exact` fields were compared. **All 36 rows are identical**
in display, structured value, kind and exact flag. The machine-readable record is
`rust/target/tmp/round25-probes.json` (36/36, mismatches 0).

The oracle spells the natural logarithm `log`; `ln(2)` is the oracle's *UnknownFunction*
refusal. `ln` is the Rust `Real` method name, and the probes use the oracle's spelling
`log(2)` / `log(10)`.

### 20 decimal places

| Expression | Oracle = Rust |
|---|---|
| `exp(1)` | `2.71828182845904523536` |
| `log(2)` | `0.69314718055994530941` |
| `sin(1)` | `0.84147098480789650665` |
| `cos(1)` | `0.5403023058681397174` |
| `tan(1)` | `1.5574077246549022305` |
| `atan(1)` | `0.78539816339744830961` |
| `sinh(1)` | `1.17520119364380145688` |
| `cosh(1)` | `1.54308063481524377847` |
| `tanh(1)` | `0.76159415595576488811` |
| `exp(-2.5)` | `0.08208499862389879516` |
| `log(10)` | `2.30258509299404568401` |
| `sin(100)` | `-0.50636564110975879365` |

### 50 decimal places

| Expression | Oracle = Rust |
|---|---|
| `exp(1)` | `2.71828182845904523536028747135266249775724709369995` |
| `log(2)` | `0.69314718055994530941723212145817656807550013436025` |
| `sin(1)` | `0.84147098480789650665250232163029899962256306079837` |
| `cos(1)` | `0.54030230586813971740093660744297660373231042061792` |
| `tan(1)` | `1.55740772465490223050697480745836017308725077238152` |
| `atan(1)` | `0.78539816339744830961566084581987572104929234984377` |
| `sinh(1)` | `1.17520119364380145688238185059560081515571798133409` |
| `cosh(1)` | `1.54308063481524377847790562075706168260152911236586` |
| `tanh(1)` | `0.76159415595576488811945828260479359041276859725793` |
| `exp(-2.5)` | `0.08208499862389879516952867446715980783780412101543` |
| `log(10)` | `2.30258509299404568401799145468436420760110148862877` |
| `sin(100)` | `-0.50636564110975879365655761045978543206503272129065` |

### 100 decimal places

| Expression | Oracle = Rust |
|---|---|
| `exp(1)` | `2.7182818284590452353602874713526624977572470936999595749669676277240766303535475945713821785251664274` |
| `log(2)` | `0.6931471805599453094172321214581765680755001343602552541206800094933936219696947156058633269964186875` |
| `sin(1)` | `0.8414709848078965066525023216302989996225630607983710656727517099919104043912396689486397435430526958` |
| `cos(1)` | `0.5403023058681397174009366074429766037323104206179222276700972553811003947744717645179518560871830893` |
| `tan(1)` | `1.5574077246549022305069748074583601730872507723815200383839466056988613971517272895550999652022429838` |
| `atan(1)` | `0.7853981633974483096156608458198757210492923498437764552437361480769541015715522496570087063355292669` |
| `sinh(1)` | `1.1752011936438014568823818505956008151557179813340958702295654130133075673043238956071174520896233918` |
| `cosh(1)` | `1.5430806348152437784779056207570616826015291123658637047374022147107690630492236989642647264355430355` |
| `tanh(1)` | `0.7615941559557648881194582826047935904127685972579365515968105001219532445766384834589475216736767144` |
| `exp(-2.5)` | `0.0820849986238987951695286744671598078378041210154366488457584105152247568804109713097515715212364659` |
| `log(10)` | `2.3025850929940456840179914546843642076011014886287729760333279009675726096773524802359972050895982983` |
| `sin(100)` | `-0.5063656411097587936565576104597854320650327212906573234433924735943579134194766964992366645129273922` |

### Further probes outside the required twelve

All of these also matched the oracle exactly (30/30): `asinh(1)`, `acosh(2)`, `atanh(0.5)`,
`asin(0.5)`, `acos(0.5)`, `atan(2)`, `atan(-0.5)`, `tanh(-2)`, `exp(3)`, `log(1000)` at
20, 50 and 100 places. The documented large-magnitude second pass matched too:
`evalf(sinh(34/3),30)` = `41780.548053564065925188446077567339`,
`evalf(sinh(65/3),30)` = `1284351149.965741094255005311674965079318`,
`evalf(sinh(1000/3),30)` = the same long integer-plus-30-decimals string. The domain
refusals match in code, category and message (`InvalidArgument/TypeMismatch` for asin/acos/acosh/atanh,
`EvaluationError/DomainError` for log(0), including the `(Parameter 'x')` suffix).

## 3. Exactness evidence

* All 36 probe rows publish `{"kind":"Real","exact":false}` from the Rust runner, identical to the
  oracle. The Lang test `evalf_transcendental_values_are_inexact_on_the_wire` asserts this for
  every probe at every precision.
* The Real-crate test `every_probe_is_an_inexact_value` asserts `!is_exact()` on the kernel results.
* The exact special points remain exact: `evalf(exp(0),20)` = Integer 1, `evalf(sin(0),20)` =
  Integer 0, `evalf(cos(0),20)` = Integer 1, `evalf(tan(0),20)` = Integer 0,
  `evalf(log(1),20)` = Integer 0, `evalf(atan(0),20)` = Integer 0 (kind, display and
  `exact:true` all identical to the oracle). These are the reference's symbolic constant folding,
  mirrored in the numeric evaluator.
* Provenance propagation is pinned: `exp(exact 0) = 1` exact; `exp(inexact 0) = 1` inexact;
  `sin/cos/tan` of an exact zero exact, of an inexact zero inexact; `atan(inexact 0) = 0` inexact;
  `atan2(inexact 0, 1) = 0` inexact.

## 4. ComplexMath

Verified against the frozen C# `Lovelace.Complex` assembly (loaded through `dotnet fsi` and
called at ambient 60 places), not against an identity where a literal was available. The Lang test
`complex_math_matches_the_reference_at_sixty_places` and `complex_sqrt_and_pow_match_the_reference`
pin every component with numeric equality against the oracle strings; the values below are the
oracle's, and every component matched.

| Call at 60 places | Real part (oracle = Rust) | Imaginary part (oracle = Rust) | exact |
|---|---|---|---|
| `exp(1+2i)` | `-1.131204383756813638431255255510794710628867995826525750217719` | `2.471726672004818927616930893551664532736190369241008184200758` | false/false |
| `log(1+2i)` | `0.804718956217050187300379666613093819762800677134258860956323` | `1.107148717794090503017065460178537040070047645401432646676539` | false/false |
| `sqrt(1+2i)` | `1.27201964951406896425242246173749149171560804184009624861664` | `0.786151377757423286069558585842958929523122057837723237664901` | false/false |
| `sqrt(-4+0i)` | `0` | `2` | true/true |
| `sin(1+2i)` | `3.165778513216168146740734617191905538379110767891468932289324` | `1.9596010414216058970703520499893582784363201601845596588019` | false/false |
| `cos(1+2i)` | `2.032723007019665529436343448499514263731990406638752381945223` | `-3.051897799151800057512115686895105452888437617733319644663374` | false/false |
| `tan(1+2i)` | `0.033812826079896690284370559725287301640187669330755874366730` | `1.014793616146633568117054175417967614163217471123533242464957` | false/false |
| `sinh(1+2i)` | `-0.48905625904129367358645456854851592115851088467859496570288` | `1.403119250622040588019490859767712944070947553407072439773952` | false/false |
| `cosh(1+2i)` | `-0.642148124715519964844800686962278789470357111147930784514838` | `1.068607421382778339597440033783951588665242815833935744426806` | false/false |
| `pow(1+2i, 3-4i)` | `932.139194643221377037990521386002319202631311217932612164482002` | `95.946533660342017314086982354678947782067994589733694956741179` | false/false |
| `pow(2, 0.5)` | `1.414213562373095048801688724209698078569671875376948073176679` | `0` | false/false |

One rendering (not value) difference: the oracle's `tan(1+2i)` real part is stored with a trailing
zero at the 60th place (`…366730`) and the Rust value normalises it to `…36673`. The numeric
values are equal (the test asserts numeric component equality); no digit differs.

## 5. Gates (after)

| Gate | Observed | Evidence |
|---|---|---|
| `cargo test --workspace` | **4,112 passed, 0 failed** across 19 test binaries (the previous 4,096 plus 8 Real + 8 Lang tests) | `rust/target/tmp/round25-cargo-after.log` |
| `cargo run --release -p lovelace-corpus` | **212 passed, 2 failed, 214 total** — only the pre-existing `sym/071`, `sym/076` | `rust/target/tmp/round25-corpus-after.log` |
| `node parity/tools/gate-fixtures.mjs` | oracle **20/20**, rust **20/20**, PASS | `rust/target/tmp/round25-fixtures-after.log` |
| `node parity/tools/gate-cli.mjs` | CLI matrix **16/16**, PASS | `rust/target/tmp/round25-cli-after.log` |
| `node parity/diff/run.mjs --cases 200 --seed 1 --no-minimize` | generated **155 match / 45 mismatch / 0 error**; replayed **654 match / 1,098 mismatch / 0 error**; **new=0**. Baseline: 154/46 and 600/1,152. Delta: **+1 generated match, +54 replayed matches, 0 new mismatches, 0 errors** | `rust/target/tmp/round25-diff-after.log` |

The full workspace suite includes the two new test modules:
`rust/lovelace-real/src/transcendental_round25.rs` (8 tests) and
`rust/lovelace-lang/src/transcendental_round25.rs` (8 tests). No existing test was deleted,
weakened or ignored.

## 6. What remains

1. **Manifest mappings.** The 166 `Lovelace.Real.Tests` and 69 `Lovelace.Complex.Tests.ComplexMath`
   manifest rows now have a Rust surface to map to (`Real::{exp,…,atanh}` and
   `lang::complex_math`), but the manifest dispositions themselves were not rewritten: `parity/` is
   outside this round's scope and the parent/porting wave owns the mapping.
2. **Test-row porting.** This round pins a representative set (16 tests, 36 Real probes, 11 complex
   values, domains and identities). The C# assertion-by-assertion port of the remaining rows is a
   mechanical follow-up now that the kernel exists.
3. **`ln` spelling.** The oracle has no `ln` builtin; it spells the natural logarithm `log`.
   The Rust library method is `Real::ln`; the language surface intentionally keeps the oracle's
   `log` vocabulary rather than adding an oracle-incompatible alias.
4. **Complex `tan` real-component rendering** keeps the oracle's trailing-zero representation only
   numerically; the digits are equal (section 4).
5. **Numeric evaluators outside the elementary family** (`min`, `max`, `floor`, `ceil`, `sign`)
   are not part of `evalf`'s symbolic evaluator yet; `evalf` of those functions still reports the
   reference's `Function 'x' has no numeric evaluator.` refusal. The elementary family named in the
   objective is complete.
6. **Pre-collapsed numeric arguments.** `evalf(cos(pi), 20)` still differs because the Rust
   language keeps `cos(pi)` symbolic while the reference constant-folds it at the ambient precision
   before `evalf`; this is the pre-existing symbolic constant-folding path, not the transcendental
   kernel (all twelve declared probes and the 30 further probes use the same kernel successfully).
7. **1000-place cap notices** for `evalf` above 1000 digits are still not projected (pre-existing,
   unchanged).

## 7. Reproduction

```powershell
$repo='C:\Users\ricar\dev\LovelaceSharp'
$env:RUSTUP_HOME="$repo\.toolchain\rustup"; $env:CARGO_HOME="$repo\.toolchain\cargo"
$env:PATH="$repo\.toolchain\cargo\bin;" + $env:PATH
$env:TMP="$repo\rust\target\tmp"; $env:TEMP=$env:TMP
Set-Location "$repo\rust"
cargo test --workspace
cargo run --release -p lovelace-corpus
Set-Location $repo
node parity/tools/gate-fixtures.mjs
node parity/tools/gate-cli.mjs
node parity/diff/run.mjs --cases 200 --seed 1 --no-minimize
```

Probe record: `rust/target/tmp/round25-probes.json`. The oracle values in this report were also
re-derived directly from `Lovelace.Complex.ComplexMath` through `dotnet fsi` against the frozen
Release DLLs.
