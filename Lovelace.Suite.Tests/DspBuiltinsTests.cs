using Lovelace.Abstractions;
using Lovelace.Suite;

namespace Lovelace.Suite.Tests;

/// <summary>
/// Verifies the opt-in DSP registration (<see cref="SuiteEngine.RegisterDspBuiltins"/>) and the
/// complex accessors (<c>re</c>/<c>im</c>/<c>conj</c>/<c>abs</c>). A bare engine must not expose
/// the DSP builtins; a registered engine must, and its complex results must be bridgeable back to
/// the <c>Natural → Integer → Real</c> lattice.
/// </summary>
public class DspBuiltinsTests
{
    [Fact]
    public async Task DspBuiltins_GivenUnregisteredEngine_ThrowsUnknownFunction()
    {
        var engine = new SuiteEngine();

        var ex = await Assert.ThrowsAsync<InvalidOperationException>(
            () => engine.EvaluateAsync("fft([1, 0, 0, 0])"));

        Assert.Equal("Unknown function 'fft'.", ex.Message);
    }

    [Fact]
    public async Task Fft_GivenImpulse_ReturnsAllOnes()
    {
        var engine = new SuiteEngine();
        engine.RegisterDspBuiltins();

        var result = await engine.EvaluateAsync("fft([1, 0, 0, 0])");

        Assert.Equal("[1, 1, 1, 1] (Vector)", ValueFormatter.FormatTyped(result));
    }

    [Fact]
    public async Task Fft_GivenImpulse_ReturnsComplexTypedVector()
    {
        var engine = new SuiteEngine();
        engine.RegisterDspBuiltins();

        var result = await engine.EvaluateAsync("fft([0, 1, 0, 0])");

        var array = result.AsArrayValue();
        Assert.Equal(DType.Complex, array.DType);
        Assert.Equal(ValueKind.Complex, ((Value)array.GetElement(0)).Kind);
    }

    [Fact]
    public async Task ComplexAccessors_GivenComplexValue_ReturnRealComponents()
    {
        var engine = new SuiteEngine();
        engine.RegisterDspBuiltins();

        // fft([0, 1, 0, 0])[1] = e^(-iπ/2) = -i
        Assert.Equal("0 (Real)", ValueFormatter.FormatTyped(await engine.EvaluateAsync("z = fft([0, 1, 0, 0])[1]; re(z)")));
        Assert.Equal("-1 (Real)", ValueFormatter.FormatTyped(await engine.EvaluateAsync("z = fft([0, 1, 0, 0])[1]; im(z)")));
        Assert.Equal("1 (Real)", ValueFormatter.FormatTyped(await engine.EvaluateAsync("z = fft([0, 1, 0, 0])[1]; abs(z)")));
        Assert.Equal("1i (Complex)", ValueFormatter.FormatTyped(await engine.EvaluateAsync("z = fft([0, 1, 0, 0])[1]; conj(z)")));
    }

    [Fact]
    public async Task Abs_GivenComplex_ReturnsMagnitude()
    {
        var engine = new SuiteEngine();
        engine.RegisterDspBuiltins();

        var result = await engine.EvaluateAsync("abs(fft([0, 1, 0, 0])[1])");

        Assert.Equal(ValueKind.Real, result.Kind);
        Assert.Equal("1 (Real)", ValueFormatter.FormatTyped(result));
    }

    [Fact]
    public async Task ComplexArithmetic_GivenComplex_ThrowsActionableError()
    {
        var engine = new SuiteEngine();
        engine.RegisterDspBuiltins();

        var ex = await Assert.ThrowsAsync<InvalidOperationException>(
            () => engine.EvaluateAsync("fft([0, 1, 0, 0])[1] + 1"));

        Assert.Contains("re()/im()/conj()/abs()", ex.Message);
    }
}
