using System.Text.Json;
using System.Text.Json.Serialization;
using BenchmarkDotNet.Attributes;
using Lovelace.Abstractions;
using Lovelace.Dsp;
using Lovelace.MathIR;
using Lovelace.Suite;
using Lovelace.Symbolics;
// Lovelace.Suite also declares an Expr (its AST node): every expression in this file is
// the symbolic kernel's Expr, so the alias keeps the reference unambiguous.
using Expr = Lovelace.Symbolics.Expr;
using Rl = Lovelace.Real.Real;
using Rat = Lovelace.Rational.Rational;

namespace SymBench;

/// <summary>Construction and canonicalization of a large shared expression DAG.</summary>
[MemoryDiagnoser]
public class ConstructionBenchmarks
{
    private ExprContext _ctx = null!;
    private string _canonical = null!;

    [GlobalSetup]
    public void Setup()
    {
        _ctx = new ExprContext();
        Exprs.Current = _ctx;
        var x = _ctx.Symbol("x");
        // a large shared DAG with LINEAR growth: a 5000-term sum of x^i + i (≈15k nodes);
        // nested doubling would grow exponentially and never terminate
        var terms = new List<Expr>(5000);
        for (int i = 0; i < 5000; i++)
            terms.Add(Exprs.Add(Exprs.Power(x, i % 64), i));
        _canonical = Printing.CanonicalPrint(Exprs.Add(terms));
    }

    [Benchmark]
    public Expr CanonicalParse_SharedDAG() => Printing.CanonicalParse(_canonical, _ctx);
}

/// <summary>High-order derivatives and vector calculus.</summary>
[MemoryDiagnoser]
public class CalculusBenchmarks
{
    private ExprContext _ctx = null!;
    private Expr _f = null!;
    private Expr[] _system = null!;
    private Symbol[] _vars = null!;

    [GlobalSetup]
    public void Setup()
    {
        _ctx = new ExprContext();
        Exprs.Current = _ctx;
        var x = _ctx.Symbol("x");
        _f = Exprs.Multiply(Exprs.Function(_ctx.Function("sin"), x), Exprs.Function(_ctx.Function("exp"), Exprs.Power(x, 2)));

        var a = _ctx.Symbol("a");
        var b = _ctx.Symbol("b");
        var c = _ctx.Symbol("c");
        var d = _ctx.Symbol("d");
        _vars = new[] { a, b, c, d };
        _system = new Expr[]
        {
            Exprs.Add(Exprs.Power(a, 3), Exprs.Multiply(b, c), d),
            Exprs.Add(Exprs.Multiply(a, b), Exprs.Power(c, 2), 1),
            Exprs.Add(Exprs.Multiply(b, d), Exprs.Power(a, 2)),
            Exprs.Add(Exprs.Power(d, 3), Exprs.Multiply(a, c)),
        };
    }

    [Benchmark]
    public Expr Diff_TenthOrder() => Calculus.DiffN(_f, _vars[0], 10, _ctx);

    [Benchmark]
    public SymbolicMatrix Jacobian_4x4() => SymbolicMatrix.Jacobian(_system, _vars, _ctx);

    [Benchmark]
    public SymbolicMatrix Hessian_4Var() => SymbolicMatrix.Hessian(Exprs.Add(Exprs.Power(_vars[0], 3), Exprs.Multiply(_vars[1], _vars[2])), _vars, _ctx);
}

/// <summary>Polynomial arithmetic and a Gröbner basis.</summary>
[MemoryDiagnoser]
public class PolynomialBenchmarks
{
    private ExprContext _ctx = null!;
    private Polynomial _p = null!;
    private Polynomial _q = null!;
    private List<Polynomial> _groebnerInput = null!;

    [GlobalSetup]
    public void Setup()
    {
        _ctx = new ExprContext();
        Exprs.Current = _ctx;
        var x = _ctx.Symbol("x");
        var y = _ctx.Symbol("y");
        // sparse bivariate degree-20 polynomials
        Expr Make(int seed)
        {
            Expr acc = Exprs.Zero;
            for (int i = 0; i <= 20; i++)
            {
                acc = Exprs.Add(acc,
                    Exprs.Multiply((seed * 7 + i * 13) % 11 + 1, Exprs.Power(x, i), Exprs.Power(y, 20 - i)));
            }
            return acc;
        }
        _p = Polynomial.FromExpr(Make(1), _ctx, new[] { x, y });
        _q = Polynomial.FromExpr(Make(2), _ctx, new[] { x, y });

        // cyclic-3 system: x + y + z, xy + yz + zx, xyz - 1
        var z = _ctx.Symbol("z");
        _groebnerInput = new List<Polynomial>
        {
            Polynomial.FromExpr(Exprs.Add(x, y, z), _ctx, new[] { x, y, z }),
            Polynomial.FromExpr(Exprs.Add(Exprs.Multiply(x, y), Exprs.Multiply(y, z), Exprs.Multiply(z, x)), _ctx, new[] { x, y, z }),
            Polynomial.FromExpr(Exprs.Subtract(Exprs.Multiply(x, y, z), 1), _ctx, new[] { x, y, z }),
        };
    }

    [Benchmark]
    public Polynomial Multiply_SparseBivariateDegree20() => Polynomial.Multiply(_p, _q);

    [Benchmark]
    public List<Polynomial> Groebner_Cyclic3_GrLex() => Groebner.Basis(_groebnerInput, MonomialOrder.GrLex);
}

/// <summary>Compile-and-evaluate pipeline: kernel, batch, and tree-vs-MathIR.</summary>
[MemoryDiagnoser]
public class CompilationBenchmarks
{
    private ExprContext _ctx = null!;
    private Expr _expr = null!;
    private CompiledKernel _kernel = null!;
    private Dictionary<string, Num[]> _batchColumns = null!;
    private Symbol _x = default;
    private Symbol _y = default;

    [GlobalSetup]
    public void Setup()
    {
        _ctx = new ExprContext();
        Exprs.Current = _ctx;
        _x = _ctx.Symbol("x");
        _y = _ctx.Symbol("y");
        // a Hornered-ish degree-10 polynomial in x times exp(-x) style transcendental mix
        _expr = Exprs.Add(
            Exprs.Multiply(Exprs.Function(_ctx.Function("exp"), Exprs.Negate(Exprs.Power(_x, 2))), Exprs.Function(_ctx.Function("sin"), Exprs.Multiply(_y, _x))),
            Exprs.Divide(Exprs.Power(_x, 10), Exprs.Add(Exprs.One, Exprs.Power(_x, 2))));
        _kernel = Compilation.CompileKernel(_expr, new[] { _x, _y }, _ctx);
        var xs = new Num[1000];
        var ys = new Num[1000];
        for (int i = 0; i < 1000; i++)
        {
            xs[i] = NumOps.FromRat(Rat.From(i + 1, 1000));
            ys[i] = NumOps.FromRat(Rat.From(2 * (i + 1), 1000));
        }
        _batchColumns = new Dictionary<string, Num[]> { [_x.Name] = xs, [_y.Name] = ys };
    }

    [Benchmark]
    public Num Kernel_EvaluateScalar()
    {
        using var scope = Rl.WithPrecision(50, 20);
        return _kernel.Evaluate(NumOps.FromRat(Rat.From(1, 2)), NumOps.FromRat(Rat.From(3, 4)));
    }

    [Benchmark]
    public Num[] Kernel_EvaluateBatch_1000Lanes()
    {
        using var scope = Rl.WithPrecision(50, 20);
        return _kernel.EvaluateBatch(_batchColumns);
    }

    [Benchmark]
    public Num TreeEvaluator_Direct()
    {
        using var scope = Rl.WithPrecision(50, 20);
        return Evaluation.EvaluateToNum(_expr, _ctx, new Dictionary<Symbol, Num>
        {
            [_x] = NumOps.FromRat(Rat.From(1, 2)),
            [_y] = NumOps.FromRat(Rat.From(3, 4)),
        });
    }
}

/// <summary>Arbitrary-precision evaluation at 64/256/1024 digits (never reduced).</summary>
[MemoryDiagnoser]
public class PrecisionBenchmarks
{
    private ExprContext _ctx = null!;
    private Expr _expr = null!;
    private Symbol _x = default;

    [GlobalSetup]
    public void Setup()
    {
        _ctx = new ExprContext();
        Exprs.Current = _ctx;
        _x = _ctx.Symbol("x");
        _expr = Exprs.Function(_ctx.Function("exp"), Exprs.Function(_ctx.Function("sin"), Exprs.Power(_x, 2)));
    }

    [Benchmark]
    public Num Eval64()
    {
        using var scope = Rl.WithPrecision(64, 20);
        return Evaluation.EvaluateToNum(_expr, _ctx, new Dictionary<Symbol, Num> { [_x] = NumOps.FromRat(Rat.From(1, 3)) });
    }

    [Benchmark]
    public Num Eval256()
    {
        using var scope = Rl.WithPrecision(256, 20);
        return Evaluation.EvaluateToNum(_expr, _ctx, new Dictionary<Symbol, Num> { [_x] = NumOps.FromRat(Rat.From(1, 3)) });
    }

    [Benchmark]
    public Num Eval1024()
    {
        using var scope = Rl.WithPrecision(1024, 20);
        return Evaluation.EvaluateToNum(_expr, _ctx, new Dictionary<Symbol, Num> { [_x] = NumOps.FromRat(Rat.From(1, 3)) });
    }
}

// ---------------------------------------------------------------------------
// Rows added by the A+ Cycle-2 Phase A deliverable (alignment plan item 18).
//
// The four classes above are the frozen baseline set and are deliberately left
// unchanged so benchmarks/symbench-baseline.md stays a like-for-like comparison
// point. The classes below cover the missing rows: expression construction and
// canonicalization, the safe-vs-full simplify split with and without the
// provenance trace, the diff/factor/solve entry points, both printers, the
// structured-record surface (construction, field lookup, member access, JSON
// projection), MathIR scalar/batch evaluation, and help-catalog generation.
// ---------------------------------------------------------------------------

/// <summary>The workloads shared by more than one new row (defined once so the rows
/// cannot drift apart).</summary>
internal static class BenchWorkloads
{
    /// <summary>A product/quotient transcendental composition (~40 nodes) — the diff and
    /// printer rows both need an expression with real structure to walk.</summary>
    public static Expr TranscendentalComposition(ExprContext ctx)
    {
        var x = ctx.Symbol("x");
        var sin = ctx.Function("sin");
        var exp = ctx.Function("exp");
        var log = ctx.Function("log");
        return Exprs.Divide(
            Exprs.Multiply(
                Exprs.Function(sin, Exprs.Multiply(x, Exprs.Function(exp, x))),
                Exprs.Function(log, Exprs.Add(Exprs.Power(x, 2), 1))),
            Exprs.Power(Exprs.Add(Exprs.One, Exprs.Power(x, 2)), 3));
    }

    /// <summary>A mixed algebraic/rational/transcendental target for the simplify rows:
    /// like-term folding is universal (safe), while the rational cancellation, sqrt(x^2)
    /// and log(exp(x)) rewrites are conditional (full only).</summary>
    public static Expr MixedSimplifyTarget(ExprContext ctx)
    {
        var x = ctx.Symbol("x");
        var sin = ctx.Function("sin");
        var cos = ctx.Function("cos");
        var exp = ctx.Function("exp");
        var log = ctx.Function("log");
        var sqrt = ctx.Function("sqrt");
        return Exprs.Add(
            Exprs.Divide(Exprs.Subtract(Exprs.Power(x, 2), 1), Exprs.Subtract(x, 1)),
            Exprs.Function(sqrt, Exprs.Power(x, 2)),
            Exprs.Function(log, Exprs.Function(exp, x)),
            Exprs.Power(Exprs.Function(sin, x), 2),
            Exprs.Power(Exprs.Function(cos, x), 2),
            Exprs.Multiply(3, x),
            Exprs.Negate(x),
            Exprs.Multiply(1, x));
    }
}

/// <summary>Expression construction and canonicalization over the language-facing API.</summary>
[MemoryDiagnoser]
public class ConstructionCanonicalizationBenchmarks
{
    private ExprContext _ctx = null!;
    private List<Expr> _terms = null!;

    [GlobalSetup]
    public void Setup()
    {
        _ctx = new ExprContext();
        Exprs.Current = _ctx;
        var x = _ctx.Symbol("x");
        // exactly the shared DAG ConstructionBenchmarks parses back from canonical text, so
        // the two rows are the two halves of one workload (build/canonicalize vs parse)
        _terms = new List<Expr>(5000);
        for (int i = 0; i < 5000; i++)
            _terms.Add(Exprs.Add(Exprs.Power(x, i % 64), i));
    }

    [Benchmark]
    public Expr Construct_SharedDAG() => Exprs.Add(_terms);

    [Benchmark]
    public string ConstructAndCanonicalize_SharedDAG() => Printing.CanonicalPrint(Exprs.Add(_terms));
}

/// <summary>simplify(): the safe entry point against the full transform, with the provenance
/// trace off and on (the trace is collected only when requested).</summary>
[MemoryDiagnoser]
public class SimplificationBenchmarks
{
    private ExprContext _ctx = null!;
    private Expr _target = null!;

    [GlobalSetup]
    public void Setup()
    {
        _ctx = new ExprContext();
        Exprs.Current = _ctx;
        _target = BenchWorkloads.MixedSimplifyTarget(_ctx);
    }

    [Benchmark]
    public Expr Simplify_Safe() => Simplify.SimplifyExpr(_target, _ctx);

    [Benchmark]
    public TransformResult Simplify_Full_TraceOff() =>
        Simplify.Transform(_target, _ctx, new Simplify.Options(Trace: false));

    [Benchmark]
    public TransformResult Simplify_Full_TraceOn() =>
        Simplify.Transform(_target, _ctx, new Simplify.Options(Trace: true));
}

/// <summary>The per-operation entry points: diff, factor, and solve.</summary>
[MemoryDiagnoser]
public class SolverBenchmarks
{
    private ExprContext _ctx = null!;
    private Expr _f = null!;
    private Expr _poly = null!;
    private Expr _equation = null!;
    private Symbol _x = default;

    [GlobalSetup]
    public void Setup()
    {
        _ctx = new ExprContext();
        Exprs.Current = _ctx;
        _x = _ctx.Symbol("x");
        _f = BenchWorkloads.TranscendentalComposition(_ctx);

        // x^6 - 14x^4 + 49x^2 - 36 = (x-1)(x+1)(x-2)(x+2)(x-3)(x+3): six rational roots
        _poly = Exprs.Add(
            Exprs.Power(_x, 6),
            Exprs.Multiply(-14, Exprs.Power(_x, 4)),
            Exprs.Multiply(49, Exprs.Power(_x, 2)),
            -36);

        // x^4 - 3x^2 + 2 == 0: a complete complex solution set (four roots)
        _equation = Exprs.Relation(
            RelOp.Eq,
            Exprs.Add(Exprs.Power(_x, 4), Exprs.Multiply(-3, Exprs.Power(_x, 2)), 2),
            Exprs.Zero);
    }

    [Benchmark]
    public Expr Diff_TranscendentalComposition() => Calculus.Diff(_f, _x, _ctx);

    [Benchmark]
    public Expr Factor_SexticRationalRoots() => Factoring.Factor(_poly, _ctx);

    [Benchmark]
    public SolutionSet Solve_QuarticAlgebraic() => Solvers.Solve(_equation, _x, _ctx, SolveDomain.Complex);
}

/// <summary>Both printer surfaces over the same nontrivial expression.</summary>
[MemoryDiagnoser]
public class PrintingBenchmarks
{
    private Expr _expr = null!;

    [GlobalSetup]
    public void Setup()
    {
        var ctx = new ExprContext();
        Exprs.Current = ctx;
        _expr = BenchWorkloads.TranscendentalComposition(ctx);
    }

    [Benchmark]
    public string PrettyPrint_TranscendentalComposition() => Printing.PrettyPrint(_expr);

    [Benchmark]
    public string CanonicalPrint_TranscendentalComposition() => Printing.CanonicalPrint(_expr);
}

/// <summary>Structured records: construction, field lookup by name, interpreter member access,
/// and JSON projection of a SolveResult-shaped record.</summary>
[MemoryDiagnoser]
public class StructuredResultBenchmarks
{
    private ExprContext _ctx = null!;
    private SuiteEngine _engine = null!;
    private Symbol _x = default;
    private RecordValue _solveResult = null!;
    private Value _structured = null!;
    private Value _structuredLarge = null!;

    [GlobalSetup]
    public void Setup()
    {
        _ctx = new ExprContext();
        Exprs.Current = _ctx;
        _x = _ctx.Symbol("x");
        _solveResult = BuildSolveResult(_x);
        // the ValueKind.Record mapping of that record, built once so the JSON row measures
        // projection + serialization rather than re-mapping the payload tree
        _structured = new Value(_solveResult);
        // 32x the solutions, to check serialization growth against the 2-solution row
        _structuredLarge = new Value(BuildSolveResult(_x, 64));

        // member access only exists through the interpreter, so the row needs a live engine
        _engine = BenchEngine.New();
        // statements are ';'-separated in this language: a newline is not a separator, so the
        // setup script must not rely on one (this row failed to run at all until this was fixed)
        _engine.EvaluateAsync("x = symbol(\"x\"); r = solve_full(x^2 - 4 == 0, x)").GetAwaiter().GetResult();
    }

    /// <summary>The record SymbolicsPlugin returns for solve_full(x^2 - 4 == 0, x): field
    /// names, order and payload shapes mirror the SolveResult contract.</summary>
    /// <summary>The shape SymbolicsPlugin returns for a SolveResult. <paramref name="solutionCount"/>
    /// is variable so serialization can be measured at two sizes and its growth checked for
    /// linearity in result size (alignment-plan §4 item 18).</summary>
    private static RecordValue BuildSolveResult(Symbol x, int solutionCount = 2)
    {
        var solutions = new object[solutionCount];
        for (int i = 0; i < solutionCount; i++)
        {
            solutions[i] = new RecordValue("Solution",
                new RecordField("value", Exprs.Integer(i + 1)),
                new RecordField("conditions", Array.Empty<object?>()),
                new RecordField("multiplicity", 1L),
                new RecordField("exactness", "Exact"));
        }
        return new RecordValue("SolveResult",
            new RecordField("status", "Solved"),
            new RecordField("variable", x.Name),
            new RecordField("domain", MathDomain.Complex),
            new RecordField("complete", true),
            new RecordField("completeness", "Complete"),
            new RecordField("solutions", solutions),
            new RecordField("families", Array.Empty<object?>()),
            new RecordField("common_conditions", Array.Empty<object?>()),
            new RecordField("represented_count", (long)solutionCount),
            new RecordField("unrepresented_count", 0L),
            new RecordField("unrepresented_reason", ""));
    }

    [Benchmark]
    public Value Record_Construct_SolveResult() => new Value(BuildSolveResult(_x));


    [Benchmark]
    public bool Record_FieldLookup_ByName() => _solveResult.TryGetField("solutions", out _);

    /// <summary>Full interpreter member-access path (parse + evaluate + EvaluateMemberAsync).</summary>
    [Benchmark]
    public Value Record_MemberAccess_Solutions() =>
        _engine.EvaluateAsync("r.solutions").GetAwaiter().GetResult();

    /// <summary>Structured JSON projection + serialization of the SolveResult-shaped record,
    /// through a LOCAL re-implementation (kept so the numbers stay comparable with the recorded
    /// baseline). New measurements should use the shipped projection — see
    /// <see cref="Json_SerializeSolveResult_ShippedProjection"/>.</summary>
    [Benchmark]
    public string Json_SerializeSolveResult() =>
        JsonSerializer.Serialize(Project(_structured), SymBenchJsonContext.Default.StructuredValueDto);

    /// <summary>The SHIPPED projection: <c>Lovelace.Suite.StructuredProjection.ToStructured</c>,
    /// the one implementation every host calls. The local copy below exists only for baseline
    /// comparability and would otherwise measure code nobody runs.</summary>
    [Benchmark]
    public string Json_SerializeSolveResult_ShippedProjection() =>
        JsonSerializer.Serialize(
            Lovelace.Suite.StructuredProjection.ToStructured(_structured),
            ShippedJsonContext.Default.StructuredValueDto);

    /// <summary>The same shipped projection at 32x the solutions: comparing this with the row above
    /// shows whether serialization is linear in result size or super-linear.</summary>
    [Benchmark]
    public string Json_SerializeSolveResult_ShippedProjection_64() =>
        JsonSerializer.Serialize(
            Lovelace.Suite.StructuredProjection.ToStructured(_structuredLarge),
            ShippedJsonContext.Default.StructuredValueDto);

    // The local copy below predates the Cycle-2 move of the projection into Lovelace.Suite (it was
    // written when the helper was internal to the Run executable, which is no longer true). It is
    // retained for baseline comparability only; the shipped-projection row above is the one that
    // measures what hosts actually execute.
    private static StructuredValueDto Project(Value value) => value.Kind switch
    {
        ValueKind.Record => new StructuredValueDto("Record", Type: value.AsRecord().TypeName,
            Fields: value.AsRecord().Fields
                .Select(f => new StructuredFieldDto(f.Name, Project((Value)f.Value!)))
                .ToArray()),
        ValueKind.Vector or ValueKind.Array => new StructuredValueDto("Array", Type: value.Kind.ToString(),
            Shape: value.AsArrayValue().Shape.ToArray(),
            Elements: value.AsVector().Select(Project).ToArray()),
        ValueKind.Symbolic => new StructuredValueDto("Symbolic",
            Pretty: Printing.PrettyPrint(value.AsSymbolic()),
            Canonical: Printing.CanonicalPrint(value.AsSymbolic()),
            Domain: Domains.DomainOf(value.AsSymbolic(), Exprs.Current).ToString().ToLowerInvariant(),
            Exact: value.AsSymbolic().IsExact,
            NodeCount: value.AsSymbolic().NodeCount,
            FreeSymbols: Printing.FreeSymbolNames(value.AsSymbolic()).ToArray()),
        ValueKind.Boolean => new StructuredValueDto("Boolean", Value: value.AsBoolean() ? "true" : "false"),
        ValueKind.Text => new StructuredValueDto("Text", Value: value.AsText()),
        ValueKind.Domain => new StructuredValueDto("Domain", Domain: value.AsDomain().ToString().ToLowerInvariant()),
        ValueKind.Natural => new StructuredValueDto("Natural", Value: value.AsNatural().ToString(), Exact: true),
        ValueKind.Integer => new StructuredValueDto("Integer", Value: value.AsInteger().ToString(), Exact: true),
        ValueKind.Real => new StructuredValueDto("Real", Value: value.AsReal().ToString(), Exact: true),
        _ => new StructuredValueDto("Null"),
    };
}

internal sealed record StructuredFieldDto(string Name, StructuredValueDto Value);

internal sealed record StructuredValueDto(
    string Kind,
    string? Type = null,
    string? Value = null,
    string? Pretty = null,
    string? Canonical = null,
    string? Domain = null,
    bool? Exact = null,
    int? NodeCount = null,
    string[]? FreeSymbols = null,
    long[]? Shape = null,
    StructuredValueDto[]? Elements = null,
    StructuredFieldDto[]? Fields = null);

[JsonSourceGenerationOptions(PropertyNamingPolicy = JsonKnownNamingPolicy.CamelCase,
    DefaultIgnoreCondition = JsonIgnoreCondition.WhenWritingNull)]
[JsonSerializable(typeof(StructuredValueDto))]
[JsonSerializable(typeof(StructuredFieldDto))]
internal sealed partial class SymBenchJsonContext : JsonSerializerContext
{
}

/// <summary>Serialization context for the SHIPPED projection
/// (<see cref="Lovelace.Suite.StructuredProjection"/>): the DTOs live in Lovelace.Suite, so this
/// context targets those types rather than the local baseline copies.</summary>
[JsonSourceGenerationOptions(PropertyNamingPolicy = JsonKnownNamingPolicy.CamelCase,
    DefaultIgnoreCondition = JsonIgnoreCondition.WhenWritingNull)]
[JsonSerializable(typeof(Lovelace.Suite.StructuredValueDto))]
[JsonSerializable(typeof(Lovelace.Suite.StructuredFieldDto))]
internal sealed partial class ShippedJsonContext : JsonSerializerContext
{
}

/// <summary>MathIR evaluation through the IR interpreter itself (not the compiled kernel):
/// one scalar pass and one 1000-lane vectorized pass over the same program.</summary>
[MemoryDiagnoser]
public class MathIrEvaluationBenchmarks
{
    private ExprContext _ctx = null!;
    private IrProgram _program = null!;
    private Dictionary<Symbol, Num> _scalarBindings = null!;
    private Dictionary<string, Num[]> _batchColumns = null!;
    private Symbol _x = default;
    private Symbol _y = default;

    [GlobalSetup]
    public void Setup()
    {
        _ctx = new ExprContext();
        Exprs.Current = _ctx;
        _x = _ctx.Symbol("x");
        _y = _ctx.Symbol("y");
        var expr = Exprs.Add(
            Exprs.Multiply(Exprs.Function(_ctx.Function("exp"), Exprs.Negate(Exprs.Power(_x, 2))), Exprs.Function(_ctx.Function("sin"), Exprs.Multiply(_y, _x))),
            Exprs.Divide(Exprs.Power(_x, 10), Exprs.Add(Exprs.One, Exprs.Power(_x, 2))));
        _program = Compilation.Compile(expr, _ctx, _x, _y).Program;

        _scalarBindings = new Dictionary<Symbol, Num>
        {
            [_x] = NumOps.FromRat(Rat.From(1, 2)),
            [_y] = NumOps.FromRat(Rat.From(3, 4)),
        };
        var xs = new Num[1000];
        var ys = new Num[1000];
        for (int i = 0; i < 1000; i++)
        {
            xs[i] = NumOps.FromRat(Rat.From(i + 1, 1000));
            ys[i] = NumOps.FromRat(Rat.From(2 * (i + 1), 1000));
        }
        _batchColumns = new Dictionary<string, Num[]> { [_x.Name] = xs, [_y.Name] = ys };
    }

    [Benchmark]
    public Num MathIR_Evaluate_Scalar()
    {
        using var scope = Rl.WithPrecision(50, 20);
        return IrEvaluator.Evaluate(_program, _scalarBindings, _ctx);
    }

    [Benchmark]
    public Num[] MathIR_Evaluate_Batch_1000Lanes()
    {
        using var scope = Rl.WithPrecision(50, 20);
        return IrEvaluator.EvaluateBatch(_program, _batchColumns, _ctx);
    }
}

/// <summary>Help-catalog generation over the product plugin set (the single renderer used by
/// the REPL, Studio completions, and the DSH tool surface).</summary>
[MemoryDiagnoser]
public class HelpCatalogBenchmarks
{
    private SuiteEngine _engine = null!;

    [GlobalSetup]
    public void Setup() => _engine = BenchEngine.New();

    [Benchmark]
    public string Help_Overview() => _engine.Help.Overview();

    [Benchmark]
    public string Help_Funcs_AllCategories() => _engine.Help.Funcs(null);
}

/// <summary>The product host's plugin set, exactly as the Suite host tests load it.</summary>
internal static class BenchEngine
{
    public static SuiteEngine New()
    {
        var engine = new SuiteEngine();
        engine.LoadPlugin(new DspPlugin());
        var symbolics = new SymbolicsPlugin();
        engine.LoadPlugin(symbolics);
        engine.LoadPlugin(new MathIRPlugin(symbolics));
        return engine;
    }
}

