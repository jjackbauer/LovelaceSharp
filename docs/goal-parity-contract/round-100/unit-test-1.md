# Round 100 — the last census row dispositioned as an exclusion

**Status: 6 090/6 209 ported (98.08 %), 119 excluded, 0 deferred; the manifest gate passes with
`failures: []`.** The last row, `Lovelace.Representation.Tests/UnitTest1/Test1#0`, is a template
placeholder with an empty body; it is recorded as a delegated exclusion (owner `user`), not as a
Rust test.
Scope was `parity/manifest-map.json` plus the generated parity files and this report. No Rust
source, `.cs`, fixture or other doc was edited.

## 1. The row and why it is excluded

Census row (`parity/census.json:39319-39328`): id
`Lovelace.Representation.Tests/Lovelace.Representation.Tests.UnitTest1/Test1#0`, source
`Lovelace.Representation.Tests/UnitTest1.cs`, `sourceLine` 7, `dataKind` `fact`.

Observed source (`Lovelace.Representation.Tests/UnitTest1.cs:1-10`, verbatim):

```csharp
namespace Lovelace.Representation.Tests;

public class UnitTest1
{
    [Fact]
    public void Test1()
    {

    }
}
```

`Test1`'s body (lines 7-9) is `{`, a blank line, `}` — no statements, no assertions, no
observable behaviour. There is nothing to reproduce. A Rust test with no assertion would pass
without being able to fail, which the failability rule forbids. Exclusion is therefore the honest
disposition, and it is reversible.

## 2. Mechanism followed (read, not assumed)

* `parity/proposed-exclusions.toml:2-4` declares itself **INERT**: "STATUS: pending-user-approval.
  This file is INERT: parity/gen-manifest.mjs does not read it, so no row becomes an approved
  exclusion until the user approves and the rows are applied with owner = "user" ...". Its four
  groups are the record of the 118 absent-project cases, not a live input.
* The live mechanism is `parity/manifest-map.json`: `parity/gen-manifest.mjs:25` loads it as
  `curated`; line 34 gives curated entries precedence (`const cur = curated[c.id] || {}`); line 39
  takes `cur.disposition`.
* `parity/tools/gate-manifest.mjs:51-54` requires every `excluded` row to carry a `reason` and
  `owner === "user"`.
* All 118 existing exclusions use one shape: `disposition: "excluded"`, `owner: "user"`, a reason
  ending *Excluded under the user's delegated authority of 2026-11-10 ("do the most reasonable
  choices and go on").* and a note *Delegated exclusion; reversible by the user. <project>*. This
  entry follows that shape exactly.

The TOML was deliberately not used: it changes no disposition (it is inert) and its grouping
("projects with no Rust equivalent") does not describe this row. No existing exclusion was altered;
exactly one map entry changed (+4/−3).

## 3. The entry

```diff
   "Lovelace.Representation.Tests/Lovelace.Representation.Tests.UnitTest1/Test1#0": {
-    "disposition": "deferred",
-    "reason": "Round 45 UNRUNNABLE: the C# row exercises a library-only surface needing Lovelace.Representation crate (the C# case has an empty body and no observable); no Rust API exists to assert the row's contract. Documented block, not an approximation. See docs/goal-parity-contract/round-45/final-sweep.md.",
-    "note": "Library-only census row; grouped by missing API in the round-45 final sweep."
+    "disposition": "excluded",
+    "owner": "user",
+    "reason": "The C# source is a template placeholder with an empty body: `public class UnitTest1 { [Fact] public void Test1() { } }` (Lovelace.Representation.Tests/UnitTest1.cs:7-9 is `{`, a blank line, `}` with no statements). It asserts no observable behaviour, so there is nothing to reproduce; a vacuous Rust test that asserts nothing would violate the rule that every test must be able to fail. Excluded under the user's delegated authority of 2026-11-10 (\"do the most reasonable choices and go on\").",
+    "note": "Delegated exclusion; reversible by the user. Lovelace.Representation.Tests"
   },
```

Generated row: `parity/manifest.toml:29412-29419` — `disposition = "excluded"`,
`targetPhase = 0`, `note`, `owner = "user"`, and no `rust` id.

## 4. Certification — observed lines

Environment: `RUSTUP_HOME`/`CARGO_HOME` under `.toolchain`, `TMP`/`TEMP` = `rust\target\tmp`.
The host blocks unsigned scripts, so `gen-rust-tests.ps1` was run under a process-scoped
`Set-ExecutionPolicy Bypass`; the node steps were invoked directly.

```text
> (Set-ExecutionPolicy -Scope Process -ExecutionPolicy Bypass); & .\parity\gen-rust-tests.ps1
crates=11 tests=5666 suiteExit=0 passed=5665 failed=0

> node parity/gen-manifest.mjs
{
  "rows": 6209,
  "dispositions": {
    "excluded": 119,
    "ported": 6090
  },
  "coveragePercent": 98.08,
  "rustTests": 5666
}

> node parity/tools/gate-manifest.mjs
{
  "censusCases": 6209,
  "manifestRows": 6209,
  "dispositions": {
    "ported": 6090,
    "consolidated": 0,
    "excluded": 119,
    "deferred": 0
  },
  "coveragePercent": 98.08,
  "claimedRustTests": 6090,
  "projectsInCensus": 15,
  "failures": []
}
PASS (structural): 6209 test cases dispositioned; 6090 claim Rust coverage (98.08%).

> node parity/tools/coverage-report.mjs
coverage.md written: 6090/6209 ported (98.08%)
```

Coverage total (`parity/coverage.md:17,23`):

```text
| Lovelace.Representation.Tests | 91 | 90 | 0 | 1 | 98.90 % | consolidated |
| **TOTAL** | **6209** | **6090** | **0** | **119** | **98.08 %** | |
```

| Item | Before | After |
|---|---|---|
| Ported | 6 090 | 6 090 |
| Excluded | 118 | 119 |
| Deferred | 1 | 0 |
| Coverage | 6 090/6 209 (98.08 %) | 6 090/6 209 (98.08 %) |

**No refusal.** `gen-manifest` accepted the curated row and `gate-manifest` passed; the gate's only
exclusion condition — a reason plus `owner = "user"` — is met, and no workaround was applied.

## 5. Observations and limits

* Baseline gate before the change failed only with `rust-tests.json is stale` (the working tree's
  Rust sources had moved past the recorded inventory), so `gen-rust-tests` was required; after it
  ran, the only gate-state change is this row's disposition.
* `parity/coverage.md:25-27` is emitted by `parity/tools/coverage-report.mjs:44-46` and still
  carries the pre-delegation sentence that the 118 proposed exclusions are "NOT counted above".
  The table does count them (119 now). That text is pre-existing, is not part of the exclusion
  declaration, and was left unchanged; the table numbers are what the gate certifies.
* The excluded row claims no Rust test, so `claimedRustTests` remains 6 090. `parity/proposed-exclusions.toml`
  is untouched.
