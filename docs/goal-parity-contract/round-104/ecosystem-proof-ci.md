# Round 104 — ecosystem inventory, proof apparatus, and release CI

Rows in scope: ECO-003, ECO-004, ECO-007, ECO-008, PROOF-002, PROOF-003, PROOF-004, REQ-010,
RUST-009, RUST-010, TEST-004.

This lane inherited five artifacts from the previous lane (`docs/parity/ecosystem.md`,
`ecosystem-api.json`, `knowledge-graph-v1.schema.json`, `numeric-representation.md`,
`proof-theorem-map.md`) and the Nat half of the exhaustive proof tests. It did **not** regenerate
them for completeness: it verified them against the tree, fixed the drift it found, and built the
pieces that were missing (the Int half of PROOF-004, the oracle-artifact record, the release-targets
record, the Rust CI workflow, and a reproducibility sequence). Every number below is from a command
that was run in this round; the transcripts are `rust/target/tmp/reproduce.log`,
`parity/build/rust-binaries.json`, `parity/build/reproduce.json` and `parity/build/oracle-verified.json`.

## 1. Row-by-row

| Row | Produced | Executed | Observed |
|---|---|---|---|
| ECO-003 | `docs/parity/ecosystem.md` §1 + `ecosystem-api.json` (re-verified, regenerated after drift) | `node parity/tools/ecosystem-inventory.mjs`; `--check` | 19 projects, 134 `.cs` files, 367 externally visible types, 1 890 members (876 methods, 104 ctors, 418 props, 47 fields, 4 events, 101 operators, 0 indexers, 257 enum values, 83 record props), 0 unclassified (ceiling 60); `--check` → `PASS: ecosystem artifacts are current` |
| ECO-004 | `ecosystem.md` §2 (14 curated consumer classes CON-001..CON-014, each with an evidence file and a needle the generator requires) | same generator run | 128 tracked scripts/projects reference a C# entry point (docs 101, harnessability-bench 12, parity 7, harness 2, .github 1, 5 project files); the curated set is unchanged, the scanned count moved 126 → 128 because this round's two new parity tools also name an entry point. Drift was detected by `--check` and regenerated, not silently accepted |
| ECO-007 | `ecosystem.md` §4: the explicit decision — **no .NET consumer requires a C ABI, FFI bridge or C# wrapper; none is built, none is needed, none is planned** | generator's interop scan (all C# for `DllImport`/`LibraryImport`/`NativeLibrary`; all Rust for `extern "C"`/`cdylib`/`staticlib`/`no_mangle`) | 0 C# P/Invoke matches, 0 Rust C-ABI matches (`"interopMatches": 0` in the generator summary); justification cites CON-001/005/006/007/009/012 (all process/file-level) and FMT-001..009 |
| ECO-008 | `ecosystem.md` §3: 9 persisted formats, each with version, writer, reader, Rust reader and a decision + migration/readability text; `knowledge-graph-v1.schema.json` | `node parity/tools/format-check.mjs --check` | 5 readable by Rust (runner envelope, MathIR `#!mathir 2`, symbolic `#!lovelace-sym 1`, plot SVG, repo corpus data), 2 migration path (knowledge-graph JSON v1, Knowledge.Run CLI JSON), 2 no obligation (Studio localStorage, Studio in-memory sessions); validator → `PASS: 1 graph file(s) conform to knowledge-graph v1` (sha256 `d445672b…`, 217 668 bytes, seed 20240617) |
| PROOF-002 | `docs/parity/proof-theorem-map.md` (7 theorem groups mapped to Rust boundaries and named tests, with the algorithm-/contract-/proof-internal classification) + new `parity/tools/verify-doc-citations.mjs` | `node parity/tools/verify-doc-citations.mjs` | 4 documents, 19 links, 27 cited paths, **29 test citations**, 0 failures → `PASS: every cited path, link and test resolves against the tree`. The checker found the two dangling links to `.github/workflows/rust-ci.yml` (`proof-theorem-map.md:116`, `numeric-representation.md:116`); the workflow was created, and both now resolve |
| PROOF-003 | `docs/parity/numeric-representation.md` (lean model → limb/decimal correspondence table, intermediate-width argument, explicit "what the model does not cover") | citation checker + the Nat/Int proof runs | all 27 paths and 29 test citations resolve; the document's own reproduce block now names both crates |
| PROOF-004 | `rust/lovelace-nat/src/proof_small_domain.rs` (7 tests, inherited, verified) **plus new** `rust/lovelace-int/src/proof_small_domain.rs` (6 tests) and the `#[cfg(test)] mod proof_small_domain;` wiring in `rust/lovelace-int/src/lib.rs`; both wired as named CI steps | `cargo test -p lovelace-nat proof_`; `cargo test -p lovelace-int proof_`; `cargo clippy -p lovelace-int --all-targets -- -D warnings`; `cargo fmt -p lovelace-int` | nat: `7 passed; 0 failed` in 0.39 s. int: `6 passed; 0 failed` in 0.21 s — every `a,b` in `[-64,64]` (16 641 pairs) against an `i128` oracle for `+`, `-`, `*`, `div_rem_signed` and ordering, ring laws over 35 937 triples, and `2^64 ± 1` products against decimal constants. clippy exit 0, fmt exit 0 |
| REQ-010 | `parity/tools/reproduce.ps1` (13-step sequence), `.github/workflows/rust-ci.yml`, `parity/build/reproduce.json` | `pwsh -NoProfile -File parity/tools/reproduce.ps1` end to end on this machine | see §2: 11 PASS, 2 FAIL, and both failures are pre-existing `rust/lovelace-desktop` lint/format violations outside this lane's scope (§3) |
| RUST-009 | `docs/parity/release-targets.md`: supported-target table, pinned build procedure, artifact hashes, and the measured reproducibility evidence | two builds of the same sources with and without `-C link-arg=/Brepro`, one in a second target directory, one after `cargo clean` | with `/Brepro` all three builds are byte-identical: `lovelace-run.exe` 3 885 056 B `291d85fe73219a16f51f4ccddf8f7f3c4ca129f0e435007b66160c2d1581308d`, `lovelace-corpus.exe` 3 627 520 B `4ae0aa5631081e919a483993ed7d48b7fc593c9f5c59f70de520af5b30c27435`; without the flag a clean relink changes both hashes (`f7ae55a2…` → `8ab7d547…`, `ceadf6c6…` → `730cd4d9…`). Linux/x86_64 is CI-configured but **not** verified on this machine (no Linux toolchain installed) — recorded as such, no hash invented |
| RUST-010 | `parity/tools/ci-binaries.mjs` (inherited, **defect fixed**: its corpus-summary regex expected `corpus: n/m` while the binary prints `corpus: N passed, F failed, T total`, so the gate failed on its own wording) + `.github/workflows/rust-ci.yml` `release-binaries` job | `node parity/tools/ci-binaries.mjs --locked` against the release binaries | `build: PASS`; `binary: lovelace-run PE … sha256=291d85fe…`; `binary: lovelace-corpus PE … sha256=4ae0aa56…`; `entry point: PASS (protocolVersion=1, display=1024)`; `CLI matrix: 16/16`; `oracle: 20/20, rust: 20/20`; `corpus: 214 passed, 0 failed, 214 total`; `PASS: release binaries exercised through their published entry points` |
| TEST-004 | `docs/parity/oracle-artifacts.md`: pinned oracle metadata, artifact hash, all 20 fixture inputs + goldens with bytes and SHA-256, CLI/corpus/graph inputs, ledger hashes, and the amendment log AMD-000..AMD-002 | `parity/build-oracle.ps1` (full build + verify) and `-Verify`; `gate-fixtures`; `gate-cli` | pin `1cc28f2…`, 36 source dirs, `dotnet 10.0.103`; `oracle: pinned sources clean`; `oracle: runner envelope ok=true protocolVersion=1`; DLL 275 968 B `a8e4ac38…`; 0 modified `.cs` files in the working tree; **three builds of the same pinned sources produced three hashes** (`f292249f…` 11-10, `383527d8…` 11-12, `a8e4ac38…` 11-13) — recorded as provenance, with the freeze checked by git against the pin instead |
| ECO-003/004/008 gate wiring | `.github/workflows/rust-ci.yml` steps `Ecosystem inventory drift gate`, `Persisted-format validator`, `Proof-document citations` | `node parity/tools/check-workflow-yaml.mjs` (new; the repo ships no YAML dependency offline) | both workflows parse: `ci.yml: 81 keys, 19 sequence items, 10 block scalars`, `rust-ci.yml: 82 keys, 28 sequence items, 2 block scalars`; `PASS: 2 workflow file(s) are structurally well formed` |

## 2. REQ-010 — the sequence, executed

```
pwsh -NoProfile -File parity/tools/reproduce.ps1
```

The script runs every step even when one is red (the report must show the whole picture) and its
exit code is the number of failed steps. Observed summary on this machine:

```
   toolchain    INFO       0s  cargo 1.99.1 / rustc 1.99.1 (58cf8f9c3 2026-10-13) / rustfmt 1.10.0 / node v24.16.0 / dotnet 10.0.103
   build        PASS       0s  Finished `release` profile [optimized] target(s) (--locked)
   suite        PASS     117s  35 test binaries, 5 856 passed, 0 failed
   proof-nat    PASS       1s  7 passed; 0 failed            (PROOF-004, digit algorithms)
   proof-int    PASS       0s  6 passed; 0 failed            (PROOF-004, signed tower)
   clippy       FAIL      36s  6 errors, all in lovelace-desktop (§3)
   fmt          FAIL       2s  21 diffs, all in lovelace-desktop/src (§3)
   corpus       PASS       0s  corpus: 214 passed, 0 failed, 214 total
   binaries     PASS       7s  entry point PASS; CLI matrix 16/16; oracle 20/20, rust 20/20; corpus 214/214
   ecosystem    PASS       0s  PASS: ecosystem artifacts are current
   formats      PASS       0s  PASS: 1 graph file(s) conform to knowledge-graph v1
   citations    PASS       0s  PASS: every cited path, link and test resolves against the tree
   workflows    PASS       0s  PASS: 2 workflow file(s) are structurally well formed

   failed steps: 2 / 13
   exit code: 2
```

The 2 failures are the two whole-workspace lint steps, and both name only
`rust/lovelace-desktop` files (§3). Everything else — build, the 5 856-test workspace suite, both
exhaustive proof modules, the corpus, the release binaries through their entry points, and all four
documentation/ledger gates — reproduced green from the checkout. The full transcript is
`rust/target/tmp/reproduce.log`; the machine-readable record is `parity/build/reproduce.json`.

## 3. What is red, and why it is not this lane's

`cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets -- -D warnings` fail on
**`rust/lovelace-desktop` only**:

* fmt: 21 diffs, all under `rust/lovelace-desktop/src` (`engine/real.rs`, `execution.rs`,
  `workbench.rs`);
* clippy: 6 errors in `lovelace-desktop` (`large size difference between variants`,
  `field assignment outside of initializer`, four `&Box<T>`).

These are pre-existing on the tree this lane inherited (the desktop lane is committing to
`rust/lovelace-desktop` concurrently), and the scope of this round explicitly excludes that crate.
The same steps with the desktop crate excluded are clean:

```
cargo clippy --workspace --exclude lovelace-desktop --all-targets -- -D warnings   -> exit 0
cargo fmt -p <each of the 11 other packages> -- --check                            -> 0 failures
```

They are reported as failures, not skipped and not converted to passes (constraint C3).

## 4. What remains open

1. **Linux artifact (RUST-009).** `x86_64-unknown-linux-gnu` is built and exercised by the new CI
   job but was not built or hashed here: this machine has only the `x86_64-pc-windows-msvc` target.
   The row is "configured, unverified" until a CI run uploads `rust-binaries-ubuntu-latest`.
2. **Byte-reproducibility of the C# oracle.** Not achievable here without a deterministic-build
   setting; TEST-004 records the artifact hash as provenance and the source pin as the freeze. A
   future amendment that changes pinned sources still needs the user's approval.
3. **Desktop lint/format.** Owned by the desktop lane; until it lands, a whole-workspace fmt/clippy
   gate cannot be green and the sequence reports 2 red steps.
4. **Ledger status fields.** `parity/requirements.json` still carries the previous adjudication for
   these 11 rows; this round produces the evidence but does not edit the ledger (out of scope:
   `parity/*.json|toml` ledgers are frozen for the round). The rows should be re-adjudicated from
   `docs/parity/release-targets.md`, `docs/parity/oracle-artifacts.md`, this report, and
   `parity/build/reproduce.json`.

## 5. Reproduce this round

```powershell
pwsh -NoProfile -File parity/tools/reproduce.ps1              # REQ-010 sequence, 13 steps
node parity/tools/ecosystem-inventory.mjs --check             # ECO-003/004 drift
node parity/tools/format-check.mjs --check                    # ECO-008
node parity/tools/verify-doc-citations.mjs                    # PROOF-002/003
node parity/tools/check-workflow-yaml.mjs                     # workflow structure
cargo test -p lovelace-nat proof_                             # PROOF-004 (7)
cargo test -p lovelace-int proof_                             # PROOF-004 (6)
node parity/tools/ci-binaries.mjs --locked                    # RUST-009/010, writes parity/build/rust-binaries.json
parity/build-oracle.ps1 -Verify                               # TEST-004, pinned sources + DLL hash
```
