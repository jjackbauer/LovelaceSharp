# Round 11 — the differential oracle can no longer pass without running

One bounded change in three parts: the oracle's unavailable case is now a reported SKIP (or a hard
FAILURE when required), the corpus was widened from 2 corpora to 6 under explicitly stated
domains, and a CI job now installs SymPy and runs the oracle with `LOVELACE_REQUIRE_SYMPY=1`.

Labels: **observed** = the command was run in this workspace and its output is pasted below;
**derived** = reading of the cited source or a hand computation, not a separate execution of the
deliverable.

---

## 0. Files

| File | Change |
| --- | --- |
| `Lovelace.Symbolics.Tests/SympyOracle.cs` | **new** — environment probe, `RequiresSympyFactAttribute`, evaluation session, numeric adjudication |
| `Lovelace.Symbolics.Tests/OracleCorpus.cs` | **new** — the six corpora; every case carries its SymPy expression and its domain |
| `Lovelace.Symbolics.Tests/DifferentialOracleTests.cs` | rewritten — 6 oracle tests (2 kept, 4 added), every one guarded by the attribute |
| `.github/workflows/ci.yml` | **new job** `sympy-oracle` (the other three jobs untouched) |
| `docs/goal-cycle-3/round-11/implementation.md` | this document |

---

## 1. Part 1 — the unavailable-oracle case is loud and conditional

* `SympyOracle.ProbeOnce` (`Lovelace.Symbolics.Tests/SympyOracle.cs:66`) probes python3 **and then
  `import sympy` inside it**. A python3 without sympy is reported as ABSENT, with the reason
  naming what was missing: no interpreter / an interpreter that exits non-zero / an interpreter
  whose `import sympy` fails.
* `RequiresSympyFactAttribute` (`SympyOracle.cs:339`) decides **at construction time**, exactly
  like the existing precedent `Lovelace.Run.Tests/StdoutPurityTests.cs`
  (`RequiresRunnerProcessFactAttribute`): if the probe failed and `LOVELACE_REQUIRE_SYMPY` is not
  `"1"`, it sets `Skip` to a message naming what was missing, so xUnit reports **SKIPPED**.
* If the probe failed **and** `LOVELACE_REQUIRE_SYMPY == "1"`, the attribute sets no skip; the
  body runs and `SympyOracle.Require()` (`SympyOracle.cs:46`) throws
  `InvalidOperationException("LOVELACE_REQUIRE_SYMPY=1 requires the SymPy oracle to RUN, but it is
  unavailable: ... The differential oracle would compare nothing, so it fails instead of reporting
  a pass.")`.
* `Require()` is the **only** way to obtain a `SympySession`, so a test body cannot evaluate
  anything without passing through it. That is the structural backstop: even if the attribute were
  replaced by a plain `[Fact]`, the body would throw rather than return quietly.

The silent path that was removed: the pre-change file used plain `[Fact]` methods whose first
statement was `if (SympyProbe() is null) return;` (`DifferentialOracleTests.cs:61-66` and
`:88-92` as it stood before this change — **derived**, from the file as read at the start of this
session). xUnit reports an early `return` from a `[Fact]` as **passed**, which is precisely the
green-everywhere, never-executed oracle this round eliminates.

## 2. Part 1 — the two observed runs (this machine)

### Run A — no environment variable: SKIPPED, not passed (**observed**)

```
> dotnet test Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.csproj -c Release --nologo --filter "FullyQualifiedName~DifferentialOracle" --logger "console;verbosity=detailed"
[xUnit.net 00:00:00.28]     Lovelace.Symbolics.Tests.DifferentialOracleTests.SympyOracle_Solve_Agrees [SKIP]
[xUnit.net 00:00:00.28]       SymPy oracle unavailable: python3 is on PATH but is not a working interpreter (exit 9009): Python was not found; run without arguments to install from the Microsoft Store, or disable this shortcut from Settings > Apps > Advanced app settings > App execution aliases.. These tests compare kernel results against SymPy and cannot run here, so they are reported as SKIPPED, not passed. Set LOVELACE_REQUIRE_SYMPY=1 to make an unavailable oracle a failure.
[xUnit.net 00:00:00.29]     Lovelace.Symbolics.Tests.DifferentialOracleTests.SympyOracle_Roots_Agree [SKIP]
[xUnit.net 00:00:00.29]       SymPy oracle unavailable: python3 is on PATH but is not a working interpreter (exit 9009): ... (identical reason)
[xUnit.net 00:00:00.29]     Lovelace.Symbolics.Tests.DifferentialOracleTests.SympyOracle_MatrixOperations_Agree [SKIP]
[xUnit.net 00:00:00.29]     Lovelace.Symbolics.Tests.DifferentialOracleTests.SympyOracle_Derivatives_Agree [SKIP]
[xUnit.net 00:00:00.29]     Lovelace.Symbolics.Tests.DifferentialOracleTests.SympyOracle_Limits_Agree [SKIP]
[xUnit.net 00:00:00.29]     Lovelace.Symbolics.Tests.DifferentialOracleTests.SympyOracle_Factorization_Agrees [SKIP]

  Skipped Lovelace.Symbolics.Tests.DifferentialOracleTests.SympyOracle_Solve_Agrees [1 ms]
  Error Message:
   SymPy oracle unavailable: python3 is on PATH but is not a working interpreter (exit 9009): Python was not found; run without arguments to install from the Microsoft Store, or disable this shortcut from Settings > Apps > Advanced app settings > App execution aliases.. These tests compare kernel results against SymPy and cannot run here, so they are reported as SKIPPED, not passed. Set LOVELACE_REQUIRE_SYMPY=1 to make an unavailable oracle a failure.
  (the same "Skipped ... Error Message:" block is printed for Roots, MatrixOperations, Derivatives, Limits, Factorization)

Test Run Successful.
Total tests: 6
    Skipped: 6
 Total time: 0.9691 Seconds
```

**Observed result: 6 tests, 6 skipped, 0 passed.** No oracle test is reported as passed here.

### Run B — `LOVELACE_REQUIRE_SYMPY=1`: FAILED (**observed**)

```
> $env:LOVELACE_REQUIRE_SYMPY='1'
> dotnet test Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.csproj -c Release --nologo --filter "FullyQualifiedName~DifferentialOracle"
[xUnit.net 00:00:00.29]     Lovelace.Symbolics.Tests.DifferentialOracleTests.SympyOracle_Solve_Agrees [FAIL]
[xUnit.net 00:00:00.29]     Lovelace.Symbolics.Tests.DifferentialOracleTests.SympyOracle_Roots_Agree [FAIL]
[xUnit.net 00:00:00.29]     Lovelace.Symbolics.Tests.DifferentialOracleTests.SympyOracle_MatrixOperations_Agree [FAIL]
[xUnit.net 00:00:00.29]     Lovelace.Symbolics.Tests.DifferentialOracleTests.SympyOracle_Derivatives_Agree [FAIL]
[xUnit.net 00:00:00.29]     Lovelace.Symbolics.Tests.DifferentialOracleTests.SympyOracle_Limits_Agree [FAIL]
[xUnit.net 00:00:00.30]     Lovelace.Symbolics.Tests.DifferentialOracleTests.SympyOracle_Factorization_Agrees [FAIL]
  Failed Lovelace.Symbolics.Tests.DifferentialOracleTests.SympyOracle_Solve_Agrees [2 ms]
  Error Message:
   System.InvalidOperationException : LOVELACE_REQUIRE_SYMPY=1 requires the SymPy oracle to RUN, but it is
unavailable: python3 is on PATH but is not a working interpreter (exit 9009): Python was not found; run without
arguments to install from the Microsoft Store, or disable this shortcut from Settings > Apps > Advanced app settings >
App execution aliases.. The differential oracle would compare nothing, so it fails instead of reporting a pass.
  (the same "Failed ... InvalidOperationException" block is printed for all six tests)

Failed!  - Failed:     6, Passed:     0, Skipped:     0, Total:     6, Duration: 23 ms - Lovelace.Symbolics.Tests.dll (net10.0)
dotnet test exit code: 1
```

**Observed result: 6 failed, 0 passed, 0 skipped, exit code 1.** A run that requires the oracle and
cannot get it now fails the build.

Why python3 is unusable here (**observed**, plain pwsh, no test involved):

```
> python3 -c "print(1)"
exit=9009
Python was not found; run without arguments to install from the Microsoft Store, or disable this shortcut from Settings > Apps > Advanced app settings > App execution aliases.
```

## 3. Part 2 — the corpus, and the domain every case is compared under

Six corpora live in `Lovelace.Symbolics.Tests/OracleCorpus.cs`. Each case is a record that
**requires** a non-empty `Domain` and a non-empty SymPy expression, so a case cannot be added
without stating them. Every mismatch message names the kernel canonical form and the SymPy
expression, the sample point and both values (`OracleCompare.NumberListsAgree`,
`SympyOracle.cs:246`).

### 3.1 Derivatives — 6 cases, kept (`OracleCorpus.cs:57`)

Domain: the derivative is compared with `sympy.diff(expr, x)` at rational points where the
function is real and differentiable; tolerance 1e-13 at 40-digit working precision.

| SymPy expression | domain / branch assumption |
| --- | --- |
| `x**7` | polynomial, entire; x = 1/3, -1/2, 7/5 |
| `sin(x**2)` | entire; x = 1/3, -1/2, 7/5 |
| `exp(-x**2)` | entire; x = 1/3, -1/2, 7/5 |
| `x*log(x)` | **x > 0** (principal real branch); x = 1/3, **4/5**, 7/5 — see finding F2 |
| `sin(x)/x` | x ≠ 0 (removable singularity not sampled); x = 1/3, -1/2, 7/5 |
| `tan(x**3)` | away from x³ = π/2 + kπ; x = 1/3, -1/2, 7/5 |

The six expressions, the three-point numeric comparison and its tolerance are unchanged from the
original corpus; only the log case's sample points moved onto its stated domain.

### 3.2 Solve — 4 cases, kept and strengthened (`OracleCorpus.cs:85`)

Domain: the complete solution set over the **complex** field (the kernel default). Compared with
`sympy.solve` on the **distinct-root count** (new: the old corpus consulted SymPy only for its
failure message), and every kernel root is still substituted back and must expand to exactly 0.

| SymPy equation | expected distinct roots |
| --- | --- |
| `x**2 - 4*x + 4` | 1 (the double root 2) |
| `x**2 - 5*x + 6` | 2 |
| `x**3 + 1` | 3 (complex radicals) |
| `x**4 - 5*x**2 + 4` | 4 |

### 3.3 Roots — 8 cases, new (`OracleCorpus.cs:100`)

Domain: the complete set of **distinct real roots** (`SolveDomain.Real`; a degree ≥ 4 factor comes
back as `RootOf`, which carries no multiplicity, so the distinct-set reading is applied on both
sides). Compared with `sympy.real_roots` on the count and on every value at 30 significant digits
(tolerance 1e-20); closed-form roots additionally have to expand the polynomial to exactly 0.

| SymPy polynomial | kernel behaviour expected |
| --- | --- |
| `x**4 - 5*x**2 + 4` | 4 rational roots |
| `x**2 - 2` | ±√2 as radicals |
| `x**4 - 2` | 2 roots as RootOf |
| `x**5 - 3*x + 1` | 3 roots as RootOf (irreducible quintic) |
| `x**3 - 2` | ∛2 |
| `x**3 - 3*x**2 + 3*x - 1` | the triple root 1 is ONE distinct value |
| `x**5 - x - 1` | 1 root as RootOf |
| `x**2 + 1` | no real root: kernel `NoSolutions`, sympy `[]` |

### 3.4 Factorization — 9 cases, new (`OracleCorpus.cs:123`)

Domain: univariate polynomial over **Q[x]**. The degree/multiplicity profile of the kernel
factorization (encoded `ddd:ddd`, sorted, `OracleCorpus.DegreeProfile`, `OracleCorpus.cs:292`) is
compared with `sympy.factor_list`, **and** the kernel product must expand back to the exact input
polynomial. A factorization that returns its input unchanged fails both checks.

| SymPy polynomial | factor profile `degree:multiplicity` |
| --- | --- |
| `x**2 - 5*x + 6` | 1:1; 1:1 |
| `x**3 + 1` | 1:1; 2:1 (irreducible residual returned whole) |
| `x**4 - 5*x**2 + 4` | 1:1 ×4 |
| `x**3 - 3*x**2 + 3*x - 1` | 1:3 (square-free decomposition) |
| `2*x**2 - 8` | 1:1; 1:1 with content 2 |
| `x**4 - 1` | 1:1; 1:1; 2:1 |
| `x**2 - 2` | 2:1 (irreducible over Q) |
| `6*x**3 - 6*x` | 1:1 ×3 with content 6 |
| `x**3 + x**2 - x - 1` | 1:1; 1:2 |

Not included, with the reason recorded in the corpus itself (`OracleCorpus.cs:129-133`): a
polynomial with a negative leading coefficient — see finding F1.

### 3.5 Limits — 17 cases, new (`OracleCorpus.cs:151`)

Domain: every case declares its direction and its comparison. A two-sided case is only in the
corpus where both sides agree, so `sympy.limit(dir='+')` is the value the kernel's `TwoSided`
evaluation must produce; a two-sided case at an **odd** pole expects `DoesNotExist` and compares
**both** one-sided results against `sympy.limit(dir='-')` and `(dir='+')`.

| kernel limit (SymPy expression, point, direction) | expectation |
| --- | --- |
| `sin(x)/x`, x→0, two-sided | 1 |
| `(1 - cos(x))/x**2`, x→0, two-sided | 1/2 |
| `(x**3 - 8)/(x - 2)`, x→2, two-sided | 12 |
| `(x**2 - 1)/(x - 1)`, x→1, two-sided | 2 |
| `tan(x)/x`, x→0, two-sided | 1 |
| `1/x**2`, x→0, two-sided (even pole) | +∞ |
| `1/(x - 1)**2`, x→1, two-sided (even pole) | +∞ |
| `1/x`, x→0, from the right | +∞ |
| `1/x`, x→0, from the left | −∞ |
| `1/x`, x→0, two-sided (odd pole) | DoesNotExist, left −∞, right +∞ |
| `1/(x - 1)`, x→1, two-sided (odd pole) | DoesNotExist, left −∞, right +∞ |
| `(3*x**2 + 5*x)/(2*x**2 - x)`, x→+∞ | 3/2 |
| `(3*x**2 + 5*x)/(2*x**2 - x)`, x→−∞ | 3/2 |
| `x/(x**2 + 1)`, x→+∞ | 0 |
| `(2*x**3 - x)/(x**3 + 1)`, x→+∞ | 2 |
| `x**3 - 2*x`, x→+∞ | +∞ |
| `x**3 - 2*x`, x→−∞ | −∞ |

### 3.6 Matrix operations — 13 cases, new (`OracleCorpus.cs:231`)

| operation | SymPy matrix | domain stated |
| --- | --- | --- |
| det | `[[x, 1], [y, x]]` | polynomial identity over Q(x, y), two rational points |
| det | `[[x, 1, 0], [0, x, 1], [1, 0, x]]` | as above |
| det | `[[2, 0, 1], [1, 3, 2], [1, 1, 4]]` | exact rational matrix over Q |
| trace | `[[x, 1], [y, x]]` | polynomial identity over Q(x, y) |
| trace | `[[2, 0, 1], [1, 3, 2], [1, 1, 4]]` | exact rational matrix over Q |
| rank | `[[1, 2], [2, 4]]` | rank over Q, no parameters |
| rank | `[[1, 2, 3], [4, 5, 6], [7, 8, 9]]` | rank over Q, no parameters |
| rank | `[[2, 0, 1], [1, 3, 2], [1, 1, 3]]` | rank over Q, no parameters |
| rank | `[[x, 1], [0, y]]` | **generic** rank over Q(x, y) — the rank for every parameter value outside the vanishing set of the pivot minors, the convention `sympy.Matrix.rank()` applies to symbolic entries |
| rank | `[[x, x], [x, x]]` | generic rank over Q(x, y) |
| inverse | `[[x, 1], [y, x]]` | adjugate/determinant, valid where det = x²−y ≠ 0 (checked at both points); every entry compared row-major |
| inverse | `[[x, 2], [3, y]]` | det = xy−6 ≠ 0 at both points |
| inverse | `[[2, 0, 1], [1, 3, 2], [1, 1, 4]]` | exact rational inverse over Q |

## 4. Part 3 — the CI job (YAML as added)

Inserted into `.github/workflows/ci.yml` between `fast-tests` and `aot-smoke`
(`ci.yml:108-159`); neither existing job was modified.

```yaml
  # ---------------------------------------------------------------------------
  # Differential oracle: SymPy IS installed here, and its absence is FATAL.
  #
  # The oracle compares kernel results against SymPy. Everywhere else the tests
  # report SKIPPED (with the reason) when python3 + sympy are missing — never a
  # silent pass. This job is the one place where the comparison actually runs:
  # python3 is set up, sympy is installed, and LOVELACE_REQUIRE_SYMPY=1 turns a
  # missing oracle into a FAILURE instead of a skip.
  # ---------------------------------------------------------------------------
  sympy-oracle:
    name: Differential oracle (SymPy installed)
    runs-on: ubuntu-latest
    timeout-minutes: 30

    env:
      DOTNET_CLI_TELEMETRY_OPTOUT: '1'
      DOTNET_NOLOGO: '1'
      # Required, not optional: a run that cannot compare anything must fail.
      LOVELACE_REQUIRE_SYMPY: '1'

    steps:
      - name: Checkout
        uses: actions/checkout@v4

      - name: Set up .NET
        uses: actions/setup-dotnet@v4
        with:
          dotnet-version: '10.0.x'

      # setup-python (not the runner's system python) keeps `pip install` out of
      # PEP 668's externally-managed environment; the oracle spawns `python3`.
      - name: Set up Python
        uses: actions/setup-python@v5
        with:
          python-version: '3.12'

      - name: Install sympy and prove python3 can import it
        shell: bash
        run: |
          set -euo pipefail
          python3 -m pip install --upgrade pip
          python3 -m pip install sympy
          python3 -c "import sympy; print('sympy', sympy.__version__)"

      - name: Run the differential oracle (an unavailable oracle fails the job)
        shell: bash
        run: |
          set -euo pipefail
          dotnet test Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.csproj \
            --configuration Release --nologo \
            --filter "FullyQualifiedName~DifferentialOracle" \
            --logger "console;verbosity=detailed"
```

YAML validation (**observed**, `node -e` with `js-yaml` against the edited file):

```
YAML OK, jobs: fast-tests,sympy-oracle,aot-smoke
sympy-oracle: steps=5 env={"DOTNET_CLI_TELEMETRY_OPTOUT":"1","DOTNET_NOLOGO":"1","LOVELACE_REQUIRE_SYMPY":"1"} runs-on=ubuntu-latest
fast-tests still present: true | aot-smoke still present: true
```

## 5. Build and suite summaries (**observed**)

```
> dotnet build LovelaceSharp.slnx -c Release --nologo
Build succeeded.
    0 Warning(s)
    0 Error(s)
Time Elapsed 00:00:03.64
```

```
> dotnet test Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.csproj -c Release --nologo
Passed!  - Failed:     0, Passed:   434, Skipped:     6, Total:   440, Duration: 13 s - Lovelace.Symbolics.Tests.dll (net10.0)
```

The 6 skipped tests are exactly the oracle tests of run A; before this change they were counted as
passed, which is why the Symbolics total moves from "440 passed, 0 skipped" to "434 passed,
6 skipped" (**derived** for the before-state — the after-state is observed above).

## 6. Findings that must be triaged

### F1 — `Factoring.Factor` loses the sign of a negative leading coefficient (**observed**)

A scratch console probe (outside the repository, `%TEMP%\lo-oracle-probe`, referencing
`Lovelace.Symbolics/Lovelace.Symbolics.csproj`) printed:

```
factor(-x**2 + 1) -> (mul (add (rat -1 1) (sym x)) (add (rat 1 1) (sym x)))
    profile=001:001;001:001 expandsToOriginal=False
```

`(x - 1)*(x + 1)` expands to `x^2 - 1`, not to the input `-x^2 + 1`: the factorization is of the
negated polynomial. The cause is in `Lovelace.Symbolics/Algebra/Factor.cs` (the square-free /
rational-root path normalises to a monic factor and the sign is never put back). **Fixing it is
outside this round's scope**, so no case with a negative leading coefficient is in the
factorization corpus; the exclusion and this reproduction are recorded at
`OracleCorpus.cs:129-133` so the case can be added the moment the kernel is repaired.

### F2 — the derivative corpus compared `x*log(x)` off its branch (**observed**)

The original corpus evaluated every derivative at x ∈ {1/3, −1/2, 7/5}. The same scratch probe,
running the original `EquivalentToSympy` kernel half, printed:

```
d/dx x*log(x) at 1/3  -> Lovelace.Symbolics.NumReal
d/dx x*log(x) at -1/2 -> EXCEPTION EvaluationException: Ln(x) requires x > 0. (Parameter 'x')
d/dx x*log(x) at 7/5  -> Lovelace.Symbolics.NumReal
```

and SymPy's side at that point is `0.693... + 3.14159...*I`, which `Real.Parse` cannot consume. The
comparison was therefore unsound at x = −1/2 and would have thrown the first time the oracle
actually ran. The case is kept, its domain is now stated as x > 0, and it is compared at
x ∈ {1/3, 4/5, 7/5}: same three points, same tolerance, on the branch where both sides are real.

### F3 — the solve corpus's exact-substitution assertion had never executed (**observed**)

Running that assertion's kernel half in the scratch probe produced `ZERO` for all ten
(case, root) pairs, including the cubic radicals of `x**3 + 1`. It is therefore sound to keep it,
and it will execute for the first time in the new CI job.

## 7. What CANNOT be verified on this machine

* **No SymPy comparison has run here, ever.** `python3` on this machine is the Microsoft Store
  app-execution alias stub (observed: exit 9009, "Python was not found"), so the oracle cannot be
  executed locally and the probe correctly reports it as absent. Run A and run B above are
  evidence about the *guard*, not about any kernel-vs-SymPy comparison.
* **The four new corpora have never been executed** — not one of the roots, factorization, limits
  or matrix comparisons, and not the strengthened solve count comparison. They are exercised only
  by the new CI job `sympy-oracle`, which cannot run here either. The SymPy expressions and the
  expected values in §3 are **hand-derived** (plus F1/F2/F3 from the kernel-side probe); the CI run
  is what turns them into verification. If a SymPy expression in the corpus is wrong, CI fails with
  a message naming both forms — the intended triage path.
* **The kernel half of every corpus entry WAS executed** by the scratch probe, and each observed
  kernel value matches the hand-derived SymPy value at the digits shown. Examples (observed):
  `roots(x**2 - 2)` → `-/+1.414213562373095048801688724209`; `roots(x**5 - 3*x + 1)` → 3 RootOf
  values `-1.388791984407254182800056694189`, `0.33473414194335268707509896247328`,
  `1.214648042698461803985828389315`; `limit(1/x, x -> 0, TwoSided)` → `DoesNotExist (left
  MinusInfinity, right PlusInfinity)`; `det([[x,1],[y,x]])` → `x^2 - y`; `rank([[x,1],[0,y]])` → 2.
  This is *not* a differential verification — it is the kernel side only, on a scratch program, and
  it is reported as such.
* **The CI job's execution is unverified.** Its YAML was parsed by `js-yaml` (observed above) and
  the other jobs are intact, but no GitHub Actions run exists in this workspace.

## 8. Forbidden outcomes, checked

* No path reports the oracle as passed when it did not run: the oracle tests are either SKIPPED
  with a reason (run A) or FAILED (run B), and the session handle is only obtainable through
  `Require()` (`SympyOracle.cs:46`).
* No existing oracle case was weakened or deleted: the 6 derivative expressions, the 4 solve
  equations, their tolerance and the exact-substitution assertion are all still present; the solve
  corpus gained the SymPy count comparison (F2/F3 document the only two repairs, both branch/
  domain corrections, both with the kernel half re-observed).
* No corpus entry lacks a domain: the case records require one, and every entry in §3 states it.
* Nothing outside SCOPE was edited; no commit was made.
