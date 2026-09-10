using Lovelace.Symbolics;
using Xunit;

namespace Lovelace.Symbolics.Tests;

/// <summary>
/// Print budget (Cycle 2 item 17). A rendering beyond the budget is abbreviated — never silently
/// truncated — and the caller gets a structured diagnostic saying why, how big the expression
/// really is, and what budget stopped it. Without a budget nothing changes.
/// </summary>
public class PrintBudgetTests
{
    private static (ExprContext Ctx, Symbol X, Expr Big) BigExpand(int power)
    {
        var ctx = new ExprContext();
        Exprs.Current = ctx;
        var x = ctx.Symbol("x");
        var big = Algebra.Expand(Exprs.Power(Exprs.Add(Exprs.Symbol(x), Exprs.One), power), ctx);
        return (ctx, x, big);
    }

    [Fact]
    public void NoBudget_IsExactlyTheUnboundedRendering()
    {
        var (_, _, big) = BigExpand(12);
        var outcome = Printing.Print(big, new Printing.PrintOptions());
        Assert.False(outcome.Truncated);
        Assert.Null(outcome.Truncation);
        Assert.Equal(Printing.PrettyPrint(big), outcome.Text);
    }

    [Fact]
    public void BudgetLargerThanTheExpression_ChangesNothing()
    {
        var (_, _, big) = BigExpand(12);
        var outcome = Printing.Print(big, new Printing.PrintOptions(Budget: new Printing.PrintBudget(MaxNodes: 100_000)));
        Assert.False(outcome.Truncated);
        Assert.Equal(Printing.PrettyPrint(big), outcome.Text);
    }

    [Fact]
    public void NodeBudget_ReportsReasonCountBudgetAndPartialResult()
    {
        var (_, _, big) = BigExpand(20);
        var full = Printing.PrettyPrint(big);
        var outcome = Printing.Print(big, new Printing.PrintOptions(Budget: new Printing.PrintBudget(MaxNodes: 20)));

        Assert.True(outcome.Truncated);
        var truncation = Assert.IsType<Printing.PrintTruncation>(outcome.Truncation);
        Assert.Equal("node-budget", truncation.Reason);
        Assert.Equal(big.NodeCount, truncation.NodeCount);
        Assert.Equal(20, truncation.Budget);
        Assert.True(big.NodeCount > 20);
        // the abbreviation is a PREFIX of the true rendering, so ordering and precedence match
        Assert.StartsWith(outcome.Text[..^2], full);
        Assert.EndsWith(" …", outcome.Text);
        Assert.True(outcome.Text.Length < full.Length);
    }

    [Fact]
    public void NodeBudget_OnACompactRendering_DropsNothingAndSaysSo()
    {
        var (ctx, _, _) = BigExpand(2);
        var small = Exprs.Add(Exprs.Symbol(ctx.Symbol("x")), Exprs.One);
        var outcome = Printing.Print(small, new Printing.PrintOptions(Budget: new Printing.PrintBudget(MaxNodes: 1)));
        // the node count exceeds the budget but the rendering is tiny: nothing was dropped, so
        // nothing is reported
        Assert.False(outcome.Truncated);
        Assert.Equal(Printing.PrettyPrint(small), outcome.Text);
    }

    [Fact]
    public void DepthBudget_ReportsDepthLimit()
    {
        var (_, _, big) = BigExpand(12);
        var outcome = Printing.Print(big, new Printing.PrintOptions(Budget: new Printing.PrintBudget(MaxDepth: 2)));
        Assert.True(outcome.Truncated);
        Assert.Equal("depth-limit", outcome.Truncation!.Reason);
        Assert.Equal(2, outcome.Truncation.Budget);
    }

    [Fact]
    public void CanonicalMode_HonoursTheSameBudget()
    {
        var (_, _, big) = BigExpand(20);
        var canonical = new Printing.PrintOptions(Printing.PrintMode.Canonical, Budget: new Printing.PrintBudget(MaxNodes: 20));
        var outcome = Printing.Print(big, canonical);
        Assert.True(outcome.Truncated);
        Assert.StartsWith("(add ", outcome.Text, StringComparison.Ordinal);
        Assert.Equal("node-budget", outcome.Truncation!.Reason);
    }

    [Fact]
    public void Abbreviation_NeverSplitsAToken()
    {
        var (_, _, big) = BigExpand(20);
        var outcome = Printing.Print(big, new Printing.PrintOptions(Budget: new Printing.PrintBudget(MaxNodes: 20)));
        var body = outcome.Text[..^2];
        // the cut lands on a boundary and drops whole tokens: the retained text never ends in the
        // middle of a number or identifier
        var last = body[^1];
        Assert.False(char.IsLetterOrDigit(last) || last == '.',
            $"the retained text ends inside a token: …'{body[^Math.Min(12, body.Length)..]}'");
        var full = Printing.PrettyPrint(big);
        Assert.StartsWith(body, full);
    }
}
