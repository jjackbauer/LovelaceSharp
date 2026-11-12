# Round 99 — the six DifferentialOracle rows applied and certified

**Status: 6/6 mappings applied (deferred -> ported), the gate passed on the first attempt, and
coverage is certified at 6 090/6 209 (98.08 %).** The only remaining deferred row is the empty
`UnitTest1` placeholder, which was **not** mapped; its exact source is reproduced in §5.

This lane's scope was `parity/manifest-map.json` plus the generated parity files and this report.
No Rust source, `.cs`, fixture or other doc was edited. `Lovelace.Representation.Tests/UnitTest1.cs`
is untouched.

## 1. Result

| Item | Before | After |
|---|---|---|
| Deferred census rows | 7 | 1 |
| Ported census rows | 6 084 | 6 090 |
| Coverage | 6 084/6 209 (97.99 %) | **6 090/6 209 (98.08 %)** |
| Excluded | 118 | 118 (unchanged) |

* Exactly the six `DifferentialOracleTests` ids were changed: `deferred -> ported`, with the
  `rust` test id added, the round-45 block reason replaced and an evidence note written.
  `git diff --numstat -- parity/manifest-map.json` is `24 18` — six entries, +4/−3 lines each,
  no other key touched.
* `Lovelace.Representation.Tests/.../UnitTest1/Test1#0` remains `deferred` with **no `rust`
  id**: its C# body is empty, so it needs a disposition decision, not a test.

Generated files rewritten by the certification sequence:

| File | Change |
|---|---|
| `parity/manifest.toml` | six rows regenerated from the map (`24 18` lines) |
| `parity/rust-tests.json` | fresh inventory: 11 crates, 5 666 listed tests, suite exit 0, 5 665 passed |
| `parity/coverage.md` | coverage report from the generated manifest (`2 2` lines) |

## 2. Per-test verification (read, not inferred from names)

The module is registered at `rust/lovelace-lang/src/lib.rs:2974`
(`mod round98_differential_oracle;`). Each test below was read in full; it embeds the SymPy 1.14.0
session's ground truth as literal expected values and applies the C# row's comparison rule. The C#
rows at the cited lines call `SympyOracle.Require()` and compare against a live SymPy session.

| Census id (short) | Rust test (file:line) | What the test actually asserts |
|---|---|---|
| `SympyOracle_Derivatives_Agree#0` | `rust/lovelace-lang/src/round98_differential_oracle.rs:284` `sympyoracle_derivatives_agree` | 6 expressions (x^7, sin(x^2), exp(-x^2), x*log(x), sin(x)/x, tan(x^3)) × 3 rational points; canonical print, reparse, evaluate; real **and** imaginary parts at `REAL_TOLERANCE` 1e-13 against SymPy literals (including the `x*log(x)` imaginary part at −1/2). |
| `SympyOracle_Solve_Agrees#0` | `...:352` `sympyoracle_solve_agrees` | 7 equations; distinct canonical-root count vs `sympy.solve` (1,2,3,4,2,2,0); every kernel root substituted back, expanded, must be exactly 0; `sqrt(x) + 2` must be `Unevaluated` and must not claim completeness (SymPy's set is empty). |
| `SympyOracle_Roots_Agree#0` | `...:455` `sympyoracle_roots_agree` | 8 polynomials; distinct real values vs `sympy.real_roots` at `TIGHT_TOLERANCE` 1e-20; closed forms substituted back exactly; `x^2 + 1` has zero real roots. |
| `SympyOracle_Factorization_Agrees#0` | `...:575` `sympyoracle_factorization_agrees` | 9 polynomials; sorted `ddd:ddd` degree:multiplicity profile vs `sympy.factor_list`; factorization must expand back to the exact input. |
| `SympyOracle_Limits_Agree#0` | `...:789` `sympyoracle_limits_agree` | 24 limits; finite values at 1e-20; `oo`/`-oo` matched by status; a two-sided `DoesNotExist` must carry the one-sided values (left −inf, right +inf). |
| `SympyOracle_MatrixOperations_Agree#0` | `...:1010` `sympyoracle_matrixoperations_agree` | 13 matrix cases: 3 determinants, 2 traces, 3 inverses at the corpus points (both parts at 1e-13) and 5 ranks compared exactly. |

All six ids resolve in the freshly generated `parity/rust-tests.json` (checked as
`lovelace-lang::round98_differential_oracle::<fn>`): 6/6 FOUND.

## 3. Key diff — the six rows

Each row in `parity/manifest-map.json`: `disposition` `deferred -> ported`, `rust` added,
`reason` replaced, `note` replaced with the C#-row/Rust-test evidence.

| # | Census id (class/method segment) | `rust` test id | C# line | Rust line |
|---:|---|---|---:|---:|
| 1 | `DifferentialOracleTests/SympyOracle_Derivatives_Agree#0` | `lovelace-lang::round98_differential_oracle::sympyoracle_derivatives_agree` | 26 | 284 |
| 2 | `DifferentialOracleTests/SympyOracle_Solve_Agrees#0` | `lovelace-lang::round98_differential_oracle::sympyoracle_solve_agrees` | 45 | 352 |
| 3 | `DifferentialOracleTests/SympyOracle_Roots_Agree#0` | `lovelace-lang::round98_differential_oracle::sympyoracle_roots_agree` | 89 | 455 |
| 4 | `DifferentialOracleTests/SympyOracle_Factorization_Agrees#0` | `lovelace-lang::round98_differential_oracle::sympyoracle_factorization_agrees` | 148 | 575 |
| 5 | `DifferentialOracleTests/SympyOracle_Limits_Agree#0` | `lovelace-lang::round98_differential_oracle::sympyoracle_limits_agree` | 170 | 789 |
| 6 | `DifferentialOracleTests/SympyOracle_MatrixOperations_Agree#0` | `lovelace-lang::round98_differential_oracle::sympyoracle_matrixoperations_agree` | 196 | 1 010 |

Representative map hunk (row 1); the other five differ only in id, test name and evidence line:

```diff
   "Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.DifferentialOracleTests/SympyOracle_Derivatives_Agree#0": {
-    "disposition": "deferred",
-    "reason": "Round 45 UNRUNNABLE: the C# row exercises a library-only surface needing SymPy oracle session (SympyOracle.Require); no Rust API exists to assert the row's contract. Documented block, not an approximation. See docs/goal-parity-contract/round-45/final-sweep.md.",
-    "note": "Library-only census row; grouped by missing API in the round-45 final sweep."
+    "disposition": "ported",
+    "rust": "lovelace-lang::round98_differential_oracle::sympyoracle_derivatives_agree",
+    "note": "C#: Lovelace.Symbolics.Tests/DifferentialOracleTests.cs:26 ... Rust: rust/lovelace-lang/src/round98_differential_oracle.rs:284 ...",
+    "reason": "Ported by round 98: the differential-oracle row now resolves to the embedded SymPy 1.14.0 Rust test (see note; docs/goal-parity-contract/round-98/differential-oracle.md)."
   },
```

## 4. Certification — observed gate lines

Sequence run from `C:\Users\ricar\dev\LovelaceSharp` with the task's toolchain environment
(`RUSTUP_HOME`, `CARGO_HOME`, `PATH`, `TMP`/`TEMP` under `rust\target\tmp`). The Windows
PowerShell script was run with `-ExecutionPolicy Bypass`; the `node` gates were invoked directly.

```text
> powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\parity\gen-rust-tests.ps1
crates=11 tests=5666 suiteExit=0 passed=5665 failed=0

> node parity/gen-manifest.mjs
{
  "rows": 6209,
  "dispositions": {
    "excluded": 118,
    "ported": 6090,
    "deferred": 1
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
    "excluded": 118,
    "deferred": 1
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

No `stale` line appeared, so no retry was needed: `gen-rust-tests.ps1` rewrote
`sourcesSha256` from the current Rust sources before the gate compared it.

## 5. `UnitTest1` — exact source (NOT mapped)

Census row (from `parity/census.json:39319`):

| Field | Value |
|---|---|
| id | `Lovelace.Representation.Tests/Lovelace.Representation.Tests.UnitTest1/Test1#0` |
| project | `Lovelace.Representation.Tests` |
| class | `Lovelace.Representation.Tests.UnitTest1` |
| method | `Test1` |
| caseIndex | `0` |
| dataKind | `fact` |
| sourceFile | `Lovelace.Representation.Tests/UnitTest1.cs` |
| sourceLine | `7` |

File facts: 130 bytes, UTF-8 **with BOM**, CRLF line endings, trailing newline;
SHA-256 `ea227974cd89249a66dac12bb53f774ee85260512a6e8be358a9abe35bbb2e91`; committed in
`6d61d8e` (2026-03-04, "Feat(Representation): impl"), working tree clean.

Verbatim content of `Lovelace.Representation.Tests/UnitTest1.cs` (all 10 lines):

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

Observations, stated only from the source: it is a `public` xunit class with no base type and no
other members; the single method is `public void Test1()` with a bare `[Fact]` attribute (xunit is
imported project-wide by `<Using Include="Xunit" />` in `Lovelace.Representation.Tests.csproj`);
the body contains **only a blank line** — no statement, no assertion, no observable, no fixture
dependency. The row stays `deferred` in `parity/manifest-map.json` pending its disposition decision.
