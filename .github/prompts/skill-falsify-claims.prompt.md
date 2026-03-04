---
agent: agent
description: Verify or refute a list of claims against the legacy C++ source and existing C# code.
---

#file:.github/prompts/legacy-knowledge-map.md
#file:.github/prompts/skill-use-digit-store.prompt.md

# Skill: Falsify Claims

## Purpose
Given a list of claims about the codebase (behaviour, naming, structure, or logic), search the legacy C++ files in `Legacy/` and the existing C# files to find supporting evidence or a concrete counterexample. Classify each claim as **Supported** or **Falsified**.

## Input
A numbered list of claims, supplied by the caller. Example:

```
1. `somar` always produces a result with no leading zeros.
2. The BCD low nibble sentinel value for an unused slot is 0x0F.
3. `InteiroLovelace::fatorial` delegates to `Lovelace::fatorial`.
```

## Procedure

For each claim:

1. **Locate evidence** — search `Legacy/*.hpp`, `Legacy/*.cpp`, and all `*.cs` files for code that directly supports or contradicts the claim.
2. **Attempt a counterexample** — try to construct a concrete input or scenario where the claim would be violated.
3. **Classify**:
   - **Supported** — at least one code location confirms the claim and no counterexample was found. Record `file:line`.
   - **Falsified** — a counterexample exists, or the claim contradicts source code. Record the reason and the contradicting `file:line`.

## Output Format

Produce a Markdown table:

| # | Claim | Evidence (file:line) | Status | Reason |
|---|---|---|---|---|
| 1 | ... | `Legacy/Lovelace.cpp:42` | ✅ Supported | Confirmed by `setBitwise` implementation |
| 2 | ... | — | ❌ Falsified | Sentinel is 0x0C for expansion, 0x0F only on reduction (see `Legacy/Lovelace.cpp:25`) |

## Loop Instruction

After producing the table, state the count of Falsified rows.  
If any rows are Falsified, instruct the caller:  
> "Revise the following claims and re-run this skill until zero Falsified rows remain: [list claim numbers]"

Do not proceed past this skill until zero rows are Falsified.
