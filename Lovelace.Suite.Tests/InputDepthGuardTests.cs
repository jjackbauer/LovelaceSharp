using Lovelace.Symbolics;
using Xunit;

namespace Lovelace.Suite.Tests;

/// <summary>
/// The nesting guard at its three enforcement points. Deep input reaches a native recursion from
/// three different directions and each one is a separate walk, so each is checked here directly:
/// <list type="bullet">
/// <item>the parser's recursive descent (nested groups, calls, lists, prefix operators),</item>
/// <item>the parsed tree, which the parser's LOOPS can make deep without ever descending
/// (<c>1-1-1-…</c>),</item>
/// <item>canonical symbolic construction, where a shallow script can build a deep expression at
/// run time (<c>for i in 1..1000 { e = abs(e) }</c>).</item>
/// </list>
/// <para>
/// The refusal is asserted by type NAME and message text rather than by referencing the exception
/// type, so this file also compiles against the pre-guard tree — which is what makes the
/// control-tree evidence (a test-host crash, 0xC00000FD) reproducible with it.
/// </para>
/// </summary>
public class InputDepthGuardTests
{
    /// <summary>The nesting budget: parser descent and canonical symbolic construction.</summary>
    private const int MaxDepth = 256;

    /// <summary>The parsed-tree budget: the interpreter's walk over the parsed tree. It is larger
    /// because a flat associative chain is legitimate, wide input (the shipped suites sum 420
    /// terms) and its parsed tree is one level per operator.</summary>
    private const int MaxParsedTreeDepth = 512;

    private static string Repeat(string text, int count) => string.Concat(Enumerable.Repeat(text, count));

    private static string ParensAtDepth(int depth) => Repeat("(", depth - 2) + "1" + Repeat(")", depth - 2) + ";";

    private static string FlatChainAtDepth(int depth) => "1" + Repeat("-1", depth - 3) + ";";

    private static Program ParseProgram(string source) =>
        new Parser().ParseProgram(new Tokenizer().Tokenize(source));

    private static Exception? Refusal(string source) => Record.Exception(() => ParseProgram(source));

    [Fact]
    public void NestingAtTheBudget_StillParses()
    {
        var program = ParseProgram(ParensAtDepth(MaxDepth));
        Assert.Single(program.Statements);
        Assert.Equal("1", ((LiteralExpr)((ExpressionStatement)program.Statements[0]).Expression).RawText);
    }

    [Fact]
    public void NestingOnePastTheBudget_IsRefusedWithTheTypedDepthError()
    {
        var ex = Refusal(ParensAtDepth(MaxDepth + 1));

        Assert.NotNull(ex);
        Assert.Equal("InputDepthExceededException", ex!.GetType().Name);
        Assert.Contains((MaxDepth + 1).ToString(), ex.Message);
        Assert.Contains(MaxDepth.ToString(), ex.Message);
    }

    /// <summary>A flat chain is built by a parser LOOP: the descent counter stays flat while the
    /// returned tree grows one level per operator, so only the tree walk can refuse it.</summary>
    [Fact]
    public void FlatChainOnePastTheBudget_IsRefused()
    {
        Assert.NotNull(ParseProgram(FlatChainAtDepth(MaxParsedTreeDepth)));
        // 420 terms is the widest flat sum the shipped suites use; it must stay accepted
        Assert.NotNull(ParseProgram(FlatChainAtDepth(423)));

        var ex = Refusal(FlatChainAtDepth(MaxParsedTreeDepth + 1));
        Assert.NotNull(ex);
        Assert.Equal("InputDepthExceededException", ex!.GetType().Name);
        Assert.Contains("tree depth", ex.Message);
        Assert.Contains(MaxParsedTreeDepth.ToString(), ex.Message);
    }

    /// <summary>Prefix operators are a descent of their own (<c>ParseUnary</c> recurses into
    /// itself); a deep unary chain must be refused before that recursion runs away.</summary>
    [Fact]
    public void UnaryChainOnePastTheBudget_IsRefused()
    {
        Assert.NotNull(ParseProgram(Repeat("-", MaxDepth - 3) + "1;"));
        var ex = Refusal(Repeat("-", MaxDepth + 1) + "1;");
        Assert.NotNull(ex);
        Assert.Equal("InputDepthExceededException", ex!.GetType().Name);
    }

    [Fact]
    public void NestedBlocksOnePastTheBudget_AreRefused()
    {
        // a literal inside n nested blocks sits at depth n + 3 (program, block chain, statement)
        Assert.NotNull(ParseProgram(Repeat("{", MaxDepth - 3) + "1;" + Repeat("}", MaxDepth - 3)));
        var ex = Refusal(Repeat("{", MaxDepth + 1) + "1;" + Repeat("}", MaxDepth + 1));
        Assert.NotNull(ex);
        Assert.Equal("InputDepthExceededException", ex!.GetType().Name);
    }

    /// <summary>Canonical construction is the third funnel: a loop can wrap a value once per
    /// iteration with nothing deep in the source or the AST.</summary>
    [Fact]
    public void SymbolicConstructionOnePastTheBudget_IsRefused()
    {
        Lovelace.Symbolics.Expr Build(int wrappers)
        {
            Lovelace.Symbolics.Expr expr = Exprs.Symbol("x");
            for (int i = 0; i < wrappers; i++)
                expr = Exprs.Function(Exprs.Current.Function("abs"), expr);
            return expr;
        }

        Assert.NotNull(Build(MaxDepth - 1));

        var ex = Record.Exception(() => Build(MaxDepth));
        Assert.NotNull(ex);
        Assert.Equal("InputDepthExceededException", ex!.GetType().Name);
        Assert.Contains("symbolic expression depth", ex.Message);
    }

    /// <summary>Ordinary nesting is untouched, and the composed depth of an interpolation is
    /// measured as one input (its own parser starts at the enclosing depth).</summary>
    [Fact]
    public void OrdinaryNesting_AndInterpolation_StillParse()
    {
        Assert.NotNull(ParseProgram("-(-(-(1 + 2*3))) + sqrt(sqrt(16)) + abs(-2);"));
        Assert.NotNull(ParseProgram("x = 1; s = $\"v={x + 1}\";"));
    }
}
