# Round 14 — the language/engine surface behind the refused Suite pool

Scope: the parser and engine surface the refused `Lovelace.Suite.Tests` rows exercise. No `.cs`
file, fixture, corpus example, or `parity/` artifact was changed. Production edits are in
`rust/lovelace-lang/src/{syntax,lib,builtins}.rs`; the new tests are
`rust/lovelace-lang/src/suite_surface_round14.rs` (17 tests). The named larger subsystems
(LReal/Motus/CaptureState/Help/events/numeric-model) were NOT attempted.

Oracle: the pinned `Lovelace.Run` at `1cc28f255eca6029849f880949fc7be1d40ab8af`
(`Lovelace.Run/bin/Release/net10.0/Lovelace.Run.dll`). Every expectation below was executed on
that oracle first and on the Rust runner after the change, except where the "before" value is
explicitly attributed to the round-8 evidence (the pre-change binary was overwritten by the build
needed for the after-run).

## 1. Capabilities added, with the probe

```
before = rust/target/debug/lovelace-run.exe --text --eval <script>   (pre-change binary)
after  = rust/target/debug/lovelace-run.exe --text --eval <script>
oracle = dotnet Lovelace.Run/bin/Release/net10.0/Lovelace.Run.dll --text --eval <script>
```

Rows marked † were not re-run on the pre-change binary (it was overwritten by the build needed for
the after-run): their "before" value is the one recorded by the round-8 report or read from the
pre-change source, and is labelled as such. Every other "before" value was executed in this session.

| # | Capability | Script | Oracle | Before (Rust) | After (Rust) |
|---|---|---|---|---|---|
| 1 | Unary plus (prefix, precedence) | `+2^2` | `4 (Natural)` | ParseError: `Unexpected token '+' at position 0…` | `4 (Natural)` |
| 1 | Unary plus on Real / repeated / signed | `+2.71`; `++1`; `+ -1` | `2.71 (Real)`; `1 (Natural)`; `-1 (Integer)` | all ParseError | all match |
| 2 | Slice step syntax | `[0,1,2,3,4][::2]` | `[0, 2, 4] (Vector)` | ParseError: `Unexpected token ':' at position 13…` | `[0, 2, 4] (Vector)` |
| 2 | Slice start:stop, column, matrix rows, elementwise | `[1:4]`; `[:,1]`; `[1:2]`; `[::2] * 2` | `[1, 2, 3]`; `[2, 5]`; `[[4, 5, 6]]`; `[2, 6, 10]` | start:stop already worked; `[::2]` variants ParseError | all match |
| 2 | Slice `::` / `1::2` / `:3:2` | `[0,1,2,3,4][::]`; `[1::2]`; `[:3:2]` | whole; `[1, 3]`; `[0, 2]` | ParseError | all match |
| 2 | Slice step guard | `[0,1,2][::-1]` | InvalidArgument/TypeMismatch `range(): argument 'step' … got -1.` | reversed (undocumented superset) / parse error | same refusal text as oracle |
| 3 | Empty sum/prod (D6) | `sum([])`; `prod([])` | `0 (Natural)`; `1 (Natural)` | `Reduction needs at least one element.` | identities, Natural |
| 3 | Empty min/max/mean/norm | `min([])` `max([])` `mean([])` `norm([])` | `min()/max()/mean()/norm() cannot reduce an empty array.` | `Reduction needs at least one element.` | same four sentences |
| 4 | `evalf` exactness flag † | `evalf(1/3, 50)` | `exact:false`, no numerator/denominator | exact:true with numerator/denominator (round-8 evidence; the old body re-parsed a finite decimal) | `exact:false`, no rational payload |
| 4 | `evalf` budget rule † | `evalf(1/3,29)`; `evalf(1/3,30)`; `setprecision(1000); evalf(1/3,50)` | exact true; false; true | always true | true; false; true |
| 4 | `evalf` integer payload † | `evalf(2,5)`; `evalf(4.0,5)` | `2 (Integer)`; `4 (Integer)` | `2 (Real)`; `4 (Real)` | `Integer` both |
| 4 | `evalf` inexact argument is not padded † | `evalf(pi(30),40)` | `3.141592653589793238462643383279`, exact false | evalf(pi(30),40) was exact true before the flag change | 30 stored digits, exact false |
| 5 | asinh/acosh/atanh registered | `asinh(1)`; `acosh(2)`; `atanh(0.5)` | `asinh(1)`; `acosh(2)`; `atanh(1/2)` (Symbolic) | Unknown function 'asinh'/'acosh'/'atanh' | same symbolic forms |
| 5 | Defining-point folds | `asinh(0)`; `acosh(1)`; `atanh(0)` | `0 (Symbolic)` each | Unknown function | `0 (Symbolic)` each |
| 6 | `pi` constant | `pi` | 100-digit `3.14159…` (Real) | `Undefined variable 'pi'.` | same 100-digit display as oracle |
| 6 | `e` constant and `e()` | `e`; `e()`; `e(10)` | 100-digit `2.71828…`; same; `2.7182818284` | `Undefined variable 'e'.` / Unknown function 'e' | same |
| 6 | `pi()` default uses display precision | `pi()`; `setprecision(50); pi` | `3.14159…` (100 digits); 50 digits | `pi()` rendered the 1000-digit computation value | 100 digits; 50 digits |
| 6 | constants are inexact | structured `pi`; `e` | exact:false, no rational payload | no value | exact:false, no rational payload |
| 7 | Integer factorial † | `(-(-5))!`; `(-0)!` | `120 (Integer)`; `1 (Integer)` | `Factorial needs a Natural, but got 'Integer'.` | matches |
| 7 | Negative Integer factorial | `(-5)!` | `Factorial is not defined for negative integers.` | `Factorial needs a Natural, but got 'Integer'.` | same sentence as oracle |
| 7 | Real / other factorial † (Real observed) | `3.14!`; `[1,2]!` | `Factorial is not supported for Real numbers.`; `… for type 'Vector'.` | `Factorial needs a Natural, but got 'Real'.` (and 'Vector') | same sentences as oracle |
| 8 | Real modulus | `5.0 % 3.0`; `-5.5 % 2`; `5.5 % -2` | `2 (Real)`; `-1.5 (Real)`; `1.5 (Real)` | `Cannot apply '%' to 'Real' and 'Real'.` | matches |
| 8 | Integer modulus sign rule | `-7 % -3`; `17 % -5` | `1 (Integer)`; `-2 (Integer)` | `-1`; `2` (C# dividend-sign rule) | matches oracle (quotient-sign rule) |
| 8 | `divrem` pair and kinds † | `divrem(17,5)`; `divrem(-7,-3)`; `divrem(-17,5)` | q/r `3/2` Natural; `2/1` Integer; `-3/-2` Integer | q/r `3/2` Integer; `2/-1`; `-3/-2` | natural pair Natural, signed pairs match |
| 8 | `divrem` Real refusal † | `divrem(3.14,1.5)` | `divrem() is not supported for values of kind 'Real'. Use Natural or Integer operands.` | `divrem needs integers.` | same sentence as oracle |
| 9 | Arity grammar (regression pin) | `sqrt()`; `sqrt(4,9)`; `pi(10,20)`; `setprecision(10,20)` | `sqrt(): expected 1 argument; got 0.` etc. | already matched on this tree | unchanged (tests added) |
| 9 | Digit-count guards | `setprecision(0)`; `pi(0)`; `pi(1001)`; `e(3.0)`; `evalf(1/3,0)` | typed InvalidArgument/TypeMismatch or DomainError sentences naming position, parameter, range and arrived value | generic DomainError (`setprecision needs a positive integer.` etc.) | same sentence as oracle |
| 10 | Exact scalar division narrows (D2/integer) | `100/5`; `4/2`; `-6/2`; `1/2` | `20 (Natural)`; `2 (Natural)`; `-3 (Integer)`; `0.5 (Real)` | Real 20 / 2 / -3 / 0.5 | Natural 20, Natural 2, Integer -3, Real 0.5 |
| 10 | Array division narrows homogeneously (D2) † | `[4,6]/[2,2]`; `[1,2]/[2,2]`; `[2.5]/[0.25]` | `[2, 3]` all Natural; `[0.5, 1]` all Real; `[10]` Natural | all Real | matches |
| 11 | Mixed literal promotion (D3) † | `[1,2.5,3]`; `[[1,2.5],[3,4]]`; `[1,-2]` | all Real; all Real; all Integer | mixed Natural/Real per element | matches |
| 12 | Empty default range | `1..0`; `5..1`; `len(1..0)` | `[] (Vector)`; `[]`; `0 (Natural)` | `[1, 0]`; `[5, 4, 3, 2, 1]`; `2` | matches |
| 12 | Stepped range is Integer † | `1..2..7`; `5..-1..1` | Integer elements | Natural/Integer per value | Integer elements |

### 1.1 Structural payload checked at the wire

```
structured evalf(1/3,50)      oracle {"kind":"Real","value":"0.333…(50)","exact":false}
                               rust   {"kind":"Real","value":"0.333…(50)","exact":false}
structured divrem(17,5)       oracle q Natural 3, r Natural 2
                               rust   q Natural 3, r Natural 2
structured [4,6]/[2,2]        oracle elements Natural 2, Natural 3
                               rust   elements Natural 2, Natural 3
```

Known residual on the constants: `pi`/`e` now render the oracle's 100-digit DISPLAY, but their
structured `value` carries those 100 digits where the oracle carries the 1000-digit computation
value. The structured exactness flag and absence of a rational payload match. Separating display
from stored value is the numeric-model knob this round did not touch.

## 2. Tests

New file: `rust/lovelace-lang/src/suite_surface_round14.rs`, 17 tests, each asserting an
oracle-observed value (typed text, error sentence + code/category, or the structured payload):

unary plus; slice forms + step guard; D6 empty reductions; evalf exactness/arity/digit guard;
asinh/acosh/atanh; pi/e constants and digit guards; factorial; modulus; divrem; range/empty range;
scalar division narrowing; array division narrowing; D3 promotion; arity grammar.

```
$ cargo test --workspace
exit=0 suites=19 passed=3674 failed=0
```

Baseline before this round was 3,657 green; the delta is exactly the 17 new tests.

## 3. Gates

```
corpus (release):   212 passed, 2 failed, 214 total   (sym/071, sym/076 — unchanged)
fixtures:           oracle 20/20, rust 20/20, PASS
differential:       node parity/diff/run.mjs --cases 200 --seed 1 --no-minimize
```

Differential counts (generated 200 + 1,752 stored regressions replayed):

| pool | before | after | delta |
|---|---:|---:|---:|
| generated match | 147 | **154** | +7 |
| generated mismatch | 53 | **46** | −7 |
| regressions match | 438 | **600** | +162 |
| regressions mismatch | 1,314 | **1,152** | −162 |

Per generated category (match/mismatch): arithmetic 35/1 → **36/0**; precision 16/4 → **20/0**;
arrays 20/12 → 21/11; symbolic 24/12 → 25/11; records 16/12 → 16/12; errors 18/6 → 18/6;
syntax 10/4 → 10/4; registry 8/2 → 8/2. Harness exit remains 1 because the remaining mismatches
and the pre-existing 1,152 regression mismatches are outside this round's surface.

The baseline differential was already red (the regression corpus records known divergences); the
delta above is measured against that same red baseline, not against a green run.

## 4. Not attempted / out of scope

- **Named out of scope:** LReal/Motus/CaptureState/Help/events/numeric-model APIs.
- **Suite rows still refused:** column-vector broadcasting (`[[1],[2]] + [[10,20],[30,40]]`),
  `squeeze` rank, `subs(1/3, x, 2)` with a numeric first argument, `det` publishing Natural where
  the oracle publishes Real, reshape's error taxonomy, the singular-matrix sentence, and the
  norm/display digit cap. These are separate semantics, not the named parser/builtin candidates.
- **Corpus:** sym/071 and sym/076 unchanged (MathIR node order / linsolve canonical shape).
- **Differential residuals:** 46 generated mismatches across arrays (11), symbolic (11), records
  (12), errors (6), syntax (4), registry (2), plus 1,152 replayed regressions. None was weakened or
  hidden.

## 5. Files changed

| file | change |
|---|---|
| `rust/lovelace-lang/src/syntax.rs` | unary plus in the power/unary grammar; slice `start:stop:step` with optional parts |
| `rust/lovelace-lang/src/lib.rs` | display precision; `pi`/`e` variable constants; Integer/Real factorial rules; test module wiring |
| `rust/lovelace-lang/src/builtins.rs` | D3 promotion; empty/stepped ranges; D2 division narrowing; integer/Real remainder; `divrem` kinds+signs; hyperbolic builtins; `pi`/`e`/setprecision digit guards; `evalf` payload; no-value reductions |
| `rust/lovelace-lang/src/suite_surface_round14.rs` | new: 17 tests |
| `docs/goal-parity-contract/round-14/suite-surface.md` | this report |
