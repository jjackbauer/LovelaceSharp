# Todos

> **Journal file**: Actionable exploration or implementation tasks.
> **Schema**: See `.github/prompts/journal-schema.md` §4.5 for the TODO entry template.
> **Append-only**: New entries are appended at the end. Existing entries are never deleted or reordered.
> Status updates (Open → In progress → Done/Cancelled) are made in-place.

---

<!-- Paste the template below to add a new entry. Replace placeholders with actual values. -->
<!-- TEMPLATE:
### TODO-{NNN}: {Short title}

- **Task**: {Description of what needs to be done}
- **Priority**: {P0 | P1 | P2}
- **Depends on**: {Entry IDs or conditions that must be met first}
- **Status**: {Open | In progress | Done | Cancelled}
- **Related**: {Comma-separated list of related entry IDs}
-->

---

### TODO-001: Explore Legacy C++ files and map to C# equivalents

- **Task**: Read all C++ header/source files in `Legacy/` and produce a method-by-method mapping to C# projects. Identify missing migrations.
- **Priority**: P1
- **Depends on**: 
- **Status**: Done
- **Related**: OQ-001, OBS-018, OBS-029, OBS-030, OBS-031, OBS-032, OBS-033, OBS-034

---

### TODO-002: Explore execution flows end-to-end

- **Task**: Trace key execution flows: (1) parsing a string to Real, (2) Real division with period detection, (3) REPL `pi(100)` evaluation. Record OBS entries for each call chain.
- **Priority**: P1
- **Depends on**: 
- **Status**: Done
- **Related**: OBS-013, OBS-014, OBS-016, OBS-021, OBS-022, OBS-023, OBS-024, OBS-025, OBS-028

---

### TODO-003: Explore edge cases and error handling

- **Task**: Document exception paths across all numeric types: division by zero, overflow handling, empty input parsing, max-precision behavior. Verify test coverage for each.
- **Priority**: P2
- **Depends on**: 
- **Status**: Done
- **Related**: OBS-007, OBS-013, OBS-035, OBS-036

---

### TODO-004: Verify REPL test coverage completeness

- **Task**: List all source files in `Lovelace.Console/Repl/` and compare against test files in `Lovelace.Console.Tests/`. Identify any source component (e.g., LineEditor, ReplSession) that lacks a test file.
- **Priority**: P2
- **Depends on**: OQ-003
- **Status**: Open
- **Related**: OQ-003, OBS-016
