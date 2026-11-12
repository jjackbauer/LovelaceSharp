# Round 89 — assume and collect: the last two weak descriptor rows

**Status: fixed and pinned.** The two descriptor examples still mapped execution-only are now
equivalent to the pinned oracle on their published payloads: `assume(and(x > 0, x < 10))` returns
the conjunction as a `Symbolic` result (it no longer answers `Void`), and
`collect(x*y + x + y + 1, x)` folds the degree-0 coefficient `y + 1` into the outer n-ary Add
(display/canonical/nodeCount now match). Measured 2026-11-12 (America/Sao_Paulo) against
`Lovelace.Run/bin/Release/net10.0/Lovelace.Run.dll` and `rust/target/release/lovelace-run.exe`.

**The 2 ids fixed:**

- `Lovelace.Suite.Tests/Lovelace.Suite.Tests.DescriptorExampleTests/Example_Executes#10` (`assume`)
- `Lovelace.Suite.Tests/Lovelace.Suite.Tests.DescriptorExampleTests/Example_Executes#24` (`collect`)

Both were `disposition = "ported"` in `parity/manifest.toml` and mapped to
`lovelace-lang::suite_engine_batch::descriptor_examples_execute`, which asserted only that the
snippet executes (VAL-004). No mapping or disposition changed; the mapped test was strengthened.

## 1. Probe (oracle first)

Every probe ran the documented prelude
`x = symbol("x"); y = symbol("y"); t = symbol("t");` plus the example through both binaries with
`--file <probe>.ls --omit-functions`, then compared the full envelope with the differential's four
volatile keys (`revision`, `elapsed`, `elapsedTime`, `timings`) removed at any depth.

| Row | Snippet | Oracle result | Rust before |
|---|---|---|---|
| #10 | `assume(and(x > 0, x < 10))` | `Symbolic`; typed `x < 10 and x > 0 (Symbolic)`; canonical `(and (lt (sym x) (rat 10 1)) (gt (sym x) (rat 0 1)))`; nodeCount 7; `_` binding present; 4 variables (`_`, t, x, y) | `$.result` **absent**; last `timings` entry `resultKind: "Void"`; **no** `_` binding; 3 variables (t, x, y) |
| #24 | `collect(x*y + x + y + 1, x)` | display/typed `1 + y + x*(y + 1)`; canonical `(add (rat 1 1) (sym y) (mul (sym x) (add (rat 1 1) (sym y))))`; nodeCount **8** | display/typed `x*(y + 1) + y + 1`; canonical `(add (mul (sym x) (add (rat 1 1) (sym y))) (add (rat 1 1) (sym y)))`; nodeCount **9** |

Both sides exit 0 with `ok: true`; the divergences are payload-only, not refusals. The oracle's
`assume` result and its automatic `_` variable carry the same structured payload (verified in the
same envelope).

### 1.1 What the missing collect node is

The reference's `Collect` returns `Exprs.Add(terms)`, and that constructor FLATTENS a `terms`
entry that is itself a sum. Rust assembled `Expr::Add(terms)` directly, so the degree-0
coefficient `y + 1` stayed a single term:

- oracle root: `(add 1 y (mul x (add 1 y)))` — the outer Add has **three** operands, and the
  constants `1` and `y` are direct operands;
- Rust before: `(add (mul x (add 1 y)) (add 1 y))` — the outer Add has **two** operands and a
  nested Add, which is the extra (ninth) node.

The missing node is therefore the flattened constant coefficient: no term is absent mathematically,
but the outer Add lost `1` and `y` as operands and gained a nested Add node in their place.

## 2. Production fixes

1. **assume returns its argument** — `rust/lovelace-lang/src/builtins.rs:3005-3011`. The
   `and` branch still asserts each conjunct recursively (same assumptions recorded), then returns
   `Value::Symbolic` of the conjunction instead of `Value::Void`. This is the reference's
   contract: `SymbolicsPlugin.cs:351-357` returns `rel` after `AssumeRecursive` records it.
2. **assume(not(...)) returns the expression as written** — same file, :3034-3037. The negation is
   still recorded (the assumption set still stores `x != 2`), but the result is the `not` node the
   caller wrote rather than the derived `x != 2` relation. This is the same `return rel` contract
   and the same arm; it was fixed together with #10. `assumptions()` output is unchanged.
3. **collect folds through the canonical adder** — `rust/lovelace-symbolics/src/lib.rs:2746-2797`
   (fix at :2773-2793). The coefficient extraction and the descending-power construction are
   unchanged; the final assembly now folds each term with `Expr::add` instead of building
   `Expr::Add(terms)`. `Expr::add` is the reference's `Exprs.Add`: it flattens nested sums,
   combines like terms and orders by the kernel's term order, which is exactly what produces
   `(add 1 y (mul x (add 1 y)))` and nodeCount 8.

The logical printer (`and`/`or`/`not` display/canonical/parser) was **re-read** before editing and
left untouched: the round-88 lane's `logical_text`, canonical tags and `canonical_rank` entries were
already in the working tree, and this round changes no printer code. There was no conflict.

## 3. Per-row before / after (Rust, measured)

| Row | Before (Rust) | After (Rust) | Oracle |
|---|---|---|---|
| #10 | result absent, `resultKind: Void`, no `_` variable, 3 variables | result `Symbolic`, typed `x < 10 and x > 0 (Symbolic)`, canonical `(and (lt (sym x) (rat 10 1)) (gt (sym x) (rat 0 1)))`, nodeCount 7, `_` present, 4 variables | matches |
| #24 | typed `x*(y + 1) + y + 1`, canonical `(add (mul ...) (add ...))`, nodeCount 9 | typed `1 + y + x*(y + 1)`, canonical `(add (rat 1 1) (sym y) (mul (sym x) (add (rat 1 1) (sym y))))`, nodeCount 8 | matches |

After the fixes the whole normalized envelopes of both probes are **deep-identical** to the
oracle's (key order aside): `result`, `variables` (including `_`), `output`, `functions` and
`artifacts`.

## 4. Strengthened tests (no assertion removed or weakened)

| Row | Test | What it now asserts |
|---|---|---|
| #10 | `lovelace-lang::suite_engine_batch::descriptor_examples_execute` (`suite_engine_batch.rs:396`) | the example's published result: typed `x < 10 and x > 0 (Symbolic)`, structured `kind: Symbolic`, canonical `(and (lt ...) (gt ...))`, nodeCount 7. A `Void` result cannot satisfy the typed assertion, so the pre-fix regression is caught. |
| #24 | same mapped test (`suite_engine_batch.rs:422`) | typed `1 + y + x*(y + 1)`, canonical `(add (rat 1 1) (sym y) (mul ...))`, nodeCount **8**, and the **node itself**: the value is matched as `Sym::Add` with exactly 3 terms, containing `1`, `y` and `x*(y + 1)` as direct operands (a nested coefficient fails the length assertion). |
| #24 (kernel) | `lovelace-symbolics::tests::Collect_GivenConstantTermThatIsASum_FlattensItIntoTheOuterAdd` (`lib.rs:5635`) | new unit test on the kernel directly: display, canonical, `node_count() == 8`, and the root `Expr::Add` has three terms `1`, `y`, `x*(y + 1)`. |

The descriptor test's `compile` and `or` arms from round 88 are unchanged and still pass.

## 5. Gates (observed 2026-11-12)

| Step | Command | Observed | Exit |
|---|---|---|---|
| Workspace suite | `cargo test --workspace` (in `rust/`) | **5,372 passed, 0 failed, 1 ignored** over 21 binaries | 0 |
| Documentation corpus | `cargo run --release -p lovelace-corpus` | `corpus: 214 passed, 0 failed, 214 total` | 0 |
| Protocol fixtures | `node parity/tools/gate-fixtures.mjs` | `oracle: 20/20`, `rust: 20/20`, `PASS: oracle 20/20, rust 20/20` | 0 |
| CLI matrix | `node parity/tools/gate-cli.mjs` | `CLI matrix: 16/16`, `PASS: every RUN-001..RUN-014 row behaves like the oracle` | 0 |
| Differential (generated + replay) | `node parity/diff/run.mjs --cases 200 --seed 1` | generated **200 match / 0 mismatch / 0 error**; replayed **1758 match / 0 mismatch / 0 error** (115 duplicate skipped; 1,958 executions); `regressions: discovered=1873 replayed=1758 stillMismatching=0` | 0 |
| Clippy (own crates) | `cargo clippy -p lovelace-lang -p lovelace-symbolics -p lovelace-run --all-targets -- -D warnings` | clean (`Finished dev profile` only) | 0 |
| Format (own crates) | `cargo fmt -p lovelace-lang -p lovelace-symbolics -p lovelace-run -- --check` | clean (no output) | 0 |

The workspace count is the measured one. Against the task's 5,366 baseline, this round adds one
test function (the new `lovelace-symbolics` collect test); the remaining +5 are concurrent lanes'
tests already present in the shared tree. The first workspace run in this session hit a
**foreign-crate** failure while the concurrent desktop lane was mid-edit
(`lovelace-desktop`: first the astral-selection test, then a duplicate
`fn last_line_maps_exactly_and_missing_lines_have_no_range` that did not compile); the crate
settled during this round and the final run above is green.
No `lovelace-desktop` file was touched by this round.

## 6. Scope and what remains

Changed files: `rust/lovelace-symbolics/src/lib.rs` (the `collect` assembly and one kernel test),
`rust/lovelace-lang/src/builtins.rs` (the `assume` arm only), and
`rust/lovelace-lang/src/suite_engine_batch.rs` (the two descriptor arms + one import). No `.cs`,
fixture, corpus, `parity/` file, `rust/lovelace-desktop` file or other document was edited; probe
files and captured envelopes live under `rust/target/tmp/` (git-ignored).

Adjacent divergences observed while probing, **not** part of the two rows and not fixed here:

- `collect((x + 1)^2, x)`: the oracle returns the input unchanged `(x + 1)^2` (its
  `TryCoefficients` refuses a base that is a sum), while Rust expands first and answers
  `x^2 + 2*x + 1`. No descriptor example covers it; it is a pre-existing expand-before-collect
  shape difference, not a regression of this round.
- `Eq` relations render `==` in Rust and `=` in the oracle (`x == 2` vs `x = 2`). This is
  pre-existing and independent of `assume`; it is visible in `assume(not(x == 2))`'s result
  (`not x == 2` vs the oracle's `not x = 2`). The #10 example contains no equality, so its
  envelope is identical to the oracle.

With #10 and #24 closed, all nine VAL-004 examples are now equivalent to the oracle on their
published payloads: #4/#89/#94 and #25/#46 by round 88, #97/#98 by round 87, and #10/#24 here.
