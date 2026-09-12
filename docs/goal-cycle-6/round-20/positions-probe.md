# Round 20 — the AOT/JIT position discrepancy is a stale artifact, not a regression

**Question.** The freshly published `out/aot/Lovelace.Run.exe` reported `det(1)` at
`{"position":0,"line":1,"column":1}`, while round-19 verification recorded `12/3/1` for the deciding
script. Regression, or two different scripts?

**Verdict: neither a regression nor a discrepancy.** The two numbers are two different scripts, and the
only "divergence" found was a **20-hour-old Debug artifact** that no gate uses.

## 1. Positions on one multi-statement script (`out/posprobe.lv`, 4 statements)

```
det(1)          <- offset 0, fails first
x = 2 + 3       <- offset 7
sqrt(x)         <- offset 17
boom()          <- offset 25
```

| binary | exit | code | category | diagnostics[0] | timings | printed |
|---|---|---|---|---|---|---|
| `out/aot/Lovelace.Run.exe` (published from HEAD, 5 804 544 B) | 1 | `InvalidArgument` | `TypeMismatch` | **0 / 1 / 1** | 1 (pos 0, `Void`) | `[]` |
| `Lovelace.Run/bin/Release/net10.0` (141 s old) | 1 | `InvalidArgument` | `TypeMismatch` | **0 / 1 / 1** | 1 (pos 0, `Void`) | `[]` |
| `Lovelace.Run/bin/Debug/net10.0` (**71 633 s old**, pre-round-19) | 1 | `InternalError` | `InternalInvariantFailure` | 0 / 1 / 1 | 1 | — |
| `Lovelace.Run/bin/Debug/net10.0` (rebuilt, `0 Warning(s) / 0 Error(s)`) | 1 | `InvalidArgument` | `TypeMismatch` | **0 / 1 / 1** | 1 | `[]` |

**`0/1/1` is the correct answer here**: the failing statement is the first one, at byte offset 0, so its
position is 0 — exactly what round 19 pinned as the fix (`EVD-293`). Round 19's `12/3/1` was measured on a
*different* script whose third statement fails.

The stale Debug row also shows what the pre-`63774da` product did with an array argument
(`Unable to cast object of type 'Lovelace.Natural.Natural' to type 'Lovelace.Abstractions.ArrayValue'`
crossing as `InternalError/InternalInvariantFailure`) — that is the closed audit-E finding, reproduced
from a stale artifact, and the rebuilt binary refutes it.

## 2. Why the run stops at the first failing statement, and why that is documented

The probe script has 4 statements; the envelope carries **1** timing and `output: []`. That is the shape
`docs/symbolics/dsh-protocol.md:187-205` documents in its own error example: one successful timing at
position 0, then the failing statement as the last, `Void` timing — nothing after it. `recoverable` is
about the **caller** continuing (`dsh-protocol.md:86`), not about the run resuming. No finding.

## 3. Consequence for the final tree

`bin/Debug` was refreshed before the third audit wave so a 20-hour-old binary cannot be mistaken for the
product. The gate artifacts (`out/aot`, Release) were already consistent: AOT and Release agree on every
deciding field of the probe.
