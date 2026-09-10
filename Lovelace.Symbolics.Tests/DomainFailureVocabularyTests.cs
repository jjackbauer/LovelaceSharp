using Lovelace.Symbolics;
using Xunit;
using Rl = global::Lovelace.Real.Real;

namespace Lovelace.Symbolics.Tests;

/// <summary>
/// The kernel's numeric domain failures use ONE vocabulary (Cycle 2, rounds 22–23). A framework
/// exception escaping here is not cosmetic: the narrowed catches in the limit, integration, fold
/// and solver-verification paths catch <see cref="EvaluationException"/> specifically, so a foreign
/// type bypasses them and surfaces to a host as an internal defect. These tests pin the type, not
/// just "it throws".
/// </summary>
public class DomainFailureVocabularyTests
{
    private static readonly ExprContext Ctx = new();

    [Fact]
    public void DivisionByZero_IsAnEvaluationFailure()
    {
        var ex = Record.Exception(() => NumOps.Divide(NumOps.FromLong(1L), NumOps.FromLong(0L)));
        Assert.IsType<EvaluationException>(ex);
    }

    [Fact]
    public void DivisionByZero_OnTheRealPath_IsAnEvaluationFailure()
    {
        var zero = NumOps.FromReal(Rl.Parse("0", null));
        var ex = Record.Exception(() => NumOps.Divide(NumOps.FromLong(1L), zero));
        Assert.IsType<EvaluationException>(ex);
    }

    [Fact]
    public void LogOfZero_IsAnEvaluationFailure()
    {
        var ex = Record.Exception(() => NumOps.Ln(NumOps.FromLong(0L), Ctx));
        Assert.IsType<EvaluationException>(ex);
    }

    [Fact]
    public void ZeroToANegativePower_IsAnEvaluationFailure()
    {
        var ex = Record.Exception(() => NumOps.Pow(NumOps.FromLong(0L), NumOps.FromLong(-1L)));
        Assert.IsType<EvaluationException>(ex);
    }
}
