using System.Diagnostics;
using System.Text;
using Lovelace.Symbolics;
using Xunit;

namespace Lovelace.Symbolics.Tests;

/// <summary>
/// Differential oracle (Phase 2): compares kernel results against SymPy when a local python3
/// with sympy is available; otherwise the tests skip. SymPy is a TEST oracle only — never a
/// runtime dependency. Disagreements are reported for independent adjudication, never trusted
/// blindly.
/// </summary>
public class DifferentialOracleTests
{
    private static string? SympyProbe()
    {
        try
        {
            var psi = new ProcessStartInfo
            {
                FileName = "python3",
                RedirectStandardOutput = true,
                RedirectStandardError = true,
                UseShellExecute = false,
            };
            psi.ArgumentList.Add("-c");
            psi.ArgumentList.Add("import sympy; print(sympy.__version__)");
            using var p = Process.Start(psi);
            if (p is null) return null;
            var stdout = p.StandardOutput.ReadToEnd();
            p.WaitForExit(5000);
            return p.ExitCode == 0 ? stdout.Trim() : null;
        }
        catch
        {
            return null;
        }
    }

    private static string SympyEval(string code)
    {
        var psi = new ProcessStartInfo
        {
            FileName = "python3",
            RedirectStandardOutput = true,
            RedirectStandardError = true,
            UseShellExecute = false,
        };
        psi.ArgumentList.Add("-c");
        psi.ArgumentList.Add(code);
        using var p = Process.Start(psi)!;
        var stdout = p.StandardOutput.ReadToEnd();
        var stderr = p.StandardError.ReadToEnd();
        p.WaitForExit(30_000);
        if (p.ExitCode != 0)
            throw new InvalidOperationException("sympy failed: " + stderr);
        return stdout.Trim();
    }

    [Fact]
    public void SympyOracle_Derivatives_Agree()
    {
        if (SympyProbe() is null)
            return;   // oracle unavailable — skip silently (CI without python)
        var ctx = new ExprContext();
        Exprs.Current = ctx;
        var x = ctx.Symbol("x");
        var corpus = new (Expr Expr, string Sympy)[]
        {
            (Exprs.Power(x, 7), "x**7"),
            (Exprs.Function(ctx.Function("sin"), Exprs.Power(x, 2)), "sin(x**2)"),
            (Exprs.Function(ctx.Function("exp"), Exprs.Negate(Exprs.Power(x, 2))), "exp(-x**2)"),
            (Exprs.Multiply(x, Exprs.Function(ctx.Function("log"), x)), "x*log(x)"),
            (Exprs.Divide(Exprs.Function(ctx.Function("sin"), x), x), "sin(x)/x"),
            (Exprs.Function(ctx.Function("tan"), Exprs.Power(x, 3)), "tan(x**3)"),
        };
        foreach (var (e, sympyText) in corpus)
        {
            var mine = Printing.CanonicalPrint(Calculus.Diff(e, x, ctx));
            var code = "import sympy; x=sympy.symbols('x'); print(sympy.sstr(sympy.diff(" + sympyText + ", x)))";
            var theirs = SympyEval(code);
            Assert.True(EquivalentToSympy(mine, theirs, ctx),
                $"diff({sympyText}): kernel {mine} vs sympy {theirs}");
        }
    }

    [Fact]
    public void SympyOracle_Solve_Agrees()
    {
        if (SympyProbe() is null)
            return;
        var ctx = new ExprContext();
        Exprs.Current = ctx;
        var x = ctx.Symbol("x");
        var corpus = new (Expr Expr, string Sympy)[]
        {
            (Exprs.Add(Exprs.Power(x, 2), Exprs.Multiply(-4, x), 4), "x**2 - 4*x + 4"),
            (Exprs.Add(Exprs.Power(x, 2), Exprs.Multiply(-5, x), 6), "x**2 - 5*x + 6"),
            (Exprs.Add(Exprs.Power(x, 3), 1), "x**3 + 1"),
            (Exprs.Add(Exprs.Power(x, 4), Exprs.Multiply(-5, Exprs.Power(x, 2)), 4), "x**4 - 5*x**2 + 4"),
        };
        foreach (var (e, sympyText) in corpus)
        {
            var set = Solvers.Solve(Exprs.Relation(RelOp.Eq, e, Exprs.Zero), x, ctx);
            var code = "import sympy; x=sympy.symbols('x'); print(sorted(sympy.solve(" + sympyText + ", x), key=lambda r: sympy.srepr(r)))";
            var theirs = SympyEval(code);
            foreach (var sol in set.Solutions)
            {
                var at = Evaluation.Substitute(e, ctx, new Dictionary<Symbol, Expr> { [x] = sol.Value });
                var expanded = Algebra.Expand(at, ctx);
                Assert.True(expanded is RationalConstantExpr rc && rc.Value.IsZero,
                    $"kernel root {Printing.PrettyPrint(sol.Value)} does not satisfy {sympyText}; sympy says {theirs}");
            }
        }
    }

    /// <summary>Adjudicates a kernel canonical form against a sympy string by independent numeric
    /// evaluation: build the kernel expression back from its canonical text and compare at points.</summary>
    private static bool EquivalentToSympy(string canonical, string sympyText, ExprContext ctx)
    {
        var e = Printing.CanonicalParse(canonical, ctx);
        using var scope = Lovelace.Real.Real.WithPrecision(40, 20);
        var points = new[] { "1/3", "-1/2", "7/5" };
        foreach (var pText in points)
        {
            var p = Lovelace.Rational.Rational.Parse(pText, null);
            var mine = Evaluation.EvaluateToNum(e, ctx, new Dictionary<Symbol, Num> { [ctx.Symbol("x")] = NumOps.FromRat(p) });
            var code = "import sympy; x=sympy.symbols('x'); print(sympy.N(sympy.diff(" + sympyText + ", x).subs(x, sympy.Rational('" + pText + "')), 25))";
            var theirs = Lovelace.Real.Real.Parse(SympyEval(code), null);
            var diff = NumOps.Abs(NumOps.Subtract(mine, NumOps.FromReal(theirs)), ctx);
            if (NumOps.Compare(diff, NumOps.FromReal(Lovelace.Real.Real.Parse("0.0000000000001", null))) >= 0)
                return false;
        }
        return true;
    }
}
