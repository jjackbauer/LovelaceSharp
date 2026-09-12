using Lovelace.Abstractions;
using Lovelace.MathIR;
using Lovelace.Suite;
using Lovelace.Symbolics;
using Rl = Lovelace.Real.Real;
using Xunit;

namespace Lovelace.Symbolics.Tests;

/// <summary>
/// The indeterminate exponential family <c>(1 + u)^v</c> with <c>u → 0</c> and <c>v → ∞</c>: the
/// value at the point does not exist, so direct substitution is NOT a proof there. The kernel used
/// to substitute anyway, the canonicaliser folded <c>Power(1, 1/0)</c> to <c>1</c>, and the record
/// published <c>1</c> with <c>exists: true</c> and an exactness claim:
/// <c>limit((1 + 1/x)^x, x, ∞)</c> answered 1 where the limit is <c>e</c>.
/// <para>
/// Every SymPy value quoted below is the output of the local SymPy 1.14.0 (the differential oracle
/// in <c>DifferentialOracleTests</c> compares the same cases numerically; this file pins them
/// exactly and runs without python). The exactness comparisons are spelled as
/// <c>ToString()</c> strings on purpose: this file is the falsification test that must also COMPILE
/// and FAIL on the pristine control tree, so it cannot name the kernel's exactness enum member
/// directly — the wire assertion next to it names the declared <c>SolutionExactness</c> value that
/// actually crosses the protocol.
/// </para>
/// </summary>
public class IndeterminateExponentialLimitTests
{
    private const string TightTolerance = "0.000000000000000000001";   // 1e-21, 30+ digits available

    private static Symbol NewContext(out ExprContext ctx)
    {
        ctx = new ExprContext();
        Exprs.Current = ctx;
        return ctx.Symbol("x");
    }

    private static SuiteEngine NewEngine()
    {
        var engine = new SuiteEngine();
        var symbolics = new SymbolicsPlugin();
        engine.LoadPlugin(symbolics);
        engine.LoadPlugin(new MathIRPlugin(symbolics));
        return engine;
    }

    private static Value F(RecordValue record, string name) =>
        (Value)record.Fields.First(f => f.Name == name).Value!;

    private static void AssertEnumField(object? payload, string typeName, string memberName)
    {
        var value = (Value)payload!;
        Assert.Equal(ValueKind.Enum, value.Kind);
        Assert.Equal(typeName, value.AsEnum().TypeName);
        Assert.Equal(memberName, value.AsEnum().Name);
    }

    /// <summary>The kernel value IS the expected exact closed form, it is not an approximation
    /// (a float would be <c>Approximate</c> and a RealConstant), and it evaluates to SymPy's own
    /// 30-digit value.</summary>
    private static void AssertExactClosedForm(LimitResult r, ExprContext ctx, Expr expected, string? sympy30Digits)
    {
        Assert.Equal(LimitStatus.Value, r.Status);
        Assert.True(expected.Equals(r.Value),
            $"kernel answered {Printing.CanonicalPrint(r.Value!)} instead of the exact {Printing.CanonicalPrint(expected)}");
        Assert.Equal("AlgebraicExact", r.Exactness.ToString());
        Assert.IsNotType<RealConstantExpr>(r.Value);
        if (sympy30Digits is null)
            return;   // a template with a free parameter has no single numeric value to compare

        using var precision = Rl.WithPrecision(80, 40);
        Num mine = Evaluation.EvaluateToNum(r.Value!, ctx, new Dictionary<Symbol, Num>());
        Num difference = NumOps.Abs(
            NumOps.Subtract(mine, NumOps.FromReal(Rl.Parse(sympy30Digits, null))), ctx);
        Assert.True(NumOps.Compare(difference, NumOps.FromReal(Rl.Parse(TightTolerance, null))) < 0,
            $"kernel value {Printing.CanonicalPrint(r.Value!)} evaluates to {OracleCompare.Show(mine)}, not to sympy's {sympy30Digits}");
    }

    // ------------------------------------------------------------------
    // 1. the three probes, against SymPy's own values
    // ------------------------------------------------------------------

    /// <summary>SymPy 1.14.0:
    /// <c>limit((1 + 1/x)**x, x, oo)</c> = <c>E</c>, <c>N(E, 30)</c> = 2.71828182845904523536028747135.
    /// Before the fix the kernel answered the exact rational 1 — its value at the (undefined) point.</summary>
    [Fact]
    public void Probe_LimitOfOnePlusOneOverXToTheX_IsE()
    {
        Symbol x = NewContext(out ExprContext ctx);
        Expr f = Exprs.Power(Exprs.Add(Exprs.One, Exprs.Divide(Exprs.One, x)), x);
        AssertExactClosedForm(
            Limits.Limit(f, x, Exprs.Infinity, LimitDirection.TwoSided, ctx), ctx,
            Exprs.E, "2.71828182845904523536028747135");
    }

    /// <summary>SymPy 1.14.0: <c>limit((1 + x)**(1/x), x, 0)</c> = <c>E</c> from either side
    /// (<c>dir='+'</c> and <c>dir='-'</c> are both <c>E</c>), N(E, 30) = 2.71828182845904523536028747135.</summary>
    [Fact]
    public void Probe_LimitOfOnePlusXToTheOneOverX_IsE()
    {
        Symbol x = NewContext(out ExprContext ctx);
        Expr f = Exprs.Power(Exprs.Add(Exprs.One, x), Exprs.Divide(Exprs.One, x));
        AssertExactClosedForm(
            Limits.Limit(f, x, Exprs.Zero, LimitDirection.TwoSided, ctx), ctx,
            Exprs.E, "2.71828182845904523536028747135");
        AssertExactClosedForm(
            Limits.Limit(f, x, Exprs.Zero, LimitDirection.FromRight, ctx), ctx,
            Exprs.E, "2.71828182845904523536028747135");
        AssertExactClosedForm(
            Limits.Limit(f, x, Exprs.Zero, LimitDirection.FromLeft, ctx), ctx,
            Exprs.E, "2.71828182845904523536028747135");
    }

    /// <summary>SymPy 1.14.0: <c>limit((1 + 2/x)**x, x, oo)</c> = <c>exp(2)</c>,
    /// <c>N(exp(2), 30)</c> = 7.38905609893065022723042746058. Before the fix the kernel answered 1.</summary>
    [Fact]
    public void Probe_LimitOfOnePlusTwoOverXToTheX_IsESquared()
    {
        Symbol x = NewContext(out ExprContext ctx);
        Expr f = Exprs.Power(Exprs.Add(Exprs.One, Exprs.Divide(2, x)), x);
        AssertExactClosedForm(
            Limits.Limit(f, x, Exprs.Infinity, LimitDirection.TwoSided, ctx), ctx,
            Exprs.Power(Exprs.E, Exprs.Integer(2)), "7.38905609893065022723042746058");
    }

    /// <summary>The same family with neither a unit rate nor a unit exponent, and at −∞:
    /// SymPy 1.14.0 gives <c>limit((1 + 1/x)**(3*x), x, oo)</c> = <c>exp(3)</c>
    /// (N = 20.0855369231876677409285296546), <c>limit((1 + 1/x)**x, x, -oo)</c> = <c>E</c>, and
    /// <c>limit((1 + 3/x)**(x/2), x, oo)</c> = <c>exp(3/2)</c> (N = 4.48168907033806482260205546012).
    /// A parameter in the rate stays exact too: SymPy gives <c>exp(a)</c> for
    /// <c>limit((1 + x)**(a/x), x, 0)</c> with a positive symbol a.</summary>
    [Fact]
    public void Family_AnswersEveryExactExponent()
    {
        Symbol x = NewContext(out ExprContext ctx);
        AssertExactClosedForm(
            Limits.Limit(
                Exprs.Power(Exprs.Add(Exprs.One, Exprs.Divide(Exprs.One, x)), Exprs.Multiply(3, x)),
                x, Exprs.Infinity, LimitDirection.TwoSided, ctx), ctx,
            Exprs.Power(Exprs.E, Exprs.Integer(3)), "20.0855369231876677409285296546");
        AssertExactClosedForm(
            Limits.Limit(
                Exprs.Power(Exprs.Add(Exprs.One, Exprs.Divide(Exprs.One, x)), x),
                x, Exprs.Negate(Exprs.Infinity), LimitDirection.TwoSided, ctx), ctx,
            Exprs.E, "2.71828182845904523536028747135");
        AssertExactClosedForm(
            Limits.Limit(
                Exprs.Power(Exprs.Add(Exprs.One, Exprs.Divide(3, x)), Exprs.Divide(x, 2)),
                x, Exprs.Infinity, LimitDirection.TwoSided, ctx), ctx,
            Exprs.Power(Exprs.E, Exprs.Rational(3, 2)), "4.48168907033806482260205546012");
        AssertExactClosedForm(
            Limits.Limit(
                Exprs.Power(Exprs.Add(Exprs.One, x), Exprs.Divide(ctx.Symbol("a"), x)),
                x, Exprs.Zero, LimitDirection.TwoSided, ctx), ctx,
            Exprs.Power(Exprs.E, Exprs.Symbol(ctx.Symbol("a"))), null);   // e^a: exact template, no numeric value
    }

    // ------------------------------------------------------------------
    // 2. the substitution value is never the answer
    // ------------------------------------------------------------------

    /// <summary>
    /// Every one of these has a FINITE value at the point (1, 1, 1 and 0 respectively) that is not
    /// its limit, so answering it would publish a wrong number with <c>exists: true</c>. SymPy
    /// 1.14.0 gives +oo for the first three and e − 1 for the sum; the kernel refuses each with a
    /// typed Unevaluated instead. A finite wrong answer is worse than a refusal here.
    /// </summary>
    [Fact]
    public void IndeterminateShapes_AreRefused_NeverAnsweredWithTheValueAtThePoint()
    {
        Symbol x = NewContext(out ExprContext ctx);
        var cases = new (string Where, Expr F, Expr Point, LimitDirection Direction, string Sympy, string Substituted)[]
        {
            ("limit((2 + x)^(1/x), x, 0, right)",                       // sympy oo, substitution would answer 1
                Exprs.Power(Exprs.Add(2, x), Exprs.Divide(Exprs.One, x)),
                Exprs.Zero, LimitDirection.FromRight, "oo", "1"),
            ("limit((1 + 1/x)^(x^2), x, inf)",                          // sympy oo, substitution would answer 1
                Exprs.Power(Exprs.Add(Exprs.One, Exprs.Divide(Exprs.One, x)), Exprs.Power(x, 2)),
                Exprs.Infinity, LimitDirection.TwoSided, "oo", "1"),
            ("limit((2^(1/x))^x, x, 0, right)",                         // sympy 2, substitution would answer 1
                Exprs.Power(Exprs.Power(2, Exprs.Divide(Exprs.One, x)), x),
                Exprs.Zero, LimitDirection.FromRight, "2", "1"),
            ("limit((1 + x)^(1/x) - 1, x, 0)",                          // sympy e - 1, substitution would answer 0
                Exprs.Subtract(Exprs.Power(Exprs.Add(Exprs.One, x), Exprs.Divide(Exprs.One, x)), Exprs.One),
                Exprs.Zero, LimitDirection.TwoSided, "E - 1", "0"),
        };

        foreach ((string where, Expr f, Expr point, LimitDirection direction, string sympy, string substituted) in cases)
        {
            LimitResult r = Limits.Limit(f, x, point, direction, ctx);
            string answered = r.Value is null ? "(no value)" : Printing.CanonicalPrint(r.Value);
            Assert.True(r.Status == LimitStatus.Unevaluated,
                $"{where}: kernel answered {r.Status} {answered} — sympy says {sympy}, " +
                $"and the value at the point ({substituted}) is not the limit");
            Assert.Null(r.Value);
            Assert.False(string.IsNullOrWhiteSpace(r.FailureReason), $"{where}: a refusal must carry its reason");
            Assert.Equal("None", r.Exactness.ToString());
        }
    }

    // ------------------------------------------------------------------
    // 3. nothing else moved
    // ------------------------------------------------------------------

    /// <summary>
    /// The five limits measured on the pristine tree BEFORE this change, with exactly the values it
    /// answered then: the guard has to leave every determined limit alone. 1/x is the odd pole
    /// whose sides disagree (DoesNotExist with both sides attached).
    /// </summary>
    [Fact]
    public void DeterminedLimits_KeepExactlyTheirPreviousAnswers()
    {
        Symbol x = NewContext(out ExprContext ctx);
        FunctionId sin = ctx.Function("sin");
        FunctionId log = ctx.Function("log");

        LimitResult ratio = Limits.Limit(Exprs.Divide(Exprs.Function(sin, x), x), x, Exprs.Zero, LimitDirection.TwoSided, ctx);
        Assert.Equal(LimitStatus.Value, ratio.Status);
        Assert.Equal(Exprs.One, ratio.Value);
        Assert.Equal("Exact", ratio.Exactness.ToString());

        LimitResult pole = Limits.Limit(Exprs.Divide(Exprs.One, x), x, Exprs.Zero, LimitDirection.TwoSided, ctx);
        Assert.Equal(LimitStatus.DoesNotExist, pole.Status);
        Assert.Equal(LimitStatus.MinusInfinity, pole.FromLeft!.Status);
        Assert.Equal(LimitStatus.PlusInfinity, pole.FromRight!.Status);

        LimitResult atInfinity = Limits.Limit(
            Exprs.Divide(Exprs.Add(Exprs.Power(x, 2), Exprs.One), Exprs.Subtract(Exprs.Power(x, 2), Exprs.One)),
            x, Exprs.Infinity, LimitDirection.TwoSided, ctx);
        Assert.Equal(LimitStatus.Value, atInfinity.Status);
        Assert.Equal(Exprs.One, atInfinity.Value);

        LimitResult polynomial = Limits.Limit(Exprs.Add(Exprs.Power(x, 2), Exprs.One), x, Exprs.Integer(2), LimitDirection.TwoSided, ctx);
        Assert.Equal(LimitStatus.Value, polynomial.Status);
        Assert.Equal(Exprs.Integer(5), polynomial.Value);

        LimitResult squeezed = Limits.Limit(Exprs.Multiply(x, Exprs.Function(log, x)), x, Exprs.Zero, LimitDirection.FromRight, ctx);
        Assert.Equal(LimitStatus.Value, squeezed.Status);
        Assert.Equal(Exprs.Zero, squeezed.Value);
    }

    /// <summary>A limit that IS the substitution value still takes the fast path: the guard fires on
    /// an exponent with no value at the point, not on every power. SymPy 1.14.0 agrees on all four
    /// through the published builtin.</summary>
    [Fact]
    public void SubstitutionShapedLimits_StillAnswerImmediately()
    {
        var engine = NewEngine();
        engine.Evaluate("x = symbol(\"x\")");
        // audit D F2 (cycle 6): the short form publishes the LimitResult record, so the value a
        // caller wants is the record's own "value" field. Same seven values, read off structure.
        AssertShortFormValue(engine, "limit(x^2 + 1, x, 2)", "5");
        AssertShortFormValue(engine, "limit((1 + x)^2, x, 0)", "1");
        AssertShortFormValue(engine, "limit(2^x, x, 3)", "8");
        AssertShortFormValue(engine, "limit(x^x, x, 0)", "1");
        // and the probes answer SymPy's E and exp(2) through the same builtin
        AssertShortFormValue(engine, "limit((1 + 1/x)^x, x, inf)", "e");
        AssertShortFormValue(engine, "limit((1 + x)^(1/x), x, 0)", "e");
        AssertShortFormValue(engine, "limit((1 + 2/x)^x, x, inf)", "e^2");
    }

    /// <summary>audit D F2 (cycle 6): a short-form limit is a LimitResult record, so the answer is
    /// the record's <c>value</c> field and the record's own <c>status</c>/<c>exists</c> say that it
    /// IS an answer. The value assertion is the one this file already made; the two beside it are
    /// what the bare result could not carry.</summary>
    private static void AssertShortFormValue(SuiteEngine engine, string call, string expected)
    {
        var record = engine.Evaluate(call).AsRecord();
        Assert.Equal("LimitResult", record.TypeName);
        AssertEnumField(F(record, "status"), "LimitStatus", "Value");
        Assert.True(F(record, "exists").AsBoolean(), $"{call}: a determined limit reports exists: true");
        Assert.Equal(expected, ValueFormatter.Format(F(record, "value")));
    }

    // ------------------------------------------------------------------
    // 4. a record with no value claims no exactness
    // ------------------------------------------------------------------

    /// <summary>
    /// The audit's P0 for this record: <c>limit_full(x*sin(1/x), x, 0)</c> is Unevaluated with
    /// <c>value</c> and <c>exists</c> both Null, and it used to report
    /// <c>exactness = SolutionExactness.Exact</c> — an exactness claim about no result. The wire now
    /// carries <c>None</c> for it, while a determined record keeps the exactness it earns.
    /// </summary>
    [Fact]
    public void UndeterminedLimit_ClaimsNoExactness_AndDeterminedOnesKeepTheirs()
    {
        Symbol x = NewContext(out ExprContext ctx);
        Expr squeezed = Exprs.Multiply(x, Exprs.Function(ctx.Function("sin"), Exprs.Divide(Exprs.One, x)));
        LimitResult kernel = Limits.Limit(squeezed, x, Exprs.Zero, LimitDirection.TwoSided, ctx);
        Assert.Equal(LimitStatus.Unevaluated, kernel.Status);
        Assert.Null(kernel.Value);
        Assert.Null(kernel.FromLeft);
        Assert.Null(kernel.FromRight);
        Assert.Equal("None", kernel.Exactness.ToString());

        var engine = NewEngine();
        engine.Evaluate("x = symbol(\"x\")");

        RecordValue undetermined = engine.Evaluate("limit_full(x*sin(1/x), x, 0)").AsRecord();
        AssertEnumField(F(undetermined, "status"), "LimitStatus", "Unevaluated");
        Assert.Equal(ValueKind.Void, F(undetermined, "exists").Kind);
        Assert.Equal(ValueKind.Void, F(undetermined, "value").Kind);
        AssertEnumField(F(undetermined, "exactness"), "SolutionExactness", "None");

        RecordValue determined = engine.Evaluate("limit_full(sin(x)/x, x, 0)").AsRecord();
        AssertEnumField(F(determined, "exactness"), "SolutionExactness", "Exact");

        // a DoesNotExist record carries a determination (its two one-sided results), so its
        // exactness is derived from those sides — it is not a value claim
        RecordValue pole = engine.Evaluate("limit_full(1/x, x, 0)").AsRecord();
        AssertEnumField(F(pole, "status"), "LimitStatus", "DoesNotExist");
        AssertEnumField(F(pole, "exactness"), "SolutionExactness", "Exact");
    }

    /// <summary>The probes through the wire: the value is the exact <c>e</c> / <c>e^2</c> and the
    /// record says so (AlgebraicExact, a closed form — never Approximate).</summary>
    [Fact]
    public void Probes_CrossTheWireAsExactClosedForms()
    {
        var engine = NewEngine();
        engine.Evaluate("x = symbol(\"x\")");

        RecordValue first = engine.Evaluate("limit_full((1 + 1/x)^x, x, inf)").AsRecord();
        AssertEnumField(F(first, "status"), "LimitStatus", "Value");
        Assert.Equal("e", ValueFormatter.Format(F(first, "value")));
        AssertEnumField(F(first, "exactness"), "SolutionExactness", "AlgebraicExact");

        RecordValue second = engine.Evaluate("limit_full((1 + 2/x)^x, x, inf)").AsRecord();
        AssertEnumField(F(second, "status"), "LimitStatus", "Value");
        Assert.Equal("e^2", ValueFormatter.Format(F(second, "value")));
        AssertEnumField(F(second, "exactness"), "SolutionExactness", "AlgebraicExact");
    }
}
