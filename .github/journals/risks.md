# Risks

> **Journal file**: Identified risks with likelihood and impact assessments.
> **Schema**: See `.github/prompts/journal-schema.md` §4.6 for the RISK entry template.
> **Append-only**: New entries are appended at the end. Existing entries are never deleted or reordered.

---

<!-- Paste the template below to add a new entry. Replace placeholders with actual values. -->
<!-- TEMPLATE:
### RISK-{NNN}: {Short title}

- **Risk**: {Description of what could go wrong}
- **Likelihood**: {High | Medium | Low}
- **Impact**: {High | Medium | Low}
- **Evidence**: {OBS/HYP/VAL IDs or file:line references that support the risk assessment}
- **Mitigation**: {Steps to reduce likelihood or impact}
-->

---

### RISK-001: Natural TryConvert methods throw NotImplementedException

- **Risk**: Natural's six `TryConvert*` methods throw `NotImplementedException` while Integer and Real return `false`. Any generic math code calling `T.CreateChecked<int>(42)` where `T = Natural` will get an unhandled exception rather than a graceful failure.
- **Likelihood**: Medium
- **Impact**: Medium
- **Evidence**: OBS-019, VAL-004, Natural.cs:901-923
- **Mitigation**: Change Natural's TryConvert stubs to return `false` (matching Integer and Real's behavior) or implement actual conversions.

---

### RISK-002: Real.Exponent has public setter — mutable after construction

- **Risk**: `Real.Exponent` has a public setter (Real.cs:104), allowing external code to modify the exponent after construction. This could violate representation invariants (e.g., changing exponent without adjusting magnitude digits).
- **Likelihood**: Low
- **Impact**: High
- **Evidence**: OBS-011, Real.cs:104
- **Mitigation**: Consider restricting the Exponent setter to `internal` or `private set` to prevent external mutation.

---

### RISK-003: Real.Pow stubs throw NotImplementedException for non-integer and negative exponents

- **Risk**: `Real.Pow` throws `NotImplementedException` for non-integer exponents (e.g., `pow(x, 0.5)`) and negative exponents (e.g., `pow(x, -1)`). Code that calls `Pow` with such values will encounter unhandled exceptions, not graceful failures.
- **Likelihood**: Medium
- **Impact**: Medium
- **Evidence**: OBS-036, VAL-009, Real.cs:642,651
- **Mitigation**: (1) Implement negative-exponent path: `Invert() * Pow(Abs(exponent))` — no dependencies missing. (2) Implement non-integer path after `Real.Log` is available. Until then, document the limitation prominently.
