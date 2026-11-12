# Harness State — desktop-v4

- **Round**: 2 of 40 (round 1 census complete; round 2 ARC-001 spike in flight)
- **Goal**: Lovelace Desktop ships as a native Rust scientific scripting workbench that satisfies every
  P0 row of the v4.0 contract with fresh, source-bound GUI evidence and passes all 22 mandatory
  journeys.
- **Definition of done**: 0/19 dimensions MET (0/171 P0 PASS; 1 waived)
- **Stop criteria**: (1) `-Verify` exit 0, all dimensions MET — **unmet**; (2) round 40 — not
  reached; (3) same blocking condition 3 rounds — not triggered.

## Gate status

| Gate | Status | Detail |
|---|---|---|
| G1 Evidence | PASS | 28 EVD rows; every claim resolves to a file:line, a command, or an observed output |
| G2 Falsification | PASS | VAL-001 falsified the v1 contract's "no replacement" claim; contract repaired, 0 open falsified rows |
| G3 Coverage | FAIL (expected) | 0/19 dimensions MET; the census supplies current-state facts but no requirement row has an acceptance test yet |
| G4 Reproduction | PASS | desktop build + `--selftest` reproduced; eframe probe built, launched, captured and UIA-dumped by the orchestrator |
| G5 Honesty | PASS | every observer's "could not determine" list is retained in its artifact; OQ-002 and RISK-004/005 recorded |

## Objective ledger

| Round | Objective | Mode | Agents | New EVD | Falsified | Gates | Outcome |
|---|---|---|---|---|---|---|---|
| 0 | Contract the goal, freeze the baseline, build a fail-closed gate | Probe | 0 | 18 | 0 | G1✓ G2✓ G3✗ G4✓ G5✓ | gate skeleton live; baseline pinned |
| 1 | Census current implementation + web reference for all 17 groups | Survey | 1 auditor + 5 observers | 10 | 1 (6 claims) | G1✓ G2✓ G3✗ G4✓ G5✓ | 784 OBS rows across 5 artifacts; audit forced contract repair; DEC-006 |
| 2 | ARC-001 framework/editor spike with measured scenarios | Adversarial (Implementer → orchestrator falsifies) | 1 implementer | in flight | — | — | in flight |

## Coverage (definition-of-done dimensions)

All 19 at **None** except D01 partial: CON-001 baseline is pinned (EVD-001/019) and CON-002 has the
register skeleton + census, but no row is PASS yet.

## Round-1 census artifacts (verified by the orchestrator)

| Artifact | OBS rows | Key verified facts |
|---|---|---|
| `round-1/census-web.md` | 107 (+23 capability rows) | plot arrives as SVG string; structured result tree; no tabs; session = browser tab (EVD-023/024) |
| `round-1/census-core.md` | 238 | 14 commands; synchronous `run_blocking`; string-only `RunOutput`; no cancel token (EVD-025) |
| `round-1/census-gui.md` | 322 | 5 child controls; plain EDIT editor; no UIA, DPI, theme integration; 9 shortcuts (EVD-004..006) |
| `round-1/census-render-engine.md` | 117 (+34 SVG, 53 engine rows) | parser covers text+polyline only; engine exposes `variable_values`/`structured::project_value` (EVD-026/027) |
| `round-1/census-tests.md` | 60 | 15 selftests; no GUI/screenshot/perf/a11y automation; CI has no Rust job (EVD-016/018) |

## Next objective

Round 2 (in flight): verify the ARC-001 spike itself — build it, run `--bench`, capture its window,
dump its UIA tree, and falsify every measurement it reports before recording the framework decision.
