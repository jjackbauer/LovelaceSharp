# Final acceptance — Rust parity contract (v2.0 standard)

Date: 2026-11-12 · HEAD: 61919a7 · Scorecard: **9/9 PASS, 0 FAILED, 0 SKIPPED**

## Definition of done, measured

| Anchor | Command | Observable | State |
|---|---|---|---|
| SHA-pinned oracle | `parity/build-oracle.ps1 -Verify` | dll sha256 `383527d8…`, dirty tree refused | **MET** (D1) |
| Census of every C# test | `parity/tools/gate-census` | 2,105 methods / **6,209 cases** / 15 projects | **MET** (D2) |
| Manifest maps every row | `node parity/tools/gate-manifest.mjs` | **6,209 dispositioned**: 6,090 ported + 119 excluded, **0 deferred** | **MET** (D3) |
| Requirement ledger | `gate-requirements` | 262 IDs accounted: 48 met / 28 partial / 186 unmet | **MET as a ledger** (see gap) |
| Project inventory | `gate-projects` | 19/19 dispositioned | **MET** (D3) |
| 5,000-case differential | `parity/diff/run.mjs --cases 5000` | harness exit 0; generated 200/200, replay 1,758/1,758 at seed 1 | **MET** (D4) |
| 20 golden fixtures | `gate-fixtures` | oracle 20/20, rust 20/20 | **MET** (D5) |
| 14 CLI requirements | `gate-cli` | every RUN-001..RUN-014 row | **MET** (D6) |
| Corpus | `lovelace-corpus` | **214/214** | **MET** (D9) |
| Mutation gate | `parity/mutate.ps1` | **12/12 detected, 12 reverted trees pass** | **MET** (D7) |
| fmt + clippy | `cargo fmt --check`, `clippy -D warnings` | exit 0 / exit 0 | **MET** (D10) |

## What the coverage number does and does not mean

- **Mapped** = 6,090 / 6,209 (98.08 %): a Rust test exists, is named, and the gate resolves it against a measured inventory.
- **Excluded** = 119, every one with `owner = "user"`, a verbatim reason, and reversibility (118 absent subsystems, 1 empty C# placeholder whose body is `{ }`).
- **Executed / passed** = 5,667 tests in the workspace suite, 0 failed, 1 ignored (the `MathIR` 1-ulp probe).
- **Equivalent** is *not* proven by mapping. The evidence for equivalence is the differential (200/200 generated, 1,758/1,758 replay), the 214-example corpus, and the 20 protocol fixtures - not the row count.
- **Semantic gap**: the requirement ledger still reports **186 unmet, 28 partial** of 262 IDs. Those are behaviours with no test-derived evidence yet; mapping does not close them.

## Fixed during this phase (selected, all with before/after measurements)

- Six SymPy-oracle defects surfaced only after installing SymPy 1.14.0: `diff(tan(u)) → 0`, `limit tan(x)/x → Unevaluated`, `x^3-2x` at ±inf → `DoesNotExist`, unfactored `x^3+1`/`x^4-1`, missing `SymbolicMatrix::trace`, complex radicals in a real-domain `SolutionSet`.
- Performance: `2^-100000` 825 s → ~0.5 s; `cos(0.5)` at 1,000 places 30.2 s → **3.65 s, faster than the C# oracle's 15.8 s**, value byte-identical.
- `plot` SVG 326 bytes → **byte-identical to the oracle** (14,636 bytes) once the real renderer was wired into `builtins_plot`.
- Nine ported-but-inequivalent rows (VAL-004) re-opened and repaired: canonical `(and/not/or)`, `compile → Text`, `assume` envelope, `collect` Add flattening, `evalir_batch` payloads, plot.
- Fail-closed defects fixed: stale mutation anchors failed closed, never counted as detected; a stale inventory is refused rather than resolved.

## Known deviations from the reference (recorded, not emulated)

1. `zeros(i64::MAX, 0)` display: the C# formatter aborts (exit -532462766); Rust returns a bounded display instead. Reference defect, not a parity gap.
2. `integrate(x^2*sin(x^3))`: the C# **library** closes it (the row's oracle); the C# **CLI** leaves it unevaluated. Rust matches the library.
3. Plot coordinates that overflow f64: the reference prints `NaN`; Rust prints `0` (deliberate hardening).
4. `MathIR` vs numeric evaluation differ by 1 ulp at 50 places (recorded as the single ignored probe).

## Residual risks

- **Three goal sessions share this checkout.** The desktop crate repeatedly broke the workspace build and the lint gate; each time parity lanes refused to gate a broken tree and waited. Recurrence is likely while that goal is active.
- **Mapping is not equivalence.** 98.08 % is a ledger figure; the semantic gap is the ledger's 186 unmet IDs, and closing them is the real remaining work.
- **Toolchain**: `rustfmt`/`clippy` were installed mid-cycle; a machine without network would have to install them before D10 can pass.

## Verdict

The **parity contract** is complete and reproducible: every anchor above is met by a command that can be re-run, the scorecard is 9/9 on one serial run, and the manifest has zero undecided rows. The **migration** is not proven complete: 186 requirement IDs remain unmet and 28 partial, and that - not 98.08 % - is the honest measure of functional equivalence.