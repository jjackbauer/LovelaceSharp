# Deliverables — parity contract

Every row was opened and executed by the orchestrator. An ART row requires at least three supporting
IDs and a "Verified by" line naming the command actually run.

### ART-001: Frozen C# oracle and its verification script

- **Path**: parity/oracle.json, parity/build-oracle.ps1, parity/build/oracle-verified.json
- **Type**: implementation
- **Supporting evidence**: EVD-004, EVD-013, EVD-014
- **Confidence**: High
- **Verified by**: powershell -File parity/build-oracle.ps1 -Verify -> exit 0, commit 1cc28f2..., dll sha256 f292249fbd8d8a20...; an untracked file in a pinned dir made it exit 2; the full run wrote oracle-verified.json
- **Last updated**: 2026-11-10

### ART-002: Complete C# test census

- **Path**: parity/census.json, parity/tools/census/, parity/census-notes.md
- **Type**: dataset
- **Supporting evidence**: EVD-003, EVD-007, OBS-005
- **Confidence**: High
- **Verified by**: two census runs produced byte-identical SHA-256 3cb60e8cc288... (census.json 3,998,378 bytes); gate-census.ps1 PASS with 15 projects / 2,105 methods / 6,209 cases; GoldenEnvelopeTests.EnvelopeMatchesGoldenFixture enumerated exactly 20 rows
- **Last updated**: 2026-11-10

### ART-003: Test disposition manifest and its pilot coverage

- **Path**: parity/manifest.toml, parity/manifest-map.json, docs/goal-parity-contract/round-2/coverage-pilot.md
- **Type**: dataset
- **Supporting evidence**: EVD-008, EVD-017, EVD-018, OBS-010, OBS-011
- **Confidence**: High
- **Verified by**: node parity/gen-manifest.mjs -> 6,209 rows, 20 ported (0.32 %); gate-manifest PASS (structural); two mappings spot-checked in both sources
- **Last updated**: 2026-11-10

### ART-004: Requirement ledger over the v2.0 contract

- **Path**: parity/requirements.json, docs/parity/contract-v2.md, parity/gate-requirements.ps1
- **Type**: document
- **Supporting evidence**: EVD-009, OBS-006, VAL-001
- **Confidence**: High
- **Verified by**: gate-requirements.ps1 PASS with 262 requirement IDs in 25 families, 7 ID-less obligations, 20 met / 13 partial / 229 unmet; a contract/ledger drift makes it exit 1
- **Last updated**: 2026-11-10

### ART-005: C# production project ledger

- **Path**: parity/projects.toml, parity/gate-projects.ps1
- **Type**: dataset
- **Supporting evidence**: EVD-010, OBS-006
- **Confidence**: High
- **Verified by**: gate-projects.ps1 PASS: 19 projects (4 port, 1 consolidated, 8 partial, 6 absent), every non-absent row points at an existing Rust path
- **Last updated**: 2026-11-10

### ART-006: Differential harness

- **Path**: parity/diff/ (run.mjs, generator.mjs, compare.mjs, minimize.mjs, README.md, regressions/)
- **Type**: test-suite
- **Supporting evidence**: EVD-016, EVD-019, EVD-020
- **Confidence**: High
- **Verified by**: --selftest exit 0 with 2 controls and 4 injected corruptions detected; --cases 0 exit 2; a renamed Rust binary made it exit 2; 33 promoted regressions with minimised scripts
- **Last updated**: 2026-11-10

### ART-007: CLI matrix and gate (RUN-001..014)

- **Path**: parity/cli-matrix.json, parity/tools/gate-cli.mjs
- **Type**: test-suite
- **Supporting evidence**: EVD-011, EVD-012, OBS-008
- **Confidence**: High
- **Verified by**: gate-cli.ps1 runs 16 rows through both runners with identical argv/stdin; the oracle's own asserts are checked first (a defective row fails); RUN-013/RUN-014 green, the rest red with named differences
- **Last updated**: 2026-11-10

### ART-008: One-command fail-closed scorecard

- **Path**: parity/run-all.ps1, parity/scorecard.json
- **Type**: implementation
- **Supporting evidence**: EVD-015, EVD-019
- **Confidence**: High
- **Verified by**: powershell -File parity/run-all.ps1 -Cases 3 -SkipMutate wrote scorecard.json with 9 dimensions, exit 1 because 6 were red and 1 skipped; every dimension is a live command, none is inferred
- **Last updated**: 2026-11-10

### ART-009: Independent intent audit

- **Path**: docs/goal-parity-contract/intent-audit.md
- **Type**: report
- **Supporting evidence**: OBS-006, VAL-001, DEC-001
- **Confidence**: High
- **Verified by**: opened the file; reproduced its two structural findings (no WIRE ID in goal.md; every ledger row owner=unassigned/phase=0); the goal and the ledger were corrected in response
- **Last updated**: 2026-11-10

### ART-010: Mutation gate

- **Path**: parity/mutate.ps1, parity/mutations.json
- **Type**: test-suite
- **Supporting evidence**: EVD-015, OQ-010
- **Confidence**: Medium
- **Verified by**: -Only GATE-012 -> exit 0, "1/1 detected; 1 reverted trees pass"; source mutations are declared but NOT VALIDATED while their target gates are red (reported, never counted)
- **Last updated**: 2026-11-10
### ART-011: Protocol recovery — the Rust runner reproduces all 20 golden envelopes

- **Path**: rust/lovelace-run/src/main.rs, rust/lovelace-lang/src/{lib.rs,structured.rs,builtins.rs}, rust/lovelace-symbolics/src/lib.rs
- **Type**: implementation
- **Supporting evidence**: EVD-021, EVD-025, EVD-026, OBS-009, OBS-016
- **Confidence**: High
- **Verified by**: node parity/tools/gate-fixtures.mjs -> "PASS: oracle 20/20, rust 20/20"; the corpus re-run after the same change -> 212/214; the CLI matrix -> 14/16
- **Last updated**: 2026-11-10

### ART-012: 5 000-case differential measurement

- **Path**: parity/diff/runs/20261110T154044Z-seed1/manifest.json, parity/scorecard.json
- **Type**: dataset
- **Supporting evidence**: EVD-016, EVD-019, EVD-020, OBS-020
- **Confidence**: High
- **Verified by**: run-all -Cases 5000 -> D4 115.6 s, 5 000 cases, 3 185 match / 1 815 mismatch / 0 harness errors; per-category tables recorded
- **Last updated**: 2026-11-10
