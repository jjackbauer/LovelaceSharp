# Differential gate — round 1 (parity/diff)

**Delivered; the gate is red.** Final measurement (2026-11-10T14:49Z): 20 generated cases →
**9 match / 11 mismatch / 0 error**, harness exit **1**. Selftest exit **0**. `--cases 0` exits
**2**. The determinism pair is byte-identical.

The Rust runner was not modified by this session and no C# source was touched.

## 1. The `--cases` falsification, answered

Observed by the orchestrator: `--cases 3 --seed 1` printed a 25-row table. That was real, and the
ambiguity was real. The run generated **3** cases and replayed **22** stored regressions; the
header split the two, but the single category table summed both ledgers and its `TOTAL` row did
not say so. `--cases` was never per-category — the *reporting* conflated the ledgers.

Fixed in three places:

1. **`--cases N` is asserted exact.** If the generator yields anything other than N (with no
   `--filter` and not `--regressions`), the harness exits 2 (run.mjs:642-645). No floor, no
   rounding, no silent truncation.
2. **Two ledgers, printed separately.** The console prints `EXECUTION LEDGER`, then
   `generated corpus by category` (ending in `GENERATED`) and
   `regressions replayed by category` (ending in `REPLAYED`) (run.mjs:776-830).
3. **Artifacts split.** `manifest.json` keeps `counts`/`categories` for the generated corpus only
   and adds `regressionCounts`; `monitor.json` has separate `generated` and `regressions` blocks;
   the ledger also reports `duplicatesSkipped` (run.mjs:343-410).

Observed now, the same command the orchestrator ran:

~~~powershell
node parity\diff\run.mjs --cases 3 --seed 1 --no-minimize
~~~

~~~
PARITY DIFF  seed=1  generated=3  replayed=26  total=29  concurrency=6
  oracle C:\Users\ricar\dev\LovelaceSharp\Lovelace.Run\bin\Release\net10.0\Lovelace.Run.dll
         sha256=f292249fbd8d8a20e60e48942992a141f6315c84e14f291ecb53b453fde4a74a
  rust   C:\Users\ricar\dev\LovelaceSharp\rust\target\release\lovelace-run.exe
         sha256=88f008da93dc199a03f72a5e559380534d5b027ab57ac431c5cc7929b0aef9d8
  normalize: revision, elapsed, elapsedTime, timings -> <volatile>

EXECUTION LEDGER
  generated cases (--cases 3): 3
  replayed regressions: 26 (3 duplicate of a generated case skipped)
  total executions: 29

generated corpus by category   cases  match  mismatch  error
  arithmetic       1      1         0      0
  arrays           1      1         0      0
  symbolic         1      1         0      0
  GENERATED        3      3         0      0

regressions replayed by category   cases  match  mismatch  error
  arithmetic       4      4         0      0
  precision        3      0         3      0
  arrays           3      3         0      0
  symbolic         4      2         2      0
  records          5      0         5      0
  errors           4      0         4      0
  syntax           2      0         2      0
  registry         1      0         1      0
  REPLAYED        26      9        17      0

regressions: discovered=29 replayed=26 stillMismatching=17 resolved=9 new=0
run dir: C:\Users\ricar\dev\LovelaceSharp\parity\diff\runs\cases3
harness exit: 1
CASES3_EXIT=1
~~~

`--cases 3` now yields exactly 3 generated cases; the 29 is 3 + 26 replayed regressions, and the
ledgers are labelled. In `--regressions` mode generation is disabled and the ledger says
`generation disabled` rather than echoing an unused `--cases`.

## 2. What was built

| File | Lines | Role |
|---|---|---|
| `parity/diff/run.mjs` | 770 | CLI, worker pool, artifacts, regression replay, selftest |
| `parity/diff/generator.mjs` | 245 | deterministic 8-category generator, corpus hash |
| `parity/diff/compare.mjs` | 204 | 4-key normalisation, structural diff, classification |
| `parity/diff/minimize.mjs` | 91 | ddmin over lines then tokens, 40-evaluation cap |
| `parity/diff/README.md` | 174 | the harness's own contract, including exact `--cases` semantics |
| `parity/diff/regressions/` | 29 files | promoted mismatches (20 seed 1, 5 seed 7, 4 seed 3 from another invocation) |

Dependency-free: only `node:child_process`, `node:crypto`, `node:fs/promises`, `node:path`
(run.mjs:10-13, compare.mjs:14, generator.mjs:13, minimize.mjs). No package manager, no network.

## 3. Provenance — and a moving target

| Item | Value | Source |
|---|---|---|
| Repo HEAD | `1cc28f255eca6029849f880949fc7be1d40ab8af` (branch `rust`) | `git rev-parse HEAD` |
| Oracle sha256 (final) | `f292249fbd8d8a20e60e48942992a141f6315c84e14f291ecb53b453fde4a74a` | final-20 manifest.json |
| Rust sha256 (final) | `88f008da93dc199a03f72a5e559380534d5b027ab57ac431c5cc7929b0aef9d8` | final-20 manifest.json |
| Node / .NET | `v24.16.0` / `10.0.103` | `node --version; dotnet --version` |

Both binaries were **present**; no build was performed and none was needed.

**The runners changed under this session** — the single most important caveat:

| Time (UTC) | Event | Oracle sha | Rust sha |
|---|---|---|---|
| 14:43 | first 20-case baseline | `0e617cff…` | `a29bba64…` |
| 14:44 | oracle DLL rebuilt, no `.cs` change | `f292249f…` | `a29bba64…` |
| 14:46 | Rust rebuilt (`structured.rs` added by another agent) | `f292249f…` | `4c74b1c3…` |
| 14:47 | Rust rebuilt again | `f292249f…` | `88f008da…` |

The first baseline measured **20/20 mismatch** (17 first differences at `$.elapsedTime`,
missing-key, because the Rust envelope carried neither `elapsedTime` nor `timings`). After the
rebuilds the same 20 generated cases measure **9 match / 11 mismatch**, and 12 of the 29 stored
regressions now match. The gate tracked real fixes rather than pinning a verdict.

The oracle DLL's sha256 changed with **no C# source change**, so the .NET build is not
bit-reproducible: a binary sha fingerprint identifies the artifact used, not the source. Pin the
oracle by commit, and treat the manifest sha as provenance for that one run.

The Rust tree carried uncommitted edits from concurrent agents
(`M rust/lovelace-json/src/lib.rs`, `M rust/lovelace-lang/src/lib.rs`,
`M rust/lovelace-lang/src/syntax.rs`, `M rust/lovelace-run/src/main.rs`,
`M rust/lovelace-symbolics/src/lib.rs`, `?? rust/lovelace-lang/src/structured.rs`). None were made
by this session; `git diff -- '*.cs'` is empty.

## 4. Commands and their OBSERVED output

### 4.1 The 20-case sanity pass (requirement 1)

~~~powershell
node parity\diff\run.mjs --cases 20 --seed 1 --out parity\diff\runs\final-20
~~~

~~~
PARITY DIFF  seed=1  generated=20  replayed=9  total=29  concurrency=6
  oracle C:\Users\ricar\dev\LovelaceSharp\Lovelace.Run\bin\Release\net10.0\Lovelace.Run.dll
         sha256=f292249fbd8d8a20e60e48942992a141f6315c84e14f291ecb53b453fde4a74a
  rust   C:\Users\ricar\dev\LovelaceSharp\rust\target\release\lovelace-run.exe
         sha256=88f008da93dc199a03f72a5e559380534d5b027ab57ac431c5cc7929b0aef9d8
  normalize: revision, elapsed, elapsedTime, timings -> <volatile>

EXECUTION LEDGER
  generated cases (--cases 20): 20
  replayed regressions: 9 (20 duplicate of a generated case skipped)
  total executions: 29

generated corpus by category   cases  match  mismatch  error
  arithmetic       4      4         0      0
  precision        2      0         2      0
  arrays           3      3         0      0
  symbolic         4      2         2      0
  records          3      0         3      0
  errors           2      0         2      0
  syntax           1      0         1      0
  registry         1      0         1      0
  GENERATED       20      9        11      0

regressions replayed by category   cases  match  mismatch  error
  arithmetic       1      1         0      0
  precision        1      0         1      0
  arrays           1      1         0      0
  symbolic         1      1         0      0
  records          2      0         2      0
  errors           2      0         2      0
  syntax           1      0         1      0
  REPLAYED         9      3         6      0

first differences (generated):
      3  $.result.structured.fields[0].value.kind (value)
      2  $.category (value)
      2  $.result.display (value)
      2  $.result.structured.denominator (missing-key)
      1  $.code (value)
      1  $.functions (array-length)

regressions: discovered=29 replayed=9 stillMismatching=6 resolved=3 new=0
  resolved (kept, not deleted): g-s7-arithmetic-natural-add-001, g-s7-arrays-vector-sum-001, g-s7-symbolic-diff-polynomial-001
run dir: C:\Users\ricar\dev\LovelaceSharp\parity\diff\runs\final-20
harness exit: 1
SANITY_EXIT=1
~~~

### 4.2 `--selftest` (requirement 2)

~~~powershell
node parity\diff\run.mjs --selftest
~~~

~~~
SELFTEST — falsification of the comparator itself
  control-oracle  PASS  matched, harness exit 0  [control: oracle vs itself]
  control-rust    PASS  matched, harness exit 0  [control: rust vs itself]
  flip-ok         PASS  mismatch at $.ok (value), harness exit 1  [corrupt: flip ok]
  drop-key        PASS  mismatch at $.result.kind (missing-key), harness exit 1  [corrupt: drop result.kind]
  change-scalar   PASS  mismatch at $.result.display (value), harness exit 1  [corrupt: change result.display]
  reorder-array   PASS  mismatch at $.functions[0].category (value), harness exit 1  [corrupt: reorder functions[]]
SELFTEST PASS: 2 controls matched (exit 0), 4 injected corruptions were reported as mismatches (exit 1).
SELFTEST_EXIT=0
~~~

Both directions are proven: the controls fail a comparator that always mismatches, the four
mutations fail one that always matches. Each mutation is a real child invocation of the CLI whose
**exit code 1** is observed by the parent (run.mjs:506-600). It is independent of the runners'
agreement, so it stays meaningful while the gate is red.

### 4.3 `--cases 0` (requirement 3)

~~~powershell
node parity\diff\run.mjs --cases 0
~~~

Message on **stderr**:

~~~
HARNESS ERROR: --cases 0 is an empty corpus; a gate that runs nothing cannot pass.
CASES0_EXIT=2
~~~

### 4.4 Determinism (requirement 5)

~~~powershell
node parity\diff\run.mjs --cases 5 --seed 7 --out parity\diff\runs\det-a --no-minimize
node parity\diff\run.mjs --cases 5 --seed 7 --out parity\diff\runs\det-b --no-minimize
~~~

Both generated the same five cases (ids, args, scripts):

| id | args | script |
|---|---|---|
| `g-s7-arithmetic-natural-add-001` | `["--omit-functions"]` | `8457 + 97084` |
| `g-s7-arrays-vector-sum-001` | `["--omit-functions"]` | `sum([4, 0, 8])` |
| `g-s7-symbolic-diff-polynomial-001` | `["--omit-functions"]` | `x = symbol("x"); diff(x^3 + 6*x^2 + 8*x + 7, x)` |
| `g-s7-records-solve-full-001` | `["--omit-functions"]` | `x = symbol("x"); solve_full(x^3 - x == 0, x)` |
| `g-s7-errors-division-by-zero-001` | `["--omit-functions"]` | `29/0` |

~~~
det-a corpus.json sha256: 4b342419da98f8ec4344525c594f9c1d08b06773bb2383c9c0c04d9ff4aaad36 bytes: 844
det-b corpus.json sha256: 4b342419da98f8ec4344525c594f9c1d08b06773bb2383c9c0c04d9ff4aaad36 bytes: 844
byte-identical: true

manifest corpusSha256 a: 48a4410f8b21a953d8d14192cb682efb95c010f09c80f86c0073e10a9a2b4c3a
manifest corpusSha256 b: 48a4410f8b21a953d8d14192cb682efb95c010f09c80f86c0073e10a9a2b4c3a
manifest hash equal: true
~~~

Byte-identical, not merely equal as JSON (`Buffer.equals` true). The corpus hash is independent of
the binaries, so it was unchanged across three Rust rebuilds.

### 4.5 Regression replay, including resolved cases

~~~powershell
node parity\diff\run.mjs --regressions --no-minimize --out parity\diff\runs\regressions-replay
~~~

~~~
PARITY DIFF  seed=1  generated=0  replayed=29  total=29  concurrency=6
  ...
EXECUTION LEDGER
  generated cases (--regressions mode: generation disabled): 0
  replayed regressions: 29
  total executions: 29

regressions replayed by category   cases  match  mismatch  error
  arithmetic       5      5         0      0
  precision        3      0         3      0
  arrays           4      4         0      0
  symbolic         5      3         2      0
  records          5      0         5      0
  errors           4      0         4      0
  syntax           2      0         2      0
  registry         1      0         1      0
  REPLAYED        29     12        17      0

regressions: discovered=29 replayed=29 stillMismatching=17 resolved=12 new=0
  resolved (kept, not deleted): g-s1-arithmetic-huge-power-003, ... (12 ids)
REGRESSIONS_EXIT=1
~~~

Twelve stored regressions now match and are **reported, not deleted**. That is DIFF-008 working in
the other direction: inputs that once failed are kept and replayed forever, so a later regression
is visible as a regression.

## 5. Mismatches at 20 cases, per category (requirement 4)

Generated corpus only (`--cases 20 --seed 1`):

| Category | Cases | Match | Mismatch | Error | First difference(s) |
|---|---|---|---|---|---|
| arithmetic | 4 | 4 | **0** | 0 | — |
| precision | 2 | 0 | **2** | 0 | `$.result.structured.denominator` missing-key ×2 |
| arrays | 3 | 3 | **0** | 0 | — |
| symbolic | 4 | 2 | **2** | 0 | `$.result.display` value ×2 |
| records | 3 | 0 | **3** | 0 | `$.result.structured.fields[0].value.kind` ×2, `$.result.display` ×1 |
| errors | 2 | 0 | **2** | 0 | `$.code` ×1, `$.category` ×1 |
| syntax | 1 | 0 | **1** | 0 | `$.category` ×1 |
| registry | 1 | 0 | **1** | 0 | `$.functions` array-length ×1 |
| **TOTAL** | **20** | **9** | **11** | **0** | |

Sources: `parity/diff/runs/final-20/manifest.json` (counts 9/11/0, `regressionCounts` 3/6/0) and
`parity/diff/runs/final-20/monitor.json` (per-category rows). Nothing was suppressed.

## 6. What the gate measured

The gate now discriminates rather than reporting one universal gap. The 11 generated mismatches
span six distinct first-difference paths, and the deeper difference counts range from 1 to 91:

- **Records** diverge inside structured records: `solve_full` and `limit_full` differ first at
  `$.result.structured.fields[0].value.kind` with 21 and 17 differing paths
  (`mismatches/g-s1-records-solve-full-001.json`, `…limit-full-003.json`); `integrate_full`
  differs first at `$.result.display` and has **91** differing paths.
- **Precision**: `$.result.structured.denominator` is missing on the Rust side for rational
  results at both 8 and 16 digits.
- **Errors**: `58/0` differs first at `$.code` (oracle `DivisionByZero` vs Rust
  `EvaluationError`); `nonexistent_fn(2)` differs first at `$.category`.
- **Registry**: `type(42)` runs without `--omit-functions` and differs at `$.functions` by
  **array length** — the populated registry is not reproduced.
- **Syntax**: `% leading comment\n2 + 3` differs at `$.category` — the comment *attempt* is
  rejected with a different error category by the two runners.

**Minimisation**: all 11 generated mismatches were minimised within the 40-evaluation cap; the
observed maximum was **19**. Examples from `runs/final-20/mismatches/`:

| Case | Original | Minimised | Evals |
|---|---|---|---|
| `g-s1-precision-setprecision-8-001` | `setprecision(8); 1/3` | `1/3` | 1 |
| `g-s1-syntax-comment-percent-001` | `% leading comment\n2 + 3` | `%` | 6 |
| `g-s1-records-solve-full-001` | `x = symbol("x"); solve_full(x^2 - 4 == 0, x)` | unchanged (no reduction preserves the path) | 19 |
| `g-s1-errors-division-by-zero-001` | `58/0` | `58/0` (single token) | 0 |

### 6.1 Comparator rules checked directly

The generated corpus always produces JSON, so the `invalid-json`, stderr and exit-code rules were
also exercised directly against `compare.mjs` (imported by a Node process, no runner involved):

~~~js
const mod = await import("…/parity/diff/compare.mjs");
mod.classifyCase({exitCode:0, stdout:"not json at all"}, {exitCode:0, stdout:'{"ok":true}'})
mod.classifyCase({exitCode:0, stdout:'{"ok":true}', stderr:"warning A"},
                 {exitCode:0, stdout:'{"ok":true}', stderr:"different warning B"})
mod.classifyCase({exitCode:1, stdout:'{"ok":false}'}, {exitCode:0, stdout:'{"ok":false}'})
~~~

Observed:

~~~
non-JSON stdout -> invalid-json => {"mismatch":true,"category":"invalid-json","first":{"path":"$","kind":"invalid-json","expected":"not json at all","actual":"<json>"},"stderrBytes":[0,0]}
stderr differs, exit+stdout same -> match => {"mismatch":false,"category":null,"first":null,"stderrBytes":[9,19]}
exit code differs -> exit-code => {"mismatch":true,"category":"exit-code","first":{"path":"$.exitCode","kind":"value","expected":1,"actual":0},"stderrBytes":[0,0]}
normalize: {"nested":{"elapsedTime":"<volatile>","keep":2,"timings":"<volatile>"},"revision":"<volatile>"}
missing volatile key is structural: {"equal":false,"differenceCount":1,"firstDifference":{"path":"$.a.revision","kind":"missing-key",...}}
key order irrelevant: true
~~~

So: a non-JSON stdout is a mismatch carrying the raw prefix, a stderr difference alone is **not** a
mismatch (length still recorded: 9 vs 19 bytes), exit-code divergence outranks envelope
comparison, the four volatile fields are replaced at depth, a *missing* volatile key is still
structural, and object key order is irrelevant.

## 7. Constraints observed

- **No Rust or C# edits by this session.** `git status --porcelain` lists my paths as untracked
  (`parity/diff/`, `docs/goal-parity-contract/`); `git diff -- '*.cs'` is empty. The Rust
  modifications listed in section 3 are concurrent agents' work.
- **Normalisation is exactly the four declared keys** — compare.mjs:11-19 declares them,
  compare.mjs:34-46 applies them. No second exclusion list exists in the tool.
- **No flag the oracle rejects.** `--precision` is not a real flag: the oracle answers
  `Error: Unknown argument '--precision'.` and exits 2 (observed). Precision is exercised through
  `setprecision/pi/e/evalf` scripts. Allowed extra flags are enforced in generator.mjs:30 and
  generator.mjs:209-216.
- **No dependency**, no package manager, no network.

## 8. What I could NOT verify

1. **Missing-binary exit path** (run.mjs:118-129). Testing it would require moving a build artifact
   outside my scope; the branch was not executed.
2. **Per-process timeout path** (run.mjs:141-146). No case came near the 30 s limit.
3. **DIFF-009 volumes.** Only 20 and 5 generated cases were run, not 5 000/PR or 100 000/nightly.
   At roughly 170 ms per case with concurrency 6, I did not measure a full-volume run.
4. **Cross-machine determinism.** The same-machine pair is byte-identical; a second machine was not
   available. Nothing in the generator reads time, locale, environment or the filesystem.
5. **Non-Windows behaviour.** Spawn, paths and timeout handling were exercised on Windows only.
6. **The 40-evaluation ceiling binding.** The maximum observed was 19, so "budget exhausted"
   behaviour is unobserved.
7. **Whether the measured Rust binary matches HEAD.** It was built from a tree another agent had
   modified, and it was rebuilt three times during the session. Results are pinned to
   `88f008da…`; the oracle's sha changed with no source change, so its binary sha is not a source
   fingerprint.
8. **Stability of the regression corpus.** It grew from 25 to 29 files while I worked (four
   `g-s3-*` cases promoted by another invocation of this harness). All counts above are for the
   29-file corpus at 2026-11-10T14:50Z.
