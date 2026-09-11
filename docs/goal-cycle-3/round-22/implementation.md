# Round 22 — `capabilities()`: the runtime's real-only limits, learnable from structure

STATUS: **DONE** — 7/7 new tests pass, all three suites end 0 failed, the runner envelope is pinned
as a golden fixture.

---

## 1. Objective (one bounded change)

Add a zero-argument structured builtin `capabilities()` returning RecordValue
`CapabilitiesResult`, registered with a `BuiltinDescriptor`, so an agent can learn the runtime's
real-only limits from structure instead of tripping over each one.

Hit ratio: **1 new builtin, 1 new record type, 1 new record type for its entries, 1 new enum type
name.** Nothing else in the kernel changed; complex roots and rational powers of negative bases stay
unsupported (explicitly out of scope).

---

## 2. Step A — test-first, the observed FAILING output

Tests were written first (`Lovelace.Symbolics.Tests/CapabilitiesBuiltinTests.cs`) and run against
the unmodified plugin. Pasted verbatim from that run (stack frames trimmed to the frames that name
each failure; every one of the 7 failures is listed). The raw transcript is also on disk at
`out/r22/stepA-observed-transcript.txt`.

```text
Command:
  dotnet test Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.csproj -c Release --nologo --filter "FullyQualifiedName~CapabilitiesBuiltinTests"

  Failed Lovelace.Symbolics.Tests.CapabilitiesBuiltinTests.AdvertisedCodesAndCategories_MatchTheLiveEnvelope [< 1 ms]
  Error Message:
   System.InvalidOperationException : Unknown function 'capabilities'.
  Stack Trace:
     at Lovelace.Suite.Interpreter.EvaluateCallAsync(CallExpr call, Scope scope) in Lovelace.Suite\Interpreter.cs:line 619
     at Lovelace.Suite.SuiteEngine.Evaluate(String source) in Lovelace.Suite\SuiteEngine.cs:line 290
     at Lovelace.Symbolics.Tests.CapabilitiesBuiltinTests.Capabilities(SuiteEngine engine) in CapabilitiesBuiltinTests.cs:line 40
     at Lovelace.Symbolics.Tests.CapabilitiesBuiltinTests.AdvertisedByOperationClass(SuiteEngine engine) in CapabilitiesBuiltinTests.cs:line 46
     at Lovelace.Symbolics.Tests.CapabilitiesBuiltinTests.AdvertisedCodesAndCategories_MatchTheLiveEnvelope() in CapabilitiesBuiltinTests.cs:line 162

  Failed Lovelace.Symbolics.Tests.CapabilitiesBuiltinTests.Capabilities_ReturnsARecordNamedCapabilitiesResult [< 1 ms]
  Error Message:
   System.InvalidOperationException : Unknown function 'capabilities'.
     at Lovelace.Symbolics.Tests.CapabilitiesBuiltinTests.Capabilities_ReturnsARecordNamedCapabilitiesResult() in CapabilitiesBuiltinTests.cs:line 66

  Failed Lovelace.Symbolics.Tests.CapabilitiesBuiltinTests.Exactness_SaysWhetherTheSetsAreExhaustiveOrBestEffort [< 1 ms]
  Error Message:
   System.InvalidOperationException : Unknown function 'capabilities'.
     at Lovelace.Symbolics.Tests.CapabilitiesBuiltinTests.Exactness_SaysWhetherTheSetsAreExhaustiveOrBestEffort() in CapabilitiesBuiltinTests.cs:line 129

  Failed Lovelace.Symbolics.Tests.CapabilitiesBuiltinTests.UnsupportedOperations_AreRecordsCarryingCodeCategoryAndMessage [< 1 ms]
  Error Message:
   System.InvalidOperationException : Unknown function 'capabilities'.

  Failed Lovelace.Symbolics.Tests.CapabilitiesBuiltinTests.UnsupportedDomains_AreIntegerAndRational_AsDomainValues [< 1 ms]
  Error Message:
   System.InvalidOperationException : Unknown function 'capabilities'.

  Failed Lovelace.Symbolics.Tests.CapabilitiesBuiltinTests.SupportedDomains_AreRealAndComplex_AsDomainValues [< 1 ms]
  Error Message:
   System.InvalidOperationException : Unknown function 'capabilities'.
     at Lovelace.Symbolics.Tests.CapabilitiesBuiltinTests.Capabilities(SuiteEngine engine) in CapabilitiesBuiltinTests.cs:line 40
     at Lovelace.Symbolics.Tests.CapabilitiesBuiltinTests.SupportedDomains_AreRealAndComplex_AsDomainValues() in CapabilitiesBuiltinTests.cs:line 76

  Failed Lovelace.Symbolics.Tests.CapabilitiesBuiltinTests.Capabilities_IsOneRegisteredBuiltin_WithCompleteMetadata [4 ms]
  Error Message:
   Assert.Single() Failure: The collection did not contain any matching items
Expected:   (predicate expression)
Collection: [StateFunction { Name = type, Parameters = <>z__ReadOnlySingleElementList`1[System.String], IsBuiltin = True, Span = , Plugin =  }, StateFunction { Name = inspect, ... }, ...]
  Standard Output Messages:
 builtin count with capabilities(): 104

Failed!  - Failed:     7, Passed:     0, Skipped:     0, Total:     7, Duration: 51 ms - Lovelace.Symbolics.Tests.dll (net10.0)
```

Note the count assertion's own output: **the builtin count before this round is 104**.

---

## 3. The capability table (what `capabilities()` returns)

`CapabilitiesResult` — ordered fields, snake_case on the wire:

| field | kind | value |
|---|---|---|
| `supported_domains` | Array (Vector) of **Domain** | `real`, `complex` |
| `unsupported_domains` | Array (Vector) of **Domain** | `integer`, `rational` |
| `unsupported_operations` | Array (Vector) of **Record** `UnsupportedCapability` | 4 entries, see §4 |
| `exactness` | **Enum** `CapabilitiesExactness` | `BestEffort` |

Each `UnsupportedCapability` record carries, in order:
`operation_class` (Text — the class identifier an agent enumerates on),
`code` (Text — the EXACT wire code the live call produces),
`category` (**Enum** `ErrorCategory`),
`message` (Text — the live human sentence),
`trigger` (Text — a runnable snippet that trips it; `;`-separated statements per the language contract).

The two domain arrays are **exhaustive over the closed `MathDomain` enum** (Integer, Rational, Real,
Complex) — all four members are classified, none is guessed.

---

## 4. Observed live-error codes vs the advertised ones

Every claimed-unsupported operation was RUN and its envelope read **before** the table was written.
The four observations, from the live `Lovelace.Run` envelopes:

| # | operation (probe) | observed envelope `code` | observed `category` | observed message |
|---|---|---|---|---|
| 1 | `sqrt(-1)` | `ArithmeticError` | `DomainError` | Square root is not defined for negative numbers. |
| 2 | `(-1)^(1/2)` | `UnsupportedOperation` | `UnsupportedOperation` | Non-integer exponents are not yet supported. |
| 3 | `x = symbol("x"); solve(x^2 - 2 == 0, x, integer)` | `InvalidOperation` | `DomainError` | solve(): currently supports domains real and complex; got integer. |
| 4 | `x = symbol("x"); solve_full(x^4 - x^2 - 1 == 0, x)` | `solve.unrepresented-roots` (inside `result.structured…diagnostics[0]`; non-fatal, `ok: true`) | `UnsupportedOperation` | complex algebraic roots not supported (RootOf is real-only in v1). |

And what `capabilities()` advertises — **identical strings**:

| `operation_class` | advertised `code` | advertised `category` | match |
|---|---|---|---|
| `complex.sqrt-negative` | `ArithmeticError` | `DomainError` | ✅ observed == advertised |
| `pow.non-integer-exponent` | `UnsupportedOperation` | `UnsupportedOperation` | ✅ observed == advertised |
| `solve.unsupported-domain` | `InvalidOperation` | `DomainError` | ✅ observed == advertised |
| `rootof.complex-algebraic` | `solve.unrepresented-roots` | `UnsupportedOperation` | ✅ observed == advertised |

**The brief's suggested code names do not exist in this runtime.** `complex.sqrt-negative`,
`pow.non-integer-exponent`, `solve.unsupported-domain` and `rootof.complex-algebraic` are matched
by zero files in the repository; advertising them as `code` would have violated the FORBIDDEN
clause. They are therefore carried as the `operation_class` identifier (what the class IS), while
`code`/`category` carry exactly what the live call emits (what an agent will actually match).
Codes 1–3 are the host's structural classification of the escaping kernel exception
(`Runner.Classify`); code 4 is the kernel's own stable diagnostic code.

Why the codes differ in shape (1–3 PascalCase, 4 dotted snake_case): the first three failures are
*thrown*, so they never reach the record vocabulary and are classified by the host from the
exception type; code 4 is a *non-fatal* kernel diagnostic that already had a stable code. The
honesty test pins whatever the runtime really produces, so this asymmetry is visible rather than
papered over.

The honesty test (`AdvertisedCodesAndCategories_MatchTheLiveEnvelope`) covers **all four** entries
(the brief required at least two) and asserts the envelope's `code`, `category` **and `message`**
equal the advertised ones. It reads the envelope from the published runner itself
(`Lovelace.Run.Runner.RunAsync` in-process), not from a re-implementation of the taxonomy — hence
the one added `ProjectReference` in `Lovelace.Symbolics.Tests.csproj`.

---

## 5. Step C — the PASSING output

```text
> dotnet test Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.csproj -c Release --nologo --filter "FullyQualifiedName~CapabilitiesBuiltinTests"
Passed!  - Failed:     0, Passed:     7, Skipped:     0, Total:     7, Duration: 145 ms - Lovelace.Symbolics.Tests.dll (net10.0)
```

Full gate (all four commands run after the change, output pasted from the run):

```text
> dotnet build LovelaceSharp.slnx -c Release --nologo
Build succeeded.
    0 Warning(s)
    0 Error(s)
Time Elapsed 00:00:02.26

> dotnet test Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.csproj -c Release --nologo
Passed!  - Failed:     0, Passed:   456, Skipped:     6, Total:   462, Duration: 13 s - Lovelace.Symbolics.Tests.dll (net10.0)

> dotnet test Lovelace.Suite.Tests/Lovelace.Suite.Tests.csproj -c Release --nologo
Passed!  - Failed:     0, Passed:   637, Skipped:     0, Total:   637, Duration: 8 s - Lovelace.Suite.Tests.dll (net10.0)

> dotnet test Lovelace.Run.Tests/Lovelace.Run.Tests.csproj -c Release --nologo
Passed!  - Failed:     0, Passed:    39, Skipped:     0, Total:    39, Duration: 290 ms - Lovelace.Run.Tests.dll (net10.0)
```

The 6 skipped Symbolics tests are the pre-existing SymPy-oracle cases (skipped because the external
sympy oracle is unavailable in this environment), not tests weakened by this round: the count is
unchanged at 6 before and after.

### Golden fixture row

`Lovelace.Run.Tests/fixtures/capabilities.ls`:

```lovelace
capabilities()
```

`Lovelace.Run.Tests/fixtures/capabilities.json` — the full envelope, volatile keys normalised to
`"<volatile>"` exactly as the corpus contract requires (this is the "runner envelope" for
`capabilities()`, verbatim, and it is asserted byte-for-byte by
`GoldenEnvelopeTests.EnvelopeMatchesGoldenFixture`):

```json
{
  "protocolVersion": 1,
  "symbolicFormatVersion": "#!lovelace-sym 1",
  "mathIrVersion": 2,
  "ok": true,
  "revision": "<volatile>",
  "result": {
    "kind": "Record",
    "display": "CapabilitiesResult(supported_domains: [real, complex], unsupported_domains: [integer, rational], unsupported_operations: [UnsupportedCapability(operation_class: complex.sqrt-negative, code: ArithmeticError, category: DomainError, message: Square root is not defined for negative numbers., trigger: sqrt(-1)), UnsupportedCapability(operation_class: pow.non-integer-exponent, code: UnsupportedOperation, category: UnsupportedOperation, message: Non-integer exponents are not yet supported., trigger: (-1)^(1/2)), UnsupportedCapability(operation_class: solve.unsupported-domain, code: InvalidOperation, category: DomainError, message: solve(): currently supports domains real and complex; got integer., trigger: x = symbol(\"x\"); solve(x^2 - 2 == 0, x, integer)), UnsupportedCapability(operation_class: rootof.complex-algebraic, code: solve.unrepresented-roots, category: UnsupportedOperation, message: complex algebraic roots not supported (RootOf is real-only in v1)., trigger: x = symbol(\"x\"); solve_full(x^4 - x^2 - 1 == 0, x))], exactness: BestEffort)",
    "typed": "CapabilitiesResult(supported_domains: [real, complex], unsupported_domains: [integer, rational], unsupported_operations: [UnsupportedCapability(operation_class: complex.sqrt-negative, code: ArithmeticError, category: DomainError, message: Square root is not defined for negative numbers., trigger: sqrt(-1)), UnsupportedCapability(operation_class: pow.non-integer-exponent, code: UnsupportedOperation, category: UnsupportedOperation, message: Non-integer exponents are not yet supported., trigger: (-1)^(1/2)), UnsupportedCapability(operation_class: solve.unsupported-domain, code: InvalidOperation, category: DomainError, message: solve(): currently supports domains real and complex; got integer., trigger: x = symbol(\"x\"); solve(x^2 - 2 == 0, x, integer)), UnsupportedCapability(operation_class: rootof.complex-algebraic, code: solve.unrepresented-roots, category: UnsupportedOperation, message: complex algebraic roots not supported (RootOf is real-only in v1)., trigger: x = symbol(\"x\"); solve_full(x^4 - x^2 - 1 == 0, x))], exactness: BestEffort) (CapabilitiesResult)",
    "structured": {
      "kind": "Record",
      "type": "CapabilitiesResult",
      "fields": [
        {
          "name": "supported_domains",
          "value": {
            "kind": "Array",
            "type": "Vector",
            "shape": [
              2
            ],
            "elements": [
              {
                "kind": "Domain",
                "domain": "real"
              },
              {
                "kind": "Domain",
                "domain": "complex"
              }
            ]
          }
        },
        {
          "name": "unsupported_domains",
          "value": {
            "kind": "Array",
            "type": "Vector",
            "shape": [
              2
            ],
            "elements": [
              {
                "kind": "Domain",
                "domain": "integer"
              },
              {
                "kind": "Domain",
                "domain": "rational"
              }
            ]
          }
        },
        {
          "name": "unsupported_operations",
          "value": {
            "kind": "Array",
            "type": "Vector",
            "shape": [
              4
            ],
            "elements": [
              {
                "kind": "Record",
                "type": "UnsupportedCapability",
                "fields": [
                  {
                    "name": "operation_class",
                    "value": {
                      "kind": "Text",
                      "value": "complex.sqrt-negative"
                    }
                  },
                  {
                    "name": "code",
                    "value": {
                      "kind": "Text",
                      "value": "ArithmeticError"
                    }
                  },
                  {
                    "name": "category",
                    "value": {
                      "kind": "Enum",
                      "type": "ErrorCategory",
                      "value": "DomainError"
                    }
                  },
                  {
                    "name": "message",
                    "value": {
                      "kind": "Text",
                      "value": "Square root is not defined for negative numbers."
                    }
                  },
                  {
                    "name": "trigger",
                    "value": {
                      "kind": "Text",
                      "value": "sqrt(-1)"
                    }
                  }
                ]
              },
              {
                "kind": "Record",
                "type": "UnsupportedCapability",
                "fields": [
                  {
                    "name": "operation_class",
                    "value": {
                      "kind": "Text",
                      "value": "pow.non-integer-exponent"
                    }
                  },
                  {
                    "name": "code",
                    "value": {
                      "kind": "Text",
                      "value": "UnsupportedOperation"
                    }
                  },
                  {
                    "name": "category",
                    "value": {
                      "kind": "Enum",
                      "type": "ErrorCategory",
                      "value": "UnsupportedOperation"
                    }
                  },
                  {
                    "name": "message",
                    "value": {
                      "kind": "Text",
                      "value": "Non-integer exponents are not yet supported."
                    }
                  },
                  {
                    "name": "trigger",
                    "value": {
                      "kind": "Text",
                      "value": "(-1)^(1/2)"
                    }
                  }
                ]
              },
              {
                "kind": "Record",
                "type": "UnsupportedCapability",
                "fields": [
                  {
                    "name": "operation_class",
                    "value": {
                      "kind": "Text",
                      "value": "solve.unsupported-domain"
                    }
                  },
                  {
                    "name": "code",
                    "value": {
                      "kind": "Text",
                      "value": "InvalidOperation"
                    }
                  },
                  {
                    "name": "category",
                    "value": {
                      "kind": "Enum",
                      "type": "ErrorCategory",
                      "value": "DomainError"
                    }
                  },
                  {
                    "name": "message",
                    "value": {
                      "kind": "Text",
                      "value": "solve(): currently supports domains real and complex; got integer."
                    }
                  },
                  {
                    "name": "trigger",
                    "value": {
                      "kind": "Text",
                      "value": "x = symbol(\"x\"); solve(x^2 - 2 == 0, x, integer)"
                    }
                  }
                ]
              },
              {
                "kind": "Record",
                "type": "UnsupportedCapability",
                "fields": [
                  {
                    "name": "operation_class",
                    "value": {
                      "kind": "Text",
                      "value": "rootof.complex-algebraic"
                    }
                  },
                  {
                    "name": "code",
                    "value": {
                      "kind": "Text",
                      "value": "solve.unrepresented-roots"
                    }
                  },
                  {
                    "name": "category",
                    "value": {
                      "kind": "Enum",
                      "type": "ErrorCategory",
                      "value": "UnsupportedOperation"
                    }
                  },
                  {
                    "name": "message",
                    "value": {
                      "kind": "Text",
                      "value": "complex algebraic roots not supported (RootOf is real-only in v1)."
                    }
                  },
                  {
                    "name": "trigger",
                    "value": {
                      "kind": "Text",
                      "value": "x = symbol(\"x\"); solve_full(x^4 - x^2 - 1 == 0, x)"
                    }
                  }
                ]
              }
            ]
          }
        },
        {
          "name": "exactness",
          "value": {
            "kind": "Enum",
            "type": "CapabilitiesExactness",
            "value": "BestEffort"
          }
        }
      ]
    }
  },
  "output": [],
  "variables": [
    {
      "name": "_",
      "kind": "Record",
      "display": "CapabilitiesResult(supported_domains: [real, complex], unsupported_domains: [integer, rational], unsupported_operations: [UnsupportedCapability(operation_class: complex.sqrt-negative, code: ArithmeticError, category: DomainError, message: Square root is not defined for negative numbers., trigger: sqrt(-1)), UnsupportedCapability(operation_class: pow.non-integer-exponent, code: UnsupportedOperation, category: UnsupportedOperation, message: Non-integer exponents are not yet supported., trigger: (-1)^(1/2)), UnsupportedCapability(operation_class: solve.unsupported-domain, code: InvalidOperation, category: DomainError, message: solve(): currently supports domains real and complex; got integer., trigger: x = symbol(\"x\"); solve(x^2 - 2 == 0, x, integer)), UnsupportedCapability(operation_class: rootof.complex-algebraic, code: solve.unrepresented-roots, category: UnsupportedOperation, message: complex algebraic roots not supported (RootOf is real-only in v1)., trigger: x = symbol(\"x\"); solve_full(x^4 - x^2 - 1 == 0, x))], exactness: BestEffort)"
    }
  ],
  "functions": [],
  "elapsed": "<volatile>",
  "elapsedTime": "<volatile>",
  "timings": "<volatile>"
}
```

The corpus row was added to `GoldenEnvelopeTests.Corpus` (`{ "capabilities", 0 }`), because
`EveryFixtureScriptHasACorpusRow` asserts the on-disk `.ls` set equals the corpus set — a new
fixture without its row is a red suite.

### Structured record for `capabilities()` (readable form)

```text
display: CapabilitiesResult(supported_domains: [real, complex], unsupported_domains: [integer, rational],
  unsupported_operations: [UnsupportedCapability(operation_class: complex.sqrt-negative, code: ArithmeticError,
  category: DomainError, message: Square root is not defined for negative numbers., trigger: sqrt(-1)),
  UnsupportedCapability(operation_class: pow.non-integer-exponent, code: UnsupportedOperation,
  category: UnsupportedOperation, message: Non-integer exponents are not yet supported., trigger: (-1)^(1/2)),
  UnsupportedCapability(operation_class: solve.unsupported-domain, code: InvalidOperation, category: DomainError,
  message: solve(): currently supports domains real and complex; got integer.,
  trigger: x = symbol("x"); solve(x^2 - 2 == 0, x, integer)),
  UnsupportedCapability(operation_class: rootof.complex-algebraic, code: solve.unrepresented-roots,
  category: UnsupportedOperation, message: complex algebraic roots not supported (RootOf is real-only in v1).,
  trigger: x = symbol("x"); solve_full(x^4 - x^2 - 1 == 0, x))], exactness: BestEffort)
```

```text
> dotnet Lovelace.Run/bin/Release/net10.0/Lovelace.Run.dll --eval 'capabilities()' --json --omit-functions
{"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":true,"revision":79,
 "result":{"kind":"Record", … "structured":{"kind":"Record","type":"CapabilitiesResult","fields":[
   {"name":"supported_domains","value":{"kind":"Array","type":"Vector","shape":[2],
      "elements":[{"kind":"Domain","domain":"real"},{"kind":"Domain","domain":"complex"}]}},
   {"name":"unsupported_domains","value":{"kind":"Array","type":"Vector","shape":[2],
      "elements":[{"kind":"Domain","domain":"integer"},{"kind":"Domain","domain":"rational"}]}},
   {"name":"unsupported_operations","value":{"kind":"Array","type":"Vector","shape":[4],"elements":[
      {"kind":"Record","type":"UnsupportedCapability","fields":[
         {"name":"operation_class","value":{"kind":"Text","value":"complex.sqrt-negative"}},
         {"name":"code","value":{"kind":"Text","value":"ArithmeticError"}},
         {"name":"category","value":{"kind":"Enum","type":"ErrorCategory","value":"DomainError"}},
         {"name":"message","value":{"kind":"Text","value":"Square root is not defined for negative numbers."}},
         {"name":"trigger","value":{"kind":"Text","value":"sqrt(-1)"}}]}, … ]}},
   {"name":"exactness","value":{"kind":"Enum","type":"CapabilitiesExactness","value":"BestEffort"}}]}}}
```

(Abridged only in the `display`/`typed` lines, which repeat the printed form above; the complete,
unabridged envelope is the golden fixture pasted in full in the previous section.)

The plain-text form the CLI prints without `--json`:

```text
> dotnet Lovelace.Run/bin/Release/net10.0/Lovelace.Run.dll --eval 'capabilities()'
CapabilitiesResult(supported_domains: [real, complex], unsupported_domains: [integer, rational], unsupported_operations: [...], exactness: BestEffort)
```

---

## 6. Builtin count

* before: **104** (printed by the new test in Step A: `builtin count with capabilities(): 104`)
* after: **105**
* growth: **exactly one**

The increase is independently confirmed by the repository's existing pinned-count tripwire, which
failed on the first post-implementation suite run with exactly this delta:

```text
  Failed Lovelace.Symbolics.Tests.HyperbolicBuiltinsAndNegativeIntegerPowersTests.BuiltinCount_IncludesTheThreeHyperbolicFunctions [10 ms]
  Error Message:
   Assert.Equal() Failure: Values differ
Expected: 104
Actual:   105
```

The pinned number was then updated to 105 (with a comment naming round 22 as the cause) — that is
the tripwire working as designed, not a weakened test.

---

## 7. What is exact and what is best-effort

`exactness: BestEffort` (Enum `CapabilitiesExactness`) is the honest answer for the record as a
whole, and it splits as follows:

* **Exhaustive:** `supported_domains` + `unsupported_domains` together cover the **closed**
  `MathDomain` enum (Integer, Rational, Real, Complex) — there is no fifth domain to miss.
  Scope caveat, also true: this is the *solver's* domain option. `integer`/`rational` are rejected
  by `solve(…, domain)` but still accepted as **symbol/assumption** domains
  (`symbol("n", integer)`, `assume_integer(n)`), which is why the two arrays are about the solver.
* **Best-effort:** `unsupported_operations`. It enumerates the four classes this round verified
  against live envelopes. What could **not** be enumerated:
  1. the kernel can raise an unsupported/domain failure from many other paths (limit, integration,
     factoring, assumption solving, rewrite budgets, …) and there is no machine-readable registry to
     enumerate them from — only the four probes were verified end-to-end;
  2. the codes for thrown failures (1–3) are produced by the *host's* classification of the
     exception type, not by the kernel, so a future re-classification in `Runner.Classify` would
     silently change the real code — the honesty test is the only thing that catches that;
  3. `rootof.complex-algebraic` is not a thrown failure at all: it is a **non-fatal** diagnostic
     (`ok: true`) inside a solve record, so its "unsupported" status is a completeness statement,
     not an error. Candidates that are *not* claimed: budget exhaustion, unevaluated limits and
     integrals (recoverable/budget states rather than capability limits), and any operation that is
     simply not attempted.

---

## 8. Files changed (scope)

| file | change |
|---|---|
| `Lovelace.Symbolics/SymbolicsPlugin.cs` | `capabilities` builtin + descriptor; `CapabilitiesRecord()` and `UnsupportedCapability()` builders (both private static); `using Lovelace.Abstractions` was already present |
| `Lovelace.Symbolics.Tests/CapabilitiesBuiltinTests.cs` | new: 7 tests (shape, domains, entries, exactness, the envelope honesty test, registration/metadata) |
| `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.csproj` | + `ProjectReference` to `Lovelace.Run` so the honesty test compares against the real published envelope |
| `Lovelace.Symbolics.Tests/HyperbolicBuiltinsAndNegativeIntegerPowersTests.cs` | pinned builtin count 104 → 105 (the existing tripwire) |
| `Lovelace.Run.Tests/fixtures/capabilities.ls` | new fixture script |
| `Lovelace.Run.Tests/fixtures/capabilities.json` | new golden envelope |
| `Lovelace.Run.Tests/GoldenEnvelopeTests.cs` | one corpus row `{ "capabilities", 0 }` — required by `EveryFixtureScriptHasACorpusRow` for the new fixture (the only edit outside `fixtures/` in that project) |

No `git commit` was made. Nothing outside the scope above was touched; complex roots and rational
powers of negative bases remain unsupported.
