# Intent audit — parity contract (cycle 8)

**Scope.** `docs/goal-parity-contract/goal.md` (the proposal) against `docs/parity/contract-v2.md` (the authoritative contract), at HEAD `1cc28f255eca6029849f880949fc7be1d40ab8af` on branch `rust`. Repository read-only; only this file was written.

**Framing.** goal.md:11–20 frames this as the measurement cycle plus protocol recovery, not the migration. That explains the removals below but does not authorize them: under the rule below, a mechanism absent from D1–D8 is REMOVED for this cycle regardless of what the standing objective retains.

**Verdict rule.** KEPT — the proposal commits a cycle artifact with a D1–D8 command that exercises the named mechanism. REPLACED BY x — a narrower substitute is the cycle deliverable. REMOVED — no cycle command exercises it. A ledger row, a "deferred to phase N" note, or a restated constraint is not delivery; a mechanism kept only as a ledger row is REMOVED.

**State at the audit point.** The working tree (HEAD `1cc28f2`) holds only `parity/requirements.json` (262 rows; every row `status=unmet`, `owner=unassigned`, `phase=0`), `parity/gen-requirements.mjs`, `parity/gate-requirements.ps1`, `parity/tools/gate-requirements.mjs`, `parity/extract-corpus.mjs`, and `parity/corpus/{lang,symbolics}.json` (111 + 103 = 214 examples). None of the D1–D8 artifacts (`oracle.json`, `census.json`, `manifest.toml`, `projects.toml`, `requirements.toml`, `cli-matrix.json`, `diff/`, `mutate.ps1`, `run-all.ps1`, `scorecard.json`) exists in the working tree. "DELIVERED this cycle" (goal.md:40–52) is therefore a commitment under audit, not a verified fact.

**Result.** KEPT 9 · REPLACED 1 · REMOVED 18 (28 rows). goal.md:54 asserts "No user-named mechanism is REMOVED or REPLACED"; under the rule above, 18 are REMOVED and 1 is REPLACED. goal.md:15 ("Nothing is removed from the standing contract") is true of the contract text only, not of this cycle's delivery.

## Mechanism disposition

| Mechanism | Verdict (KEPT \| REPLACED BY x \| REMOVED) | Proposal location | Citation | Consequence |
|---|---|---|---|---|
| C# oracle freeze (REQ-001; TEST-001–005) | KEPT | goal.md:40 (disposition), 62 (D1), 75 (C1), 78 (C4) | contract-v2.md:33, 396–400 | SHA pin, dirty-tree refusal and build script are gated by D1. TEST-004 (approved amendments) and TEST-005 (archived binaries/build inputs) have no cycle element; TEST-002 (build and execute in CI) has no separate dimension; D1 runs the build/verify script. Standing objective keeps all five. |
| C# test census (TEST-006, 009, 012, 013) | KEPT | goal.md:41 (disposition), 63 (D2), 99–102 (CAP-001) | contract-v2.md:64–66, 404–411 | Reflection census replaces `dotnet test --list-tests` because VSTest fails here (CAP-001, recorded rather than assumed). D2's "count reconciles with 2 087 or the difference is explained" can absorb an undercount: OBS-005 counts 1 904 Fact + 183 Theory attributes against 709 InlineData and 45 MemberData rows, and the contract requires parameterized cases enumerated (contract-v2.md:65–66). TEST-009 detection, TEST-012 per-subsystem coverage and TEST-013 re-freeze completeness are not operationalized by D2. C# test execution stays unverifiable in this environment. |
| parity/manifest.toml (TEST-007, 008, 010) | KEPT | goal.md:42 (disposition), 64 (D3) | contract-v2.md:405–408 | One disposition per census id, no orphans, every "ported" row resolves to a Rust test that ran (TEST-008). TEST-010 (explicit exclusion reasons) is not separately checked, so a non-ported row needs no approval or reason to satisfy D3; TEST-011 (no weakened assertions) is declared as C2 but no D1–D8 gate detects weakening. Completeness inherits the census's case-count risk. |
| C# production project ledger (ECO-001/002) | KEPT | goal.md:43 (disposition), 64 (D3) | contract-v2.md:120–147 | Every project has a row (D3). ECO-001 also requires each row to resolve to Rust modules with tests linked to the C# capabilities; D3 checks presence, not the module/test link. |
| Requirement/coverage manifest over v2.0 (REQ-005; HARNESS-001) | KEPT | goal.md:16–20, 33–34 (extraction), 44 (disposition), 64 (D3) | contract-v2.md:4–5, 37, 498 | All 262 IDs are extractable (parity/requirements.json totals.byFamily), so no ID can silently drop. Obligations without IDs — §12.4 property families, §2 gates, §4 project rows, §16 prose — are outside the extractor. HARNESS-001 is asserted by the artifact, not tested; recording every ID is not delivering every requirement. |
| 214-example documentation corpus (§2, §8.1, §16) | REMOVED | goal.md:51 (UNMET at 212/214; ledger row) | contract-v2.md:54, 274–276, 519, 532 | 212/214 admitted; sym/071 and sym/076 are neither fixed nor recorded as approved deviations (REQ-006), and a ledger row is not approval. Corpus exists (111 lang + 103 symbolics = 214) but no D1–D8 gate runs it. Standing objective keeps 214/214 (Phase 4); goal.md names no owner. |
| 20 protocol golden fixtures (§3) | KEPT | goal.md:45 (disposition), 66 (D5), 86 (four volatile fields) | contract-v2.md:54, 112–116 | D5 requires Rust 20/20 and C# 20/20; 20 fixtures exist. Baseline is 0/20 (OBS-003), so the gate is real. The extra acceptance clauses (runner integration tests reproduced/matched; dsh-protocol.md normative) are not carried — see the WIRE row. |
| WIRE-001–015 JSON envelope / wire protocol | REPLACED BY the 20-fixture structural gate (D5) | goal.md:45, 66, 86 | contract-v2.md:96–110, 112–116 | No WIRE ID appears anywhere in goal.md. WIRE-004 (omitted/null/empty), WIRE-006 (enum types), WIRE-007 (field order), WIRE-013 (escaping), WIRE-014 (truncation), WIRE-015 (artifact failure) have no named assertion. The "Lovelace.Run.Tests integration tests reproduced or matched" clause and the normative dsh-protocol.md are dropped. Standing objective keeps all 15; no owner; the narrower acceptance has no approval. |
| 14 CLI requirements (RUN-001–014) | KEPT | goal.md:46 (disposition), 67 (D6) | contract-v2.md:77–92 | parity/cli-matrix.json with 14/14 rows matched against the oracle's observable behaviour. Coverage is per RUN ID; WIRE-011 exit codes are only implied. |
| Differential harness, 5,000 cases/PR (DIFF-001–010, PR slice) | KEPT | goal.md:47 (disposition), 65 (D4) | contract-v2.md:57, 415–426 | D4 runs both binaries on the generated cases, seed 1, 5 000 cases, 0 mismatches, writes a manifest, and fails non-zero on a missing binary or `--cases 0` (DIFF-010). DIFF-006 minimiser, DIFF-007 saved minimised example, DIFF-008 promotion to regression are absent; DIFF-002/003 generator domain and variation are unspecified. With 212/214 and 0/20 admitted, 0 mismatches is credible only on a generator domain that avoids the known gaps. |
| 100,000-case nightly differential run (DIFF-009 nightly half; §2) | REMOVED | goal.md:47 claims it; 65 (D4) stops at 5 000 | contract-v2.md:58, 425 | The proposal defines no nightly job, schedule, command or acceptance dimension. goal.md:47 asserts the nightly scale, D4 caps at 5 000; the claim is not delivered. Standing objective keeps DIFF-009; no owner/phase. |
| Mutation gate GATE-001–012 | KEPT | goal.md:48 (disposition), 68 (D7) | contract-v2.md:449–462 | D7 requires all 12 injections detected by the expected gate. GATE-001 (decimal period), GATE-002 (widening), GATE-006 (canonical order), GATE-007 (rounding) and GATE-010 (precision cap) target NUM/REAL/SYM behaviour with no gate in D1–D8, so "expected gate" is a proxy or D7 cannot honestly pass. GATE-012 is tested as `--cases 0`, not an empty/incomplete corpus. GATE-011/012 align with the manifest and diff fail-closed. |
| Twelve property families (§12.4) | REMOVED | None — absent from goal.md:36–52 and 60–69 | contract-v2.md:428–434 | goal.md names no IDs, artifact, owner or phase. Because §12.4 has no requirement IDs, the gate-requirements extractor cannot even record it as a ledger row. The standing objective retains §12.4, but the proposal drops it entirely: no ledger row, owner or phase. |
| SymPy differential oracle (SYM-014) | REMOVED | None — goal.md:49 names "Symbolics" only | contract-v2.md:272 | The oracle is described as already existing, so retention is zero-cost; it still has no cycle element. Ledger row only (owner=unassigned, phase=0 at HEAD). Standing objective keeps SYM-014; contract Phase 4; no owner. |
| Lean proofs (PROOF-001, 003, 004) | REMOVED | None — goal.md:49 names "Proofs mapping" only | contract-v2.md:436–443 | goal.md commits no pinned Lean build, representation-correspondence document or exhaustive small-domain CI. Standing objective keeps PROOF-001/003/004; ledger rows only; contract Phase 7; no owner. |
| Proofs mapping (PROOF-002) | REMOVED | goal.md:49 (inside the P4 ledger row) | contract-v2.md:439 | goal.md commits no theorem-to-Rust-property map. Standing objective keeps PROOF-002; ledger row only; contract Phase 7; no owner. |
| Performance PERF-001–010 (≥80% of reference) | REMOVED | goal.md:52 (UNMET this cycle; ledger rows) | contract-v2.md:466–477 | goal.md commits no PERF gate, no 80% measurement (PERF-009) and no warmed/JIT/distribution/environment accounting, although bench harnesses exist in the repo. Ledger rows only; contract Phase 7; no owner. Standing objective keeps PERF-001–010. |
| DSP subsystem (DSP-001–010) | REMOVED | goal.md:49 (UNMET; ledger rows with owner/phase) | contract-v2.md:293–302, 520 | Ledger row only; goal.md commits no 16-builtin port, convolution/DFT/FFT, seeded noise or cancellation work. goal.md asserts "owner/phase" but the only ledger at HEAD records owner=unassigned, phase=0. Standing objective keeps DSP-001–010; contract Phase 5; no owner. |
| Statistics subsystem (STAT-001–004) | REMOVED | goal.md:49 | contract-v2.md:306–312, 520 | Ledger row only; goal.md commits no opt-in Real addition kernel or typed-array dispatch equivalence. Standing objective keeps STAT-001–004; contract Phase 5; no owner. |
| Console / REPL subsystem (CLI-001–007) | REMOVED | goal.md:46, 49 | contract-v2.md:318–324, 520 | Ledger row only; goal.md commits no REPL, history, multiline, commands or retained state. goal.md:46's "14 CLI requirements" are RUN-001–014 (flags), not CLI-001–007 (REPL); the family is otherwise absent. Standing objective keeps CLI-001–007; contract Phase 5; no owner. |
| Knowledge / MGIR subsystem (KNOW-001–010) | REMOVED | goal.md:49 | contract-v2.md:328–337, 520 | Ledger row only; goal.md commits no sampling/reduction/convergence/behavioural graph, no MGIR stdio protocol and no seed-20240617 reproduction. Standing objective keeps KNOW-001–010; contract Phase 5; no owner. |
| Studio desktop IDE (DESK-001–027; Lovelace.Studio) | REMOVED | goal.md:49–50 | contract-v2.md:143, 346–379, 521 | The repo already contains rust/lovelace-desktop (Win32 shell, editor/variables/logs panes, settings, --selftest, netcheck), but goal.md names no desktop deliverable and D1–D8 runs no desktop test; the P6 row calls desktop UNMET and leaves the platform open. DESK-001–027 remain ledger rows; standing objective keeps them; contract Phase 6; no owner. |
| Desktop platform decision — Windows/Linux/macOS (DESK-001; §11.4) | REMOVED | goal.md:50 (OQ-001 pending), 93 (zero P0 OQ) | contract-v2.md:388–390 | The contract requires a deliberately recorded scope change; the proposal defers to OQ-001, which state.md:6 records as the open P0 question its stop criteria require closed. No decision, no owner, no phase; standing objective keeps DESK-001 (three platforms absent an approved change). Without an approved Windows-only change, the current Win32 shell cannot be the final implementation (contract-v2.md:389–390). |
| DESK-028 negative constraint | REMOVED | goal.md:81 (C7) | contract-v2.md:379 | C7 restates DESK-028 unchanged; rust/lovelace-desktop/src/netcheck.rs already implements a structural no-socket proof and --selftest exercises it, but no D1–D8 command runs it, so this cycle neither re-verifies nor gates the demonstration. The constraint survives; the check is not delivered by the proposal. |
| ECO-007 .NET consumer compatibility decision | REMOVED | None — absent from goal.md:36–52 | contract-v2.md:155 | goal.md records no decision on a C ABI, FFI bridge or C# wrapper; an unmet ledger row is not the "explicit decision" ECO-007 requires. Standing objective keeps the ID; no owner/phase. |
| ECO-008 persisted-format compatibility | REMOVED | None — absent from goal.md:36–52 | contract-v2.md:156 | goal.md commits no inventory of persisted formats and no verified migration path. Standing objective keeps ECO-008; ledger row only; no owner/phase. |
| P0–P2 semantic parity — NUM, REAL, CPLX, ARRAY, LANG, ENG, PLUG, MIR (§5–§8) | REMOVED | None — absent from goal.md:5–9 and 36–52 | contract-v2.md:19–21, 158–287, 518–519 | No numerical, array, language/engine, plugin or MathIR work in the outcome or D1–D8; all eight families sit at status=unmet, owner=unassigned, phase=0. The only indirect contact is the 5 000-case diff, whose generator domain is unspecified and which cannot substitute for the family gates. Contract Phases 3–4. |
| Rust engineering quality (RUST-001–010; §13.2) | REMOVED | None — absent from goal.md:5–9 and 36–52 | contract-v2.md:479–490, 492–494 | goal.md commits no fmt/clippy/unsafe/dependency/documentation/release-artifact gates; the zero-dependency strategy is not made an explicit engineering decision. Standing objective keeps RUST-001–010; ledger rows only; no owner/phase. |

## Question for the user

The desktop is REMOVED from this cycle's deliverables (see "Studio desktop IDE"), so this question decides the standing objective, not this cycle's DoD. The contract requires Windows, Linux and macOS unless a Windows-only scope change is deliberately recorded (contract-v2.md:388–390); the proposal leaves that choice open as OQ-001 (goal.md:50) while its stop criteria require zero P0 open questions (goal.md:93). Do you approve Windows-only as the desktop scope, or must all three platforms remain required?

## Standing-contract coverage

**No — not under the delivery reading.** Of the 25 families, 8 have some cycle mechanism (RUN and GATE fully; REQ, ECO, TEST, DIFF, HARNESS partially; WIRE by proxy only) and 17 have none; 168 of the 262 IDs sit in those 17 families. All 262 IDs exist in the round-0 ledger as unmet/unassigned/phase 0; if "accounted for" means recorded, yes by construction — if it means named with a cycle gate and an owner, no.

| Family | Cycle mechanism? | Missing IDs / gap |
|---|---|---|
| REQ | Partial — 001 (oracle), 002 (projects and tests only), 005 (ledger), 008 (fail-closed) | 003, 004, 006, 007, 009, 010 |
| RUN | Yes — 001–014, D6 | none |
| WIRE | Proxy only — 20 fixtures, no WIRE ID | 001–015 |
| ECO | Partial — 001/002, projects.toml | 003–008 |
| NUM | No | 001–010 |
| REAL | No | 001–010 |
| CPLX | No | 001–006 |
| ARRAY | No | 001–010 |
| LANG | No | 001–007 |
| ENG | No | 001–012 |
| PLUG | No | 001–008 |
| SYM | No — "Symbolics" word and the 212/214 row, no SYM ID | 001–014 |
| MIR | No | 001–008 |
| DSP | No — word only, ledger row | 001–010 |
| STAT | No — word only, ledger row | 001–004 |
| CLI | No — "Console" word only; the 14 CLI flags are RUN | 001–007 |
| KNOW | No — word only, ledger row | 001–010 |
| DESK | No — 028 restated as C7 only | 001–028 |
| TEST | Partial — 001/003/006/007/011 | 002, 004, 005, 008, 009, 010, 012, 013 |
| DIFF | Partial — 001–010 named as a range; D4 delivers the 5 000/PR slice | 006–008 undelivered; 002–005 not itemised; 009 nightly removed |
| PROOF | No — "Proofs mapping" word only | 001–004 |
| GATE | Yes — 001–012 named, D7 | none by ID; 001/002/006/007/010 lack an expected gate |
| PERF | No — "PERF" word only, ledger rows | 001–010 |
| RUST | No | 001–010 |
| HARNESS | Partial — 001 (ledger), 012 (C6) | 002–011 |

Missing families (no cycle mechanism), with IDs: NUM-001–010; REAL-001–010; CPLX-001–006; ARRAY-001–010; LANG-001–007; ENG-001–012; PLUG-001–008; SYM-001–014; MIR-001–008; DSP-001–010; STAT-001–004; CLI-001–007; KNOW-001–010; DESK-001–028; PROOF-001–004; PERF-001–010; RUST-001–010.

Gaps inside the eight partial/proxy families: REQ-003/004/006/007/009/010 (REQ-002 unproven beyond projects and tests); WIRE-001–015; ECO-003–008; TEST-002/004/005/008/009/010/012/013 (011 declared as C2, not gated); DIFF-006–008 plus the 100,000 nightly; HARNESS-002–011.

The single REPLACED row (WIRE-001–015 → 20-fixture gate) is also an unapproved narrowing, but it is reported as REPLACED, not counted below. No approval artifact exists in the proposal; goal.md:54 instead asserts that nothing is removed.

REMOVED without approval: 18 — the 214-example corpus; the 100,000-case nightly differential run; the twelve property families; the SymPy oracle (SYM-014); the Lean proofs (PROOF-001/003/004); the Proofs mapping (PROOF-002); performance (PERF-001–010); DSP; Statistics; Console/REPL (CLI-001–007); Knowledge (KNOW-001–010); the Studio desktop (DESK-001–027); the Windows/Linux/macOS platform decision; DESK-028; ECO-007; ECO-008; P0–P2 semantic parity (NUM/REAL/CPLX/ARRAY/LANG/ENG/PLUG/MIR); Rust engineering quality (RUST-001–010).
