using Lovelace.Knowledge;

namespace Lovelace.Knowledge.Tests;

public class ObservationTests
{
    [Fact]
    public void Canonicalize_SuccessReal_KeepsKindAndTyped()
    {
        var o = CanonicalObservation.FromRunnerOutput(
            """{"ok":true,"revision":1,"result":{"kind":"Real","display":"0.(3)","typed":"0.(3) (Real)"},"elapsed":"1 ms"}""");
        Assert.True(o.Success);
        Assert.Equal("ok|Real|0.(3) (Real)", o.Sigma);
        Assert.Equal("Real", CanonicalObservation.PlaneSigma(o));
    }

    [Fact]
    public void Canonicalize_Error_KeepsMessageOnly()
    {
        var o = CanonicalObservation.FromRunnerOutput(
            """{"ok":false,"message":"Cannot divide by zero.","diagnostics":[],"elapsed":"1 ms"}""");
        Assert.False(o.Success);
        Assert.Equal("err|Cannot divide by zero.", o.Sigma);
        Assert.Equal("err|Cannot divide by zero.", CanonicalObservation.PlaneSigma(o));
    }

    [Fact]
    public void PlaneSigma_Boolean_IsTagged()
    {
        var o = CanonicalObservation.FromRunnerOutput(
            """{"ok":true,"result":{"kind":"Boolean","display":"True","typed":"True (Boolean)"}}""");
        Assert.Equal("Boolean:True", CanonicalObservation.PlaneSigma(o));
    }

    [Fact]
    public void PlaneSigma_Real_IsSingleClass()
    {
        var periodic = CanonicalObservation.FromRunnerOutput(
            """{"ok":true,"result":{"kind":"Real","typed":"0.(3) (Real)"}}""");
        var terminating = CanonicalObservation.FromRunnerOutput(
            """{"ok":true,"result":{"kind":"Real","typed":"0.5 (Real)"}}""");
        Assert.Equal("Real", CanonicalObservation.PlaneSigma(periodic));
        Assert.Equal("Real", CanonicalObservation.PlaneSigma(terminating));
    }
}
