# Round 93 — the six deferred eval/print families (12 rows)

**Status: fixed and pinned, 12/12 rows closed, 0 blocked.** Every row below had
`disposition = "deferred"` in `parity/manifest.toml`. Four families needed production surface
that did not exist (the kernel print budget, the route-carrying real literal, the fallible value
walk and the docs checker); the evalir family needed two fixes in the digit/evalir payload path.
Measured 2026-11-12 (America/Sao_Paulo) against
`Lovelace.Run/bin/Release/net10.0/Lovelace.Run.dll` and
`rust/target/release/lovelace-run.exe` (sha256 `7d9306516c345219858d63e518048ff60f6d2bd02232d4fc7aa24b66f6f295ca`).

Scope: `rust/lovelace-lang/src` (production and tests). No `.cs` file, fixture, corpus,
`parity/` artifact or other document was edited. Probes and captured envelopes live under
`rust/target/tmp/r93/` (git-ignored). The differential run was written to
`rust/target/tmp/r93/diff-200`, not under `parity/`.

## 1. The 12 ids (extracted from parity/manifest.toml)

| # | Class | Id | Rust test (all in `round93_eval_print`) |
|---:|---|---|---|
| 1 | EvalirDigitArgumentContract | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.EvalirDigitArgumentContractTests/AnExactlyIntegralRealLiteral_IsTheIntegerItDenotes#0` | `evalirdigitargumentcontracttests_an_exactly_integral_real_literal_is_the_integer_it_denotes_0` |
| 2 | EvalirDigitArgumentContract | `.../AnExactlyIntegralRealLiteral_IsTheIntegerItDenotes#1` | `..._the_integer_it_denotes_1` |
| 3 | PrintBudget | `.../PrintBudgetTests/DepthBudget_ReportsDepthLimit#0` | `printbudgettests_depth_budget_reports_depth_limit` |
| 4 | PrintBudget | `.../PrintBudgetTests/NodeBudget_OnACompactRendering_DropsNothingAndSaysSo#0` | `printbudgettests_node_budget_on_a_compact_rendering_drops_nothing_and_says_so` |
| 5 | PrintBudgetTokenBoundary | `.../PrintBudgetTokenBoundaryTests/AnIdentifierLongerThanTheAllowance_IsKeptWhole#0` | `printbudgettokenboundarytests_an_identifier_longer_than_the_allowance_is_kept_whole` |
| 6 | PrintBudgetTokenBoundary | `.../PrintBudgetTokenBoundaryTests/ARenderingWithinTheAllowance_IsUntouched#0` | `printbudgettokenboundarytests_a_rendering_within_the_allowance_is_untouched` |
| 7 | TruncatingEvaluationStaysInexact | `.../TruncatingEvaluationStaysInexactTests/FunctionOfATruncatedArgument_IsInexact#0` | `truncatingevaluationstaysinexacttests_function_of_a_truncated_argument_is_inexact` |
| 8 | TruncatingEvaluationStaysInexact | `.../TruncatingEvaluationStaysInexactTests/LiteralRoundTrip_KeepsGenuinelyExactValuesExact#0` | `truncatingevaluationstaysinexacttests_literal_round_trip_keeps_genuinely_exact_values_exact` |
| 9 | ValueDepthGuard | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.ValueDepthGuardTests/ADeepRecordChain_IsRefusedTheSameWay#0` | `valuedepthguardtests_a_deep_record_chain_is_refused_the_same_way` |
| 10 | ValueDepthGuard | `.../ValueDepthGuardTests/ValueOnePastTheBudget_IsRefusedByTheStructuredProjection#0` | `valuedepthguardtests_value_one_past_the_budget_is_refused_by_the_structured_projection` |
| 11 | DocsSync | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.DocsSyncTests/Every_CSharpSnippet_AppearsInUsageExamples#0` | `docssynctests_every_c_sharp_snippet_appears_in_usage_examples` |
| 12 | DocsSync | `.../DocsSyncTests/Every_LovelaceExample_HasAnExpectedResult#0` | `docssynctests_every_lovelace_example_has_an_expected_result` |

The `...` prefix is
`Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.` for the Symbolics rows and
`Lovelace.Suite.Tests/Lovelace.Suite.Tests.` for the Suite rows.

## 2. Probes: exact differing field and both values

Rows 1–2 and 9–10 have runner argv. Rows 3–8 are library-only C# rows and rows 11–12 are library/repo-only checks
(`Printing.Print`, `RealLiteral`, `ValueFormatter`/`StructuredProjection`); for those the probe is
the library call and the runner proxy is recorded where one exists. All 12 rows were probed;
"no Rust API" below means the deferred reason was a missing surface, not a silent failure.

| # | Probe | Oracle (before) | Rust (before) | After |
|---:|---|---|---|---|
| 1 | `--file evalir_2_0.ls`: `x = symbol("x"); evalir(compile(x*1/3, [x]), [1], 2.0)` | exit 0; `$.result.display` = `0.33`; `$.result.structured.exact` = `false` | exit 1; `$.code` = `InvalidArgument`; `$.category` = `TypeMismatch`; message "…got the Real 2, which is integral — write it as 2."; `$.result` absent | exit 0; display `0.33`; `exact:false`; envelope identical after the four volatile keys |
| 2 | `--file evalir_batch_2_0.ls`: same with `[1, 2]` | exit 0; display `[0.33, 0.66]`; every element `exact:false` | exit 1; same `InvalidArgument/TypeMismatch` refusal | exit 0; display `[0.33, 0.66]`; elements `exact:false` |
| 3 | library: `Print(big_expand(12), MaxDepth: 2)` | `Truncated=true`, `Reason="depth-limit"`, `Budget=2`, `NodeCount=big.NodeCount`, text ends `" …"` | no kernel budget API; `structured::project_value` accepts a node budget only and can never publish `depth-limit` | `print_budget::print` returns the oracle's four fields; test asserts all of them |
| 4 | library: `Print(x + 1, MaxNodes: 1)` | `Truncated=false`, `Truncation=null`, text `x + 1` | no kernel budget API; runner proxy `--print-budget 1` (the projection, which totalizes the node budget) returns `truncated:true`, pretty `x …` | `Truncated=false`, `Truncation=None`, text `x + 1` |
| 5 | library: 54×`a` symbol + 1, `MaxNodes: 1` | body is the WHOLE 54-character name; `" …"` appended; `truncated:true`, `node-budget`, `budget:1` | no kernel budget API; the runner projection already returned the whole name + `" …"` | same body/fields through `print_budget::print`; token-boundary assertion added |
| 6 | library: 44×`a` symbol + 1 (48 chars), `MaxNodes: 1` | `Truncated=false`, `Truncation=null`, text is the full 48-character rendering | no kernel budget API; runner projection `--print-budget 1` returns `truncated:true`, 44-name + `" …"` (measured on both runners) | `Truncated=false`, text full; the allowance returns it whole |
| 7 | library: every elementary function of every truncated argument (11 args × 9 functions) | each result is a `NumReal` with `IsExact=false` | no route-carrying `RealLiteral` API; the language proxy `evalf(sin(pi(30)/6), 40)` already matched (display `0.499…`, `exact:false`) | `RealLiteral::from_real_exact(...).to_real()` keeps the route; 99/99 evaluated pairs are inexact |
| 8 | library: `RealLiteral.FromRealExact(v).ToReal()` for 7 genuinely exact values | value equal and `IsExact=true` for 0, 1, 1/2, -3/2, 1/8, 1000000, 1/(10^19) | no route-carrying `RealLiteral` API; language proxy `evalf(1/2, 40)` = `0.5`, `exact:true` on both | 7/7 round trips preserve the digits and the exact route |
| 9 | library: display/structured walk over a 1023/1024/20000-level record chain | depth 1024 renders; 1025 refused by `ValueFormatter.Format`; 20000 refused by `StructuredProjection.ToStructured` instead of dying | no fallible display/projection API for hand-built values; the ENGINE refused a 1024-wrap script (`DepthExceeded`, "value depth 1025 … of 1024") but `Value::display`/`project_value` still recursed | `Value::try_display` returns `DepthExceeded/BudgetExceeded` at 1025; `structured::try_project_value` refuses 20000 without recursing; 1024 still renders |
| 10 | library: `StructuredProjection.ToStructured(Nested(1024))` | `InputDepthExceededException`; message contains "value depth 1025 exceeds the maximum supported nesting depth of 1024" | no fallible projection API; the direct call recurses | `try_project_value` refuses at 1025 with the same prefix; the depth-1024 control still projects `kind:Array` |
| 11 | library/repo: every fenced `csharp` block in `Lovelace.Symbolics/README.md` appears (whitespace-normalized) in `UsageExamples.cs` | 8/8 snippets present; non-empty snippet list | no checker API | `docs_sync::check_csharp_snippets_appear` checks 8/8 |
| 12 | library/repo: every fenced `lovelace` block followed by a fenced `result` block has a non-blank expected result | 103 examples, every expected result non-blank | no checker API | `docs_sync::parse_examples` returns 103 examples; 103/103 non-blank |

Runner probes for the two rows that are expressible as scripts:

```text
depth1023 wraps: oracle ok=True / rust ok=True
depth1024 wraps: oracle DepthExceeded "value depth 1025 exceeds the maximum supported nesting depth of 1024. ..."
                 rust   DepthExceeded (identical message)
```

The projection proxies for rows 4 and 6 are NOT the rows' surface: `--print-budget` is the
suite projection, which deliberately makes the node budget total, while these C# rows call the
kernel `Printing.Print`, which returns a compact rendering whole and reports nothing. That is why
the rows stayed deferred behind "no Rust API", and the new `print_budget` module is the kernel
surface the tests drive.

## 3. Production fixes

1. **Integral Real digit count accepted (rows 1–2)** — `rust/lovelace-lang/src/builtins.rs:3744`
   `evalir_digits` now accepts a `Value::Real` only when it is EXACT and integral, and passes its
   integer decimal through the SAME bounds check and message as a `Natural`/`Integer`. Non-exact
   integral Reals keep the existing "which is integral" refusal; `2.5` keeps the exact-integer
   refusal (re-probed against the oracle, identical message). This is the reference host's
   normalization of `2.0` to the integer 2.
2. **Truncated exact rationals publish inexact (rows 1–2, full-envelope fix)** —
   `rust/lovelace-lang/src/numeric.rs:739` `rational_to_published` now marks the published Real
   inexact whenever the decimal expansion did not terminate within the requested places (the old
   rule only marked it at 1000 places). The evalir result is then envelope-identical to the
   oracle: `0.33`/`[0.33, 0.66]` with `exact:false`, while the terminating controls
   (`evalir(..., 2)`, `evalir(..., x*1/2)`) stay exact.
3. **Kernel print budget (rows 3–6)** — new `rust/lovelace-lang/src/print_budget.rs`:
   `PrintBudget { max_nodes, max_depth }`, `PrintTruncation`, `PrintOutcome`, `print` and
   `expr_depth`. It ports `Printing.Print` exactly: node cap consulted first, depth cap only when
   no node cap was given; a capped rendering is abbreviated only when it is longer than
   `max(48, 6 × limit)` characters; the cut is at a token boundary with the whole-first-token
   rescue and the ellipsis-alone degenerate case. The depth measure is iterative.
4. **Route-carrying real literal (rows 7–8)** — new
   `rust/lovelace-lang/src/real_literal_route.rs`: `RealLiteral::from_real_exact` stores the
   decimal digits and the `Real::is_exact` route; `to_real` parses the digits and restores the
   route with `mark_inexact` when the source was a truncation. The compact
   `lovelace_symbolics::RealLiteral` has no route field and is unchanged.
5. **Fallible value walks (rows 9–10)** — `rust/lovelace-lang/src/lib.rs:2264`
   `check_value_depth` is public and iterative; `Value::try_display`/`try_typed`
   (`lib.rs:341`) refuse before the recursive formatter, naming `value depth N … of 1024`.
   `rust/lovelace-lang/src/structured.rs:27` `try_project_value` refuses before the recursive
   projection. The engine's own post-eval check now calls the same `check_value_depth`.
6. **Docs sync checker (rows 11–12)** — new `rust/lovelace-lang/src/docs_sync.rs`: the reference
   `UsageDocParser` line rules (fenced `lovelace`/`result` pairs, fenced `csharp` blocks),
   whitespace normalization, and `check_csharp_snippets_appear` returning every missing snippet.
   The tests read the repository's `Lovelace.Symbolics/README.md` and
   `Lovelace.Symbolics.Tests/UsageExamples.cs`; no document is edited.

## 4. Tests (12/12, one per row)

`rust/lovelace-lang/src/round93_eval_print.rs` (declared in `lib.rs`) holds the twelve tests named
in §1. Run:

```text
$ cargo test -p lovelace-lang round93
running 12 tests
test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 1672 filtered out
```

The expected values are the C# assertions (oracle observables), never read off the Rust output:
the evalir rows compare the two call forms, the two-place rendering and the published exactness; the print rows assert the
oracle's `Truncated`/`Reason`/`Budget`/`NodeCount`/`Text`; the truncation rows assert the exact
tier of every result and every control; the depth rows assert the boundary message at 1025 and
the accepted 1024 control; the docs rows assert the reference's two properties.

## 5. Gates run

All commands from `C:\Users\ricar\dev\LovelaceSharp`; `CARGO_TARGET_DIR` was
`rust/target/r93` for the Rust runs (the default target was in use by another lane) and the
release runner under `rust/target/release/` was rebuilt from this tree.

| Gate | Command | Result |
|---|---|---|
| Lovelace-lang suite | `cargo test -p lovelace-lang` | **1,684 passed, 0 failed** |
| Lovelace-real suite | `cargo test -p lovelace-real` | **2,485 passed, 0 failed** |
| Lovelace-run suite | `cargo test -p lovelace-run` | **419 passed, 0 failed** + **1 passed** integration |
| Corpus | `cargo run -p lovelace-corpus --release -- --verbose` | **214 passed, 0 failed, 214 total** |
| Fixtures | `node parity/tools/gate-fixtures.mjs` | oracle **20/20**, rust **20/20**, PASS |
| CLI | `node parity/tools/gate-cli.mjs` | **16/16**, PASS |
| Differential | `node parity/diff/run.mjs --cases 200 --seed 1 --out rust/target/tmp/r93/diff-200 --no-minimize --concurrency 10` | generated **200 match / 0 mismatch / 0 error**; replayed **1,758 match / 0 mismatch / 0 error**; discovered 1,873, 115 duplicates skipped, 1,958 executions; `stillMismatching=0`, `new=0`; harness exit **0** |
| Clippy | `cargo clippy -p lovelace-lang -p lovelace-run -p lovelace-real --all-targets -- -D warnings` | clean |
| Format | `cargo fmt -p lovelace-lang -p lovelace-run -p lovelace-real -- --check` | clean |

The differential run's oracle sha256 was `04eba2dee0f86d5f39b8103ea69db3f6bc4b68f2e4db92d84e541f6379f90921`;
the Rust runner's was `7d9306516c345219858d63e518048ff60f6d2bd02232d4fc7aa24b66f6f295ca`.

## 6. Workspace test blocker (not ours)

The brief said the workspace suite could not COMPILE because of
`rust/lovelace-desktop/src/ui/app.rs:3130` calling a 5-argument function with 3 arguments. That
call is no longer present (line 3130 is now a doc comment), but the workspace is still red in the
same crate from another lane's in-progress edits. `cargo test --workspace --no-run` ends with:

```text
error[E0599]: no method named `canonical` found for reference `&lovelace_lang::Value` in the current scope
   --> lovelace-desktop\src\engine\inspection.rs:356:43
error[E0308]: mismatched types
   --> lovelace-desktop\src\inspector.rs:385:5
    |     expected `Section<MathIr>`, found `Option<Section<MathIr>>`
error[E0063]: missing field `result_inspection` in initializer of `workbench::DocumentView`
    --> lovelace-desktop\src\ui\app.rs:3098:9
error: could not compile `lovelace-desktop` (bin "lovelace-desktop" test) due to 3 previous errors
```

A concurrently running workspace attempt also reported `result_inspection` missing from
`RunOutput` initializers, so the desktop lane was still moving while this round ran. No
`lovelace-desktop` file was read for edit, touched, or reverted; the three crates in scope are
green as listed above. The workspace gate should be re-run once the desktop lane settles.

## 7. Scope, conflicts and reproducibility

Changed files:

- `rust/lovelace-lang/src/print_budget.rs` — new production module (kernel `Printing.Print`).
- `rust/lovelace-lang/src/real_literal_route.rs` — new production module (route-carrying literal).
- `rust/lovelace-lang/src/docs_sync.rs` — new production checker + 1 unit test.
- `rust/lovelace-lang/src/round93_eval_print.rs` — new, the 12 row tests.
- `rust/lovelace-lang/src/lib.rs` — module declarations, `check_value_depth`,
  `Value::try_display`/`try_typed`, the eval check routed through `check_value_depth`, and the
  test-module declaration.
- `rust/lovelace-lang/src/structured.rs` — `try_project_value`.
- `rust/lovelace-lang/src/builtins.rs` — `evalir_digits` integral-Real acceptance.
- `rust/lovelace-lang/src/numeric.rs` — `rational_to_published` route rule.
- `docs/goal-parity-contract/round-93/eval-print-families.md` — this report.

Conflict note: `builtins.rs`, `lib.rs`, `round19_batch.rs`, `registry.rs` and `help.rs` already
carried another lane's uncommitted edits when this round started. Every edit above is additive or
a targeted replacement inside the named function; the pre-existing hunks (cancellable kernels,
Unicode output, plot capture, registry suggestions, round-92 modules) were re-read before editing
and are still present. No file was overwritten wholesale. `parity/manifest.toml` was read only;
its 12 rows are still `deferred` because this round's scope forbids editing `parity/`. The parent
can map them to the tests in §1.

Probe scripts: `rust/target/tmp/r93/*.ls`; differential artifacts:
`rust/target/tmp/r93/diff-200/`.
