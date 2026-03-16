# Decisions

> **Journal file**: Architectural or implementation decisions with rationale.
> **Schema**: See `.github/prompts/journal-schema.md` §4.4 for the DEC entry template.
> **Append-only**: New entries are appended at the end. Existing entries are never deleted or reordered.

---

<!-- Paste the template below to add a new entry. Replace placeholders with actual values. -->
<!-- TEMPLATE:
### DEC-{NNN}: {Short title}

- **Decision**: {What was decided}
- **Rationale**: {Why — with links to supporting evidence (OBS, HYP, VAL IDs)}
- **Alternatives considered**: {What other options were evaluated and why they were rejected}
- **Related**: {Comma-separated list of related entry IDs}
-->

---

*No entries yet.*

---

### DEC-001: migration-findings.md distillation scope — OBS only, no validated DEC entries yet

- **Decision**: `migration-findings.md` was populated from OBS-029–036 (observed migration differences) and VAL-008/009 (validated HYPs) rather than waiting for DEC entries, because the document was at Low confidence (empty) and blocking stopping criterion 3. The marker level ⚠️ (Tentative) was assigned to all claims since no DEC-backed formal decision records exist yet for the migration choices.
- **Rationale**: Stopping criterion 3 required all distilled docs to reach Medium confidence. The OBS evidence base was sufficient for ⚠️-marked claims, which moves the document from Low to Medium without overstating confidence.
- **Alternatives considered**: Waiting for formal DEC entries from the development team. Rejected because DEC entries are optional and there is no scheduled process to create them for historical migration decisions.
- **Related**: OBS-029, OBS-030, OBS-031, OBS-032, OBS-033, OBS-034, OBS-035, OBS-036
