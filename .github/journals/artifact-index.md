# Artifact Index

> **Journal file**: Registry of generated downstream artifacts with evidence links.
> **Schema**: See `.github/prompts/journal-schema.md` §4.8 for the ART entry template.
> **Append-only**: New entries are appended at the end. Existing entries are never deleted or reordered.

---

<!-- Paste the template below to add a new entry. Replace placeholders with actual values. -->
<!-- TEMPLATE:
### ART-{NNN}: {Artifact name}

- **Artifact path**: {Relative path to the generated artifact, e.g., `.github/artifacts/architecture-report.md`}
- **Type**: {architecture-report | migration-plan | risk-assessment | implementation-plan}
- **Supporting evidence**: {Comma-separated list of OBS, HYP, and VAL IDs that back this artifact}
- **Confidence**: {High | Medium | Low}
- **Last updated**: {YYYY-MM-DD}
-->

---

### ART-001: system-overview.md

- **Artifact path**: `.github/distilled/system-overview.md`
- **Type**: architecture-report
- **Supporting evidence**: OBS-001, OBS-005, OBS-008, OBS-011, OBS-016, OBS-018, OBS-019, VAL-001, VAL-002
- **Confidence**: Medium
- **Last updated**: 2026-03-12

---

### ART-002: module-map.md

- **Artifact path**: `.github/distilled/module-map.md`
- **Type**: architecture-report
- **Supporting evidence**: OBS-001, OBS-002, OBS-003, OBS-004, OBS-005, OBS-006, OBS-007, OBS-008, OBS-009, OBS-010, OBS-011, OBS-012, OBS-013, OBS-014, OBS-015, OBS-016, OBS-017, OBS-018, OBS-019, OBS-020, OBS-029, OBS-031, OBS-034, VAL-001, VAL-002, VAL-003, VAL-004, VAL-005, VAL-008
- **Confidence**: Medium
- **Last updated**: 2026-03-16

---

### ART-003: domain-concepts.md

- **Artifact path**: `.github/distilled/domain-concepts.md`
- **Type**: architecture-report
- **Supporting evidence**: OBS-001, OBS-002, OBS-004, OBS-011, OBS-013, VAL-001
- **Confidence**: Medium
- **Last updated**: 2026-03-12

---

### ART-004: dependencies.md

- **Artifact path**: `.github/distilled/dependencies.md`
- **Type**: architecture-report
- **Supporting evidence**: OBS-003, OBS-006, OBS-010, OBS-018, OBS-020, VAL-001, VAL-002
- **Confidence**: Medium
- **Last updated**: 2026-03-12

---

### ART-005: invariants-and-risks.md

- **Artifact path**: `.github/distilled/invariants-and-risks.md`
- **Type**: architecture-report
- **Supporting evidence**: OBS-001, OBS-003, OBS-004, OBS-009, OBS-019, OBS-033, OBS-035, OBS-036, VAL-001, VAL-002, VAL-003, VAL-004, VAL-005, VAL-009, RISK-001, RISK-002, RISK-003
- **Confidence**: Medium
- **Last updated**: 2026-03-16

---

### ART-006: trusted-facts.md

- **Artifact path**: `.github/distilled/trusted-facts.md`
- **Type**: architecture-report
- **Supporting evidence**: OBS-001, OBS-002, OBS-003, OBS-005, OBS-006, OBS-007, OBS-008, OBS-009, OBS-010, OBS-011, OBS-018, OBS-019, VAL-001, VAL-002, VAL-003, VAL-004, VAL-005
- **Confidence**: High
- **Last updated**: 2026-03-12

---

### ART-007: unresolved-areas.md

- **Artifact path**: `.github/distilled/unresolved-areas.md`
- **Type**: architecture-report
- **Supporting evidence**: OQ-001, OQ-002, OQ-003, RISK-001, RISK-002, TODO-001, TODO-003
- **Confidence**: Medium
- **Last updated**: 2026-03-12

---

### ART-008: runtime-flows.md

- **Artifact path**: `.github/distilled/runtime-flows.md`
- **Type**: architecture-report
- **Supporting evidence**: OBS-005, OBS-011, OBS-012, OBS-013, OBS-014, OBS-015, OBS-016, OBS-017, OBS-021, OBS-022, OBS-023, OBS-024, OBS-025, OBS-028, VAL-006
- **Confidence**: Medium
- **Last updated**: 2026-03-12

---

### ART-009: glossary.md

- **Artifact path**: `.github/distilled/glossary.md`
- **Type**: architecture-report
- **Supporting evidence**: OBS-001, OBS-002, OBS-005, OBS-008, OBS-011, OBS-021, OBS-026, OBS-027, VAL-006, VAL-007
- **Confidence**: Medium
- **Last updated**: 2026-03-12

---

### ART-010: migration-findings.md

- **Artifact path**: `.github/distilled/migration-findings.md`
- **Type**: migration-plan
- **Supporting evidence**: OBS-029, OBS-030, OBS-031, OBS-032, OBS-033, OBS-034, OBS-035, OBS-036, VAL-008, VAL-009, RISK-001, RISK-002, RISK-003
- **Confidence**: Medium
- **Last updated**: 2026-03-16
