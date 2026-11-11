# Round 9 — unary-minus precedence and the documented recursion depth

Scope: `rust/lovelace-lang/src/syntax.rs` (the parser the engine actually uses),
`rust/lovelace-lang/src/lib.rs` (tests), and `rust/lovelace-run/src/main.rs` (the evaluation
thread). No C# source, fixture, `parity/` file or other doc was changed.

Environment for every command below:

```powershell
$repo='C:\Users\ricar\dev\LovelaceSharp'
$env:RUSTUP_HOME="$repo\.toolchain\rustup"; $env:CARGO_HOME="$repo\.toolchain\cargo"
$env:PATH="$repo\.toolchain\cargo\bin;" + $env:PATH
$env:TMP="$repo\rust\target\tmp"; $env:TEMP=$env:TMP
```

The **before** probes were captured from binaries built from the unmodified source at the start of
this round. To re-run them as an exact before/after pair after the edits, the workspace source was
copied to `rust/target/tmp/before/rust`, the two edits below were reverted in that copy only, and it
was built with `CARGO_TARGET_DIR=rust/target/tmp/before-target`:

```powershell
robocopy rust rust\target\tmp\before\rust /E /XD target     # 59 files
$env:CARGO_TARGET_DIR="$repo\rust\target\tmp\before-target"
cargo build -p lovelace-run              # pre-fix debug   (exit 0)
cargo build --release -p lovelace-run    # pre-fix release (exit 0)
```

## 1. Defect 1 — unary minus bound tighter than exponentiation

### Root cause

`rust/lovelace-lang/src/syntax.rs` parsed `power` as `range()` followed by `^`, and `range()`
called `unary()`, which consumed the leading `-`. So `-2^2` became `Pow(Neg(2), 2)` = 4.
The reference parser (`Lovelace.Suite/Parser.cs`, `ParsePower`/`ParseUnary`/`ParseRangeTail`) handles
the sign at the POWER level: the sign negates the whole power. (`rust/lovelace-lang/src/parser.rs`
is a dead, uncompiled file — `lib.rs` declares only `syntax` — but it happened to carry the correct
shape and was used as the model.)

### Fix

`rust/lovelace-lang/src/syntax.rs:569` — `power()` now consumes a leading `-` itself, parses its
operand with `unary()`, negates `power_tail(operand)` when `^` follows, and otherwise negates the
operand (with the reference's `..` case handled explicitly). `power_tail()` keeps `^`
right-associative and parses its right operand with `power()` (so `2^-2` still works). `range()`
now delegates its tail to a new `range_tail()`. This is a direct port of the reference grammar;
only the binding of the sign moved.

### Runner probes — exact stdout, `--text`

```powershell
# before
& "$repo\rust\target\tmp\before-target\release\lovelace-run.exe" --eval '-2^2' --text
# after / oracle
& "$repo\rust\target\release\lovelace-run.exe" --eval '-2^2' --text
dotnet exec Lovelace.Run\bin\Release\net10.0\Lovelace.Run.dll --eval '-2^2' --text
```

| script | before (`= …`) | after (`= …`) | oracle (`= …`) |
|---|---|---|---|
| `-2^2` | **`= 4 (Integer)`** | **`= -4 (Integer)`** | `= -4 (Integer)` |
| `-2^3` | `= -8 (Integer)` | `= -8 (Integer)` | `= -8 (Integer)` |
| `2^-2` | `= 0.25 (Real)` | `= 0.25 (Real)` | `= 0.25 (Real)` |
| `-2^2+1` | **`= 5 (Integer)`** | **`= -3 (Integer)`** | `= -3 (Integer)` |
| `(-2)^2` | `= 4 (Integer)` | `= 4 (Integer)` | `= 4 (Integer)` |
| `-2*3` | `= -6 (Integer)` | `= -6 (Integer)` | `= -6 (Integer)` |
| `-2+3` | `= 1 (Integer)` | `= 1 (Integer)` | `= 1 (Integer)` |
| `x = 3; -x^2` | **`= 9 (Integer)`** | **`= -9 (Integer)`** | `= -9 (Integer)` |

Exact stdout for the fixed case (two lines, the second begins with two spaces):

```text
= -4 (Integer)
  _ = -4
```

`-2^3` is unchanged at -8 because `-(2^3) = (-2)^3`; it is pinned as a test all the same.
The JSON envelope after the fix for `-2^2 --omit-functions --omit-variables`:
`ok=True display=-4 typed=-4 (Integer) structured={"exact":true,"kind":"Integer","value":"-4"}`.

Rendering constraint: `-2*3` and `-2+3` are byte-identical before and after (their AST is
unchanged; only a power's binding moved).

Extra edge forms, before → after (every after value equals the oracle's):

| script | before | after |
|---|---|---|
| `-2^-2` | `0.25 (Real)` | `-0.25 (Real)` |
| `--2^2` | `4 (Integer)` | `-4 (Integer)` |
| `-2^2*3` | `12 (Integer)` | `-12 (Integer)` |
| `2*-2^2` | `8 (Integer)` | `-8 (Integer)` |
| `(-2)^-2` | `0.25 (Real)` | `0.25 (Real)` (unchanged) |
| `-2.0^2` | `4 (Real)` | `-4 (Real)` |
| `1..3 ^ 2` | `[1, 4, 9] (Vector)` | unchanged |
| `-1..2` | `[-1, 0, 1, 2] (Vector)` | unchanged |
| `2^3^2` | `512 (Natural)` | unchanged |
| `-(2)^2` | `4 (Integer)` | `-4 (Integer)` |
| `2*-3^2` | `18 (Integer)` | `-18 (Integer)` |
| `-2!^2` | `4 (Integer)` | `-4 (Integer)` |
| `1+-2^2` | `5 (Integer)` | `-3 (Integer)` |
| `1--2^2` | `-3 (Integer)` | `5 (Integer)` |

Not changed / out of scope: `-1..2^2` still computes `[1, 0, 1, 4]` in Rust while the oracle
raises `InvalidArgument/TypeMismatch: Base cannot be zero.` because its `0^2` is refused. The
parsed tree of that script is the same before and after this round (both build
`Pow(Range(-1, 2), 2)`), so this is a pre-existing vector-power difference, not a precedence change.

## 2. Defect 2 — the documented deepest recursion

### Reproduction (before)

```powershell
$s="func f(n) { if (n == 0) { return 0 }; return f(n - 1) }; f(85)"
& "$repo\rust\target\tmp\before-target\debug\lovelace-run.exe" --eval $s --text
```

Observed: exit `-1073741571` (`0xC00000FD`), **stdout empty**, stderr:

```text
thread 'main' (23628) has overflowed its stack
```

The release profile on this host happened to survive f(85) (exit 0, `= 0 (Natural)`) but died the
same way at f(200) — the native stack, not a budget, was the limit in both profiles.

### Fix

`rust/lovelace-run/src/main.rs:214` reserves `RUNNER_STACK_BYTES = 256 MiB` and `main()` spawns the
whole runner (`fn run()`, line 231 — argument handling, parsing, evaluation, rendering) on a thread
with that stack. The reservation is virtual address space committed lazily by the OS, not resident
memory; a panic on the worker is re-raised with `resume_unwind` so the default panic contract and
exit code (101) survive.

The engine itself is unchanged: a library caller that evaluates `f(85)` on its own small thread still
has the native-stack bound. The documented runtime contract this round had to restore is the
runner's, and the runner now runs every evaluation on the sized thread.

### Runner probes — exact stdout

```powershell
$s="func f(n) { if (n == 0) { return 0 }; return f(n - 1) }; f(85)"
& rust\target\debug\lovelace-run.exe   --eval $s --text
& rust\target\release\lovelace-run.exe --eval $s --text
dotnet exec Lovelace.Run\bin\Release\net10.0\Lovelace.Run.dll --eval $s --text
```

| runner | f(85) before | f(85) after | f(200) after |
|---|---|---|---|
| Rust debug | exit `-1073741571`, stdout empty, stack-overflow stderr | exit 0, `= 0 (Natural)` | exit 0 |
| Rust release | exit 0, `= 0 (Natural)` (dies at f(200) with `0xC00000FD`) | exit 0, `= 0 (Natural)` | exit 0 |
| C# oracle | exit 0, `= 0 (Natural)` | exit 0, `= 0 (Natural)` | — |

Exact stdout after the fix, debug and release (identical):

```text
= 0 (Natural)
  _ = 0
```

This is byte-identical to the oracle's stdout for the same script.

Boundary note (not a regression, still different from the oracle): the oracle refuses f(86) with
`Error [DepthExceeded/BudgetExceeded]: evaluation depth 513 exceeds the maximum supported evaluation
depth of 512. …`. The Rust engine has no evaluation-depth budget (it had none before this round
either; pre-fix release accepted f(86)), so Rust now accepts f(86) in both profiles. The requirement
was that the same script the oracle ACCEPTS (f(85)) is accepted with the same result; that holds.

## 3. Tests added

1. `rust/lovelace-lang/src/lib.rs:1027` — `Eval_GivenUnaryMinusBeforePower_BindsLooserThanPower`
   (10 assertions: `-2^2`, `-2^3`, `x = 3; -x^2`, `(-2)^2`, `2^-2`, `-2^2+1`, `2^3^2`,
   `-1..2`, plus the rendering guards `-2*3`, `-2+3`).
2. `rust/lovelace-run/tests/recursion_depth.rs:17` —
   `the_documented_deepest_recursion_is_accepted`: spawns the built `lovelace-run` binary with the
   audit's `f(85)` script, requires exit 0, empty stderr, `ok:true`, `display:0`, `typed:0 (Natural)`.
   This is the test that fails with `0xC00000FD` if the large-stack thread is removed.

```text
cargo test -p lovelace-lang Eval_GivenUnaryMinusBeforePower
  test tests::Eval_GivenUnaryMinusBeforePower_BindsLooserThanPower ... ok
  test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 540 filtered out

cargo test -p lovelace-run --test recursion_depth
  running 1 test
  test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

The recursion test is a real guard: run against the pre-fix reconstruction it fails exactly as the
defect did.

```text
cargo test -p lovelace-run --test recursion_depth   (pre-fix binary, same test source)
  thread 'the_documented_deepest_recursion_is_accepted' panicked at recursion_depth.rs:24:5:
  assertion `left == right` failed: f(85) must exit 0; status Some(-1073741571), stderr:
  thread 'main' (22964) has overflowed its stack
  test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out
```

## 4. Gates

### Workspace tests — green

```text
cmd /c "cargo test --workspace > target\tmp\verify4-test.log 2>&1"; TEST_EXIT=0
58 + 67 + 125 + 7 + 541 + 134 + 2308 + 101 + 1 + 27 = 3,369 passed, 0 failed (doctests: 0)
```

The round started at 3,266 passing tests. This change adds 2: the language precedence test (the
541 above is 540 + 1) and the runner integration test. The other increase is a CONCURRENT agent's
work: while this round ran, that agent added `rust/lovelace-run/src/parity_batch.rs`,
`wire_batch.rs`, `golden_batch.rs` and `pretty_batch.rs` (runner unit tests, 101 at the final run,
not authored here; all pass against the changed runner). The tree was transiently red while those
files were half-written (a test helper omitted variables while its assertion read them); after that
agent finished, the whole gate above was clean. The release runner was rebuilt from the current
source before the probes and fixtures.

### Documentation corpus — unchanged

```text
cargo run --release -p lovelace-corpus
FAIL sym/071
FAIL sym/076
corpus: 212 passed, 2 failed, 214 total      (exit 1, the same two pre-existing failures)
```

`sym/071` (`lower(optimize(...))`) and `sym/076` (`linsolve` simplification) are identical before and
after; no precedence case is in the corpus.

### Golden fixtures — green

```text
node parity/tools/gate-fixtures.mjs
oracle: 20/20
rust: 20/20
PASS: oracle 20/20, rust 20/20             (exit 0)
```

## 5. Files changed

| file | change |
|---|---|
| `rust/lovelace-lang/src/syntax.rs` | `power`/`power_tail`/`range`/`range_tail`: sign handled at power level; `range_tail` split out |
| `rust/lovelace-lang/src/lib.rs` | one precedence test |
| `rust/lovelace-run/src/main.rs` | 256 MiB evaluation thread; old `main` body is `run()` |
| `rust/lovelace-run/tests/recursion_depth.rs` | new integration test (this change) |
| `docs/goal-parity-contract/round-9/language-fixes.md` | this report |

Untouched: all `.cs`, `parity/`, `Lovelace.Run.Tests/fixtures/`, every other doc. The pre-fix
reconstruction lives under `rust/target/tmp/` (build output, not source).
`rust/lovelace-run/src/{parity,wire,golden,pretty}_batch.rs` are a concurrent agent's new files
(runner unit tests), not part of this change; the `#[cfg(test)] mod …;` declarations they need in
`main.rs` are that work's.
