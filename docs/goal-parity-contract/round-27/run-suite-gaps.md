# Round 27 — Run/Suite production gaps

Scope: `rust/lovelace-lang/src` (builtins, interp, lib, structured, two new test modules) and
`rust/lovelace-run/src` (main, parity_batch, one new test module). No `.cs`, fixture, `parity/`
or other doc was changed. Oracle for every probe: `dotnet exec Lovelace.Run/bin/Release/net10.0/
Lovelace.Run.dll --eval '<script>' --omit-functions --omit-variables` (pinned build); Rust: the
release `lovelace-run`. Volatile durations are omitted from the comparisons where they are not
the subject.

## 1. What changed

| # | Gap | Where | Change |
|---|---|---|---|
| 1 | `evalir`/`evalir_batch` ignored the digit count | `builtins.rs` | The third argument is read through the exact-integer contract (bounds [1, int.MaxValue], the reference's four refusal classes), the answer is computed at that count and truncated to it when the quotient does not terminate; the declared 3-argument arity is enforced. |
| 2 | `zeros(-1)`/`ones(-1)`/`series(...,-1)` | — | **Already conformant before this round** (measured; the round-19 refusal row was stale). Pinned by new tests; no production change. |
| 3 | Wide-index taxonomy | `builtins.rs` `index_int`/`index_argument`, `interp.rs` | Every index goes through `ExactIntegerArgument` semantics: a Real is refused by kind (never floored), an integer wider than Int64 is refused by name, and a rank-≥2 subscript out of range names the DIMENSION and its size. |
| 4 | `solve` ignored `assume(...)` | `lib.rs` Engine store, `builtins.rs` assume/solve/solve_system | The session store now also holds the atoms as expressions and the domain assumptions. Roots are judged against them (refuted roots dropped, an all-refuted set becomes the no-note NoSolutions record, undecidable atoms ride on the family conditions); `assume_real`/`assume_integer`/`assume_negative` are dispatcher builtins that answer the symbol; `solve_system` filters assignments through the same store and gained a triangular (root-enumerating) fallback so `[x == 1, y^2 == 4]` is solvable at all. |
| 5 | BOM / UTF-16 surfaces | `lib.rs` `eval_inner`, `main.rs` | The engine strips a leading U+FEFF (instead of refusing it) and maps every published timing/diagnostic offset back across it; `--file` decodes UTF-8/UTF-16 LE/BE/UTF-32 from the byte-order mark and KEEPS the mark, like the reference's ScriptText; `--stdin` reads bytes and keeps a UTF-8 mark. |
| 6 | evalf digit-cap notice | `builtins.rs`, `lib.rs` `Value::RealClamped`, `structured.rs` | A request above the 1000-place cap whose answer is INEXACT now publishes `truncated:true`, `truncationReason:"digit-cap"`, `budget:1000` on the result and on the `_` variable; exact answers and at-cap requests stay unmarked. |
| 7 | elapsed vs elapsedTime scale | `main.rs` | One selector (the reference's Timing.Scale: ns/µs/ms/s/min/h, two decimals, trailing zeros trimmed) drives both the human string and the structured pair; the unit is the micro sign. |
| 8 | eye/mean/norm 2-argument forms | `builtins.rs` | `eye(rows, cols)`; `mean(a, axis)` and `norm(a, axis)` with the reference's shape refusal (with signature), axis guard (rank-aware message) and per-axis reduction. |

## 2. Before/after probes (oracle vs Rust)

`before` is the measured Rust output before this round's edits (same binary path, pre-change
build); `after` is the measured Rust now. Every `after` row below equals the oracle byte for
byte except where §4 notes otherwise.

### Item 1 — evalir digit plumbing

| Probe | before (Rust) | after (Rust = oracle) |
|---|---|---|
| `evalir(compile(x*1/3,[x]), [1], 2)` | `0.(3)` (count ignored) | `0.33` |
| `... 40` | `0.(3)` | `0.3333…` (40 threes) |
| `evalir(compile(x^2+1,[x]), [3], 5/20/40)` | `10` (integer answer; count invisible) | `10` at every count (same; the integer path never used the count) |
| `evalir(..., 2.9)` | `ok:true` | `InvalidArgument/TypeMismatch`, "evalir(): argument 3 (digits) must be a Natural or Integer that denotes an exact integer; got 2.9." |
| `evalir(..., 0)` / `-5` | `ok:true` | `InvalidArgument/TypeMismatch`, "…between 1 and 2147483647; got 0." / "-5" |
| `evalir(..., 10^30)` | `ok:true` | `InvalidArgument/TypeMismatch`, "…between 1 and 2147483647; got 1000000000000000000000000000000." |
| `evalir(..., "forty")` | `ok:true` | `InvalidArgument/TypeMismatch`, "…that denotes an exact integer; got Text." |
| `evalir_batch(..., 2.5)` | `ok:true` | `InvalidArgument/TypeMismatch` with the `evalir_batch()` head |

### Item 2 — zeros/ones/series refusals (already conformant)

Measured before any change; unchanged after. Invariant: the same `InvalidArgument/TypeMismatch`
pair and the reference's sentence for every row below.

| Probe | before (Rust) | oracle |
|---|---|---|
| `zeros(-1)` | "zeros(): argument 1 (a dimension) must be a Natural or Integer that is a non-negative count (0 or more, at most 9223372036854775807); got -1." | identical |
| `zeros(2,-1)` | "…argument 2…got -1." | identical |
| `ones(-1)` / `ones(2,-1)` | same shape with the `ones()` head | identical |
| `series(sin(x),x,0,-1)` | "series(): argument 4 (order) must be a Natural or Integer between 1 and 2147483647; got -1." | identical |
| `series(...,0)` / `series(...,10^30)` | bound refusal / "denotes an exact integer; got the Natural 1000…" | identical |
| `zeros(0)` / `ones(0)` | `[]` | `[]` |

### Item 3 — wide-index taxonomy

| Probe | before (Rust) | after (Rust = oracle) |
|---|---|---|
| `[1,2,3][10^30]` | `InvalidOperation/DomainError` "Index is too large." | `InvalidArgument/TypeMismatch` "index(): argument 'index' must be a Natural or Integer that fits a signed 64-bit integer (at most 9223372036854775807 in absolute value); got 1000000000000000000000000000000." |
| `[1,2,3][1.5]` | `InvalidOperation/DomainError` "Index must be an Integer." | `InvalidArgument/TypeMismatch` "…that denotes an exact integer; got the Real 1.5." |
| `[1,2,3][1.0]` | `InvalidOperation/DomainError` "Index must be an Integer." | `InvalidArgument/TypeMismatch` "…must be a Natural or Integer; got the Real 1, which is integral — write it as 1." |
| `[1,2,3]["a"]` | `InvalidOperation/DomainError` "Index must be an Integer." | `InvalidArgument/TypeMismatch` "…that denotes an exact integer; got the Text." |
| `[[1,2],[3,4]][5,0]` | "Index 5 is out of range for vector of length 2." | "Index 5 is out of range for dimension 0 of size 2." |
| `[1,2,3][-1]` / `[5]` | unchanged | "Index -1/5 is out of range for vector of length 3." (control) |

### Item 4 — solve session assumptions

| Probe | before (Rust) | after (Rust = oracle) |
|---|---|---|
| `assume(x > 0); solve(x^2 == 4, x)` | roots `[-2, 2]` | `2` |
| `assume_positive(x); solve(x^2 == 4, x)` | roots `[-2, 2]` | `2` |
| `assume(x < 0); solve(x^2 == 4, x)` | roots `[-2, 2]` | `-2` |
| `assume(x == 2); solve(x^2 == 4, x)` | roots `[-2, 2]` | `2` |
| `assume(x > 5); solve(x == 3, x)` | `Solved, solutions [3]` | `NoSolutions`, complete `True`, solutions `[]`, diagnostics `[]` |
| `assume_real(x); solve(x^2 + 1 == 0, x)` | `UnknownFunction 'assume_real'` | `NoSolutions` (same record shape) |
| `assume_integer(x); solve(x == 1/2, x)` | `UnknownFunction 'assume_integer'` | `NoSolutions` |
| `assume(x > 5); solve(sin(x) == 0, x)` | family conditions `[]` | family conditions `[x > 5]` |
| `assume(y > 0); solve_system([x == 1, y^2 == 4], [x, y])` | `InvalidOperation` "No solver for this system." | the one assignment `x=1, y=2` |
| `assume(x > 5); solve_system([x == 3], [x])` | `Solved` with `x=3` | `NoSolutions`, complete `True`, solutions `[]` |
| controls (no assumptions; unrelated symbol; admit-all) | unchanged | unchanged |

### Item 5 — BOM / UTF-16 surfaces (script `a = 1\nb = 2\ndet(1)`)

| Surface | before (Rust) | after (Rust = oracle) |
|---|---|---|
| `--eval` with leading U+FEFF | `ParseError` "Unexpected character '∩' at position 0.", timings `[]` | `InvalidArgument`, diagnostics position 13, timings `[1, 7, 13]`, line 3 column 1 |
| `--eval` without BOM | position 12, timings `[0, 6, 12]` | unchanged (control) |
| `--file` UTF-8 BOM | ParseError as above | position 13, timings `[1, 7, 13]` |
| `--file` UTF-16 LE / BE (with mark) | `FileReadError` "stream did not contain valid UTF-8" | position 13, timings `[1, 7, 13]` |
| `--stdin` UTF-8 BOM bytes | ParseError | position 13, timings `[1, 7, 13]` |
| `s = "😀"; det(1)` | position 10 | unchanged (control: UTF-16 code units already) |

### Item 6 — evalf digit-cap notice

| Probe | before (Rust) | after (Rust = oracle) |
|---|---|---|
| `setprecision(5000); evalf(sqrt(2), 5000)` | 1000 places, no marker fields | 1000 places, `truncated:true`, `truncationReason:"digit-cap"`, `budget:1000` (also on the `_` variable) |
| `evalf(1/3, 1001)` | 1000 places, no marker | same three fields |
| `evalf(1/2, 5000)` | `0.5`, exact | `0.5`, exact, unmarked (control) |
| `evalf(pi, 1000)` / `setprecision(5000); sqrt(2)` | unmarked | unmarked (controls) |

### Item 7 — elapsed vs elapsedTime

| Probe | before (Rust) | after (Rust = oracle) |
|---|---|---|
| a fast failing script | `elapsed:"0.09 ms"`, `elapsedTime:{unit:"us",value:93}` — the two disagree | `elapsed:"139.5 µs"`, `elapsedTime:{value:139.5,unit:"µs"}` — one selector, same unit, same value |
| success / parse error / pre-engine failure | same mismatch on every path | same scale on every path; the zero pair is `"0 ns"` / `{0,"ns"}` |

### Item 8 — eye/mean/norm

| Probe | before (Rust) | after (Rust = oracle) |
|---|---|---|
| `eye(2,3)` | `InvalidArgument` "eye(): expected 1 to 2 arguments; got 2." | `[[1, 0, 0], [0, 1, 0]]` |
| `eye(3,2)` / `eye(2)` | `eye(2)` ok; `eye(3,2)` refused | both answer their matrices |
| `mean([1,2,3], 0)` | arity refusal | `2` |
| `mean([[1,2],[3,4]], 1)` | arity refusal | `[1.5, 3.5]` |
| `norm([[3,4],[0,0]], 1)` | arity refusal | `[5, 0]` |
| `mean(1)` / `norm(1)` | answers `1` (treated as a scalar reduction) | `InvalidArgument/TypeMismatch` "mean(): argument 1 must be an array or vector; got Natural." plus the shape signature |
| `mean([1,2,3], 1)` | arity refusal | `InvalidArgument/TypeMismatch` "mean(): argument 2 (axis) must be the only axis (0); got 1." |
| `norm([[3,4],[0,0]], 2)` | arity refusal | "norm(): argument 2 (axis) must be one of the value's 2 axes, 0 to 1; got 2." |

## 3. Gates

```text
$ cargo test --workspace --no-fail-fast
lovelace-array 58 ok; lovelace-desktop 67 ok; lovelace-int 125 ok; lovelace-json 7 ok;
lovelace-lang 1,110 ok; lovelace-nat 134 ok; lovelace-real 2,373 ok; lovelace-run 291 ok
(+1 integration test ok); lovelace-symbolics 52 ok;
4,218 tests collected, 0 failed, exit 0.               (baseline 4,112+; +50 new tests)

$ cargo run --release -p lovelace-corpus
corpus: 212 passed, 2 failed, 214 total                 (sym/071, sym/076 — unchanged)

$ node parity/tools/gate-fixtures.mjs
oracle: 20/20
rust:   20/20
PASS: oracle 20/20, rust 20/20

$ node parity/tools/gate-cli.mjs
CLI matrix: 16/16
PASS: every RUN-001..RUN-014 row behaves like the oracle

$ node parity/diff/run.mjs --cases 200 --seed 1 --no-minimize
200 generated cases: 157 match, 43 mismatch, 0 errors
regressions: discovered=1845 replayed=1752 stillMismatching=1046 resolved=706 new=0
```

The differential delta is **new=0, resolved=706**: this round introduced no new mismatch on the
replayed corpus and resolved 706 previously-recorded ones (BOM/parse rows, index refusals, the
precision rows, and the solve-record rows among them). The 43 remaining fresh-case mismatches are
pre-existing classes (§4).

## 4. Not fixed / limitations

1. **`evalf(pi, 1000)` still carries 100 digits** (`StructuredRealPrecisionTests`'s
   `EvalfCountAboveTheAmbientDisplayPrecision`). The value model differs: the reference stores
   `Rl.Pi` at the computation precision and truncates only its RENDERING at the display precision,
   while Rust's bare `pi`/`e` are computed at the display precision (100). Fixing it needs the
   display-precision rendering rule in `Value::display`/`Engine::format`, which also moves
   `sqrt(2)`'s published `display` from 1000 digits to 100; that is a separate, broad change and
   was not attempted here. Item 6's own contract (the cap marker) is fixed.
2. **Nested-call diagnostic positions.** For an argument refusal raised inside a nested call
   (`print(evalir(..., 2.9))`) the oracle reports the STATEMENT offset (17); Rust reports the
   inner call site (23). This is the engine's general "a builtin failure is positioned at its call
   site" rule (`Engine::call`), not an evalir rule; changing it would move every nested-call
   refusal and was left alone.
3. **Symbolic constant canonical form.** `solve(x^2+1==0, x)` still publishes `i` as
   `(sym i)` with `freeSymbols:["i"]` where the oracle publishes `(i)` with an empty symbol
   list — a pre-existing Symbolics construction difference, unrelated to this round's items.
4. **Statement timing entries still publish zero durations** (`{0,"ns"}`); the C# runner
   measures each statement. The *envelope* pair now agrees (item 7), but per-statement timings do
   not carry real elapsed values.
5. **Solve-assumption coverage is the store the tests exercise**: relations with numeric bounds,
   the positive/non-negative/negative property atoms, and the real/integer domain atoms. A
   domain atom that cannot be decided at a root is kept (not published as a condition); only
   relation atoms are carried onto a family. `solve_system` gained a triangular root-enumerating
   fallback but remains a linear/triangular solver — a fully coupled nonlinear system still
   answers "No solver for this system.".
6. **`--file` encoding detection covers UTF-8/UTF-16 LE/BE and UTF-32 LE/BE marks**; a file
   with no mark is decoded as UTF-8 (the reference's `File.ReadAllText` default).

## 5. Tests added

50 tests in two modules (production-adjacent, `#[cfg(test)]` only):

- `rust/lovelace-lang/src/round27_batch.rs` (37): evalir digit count/refusals/arity, the
  zeros/ones/series refusal guards, the index taxonomy, the solve assumption store (roots,
  NoSolutions shape, domains, family conditions, system filtering, controls, assume* returns),
  the engine-level BOM/UTF-16 position rules, the evalf cap markers, and eye/mean/norm.
- `rust/lovelace-run/src/round27_run_batch.rs` (13): the wire-level elapsed/elapsedTime
  agreement (success, parse failure, sub-millisecond failure, pre-engine zero pair), the three
  BOM surfaces (--eval, --file UTF-8/UTF-16 LE/BE, --stdin via the production decoder), surrogate
  positions, evalir answers/refusals on the wire, the wide-argument refusals, the evalf cap
  marker on the result and the `_` variable, the solve assumption verdict, and eye/mean/norm.

`parity_batch::run_args` now reads a script file through the SAME `decode_script_bytes` the
binary uses, so the in-process file-surface tests exercise the production decoder.
