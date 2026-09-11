using System.Numerics;
using System.Text.Json.Nodes;

namespace Lovelace.Run.Tests;

/// <summary>
/// <c>divrem(a, b)</c> returns the quotient and the remainder as TWO integer fields of a
/// <c>DivRemResult</c> record, never as the display sentence <c>"quotient = 3, remainder = 1"</c>
/// (A1-N-08).
///
/// <para>The field names are the ones the sentence used — <c>quotient</c> and <c>remainder</c>, the
/// names the underlying integer operations carry — so an agent reads
/// <c>result.structured.fields[quotient].value</c> without learning a convention, and the two
/// integers satisfy the division identity in the test rather than in a parser.</para>
/// </summary>
public class DivRemRecordTests
{
    /// <summary>The division cases the identity is proved over: both Naturals, a dividend larger
    /// than any fixed-width integer, a large power of ten, and the three sign combinations.</summary>
    public static TheoryData<string, string, string, string> DivisionCases() => new()
    {
        { "7", "2", "7", "2" },
        { "2^1000", "3", BigInteger.Pow(2, 1000).ToString(), "3" },
        { "10^100", "7", BigInteger.Pow(10, 100).ToString(), "7" },
        { "-17", "5", "-17", "5" },
        { "17", "-5", "17", "-5" },
        { "-17", "-5", "-17", "-5" },
    };

    private static async Task<JsonNode> DivRemAsync(string a, string b)
    {
        var (exitCode, stdout, stderr) = await TestSupport.RunScriptAsync(
            $"A = {a}; B = {b}; divrem(A, B)", "--omit-functions");
        Assert.True(exitCode == 0, $"divrem({a}, {b}) exited {exitCode}. stderr: {stderr.Trim()}");

        JsonNode envelope = TestSupport.ParseExactlyOneJsonDocument(stdout, $"divrem({a}, {b}) envelope");
        return envelope["result"]!["structured"]!;
    }

    [Theory]
    [MemberData(nameof(DivisionCases))]
    public async Task DivRem_ReturnsTwoIntegerFields_ThatSatisfyTheDivisionIdentity(
        string aSource, string bSource, string aDecimal, string bDecimal)
    {
        JsonNode structured = await DivRemAsync(aSource, bSource);

        Assert.Equal("Record", structured["kind"]!.GetValue<string>());
        Assert.Equal("DivRemResult", structured["type"]!.GetValue<string>());

        JsonNode fields = structured["fields"]!;
        Assert.Equal(new[] { "quotient", "remainder" },
            fields.AsArray().Select(f => f!["name"]!.GetValue<string>()).ToArray());

        JsonObject byName = TestSupport.FieldsByName(structured);
        JsonNode quotient = byName["quotient"]!;
        JsonNode remainder = byName["remainder"]!;
        Assert.Contains(quotient["kind"]!.GetValue<string>(), new[] { "Natural", "Integer" });
        Assert.Contains(remainder["kind"]!.GetValue<string>(), new[] { "Natural", "Integer" });

        BigInteger a = BigInteger.Parse(aDecimal);
        BigInteger b = BigInteger.Parse(bDecimal);
        BigInteger q = BigInteger.Parse(quotient["value"]!.GetValue<string>());
        BigInteger r = BigInteger.Parse(remainder["value"]!.GetValue<string>());

        // The two published integers ARE a division — in the convention the engine documents
        // (Integer.DivRem, Integer.cs:293-305): the MAGNITUDES are divided and the equal-signs
        // rule is applied to both results. So the invariant is |a| = |q|·|b| + |r| with
        // |r| < |b| and sign(q) = sign(r) = sign(a)·sign(b) — not the schoolbook a = q·b + r,
        // which that sign rule satisfies only when the operands' signs agree.
        Assert.Equal(BigInteger.Abs(a), BigInteger.Abs(q) * BigInteger.Abs(b) + BigInteger.Abs(r));
        Assert.True(BigInteger.Abs(r) < BigInteger.Abs(b),
            $"|remainder| = {BigInteger.Abs(r)} is not below |divisor| = {BigInteger.Abs(b)}");
        if (q != BigInteger.Zero)
            Assert.Equal(a.Sign * b.Sign, q.Sign);
        if (q != BigInteger.Zero && r != BigInteger.Zero)
            Assert.Equal(q.Sign, r.Sign);
    }

    /// <summary>The reported case, stated directly: the two integers are fields, and the result is
    /// not a Text sentence an agent would have to parse.</summary>
    [Fact]
    public async Task DivRemOfTwoNaturals_CrossesAsStructureNotProse()
    {
        JsonNode structured = await DivRemAsync("7", "2");

        Assert.NotEqual("Text", structured["kind"]!.GetValue<string>());
        JsonObject byName = TestSupport.FieldsByName(structured);
        Assert.Equal("3", byName["quotient"]!["value"]!.GetValue<string>());
        Assert.Equal("1", byName["remainder"]!["value"]!.GetValue<string>());
        Assert.Equal("Natural", byName["quotient"]!["kind"]!.GetValue<string>());
    }

    /// <summary>A widened pair is Integer on both sides, even when the dividend alone needed the
    /// wider kind: the two fields describe ONE division, so they share the kinds it was done in.</summary>
    [Fact]
    public async Task DivRemWithANegativeOperand_IsIntegerOnBothFields()
    {
        JsonNode structured = await DivRemAsync("-17", "5");

        JsonObject byName = TestSupport.FieldsByName(structured);
        Assert.Equal("Integer", byName["quotient"]!["kind"]!.GetValue<string>());
        Assert.Equal("Integer", byName["remainder"]!["kind"]!.GetValue<string>());
        Assert.Equal("-3", byName["quotient"]!["value"]!.GetValue<string>());
        Assert.Equal("-2", byName["remainder"]!["value"]!.GetValue<string>());
    }

    /// <summary>The sign combinations the prose used to spell out — the record carries the SAME
    /// two integers the sentence did, so a consumer switching on the fields never sees a different
    /// answer: (17, −5) and (−17, 5) put both results negative, (−17, −5) puts both positive.</summary>
    [Theory]
    [InlineData("17", "-5", "-3", "-2")]
    [InlineData("-17", "5", "-3", "-2")]
    [InlineData("-17", "-5", "3", "2")]
    [InlineData("1", "5", "0", "1")]
    public async Task DivRem_KeepsTheEnginesDocumentedSignRule(string a, string b, string quotient, string remainder)
    {
        JsonObject byName = TestSupport.FieldsByName(await DivRemAsync(a, b));

        Assert.Equal(quotient, byName["quotient"]!["value"]!.GetValue<string>());
        Assert.Equal(remainder, byName["remainder"]!["value"]!.GetValue<string>());
    }

    /// <summary>Division by zero is still refused as a typed, recoverable error — the record shape
    /// did not turn a domain error into a record.</summary>
    [Fact]
    public async Task DivRemByZero_IsStillARecoverableDomainError()
    {
        var (exitCode, stdout, _) = await TestSupport.RunScriptAsync("divrem(7, 0)", "--omit-functions");

        Assert.Equal(1, exitCode);
        JsonNode envelope = TestSupport.ParseExactlyOneJsonDocument(stdout, "divrem(7, 0) envelope");
        Assert.False(envelope["ok"]!.GetValue<bool>());
        Assert.Equal("DivisionByZero", envelope["code"]!.GetValue<string>());
        Assert.Equal("DomainError", envelope["category"]!.GetValue<string>());
        Assert.True(envelope["recoverable"]!.GetValue<bool>());
    }
}
