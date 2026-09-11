# Deliverables — cycle-4

> An `ART` row needs ≥3 supporting IDs and a `Verified by` command. Fewer than that is a TODO, not a
> deliverable.

### ART-001: Safety net for the uncommitted Cycle-3 tree

- **Path**: git tag `cycle-3-safety-net` → commit `8539cf4e46aa760cbd574664a6a152349f64b3ba`
- **Type**: dataset
- **Supporting evidence**: EVD-149, EVD-150, DEC-001
- **Confidence**: High
- **Verified by**: `git status --porcelain | Measure-Object` = 78 both before and after creation;
  `git diff cycle-3-safety-net --stat` shows no tracked-file difference
- **Last updated**: 2026-09-11

### ART-002: Cycle-4 report

- **Path**: `docs/symbolics/a-plus-cycle-4-report.md`
- **Type**: report
- **Supporting evidence**: EVD-148…EVD-196, EVD-191 (final measurements), EVD-196 (the falsified claim), VAL-006
- **Confidence**: High
- **Verified by**: every measurement in it was produced by a command the orchestrator ran; the two P0
  audit findings it reports were reproduced by hand against the published binary
- **Last updated**: 2026-09-11

### ART-003: Alignment amendment (section N)

- **Path**: `docs/symbolics/a-plus-convergence-alignment-plan.md` §N
- **Type**: plan
- **Supporting evidence**: DEC-007, DEC-008, EVD-196, the maintainer's §4 decisions
- **Confidence**: High
- **Verified by**: the amendment names the four residual bounds it does **not** close; §N.3 was checked
  against the probes that established them
- **Last updated**: 2026-09-11

### ART-004: SymPy differential oracle, executing

- **Path**: `Lovelace.Symbolics.Tests/{SympyOracle,OracleCorpus,DifferentialOracleTests}.cs`
- **Type**: test-suite
- **Supporting evidence**: EVD-160, EVD-161, EVD-176, EVD-159
- **Confidence**: High
- **Verified by**: `LOVELACE_REQUIRE_SYMPY=1` with python3 3.12.14 + sympy 1.14.0 on PATH →
  `Failed 0 / Passed 6 / Skipped 0`; also 819/0 with 0 skips for the whole Symbolics project
- **Last updated**: 2026-09-11

### ART-005: Cycle-4 adversarial audit

- **Path**: `docs/goal-cycle-4/round-09/audit-{P1-numeric,P2P6-wire,P4-symbolic}.md`
- **Type**: dataset
- **Supporting evidence**: EVD-193, EVD-194, EVD-195, EVD-196, VAL-005, VAL-006
- **Confidence**: High
- **Verified by**: 332 probe rows across three independent falsifiers; the orchestrator reproduced
  every finding it allowed into the report, and re-ran the P0 claims against the pre-Cycle-4 tree to
  establish that they are inherited rather than regressions
- **Last updated**: 2026-09-11

### ART-006: ShortRun benchmark error bars (§4.4)

- **Path**: `benchmarks/symbench-shortrun-errorbars.md` + `benchmarks/bdn-reports-shortrun-c4/run1/`
- **Type**: benchmark
- **Supporting evidence**: the 12:42 sweep's per-class reports; the artifact's own non-idle caveat
- **Confidence**: Medium — the numbers are real but the machine was not idle, so they bound direction,
  not performance
- **Verified by**: the `Error`/`StdDev` columns are BenchmarkDotNet's own, copied verbatim from the
  sweep's reports rather than recomputed
- **Last updated**: 2026-09-11
