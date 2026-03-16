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

## Agent Roles

This skill uses a 4-agent parallel architecture:

- **Falsifier A** — reviews **all** claims independently. Runs in parallel with B and C.
- **Falsifier B** — reviews **all** claims independently. Runs in parallel with A and C.
- **Falsifier C** — reviews **all** claims independently. Runs in parallel with A and B.
- **Synthesizer** — runs sequentially after all 3 Falsifiers complete. Reconciles the 3 independent tables, resolves disagreements, and applies the Loop Instruction.

Each Falsifier works from the full claim list with no overlap restriction. The Synthesizer uses the independent results to produce a higher-confidence verdict for each claim.

## Procedure

1. **Dispatch in parallel** — launch Falsifier A, Falsifier B, and Falsifier C simultaneously, each receiving the full claim list.  
   Each Falsifier, for every claim:  
   a. **Locate evidence** — search `Legacy/*.hpp`, `Legacy/*.cpp`, and all `*.cs` files for code that directly supports or contradicts the claim.  
   b. **Attempt a counterexample** — try to construct a concrete input or scenario where the claim would be violated.  
   c. **Classify**:  
      - **Supported** — at least one code location confirms the claim and no counterexample was found. Record `file:line`.  
      - **Falsified** — a counterexample exists, or the claim contradicts source code. Record the reason and the contradicting `file:line`.  
   d. Return a full Markdown table (same schema as the Output Format) covering all claims.

2. **Synthesize** — after all 3 Falsifiers complete, launch the Synthesizer agent:
   - For each claim, compare the 3 independent verdicts.
   - If all 3 agree, use the consensus verdict and representative evidence.
   - If there is disagreement, mark the claim **Falsified** (conservative default) and note the conflicting findings as the reason.
   - Produce the final merged table sorted by claim number.
   - Apply the Loop Instruction (see below).

## Output Format

Each **Falsifier** produces a full table covering all claims.

The **Synthesizer** produces the final reconciled table — this is the skill's authoritative output:

| # | Claim | Evidence (file:line) | Status | Reason |
|---|---|---|---|---|
| 1 | ... | `Legacy/Lovelace.cpp:42` | ✅ Supported | Confirmed by `setBitwise` implementation |
| 2 | ... | — | ❌ Falsified | Sentinel is 0x0C for expansion, 0x0F only on reduction (see `Legacy/Lovelace.cpp:25`) |

## Loop Instruction

Applied by the **Synthesizer** to the merged output only.

After producing the merged table, state the count of Falsified rows.  
If any rows are Falsified, instruct the caller:  
> "Revise the following claims and re-run this skill until zero Falsified rows remain: [list claim numbers]"

Do not proceed past this skill until zero rows are Falsified.
