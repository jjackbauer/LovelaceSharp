# Cycle-4 commit plan — the 5-stage split

Maintainer decision (this session): the Cycle-3 tree lands as **five staged commits**, with the P0
wire revisions revertible on their own (OQ-007).

## Why the brief's stage order is not used verbatim

The brief suggested: 1 docs → 2 runner test project + goldens + slnx/CI → 3 P0 wire revisions →
4 fixes → 5 remaining items. **Measured evidence makes stages 2→3 in that order impossible to keep
green:**

| Evidence | Consequence |
|---|---|
| `Lovelace.Run.Tests/GoldenEnvelopeTests.cs:84` asserts `fields["status"]["kind"] == "Enum"` | The runner test project fails to pass until the wire revision is present |
| `Lovelace.Run.Tests/GoldenEnvelopeTests.cs:39` pins a `capabilities` fixture | It also needs the `capabilities()` item, which the brief puts in stage 5 |
| `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.csproj` adds a `Lovelace.Run` project reference for the capabilities honesty test | The capabilities tests need the new `Lovelace.Run.Runner` seam |
| `Lovelace.Symbolics/SymbolicsPlugin.cs` diff carries CANCEL + CAPS + LATEX + LIMIT-EXISTS + N2 + ARITY; `Interpreter.cs` carries wire + cancel; `DxStructuredResultsTests.cs` asserts both wire shape and the N2 message | No file-level partition separates these concerns cleanly |

So the series is **reordered so every commit builds and its own suites pass**, which is what the
approved option actually required ("verify each stage builds and its suites pass before committing
it"). The wire revision remains a single revertible commit.

**Assignment rule, applied mechanically:**

- An **implementation** file goes to the **earliest** stage that needs any part of it.
- A **test** file goes to the **latest** stage among the concerns its assertions touch, so it never
  runs before the code it asserts on exists.

**Honest limit of this split:** two files mix concerns at file granularity
(`Lovelace.Symbolics/SymbolicsPlugin.cs`, `Lovelace.Symbolics/Printing.cs`, plus `Interpreter.cs`).
Their commit messages name every concern they carry, so a reviewer is never misled about what a
commit contains. Hunk-level splitting was rejected: it would produce intermediate file states that
were never compiled as such, and it buys reviewability at the cost of buildability.

## The series

### Stage 1 — harness, cycle records and measurement artifacts (no product code)

`docs/goal-cycle-3/**`, `docs/goal-cycle-4/**`, `docs/symbolics/a-plus-cycle-3-alignment.md`,
`docs/symbolics/a-plus-cycle-3-report.md`, `docs/symbolics/a-plus-cycle-4-brief.md`,
`benchmarks/bdn-reports-longrun/**`, `benchmarks/raw-longrun.txt`,
`benchmarks/raw-longrun-cpu.txt`, `benchmarks/symbench-longrun.md`

### Stage 2 — P0 wire revisions (the revertible protocol commit)

`Lovelace.Abstractions/Diagnostic.cs` (new), `Lovelace.Abstractions/EnumValue.cs` (new),
`Lovelace.MathIR/Evaluator.cs`, `Lovelace.Suite/{PayloadMap,RecordSchemas,StructuralEquality,StructuredProjection,Value,ValueFormatter,Interpreter}.cs`,
`Lovelace.Studio/IncrementalRunner.cs`, `Lovelace.Studio.Tests/StructuredPayloadTests.cs`,
`Lovelace.Suite.Tests/{EnumPayloadSeamTests,RecordSchemaTests}.cs`,
`Lovelace.Symbolics/{Solvers/Solve.cs,Simplify.cs,SymbolicsPlugin.cs}`,
`Lovelace.Symbolics.Tests/{DiagnosticContractTests,EnumValueEmissionTests,SolveCompletenessMappingTests,SolveCompletenessPairingTests,DxSemanticClosureTests,RewriteProtocolTests}.cs`,
`docs/symbolics/dsh-protocol.md`

### Stage 3 — the correctness fixes (N13, N14, N15, N17, N18, N19, N2)

`Lovelace.Symbolics/Algebra/{Factor,Polynomial}.cs` (N13 sign, N14 hang),
`Lovelace.Symbolics/Printing.cs` + `Lovelace.Symbolics/README.md` (N15 round-trip),
`Lovelace.Integer/Integer.cs` (N2 wrong-parameter blame), `Lovelace.Natural/Natural.cs` (N17),
`Lovelace.Suite/NumericOps.cs` + `Lovelace.Symbolics/Constructors.cs` (N18 indeterminate forms),
`Lovelace.Suite.Tests/CancellationObservationTests.cs`,
`Lovelace.Symbolics.Tests/{FactorMetamorphicTests,PrettyParenthesesTests,IndeterminateFormTests,LimitExistenceTriStateTests,DxStructuredResultsTests}.cs`

### Stage 4 — remaining items (6–13, 17) and analyzer hygiene

`Lovelace.Abstractions/BuiltinDescriptor.cs` + `Lovelace.Suite/ModusHost.cs` + `Lovelace.Console.Tests/ReplOutputTests.cs` + `Lovelace.Suite.Tests/ModusArityTests.cs` (N1 arity),
`Lovelace.Symbolics/Assumptions.cs` + `Lovelace.Symbolics.Tests/AssumptionAddScalingTests.cs` (53×),
`Lovelace.Symbolics/Algebra/RationalFunctions.cs` (cancel_full),
`Lovelace.Symbolics.Tests/{CapabilitiesBuiltinTests,FalsificationGateTests,MetamorphicSetClosureTests,AbsSymbolicRoundTripTests,HyperbolicBuiltinsAndNegativeIntegerPowersTests,LatexPrinterTests,LatexSymbolNameEscapingTests,OracleCorpus,SympyOracle,FalsificationTests,DifferentialOracleTests}.cs`,
`Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.csproj`,
`Lovelace.Run/{Runner.cs,RunProtocol.cs,Program.cs}`,
`Lovelace.Real/Real.cs`, `Lovelace.Real.Tests/*`, `Lovelace.Representation/DigitStore.cs`, `Lovelace.Representation.Tests/DigitStoreSnapshotDigitsTests.cs`, `Lovelace.Suite.Tests/EmptyReductionTests.cs`

### Stage 5 — the published wire contract: runner test project, goldens, slnx/CI wiring

`Lovelace.Run.Tests/**`, `LovelaceSharp.slnx`, `.github/workflows/ci.yml`

## Verification

Each stage commit is checked in a **separate `git worktree`** at that commit (a clean checkout, so
the result cannot be contaminated by later stages): `dotnet build -c Release`, then the affected
test projects. If a stage is red, the offending file moves to the next stage and the two commits
are rewritten — the `cycle-3-safety-net` tag keeps the series redoable.
