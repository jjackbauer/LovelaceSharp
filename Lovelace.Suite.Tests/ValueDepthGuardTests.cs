using Xunit;

namespace Lovelace.Suite.Tests;

/// <summary>
/// The run-time VALUE budget, at the two walks it protects. A value built one level at a time —
/// <c>x = 1; x = [x]; x = [x]; …</c> — is deep in neither the source (every statement is flat) nor
/// the parsed tree (every statement is shallow), so neither input budget sees it; the display walk
/// and the structured projection each recurse once per level and used to exhaust the process stack
/// (0xC00000FD, no envelope, no exit code).
/// <para>
/// The refusal is asserted by type NAME and message text, and the budget as a literal, rather than
/// by referencing <c>InputDepth.MaxValueDepth</c> or <c>ValueDepth</c> — the whole file therefore
/// also compiles against the pre-fix tree, which is what makes the control-tree evidence (this test
/// host dying with 0xC00000FD) reproducible with it.
/// </para>
/// </summary>
public class ValueDepthGuardTests
{
    /// <summary>The value-nesting budget (Lovelace.Abstractions.InputDepth.MaxValueDepth).</summary>
    private const int MaxValueDepth = 1024;

    /// <summary>Builds <paramref name="wraps"/> nested rank-1 arrays around the integer 1: the value
    /// the script <c>x = 1; x = [x]; …</c> builds. The scalar leaf is level 1, so the value's depth
    /// is <c>wraps + 1</c>.</summary>
    private static Value Nested(int wraps)
    {
        Value value = new Value(1L);
        for (int i = 0; i < wraps; i++)
            value = new Value(new Value[] { value });
        return value;
    }

    /// <summary>The same nesting, one level per RECORD instead of per array.</summary>
    private static Value NestedRecords(int levels)
    {
        Value value = new Value(1L);
        for (int i = 0; i < levels; i++)
            value = new Value(new Lovelace.Abstractions.RecordValue("Level",
                new Lovelace.Abstractions.RecordField("inner", value)));
        return value;
    }

    /// <summary>The display of <paramref name="wraps"/> nested single-element arrays:
    /// <c>[[…[1]…]]</c> with one bracket pair per wrap (the scalar itself adds none).</summary>
    private static string ExpectedDisplay(int wraps) =>
        new string('[', wraps) + "1" + new string(']', wraps);

    /// <summary>The audit's shape, built by the ENGINE rather than by hand:
    /// <c>x = 1; x = [x]; …</c> does not nest elements, it raises the array's RANK (a rank-N array
    /// holding one scalar), so this is what the display walk actually recurses down.</summary>
    private static Task<Value> RankByRepetition(int wraps) =>
        new SuiteEngine().EvaluateAsync("x = 1" + string.Concat(Enumerable.Repeat("; x = [x]", wraps)) + "; x");

    /// <summary>A refusal: the typed budget stop, naming the budget it broke and the depth it
    /// measured. The measured depth is whatever the value really is, so it is only asserted
    /// exactly at the boundary.</summary>
    private static void AssertRefusal(Exception? ex)
    {
        Assert.NotNull(ex);
        Assert.Equal("InputDepthExceededException", ex!.GetType().Name);
        Assert.StartsWith("value depth ", ex.Message);
        Assert.Contains($"exceeds the maximum supported nesting depth of {MaxValueDepth}", ex.Message);
    }

    /// <summary>A refusal measured at exactly one level past the budget.</summary>
    private static void AssertRefusalAtTheBoundary(Exception? ex)
    {
        Assert.NotNull(ex);
        Assert.Equal("InputDepthExceededException", ex!.GetType().Name);
        Assert.Contains(
            $"value depth {MaxValueDepth + 1} exceeds the maximum supported nesting depth of {MaxValueDepth}",
            ex.Message);
    }

    // ------------------------------------------------------------------
    // The boundary: limit-1 accepted, limit+1 refused
    // ------------------------------------------------------------------

    [Fact]
    public void ValueAtTheBudget_IsStillRendered()
    {
        Value value = Nested(MaxValueDepth - 1);

        Assert.Equal(ExpectedDisplay(MaxValueDepth - 1), ValueFormatter.Format(value));
        Assert.Equal("Array", StructuredProjection.ToStructured(value).Kind);
    }

    /// <summary>
    /// The audit's exact shape, built by the ENGINE rather than by hand: <c>x = 1; x = [x]; …</c>
    /// raises the array's RANK (a rank-N array holding ONE scalar) instead of nesting elements, and
    /// the display walk descends one level per DIMENSION. An element-counting measure calls this two
    /// levels deep and lets the renderer below it exhaust the stack, which is the defect this test
    /// pins.
    /// </summary>
    [Fact]
    public async Task ARankRaisedByRepetition_IsRefusedByTheDisplayWalk()
    {
        Value atBudget = await RankByRepetition(MaxValueDepth - 1);
        Assert.Equal(MaxValueDepth - 1, atBudget.AsArrayValue().Rank);
        Assert.Equal(ExpectedDisplay(MaxValueDepth - 1), ValueFormatter.Format(atBudget));

        Value past = await RankByRepetition(MaxValueDepth);
        Assert.Equal(MaxValueDepth, past.AsArrayValue().Rank);
        AssertRefusalAtTheBoundary(Record.Exception(() => ValueFormatter.Format(past)));
        AssertRefusalAtTheBoundary(Record.Exception(() => StructuredProjection.ToStructured(past)));
    }

    /// <summary>20000 repetitions — the audit's script — refuses instead of dying, which is only
    /// possible because the measure is iterative.</summary>
    [Fact]
    public async Task ARankFarPastTheBudget_IsRefusedRatherThanExhaustingTheStack()
    {
        Value past = await RankByRepetition(20000);

        AssertRefusal(Record.Exception(() => ValueFormatter.Format(past)));
    }

    [Fact]
    public void ValueOnePastTheBudget_IsRefusedByTheDisplayWalk()
    {
        AssertRefusalAtTheBoundary(Record.Exception(() => ValueFormatter.Format(Nested(MaxValueDepth))));
    }

    [Fact]
    public void ValueOnePastTheBudget_IsRefusedByTheStructuredProjection()
    {
        AssertRefusalAtTheBoundary(Record.Exception(() => StructuredProjection.ToStructured(Nested(MaxValueDepth))));
    }

    // ------------------------------------------------------------------
    // The measure must not itself recurse, and must stop at the budget
    // ------------------------------------------------------------------

    /// <summary>
    /// 20000 levels: the exact value the audit's 20000-statement script builds, and far past the
    /// measured death of both walks (10500). Both entry points must refuse it, and the refusal must
    /// be raised by the ITERATIVE measure — a recursive one would die here instead.
    /// </summary>
    [Fact]
    public void AValueFarPastTheBudget_IsRefusedRatherThanExhaustingTheStack()
    {
        Value value = Nested(20000);

        AssertRefusal(Record.Exception(() => ValueFormatter.Format(value)));
        AssertRefusal(Record.Exception(() => StructuredProjection.ToStructured(value)));
        AssertRefusal(Record.Exception(() => ValueFormatter.FormatTyped(value)));
    }

    [Fact]
    public void ADeepRecordChain_IsRefusedTheSameWay()
    {
        Assert.NotNull(ValueFormatter.Format(NestedRecords(MaxValueDepth - 1)));

        AssertRefusalAtTheBoundary(Record.Exception(() => ValueFormatter.Format(NestedRecords(MaxValueDepth))));
        AssertRefusal(Record.Exception(() => StructuredProjection.ToStructured(NestedRecords(20000))));
    }

    /// <summary>Wide is not deep: a 20000-element flat vector is two levels and must render.</summary>
    [Fact]
    public void AWideValue_IsNotRefused()
    {
        var elements = new Value[20000];
        for (int i = 0; i < elements.Length; i++)
            elements[i] = new Value((long)i);

        string text = ValueFormatter.Format(new Value(elements));

        Assert.StartsWith("[0, 1, 2", text);
        Assert.EndsWith("19999]", text);
    }
}
