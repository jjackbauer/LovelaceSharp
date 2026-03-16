# Distilled Knowledge Schema

> **Usage**: Include this file in any prompt via `#file:.github/prompts/distilled-knowledge-schema.md`.
> This file is a reference-only document — it is not a runnable prompt.
>
> **Purpose**: Defines the canonical header template, inline uncertainty markers, and update
> criteria for all distilled knowledge files under `.github/distilled/`. Every skill that reads
> or writes distilled knowledge must conform to this schema.
>
> **Companion**: This schema builds on `journal-schema.md`. Journal entries are the raw evidence;
> distilled documents are the synthesised, curated knowledge derived from that evidence.

---

## 1. Distilled Document Header Template

Every file in `.github/distilled/` must begin with the following YAML-style metadata header
immediately after the H1 title:

```markdown
# {Document Title}

> **Scope**: {What area of the codebase or domain this document covers}
> **Confidence**: {High | Medium | Low} — overall confidence in the document's claims
> **Last updated**: {YYYY-MM-DD}
> **Source entries**: {Comma-separated list of journal entry IDs, e.g., OBS-001, HYP-003, VAL-002}
```

**Field definitions**:

| Field | Required | Description |
|---|---|---|
| Scope | Yes | A concise statement of what this document covers — a module, a cross-cutting concern, or a domain concept. |
| Confidence | Yes | Overall confidence level for the document as a whole: **High** (all claims verified), **Medium** (most claims supported, some tentative), **Low** (exploratory, many unverified claims). See §3 for mapping to §2 markers. |
| Last updated | Yes | The date of the most recent substantive update, in `YYYY-MM-DD` format. |
| Source entries | Yes | Journal entry IDs (OBS, HYP, VAL, DEC, etc.) that provide the evidential basis for this document. Must contain at least one ID. |

---

## 2. Inline Uncertainty Markers

Every factual claim within a distilled document must be tagged with one of three inline markers
to communicate the strength of its evidential backing:

| Marker | Name | Meaning | Evidential Requirement |
|---|---|---|---|
| ✅ | Verified | Claim is backed by a **Supported** validation (VAL entry with Result = Supported) | At least one VAL entry with Result = Supported that directly addresses this claim |
| ⚠️ | Tentative | Claim has supporting observations but **no formal validation** has been performed | At least one OBS entry; no VAL entry exists, or VAL Result = Unresolved |
| ❓ | Unverified | Claim is inferred from naming, structure, analogy, or incomplete evidence | No direct OBS grounding; derived from patterns, naming conventions, or structural analogy |

### 2.1 Marker Placement

Place the marker **at the start of the claim sentence or bullet point**:

```markdown
- ✅ `DigitStore` packs two decimal digits per byte using BCD encoding (OBS-001, VAL-005).
- ⚠️ The `Natural` class never directly accesses the backing `byte[]` (OBS-012).
- ❓ Division may use a Newton-Raphson refinement step for periodic decimal detection.
```

### 2.2 Marker Rules

1. **Never omit the marker.** Every factual claim in a distilled document must carry exactly one marker.
2. **Never promote a marker without evidence.** Moving ❓ → ⚠️ requires adding an OBS reference. Moving ⚠️ → ✅ requires a Supported VAL entry.
3. **Never remove a marker.** Even when a claim is well-established, keep the ✅ marker for traceability.
4. **Cite sources inline.** After each claim, include the journal entry IDs that support it in parentheses.

### 2.3 Confidence ↔ Marker Relationship

The document-level **Confidence** field (see §1) is derived from the distribution of inline markers:

| Document Confidence | Criterion |
|---|---|
| **High** | All claims are ✅ Verified; zero ⚠️ or ❓ markers remain |
| **Medium** | Majority of claims are ✅ Verified or ⚠️ Tentative; ❓ markers are a minority |
| **Low** | Significant number of ❓ Unverified claims; exploratory document |

---

## 3. Update Criteria

A distilled knowledge document must be updated when any of the following triggers occur:

| # | Trigger | Action | Example |
|---|---|---|---|
| 1 | **Claim falsified or revised** | Find all claims sourced from the falsified HYP. Downgrade their marker (✅ → ⚠️ or ⚠️ → ❓) or remove the claim entirely. Update Source entries and Last updated. | VAL-010 falsifies HYP-003 → remove or revise claims citing HYP-003 |
| 2 | **Tentative conclusion gains Supported validation** | Upgrade the claim's marker from ⚠️ to ✅. Add the VAL ID to the inline citation and to the header Source entries. Update Last updated. | VAL-015 supports HYP-007 → promote ⚠️ claim to ✅ |
| 3 | **Completeness review identifies gap** | Add a new section or bullet for the gap, marked ❓ or ⚠️ as appropriate. Add a TODO or OQ entry in the journal for follow-up investigation. Update Last updated. | Completeness review finds no coverage of thread safety → add ❓ placeholder |
| 4 | **New observations extend coverage** | Add new claims with appropriate markers (typically ⚠️ for newly observed facts). Add the OBS IDs to Source entries. Update Last updated. | OBS-045 documents a new execution flow → add ⚠️ claim to `runtime-flows.md` |

### 3.1 Update Procedure

When updating a distilled document:

1. **Read the trigger** — identify which journal entry or event caused the update.
2. **Locate affected claims** — search the document for claims citing the affected entry IDs.
3. **Apply the marker change** — upgrade, downgrade, or add markers per the trigger rules above.
4. **Update inline citations** — add or remove journal entry IDs in the parenthetical references.
5. **Update the header** — refresh Source entries (add new IDs, keep existing), set Last updated to today's date, and re-evaluate overall Confidence.
6. **Record a DEC entry** — if the update represents a significant synthesis decision, append a DEC entry to `.github/journals/decisions.md`.

---

## 4. Distilled Document Inventory

The following files live under `.github/distilled/` and each must conform to this schema:

| Document | Scope | Primary Sources |
|---|---|---|
| `system-overview.md` | High-level architecture description | OBS (project structure), VAL (boundary claims) |
| `module-map.md` | Per-project responsibilities and interfaces | OBS (class surface), supported HYP, VAL |
| `domain-concepts.md` | BCD packing, periodic decimals, exponent model | OBS, validated HYP |
| `runtime-flows.md` | Key execution paths | OBS (call chains), VAL |
| `dependencies.md` | Inter-project and external dependencies | OBS, VAL (boundary claims) |
| `invariants-and-risks.md` | Architectural invariants plus known risks | Supported HYP, RISK entries |
| `migration-findings.md` | C++ → C# migration decisions and lessons | DEC, VAL, OBS |
| `trusted-facts.md` | Claims with High confidence + Supported validation only | **Strict ✅ markers only** — no ⚠️ or ❓ allowed |
| `unresolved-areas.md` | Gaps, weak evidence, open questions | OQ, unresolved VAL |
| `glossary.md` | Domain terms (Portuguese → English, BCD terminology) | Extends `legacy-knowledge-map.md` |

### 4.1 Special Rules

- **`trusted-facts.md`**: This document may only contain ✅ Verified claims. If a claim's validation is later overturned, it must be removed from this file (not downgraded to ⚠️).
- **`glossary.md`**: Extends `legacy-knowledge-map.md` — it should reference that file and add terms not already covered there.
