# Observation A — Declared and emitted shape of the Lovelace symbolic rich result records

Observer: subagent obs-A (multi-agent harness). Repository: `C:\Users\ricar\dev\LovelaceSharp`, git HEAD 35e2609.
Method: source reading only. No build, no test, no publish, no execution of `out/aot/Lovelace.Run.exe` was
performed; all citations are file:line relative to the repository root. Findings are descriptions of the
observed shape only. `[INFERRED]` marks a statement derived by applying a quoted generic rule to a quoted call
site rather than a verbatim declaration/emission; everything else is a verbatim quote.

---

## Q1 — Human solution assignment for a solved system

| ID | Question | Source (file:line) | Observed declaration / emission (quote it) | Confidence |
|---|---|---|---|---|
| A1 | Q1 | `Lovelace.Symbolics/Solvers/Solve.cs:839` | `public sealed class SystemSolveResult` — the system-solve result is declared as a sealed **class**, not a record. | High |
| A2 | Q1 | `Lovelace.Symbolics/Solvers/Solve.cs:841` | `public List<SystemSolution> Solutions { get; } = new();` — the member is a list of a record, not a list of strings. | High |
| A3 | Q1 | `Lovelace.Symbolics/Solvers/Solve.cs:837` | `public sealed record SystemSolution(IReadOnlyDictionary<Symbol, Expr> Assignment, AssumptionSet Conditions);` — the assignment member is a dictionary, not a string. | High |
| A4 | Q1 | `Lovelace.Symbolics/Solvers/Solve.cs:69` | `public sealed record Binding(string Name, Expr Value);` — a `Binding` record is declared, but a repo-wide search for the token `Binding` in `*.cs` returns this declaration line only (no construction or use site). | High |
| A5 | Q1 | `Lovelace.Symbolics/SymbolicsPlugin.cs:335-337` | `var solutions = result.Solutions.Select(sol => (object)new RecordValue("SystemSolution",` / `new RecordField("assignment", vs.Select(v => (object)(v.Name + " = " + Printing.PrettyPrint(sol.Assignment[v]))).ToArray()),` / `new RecordField("conditions", ConditionExprs(sol.Conditions)))).ToArray();` — at the interpreter boundary the `assignment` field is an **array of strings** of the form `"x = <pretty>"`. | High |
| A6 | Q1 | `Lovelace.Symbolics/SymbolicsPlugin.cs:338-341` | `return new RecordValue("SystemSolveResult",` … `new RecordField("status", …),` `new RecordField("solutions", solutions),` `new RecordField("diagnostics", result.Note ?? ""));` — the builtin `solve_system_full` writes into the emitted record; it does not write into the C# `Solutions` member (the kernel `SystemSolvers.Solve` fills that at `Solve.cs:898/921/935`). | High |
| A7 | Q1 | `Lovelace.Symbolics/SymbolicsPlugin.cs:339` | `new RecordField("status", result.Solutions.Count > 0 ? "Solved" : (result.Note is null ? "NoSolutions" : "Unevaluated")),` | High |
| A8 | Q1 | `Lovelace.Symbolics/SymbolicsPlugin.cs:76-78` | Descriptor text: `"Structured system solve: a SystemSolveResult with status, completeness, per-solution variable bindings and conditions."`; the emitted record at lines 338-341 carries exactly three fields: `status`, `solutions`, `diagnostics`. | High (both quoted; no judgement) |
| A9 | Q1 | `Lovelace.Symbolics/SymbolicsPlugin.cs:323-324` | Plain `solve_system` returns text: `return string.Join("; ", result.Solutions.Select(sol => string.Join(", ", vs.Select(v => v.Name + " = " + Printing.PrettyPrint(sol.Assignment[v])))));` | High |
| A10 | Q1 | `Lovelace.Suite/RecordSchemas.cs:75-76` | `Schema("SystemSolveResult", ("status", "Text"), ("solutions", "Array"), ("diagnostics", "Text")),` and `Schema("SystemSolution", ("assignment", "Array"), ("conditions", "Array")),` | High |
| A11 | Q1 | `Lovelace.Suite/StructuredProjection.cs:68-73, 79-83, 61` | Records project as `new StructuredValueDto("Record", Type: record.TypeName, Fields: …)`; a vector projects as Kind `"Array"` with `Elements`; a string payload projects as `new StructuredValueDto("Text", Value: value.AsText())`. So the emitted `assignment` array surfaces as `kind: "Array"` whose elements are `kind: "Text"`, value = the `"x = <pretty>"` strings. | High for the mapping; Medium for the field-level composition `[INFERRED]` |
| A12 | Q1 | `Lovelace.Studio.Tests/StructuredPayloadTests.cs:47-58` | Corroborating assertion source: `Assert.Equal("Array", solutions.Kind);` and `Assert.Equal("Text", solution.Fields!.Single(f => f.Name == "exactness").Value.Kind);` (test source, not production). | Medium (corroboration) |

---

## Q2 — SolutionFamily parameter and parameter-domain members

| ID | Question | Source (file:line) | Observed declaration / emission (quote it) | Confidence |
|---|---|---|---|---|
| B1 | Q2 | `Lovelace.Symbolics/Solvers/Solve.cs:78` | `public sealed record SolutionFamily(Expr Template, Symbol Parameter, Expr Period, ParameterDomain Domain)` — parameter member is `Symbol Parameter`, domain member is `ParameterDomain Domain`. | High |
| B2 | Q2 | `Lovelace.Symbolics/Solvers/Solve.cs:72` | `public enum ParameterDomain { Integers, NonNegativeIntegers }` — the domain is an enum, not a string. | High |
| B3 | Q2 | `Lovelace.Symbolics/Solvers/Solve.cs:749` | Kernel parameter value is a fresh symbol: `var k = FreshParameter(ctx, c, u);`; `FreshParameter` starts from `string name = "k";` (`Solve.cs:816`) and appends an index on collision (`Solve.cs:818-819`). | High |
| B4 | Q2 | `Lovelace.Symbolics/Solvers/Solve.cs:763-764, 787, 794-795, 800-806` | All five construction sites pass the same symbol and `ParameterDomain.Integers`; e.g. line 787 `set.Families.Add(new SolutionFamily(template, k, pi, ParameterDomain.Integers));` and line 806 `k, twoPi, ParameterDomain.Integers));`. | High |
| B5 | Q2 | `Lovelace.Symbolics/Solvers/Solve.cs:72` | `NonNegativeIntegers` occurs on this line only in the whole repository; no producer selects it. | High |
| B6 | Q2 | `Lovelace.Symbolics/SymbolicsPlugin.cs:393-399` | Emission: `new RecordField("parameter", f.Parameter.Name),` and `new RecordField("parameter_domain", ParameterDomainOf(f.Domain)),` — the emitted `parameter` is the symbol **name** (string), not the symbol. | High |
| B7 | Q2 | `Lovelace.Symbolics/SymbolicsPlugin.cs:612` | `private static MathDomain ParameterDomainOf(ParameterDomain p) => MathDomain.Integer;` — the parameter `p` is not read in the body; the emitted domain is always `MathDomain.Integer`. | High |
| B8 | Q2 | `Lovelace.Suite/RecordSchemas.cs:72-74` | `Schema("SolutionFamily",` `("template", "Symbolic"), ("parameter", "Text"), ("period", "Symbolic"),` `("parameter_domain", "Domain"), ("conditions", "Array"), ("exactness", "Text")),` | High |
| B9 | Q2 | `Lovelace.Suite/StructuredProjection.cs:61-62` | `ValueKind.Text => new StructuredValueDto("Text", Value: value.AsText()),` and `ValueKind.Domain => new StructuredValueDto("Domain", Domain: value.AsDomain().ToString().ToLowerInvariant()),` — the emitted `parameter` surfaces as Kind `"Text"`, `parameter_domain` as Kind `"Domain"` with a lower-cased name. | High |

---

## Q3 — Diagnostics and completeness/status members, per record

| ID | Question | Source (file:line) | Observed declaration / emission (quote it) | Confidence |
|---|---|---|---|---|
| C1 | Q3 SolveResult | search `(record\|class\|struct)\s+SolveResult` over all `*.cs` → 0 hits; name appears at `Lovelace.Symbolics/SymbolicsPlugin.cs:401` and `Lovelace.Suite/RecordSchemas.cs:65` | **0 declarations.** There is no C# type named `SolveResult`; the name exists only as a `RecordValue` `TypeName` string produced at `Lovelace.Symbolics/SymbolicsPlugin.cs:401` (`return new RecordValue("SolveResult",`) and as a Suite schema name at `Lovelace.Suite/RecordSchemas.cs:65`. | High |
| C2 | Q3 SolveResult (emitted) | `Lovelace.Symbolics/SymbolicsPlugin.cs:401-413` | `new RecordField("status", status.ToString()),` (402); `new RecordField("complete", status == SolveStatus.Solved),` (405); `new RecordField("completeness", (status == SolveStatus.Solved ? Completeness.Complete : set.Complete).ToString()),` (406); `new RecordField("diagnostics", set.Note ?? ""));` (413). Emitted types: `status`/`completeness`/`diagnostics` are strings, `complete` is a bool. `status` is the local `SolveStatus` declared at line 383. | High |
| C3 | Q3 SystemSolveResult (declared + emitted) | `Lovelace.Symbolics/Solvers/Solve.cs:842, 854, 862, 866, 870`; `Lovelace.Symbolics/SymbolicsPlugin.cs:339, 341` | Declared: `public string? Note { get; }` (842); `public SolveStatus Status` (854) with `return Note is null ? SolveStatus.NoSolutions : SolveStatus.Unevaluated;` (862); `public Completeness Complete => Status switch` (866) with `_ => Completeness.Unknown,` (870). Emitted: `new RecordField("diagnostics", result.Note ?? ""));` (341); `new RecordField("status", … ? "Solved" : … "NoSolutions" … )` (339). No completeness/complete field is emitted. | High |
| C4 | Q3 TransformResult (declared) | `Lovelace.Symbolics/Simplify.cs:13-17, 23-25, 28, 31-32` | `public sealed record TransformResult(Expr Expression, AssumptionSet Conditions, IReadOnlyList<RewriteStep> Steps, bool BudgetExceeded = false)` (13-17). No member named `Diagnostics`. Status member is computed text: `public string Status => Conditions.IsUnsatisfiable ? "Unsatisfiable" : BudgetExceeded ? "BudgetExceeded" : "Satisfied";` (23-25). `public string? BudgetKind { get; init; }` (28); `public IReadOnlyList<Rewriting.RuleAttemptDiagnostic> Attempts { get; init; }` (31-32). | High |
| C5 | Q3 TransformResult (emitted) | `Lovelace.Symbolics/SymbolicsPlugin.cs:241-254` | `return new RecordValue("TransformResult",` `new RecordField("status", r.Status),` (242) … `new RecordField("budget_exceeded", r.BudgetExceeded),` (253), `new RecordField("budget_kind", r.BudgetKind ?? ""));` (254). No `diagnostics` field and no `attempts` field are emitted. | High |
| C6 | Q3 LimitResult (declared) | `Lovelace.Symbolics/Calculus/Limits.cs:11, 15, 32` | `public sealed record LimitResult(LimitStatus Status, Expr? Value = null, LimitResult? FromLeft = null, LimitResult? FromRight = null, string? FailureReason = null)` (10-15). No member named `Diagnostics`; the diagnostic carrier is `string? FailureReason`, assigned only at `public static LimitResult Uneval(string reason) => new(LimitStatus.Unevaluated, FailureReason: reason);` (32, callers 89/133/149). Status member is the `LimitStatus` enum (line 11). | High |
| C7 | Q3 LimitResult (emitted) | `Lovelace.Symbolics/SymbolicsPlugin.cs:297-307` | `return new RecordValue("LimitResult",` `new RecordField("status", r.Status.ToString()),` (298) … `new RecordField("diagnostics", r.FailureReason ?? ""));` (307). | High |
| C8 | Q3 IntegrationResult (declared) | `Lovelace.Symbolics/Calculus/Integrate.cs:11-15, 26-28, 62-66` | `public sealed record IntegrationResult(IntegrationKind Kind, Expr Expression, AssumptionSet Conditions, string? Note = null)` (11-15). No member named `Diagnostics`; carrier is `string? Note`. The token `Note` occurs in this file only at line 15; all instances are built via `Exact`/`Conditional`/`Unevaluated` (26-28, 62-66), none of which pass `Note`. Status member is the `IntegrationKind` enum (line 12). | High |
| C9 | Q3 IntegrationResult (emitted) | `Lovelace.Symbolics/SymbolicsPlugin.cs:210-220` | `return new RecordValue("IntegrationResult",` `new RecordField("status", r.Kind.ToString()),` (211) … `new RecordField("diagnostics", r.Note ?? ""));` (220). | High |
| C10 | Q3 OptimizationResult (declared) | `Lovelace.Symbolics/Optimization/Optimize.cs:16-22` | `public sealed record OptimizationResult(Expr Expression, int NodesBefore, int NodesAfter, int SharedSubtrees, int HornerRewrites, OptimizeOptions Options);` — no diagnostics member, no status/completeness member. | High |
| C11 | Q3 OptimizationResult (emitted) | `Lovelace.Symbolics/SymbolicsPlugin.cs:472-481` | Fields emitted: `original`, `optimized`, `estimated_cost_before`, `estimated_cost_after`, `shared_subtrees`, `horner_rewrites`, `transformations`, `target`, `policy` — no `diagnostics`, `status`, `complete`, or `completeness` field. | High |
| C12 | Q3 CompilationResult (declared) | `Lovelace.MathIR/Compilation.cs:9, 11` | `public sealed record CompilationResult(IrProgram Program, Symbol[] Parameters)` with body `public string IrText => Program.Serialize();` — no diagnostics member, no status/completeness member. (Lovelace.MathIR is outside the listed SCOPE but was read.) | High |
| C13 | Q3 CompilationResult (emitted) | `Lovelace.MathIR/Evaluator.cs:405-416` | Fields emitted: `ir`, `parameters`, `result_domain`, `result_type`, `precision_policy`, `optimization_policy`, `target`, `mathir_version`, `exact` — no `diagnostics`/`status`/`completeness` field. | High |
| C14 | Q3 Suite projection (mechanism) | `Lovelace.Suite/StructuredProjection.cs:48-73, 57-65, 79-83` | `ValueKind.Record => Record(value.AsRecord(), budget),` (50); `private static StructuredValueDto Record(RecordValue record, …) => new("Record", Type: record.TypeName, Fields: record.Fields.Select(f => new StructuredFieldDto(f.Name, ToStructured((Value)f.Value!, budget))).ToArray());` (68-73). Leaf kinds: string → `new StructuredValueDto("Text", Value: value.AsText())` (61); bool → `new StructuredValueDto("Boolean", Value: value.AsBoolean() ? "true" : "false")` (60); long → `new StructuredValueDto("Integer", Value: value.AsInteger().ToString(), Exact: true)` (58); arrays → `new StructuredValueDto("Array", Type: value.Kind.ToString(), Shape: …, Elements: …)` (79-83). There is no per-member special case for `diagnostics`/`completeness`: every field is projected by its wrapped Value kind. | High |
| C15 | Q3 Suite projection (where the diagnostics text comes from) | `Lovelace.Suite/PayloadMap.cs:26`; `Lovelace.Suite/Value.cs:138-145`; plugin sites above | `string text => new Value(text),` (PayloadMap.cs:26) is the only string→Value conversion, reached through `new Value(record)` which wraps each field `f => new RecordField(f.Name, PayloadMap.Wrap(f.Value))` (Value.cs:140-142). Therefore the projected text is exactly the C# string written by the plugin: `set.Note ?? ""` (SymbolicsPlugin.cs:413), `result.Note ?? ""` (341), `r.FailureReason ?? ""` (307), `r.Note ?? ""` (220); records with no such field carry none. | High for the mapping; the per-field composition is `[INFERRED]` from C2/C3/C5/C7/C9/C11/C13 |
| C16 | Q3 JSON key naming | `Lovelace.Run/Program.cs:30, 392-393`; `Lovelace.Studio/StudioJsonContext.cs:12`; `Lovelace.Studio.Tests/StructuredPayloadTests.cs:66, 70` | `//   * JSON keys are camelCase; structured record FIELD NAMES are snake_case.` (Program.cs:30); `[JsonSourceGenerationOptions(PropertyNamingPolicy = JsonKnownNamingPolicy.CamelCase, DefaultIgnoreCondition = JsonIgnoreCondition.WhenWritingNull)]` (392-393). Test asserts the wire spelling: `Assert.Equal("Record", wireStructured.GetProperty("kind").GetString());` (StructuredPayloadTests.cs:66) and `Assert.Equal("Array", wireSolutions.GetProperty("kind").GetString());` (70). So DTO property `Kind` serializes as `"kind"`, and each field carries `"name"` + `"value"` (`"value".kind`). | High |
| C17 | Q3 Suite RecordSchemas (declared expected kinds) | `Lovelace.Suite/RecordSchemas.cs:65-97` | `SolveResult`: `("complete","Boolean"), ("completeness","Text"), … ("diagnostics","Text")` (66-69). `SystemSolveResult`: `("status","Text"), ("solutions","Array"), ("diagnostics","Text")` (75). `TransformResult`: `("status","Text"), … ("budget_exceeded","Boolean"), ("budget_kind","Text")` — no diagnostics entry (77-79). `LimitResult`: `("status","Text"), ("exists","Boolean"), … ("diagnostics","Text")` (83-86). `IntegrationResult`: `("status","Text"), … ("verified","Boolean"), … ("diagnostics","Text")` (87-89). `OptimizationResult`: no diagnostics/status entry (90-93). `CompilationResult`: no diagnostics/status entry (94-97). | High |

---

## Q4 — `SolveStatus.NoSolutions` paired with which completeness value

| ID | Question | Source (file:line) | Observed declaration / emission (quote it) | Confidence |
|---|---|---|---|---|
| D1 | Q4 (kernel `SolutionSet`) | `Lovelace.Symbolics/Solvers/Solve.cs:96-101` | `public Completeness Complete => Status switch { SolveStatus.Solved => Completeness.Complete, SolveStatus.Partial => Completeness.Partial, _ => Completeness.Unknown, };` — `NoSolutions` falls to the `_` arm, i.e. `Completeness.Unknown`. | High |
| D2 | Q4 (kernel `SystemSolveResult`) | `Lovelace.Symbolics/Solvers/Solve.cs:866-871` | `public Completeness Complete => Status switch { SolveStatus.Solved => Completeness.Complete, SolveStatus.Partial => Completeness.Partial, _ => Completeness.Unknown, };` — `NoSolutions` (line 862) → `Completeness.Unknown`. | High |
| D3 | Q4 (the emitted `SolveResult` NoSolutions path) | `Lovelace.Symbolics/SymbolicsPlugin.cs:383-385` | `var status = set.Status;` / `if (status == SolveStatus.Solved && accepted.Count == 0 && set.Families.Count == 0)` / `status = SolveStatus.NoSolutions;` — the local `status` (not `set.Status`) is reassigned; `set.Status` remains `Solved`. | High |
| D4 | Q4 (completeness passed on that path) | `Lovelace.Symbolics/SymbolicsPlugin.cs:406` + `Solve.cs:98` | `new RecordField("completeness", (status == SolveStatus.Solved ? Completeness.Complete : set.Complete).ToString()),` — with `status` now `NoSolutions`, the expression evaluates `set.Complete`; because `set.Status` is still `Solved`, `Solve.cs:98` returns `Completeness.Complete`. Emitted pairing on this path: `status: "NoSolutions"` with `completeness: "Complete"`. | High for the quoted expressions; the evaluation result is `[INFERRED]` |
| D5 | Q4 (same path, `complete` flag) | `Lovelace.Symbolics/SymbolicsPlugin.cs:405` | `new RecordField("complete", status == SolveStatus.Solved),` — on the D3 path this emits `false` alongside `completeness: "Complete"`. | High for the quote; `[INFERRED]` for the value |
| D6 | Q4 (kernel-originated NoSolutions) | `Lovelace.Symbolics/Solvers/Solve.cs:238, 351, 417, 624, 626, 684, 776`; `SymbolicsPlugin.cs:383, 406` | `return new SolutionSet(SolveStatus.NoSolutions, domain, "the equation reduces to a nonzero constant.");` (351; the same construction appears at 238, 417, 624, 626, 684, 776). When such a set reaches `solve_full`, line 383 keeps `status = NoSolutions`, so line 406 evaluates `set.Complete` → `Completeness.Unknown` (`Solve.cs:100`). Emitted pairing: `status: "NoSolutions"` with `completeness: "Unknown"`. | High for the quotes; `[INFERRED]` for the composition |
| D7 | Q4 (SystemSolveResult emission) | `Lovelace.Symbolics/SymbolicsPlugin.cs:339` + `Solve.cs:862` | `new RecordField("status", result.Solutions.Count > 0 ? "Solved" : (result.Note is null ? "NoSolutions" : "Unevaluated")),` and `return Note is null ? SolveStatus.NoSolutions : SolveStatus.Unevaluated;` — the record emits no completeness/complete field at all (lines 338-341). | High |

---

## Could not determine

1. **No declared C# `SolveResult` type exists.** A repo-wide search for `(record|class|struct)\s+SolveResult` in `*.cs`
   returned no declaration (only `SystemSolveResult`). I therefore cannot report a declared type for a
   `SolveResult` diagnostics or completeness member; only the emitted `RecordValue` field expressions at
   `Lovelace.Symbolics/SymbolicsPlugin.cs:401-413` are observable.
2. **Observed runtime JSON values were not captured.** Per task constraints I ran no build, test, or published
   binary, so I cannot state which concrete inputs reach the `Solved → NoSolutions` reassignment
   (`SymbolicsPlugin.cs:385`), nor the actual string content of any `diagnostics` member at runtime. The
   projection mapping (kinds) is stated from `StructuredProjection.cs` only.
3. **Whether `TransformResult.Attempts` is ever populated or surfaced.** `Simplify.cs:70` allocates attempts only
   when `options.Diagnose` is true (`Simplify.Options(bool Trace = false, int MaxSteps = 400, bool Diagnose = false)`,
   `Simplify.cs:51`), and the `simplify_full` call site passes `new Simplify.Options(Trace: true)`
   (`SymbolicsPlugin.cs:240`); I did not find a builtin that surfaces `Attempts` as a record field.
4. **`IntegrationResult.Note` and `Method`/`VerificationMethod` producers.** No assignment site for `Note` was found
   in this repository (only the declaration at `Integrate.cs:15`); I cannot determine whether a producer exists
   outside the read scope.
5. **Anything outside the checkout.** Whether another repository, package, or generated artifact constructs a
   record named `SolveResult` or supplies `ParameterDomain.NonNegativeIntegers` could not be determined from
   this tree.
6. **Type of the emitted `completeness` for records other than `SolveResult`.** `SystemSolveResult`,
   `TransformResult`, `LimitResult`, `IntegrationResult`, `OptimizationResult`, and `CompilationResult` emit no
   field literally named `completeness`; their status-like fields are the ones listed in Q3.
