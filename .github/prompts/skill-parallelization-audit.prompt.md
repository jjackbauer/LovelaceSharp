---
agent: agent
description: Audit a C# class for thread-safety gaps and parallelization opportunities, gated by Falsify Claims and Impl Completeness.
---

#file:.github/prompts/skill-falsify-claims.prompt.md
#file:.github/prompts/skill-impl-completeness.prompt.md

## Input

```
CsProject:  <project folder, e.g. Lovelace.Natural>
CsFile:     <specific class file, e.g. Natural.cs — if omitted, all *.cs in CsProject are read>
```

---

## Procedure

### Step 1 — Enumerate the class surface

Read all `*.cs` files in `<CsProject>/`. List every method (public + private), field, property, and auto-property. Note accessibility, and whether each is `static`, `readonly`, or mutable.

### Step 2 — Build the Data Mutability Table

For each field/property:

| Member | Type | Mutable? | Shared across calls? | Thread-safety risk |
|---|---|---|---|---|
| `_digits` (`byte[]`) | reference type | Yes | Yes | High |
| `IsZero` (internal set) | `bool` | Yes | Yes | Medium |
| ... | | | | |

Risk ratings:
- **None** — read-only or value-type local
- **Low** — write-once (set in constructor only)
- **Medium** — reassigned after construction
- **High** — mutated in place (arrays, collections, counters)

### Step 3 — Build the Sequential Dependency Graph (per method)

For each method, identify:
- (a) which shared mutable members it reads / writes
- (b) whether it calls other methods on `this` that also mutate shared state
- (c) whether any loops contain independent iterations (no carry chain or cross-iteration data dependency)

Record each of these observations as a claim for Step 4.

### Step 4 — Run Falsify Claims on all data-access and dependency claims

Collect every claim from Steps 2–3 into a numbered list and invoke the **Falsify Claims** skill. Revise until **zero Falsified rows** remain before proceeding.

### Step 5 — Thread Safety Assessment Table

For each piece of shared mutable state, recommend a concrete .NET mechanism. This phase must be completed before any parallelization is applied.

| Member | Risk | Recommendation | Priority |
|---|---|---|---|
| `_digits` (`byte[]`) | High | Wrap all mutations in `lock (_syncRoot)`; use `Interlocked` for atomic scalar counters | P0 |
| `IsZero` (internal set) | Medium | Update always under the same lock as `_digits` writes | P0 |
| ... | | | |

Priority legend:
- **P0** — must fix before any parallelization (correctness blocker)
- **P1** — should fix (race condition under concurrent read+write)
- **P2** — optional improvement (contention reduction, e.g. `ReaderWriterLockSlim`)

### Step 6 — Parallelization Opportunity Table

For each method (and notable inner loop), classify:

| Method / Loop | Shared Writes | Iterations Independent? | Parallelizable? | Suggested .NET API |
|---|---|---|---|---|
| `Add` carry propagation loop | `_digits[i]` | No (carry chain) | ❌ Sequential | — |
| `Multiply` partial-products loop | partial sums | Yes (after operand split) | ✅ With reduction | `Parallel.For` + `Interlocked.Add` |
| `ToString` digit-extraction loop | none (read-only) | Yes | ✅ Embarrassingly parallel | `AsParallel()` / `Span` slicing |
| ... | | | | |

### Step 7 — Coverage check via Impl Completeness skill

Invoke the **Impl Completeness** skill with `CsProject` to confirm that every method appearing in the C# class was included in the tables above. Flag any omissions as ⬜ Missing from audit.

### Step 8 — Produce the Improvement Checklist

Ordered by dependency: thread safety before parallelization, prerequisites before dependents.

```
## Parallelization Audit Checklist for `<CsProject>.<ClassName>`

### Phase 0 — Thread Safety (complete before Phase 1)
- [ ] `_digits`: wrap all writes in `lock (_syncRoot)` [P0 — prerequisite for all parallelization]
- [ ] `IsZero`: ensure update is atomic under the same lock [P0]
- [ ] ...

### Phase 1 — Parallelization
- [ ] `Multiply` — parallelize partial-products with `Parallel.For` + reduction [depends on Phase 0]
- [ ] `ToString` — use `AsParallel()` on read-only digit extraction [independent of Phase 0]
- [ ] ...
```

---

## Output

Write all results to `.github/requirements/<CsProject>-parallelization-audit.md` using the structure below, then confirm the file path to the caller.

```markdown
# Parallelization Audit — `<CsProject>.<ClassName>`

## 1. Data Mutability Table
<!-- Step 2 table -->

## 2. Sequential Dependency Graph
<!-- Step 3 narrative -->

## 3. Falsify Claims Result
<!-- Step 4 table — confirm zero Falsified rows -->

## 4. Thread Safety Assessment
<!-- Step 5 table -->

## 5. Parallelization Opportunities
<!-- Step 6 table -->

## 6. Impl Completeness Coverage
<!-- Step 7 findings — flag any ⬜ Missing from audit -->

## 7. Improvement Checklist
<!-- Step 8 checklist — Phase 0 then Phase 1 -->
```

---

## Design Notes

- **Always writes output** — saves the audit report to `.github/requirements/<CsProject>-parallelization-audit.md` and confirms the path; unlike pure `skill-*` prompts this one persists results so they can be referenced by follow-up workflows.
- **`skill-impl-completeness` as coverage gate** (Step 7) — referenced, not re-implemented; consistent with how `skill-falsify-claims` is used in other skills.
- **Phase 0 / Phase 1 labels** — make the sequencing constraint (make thread-safe *before* parallelizing) visually unambiguous.
- **`CsFile` is optional** — defaults to all `*.cs` in the project, keeping input lightweight and matching the `CppClass`/`CsProject` pattern used elsewhere.
- **`async`/`await` patterns** — out of scope for this skill; treat `Task`-returning methods as a separate audit concern.
