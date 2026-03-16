using System.Collections.Concurrent;
using System.Threading;
using Lovelace.Real;

namespace Lovelace.Real.Tests;

/// <summary>
/// Thread-safety and AsyncLocal propagation audit tests.
/// Checklist item: Thread-safety and AsyncLocal propagation audit — Verify that
/// _localMaxComputationDecimalPlaces (AsyncLocal) flows correctly into child Tasks,
/// that sibling Tasks do not observe each other's precision (no lateral leak), and
/// that PrecisionScope.Dispose() restores the outer scope.
/// </summary>
public class RealAsyncLocalTests
{
    // -------------------------------------------------------------------------
    // Test 13 — no lateral leakage between sibling tasks
    // -------------------------------------------------------------------------

    [Fact]
    public void Pi_GivenConcurrentTasksWithDifferentLocalPrecisions_PrecisionDoesNotLeakAcrossTasks()
    {
        // Two sibling tasks each establish a distinct local precision value;
        // after synchronising on a barrier (so both are active simultaneously),
        // each task must observe only its own value — not the sibling's.
        const long precisionA = 55L;
        const long precisionB = 77L;
        long observedA = -1L;
        long observedB = -1L;

        // barrier(2): lets both tasks reach the read point concurrently.
        var barrier = new Barrier(2);

        var taskA = Task.Run(() =>
        {
            using (Real.WithLocalPrecision(precisionA))
            {
                barrier.SignalAndWait();          // synchronise with task B
                observedA = Real.MaxComputationDecimalPlaces;
            }
        });

        var taskB = Task.Run(() =>
        {
            using (Real.WithLocalPrecision(precisionB))
            {
                barrier.SignalAndWait();          // synchronise with task A
                observedB = Real.MaxComputationDecimalPlaces;
            }
        });

        Task.WhenAll(taskA, taskB).GetAwaiter().GetResult();

        // AsyncLocal values are per-ExecutionContext; lateral leakage cannot occur.
        Assert.Equal(precisionA, observedA);
        Assert.Equal(precisionB, observedB);
    }

    // -------------------------------------------------------------------------
    // Test 14 — caller's local precision flows into Task.Run children
    // -------------------------------------------------------------------------

    [Fact]
    public void Sqrt_GivenBatchWithCallerLocalPrecision_EachChildTaskInheritsCallerPrecision()
    {
        // .NET captures the caller's ExecutionContext (including AsyncLocal values)
        // at Task.Run() call time.  Tasks spawned inside a WithLocalPrecision scope
        // must therefore inherit that scope's precision.
        const long callerPrecision = 42L;

        Task<long> childA;
        Task<long> childB;

        using (Real.WithLocalPrecision(callerPrecision))
        {
            // Both tasks are spawned inside the scope — they capture the local precision.
            childA = Task.Run(() => Real.MaxComputationDecimalPlaces);
            childB = Task.Run(() => Real.MaxComputationDecimalPlaces);
        }

        long[] results = Task.WhenAll(childA, childB).GetAwaiter().GetResult();

        Assert.Equal(callerPrecision, results[0]);
        Assert.Equal(callerPrecision, results[1]);
    }

    // -------------------------------------------------------------------------
    // Test 15 — nested precision scopes restore correctly on Dispose
    // -------------------------------------------------------------------------

    [Fact]
    public void Pi_GivenNestedPrecisionScopes_OuterScopeRestoredAfterInnerDisposes()
    {
        // PrecisionScope.Dispose() writes _saved back to _localMaxComputationDecimalPlaces.Value.
        // Nesting two scopes must restore each level in LIFO order.
        // Capture the global default before entering any scope (no local active → getter returns global).
        long globalDefault = Real.MaxComputationDecimalPlaces;
        const long outerPrecision = 30L;
        const long innerPrecision = 75L;

        using (Real.WithLocalPrecision(outerPrecision))
        {
            Assert.Equal(outerPrecision, Real.MaxComputationDecimalPlaces);

            using (Real.WithLocalPrecision(innerPrecision))
            {
                Assert.Equal(innerPrecision, Real.MaxComputationDecimalPlaces);
            }

            // Inner scope disposed — outer value must be restored.
            Assert.Equal(outerPrecision, Real.MaxComputationDecimalPlaces);
        }

        // Outer scope disposed — null local → getter falls back to global backing field.
        Assert.Equal(globalDefault, Real.MaxComputationDecimalPlaces);
    }

    // -------------------------------------------------------------------------
    // Test 16 — Interlocked provides atomic 64-bit access to static properties
    // -------------------------------------------------------------------------

    [Fact]
    public void Pi_StaticDisplayDecimalPlaces_ConcurrentReadsMutationsAreAtomic()
    {
        // Concurrent reads and writes via Interlocked.Read/Exchange must
        // never produce a torn (half-written) value.  Every observed value
        // must be exactly one of the three values that the tasks write.
        long[] validValues = [50L, 100L, 150L];
        var invalidObservations = new ConcurrentBag<long>();
        long originalDisplay = Real.DisplayDecimalPlaces;

        try
        {
            var tasks = Enumerable.Range(0, 60).Select(i => Task.Run(() =>
            {
                long toWrite = validValues[i % 3];
                Real.DisplayDecimalPlaces = toWrite;
                long read = Real.DisplayDecimalPlaces;
                if (read != 50L && read != 100L && read != 150L)
                    invalidObservations.Add(read);
            })).ToArray();

            Task.WaitAll(tasks);
        }
        finally
        {
            Real.DisplayDecimalPlaces = originalDisplay;
        }

        // No torn reads: every observed value must be one the tasks actually wrote.
        Assert.Empty(invalidObservations);
    }
}
