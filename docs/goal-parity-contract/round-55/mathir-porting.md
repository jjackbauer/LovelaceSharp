# Round 55 — the MathIR typing rows: seven mirrors, fifteen mappings, and the typing fix that unblocked the last one

**Status: all seven `MathIrTypingTests` rows and all eight `CompilationTests` rows are mirrored and
mapped (15 of 15); every gate is green.** The seventh typing row needed the one authorized
production change of this round: `IrTyping::infer` now applies the reference's **one-input**
transcendental rule to unary ops (so `exp(2)` is `Real`) and reads both operands for `Div`. A
throwaway probe first proved the divergence; after the fix the full row mirrors verbatim.

Measured 2026-11-11 (America/Sao_Paulo) on the round-54 working tree (`813755c` plus the
uncommitted round-54 additions), workspace suite 5,000 tests green.

## 1. What this round changed

* **New:** `rust/lovelace-lang/src/mathir_typing_round55.rs` — the seven mirrors of section 3, each
  driving `lovelace_lang::mathir` directly (never the language's `evalir` builtin) with the C#
  case's own program.
* **Edited:** `rust/lovelace-lang/src/mathir.rs` — the authorized typing fix of section 4: a
  `transcendental_unary` rule for the unary ops and a two-operand `Div`, with a comment tying the
  rule to `Typing.cs`. This is the round's only behavioural production change; it was made after
  the round had already been reported with the row deferred, and it is the fix that report
  proposed.
* **Edited:** `rust/lovelace-lang/src/lib.rs` — one line, `mod mathir_typing_round55;`.
* **Edited:** `parity/manifest-map.json` — all 15 rows mapped `ported` to their exact Rust test;
  the generated `parity/manifest.toml` and `parity/coverage.md` were regenerated from it.
* **Regenerated:** `parity/rust-tests.json` (5,000 tests, suite exit 0).
* **Untouched:** every `.cs` file, fixture, corpus file, `builtins.rs` and runner envelope. The
  round-54 tests in `mathir_round54.rs` are reused unchanged for the eight `CompilationTests`
  rows.

## 2. The eight CompilationTests rows

Round 54 delivered the eight mirrors; this round maps them and pins the exact test location. The
C# member contract (`Compilation.Compile` / `CompileKernel` / `ToKernel`, `IrProgram`,
`CompiledKernel`, `IrEvaluator`) is the one described in
`docs/goal-parity-contract/round-54/compilation-ir.md` section 2.

| Census id (`CompilationTests`) | Rust test | C# assertion mirrored |
|---|---|---|
| `CompilationResult_IrText_RoundTrips#0` | `lovelace-lang::mathir_round54::compilation_result_ir_text_round_trips` | `Deserialize(IrText).Serialize() == Program.Serialize()`; also pins the 9-line text, typed nodes and pool |
| `Kernel_BatchEvaluation_Columnwise#0` | `…::kernel_batch_evaluation_columnwise` | lanes 2/5/10; mismatched columns refused as `Column 'y' has 1 lanes; expected 2.` |
| `Kernel_BatchLazySelect_MixedGuards#0` | `…::kernel_batch_lazy_select_mixed_guards` | `x = [2, -3, 1/2]` → 1/2, 3, 2 with both branches demanded |
| `Kernel_BatchMatchesScalarEvaluation#0` | `…::kernel_batch_matches_scalar_evaluation` | every batch lane equals its scalar evaluation at three rational pairs, precision 40 |
| `Kernel_ExactTiers_Preserved#0` | `…::kernel_exact_tiers_preserved` | `2*x^3 + 1/2` at `1/2` is the exact `NumRat` 3/4 |
| `Kernel_LazySelect_UntakenBranchNeverEvaluates#0` | `…::kernel_lazy_select_untaken_branch_never_evaluates` | 1 at `x = 0` without dividing, 1/2 at `x = 2` |
| `Kernel_NamedAndPositional_Evaluate#0` | `…::kernel_named_and_positional_evaluate` | positional 7, named 7, one-argument refusal |
| `Kernel_Transcendental_ApproximateTier#0` | `…::kernel_transcendental_approximate_tier` | `NumReal` (inexact) equal to 1 at precision 40 |

Every one of these test ids was confirmed present in `parity/rust-tests.json` after regeneration;
the manifest gate resolved all eight (section 5).

## 3. The seven MathIrTypingTests mirrors

Sources: `Lovelace.Symbolics.Tests/MathIrTypingTests.cs` (lines 21-121) and
`Lovelace.MathIR/Typing.cs`. All seven live in `mathir_typing_round55` and assert the C# case's
concrete behaviour, not a weaker approximation.

| Census id (`MathIrTypingTests`) | Rust test | What the mirror asserts |
|---|---|---|
| `Infer_TypesForLoweredExpressions#0` | `infer_types_for_lowered_expressions` | `Integer` for `x^2 + 1`, `Rational` for `x / 2`, `Real` for `exp(2)`, unconstrained for `exp(x)`, `Bool` at the `Gt` guard a piecewise emits, `Integer` for the Select that wraps it, and `Complex` for `(1 + i) * x` |
| `Validate_AcceptsLoweredCorpus#0` | `validate_accepts_lowered_corpus` | the same five corpus expressions over `x`, `y` lower; `IrTyping::validate` is Ok, a root exists, and the accepted text round-trips through `IrProgram::deserialize` |
| `Validate_RejectsOutOfRangeOperand#0` | `validate_rejects_out_of_range_operand` | `Add 0 0,42` is refused with `IR validation: node 2 (Add) references out-of-range operand 42.`, code `InvalidOperation`, category `DomainError`; the in-range operand validates |
| `Validate_RejectsNonBoolSelectGuard#0` | `validate_rejects_non_bool_select_guard` | `Select 2,0,1` over an `Integer` Add is refused with `Select node 3 requires a Bool guard, got Integer.`; the guard's inferred type is `Integer` and the prefix without the Select validates |
| `Validate_RejectsOrderedComparisonOfComplex#0` | `validate_rejects_ordered_comparison_of_complex` | `Lt` over the `(cplx 1 1 1 1)` pool entry is refused with `ordered comparison Lt requires real operands, node 2 operand 0 is Complex.`; the constant types as `Complex` |
| `Validate_RejectsWrongArity#0` | `validate_rejects_wrong_arity` | `Add 0 0,0` is refused with `node 1 (Add) has 3 operands; expected 2.`; `IrOpKind::Add` arity is 2 and the binary Add validates |
| `Deserialize_RejectsMalformedProgram#0` | `deserialize_rejects_malformed_program` | the C# text `Add 0 0,42` is refused by `deserialize` with the validate sentence and the reference taxonomy; an unreadable opcode is refused with `Malformed IR line: Addish 0 0` (`InvalidInput`/`ParseError`); the well-formed program deserializes |

Two deliberate points about the mirror, neither of which changes an expected value:

1. **The Validate rows go through `deserialize`.** C# `IrProgram` exposes three mutable lists, so
   its rows build a defective program node by node; the Rust `IrProgram` keeps them private and
   offers no in-memory builder. `IrProgram::deserialize` documents that its last step IS
   `IrTyping::validate`, and these texts parse cleanly, so the sentence asserted is validate's own
   first refusal — exactly what `Assert.Throws<InvalidOperationException>` pins. Each mirror also
   validates the defect-free counterpart with a direct `IrTyping::validate` call, which localizes
   the refusal to the one defect.
2. **`x / 2` is built as the reference builds it.** C# `Exprs.Divide(x, 2)` is
   `Multiply(x, Power(2, -1))`, and `Power` folds the numeric pair to `(rat 1 2)`; `Sym::div` is
   that same constructor, so the program is a product over a Rational constant. The expression
   type's explicit `Div` leaf (which C# does not have) is not what this row constructs.

## 4. The typing fix that unblocked the seventh row

The C# row (`MathIrTypingTests.cs:21`) makes six assertions (five tiers plus the Select's promoted
type). Four of the tier facts held from the start; `Real` for `exp(2)` did **not**:

* C# `IrTyping.Infer` (`Typing.cs:31`) calls `Transcendental(T(0))` for every unary op, so
  `exp(Integer)` is `Real`; `Transcendental(T(0), T(1))` is used only for `Pow`
  (`Typing.cs:125-136`).
* Rust `IrTyping::infer` called `transcendental(left, None)` for every non-`Pow` member of the
  group, and `transcendental` returns `None` when any input is `None`. The synthetic second
  input therefore made **every unary transcendental — and `Div` — infer unconstrained, whatever
  its operand's type was**.

Observed before the fix with a throwaway integration test (created, run, then deleted; it is not
part of the shipped tree):

    running 1 test
    test exp_of_integer_infers_real ... FAILED

    thread 'exp_of_integer_infers_real' (2272) panicked at lovelace-lang\tests\round55_exp_probe.rs:13:5:
    assertion `left == right` failed
      left: None
     right: Some(Real)

A second throwaway probe asserted the four facts that already held and passed
(`test four_infer_facts_hold ... ok` / `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured;
0 filtered out`), which isolated the divergence to the transcendental rule.

**The fix (applied, `mathir.rs`).** The unary arms now call a new one-input rule,
`transcendental_unary`, which reproduces `Typing.cs`'s `Transcendental(T(0))`: `None` stays
unconstrained, `Complex` stays `Complex`, any other typed input is `Real`. `Div` shares the
two-input form with `Pow`, because the Rust surface emits a `Div` node for an explicit quotient
where the reference emits `Multiply(x, Pow(y, -1))` and `Mul`'s promote reads both sides. The
source comment states both rules and names the reference lines they mirror.

**No other row's typing output changed.** The only pre-existing assertions that read
`IrTyping::infer` or `CompilationResult::result_type` are round-54's Add/Mul/parameter tier test
and round-55's Add-guard and Complex-constant facts; the fix touches neither Add/Mul promotion nor
constant classification. The full suite went from 4,993 (round-54 baseline) to 5,000 with no
existing expectation edited, and the corpus, protocol fixtures and CLI matrix — which exercise the
runner, not the library — are unchanged and green (section 5).

## 5. Gate outputs

Cargo commands ran from the repository root with the repository toolchain environment
(`RUSTUP_HOME`, `CARGO_HOME`, `PATH`, `TMP`); the Node gates ran from the root. The release
runner was rebuilt after the fix and before the fixture and CLI gates.

| Gate | Command | Observed | Exit |
|---|---|---|---:|
| Workspace suite (recorded by the inventory) | `cargo test --workspace` inside `parity/gen-rust-tests.ps1` | `passed=5000 failed=0` across 11 crates | 0 |
| New mirrors | `cargo test -p lovelace-lang mathir_typing_round55` | `7 passed; 0 failed; 0 ignored; 0 measured; 1497 filtered out` | 0 |
| Inventory | `parity/gen-rust-tests.ps1` | `crates=11 tests=5000 suiteExit=0 passed=5000 failed=0` | 0 |
| Manifest generation | `node parity/gen-manifest.mjs` | `rows: 6209`, `ported: 5857`, `deferred: 234`, `excluded: 118`, `coveragePercent: 94.33`, `rustTests: 5000` | 0 |
| Manifest gate | `node parity/tools/gate-manifest.mjs` | `PASS (structural): 6209 test cases dispositioned; 5857 claim Rust coverage (94.33%).` with `"failures": []`; `claimedRustTests: 5857` | 0 |
| Coverage report | `node parity/tools/coverage-report.mjs` | `coverage.md written: 5857/6209 ported (94.33%)` | 0 |
| Release runner | `cargo build --release -p lovelace-run` | `Finished` `release` `profile [optimized] target(s) in 21.16s` | 0 |
| Corpus | `cargo run --release -p lovelace-corpus` | `corpus: 214 passed, 0 failed, 214 total` | 0 |
| Protocol fixtures | `node parity/tools/gate-fixtures.mjs` | `oracle: 20/20`, `rust: 20/20`, `PASS: oracle 20/20, rust 20/20` | 0 |
| CLI matrix | `node parity/tools/gate-cli.mjs` | `CLI matrix: 16/16`, `PASS: every RUN-001..RUN-014 row behaves like the oracle` | 0 |
| Format | `cargo fmt --all -- --check` | clean | 0 |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` | clean, no warnings | 0 |

The manifest gate's freshness check ran against the same tree (its `sourcesSha256` comparison
passed), so all 15 mappings resolve to tests that exist in the measured inventory **and** whose
suite ran green.

## 6. Coverage and what remains

* Census coverage moved from **5,842/6,209 (94.09 %)** to **5,857/6,209 (94.33 %)**; deferred moved
  from 249 to 234, exclusions are unchanged at 118. `Lovelace.Symbolics.Tests` is now
  1,099/1,226 ported (89.64 %), 127 deferred, 0 excluded.
* **No unresolved ids.** All 8 `CompilationTests` rows and all 7 `MathIrTypingTests` rows of this
  round resolve to exact tests (15/15).
* Nothing else in the round-50 resweep's "IrProgram.Deserialize/Validate/Infer typing API" group
  remains: every row is mapped here or in section 2.
