using Lovelace.Symbolics;
using Rl = Lovelace.Real.Real;
using Xunit;

namespace Lovelace.Symbolics.Tests;

/// <summary>
/// Differential oracle: compares kernel results against SymPy. SymPy is a TEST oracle only —
/// never a runtime dependency of the kernel. A disagreement is reported with BOTH forms (the
/// kernel's canonical form and the SymPy expression), the sample point and both values, so it can
/// be adjudicated instead of trusted blindly.
///
/// <para>An oracle that did not run is never a pass. <see cref="RequiresSympyFactAttribute"/>
/// decides at construction time: without python3 + sympy the run reports SKIPPED with the reason,
/// and with <c>LOVELACE_REQUIRE_SYMPY=1</c> nothing is skipped — the body runs and FAILS. The CI
/// job <c>sympy-oracle</c> installs SymPy and sets that variable, so these comparisons execute
/// somewhere on every push.</para>
///
/// <para>The corpora live in <c>OracleCorpus</c>; the kernel half of every case (and the fact that
/// every case states its domain and its SymPy expression) is checked on any machine by
/// <c>OracleCorpusDeclarationTests</c>. Only the comparison against SymPy needs the oracle.</para>
/// </summary>
public class DifferentialOracleTests
{
    [RequiresSympyFact]
    public void SympyOracle_Derivatives_Agree()
    {
        SympySession sympy = SympyOracle.Require();
        ExprContext ctx = NewContext(out Symbol x);
        foreach (DerivativeCase c in OracleCorpus.Derivatives(ctx))
        {
            string label = $"diff({c.Sympy})";
            string mine = Printing.CanonicalPrint(Calculus.Diff(c.Expr, x, ctx));
            string? mismatch = OracleCompare.NumberAgrees(
                sympy, label, mine, $"sympy.diff({c.Sympy}, x)", ctx, new[] { "x" }, c.Points);
            Assert.True(mismatch is null,
                mismatch is null ? $"{label} agrees with sympy" : $"{mismatch} [domain: {c.Domain}]");
        }
    }

    /// <summary>The original solve corpus, strengthened: it used to consult SymPy only for its
    /// failure message. The distinct-root count is now compared with sympy.solve, and every kernel
    /// root still has to substitute back to exactly zero.</summary>
    [RequiresSympyFact]
    public void SympyOracle_Solve_Agrees()
    {
        SympySession sympy = SympyOracle.Require();
        ExprContext ctx = NewContext(out Symbol x);
        foreach (SolveCase c in OracleCorpus.Solve(ctx))
        {
            string label = $"solve({c.Sympy})";
            SolutionSet set = Solvers.Solve(Exprs.Relation(RelOp.Eq, c.Expr, Exprs.Zero), x, ctx);
            IReadOnlyList<string> theirs = sympy.EvalLines(
                SympyScript.Symbols(new[] { "x" }) + "\n" +
                $"roots = sympy.solve({c.Sympy}, x)\n" +
                "print(len(set(roots)))\n" +
                "print(sympy.sstr(sorted(roots, key=sympy.srepr)))");
            int theirCount = int.Parse(theirs[0]);
            int myCount = set.Solutions
                .Select(solution => Printing.CanonicalPrint(solution.Value))
                .Distinct(StringComparer.Ordinal)
                .Count();

            Assert.True(set.Status == SolveStatus.Solved,
                $"{label}: the kernel did not claim a complete solution set (status {set.Status}, note {set.Note ?? "none"}) [domain: {c.Domain}]");
            Assert.True(myCount == theirCount,
                $"{label}: kernel {myCount} distinct solution(s) [{Describe(set)}] vs sympy {theirCount} -> {theirs[1]} [domain: {c.Domain}]");
            foreach (Solution solution in set.Solutions)
            {
                Expr at = Evaluation.Substitute(c.Expr, ctx, new Dictionary<Symbol, Expr> { [x] = solution.Value });
                Expr expanded = Algebra.Expand(at, ctx);
                Assert.True(expanded is RationalConstantExpr rc && rc.Value.IsZero,
                    $"{label}: kernel root {Printing.PrettyPrint(solution.Value)} does not satisfy {c.Sympy}; sympy says {theirs[1]} [domain: {c.Domain}]");
            }
        }
    }

    [RequiresSympyFact]
    public void SympyOracle_Roots_Agree()
    {
        SympySession sympy = SympyOracle.Require();
        ExprContext ctx = NewContext(out Symbol x);
        using var precision = Rl.WithPrecision(80, 40);
        foreach (RootCase c in OracleCorpus.Roots(ctx))
        {
            string label = $"real_roots({c.Sympy})";
            SolutionSet set = Solvers.Solve(Exprs.Relation(RelOp.Eq, c.Expr, Exprs.Zero), x, ctx, SolveDomain.Real);
            IReadOnlyList<string> theirs = sympy.EvalLines(
                SympyScript.Symbols(new[] { "x" }) + "\n" +
                $"roots = sympy.real_roots({c.Sympy}, x)\n" +
                "distinct = sorted({sympy.N(r, 30) for r in roots})\n" +
                "print(len(distinct))\n" +
                "for value in distinct:\n" +
                "    print(sympy.sstr(value))");
            int theirCount = int.Parse(theirs[0]);
            string theirValues = string.Join(", ", theirs.Skip(1));

            var mine = new List<Num>();
            var mineText = new List<string>();
            foreach (Solution solution in set.Solutions)
            {
                Num value = OracleCompare.ToNum(solution.Value, ctx, new Dictionary<Symbol, Num>());
                bool alreadySeen = mine.Any(previous =>
                    NumOps.Compare(
                        NumOps.Abs(NumOps.Subtract(previous, value), ctx),
                        NumOps.FromReal(Rl.Parse(OracleCompare.TightTolerance, null))) < 0);
                if (alreadySeen)
                    continue;   // distinct-set reading on both sides: (x - 1)^3 is ONE real root
                mine.Add(value);
                mineText.Add($"{Printing.PrettyPrint(solution.Value)} = {OracleCompare.Show(value)}");
            }

            Assert.True(set.Status is SolveStatus.Solved or SolveStatus.NoSolutions,
                $"{label}: the kernel made no definite real-root claim (status {set.Status}, note {set.Note ?? "none"}) [domain: {c.Domain}]");
            Assert.True(mine.Count == theirCount,
                $"{label}: kernel {mine.Count} distinct real root(s) [{string.Join("; ", mineText)}] vs sympy {theirCount} [{theirValues}] [domain: {c.Domain}]");
            for (int i = 0; i < mine.Count; i++)
            {
                if (!Rl.TryParse(theirs[i + 1], null, out Rl? theirValue))
                    Assert.Fail($"{label}: sympy returned '{theirs[i + 1]}' for root {i}, which is not a real number [domain: {c.Domain}]");
                Num difference = NumOps.Abs(NumOps.Subtract(mine[i], NumOps.FromReal(theirValue)), ctx);
                Assert.True(
                    NumOps.Compare(difference, NumOps.FromReal(Rl.Parse(OracleCompare.TightTolerance, null))) < 0,
                    $"{label}: kernel root {mineText[i]} vs sympy root {theirs[i + 1]} (index {i}) [domain: {c.Domain}]");
            }

            foreach (Solution solution in set.Solutions.Where(s => s.Value is not RootOfExpr))
            {
                Expr expanded = Algebra.Expand(
                    Evaluation.Substitute(c.Expr, ctx, new Dictionary<Symbol, Expr> { [x] = solution.Value }), ctx);
                Assert.True(expanded is RationalConstantExpr rc && rc.Value.IsZero,
                    $"{label}: the kernel's closed-form root {Printing.PrettyPrint(solution.Value)} does not satisfy {c.Sympy} exactly; sympy says [{theirValues}] [domain: {c.Domain}]");
            }
        }
    }

    [RequiresSympyFact]
    public void SympyOracle_Factorization_Agrees()
    {
        SympySession sympy = SympyOracle.Require();
        ExprContext ctx = NewContext(out Symbol x);
        foreach (FactorCase c in OracleCorpus.Factorizations(ctx))
        {
            string label = $"factor({c.Sympy})";
            Expr factored = Factoring.Factor(c.Expr, ctx);
            string mine = Printing.CanonicalPrint(factored);
            string expandsTo = Printing.CanonicalPrint(Algebra.Expand(factored, ctx));
            string original = Printing.CanonicalPrint(Algebra.Expand(c.Expr, ctx));
            Assert.True(expandsTo == original,
                $"{label}: the kernel factorization {mine} expands to {expandsTo}, not to the input {original} [domain: {c.Domain}]");

            IReadOnlyList<string> theirs = sympy.EvalLines(OracleCorpus.FactorProfileScript(c.Sympy, x));
            string myProfile = OracleCorpus.DegreeProfile(factored, ctx, x);
            Assert.True(myProfile == theirs[0],
                $"{label}: kernel factor degrees [{myProfile}] from {mine} vs sympy.factor_list degrees [{theirs[0]}] from {theirs[1]} [domain: {c.Domain}]");
        }
    }

    [RequiresSympyFact]
    public void SympyOracle_Limits_Agree()
    {
        SympySession sympy = SympyOracle.Require();
        ExprContext ctx = NewContext(out Symbol x);
        foreach (LimitCase c in OracleCorpus.Limits(ctx))
        {
            LimitResult result = Limits.Limit(c.Expr, x, c.Point, c.Direction, ctx);
            string label = $"limit({c.Sympy}, x -> {c.SympyPoint}, {c.Direction})";
            if (c.Direction == LimitDirection.TwoSided && result.Status == LimitStatus.DoesNotExist)
            {
                // a two-sided limit whose sides disagree is a RESULT, not a non-answer: the kernel
                // must say DoesNotExist and carry both sides, each compared with sympy's own side
                string? left = CompareLimit(ctx, result.FromLeft, SympyLimit(sympy, c, "-"));
                string? right = CompareLimit(ctx, result.FromRight, SympyLimit(sympy, c, "+"));
                Assert.True(left is null && right is null,
                    $"{label}: kernel {Describe(result)} vs sympy one-sided: {left ?? "left agrees"}, {right ?? "right agrees"} [domain: {c.Domain}]");
                continue;
            }
            string direction = c.Direction == LimitDirection.FromLeft ? "-" : "+";
            string? mismatch = CompareLimit(ctx, result, SympyLimit(sympy, c, direction));
            Assert.True(mismatch is null,
                mismatch is null ? $"{label} agrees with sympy" : $"{label}: {mismatch} [domain: {c.Domain}]");
        }
    }

    [RequiresSympyFact]
    public void SympyOracle_MatrixOperations_Agree()
    {
        SympySession sympy = SympyOracle.Require();
        ExprContext ctx = NewContext(out Symbol x);
        string[] symbols = { "x", "y" };
        foreach (MatrixCase c in OracleCorpus.MatrixOperations(ctx))
        {
            SymbolicMatrix kernel = SymbolicMatrix.From(c.Kernel);
            string label = $"{c.Operation}({c.Sympy})";
            switch (c.Operation)
            {
                case "det":
                case "trace":
                {
                    Expr value = c.Operation == "det" ? kernel.Det(ctx) : kernel.Trace();
                    string? mismatch = OracleCompare.NumberAgrees(
                        sympy, label, Printing.CanonicalPrint(value), $"({c.Sympy}).{c.Operation}()", ctx, symbols, c.Points);
                    Assert.True(mismatch is null,
                        mismatch is null ? $"{label} agrees with sympy" : $"{mismatch} [domain: {c.Domain}]");
                    break;
                }

                case "rank":
                {
                    int myRank = kernel.Rank(ctx);
                    string theirText = sympy.Eval(
                        SympyScript.Symbols(symbols) + "\n" + $"print(sympy.sstr(({c.Sympy}).rank()))");
                    Assert.True(int.TryParse(theirText, out int theirRank) && theirRank == myRank,
                        $"{label}: kernel rank {myRank} vs sympy rank {theirText} [domain: {c.Domain}]");
                    break;
                }

                case "inverse":
                {
                    SymbolicMatrix inverse = kernel.Inverse(ctx);
                    var kernelEntries = new List<string>();
                    var sympyEntries = new List<string>();
                    for (int row = 0; row < inverse.Rows; row++)
                    {
                        for (int column = 0; column < inverse.Columns; column++)
                        {
                            kernelEntries.Add(Printing.CanonicalPrint(inverse[row, column]));
                            sympyEntries.Add($"({c.Sympy}).inv()[{row}, {column}]");
                        }
                    }
                    string? mismatch = OracleCompare.NumberListsAgree(
                        sympy, label, kernelEntries, sympyEntries, ctx, symbols, c.Points);
                    Assert.True(mismatch is null,
                        mismatch is null ? $"{label} agrees with sympy" : $"{mismatch} [domain: {c.Domain}]");
                    break;
                }

                default:
                    Assert.Fail($"{label}: unknown matrix operation '{c.Operation}' in the corpus");
                    break;   // reachable only if Assert.Fail returns, which it never does
            }
        }
    }

    private static ExprContext NewContext(out Symbol x)
    {
        var ctx = new ExprContext();
        Exprs.Current = ctx;
        x = ctx.Symbol("x");
        return ctx;
    }

    private static string Describe(SolutionSet set) =>
        string.Join("; ", set.Solutions.Select(solution => Printing.CanonicalPrint(solution.Value)));

    private static string SympyLimit(SympySession sympy, LimitCase c, string direction) =>
        sympy.Eval(
            SympyScript.Symbols(new[] { "x" }) + "\n" +
            $"value = sympy.limit(({c.Sympy}), x, {c.SympyPoint}, dir='{direction}')\n" +
            "print(sympy.sstr(sympy.N(value, 30)) if value.is_finite else sympy.sstr(value))");

    private static string Describe(LimitResult result) => result.Status switch
    {
        LimitStatus.Value => $"{Printing.PrettyPrint(result.Value!)} (Value)",
        LimitStatus.PlusInfinity => "+oo",
        LimitStatus.MinusInfinity => "-oo",
        LimitStatus.DoesNotExist => $"DoesNotExist (left {result.FromLeft?.Status}, right {result.FromRight?.Status})",
        _ => $"{result.Status} ({result.FailureReason ?? "no reason"})",
    };

    /// <summary>Returns null when the kernel result matches the SymPy text; otherwise a triage
    /// message naming both sides.</summary>
    private static string? CompareLimit(ExprContext ctx, LimitResult? actual, string sympyText)
    {
        if (actual is null)
            return $"the kernel reported nothing vs sympy {sympyText}";
        string kernelText = Describe(actual);
        if (sympyText == "oo")
            return actual.Status == LimitStatus.PlusInfinity ? null : $"kernel {kernelText} vs sympy {sympyText}";
        if (sympyText == "-oo")
            return actual.Status == LimitStatus.MinusInfinity ? null : $"kernel {kernelText} vs sympy {sympyText}";
        if (actual.Status != LimitStatus.Value)
            return $"kernel {kernelText} vs sympy {sympyText}";

        using var precision = Rl.WithPrecision(80, 40);
        if (!Rl.TryParse(sympyText, null, out Rl? theirs))
            return $"kernel {kernelText} vs sympy {sympyText}, which is neither a finite number nor +/-oo";
        Num mine;
        try
        {
            mine = OracleCompare.ToNum(actual.Value!, ctx, new Dictionary<Symbol, Num>());
        }
        catch (Exception ex) when (ex is EvaluationException or InvalidOperationException)
        {
            return $"the kernel value {kernelText} did not evaluate ({ex.Message}) vs sympy {sympyText}";
        }
        Num difference = NumOps.Abs(NumOps.Subtract(mine, NumOps.FromReal(theirs)), ctx);
        return NumOps.Compare(difference, NumOps.FromReal(Rl.Parse(OracleCompare.TightTolerance, null))) < 0
            ? null
            : $"kernel {kernelText} vs sympy {sympyText}";
    }
}
