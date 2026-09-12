using System.Globalization;
using Lovelace.Suite;

namespace Lovelace.Suite.Tests;

/// <summary>
/// F2-C (docs/goal-cycle-6/round-13/audit-C-hostile.md:82-127): <c>zeros(1000000000)</c> crossed as
/// <c>InternalError/InternalInvariantFailure</c> "Insufficient memory to continue the execution of the
/// program." with <c>recoverable:false</c> — an <c>OutOfMemoryException</c> reaching the runner's generic
/// handler, on a machine with 39 GB free. A request the process cannot serve is a BUDGET stop, the class the
/// depth guard already refuses with (<c>Lovelace.Abstractions/InputDepth.cs:97-127</c>,
/// <c>InputDepthExceededException</c>), so it must be typed and recoverable and it must be refused BEFORE the
/// allocation is attempted.
///
/// These tests name the refusal type through reflection on purpose: the file must still COMPILE in a tree
/// that has not implemented the guard (the control run), where it then fails at RUNTIME.
/// </summary>
public class ArrayAllocationRefusalTests
{
    private const string RefusalTypeName = "Lovelace.Suite.ArrayAllocationRefusedException";

    /// <summary>8 GiB of Value references for the audit's own shape — far above any budget a test can serve.</summary>
    private const long AuditShapeElements = 1000000000L;

    /// <summary>A delta 16x below the 8 GiB the refused request would need; parallel test classes can add
    /// noise to the process-wide counter, but not 512 MB of it.</summary>
    private const long AllocationCeiling = 512L * 1024 * 1024;

    private static async Task<Value> Eval(string source) => await new SuiteEngine().EvaluateAsync(source);

    private static Type? RefusalType() =>
        typeof(SuiteEngine).Assembly.GetType(RefusalTypeName, throwOnError: false);

    [Fact]
    public void TheSuiteDeclaresTheAllocationRefusalTheRunnerClassifies()
    {
        Assert.NotNull(RefusalType());
    }

    [Fact]
    public async Task Zeros_TenToTheNinth_IsRefusedWithoutAllocating()
    {
        long before = GC.GetTotalAllocatedBytes(precise: true);
        Exception? failure = await Record.ExceptionAsync(() => Eval($"zeros({AuditShapeElements})"));
        long allocated = GC.GetTotalAllocatedBytes(precise: true) - before;

        Assert.NotNull(failure);
        Assert.Equal(RefusalTypeName, failure!.GetType().FullName);
        Assert.True(allocated < AllocationCeiling,
            $"the refusal must precede the allocation: the evaluation allocated {allocated} bytes, " +
            $"and the refused request alone needs {AuditShapeElements * 8} bytes");
    }

    [Fact]
    public async Task Zeros_ElementProductBeyondTheBudget_IsRefusedByNameAndLimit()
    {
        Exception? failure = await Record.ExceptionAsync(() => Eval("zeros(1000000000, 1000000000)"));

        Assert.NotNull(failure);
        Assert.Equal(RefusalTypeName, failure!.GetType().FullName);

        Type type = failure.GetType();
        object requested = type.GetProperty("RequestedElements")!.GetValue(failure)!;
        object limit = type.GetProperty("Limit")!.GetValue(failure)!;
        Assert.Equal(1000000000000000000L, (long)requested);
        Assert.True((long)limit > 0);

        string message = failure.Message;
        Assert.Contains("zeros", message);
        Assert.Contains("1000000000000000000", message);
        Assert.Contains(((long)limit).ToString(CultureInfo.InvariantCulture), message);
    }

    /// <summary>
    /// Requirement 3 of the round brief: ordinary shapes (including the zero-dimension ones of D5) still
    /// build, and a large-but-servable request is not refused.
    /// </summary>
    [Fact]
    public async Task Zeros_ShapesWithinTheBudget_StillBuild()
    {
        Assert.Equal(new long[] { 2, 3 }, (await Eval("zeros(2,3)")).AsArrayValue().Shape.ToArray());
        Assert.Equal(new long[] { 2, 0 }, (await Eval("zeros(2,0)")).AsArrayValue().Shape.ToArray());
        Assert.Equal(1000000L, (await Eval("zeros(1000000)")).AsArrayValue().Numel);
    }
}
