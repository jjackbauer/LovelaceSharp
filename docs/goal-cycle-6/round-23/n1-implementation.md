# N-1 (P1) — the trailing line of captured print output was dropped while `hasOutput` said it was written

**Finding**: audit N, wave 5 (round 23), `docs/goal-cycle-6/round-23/audit-N-protocol-fuzz.md` §3 N-1.
**Severity**: **P1** — a false claim in the machine API (`timings[].hasOutput`) and data the script
printed being lost from `output[]`.
**Status**: fixed and verified in a worktree; landing and re-publish follow the two auditors still
reading the published binary (dispatch rule §3.2).

## 1. Reproduced by the orchestrator, twice, with controls

`out\aot\Lovelace.Run.exe` (5 764 608 bytes, 2026-09-12 21:05:23), scripts through `--file`
(PS 5.1 strips embedded quotes from `--eval` argv — EVD-299). Transcript:
`round-23/n1-reverify.txt`, two identical passes.

| script | `output[]` | `timings[].hasOutput` |
|---|---|---|
| `print("a"); print()` | `["a"]` | `(true, true)` ← the defect |
| `print("a"); print(); print()` | `["a"]` | `(true, true, true)` |
| `print("a"); print(""); print("b")` | `["a","","b"]` | `(true, true, true)` — interior blank KEPT |
| `print("a"); print(); det(1)` (exit 1) | `["a"]` | `(true, true, false)` |
| `print()` (single blank) | `[""]` | `(true)` |

So the loss is not a blank-line convention: an **interior** blank line is kept and a single blank
print is kept, while a **trailing** blank line vanishes and the envelope's own flag still says the
statement printed. The line is unrecoverable from the envelope.

**One control of the audit's I could not reproduce, recorded rather than smoothed over.** The audit's
table also lists `print("a\n")` → `output:["a"]` ("the trailing newline the script printed is
dropped"). Through `--file` with the two characters backslash-n, the language does not process the
escape and the element is `"a\\n"` (three characters); the audit's harness passed a **real** LF in
the argv string, which is the case its row describes. The finding does not depend on that row.

## 2. Root cause

`Lovelace.Run/Runner.cs` (both the success path :268 and the failing path :313 read it):

```csharp
private static string[] SplitLines(string text)
{
    if (text.Length == 0) return Array.Empty<string>();
    string[] lines = text.TrimEnd('\r', '\n').Split('\n');   // <-- ALL trailing terminators
    for (int i = 0; i < lines.Length; i++) lines[i] = lines[i].TrimEnd('\r');
    return lines;
}
```

The capture writer terminates **every** printed line with `Environment.NewLine`, so the captured
text is `"a\r\n" + "\r\n"` for `print("a"); print()`; `TrimEnd('\r','\n')` removes **both**
terminators, so the empty line the script printed never becomes an element. The `\r` trim after the
split is the cycle-5 fix for "each element but the last keeps the CR" and stays.

**The same function in both Studio hosts is already correct** — `Lovelace.Studio/EngineHost.cs:319-326`
and `Lovelace.Studio/IncrementalRunner.cs:418-425` normalise CRLF/CR to LF, split, and drop **exactly
one** trailing empty element. The CLI was the only copy that removed more. That is the K-2 shape
again: a host and the CLI must not disagree about one value, and here the hosts had the right answer.

## 3. The fix (exactly one terminator, never two)

```csharp
if (text.EndsWith("\r\n", StringComparison.Ordinal))
    text = text[..^2];
else if (text[^1] is '\n' or '\r')
    text = text[..^1];

string[] lines = text.Split('\n');
for (int i = 0; i < lines.Length; i++) lines[i] = lines[i].TrimEnd('\r');
```

Boundary behaviour, by construction: `"a\r\n"` → `["a"]` (no phantom element);
`"a\r\n\r\n"` → `["a",""]`; `"\r\n"` → `[""]`; `"\r\n\r\n"` → `["",""]`;
`"a\r\nb\r\n"` → `["a","b"]` (still no CR in either element); `""` → `[]`.

## 4. Control tree and fix tree (the cycle's D2 standard)

| tree | what it is | result |
|---|---|---|
| `.worktrees/r23-n1-ctl` @ `dfae21b` | HEAD **with only the new test file added** | **Failed: 4, Passed: 3, Total: 7** |
| `.worktrees/r23-n1` (branch `r23-n1`) | the fix + the same test file | **Passed: 7 / Failed: 0 / Total: 7** |

The four control failures are the defect's own faces: the trailing blank line, two trailing blank
lines, the failing path's trailing blank line, and the count invariant
(`print("a"); print(); print("b"); print()` → the envelope must carry four lines and four statements
say they printed). The three that pass on the control tree are the controls that must NOT change:
`print()` → one empty element, the interior blank line, and `print("a")` → exactly one element.

**No regression, measured in the fix tree:** `Lovelace.Run.Tests` **352 passed / 0 failed** (345 + the
7 new), `Lovelace.Console.Tests` **15/0**, `Lovelace.Studio.Tests` **22/0**.

**Wire evidence on the fix tree's JIT build** (`Lovelace.Run/bin/Release/net10.0/Lovelace.Run.dll`):
`print("a"); print()` → `output:["a",""]`, `hasOutput:(true,true)`;
`print("a"); print(""); print("b")` → `output:["a","","b"]` (unchanged).

## 5. What is deliberately NOT changed

* No existing test was weakened, skipped or re-pinned: `PrintOutputTests`' four cases and
  `PrintOutputOnFailureTests`' four all still pass unchanged, and none of them pinned the lost line.
* `Lovelace.Studio` is untouched — it is the correct reference here, not the defect.
* The audit's other four findings (N-2…N-5) are P2s and are dispositioned in the round-23 triage,
  not silently folded into this fix.
