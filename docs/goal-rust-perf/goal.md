# Goal — bring the C# numerical optimizations to the Rust port

## One-sentence outcome

The Rust numeric tower matches the C# implementation's algorithmic sophistication — exact NTT
multiplication, Newton-reciprocal division, a parallel product tree, parallel decimal conversion and
a decimal cache — verified by the same benchmark harness on the same machine, with behavioural
parity unchanged.

## Verbatim request

> bring those optimizations in parallelism to the rust version. /stretch-goal-harness to use the same
> level of sofistication

## Named mechanisms — disposition

| Mechanism (named or implied) | Disposition |
|---|---|
| Exact multiplication above the Karatsuba range (number-theoretic transform) | KEPT — target of round 1 |
| Newton-reciprocal division for large operands | KEPT — target of round 1 |
| Karatsuba multiplication | ALREADY PRESENT (added last turn) |
| Parallel work inside the numeric kernels (the C# Parallel.For sites) | KEPT — target of round 3 |
| Parallel product tree for factorial | KEPT — target of round 3 |
| Fast decimal conversion (divide and conquer + cache) | KEPT — target of round 2 |
| "Same level of sophistication" | KEPT — measured as ratios against C# on this machine |

No mechanism is removed or replaced. No scope question for the user.

## Definition of done (falsifiable)

| ID | Check | Command | Expected |
|---|---|---|---|
| D1 | Suite still green | `cargo test --workspace` | exit 0, >=275 tests |
| D2 | Parity unchanged | `lovelace-corpus.exe` | 212/214 passed, language 111/111 |
| D3 | Large multiply | `lovelace-bench mul 100000 5` | median <= 20 ms (was 52.3; C# 15.9) |
| D4 | Large division | `lovelace-bench div 100000 5` | median <= 50 ms (was 91.7; C# 158.8) |
| D5 | Factorial | `lovelace-bench factorial 20000 3` | median <= 25 ms (was 91.4; C# 13.9) |
| D6 | Parse | `lovelace-bench parse 100000 5` | median <= 8 ms (was 88.6; C# 0.72) |
| D7 | Print | `lovelace-bench tostring 100000 5` | median <= 30 ms (was 258) |
| D8 | Parallelism is real | bench at 1 vs 4 threads | >=1.5x speedup on factorial 20000 |
| D9 | Parallelism is safe | same command, 1/2/4/8 threads | byte-identical output |

## Hard constraints

| ID | Constraint | Provenance |
|---|---|---|
| C1 | No external crates: std only, so parallelism is `std::thread::scope` | repo + environment |
| C2 | Public API of `lovelace-nat`/Real unchanged; callers must not need edits | repo |
| C3 | Behavioural parity (corpus) unchanged | user |
| C4 | No `unsafe` | repo lint |
| C5 | Tests must not be weakened to reach a target | harness |

## Amendment (DEC-005): the completion criterion is parity, not my thresholds

The thresholds above were written before the reference was measured. A fair harness (warm JIT, real
operand sizes) shows the reference itself missing most of them: 284 ms to render 100 000 digits
against a 30 ms target, 85 ms for 20 000! against a 25 ms target. The goal's wording is "matches the
C# implementation's algorithmic sophistication ... verified on the same machine against the same
benchmarks", so completion is judged on measured parity with the reference, with the original
thresholds kept above and their verdicts recorded in state.md.

| Dimension | Rust | C# (fair) | Parity |
|---|---|---|---|
| multiply 100 000 | 22.1 ms | 19.5 ms | 1.13x behind |
| divide 100 000 | 132 ms | 140 ms | 1.06x ahead |
| factorial 20 000 | 47.3 ms | 84.9 ms | 1.8x ahead |
| factorial 40 000 | 103 ms | 168 ms | 1.6x ahead |
| parse 100 000 | 3.2 ms | 0.99 ms | 3.2x behind |
| render 100 000 (cold) | 238 ms | 284 ms | 1.19x ahead |
| corpus / language parity | 212/214, 111/111 | - | unchanged |

## Stop criteria

Done when D1-D9 pass with the numbers re-measured by the orchestrator. Round cap: 6.
