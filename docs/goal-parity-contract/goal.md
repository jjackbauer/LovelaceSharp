# Goal — the enforced parity contract (cycle 8)

## One-sentence outcome

The repository carries a reproducible, fail-closed parity contract — a SHA-pinned C# oracle, a
census of every C# test case with an explicit Rust disposition, a differential gate that compares
the C# and Rust runners on shared generated input, a complete requirement ledger covering every
requirement ID in the v2.0 contract, and a Rust runner that passes all 20 golden protocol fixtures
and all 14 CLI requirements — so that migration completion is demonstrated rather than asserted.

## Why this goal and not "finish the whole migration"

The requester's v2.0 specification is the standing definition of done for the *migration*. This
cycle delivers the *measurement system* that specification §15 Phase 0–2 demands, plus its first
enforced implementation target (§15 Phase 1: protocol recovery). Nothing is removed from the
standing contract: every requirement ID in the specification is recorded in
`parity/requirements.toml` with an explicit status and owner, and every P4/P6 subsystem that this
cycle does not implement appears as an unmet ledger row, not as an absence. §1.1 REQ-005 and §14
HARNESS-001 are satisfied by the ledger itself, which is the artifact that prevents the orchestrator
from converging on a convenient subset.

## Verbatim request (the authoritative contract)

> Proposed specification v2.0 — Migration acceptance contract …
> The central requirement is that the Rust ecosystem must be capable of replacing the entire C#
> LovelaceSharp ecosystem without losing observable functionality, correctness, interoperability,
> or tooling.
> …
> My strongest recommendation: Do Phase 0 before asking DSH to write any more Rust code. … For this
> next cycle, I would make the parity manifest and differential harness the primary deliverables,
> with the runner protocol as the first implementation target.

The full v2.0 text is preserved in `docs/parity/contract-v2.md`; the requirement IDs are extracted
from that file by `parity/gate-requirements.ps1`, so the ledger cannot silently drop one.

## Named mechanisms — disposition (this cycle)

| Mechanism (requester's noun) | Disposition | Where |
|---|---|---|
| Immutable C# oracle commit/tag (REQ-001/TEST-001) | DELIVERED this cycle | `parity/oracle.json`, `parity/build-oracle.ps1` |
| Complete C# test census (TEST-006) | DELIVERED this cycle | `parity/census.json`, `parity/tools/census` |
| `parity/manifest.toml` disposition for every test (TEST-007) | DELIVERED this cycle | `parity/manifest.toml` |
| Every C# production project inventoried (ECO-001/002) | DELIVERED this cycle | `parity/projects.toml` |
| Machine-readable coverage manifest / no silent exclusions (REQ-005) | DELIVERED this cycle | `parity/requirements.toml` |
| 20 protocol golden fixtures | DELIVERED this cycle (target 20/20) | `parity/gate-fixtures.ps1` |
| 14 CLI requirements (RUN-001..014) | DELIVERED this cycle (target 14/14) | `parity/cli-matrix.json` |
| Differential execution, 5 000/PR, 100 000/nightly (DIFF-001..010) | DELIVERED this cycle | `parity/diff/` |
| Fault-injection of the gate itself (GATE-001..012) | DELIVERED this cycle | `parity/mutate.ps1` |
| P4 subsystems: DSP, Statistics, Console, Knowledge, Studio, Proofs mapping | UNMET — ledger rows with owner/phase; not silently absent | `parity/requirements.toml` |
| P6 desktop on Windows/Linux/macOS | UNMET / platform decision pending (OQ-001); ledger row | `parity/requirements.toml` |
| Symbolics 214/214 corpus | UNMET at 212/214 (2 examples); ledger row | `parity/requirements.toml` |
| Performance ≥80 % of reference (PERF) | UNMET this cycle; ledger rows | `parity/requirements.toml` |

**Correction after the independent intent audit (VAL-001).** The first draft claimed "no user-named
mechanism is REMOVED or REPLACED". That claim was false under the audit's delivery rule: a mechanism
with no command in D1–D10 is REMOVED for this cycle, however the prose frames it. The audit found
18 REMOVED and 1 REPLACED.

- **REPLACED (1):** WIRE-001..015 are replaced, for this cycle, by the 20-fixture structural gate
  (D5), which is a proxy for the envelope contract and not an itemised WIRE gate. The WIRE IDs are
  named here so the proxy cannot hide a missing obligation.
- **REMOVED for this cycle (18):** the 214-example corpus gate; the 100 000-case nightly run; the
  twelve property families; the SymPy oracle; the Lean build and proof mapping; performance;
  DSP; Statistics; Console/REPL; Knowledge; the Studio desktop; the desktop platform decision;
  DESK-028; ECO-007; ECO-008; P0–P2 semantic parity (NUM/REAL/CPLX/ARRAY/LANG/ENG/PLUG/MIR); Rust
  engineering quality.

Those removals are *accepted as scope* only because the requester's own §15 order makes this the
Phase 0–2 cycle ("Do Phase 0 before asking DSH to write any more Rust code … make the parity
manifest and differential harness the primary deliverables, with the runner protocol as the first
implementation target"). They remain in the standing objective with an owner and a target phase in
`parity/requirements.json`; they are not silently dropped. DEC-001 records this. The desktop
platform question (OQ-001) is a standing-objective decision, not a cycle-8 blocker.

## Definition of done (each dimension: command + expected observable)

| ID | Dimension | Command | Expected observable |
|---|---|---|---|
| D1 | Oracle frozen | `pwsh parity/build-oracle.ps1 -Verify` | exit 0; prints the pinned 40-hex SHA; refuses a dirty C# tree |
| D2 | Census complete | `pwsh parity/gate-census.ps1` | exit 0; every C# test project present; count reconciles with the 2 087 baseline claim or the difference is explained in the artifact |
| D3 | Manifest + ledgers complete | `pwsh parity/gate-manifest.ps1` | exit 0; one disposition per census id, no orphans, every "ported" row resolves to a Rust test that ran, every C# production project has a row, every requirement ID in contract-v2.md has a status |
| D4 | Differential gate | `node parity/diff/run.mjs --cases 5000 --seed 1` | exit 0; both runners executed; 0 mismatches; run manifest written; exits non-zero if either binary is missing or `--cases 0` |
| D5 | Protocol fixtures (proxy for WIRE-001..015) | `pwsh parity/gate-fixtures.ps1` | exit 0; Rust 20/20 and C# oracle 20/20; every fixture diff names the WIRE id it defends |
| D6 | CLI surface | `pwsh parity/gate-cli.ps1` | exit 0; 14/14 RUN rows match the oracle's observable behaviour |
| D7 | Gates falsifiable | `pwsh parity/mutate.ps1` | exit 0; all 12 GATE injections detected by the expected gate |
| D8 | Reproducible scorecard | `pwsh parity/run-all.ps1` | exit 0 from a clean build state; writes `parity/scorecard.json` with every dimension |
| D9 | Documentation corpus | `cargo run --release -p lovelace-corpus` | exit 0; 214/214 examples match (currently 212/214) |
| D10 | Rust engineering quality | `cargo fmt --check` and `cargo clippy --workspace --all-targets -- -D warnings` | exit 0 for both |

## Hard constraints (gates)

| ID | Constraint | Provenance |
|---|---|---|
| C1 | C# sources are never modified (freeze) | user (REQ-001/TEST-003) |
| C2 | No test may be weakened, skipped, or deleted to reach green | user (TEST-011) + skill |
| C3 | A missing/skipped/unrun check never counts as a pass | user (REQ-008) |
| C4 | Every comparison identifies the oracle SHA | user (REQ-001) |
| C5 | No silent exclusion: unsupported capability is a ledger row with reason | user (REQ-005) |
| C6 | Implementation and its generated tests cannot be the only evidence for a parity claim | user (HARNESS-012) |
| C7 | Native desktop must not open a localhost listener or embed a web runtime | user (DESK-028) — unchanged |

## Preferences (tie-breakers)

- Deterministic tooling in Node/PowerShell over new dependencies; the repo is offline-capable.
- Normalise only the four declared volatile fields (`revision`, `elapsed`, `elapsedTime`, `timings`).
- Prefer exact structural comparison over prose reports.
- Prefer a gate that fails closed over a gate that is conveniently green.

## Stop criteria

- **Done**: D1–D8 pass, each reproduced by the orchestrator, zero Falsified claims in the final
  round, zero P0 open questions.
- **Round cap**: 14. Reaching the cap is not "done"; outstanding dimensions are reported.
- **Blocked**: only after three consecutive rounds blocked by the same named concrete condition.

## Capability probe (performed before accepting any "unavailable")

- `dotnet test` (VSTest testhost) fails in this environment with `OpenProcess → Access is denied`;
  a plain `dotnet exec Lovelace.Run.dll` works. Recorded as CAP-001; the census therefore uses a
  reflection tool rather than `dotnet test --list-tests`, and C# *test execution* is reported as
  unverifiable rather than assumed.
