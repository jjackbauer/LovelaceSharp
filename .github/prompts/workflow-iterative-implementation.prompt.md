---
agent: agent
description: Implement one checklist item end-to-end — write functional tests, implement the feature, build, test, and mark the item done.
---

#file:.github/prompts/legacy-knowledge-map.md
#file:.github/prompts/skill-falsify-claims.prompt.md
#file:.github/prompts/skill-test-standards.prompt.md
#file:.github/prompts/skill-impl-completeness.prompt.md

# Workflow: Iterative Implementation

## Purpose
Take **one unchecked item** from a prior Requirements Gathering checklist and bring it to a fully passing, committed state:
- Functional xUnit tests written, verified, and passing
- C# implementation fully written (zero `throw new NotImplementedException()` stubs remain for the covered members)
- `dotnet build` passes
- `dotnet test` passes

One item per invocation. The developer triggers the next item manually.

## Hard Rules — read before writing a single line of code

> **These rules are non-negotiable. Violating any one of them means the item is NOT done.**

1. **Every test must be a functional test.**  
   A functional test calls the method/operator/constructor under test with **concrete literal inputs** and asserts on the **concrete return value or observable state**.  
   Tests that only call `typeof(T).IsAssignableFrom(...)`, check a type name, or otherwise inspect metadata are *scaffolding tests* — they do not count toward the checklist item and must **not** be written or modified here.

2. **No `NotImplementedException` stubs may remain** in any member that the checklist item covers.  
   If the item is "Constructors: `Natural()`, `Natural(ulong)`, `Natural(int)`" then all three constructor bodies must contain real logic when Step 4 is reached.

3. **A test must be able to fail.**  
   If the only possible failure mode is a compile error, the test is a scaffolding test, not a functional test. Reject it.

4. **Tests must not be weakened to make them pass.**  
   Only fix an expectation when the Falsify Claims skill confirms the expectation was factually wrong. Never change `Assert.Equal("12345", n.ToString())` to `Assert.NotNull(n)` just to get green.

---

## Input (supplied by caller)

```
ChecklistItem:  <Exact text of the unchecked checklist item, e.g. "[ ] Add(Natural, Natural) → operator+">
CsProject:      <Target C# project, e.g. Lovelace.Natural>
TestProject:    <Corresponding test project, e.g. Lovelace.Natural.Tests>
```

---

## Procedure

### Step 1 — Derive functional test cases

1. Retrieve the named test list for this item from the previously produced Test Plan (or re-run the Test Standards skill if unavailable).
2. For **each** named test, verify it satisfies all three criteria from Hard Rule 1:
   - It calls the target member with a concrete input.
   - It asserts on a concrete expected value or side-effect.
   - Removing the implementation body (replacing it with `throw new NotImplementedException()`) would cause the test to **fail**, not just compile-error.  
   If a test does not satisfy all three, discard it and derive a replacement.
3. Run the **Falsify Claims** skill on all remaining test assumptions.
4. If any assumption is Falsified, revise the affected tests and repeat from Step 1.3 until zero Falsified rows remain.

---

### Step 2 — Write the xUnit tests

1. Write the finalised test methods in `<TestProject>/` in an appropriately named file (e.g. `NaturalConstructorTests.cs`, `NaturalAddTests.cs`).
   - Use `[Fact]` for single-case tests.
   - Use `[Theory]` + `[InlineData]` or `[MemberData]` for parameterised tests.
   - Reference only the public API of `<CsProject>` — never internal or implementation details.
   - Every assert must use a concrete expected value: `Assert.Equal(expected, actual)`, `Assert.True(actual)`, `Assert.Throws<T>(...)` etc.
2. Confirm the test file compiles: run `dotnet build <TestProject>` and check for errors.
3. **Before implementing**, run `dotnet test <TestProject>` and confirm that **every new test fails** (because the implementation still throws `NotImplementedException`).  
   If a new test passes before implementation exists, it is a scaffolding test — replace it.

---

### Step 3 — Write the C# implementation

1. Consult the legacy C++ `.hpp`/`.cpp` for the corresponding method's logic.
2. State a numbered list of implementation claims (e.g. "Carry propagation stops when carry becomes 0").
3. Run the **Falsify Claims** skill on all implementation claims.
4. If any claim is Falsified, revise the implementation plan and repeat from Step 3.3 until zero Falsified rows remain.
5. Write the C# implementation in `<CsProject>/`:
   - Follow all rules in `.github/copilot-instructions.md` (English names, BCD via `GetDigit`/`SetDigit` only, implement relevant `System.Numerics` interfaces).
   - Keep the implementation idiomatic C# — do not transliterate C++ pointer arithmetic literally.
   - **Replace every `throw new NotImplementedException()` stub** in the members covered by this checklist item with real logic. No stub may survive.

---

### Step 4 — Build and test loop

Repeat the following until both commands exit with code 0:

```
dotnet build <CsProject>
dotnet test  <TestProject> --no-build
```

On failure:
1. Read the compiler or test-runner output carefully.
2. Identify the root cause (compilation error, assertion failure, exception).
3. Fix the root cause in the implementation. Only touch a test when the Falsify Claims skill confirms the test's expectation was factually wrong — and even then, never weaken the assertion, only correct it.
4. Run again.

Do not exit this loop with a non-zero exit code.

---

### Step 5 — Mark the item done and report

Before marking done, perform a final self-check:

| Check | Criterion |
|---|---|
| All added tests are functional | Each test calls a real method with a real input and asserts a real output |
| Zero stubs remain | No `throw new NotImplementedException()` in any member covered by this item |
| Build passes | `dotnet build <CsProject>` exits 0 |
| Tests pass | `dotnet test <TestProject>` exits 0 |

Only if all four checks pass:

1. In the checklist document, change `- [ ]` to `- [x]` for the completed item.
2. Output a brief summary:

```markdown
## ✅ Completed: `<ChecklistItem>`

**Functional tests added** (`<TestProject>/<TestFile>.cs`):
- `Add_GivenTwoPositiveNumbers_ReturnsCorrectSum` — asserts `Natural(2) + Natural(3) == Natural(5)`
- `Add_GivenZeroAndN_ReturnsN`                  — asserts `Natural(0) + Natural(42) == Natural(42)`
- ...

**Implementation**: `<CsProject>/<FileName>.cs` — members `<list covered members>`

**Build**: ✅ passed  
**Tests**: ✅ X/X passed (Y new functional tests)

**Remaining checklist items**:
- [ ] `Subtract(Natural)` → `operator-`
- [ ] `Multiply(Natural)` → `operator*`
...
```

3. Tell the developer:
> "Item complete. To continue, run `workflow-iterative-implementation` again with the next unchecked item."
