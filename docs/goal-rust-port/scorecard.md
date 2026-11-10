# Parity scorecard — Rust port

Measured 2026-11-09 on this machine, from the working tree, by the orchestrator.

## Gate summary

| Dimension | Result | Gate | Status |
|---|---|---|---|
| Workspace build (debug) | 8 crates, exit 0 | must compile | PASS |
| Unit tests | **257 passed, 0 failed** | must pass | PASS |
| Corpus, whole | **212 / 214** (99.1 %) | 100 % | **FAIL** |
| Corpus, language document | **111 / 111** (100 %) | 100 % | **FAIL** (11 symbolic) |
| Corpus, symbolics document | **101 / 103** (98.1 %) | 100 % | **FAIL** (not ported) |
| CLI text mode | `1/3` -> `0.(3) (Real)` | documented | PASS |
| Wire envelope | one JSON document, protocolVersion 1, exit 0/1/2 | documented | PASS |
| Desktop selftest | 15 / 15 checks | all | PASS |
| Desktop window | launched, found by title (`Lovelace Desktop - Untitled 1`), closed cleanly, exit 0 | must exist | PASS |
| No TCP listener | desktop selftest check `no-network-imports` | mandatory | PASS |
| C# tree untouched | 0 modified C# files | freeze protocol | PASS |
| Release build | `cargo build --workspace --release` exit 0, 18.7 s | must compile | PASS |

## What the 245 tests cover

| Crate | Tests | Coverage |
|---|---|---|
| lovelace-nat | 44 | base-10^9 limbs, add/sub/mul/div_rem/pow/factorial, 100! exact, parse/format |
| lovelace-int | 52 | sign-magnitude, truncating div_rem, i64::MIN, no negative zero |
| lovelace-real | 20 | period detection (1/3, 1/6, 1/7), exact arithmetic, sqrt, pi(5)/pi(50) literals |
| lovelace-array | 50 | shape/strides, indexing, slicing, reshape/transpose/concat, reductions, errors |
| lovelace-json | 7 | parsing, escapes, surrogate pairs, non-ASCII escaping |
| lovelace-lang | 8 | literals, division, widening, ranges, recursion, errors, blocks |
| lovelace-desktop | 67 |
| lovelace-symbolics | 26 | document/tab model, settings, cancel, undo, layout, log cap, SVG parse |

## The gap, stated exactly

111 language examples: 100 pass, 11 fail — every failure is a symbolic example
(`lang/100`-`lang/111`), because the symbolic kernel is not ported.

103 symbolics-document examples: 4 pass (the ones that are plain arithmetic), 99 fail, all for the
same reason.

The symbolic kernel is the one subsystem deliberately left out: 13 980 lines of C#, 137 public
types, 59 builtins, hash-consing, assumption sets, a rewrite engine with proofs, solvers and
Gröbner bases. A half-built version would answer symbolic questions wrongly, which is worse than
answering "Unknown function 'symbol'." — which is what the engine does today, and which RISK-002
of the requirements set as the required failure mode for an absent builtin.

## Release artefacts

| Binary | Size | Verified behaviour |
|---|---|---|
| `lovelace-run.exe` | 0.7 MB | `2^100` and `pi(50)` render exactly; envelope exit codes 0/1/2 |
| `lovelace-corpus.exe` | 0.7 MB | 134/214 corpus, same result in release |
| `lovelace-desktop.exe` | 0.8 MB | real window verified by title + clean close; `--selftest` 15/15 |

No runtime to install: the binaries are statically linked Rust.

## Reconciliation note

After the first report, the desktop agent returned with its own Win32 shell (src/win32/*) and
67 tests, replacing the shell the orchestrator had written during the agent's silence. The
orchestrator re-verified the resulting tree from scratch: release build, 248 tests, corpus, and a
live launch of the window (handle found, title read, clean exit 0). The orchestrator's now-orphaned
src/shell.rs was deleted. Every number above is from that re-verification.

## Round 2 update

Symbolic kernel started: `lovelace-symbolics` implements canonical expressions, the documented term and factor ordering, printing, differentiation, expansion and collection. Corpus 104 -> 134 (lang 102/111, sym 32/103). Remaining: the algebra (factor, cancel, apart), solving, series, limits, jacobian, and the `*_full` structured-result records.

## Round 3 update

Solving, factorisation, Jacobians and basic integration landed. `solve` returns the exact SolveResult record (status, variable, domain, completeness, Solution records, counts, diagnostics), verified end to end through the CLI. Corpus 134 -> 154; the language document is 109/111, missing only `series` and `limit`.

## Round 4 update

The language document is COMPLETE: 111/111. A series engine (truncated Laurent series with exact rational coefficients, including reciprocal series for division) now powers `series` and `limit`, which produce the documented `O(x^n)` expansions and the full `LimitResult` record including one-sided values, conditions and the Unevaluated diagnostic. Also added: hessian, subs. Corpus 154 -> 167.

## Round 5 update

Symbolic domain model and record families: `Domain` values (real/complex) with their typed rendering, the infinity constant as a symbolic value (`inf`, `-inf`), limits at infinity via the t -> 0 substitution, `Partial` solving outcomes, `inspect`, `and`/`or`/`not`, `integrate_full` and `compile_full` records, real-domain no-solution answers. Corpus 167 -> 183 (85.5 %); the language document stays at 111/111.

## Round 6 update

Assumption theory: `assume` now records interval bounds, `simplify` decides relations against them, and a contradictory assumption raises the documented error with the negation it proves. Also `cancel` (exact univariate polynomial division) and a corrected coefficient extractor that no longer reads `exp(x)` as a polynomial of degree one. Corpus 183 -> 188.

## Round 7 update

Structured results and quotient solving: `solve` now cancels a quotient's numerator, carries the denominator's non-vanishing condition into the record, and drops roots that would null it; `simplify_full` reports expression/conditions/steps with the documented rule id; `assume(and(...))` records each conjunct in order and `not` lifts over relations. Corpus 188 -> 195 (91.1 %).

## Round 8 update

`collect` publishes the constant first then descending powers; quotients parenthesise a compound denominator; `evalir`/`evalir_batch` evaluate a compiled polynomial at points through exact Real arithmetic (`evalir_batch(compile(x^2+1,[x]), [1,2,3], 40)` -> `[2, 5, 10]`). Corpus 195 -> 197. A proposed global constant-first ordering rule was tried and reverted: it matched two examples and broke five.

## Round 9 update

Complex algebraic roots: a cubic with a negative quadratic remainder now returns `1/2*(i*sqrt(3) - 1)` and its conjugate with `exactness: AlgebraicExact` alongside the exact rational root, matching the documented ordering (complex first, exact last). Corpus 197 -> 198.

## Round 10 update

Partial fractions: `apart` decomposes a proper rational function with simple rational poles (residue = p(r)/q'(r)), and `integrate` falls back to integrating that decomposition term by term (A/(x-r) -> A*log(x-r)). `matrix_rank` reports the generic rank, falling back to n-1 only when the determinant is structurally zero. A bare reciprocal now counts as a quotient. Corpus 198 -> 201.

## Round 11 update

Symbolic-coefficient solving: `solve(x + y == 0, x)` returns `-y` with the coefficient's non-vanishing condition (`1 != 0`). The real domain now refuses imaginary roots rather than reporting them as real solutions. Matrix accumulation publishes its constant term first. Corpus 201 -> 204 (95.3 %).

## Round 12 update

The symbolic layer's test suite grew from 11 to 26 tests covering solving, partial fractions, series, substitution, collection, polynomial division and the chain rule. One of those tests caught an error in the test itself (the expected derivative was wrong; the engine was right). Final state: 274 tests, 204/214 corpus, 111/111 language document, 10 crates, ~19 800 lines, zero external dependencies.

## Clean-build verification (final round)

After `cargo clean`, the workspace rebuilt in 23.7 s (release) and the suite ran in 11.5 s: 274 tests, 0 failures. From those freshly built artefacts: corpus 204/214 (language document 111/111), `1/3` -> `0.(3)`, `2^100` exact, desktop selftest 15/15, and a window that opened (`Lovelace Desktop - Untitled 1`) and closed with exit 0. Binary sizes 1.1-1.3 MB.

## Round 13 update (post-cap parity push)

Four structural gaps closed with real capabilities rather than output matching:

1. An explicit `Div` node that never collapses, so `inv` publishes `0/(x*y)` as the reference does.
2. Parametric solution families for periodic equations (`log(5) + 2*k*pi*i`, `k*pi`) with a `SolutionFamily` record.
3. Biquadratic solving through `y = x^2`, producing nested radicals and an honest unrepresented count.
4. Horner form for `optimize`, plus a real `lower` -> IR text -> `evalir` round trip that parses its own output.

Corpus 204 -> 211 (98.6 %).

## Round 14 update

Equation systems: `solve_system` splits on a product equation, substitutes each factor and solves the remainder, publishing `SystemSolveResult` with bindings ordered by the reference's sweep (last variable, then first). The MathIR emitter was re-ported from the C# source: a power emits its base, then its exponent as a constant node, then the power, with CSE by subexpression - so `lower` now emits the published format and `evalir` parses it back. Corpus 211 -> 212.
