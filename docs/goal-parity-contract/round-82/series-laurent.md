# Round 82 — series Laurent-tail family (4 rows)

**Status: complete.** All four deferred `SeriesLaurentAndBranchPointTests` rows now produce a
full-envelope match against the pinned oracle after normalising `revision`, `elapsed`,
`elapsedTime` and `timings`. Two production behaviours were wrong, not missing APIs: the Rust series
path dropped every NEGATIVE exponent (a pole was flattened to `0`, and `sin(x)/x` lost its x^2
coefficient because the numerator was not expanded past the requested order), and the branch-point
fallback republished the unresolved source (`sqrt(abs(x))`) instead of the kink-resolved one
(`sqrt(x)`). A third defect in the same path — the geometric inverse overwrote its own base, so
`1/(x-1)`'s x^2 coefficient had the wrong sign from order 3 up — was fixed with it. No test
expectation was changed or weakened.

Measured 2026-11-12 (America/Sao_Paulo) against the pinned oracle
`1cc28f255eca6029849f880949fc7be1d40ab8af` and `rust/target/release/lovelace-run.exe`, at repo
root, via `--file <script> --json --omit-functions --omit-variables` on both runners.

## 1. The four rows, before and after

| # | Census id | Probe | Field | Before (Rust) | After (Rust) = oracle |
|---:|---|---|---|---|---|
| 0 | `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.SeriesLaurentAndBranchPointTests/Series_AcrossAPoleOrABranchPoint_PublishesTheExpansion#0` | `x = symbol("x"); series(1/x, x, 0, 2)` | `$.result.display` / canonical | `0`, canonical `(rat 0 1)` — the pole was dropped | `x^-1 + O(x^2)`, canonical `(add (pow (sym x) (rat -1 1)) (order (sym x) (rat 0 1) (rat 2 1)))` |
| 1 | `.../Series_AcrossAPoleOrABranchPoint_PublishesTheExpansion#1` | `x = symbol("x"); series(1/x^2, x, 0, 2)` | `$.result.display` / canonical | `0`, canonical `(rat 0 1)` — the denominator's leading term sat at index 2, beyond a two-coefficient expansion | `x^-2 + O(x^2)`, canonical `(add (pow (sym x) (rat -2 1)) (order (sym x) (rat 0 1) (rat 2 1)))` |
| 2 | `.../Series_AcrossAPoleOrABranchPoint_PublishesTheExpansion#2` | `x = symbol("x"); series(sin(x)/x, x, 0, 3)` | `$.result.display` / canonical | `1 + O(x^3)`, canonical `(add (rat 1 1) (order (sym x) (rat 0 1) (rat 3 1)))` — sin(x)'s x^3 coefficient was invisible at the requested order, so the quotient dropped -1/6*x^2 while still claiming O(x^3) | `1 - 1/6*x^2 + O(x^3)`, canonical `(add (rat 1 1) (mul (rat -1 6) (pow (sym x) (rat 2 1))) (order (sym x) (rat 0 1) (rat 3 1)))` |
| 4 | `.../Series_AcrossAPoleOrABranchPoint_PublishesTheExpansion#4` | `x = symbol("x"); series(sqrt(abs(x)), x, 0, 2)` | `$.result.display` / canonical | `sqrt(abs(x))`, canonical `(pow (fn abs (sym x)) (rat 1 2))` — the branch-point fallback republished the UNRESOLVED source | `sqrt(x)`, canonical `(pow (sym x) (rat 1 2))` — the kink is resolved before publication |

After the fixes the four probes are **full-envelope matches** (not just matching display text):
a sorted-key, volatile-field-stripped JSON comparison of the whole envelope printed `FULL MATCH`
for each of the four row scripts (`1overx`, `1overx2`, `sinx_over_x`, `sqrtabsx`); the same
comparison over 33 series probes (the four rows, the two controls and the order/Laurent boundaries
below) reported **33/33 full matches**, no mismatches.

Case #3 (`series(sqrt(x), x, 0, 2)`) and the control
`Series_OfARemovableDenominator_IsUnchanged` (`series(1/(x-1), x, 0, 2)`) already matched and were
not moved: `sqrt(x)` and `-x + O(x^2) - 1`.

## 2. Production fixes

### 2.1 A Laurent-aware series path

`rust/lovelace-symbolics/src/series_kink.rs` (new `Laurent`, `power_series`, `split_fraction`,
`leading_index_within`, `divide`, `laurent_expression`; `publish_series` rewritten at
`:433`). The old path expanded the whole expression with `maclaurin`, which keeps only
`exponent >= 0` and at most `order` coefficients — so a pole vanished and a denominator's leading
term beyond the requested order was invisible. The new path reproduces the reference's
`Series.Of`/`Divide`:

* `split_fraction` moves a top-level negative power (or a product's negative-power factors) into a
  denominator, so numerator and denominator are expanded separately.
* `leading_index_within` doubles its probe up to 64 coefficients to find an operand's first non-zero
  coefficient (`x^2` for `1/x^2`, sin(x)'s x^3 coefficient for `sin(x)/x`).
* the numerator is expanded to `order + max(lb, 0)` and the denominator to
  `order + max(2*lb - la, 0)`, exactly the reference's headroom.
* `divide` computes the quotient's leading power as `la - lb` (the difference of leading ORDERS,
  not just leading powers), carries every power below the truncation, and `laurent_expression`
  publishes the tail at `leading_power + coefficients.len()` — so `1/x` at order 2 is
  `x^-1 + O(x^2)` and `1/x^2` carries powers -2..1 under the same O(x^2) tail.
* `power_series` reports `None` for a branch point (or a pole reached without the split), and
  `publish_series` then publishes the KINK-RESOLVED source, translated back to the caller's
  coordinates. That is why row 4 is `sqrt(x)` rather than `sqrt(abs(x))` (or `sqrt(x + point)`
  at a non-zero expansion point).

The same path now also matches the oracle on adjacent probes that were previously wrong:
`series(1/(x-1), x, 0, 3) = -x - x^2 + O(x^3) - 1` and order 4,
`series(1/(x^2 + x), x, 0, 2) = x^-1 - 1 + x + O(x^2)`,
`series(exp(x)/x, x, 0, 3) = x^-1 + 1 + 1/2*x + 1/6*x^2 + O(x^3)`,
`series(sin(x)/x^2, x, 0, 2) = x^-1 - 1/6*x + O(x^2)`,
`series(1/x + 1/x^2, x, 0, 2) = x^-1 + x^-2` (the reference publishes the source unchanged when
the Laurent sum cannot be split) and `series(sqrt(abs(x - 1)), x, 1, 2) = sqrt(x - 1)`.

### 2.2 The geometric inverse's base was overwritten

`rust/lovelace-symbolics/src/lib.rs:2118` (`series_inv`). The 1/(1+u) loop built `term = term * u`
and then assigned `u = term.clone()`, replacing the series' BASE with the current power: the
iteration squared itself, so every coefficient from x^2 up was wrong. Deleting that assignment
restores `1 - u + u^2 - u^3 + ...`. Observable before/after:
`series(1/(x-1), x, 0, 3)` was `-x + x^2 + O(x^3) - 1`, now `-x - x^2 + O(x^3) - 1` (the
oracle's answer); the order-2 control was inside the truncation and is unchanged.

## 3. Tests added (one per row plus two boundaries)

New module `rust/lovelace-lang/src/round82_series_laurent.rs` (declared at
`rust/lovelace-lang/src/lib.rs:2446`). Each test drives the row's own script through the language
surface (`Engine::eval`) and asserts the oracle-observed display; the row tests also pin the
canonical form.

| Row | Test | What it pins |
|---:|---|---|
| 0 | `round82_series_laurent::series_across_a_pole_keeps_the_laurent_leading_power` (`round82_series_laurent.rs:46`) | `x^-1 + O(x^2)` and canonical `(add (pow (sym x) (rat -1 1)) (order ...))` |
| 1 | `...::series_across_a_second_order_pole_keeps_two_negative_powers` (`:59`) | `x^-2 + O(x^2)` and the canonical negative powers |
| 2 | `...::series_at_a_removable_singularity_keeps_the_hidden_quadratic_term` (`:72`) | `1 - 1/6*x^2 + O(x^3)` and canonical |
| 4 | `...::series_at_a_branch_point_publishes_the_kink_resolved_source` (`:85`) | `sqrt(x)`, canonical `(pow (sym x) (rat 1 2))`, no `abs`, no `sqrt(0)`, no `/0` |
| boundary — order | `...::series_order_boundary_truncates_at_the_requested_power` (`:106`) | `series(1/x, x, 0, 1) = x^-1 + O(x)`, `series(1/x^2, x, 0, 1) = x^-2 + O(x)`, the same at order 3, and `series(sin(x)/x, x, 0, 4) = 1 - 1/6*x^2 + O(x^4)` — the tail sits at the requested order, never at leading+count |
| boundary — Laurent | `...::laurent_negative_exponents_publish_ascending_with_the_requested_tail` (`:124`) | `series(x^-3, x, 0, 2) = x^-3 + O(x^2)`, `series((1 + x)/x^2, x, 0, 2) = x^-2 + x^-1 + O(x^2)`, and the unsplit Laurent sum `series(1/x + 1/x^2, x, 0, 2) = x^-1 + x^-2` |

The four census ids are listed in section 1; they are the rows a porting round can now flip from
`deferred` to `ported` in `parity/manifest-map.json` (that map was not edited here, per the
round-74/79/81 split).

## 4. Gates

| Gate | Command | Observed | Exit |
|---|---|---|---:|
| Focused | `cargo test -p lovelace-lang round82_series_laurent` | **6 passed; 0 failed; 0 ignored** (1,602 filtered) | 0 |
| Workspace | `cargo test --workspace` | **5,277 passed; 0 failed; 1 ignored** across 21 result blocks. The shared tree also carries the concurrent evalf lane's additional tests; this lane's six are included and green | 0 |
| Corpus | `cargo run --release -p lovelace-corpus` | `corpus: 214 passed, 0 failed, 214 total` | 0 |
| Fixtures | `node parity/tools/gate-fixtures.mjs` | `oracle: 20/20`, `rust: 20/20`, `PASS` | 0 |
| CLI | `node parity/tools/gate-cli.mjs` | `CLI matrix: 16/16`, `PASS: every RUN-001..RUN-014 row behaves like the oracle` | 0 |
| Differential | `node parity/diff/run.mjs --cases 200 --seed 1 --no-minimize --concurrency 10` | generated **200 match / 0 mismatch / 0 error**; replayed **1,758 match / 0 mismatch / 0 error**; `mismatches` empty; `harness exit: 0` | 0 |
| Clippy | `cargo clippy -p lovelace-symbolics -p lovelace-lang --all-targets -- -D warnings` | `Finished`, no diagnostics | 0 |
| Format | `cargo fmt -p lovelace-symbolics -p lovelace-lang -- --check` | clean | 0 |

The differential run directory is `parity/diff/runs/20261112T143025Z-seed1`; its `monitor.json`
records `generated {total: 200, match: 200, mismatch: 0, error: 0}` and
`regressions {total: 1758, match: 1758, mismatch: 0, error: 0}`.

## 5. Scope, shared files and boundaries

Changed by this round:

* `rust/lovelace-symbolics/src/series_kink.rs` — the Laurent-aware path (§2.1).
* `rust/lovelace-symbolics/src/lib.rs` — the `series_inv` recurrence fix (§2.2).
* `rust/lovelace-lang/src/lib.rs` — **shared file**: one `#[cfg(test)] mod round82_series_laurent;`
  declaration plus its doc comment. The evalf lane's `pi`/`e` precision change and its
  `round82_evalf_precision` declaration are also in this file and were not touched. Running
  `cargo fmt -p lovelace-lang` to make the package format gate pass rewrapped ONE line of their
  `e` arm (`None if name == "e" => { ... }`); it is whitespace-only, no token or logic changed,
  and is disclosed rather than hidden.
* `rust/lovelace-lang/src/round82_series_laurent.rs` — new (6 tests).
* `docs/goal-parity-contract/round-82/series-laurent.md` — this report.

No `.cs` file, fixture, corpus file, `parity/` file, `rust/lovelace-desktop` file or other
document was edited. `rust/lovelace-lang/src/builtins.rs`, `numeric.rs` and
`rust/lovelace-run/src/main.rs` are the concurrent evalf lane's changes; this lane did not edit
them.

Neighbouring series probes that remain different are **not** these rows and were left alone:

* `series(tan(x), x, 0, 3)`: oracle `x + O(x^3)`, Rust `tan(x)` — the rational engine has no
  `tan` expansion, so the branch-point fallback publishes the source.
* `series(1/sqrt(x), x, 0, 2)`: oracle `1/sqrt(x)`, Rust `sqrt(x)^-1` — source-printer shape for a
  negative power of a sqrt function.
* `series(sqrt(abs(x)), x, 0, 1)` and `series(sqrt(x), x, 0, 1)`: oracle `0`, Rust `sqrt(x)` —
  the reference only marks the branch point from order 2 (the order-1 coefficient is a value);
  neither variant is a census row.
* `series(-1/x, x, 0, 2)`: oracle `-1/x + O(x^2)`, Rust `-x^-1 + O(x^2)` — rendering a negative
  unit coefficient's magnitude inside a series sum.
* `series(abs(x - 1), x, 1, 3)`: oracle `x + O((x - 1)^3) - 1`, Rust `x - 1 + O((x - 1)^3)` —
  constant placement for a non-zero expansion point.

## 6. Reproduction

```powershell
$repo = 'C:\Users\ricar\dev\LovelaceSharp'
$env:RUSTUP_HOME = "$repo\.toolchain\rustup"; $env:CARGO_HOME = "$repo\.toolchain\cargo"
$env:PATH = "$repo\.toolchain\cargo\bin;" + $env:PATH
$env:TMP = "$repo\rust\target\tmp"; $env:TEMP = $env:TMP
Set-Location $repo

# the four rows
'x = symbol("x"); series(1/x, x, 0, 2)'        | Set-Content row0.ls
'x = symbol("x"); series(1/x^2, x, 0, 2)'      | Set-Content row1.ls
'x = symbol("x"); series(sin(x)/x, x, 0, 3)'   | Set-Content row2.ls
'x = symbol("x"); series(sqrt(abs(x)), x, 0, 2)' | Set-Content row4.ls
foreach ($row in 'row0','row1','row2','row4') {
  dotnet exec "$repo\Lovelace.Run\bin\Release\net10.0\Lovelace.Run.dll" --file "$row.ls" --omit-functions
  "$repo\rust\target\release\lovelace-run.exe" --file "$row.ls" --omit-functions
}

Set-Location "$repo\rust"
cargo test -p lovelace-lang round82_series_laurent
cargo test --workspace
cargo run --release -p lovelace-corpus
Set-Location $repo
node parity/tools/gate-fixtures.mjs
node parity/tools/gate-cli.mjs
node parity/diff/run.mjs --cases 200 --seed 1 --no-minimize --concurrency 10
```
