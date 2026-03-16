# Open Questions

> **Journal file**: Unresolved questions requiring further investigation.
> **Schema**: See `.github/prompts/journal-schema.md` §4.7 for the OQ entry template.
> **Append-only**: New entries are appended at the end. Existing entries are never deleted or reordered.

---

<!-- Paste the template below to add a new entry. Replace placeholders with actual values. -->
<!-- TEMPLATE:
### OQ-{NNN}: {Short title}

- **Question**: {The specific question to be resolved}
- **Needed evidence**: {What kind of evidence would answer the question}
- **Priority**: {P0 | P1 | P2}
- **Related**: {Comma-separated list of related entry IDs}
-->

---

### OQ-001: Legacy C++ to C# method mapping completeness

- **Question**: Which C++ methods in `Lovelace.cpp`, `InteiroLovelace.cpp`, and `RealLovelace.cpp` have not yet been migrated to their C# counterparts?
- **Needed evidence**: Side-by-side comparison of C++ method list vs. C# method list for each class pair (Lovelace↔Representation+Natural, InteiroLovelace↔Integer, RealLovelace↔Real)
- **Priority**: P1
- **Status**: Resolved — OBS-029 through OBS-034 provide the full method-by-method mapping. See migration-findings.md.
- **Related**: OBS-018, OBS-029, OBS-030, OBS-031, OBS-032, OBS-033, OBS-034

---

### OQ-002: Thread safety of Real's mutable Exponent property

- **Question**: Is `Real.Exponent`'s public setter thread-safe or could concurrent reads/writes to the same Real instance cause torn reads? Is there evidence that Real instances are shared across threads?
- **Needed evidence**: Read Real.cs to check if Exponent uses volatile/Interlocked or any locking. Check if any code shares Real instances across threads.
- **Priority**: P2
- **Related**: OBS-012, RISK-002

---

### OQ-003: Test coverage completeness for Console REPL

- **Question**: Are there test files for all REPL components (Tokenizer, Parser, Evaluator, Value, LineEditor, ReplSession)?
- **Needed evidence**: List test files in Lovelace.Console.Tests/ and compare against source files in Lovelace.Console/Repl/
- **Priority**: P2
- **Related**: OBS-016
