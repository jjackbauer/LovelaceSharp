# Round 35 — the desktop incremental cache (INC-001..INC-010)

Status: **implemented and observed**. `cargo test -p lovelace-desktop` is **382 passed, 0 failed**;
the incremental module alone has **25** tests (14 engine-free + 11 real-engine end-to-end,
including the metamorphic 20-edit run), and the workbench adds a per-statement reuse test for each
engine. `--journey J-07` and `--journey J-08` are new modes and both **PASS** with REF-incremental's
exact values (`reused=3`); all seven implemented journeys pass (`journeys=7 failed=0`);
`--gui-selftest` is 19/19. Nothing is asserted that was not observed.

## 1. What changed

| File | Change |
|---|---|
| [`src/incremental.rs`](../../../rust/lovelace-desktop/src/incremental.rs) | New: token scan, statement normalization, per-statement read/call facts, the user-function table (transitive reads, calls and mutations), effect classification, `StatementKey`, `CachedEntry`, and the per-document `DocumentCache`. 25 tests. |
| [`src/engine.rs`](../../../rust/lovelace-desktop/src/engine.rs) | `CachedValue` (the engine's own typed value) plus five `Evaluator` methods: `supports_cached_values`, `variable_key`, `last_value`, `apply_cached_value`, `tracks_result_variable`. All default off, so a minimal evaluator is never offered a reuse. |
| [`src/engine/real.rs`](../../../rust/lovelace-desktop/src/engine/real.rs) | Keeps the `lovelace_lang::Value` of the last `eval`; canonical dependency key from `structured::project_value`; applies a cached value through `Engine::set_variable`; reports that `eval` stores `_`. |
| [`src/engine/fake.rs`](../../../rust/lovelace-desktop/src/engine/fake.rs) | Same seam for the deterministic fake (`--no-default-features` builds). |
| [`src/execution.rs`](../../../rust/lovelace-desktop/src/execution.rs) | `ExecutionHost` owns one `DocumentCache` per document; `RunWorker` takes it for the run, plans/keys each statement before evaluation, applies a hit, stores a miss, tracks function revisions, and returns the cache on every exit path. `StepCommitted { reused }` now carries the observed flag. `ExecutionHost::clear_cache` is Run All Fresh. |
| [`src/workbench.rs`](../../../rust/lovelace-desktop/src/workbench.rs) | `committed_reused: Vec<bool>` (per-statement flags), `clear_incremental_cache` (INC-009), and the `reused` field documented as the observed count. |
| [`src/journey.rs`](../../../rust/lovelace-desktop/src/journey.rs) | J-07 and J-08 headless journey modes with their expectations from `journeys.md`/REF-incremental. |
| [`src/main.rs`](../../../rust/lovelace-desktop/src/main.rs) | Usage/doc mention the two new journey ids. |

Engine crates were not touched.

## 2. The key scheme

One `StatementKey` per statement, all parts full canonical strings:

1. **normalized statement source** — comments (`# …` outside strings) dropped, whitespace runs
   collapsed to one space, spaces adjacent to ASCII punctuation dropped, string literals copied
   verbatim. `a = 1`, `a=1` and a re-indented `b = a +\n 1` normalize to the same key.
2. **document precision** (the value submitted with the run) **and effective precision** (what
   `engine.precision()` reported when the statement was planned, so a `setprecision(...)` earlier
   in the script participates).
3. **ordered dependency keys** — for each global the statement reads, sorted by name:
   `name -> canonical typed value key`, or `UNDEF` when the engine has no such global. The
   canonical key is the engine's own structured projection of the **typed value**
   (`{"exact":true,"kind":"Natural","value":"2"}` for `a = 2`), never a display string. The fake
   publishes `Fake:Int:<n>` / `Fake:Text:<len>:<text>`.
4. **ordered function keys** — for every callable the statement reaches directly or transitively,
   sorted by name: `builtin` or `rev:<n>` from the per-document function-revision counter.

For example, `b = a + 1` with `a = 2`, both precisions 1000, no calls:

```
StatementKey {
  source: "b=a+1",
  document_precision: 1000, effective_precision: 1000,
  dependencies: [("a", "{\"exact\":true,\"kind\":\"Natural\",\"value\":\"2\"}")],
  functions: [],
}
```

The reference digests these parts with SHA-256; this port stores the canonical strings themselves
as the `HashMap` key, so no hash collision can ever reuse a different statement. The whole key is
compared for equality (`source` + both precisions + every dependency + every function revision).

## 3. Dependency extraction — and its over-approximation

The `lovelace-lang` parser is not reachable from the desktop crate, so reads are recovered with a
token scan of the statement text (`statements::slice` has already isolated top-level statements):

- a `#` outside a string starts a comment to end of line; a quoted string (with escapes) is one
  opaque token; an identifier is a letter/underscore followed by letters, digits or `_`;
- every identifier outside strings/comments is a **read**, except language keywords
  (`func if else while for return break continue in not and or true false`), the statement's own
  simple assignment target, and the declared parameters of a `func` body being scanned;
- an identifier immediately followed by `(` is a **call**, not a variable read;
- a call's target is expanded through the source-level function table: the body's global reads and
  its calls are added transitively, so changing a global only read inside `f` invalidates a
  statement that calls `f` (and a function that calls `f`).

**State this over-approximates:** field names after `.`, record field labels, index operands and
loop variables are all treated as global reads; an identifier read only inside a called function's
body is added to the caller's read set; a call to any callable the desktop cannot classify — not a
source `func` and not an engine builtin — makes the statement effectful. Over-approximation can
only cost a reuse, never serve a stale value (INC-006 is satisfied because the key is derived from
the full canonical typed value, never from a displayed/precision-truncated string).

A statement is **memoizable** only when it is a single expression or a simple `name = expression`
(and it contains no nested/compound assignment); `func` definitions, `if`, `while`, `for`,
`return`, `break`, `continue` and `{ … }` blocks always evaluate, matching the reference's
`IsMemoizable`.

## 4. Effect detection

A statement is effectful — always re-evaluated, never cached — when it calls, directly or
transitively through user functions:

- `print` → output;
- `plot` → a plot capture and file;
- `setprecision` → the engine's display precision;
- `assume`, `assume_clear`, `assume_integer`, `assume_negative`, `assume_nonnegative`,
  `assume_positive`, `assume_real`, `assumptions` → the engine's global symbolic assumption
  context (mutating or reading it);
- `clear` (the fake engine's builtin) → removes every global;
- any callable of **unknown purity** (not a source `func` and not an engine builtin);
- any user function whose body **contains an assignment** (`func f(x) = (g = x)` writes a global
  while returning a value), or that calls one of the above.

A call to a known builtin outside that list (`sqrt`, `pi`, `diff`, …) is treated as pure, exactly
like the reference's three-name side-effect list plus its transitive user-function walk; the extra
deny-list entries are the stateful builtins the reference's list does not name. Function
definitions are never memoized; the effect classification is computed once per run from the
sliced source plus the engine's own builtin registry (`capture_state`, builtins only).

## 5. Apply-on-reuse — and the trade-off

A cache hit is reported as `reused` **only after the cached typed value has been applied to the
run's engine**:

- the statement's simple assignment target (if any) receives the value through
  `Evaluator::apply_cached_value` (the real engine's `Engine::set_variable`);
- for a non-`Void` value the engine's `_` result binding is set too, because
  `lovelace_lang::Engine::eval` stores `_` after every statement — a fresh run would have set it,
  and the metamorphic test would catch the difference in the variables table. An engine that
  publishes no result variable (the deterministic fake) says so through
  `Evaluator::tracks_result_variable`, and then a reuse applies only the write target: the fake's
  fresh and reused runs must publish the same variables, and the workbench test
  `the_fake_engine_reuses_without_inventing_a_result_variable` pins that;
- `Void` values apply nothing (a print/plot returns `Void`; those statements are effectful anyway).

If the engine refuses (`supports_cached_values` false, or `apply_cached_value` returns false),
the statement is **evaluated** and reported as computed. The cache never skips a statement whose
value it could not apply: that would silently corrupt the workspace, and the observation
`an_engine_that_cannot_apply_a_cached_value_never_reports_reuse` proves the fallback (3
evaluations, 0 reuse, identical values). Cached entries also carry the commit's display,
structured projection and inspection payload, because a reuse must publish the same coherent
`WorkspaceSnapshot` a fresh evaluation would have (`a_symbolic_result_...` compares all three).

Trade-off: reuse skips **evaluation**, not the engine's state capture. Every commit — reused or
computed — still calls `variables()` / `functions()` / `state()` so the panes and the completion
catalogue describe the actual workspace; the cost saved is the statement evaluation, and the key
building itself projects only the globals that statement reads.

## 6. Invalidation

- **Variable change:** `b = a + 1` is keyed on `a`'s typed value; changing `a` changes the key and
  `b` recomputes. `c = pi(100)` has no dependency, so it reuses. A rewrite that only re-indents or
  re-comments a statement keeps its source key.
- **Function redefinition:** a `func` statement always evaluates, and its revision moves only when
  the combined normalized definition text changes (`note_definition`). A call site's key includes
  that revision transitively, so redefining `f` invalidates `q = f(1)` **and** `y = g(3)` where
  `g` calls `f` (which calls `h`), while an unchanged re-run of the definition leaves the call
  sites reusable. Redefinitions of one name are merged (union of bodies) for dependency purposes.
- **Precision change:** both the document precision and the effective precision are in the key. A
  precision-control change reuses nothing (J-03 still reports `reused=0`; the precision test adds
  the positive control: returning to the same precision reuses again).
- **Clear Variables (WKS-004):** removes the published globals but does not touch the cache; a
  later statement reading a removed global keys its dependency as `UNDEF`, so it recomputes
  rather than replaying a stale value.
- **Run All Fresh (INC-009):** `ExecutionHost::clear_cache` / `Workbench::clear_incremental_cache`
  drop the memo table (keeping function revisions) so the next run evaluates everything; the test
  proves the values are unchanged. A clear requested during a run wins: the worker returns its
  cache only if the document has no slot.
- **Capacity:** a deterministic cap of 4096 entries per document clears the table before the next
  insertion. The cache is a memo table, so eviction can only cause recomputation.

## 7. Wiring and journeys

`ExecutionHost` now keeps the per-document cache and gives it to the run's worker; the worker
plans each statement with the *pre-evaluation* typed inputs, applies a hit with
`apply_cached_value`, otherwise evaluates and stores. The `StepCommitted { reused }` flag is the
observed truth, and `WorkbenchDocument` records it per statement (`committed_reused`) as well as
the total (`reused`).

- **J-07** (`a = 2`; `b = a + 1`; `c = pi(100)`; `b + c`, edit only the last statement, run twice):
  run 1 `reused=0`, run 2 `reused=3` with flags `[true, true, true, false]`, identical `a`, `b`,
  `c`, and `pi(100)`/`b + c`/`b + c + 1` byte-identical to REF-incremental.
- **J-08** (change `a`, then redefine `func f`): changing `a` gives flags `[false, false, true]`
  (`b=6`, `c` reused); redefining `f` gives flags `[false, false, false, false, true]` (`y=27`
  through `g` and `h`, `z` reused).

Raw output: [`journey-j07-j08.log`](journey-j07-j08.log) and the seven-journey run
[`journey-all.log`](journey-all.log).

## 8. Tests — the requested concrete cases

`cargo test -p lovelace-desktop incremental` → **25 passed, 0 failed**
([`incremental-tests.log`](incremental-tests.log)). The end-to-end tests run the real engine behind
a counting wrapper; the counter is the falsifier for INC-008 ("reuse" must mean *not evaluated*).

| Requirement | Test | Observed |
|---|---|---|
| 3 statements, run twice: 0 then 3, identical values | `three_statements_run_twice_reuse_zero_then_three_with_identical_values` | first run flags `[false,false,false]`, 3 evals; second `[true,true,true]`, **0 evals**, results `["2","3","9"]` and the whole variables table identical |
| Edit the last of three independent statements → 2 reuse, 1 recompute | `editing_the_last_of_three_independent_statements_reuses_two` | flags `[true,true,false]`, `reused=2`, **1 eval**, results `["2","5","9"]` |
| Change `a` → `b = a + 1` recomputes transitively, `c = pi(100)` reused | `changing_a_recomputes_b_transitively_and_leaves_c_reused` | flags `[false,false,true]`, `reused=1`, 2 evals, `b=6`, `c` identical with 100 places |
| Redefining a user function invalidates call sites and callers | `redefining_a_user_function_invalidates_call_sites_and_their_callers` | unchanged re-run `[false,false,true,true,true]` (2 evals); redefining `f` `[false,false,false,false,true]`, `y=26`, `q=11`, `z` reused |
| `print`/`plot`/`setprecision` never reuse | `print_plot_and_setprecision_never_reuse` | both runs `[false,false,false]`, 3 evals each, output `["one"]` once, plot captured |
| A precision change reuses nothing | `a_precision_change_reuses_nothing` | 1000→20: `[false]`, 1 eval, 19 fractional digits; back to 1000: `[true]`, 0 evals, same 1000-digit value |
| Reuse count is exactly the cache hits | `reuse_counts_are_exactly_the_cache_hits` | `reused` equals the number of `true` flags every run; 4 evals/0 reuse, then 0 evals/4, then 1 eval/3; entry count 4→5 |
| Never claim a reuse that could not be applied | `an_engine_that_cannot_apply_a_cached_value_never_reports_reuse` | 3 evals, 0 reuse, identical values |
| Run All Fresh yields the same values | `run_all_fresh_clears_the_cache_and_yields_the_same_values` (+ workbench `run_all_fresh_clears_the_cache_and_keeps_the_values`) | `reused` 3→0, 3 evals, same variables |
| The offline (fake) engine reuses without inventing `_` | workbench `the_fake_engine_reuses_without_inventing_a_result_variable` | second run flags `[true,true]`, 2 reused, variables table identical to the fresh run (no `_`) |
| Symbolic commit equality | `a_symbolic_result_reuses_with_the_same_projection_and_inspection` | 0 evals on the re-run; display, structured projection and inspection identical |

Engine-free tests cover normalization, the string/comment scan, assignment-vs-comparison,
non-memoizable control flow, direct and transitive effect detection, the mutation guard, the
transitive body reads, the definition signature, key input sensitivity (value, precision, UNDEF),
function revisions, and the capacity bound.

### The metamorphic test

`twenty_random_edits_match_a_fresh_run` ([`metamorphic.log`](metamorphic.log)) builds 20 sources
with a fixed xorshift seed: the first edit changes only the last statement (J-07's shape), then
each edit replaces/inserts/removes a statement drawn from a catalogue of definitions, dependent
arithmetic, effects and a user-function chain. The incremental document runs after every edit; the
fresh document receives the same 20 edits (so the same revision, 20) and runs once with an empty
cache. It asserts the second incremental run actually reused (`>=1`), that reuse happened at all
over the 20 runs, and that the final observations are equal:

```
metamorphic: 20 edits, 19 reused statements, final revision 20
test result: ok. 1 passed; 0 failed
```

The compared object is `Observation { result, result_structured, variables (name, kind, display),
variables_legacy, committed, total, revision, diagnostic, logs }`. Logs are the final run's lines
(kind, statement index, text) — wall-clock milliseconds are masked (`in <elapsed>`) and the run id
in `Run queued` is masked (`<run>`); everything else, including every `[progress]`/`[stdout]`/
`[result]`/`[diagnostic]` line, is compared verbatim.

## 9. Suite summary

```
# default features (real engine)
cargo test -p lovelace-desktop --manifest-path rust/Cargo.toml
test result: ok. 382 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.35s

# offline build (fake engine)
cargo test -p lovelace-desktop --no-default-features
test result: ok. 344 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.61s

cargo fmt -p lovelace-desktop -- --check        # clean
--gui-selftest                                  # GUI_SELFTEST summary checks=19 failed=0
--journey (7 ids)                               # JOURNEY summary journeys=7 failed=0
```

Unedited logs: [`cargo-test.log`](cargo-test.log),
[`cargo-test-nodefault.log`](cargo-test-nodefault.log), [`fmt-check.log`](fmt-check.log),
[`gui-selftest.log`](gui-selftest.log), [`journey-all.log`](journey-all.log),
[`journey-j07-j08.log`](journey-j07-j08.log), [`incremental-tests.log`](incremental-tests.log),
[`metamorphic.log`](metamorphic.log), [`build.log`](build.log).

### Plot tests and APPDATA

A successful `plot` statement needs a writable plot directory. The effect test points the run's
engines at `%TMP%\lovelace-desktop-incremental-plots` through the engine's own `set_plot_dir`
seam; the harness redirects `TMP`/`TEMP` to `rust\target-desktop\tmp`, so plot writes stay inside
the workspace. Plot statements under the shipped workbench need the app-scoped `%APPDATA%` path
outside the process working directory (PLF-006), so the journey runs above execute from
`round-35\work` with `APPDATA=round-35\appdata`; without that redirect J-09's plot write is denied
by the sandbox (the same environmental limitation round 34 documented), and J-07/J-08 do not need
it.

## 10. Honest notes / not done

- **INC-009 is API-level.** `ExecutionHost::clear_cache` and `Workbench::clear_incremental_cache`
  implement "Run All Fresh" and are tested, but the window has no toolbar button wired to it yet;
  `clear_incremental_cache` carries `#[allow(dead_code)]` for that reason. No journey drives it.
- **INC-011 is outside INC-001..INC-010.** A deterministic 4096-entry cap with clear-on-full is
  present as a safety measure; there is no formal memory budget or eviction policy beyond it.
- **Memory-only, per session.** The cache does not persist across application restarts, and a
  document id's slot lives as long as the `ExecutionHost`; closing a tab does not currently drop
  its cache.
- **Quick eval shares the document cache.** It submits a one-line source under the document's id,
  revision and precision; keys still require identical source and typed inputs, and the cached
  value is applied to a fresh engine, so the shared slot cannot change a result, but it can hold
  quick-eval entries.
- **Dependency extraction is the documented token-scan over-approximation** (§3); it is not the
  engine parser. A newly added stateful builtin that is not in `SIDE_EFFECT_BUILTINS` would be
  treated as pure, exactly like the reference's allow-list exposure; unknown callables are safe
  (effectful).
- **`docs/goal-desktop-v4/journeys.md` was not edited**: this round's scope is `round-35/`, and the
  journey status rows are the gate's to update from this evidence.
- No window screenshot/UIA artifact was captured for J-07/J-08; they are headless journey modes
  (the run's reuse flags and values are the observable), and the window UI does not render a
  reuse counter yet.

## 11. Reproduce

```powershell
$env:RUSTUP_HOME="$repo\.toolchain\rustup"; $env:CARGO_HOME="$repo\.toolchain\cargo"
$env:CARGO_TARGET_DIR="$repo\rust\target-desktop"; $env:TMP="$repo\rust\target-desktop\tmp"; $env:TEMP=$env:TMP
& "$repo\.toolchain\cargo\bin\cargo.exe" test -p lovelace-desktop --manifest-path "$repo\rust\Cargo.toml"

$env:APPDATA="$repo\docs\goal-desktop-v4\round-35\appdata"
Push-Location "$repo\docs\goal-desktop-v4\round-35\work"
& "$repo\rust\target-desktop\debug\lovelace-desktop.exe" --journey J-07 --journey J-08
& "$repo\rust\target-desktop\debug\lovelace-desktop.exe" --journey J-02 --journey J-03 --journey J-07 --journey J-08 --journey J-09 --journey J-10 --journey J-15 --out "$repo\docs\goal-desktop-v4\round-35\journey-out"
Pop-Location
```
