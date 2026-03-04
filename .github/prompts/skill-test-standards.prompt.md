---
agent: agent
description: Generate a complete xUnit test plan for a C# method migrated from the legacy C++ source.
---

#file:.github/prompts/legacy-knowledge-map.md
#file:.github/prompts/skill-falsify-claims.prompt.md

# Skill: Test Standards

## Purpose
Given a C# method signature and a plain-English description of its expected behaviour (derived from the legacy C++ source), produce a complete, named list of xUnit `[Fact]` and `[Theory]` test cases. Every assumption embedded in the test plan is verified by the Falsify Claims skill before the plan is finalised.

## Input (supplied by caller)

```
Method:      <C# method signature>
Description: <plain-English functional description derived from Legacy/>
```

## Procedure

### Step 1 — Identify test categories

For the given method, enumerate tests across all applicable categories:

| Category | Examples |
|---|---|
| Happy path | Typical inputs → expected output |
| Identity elements | Adding zero, multiplying by one |
| Commutativity / Associativity | A+B == B+A, (A+B)+C == A+(B+C) |
| Boundary values | Zero, one, max representable, min representable |
| Sign handling | Positive × negative, negative × negative |
| Zero edge cases | Division by zero, zero factorial |
| Parse / Format round-trips | `Parse(x.ToString()) == x` |
| Divide-by-zero / Overflow equivalents | Throws expected exception |
| Large-number correctness | Values that exceed `ulong.MaxValue` |

Only include categories relevant to the method's semantics.

### Step 2 — Name each test

Use the convention: `MethodName_GivenScenario_ExpectedResult`

Examples:
- `Add_GivenTwoPositiveNumbers_ReturnsSumWithNoLeadingZeros`
- `IsZero_GivenDefaultInstance_ReturnsTrue`
- `Parse_GivenRoundTrip_ReturnsOriginalValue`

### Step 3 — State the assumption for each test

For each named test, write one sentence describing the assumption it encodes.

### Step 4 — Run Falsify Claims on all assumptions

Collect every assumption from Step 3 into a numbered list and invoke the Falsify Claims skill on that list.  
If any assumption is Falsified, revise the affected test cases and repeat Step 4 until zero Falsified rows remain.

## Output Format

After all assumptions are Supported, output:

```
### Test Plan for `<MethodName>`

1. `Add_GivenTwoPositiveNumbers_ReturnsSumWithNoLeadingZeros`  
   *Assumption*: Adding two positive Natural numbers produces a Natural with the mathematically correct digit sequence and no leading zeros.

2. `Add_GivenZeroAndN_ReturnsN`  
   *Assumption*: Zero is the additive identity — `0 + N == N` for all N.

...
```

Then state: "All assumptions Supported. Test plan is ready."
