# Round 39 — falsify the verification system (GATE-001..012)

Status: **complete — 12/12 detected, 12/12 reverted trees pass**. The run was made against the
committed tree `05f5426` ("Parity round 39: Natural/Real API surface, symbolics residue"), with a
clean `git status` except the untracked `cryptanalysis/` directory. The parent's pre-run baseline
was workspace 4,808 tests green; this lane re-ran corpus (214/214), fixtures (20/20), CLI (16/16)
and manifest green before the harness, and re-ran the full workspace suite after it.

Command (exit 0):

```
powershell -NoProfile -ExecutionPolicy Bypass -File parity/mutate.ps1
```

## 1. Harness changes

`parity/mutate.ps1` now:

- accepts case filters on the `corpus` and `cargo-test` gates as well as the existing
  `fixtures`/`cli` filters (`corpus:lang/020`, `cargo-test:<test-name>`);
- splits a gate name on the **first** colon, so a filter may itself contain `::`;
- fails closed when a filter matches nothing: a filtered corpus gate that reports `0 passed` or a
  filtered cargo-test run that reports `0 passed` returns exit 2 instead of a vacuous pass;
- restores the target with `ReadAllBytes`/`WriteAllBytes`, not a text round-trip. A text round-trip
  can rewrite a BOM or a line ending; that made `parity/rust-tests.json`'s `sourcesSha256` stale
  even though the gate still passed. Byte-exact restore keeps the reverted tree byte-identical;
- keeps the existing fail-closed rule: a source/file gate that is red **before** the mutation is
  reported as `NOT VALIDATED`, never as detected.

The first full attempt exposed the line-ending defect honestly: GATE-011 came back
`NOT VALIDATED: manifest was already red before the mutation` and was not counted as detected. The
byte-exact restore fixed the cause; the retry below is the measured result.

## 2. The twelve mutations

Each anchor was re-checked against the committed tree with a literal single-occurrence match before
the run (all twelve: exactly one). Multi-line anchors are quoted in full in `parity/mutations.json`;
the table shows their first line.

| ID | Fault | Anchor (target :: literal) | Gate | Detected | Restored |
|---|---|---|---|---|---|
| GATE-001 | decimal period boundary at the computation cap | `rust/lovelace-real/src/lib.rs` :: `} else if position >= max_digits {` → `>` | `cargo-test:the_reference_truncates_a_period_that_closes_exactly_at_the_cap` | yes | yes |
| GATE-002 | numeric widening rule | `rust/lovelace-lang/src/builtins.rs` :: `let target = ra.max(rb);` → `ra.min(rb)` | `corpus:lang/020` | yes | yes |
| GATE-003 | Unicode source position | `rust/lovelace-run/src/main.rs` :: `let upto = units.len().min(position);` → `position + 1` | `fixtures:error_envelope` | yes | yes |
| GATE-004 | envelope field removal | `rust/lovelace-run/src/main.rs` :: `("protocolVersion", Json::Num(...))` → `protocolVersionMoved` | `fixtures:symbolic` | yes | yes |
| GATE-005 | diagnostic code | `rust/lovelace-run/src/main.rs` :: `err.code.to_string()` → `"EvalError"` | `fixtures:error_envelope` | yes | yes |
| GATE-006 | symbolic canonical ordering | `rust/lovelace-symbolics/src/lib.rs` :: `canonical_order` rank comparator `ra.cmp(&rb)` → `rb.cmp(&ra)` | `fixtures:symbolic` | yes | yes |
| GATE-007 | final-digit rounding | `rust/lovelace-real/src/lib.rs` :: `let frac: String = frac.chars().take(digits).collect();` → increment the retained last digit | `corpus:sym/080` | yes | yes |
| GATE-008 | duplicated print output | `rust/lovelace-run/src/main.rs` :: `let output = engine.take_output(); … fields.push(("variables",` → `output.extend(output.clone())` | `fixtures:print_purity` | yes | yes |
| GATE-009 | builtin registry metadata | `rust/lovelace-lang/src/registry.rs` :: `abs` descriptor's `summary: Some(...)` → `summary: None` | `cli:RUN-008` | yes | yes |
| GATE-010 | precision cap | `rust/lovelace-lang/src/builtins.rs` :: `let places = digits.min(1000);` → `let places = digits;` | `cargo-test:the_smallest_over_cap_request_is_marked_too` | yes | yes |
| GATE-011 | deleted ported test | `parity/rust-tests.json` :: `tests::add_GivenOpposites_ReturnsNonNegativeZero` → `…_REMOVED` | `manifest` | yes | yes |
| GATE-012 | empty corpus | `invoke` :: `node parity/diff/run.mjs --cases 0` | `diff-empty` | yes (exit 2) | yes (no tree change) |

GATE-012 is an `invoke` mutation: the invocation *is* the fault and there is no tree to mutate or
restore. The harness counts it as restored; the table discloses that the restoration is vacuous.

## 3. Observed summary line

```
mutations: 12/12 detected; 12 reverted trees pass
  GATE-001 detected=True restored=True - cargo-test:the_reference_truncates_a_period_that_closes_exactly_at_the_cap exit 101
  GATE-002 detected=True restored=True - corpus:lang/020 exit 1
  GATE-003 detected=True restored=True - fixtures:error_envelope exit 1
  GATE-004 detected=True restored=True - fixtures:symbolic exit 1
  GATE-005 detected=True restored=True - fixtures:error_envelope exit 1
  GATE-006 detected=True restored=True - fixtures:symbolic exit 1
  GATE-007 detected=True restored=True - corpus:sym/080 exit 1
  GATE-008 detected=True restored=True - fixtures:print_purity exit 1
  GATE-009 detected=True restored=True - cli:RUN-008 exit 1
  GATE-010 detected=True restored=True - cargo-test:the_smallest_over_cap_request_is_marked_too exit 101
  GATE-011 detected=True restored=True - manifest exit 1
  GATE-012 detected=True restored=True - diff-empty exit 2
```

The cargo-test exit 101 is a failing test run (the injected fault), not a build failure: the release
build of the mutated tree succeeds before each gate is run.

## 4. Anchors that changed, and why

The declarations inherited from the earlier round were not simply extended: three were retargeted
because their original sites could not affect the named gate, and one stale anchor was refreshed.

| Mutation | Earlier declaration | Why it changed |
|---|---|---|
| GATE-002 | `rust/lovelace-lang/src/value.rs` :: `let target = a.max(b);` | `value.rs` is not in the crate's module graph (no `mod value;`); it is not compiled and does not appear in cargo's dep-info. The anchor occurred once but the mutation could not affect any gate. Retargeted to the live widening rule in `builtins.rs::numeric_pair`. |
| GATE-006 | `terms.sort_by(term_order);` (reversed) | `term_order` controls display order, while the fixture compares `canonical` text, which is produced by `canonical_order`. Reversing the display sort left `fixtures:symbolic` green. Retargeted to the `canonical_order` rank comparator. |
| GATE-007 | `truncate_frac` in `transcendental.rs` (+1 to the kept magnitude) | `evalf(sqrt(2), 5)` receives an already-evaluated `Real` argument, so it renders through `Real::to_decimal_fixed`, not `truncate_frac`. The mutation was unobservable on the chosen corpus case. Retargeted to the final-digit step of `to_decimal_fixed`. |
| GATE-008 | the old `take_output` anchor | The source block had been rewritten; the old 3-line anchor no longer occurred. Refreshed to the current `success_envelope` block. |

GATE-001, GATE-009 and GATE-010 are new declarations. GATE-003/004/005/011/012 keep their original
anchors, re-verified as single occurrences in the committed tree.

## 5. Restored-tree verification

After the `12/12` run, with no source mutation outstanding:

| Check | Result |
|---|---|
| `git status --porcelain` | no `rust/` change; this run left `parity/mutate.ps1`, `parity/mutations.json`, this report and untracked `cryptanalysis/` (a concurrent porting lane later updated `parity/manifest-map.json`; see note) |
| `node parity/tools/gate-manifest.mjs` | PASS (`sourcesSha256` fresh) |
| `node parity/tools/gate-fixtures.mjs` | PASS, oracle 20/20, rust 20/20 |
| `node parity/tools/gate-cli.mjs` | PASS, CLI 16/16 |
| `cargo run --release -p lovelace-corpus` | PASS, corpus 214/214 |
| `cargo test --workspace` | PASS, 4,808 passed / 0 failed (exit 0) |

After these checks a concurrent porting lane added round-39 Natural mappings to
`parity/manifest-map.json` (last write 15:47:43, after the mutation run finished at 15:44:49). That
file is not written by `mutate.ps1`; `gate-manifest` was re-run green with the new mappings present.

## 6. Fail-closed evidence

- Every source/file row runs the target gate **before** mutating. If that pre-check is non-zero, the
  row is printed as `NOT VALIDATED` and `detected` stays false. This was observed live: during the
  first full attempt GATE-011 reported `NOT VALIDATED: manifest was already red before the mutation`
  and the summary counted it as `detected=False`, not as a detection.
- A filtered `corpus` or `cargo-test` gate that matches zero cases now exits 2, so a filter typo
  cannot become a vacuous green precondition.
- The restore step runs in `finally`, then reruns the same gate; `restored` is true only when the
  reverted tree passes it. The summary's `12 reverted trees pass` is the harness's own count; the
  GATE-012 caveat above is stated explicitly rather than hidden in that count.
