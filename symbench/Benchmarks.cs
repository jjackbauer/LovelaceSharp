using BenchmarkDotNet.Attributes;
using Lovelace.MathIR;
using Lovelace.Symbolics;
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
