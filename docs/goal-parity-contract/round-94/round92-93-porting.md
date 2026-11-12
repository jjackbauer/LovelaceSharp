# Round 94 — the 36 round-92/93 rows, applied to the curated map

**Status: 36/36 mapped, 0 unverified, 0 blocked. Data-only: `parity/manifest-map.json` and
this report are the only files this lane wrote. No generator was run and no coverage figure
is claimed.**

Measured 2026-11-12 (America/Sao_Paulo). Inputs:

- `docs/goal-parity-contract/round-92/symbolic-small-families.md` — 12 ids
- `docs/goal-parity-contract/round-92/runner-suite-families.md` — 12 ids
- `docs/goal-parity-contract/round-93/eval-print-families.md` — 12 ids

## 1. Verification method

For each of the 36 rows:

1. read the census id in the source report's id -> test table;
2. located the C# row method in the main tree (file and method line);
3. located the named Rust test in the current tree, confirmed it is exactly one `#[test]`
   function (no `#[ignore]`) in a module declared by the crate's `lib.rs`;
4. read the test body and matched its assertions to the observable the report documents
   for that row (and to the C# row's own assertion).

All 36 resolved. The Rust source was not edited. The source reports record the isolated
package runs (12/12, 12/12, 12/12); this step did not re-run `cargo test` because the
workspace gate is blocked by the desktop lane's mid-refactor and a data-only map edit does
not need an execution pass. No test result is claimed here beyond what those reports
measured.

## 2. Map edit and diff evidence

```text
keys before / after      : 6209 / 6209  (order identical)
changed keys             : 36
added / removed keys     : 0 / 0
dispositions before      : ported 6018, deferred 73, excluded 118
dispositions after       : ported 6054, deferred 37, excluded 118
sha256 before            : f9be900b502ac22d282aaea24448935c42049a6539166e13937a90d2d889490b
sha256 after             : b6daade03ff463fb08636cf7273abf6c52a26a2d3fd152ac6fb42363e09a91fc
git diff --numstat       : 144 insertions, 108 deletions
```

The numstat is exactly 36 replacements: each deferred entry had `disposition`/`reason`/`note`
(3 field lines) and each ported entry has `disposition`/`rust`/`note`/`reason` (4 field
lines), so 36 x 3 = 108 deletions and 36 x 4 = 144 insertions. The entry's key line, braces
and every other entry are unchanged.

Each of the 36 entries now carries:

- `disposition = "ported"`;
- the `rust` test id that exists in the current tree;
- a `note` naming the C# row and the Rust test and restating the observable;
- a `reason` naming this porting round and this report.

## 3. The 36 mappings

| # | Census id | Rust test | C# row |
|---:|---|---|---|
| 1 | `Lovelace.Complex.Tests/Lovelace.Complex.Tests.ComplexMathInexactArgumentProvenanceTests/SymbolicNodeAndNumericResult_AgreeForSine#0` | `rust/lovelace-lang/src/round92_symbolic_families.rs:87` `complexmathinexactargumentprovenancetests_symbolicnodeandnumericresult_agree_for_sine` | `Lovelace.Complex.Tests/ComplexMathInexactArgumentProvenanceTests.cs:188 SymbolicNodeAndNumericResult_AgreeForSine` |
| 2 | `Lovelace.Complex.Tests/Lovelace.Complex.Tests.ComplexMathInexactArgumentProvenanceTests/SymbolicNodeAndNumericResult_AgreeForTheSameExpression#0` | `rust/lovelace-lang/src/round92_symbolic_families.rs:50` `complexmathinexactargumentprovenancetests_symbolicnodeandnumericresult_agree_for_the_same_expression` | `Lovelace.Complex.Tests/ComplexMathInexactArgumentProvenanceTests.cs:161 SymbolicNodeAndNumericResult_AgreeForTheSameExpression` |
| 3 | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.RecursionBudgetCompositionTests/TheDeepestAcceptedCall_Answers_InEveryComposition#14` | `rust/lovelace-lang/src/round92_symbolic_families.rs:240` `recursionbudgetcompositiontests_thedeepestacceptedcall_answers_in_a_for_body` | `Lovelace.Suite.Tests/RecursionBudgetCompositionTests.cs:101 TheDeepestAcceptedCall_Answers_InEveryComposition (row 14)` |
| 4 | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.RecursionBudgetCompositionTests/TheDeepestAcceptedCall_Answers_InEveryComposition#15` | `rust/lovelace-lang/src/round92_symbolic_families.rs:254` `recursionbudgetcompositiontests_thedeepestacceptedcall_answers_in_a_while_body` | `Lovelace.Suite.Tests/RecursionBudgetCompositionTests.cs:101 TheDeepestAcceptedCall_Answers_InEveryComposition (row 15)` |
| 5 | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.DxSemanticClosureTests/DomainOf_NegativeIntegerPower_IsRationalNotInteger#0` | `rust/lovelace-symbolics/src/round92_symbolic_families.rs:28` `dxsemanticclosuretests_domainof_negativeintegerpower_isrationalnotinteger` | `Lovelace.Symbolics.Tests/DxSemanticClosureTests.cs:286 DomainOf_NegativeIntegerPower_IsRationalNotInteger` |
| 6 | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.DxSemanticClosureTests/Solve_QuarticNoRealRoots_DefaultDomain_ReportsUnevaluated#0` | `rust/lovelace-symbolics/src/round92_symbolic_families.rs:56` `dxsemanticclosuretests_solve_quartic_no_real_roots_default_domain_reports_unevaluated` | `Lovelace.Symbolics.Tests/DxSemanticClosureTests.cs:179 Solve_QuarticNoRealRoots_DefaultDomain_ReportsUnevaluated` |
| 7 | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.LinsolveCheckTests/Solve_3x3_VerifiesByNumericSubstitution#0` | `rust/lovelace-lang/src/round92_symbolic_families.rs:184` `linsolvechecktests_solve3x3_verifiesbynumericsubstitution` | `Lovelace.Symbolics.Tests/LinsolveCheckTests.cs:40 Solve_3x3_VerifiesByNumericSubstitution` |
| 8 | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.LinsolveCheckTests/Solve_VerifiesByNumericSubstitution#0` | `rust/lovelace-lang/src/round92_symbolic_families.rs:150` `linsolvechecktests_solve_verifiesbynumericsubstitution` | `Lovelace.Symbolics.Tests/LinsolveCheckTests.cs:11 Solve_VerifiesByNumericSubstitution` |
| 9 | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.SolveCompletenessMappingTests/BudgetExceeded_IsIncomplete_WithTheKernelCompletenessOfThePartialSubset#0` | `rust/lovelace-symbolics/src/round92_symbolic_families.rs:84` `solvecompletenessmappingtests_budgetexceeded_is_incomplete_with_the_kernel_completeness` | `Lovelace.Symbolics.Tests/SolveCompletenessMappingTests.cs:47 BudgetExceeded_IsIncomplete_WithTheKernelCompletenessOfThePartialSubset` |
| 10 | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.SolveCompletenessMappingTests/EveryStatusCarriesTheDocumentedPair#3` | `rust/lovelace-symbolics/src/round92_symbolic_families.rs:102` `solvecompletenessmappingtests_everystatuscarriesthedocumentedpair` | `Lovelace.Symbolics.Tests/SolveCompletenessMappingTests.cs:39 EveryStatusCarriesTheDocumentedPair` |
| 11 | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.RewriteProofBridgeTests/EveryObligation_IsStatedAndHonestAboutItsStatus#0` | `rust/lovelace-symbolics/src/round92_symbolic_families.rs:152` `rewriteproofbridgetests_everyobligation_isstatedandhonestaboutitsstatus` | `Lovelace.Symbolics.Tests/RewriteProofBridgeTests.cs:26 EveryObligation_IsStatedAndHonestAboutItsStatus` |
| 12 | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.RewriteProofBridgeTests/EveryShippedRule_HasExactlyOneProofObligation#0` | `rust/lovelace-symbolics/src/round92_symbolic_families.rs:131` `rewriteproofbridgetests_everyshippedrule_hasexactlyoneproofobligation` | `Lovelace.Symbolics.Tests/RewriteProofBridgeTests.cs:14 EveryShippedRule_HasExactlyOneProofObligation` |
| 13 | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.CancellationObservationTests/Evaluate_GivenLongNaturalPowerAndShortBudget_CancelsPromptly#0` | `rust/lovelace-lang/src/round92_runner_families.rs:94` `evaluate_given_long_natural_power_and_short_budget_cancels_promptly` | `Lovelace.Suite.Tests/CancellationObservationTests.cs:28 Evaluate_GivenLongNaturalPowerAndShortBudget_CancelsPromptly` |
| 14 | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.CancellationObservationTests/Evaluate_GivenLongFactorialAndShortBudget_CancelsPromptly#0` | `rust/lovelace-lang/src/round92_runner_families.rs:100` `evaluate_given_long_factorial_and_short_budget_cancels_promptly` | `Lovelace.Suite.Tests/CancellationObservationTests.cs:52 Evaluate_GivenLongFactorialAndShortBudget_CancelsPromptly` |
| 15 | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.SuiteEngineOutputPlotTests/EvaluateAsync_GivenOutputWriter_CapturesPrintLinesToThatWriter#0` | `rust/lovelace-lang/src/round92_runner_families.rs:111` `evaluate_async_given_output_writer_captures_print_lines_to_that_writer` | `Lovelace.Suite.Tests/SuiteEngineOutputPlotTests.cs:8 EvaluateAsync_GivenOutputWriter_CapturesPrintLinesToThatWriter` |
| 16 | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.SuiteEngineOutputPlotTests/ResetPlotCapture_GivenPreviousPlot_ClearsLastPlot#0` | `rust/lovelace-lang/src/round92_runner_families.rs:126` `reset_plot_capture_given_previous_plot_clears_last_plot` | `Lovelace.Suite.Tests/SuiteEngineOutputPlotTests.cs:45 ResetPlotCapture_GivenPreviousPlot_ClearsLastPlot` |
| 17 | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.HelpServiceTests/Help_Overview_ListsAllCategories#0` | `rust/lovelace-lang/src/round92_runner_families.rs:147` `help_overview_lists_all_categories` | `Lovelace.Suite.Tests/HelpServiceTests.cs:26 Help_Overview_ListsAllCategories` |
| 18 | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.HelpServiceTests/UnicodeOutput_ThreadsThroughEngineFormatting#0` | `rust/lovelace-lang/src/round92_runner_families.rs:163` `unicode_output_threads_through_engine_formatting` | `Lovelace.Suite.Tests/HelpServiceTests.cs:103 UnicodeOutput_ThreadsThroughEngineFormatting` |
| 19 | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.NameSuggestionsTests/ANameNothingIsCloseTo_IsNotGuessedAt#0` | `rust/lovelace-lang/src/round92_runner_families.rs:202` `a_name_nothing_is_close_to_is_not_guessed_at` | `Lovelace.Suite.Tests/NameSuggestionsTests.cs:42 ANameNothingIsCloseTo_IsNotGuessedAt` |
| 20 | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.NameSuggestionsTests/EditDistance_IsTheStructuralMetric#3` | `rust/lovelace-lang/src/round92_runner_families.rs:213` `edit_distance_is_the_structural_metric` | `Lovelace.Suite.Tests/NameSuggestionsTests.cs:92 EditDistance_IsTheStructuralMetric` |
| 21 | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.CapabilitiesBuiltinTests/AdvertisedCodesAndCategories_MatchTheLiveEnvelope#0` | `rust/lovelace-lang/src/round92_runner_families.rs:263` `advertised_codes_and_categories_match_the_live_envelope` | `Lovelace.Symbolics.Tests/CapabilitiesBuiltinTests.cs:368 AdvertisedCodesAndCategories_MatchTheLiveEnvelope` |
| 22 | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.CapabilitiesBuiltinTests/PowerExponentClass_RefusesWhatItSays_AndNamesWhatIsSupported#0` | `rust/lovelace-lang/src/round92_runner_families.rs:332` `power_exponent_class_refuses_what_it_says_and_names_what_is_supported` | `Lovelace.Symbolics.Tests/CapabilitiesBuiltinTests.cs:494 PowerExponentClass_RefusesWhatItSays_AndNamesWhatIsSupported` |
| 23 | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.DiagnosticContractTests/ErrorCategory_HasExactlyTheSevenFrozenMembers#0` | `rust/lovelace-lang/src/round92_runner_families.rs:400` `error_category_has_exactly_the_seven_frozen_members` | `Lovelace.Symbolics.Tests/DiagnosticContractTests.cs:52 ErrorCategory_HasExactlyTheSevenFrozenMembers` |
| 24 | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.DiagnosticContractTests/KernelNotes_CrossAsDiagnosticMessages#0` | `rust/lovelace-lang/src/round92_runner_families.rs:422` `kernel_notes_cross_as_diagnostic_messages` | `Lovelace.Symbolics.Tests/DiagnosticContractTests.cs:209 KernelNotes_CrossAsDiagnosticMessages` |
| 25 | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.EvalirDigitArgumentContractTests/AnExactlyIntegralRealLiteral_IsTheIntegerItDenotes#0` | `rust/lovelace-lang/src/round93_eval_print.rs:88` `evalirdigitargumentcontracttests_an_exactly_integral_real_literal_is_the_integer_it_denotes_0` | `Lovelace.Symbolics.Tests/EvalirDigitArgumentContractTests.cs:125 AnExactlyIntegralRealLiteral_IsTheIntegerItDenotes (evalir)` |
| 26 | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.EvalirDigitArgumentContractTests/AnExactlyIntegralRealLiteral_IsTheIntegerItDenotes#1` | `rust/lovelace-lang/src/round93_eval_print.rs:94` `evalirdigitargumentcontracttests_an_exactly_integral_real_literal_is_the_integer_it_denotes_1` | `Lovelace.Symbolics.Tests/EvalirDigitArgumentContractTests.cs:125 AnExactlyIntegralRealLiteral_IsTheIntegerItDenotes (evalir_batch)` |
| 27 | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.PrintBudgetTests/DepthBudget_ReportsDepthLimit#0` | `rust/lovelace-lang/src/round93_eval_print.rs:113` `printbudgettests_depth_budget_reports_depth_limit` | `Lovelace.Symbolics.Tests/PrintBudgetTests.cs:73 DepthBudget_ReportsDepthLimit` |
| 28 | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.PrintBudgetTests/NodeBudget_OnACompactRendering_DropsNothingAndSaysSo#0` | `rust/lovelace-lang/src/round93_eval_print.rs:146` `printbudgettests_node_budget_on_a_compact_rendering_drops_nothing_and_says_so` | `Lovelace.Symbolics.Tests/PrintBudgetTests.cs:61 NodeBudget_OnACompactRendering_DropsNothingAndSaysSo` |
| 29 | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.PrintBudgetTokenBoundaryTests/AnIdentifierLongerThanTheAllowance_IsKeptWhole#0` | `rust/lovelace-lang/src/round93_eval_print.rs:211` `printbudgettokenboundarytests_an_identifier_longer_than_the_allowance_is_kept_whole` | `Lovelace.Symbolics.Tests/PrintBudgetTokenBoundaryTests.cs:69 AnIdentifierLongerThanTheAllowance_IsKeptWhole` |
| 30 | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.PrintBudgetTokenBoundaryTests/ARenderingWithinTheAllowance_IsUntouched#0` | `rust/lovelace-lang/src/round93_eval_print.rs:247` `printbudgettokenboundarytests_a_rendering_within_the_allowance_is_untouched` | `Lovelace.Symbolics.Tests/PrintBudgetTokenBoundaryTests.cs:105 ARenderingWithinTheAllowance_IsUntouched` |
| 31 | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.TruncatingEvaluationStaysInexactTests/FunctionOfATruncatedArgument_IsInexact#0` | `rust/lovelace-lang/src/round93_eval_print.rs:303` `truncatingevaluationstaysinexacttests_function_of_a_truncated_argument_is_inexact` | `Lovelace.Symbolics.Tests/TruncatingEvaluationStaysInexactTests.cs:273 FunctionOfATruncatedArgument_IsInexact` |
| 32 | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.TruncatingEvaluationStaysInexactTests/LiteralRoundTrip_KeepsGenuinelyExactValuesExact#0` | `rust/lovelace-lang/src/round93_eval_print.rs:337` `truncatingevaluationstaysinexacttests_literal_round_trip_keeps_genuinely_exact_values_exact` | `Lovelace.Symbolics.Tests/TruncatingEvaluationStaysInexactTests.cs:239 LiteralRoundTrip_KeepsGenuinelyExactValuesExact` |
| 33 | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.ValueDepthGuardTests/ADeepRecordChain_IsRefusedTheSameWay#0` | `rust/lovelace-lang/src/round93_eval_print.rs:446` `valuedepthguardtests_a_deep_record_chain_is_refused_the_same_way` | `Lovelace.Suite.Tests/ValueDepthGuardTests.cs:151 ADeepRecordChain_IsRefusedTheSameWay` |
| 34 | `Lovelace.Suite.Tests/Lovelace.Suite.Tests.ValueDepthGuardTests/ValueOnePastTheBudget_IsRefusedByTheStructuredProjection#0` | `rust/lovelace-lang/src/round93_eval_print.rs:411` `valuedepthguardtests_value_one_past_the_budget_is_refused_by_the_structured_projection` | `Lovelace.Suite.Tests/ValueDepthGuardTests.cs:126 ValueOnePastTheBudget_IsRefusedByTheStructuredProjection` |
| 35 | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.DocsSyncTests/Every_CSharpSnippet_AppearsInUsageExamples#0` | `rust/lovelace-lang/src/round93_eval_print.rs:509` `docssynctests_every_c_sharp_snippet_appears_in_usage_examples` | `Lovelace.Symbolics.Tests/DocsSyncTests.cs:16 Every_CSharpSnippet_AppearsInUsageExamples` |
| 36 | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.DocsSyncTests/Every_LovelaceExample_HasAnExpectedResult#0` | `rust/lovelace-lang/src/round93_eval_print.rs:521` `docssynctests_every_lovelace_example_has_an_expected_result` | `Lovelace.Symbolics.Tests/DocsSyncTests.cs:35 Every_LovelaceExample_HasAnExpectedResult` |

The `rust` field values are the crate/module/function ids; the file:line is the current
source location of the test function.

## 4. Unverified ids

None. 36/36 C# rows and Rust tests were found and read; every Rust test's assertions match
the row's documented observable. No key was left deferred.

## 5. Scope and untouched artifacts

- Edited: `parity/manifest-map.json`; added: this report.
- Not run: `parity/gen-rust-tests.ps1`, `parity/gen-manifest.mjs`,
  `parity/tools/coverage-report.mjs`.
- Not touched: `parity/manifest.toml`, `parity/rust-tests.json`, `parity/coverage.md`,
  any `.rs`/`.cs`/fixture, or any other document.
- No coverage figure is claimed. `parity/rust-tests.json` is expected to be stale until the
  parent regenerates it; the map's new `rust` ids should resolve then. The generated
  manifest and coverage report are the parent's quiet-window step.

The workspace `git status` also shows other lanes' edits (desktop refactor, language/runner
work). This lane did not touch them.
