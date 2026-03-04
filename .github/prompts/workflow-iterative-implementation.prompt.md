---
agent: agent
description: Implement one checklist item end-to-end — write tests, implement the feature, build, test, and mark the item done.
---

#file:.github/prompts/legacy-knowledge-map.md
#file:.github/prompts/skill-falsify-claims.prompt.md
#file:.github/prompts/skill-test-standards.prompt.md
#file:.github/prompts/skill-impl-completeness.prompt.md

# Workflow: Iterative Implementation

## Purpose
Take **one unchecked item** from a prior Requirements Gathering checklist and bring it to a fully passing, committed state:
- xUnit tests written and verified
- C# implementation written and verified
- `dotnet build` passes
- `dotnet test` passes

One item per invocation. The developer triggers the next item manually.

## Input (supplied by caller)

```
ChecklistItem:  <Exact text of the unchecked checklist item, e.g. "[ ] Add(Integer) → operator+ (IAdditionOperators<T,T,T>)">
CsProject:      <Target C# project, e.g. Lovelace.Integer>
TestProject:    <Corresponding test project, e.g. Lovelace.Integer.Tests>
```

## Procedure

---

### Step 1 — Write xUnit tests

1. Retrieve the named test list for this item from the previously produced Test Plan (or re-run the Test Standards skill if unavailable).
2. Run the **Falsify Claims** skill on all test assumptions.
3. If any assumption is Falsified, revise the affected tests and repeat Step 1 until zero Falsified rows.
4. Write the xUnit test methods in `<TestProject>/` using the naming convention `MethodName_GivenScenario_ExpectedResult`.
   - Use `[Fact]` for single-case tests.
   - Use `[Theory]` + `[InlineData]` or `[MemberData]` for parameterised tests.
   - Reference only the public API of `<CsProject>` — never internal or implementation details.
5. Confirm the test file compiles (run `dotnet build <TestProject>` and check for errors).

---

### Step 2 — Write the C# implementation

1. Consult the legacy C++ `.hpp`/`.cpp` for the corresponding method's logic.
2. State a numbered list of implementation claims (e.g. "Carry propagation stops when carry is 0").
3. Run the **Falsify Claims** skill on all implementation claims.
4. If any claim is Falsified, revise the implementation plan and repeat Step 2 until zero Falsified rows.
5. Write the C# implementation in `<CsProject>/`:
   - Follow all rules in `.github/copilot-instructions.md` (English names, BCD via `GetDigit`/`SetDigit` only, implement relevant `System.Numerics` interfaces).
   - Keep the implementation idiomatic C# — do not transliterate C++ pointer arithmetic literally.

---

### Step 3 — Build and test loop

Repeat the following until both commands exit with code 0:

```
dotnet build <CsProject>
dotnet test  <TestProject> --no-build
```

On failure:
1. Read the compiler or test-runner output carefully.
2. Identify the root cause (compilation error, assertion failure, exception).
3. Fix the smallest possible change in either the implementation or the test (but never weaken a test to make it pass — only fix incorrect expectations discovered by Falsify Claims).
4. Run again.

Do not exit this loop with a non-zero exit code.

---

### Step 4 — Mark the item done and report

1. In the checklist document (if it exists as a file), change `- [ ]` to `- [x]` for the completed item.
2. Output a brief summary:

```markdown
## ✅ Completed: `<ChecklistItem>`

**Tests added** (`<TestProject>/`):
- `Add_GivenTwoPositiveIntegers_ReturnsCorrectSum`
- `Add_GivenZeroAndN_ReturnsN`
- ...

**Implementation**: `<CsProject>/<FileName>.cs` — method `Add(Integer b)`

**Build**: ✅ passed  
**Tests**: ✅ X/X passed

**Remaining checklist items**:
- [ ] `Subtract(Integer)` → `operator-`
- [ ] `Multiply(Integer)` → `operator*`
...
```

3. Tell the developer:
> "Item complete. To continue, run `workflow-iterative-implementation` again with the next unchecked item."
