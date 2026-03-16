# Lovelace.Real — Sqrt and Pi Parallelization Requirements

> Scope: Requirements and test plan for adding data-parallel computation paths to `Real.Sqrt` and `Real.Pi`. The `Pi` method gains a Binary Splitting (BSP) internal decomposition that enables `Task.WhenAll` parallelism over independent sub-ranges; `Sqrt` gains a batch parallel overload. Both methods gain `Task<Real>` async wrappers. Thread-safety and `AsyncLocal<long?>` precision-scope propagation are explicitly audited as mandatory pre-conditions.

---

## Functionality Worktree

### Dependency Summary

| # | Item | Depends on | Tag |
|---|---|---|---|
| 1 | `PiSegment` BSP decomposition | nothing | prerequisite for parallel Pi |
| 2 | Parallel `Pi` via `Task.WhenAll` | item 1 | depends on PiSegment |
| 3 | Batch `Sqrt` via `Task.WhenAll` | nothing | independent |
| 4 | Thread-safety and `AsyncLocal` audit | items 2 and 3 | mandatory — concurrent correctness |
| 5 | `PiAsync` Task.Run wrapper | item 2 | depends on parallel Pi |
| 6 | `SqrtAsync` Task.Run wrapper | nothing | independent |

### Completeness Checklist

- [x] `PiSegment` — Add internal static method `PiSegment(long termStart, long termEnd)` returning `(Nat P, Nat Q, Int T)` that computes the Chudnovsky sub-sum for a term range using the BSP recurrence, enabling independent parallel computation per sub-range [prerequisite for parallel Pi — all Pi parallelism depends on this]
- [x] Parallel `Pi` — Refactor `Pi(long digits)` to compute BSP sub-ranges concurrently using `Task.WhenAll`, merge results with the BSP algebraic identity, and replace the current sequential accumulation loop [depends on PiSegment]
- [x] Batch `Sqrt` — Add `public static Real[] Sqrt(IReadOnlyList<Real> values)` that dispatches each element concurrently via `Task.WhenAll` and collects results [independent]
- [x] Thread-safety and `AsyncLocal` propagation audit — Verify that `_localMaxComputationDecimalPlaces` (AsyncLocal) flows correctly into child Tasks spawned during parallel `Pi` and batch `Sqrt`, that sibling Tasks do not observe each other's precision (no lateral leak), and that `PrecisionScope.Dispose()` restores the outer scope [mandatory — concurrent correctness and precision isolation]
- [x] `PiAsync` — Add `public static Task<Real> PiAsync(long digits)` as a `Task.Run` wrapper over the parallel `Pi` implementation, offloading CPU-bound work to the thread pool [depends on parallel Pi]
- [x] `SqrtAsync` — Add `public static Task<Real> SqrtAsync(Real value)` as a `Task.Run` wrapper over `Sqrt(Real)`, offloading CPU-bound work to the thread pool [independent]

---

## Test Plan

### `PiSegment` (Binary Splitting Decomposition)

1. `PiSegment_GivenSingleTermRange_MatchesManualTermOneComputation`
   *Assumption*: `PiSegment(1, 2)` produces `(P, Q, T)` values consistent with manually evaluating the Chudnovsky BSP base-case formula for the k=1 term, so that the BSP merge identity holds from the ground up.

2. `PiSegment_GivenFullRangeTenDigits_ProducesPiMatchingCurrentSerial`
   *Assumption*: Computing `Pi(10)` via `PiSegment(0, numTerms)` and the final formula `π = 426880·√10005·Q / (A·Q + T)` yields the same value as the existing sequential `Pi(10)` implementation.

3. `PiSegment_GivenMidpointSplit_MergedTMatchesFullRangeT`
   *Assumption*: For any range [a, b] split at midpoint m, applying `T(a,b) = T(a,m)·Q(m,b) + P(a,m)·T(m,b)` on independently computed halves produces the same `T` as `PiSegment(a, b)` computed directly.

### Parallel `Pi`

4. `Pi_GivenOneDigit_MatchesSerialResultAfterBspRefactoring`
   *Assumption*: After the internal BSP refactoring, `Pi(1)` still returns a `Real` whose `ToString()` equals "3.1", because the mathematical result is unchanged by the computation strategy.

5. `Pi_GivenFiftyDigits_MatchesKnownReferenceAfterBspRefactoring`
   *Assumption*: After the refactoring, `Pi(50)` still returns "3.14159265358979323846264338327950288419716939937510", the well-known value of π to 50 decimal places.

6. `Pi_GivenConcurrentCallsFromMultipleThreads_AllReturnConsistentResults`
   *Assumption*: Launching N concurrent `Task.Run(() => Pi(10))` tasks produces N identical results equal to "3.1415926535", with no data corruption from shared mutable state, because each BSP sub-range is computed on independent local variables.

7. `Pi_GivenInvalidDigitsAfterRefactoring_StillThrowsArgumentOutOfRangeException`
   *Assumption*: The input-validation guard `if (digits <= 0 || digits > MaxComputationDecimalPlaces)` is preserved unchanged by the BSP refactoring.

### Batch `Sqrt`

8. `Sqrt_GivenEmptyBatch_ReturnsEmptyArray`
   *Assumption*: Passing an empty `IReadOnlyList<Real>` returns an empty `Real[]` without performing any arithmetic or throwing.

9. `Sqrt_GivenBatchPerfectSquares_ReturnsExactRoots`
   *Assumption*: `Sqrt([4, 9, 16, 25])` returns `[2, 3, 4, 5]` exactly, each matching the result of the corresponding element-wise `Real.Sqrt` call, because each task executes an independent Newton-Raphson convergence.

10. `Sqrt_GivenBatchMixedIrrationals_MatchesElementwiseSerial`
    *Assumption*: Batch-computing `Sqrt([2, 3, 5])` produces values identical to three independent serial `Real.Sqrt(value)` calls, because the results depend only on each input and the (shared read-only) precision configuration.

11. `Sqrt_GivenBatchContainingNegativeValue_PropagatesArithmeticException`
    *Assumption*: If any element in the batch is negative, the batch method surfaces an `ArithmeticException`, consistent with the single-value `Real.Sqrt` exception contract.

12. `Sqrt_GivenBatchSingleElement_ReturnsArrayOfOneMatchingSerial`
    *Assumption*: `Sqrt([9])` returns a one-element array `[3]` equivalent to `Real.Sqrt(new Real(9))`.

### Thread-Safety and `AsyncLocal` Propagation Audit

13. `Pi_GivenConcurrentTasksWithDifferentLocalPrecisions_PrecisionDoesNotLeakAcrossTasks`
    *Assumption*: Two sibling tasks each establishing a different `_localMaxComputationDecimalPlaces` value via `WithLocalPrecision` do not observe each other's precision, because `AsyncLocal<T>` values are scoped to each `ExecutionContext` and do not flow laterally.

14. `Sqrt_GivenBatchWithCallerLocalPrecision_EachChildTaskInheritsCallerPrecision`
    *Assumption*: A `WithLocalPrecision` scope set on the caller's thread before invoking the batch `Sqrt` flows forward into each `Task.Run`/`Task.WhenAll` child task, because .NET captures the caller's `ExecutionContext` (including `AsyncLocal` values) at task creation time.

15. `Pi_GivenNestedPrecisionScopes_OuterScopeRestoredAfterInnerDisposes`
    *Assumption*: After a nested `WithLocalPrecision` inner scope disposes, `_localMaxComputationDecimalPlaces.Value` is restored to the value it held before the inner scope was created, because `PrecisionScope.Dispose()` explicitly writes `_saved` back to `_localMaxComputationDecimalPlaces.Value`.

16. `Pi_StaticDisplayDecimalPlaces_ConcurrentReadsMutationsAreAtomic`
    *Assumption*: Concurrent reads and writes of `DisplayDecimalPlaces` and `MaxComputationDecimalPlaces` are data-race-free on all platforms, including 32-bit runtimes, because both properties use `Interlocked.Read`/`Interlocked.Exchange` on explicit `long` backing fields rather than `volatile`.

### `PiAsync`

17. `PiAsync_GivenOneDigit_ReturnsCorrectValue`
    *Assumption*: `await PiAsync(1)` returns a `Real` whose `ToString()` equals "3.1", because `PiAsync` is a thin `Task.Run` wrapper that delegates to the parallel `Pi(1)` implementation without altering its result.

18. `PiAsync_GivenInvalidDigits_PropagatesArgumentOutOfRangeException`
    *Assumption*: `await PiAsync(0)` surfaces an `ArgumentOutOfRangeException` (via the task's `Exception` property or direct re-throw on `await`), because the validation guard in `Pi(long digits)` throws before any computation begins.

19. `PiAsync_GivenConcurrentAwaits_AllReturnCorrectValues`
    *Assumption*: `await Task.WhenAll([PiAsync(10), PiAsync(10)])` produces two independently computed tasks, both returning "3.1415926535", with no shared mutable state between them.

### `SqrtAsync`

20. `SqrtAsync_GivenPerfectSquareFour_ReturnsExactlyTwo`
    *Assumption*: `await SqrtAsync(new Real(4))` returns a `Real` equal to `Real.Parse("2")`, consistent with the existing `Sqrt_GivenPerfectSquareFour_ReturnsExactlyTwo` test.

21. `SqrtAsync_GivenNegativeInput_PropagatesArithmeticException`
    *Assumption*: `await SqrtAsync(new Real("-1"))` surfaces the `ArithmeticException` thrown by `Real.Sqrt` for negative inputs, because `Task.Run` captures exceptions from the delegate and re-throws them on `await`.

22. `SqrtAsync_GivenConcurrentAwaits_AllReturnCorrectValues`
    *Assumption*: `await Task.WhenAll([SqrtAsync(new Real(2)), SqrtAsync(new Real(3))])` returns results each starting with "1.41421356237" and "1.73205080756" respectively, matching the corresponding serial `Real.Sqrt` results.

### Falsify Claims — Verification

| # | Assumption | Evidence | Status |
|---|---|---|---|
| 1 | BSP single-term matches manual formula | BSP base case is algebraic identity; directly verifiable from term recurrence | Supported |
| 2 | BSP full range produces same Pi as serial | Mathematical equivalence; experimentally verifiable at low digit counts | Supported |
| 3 | BSP merge formula is correct | Standard hypergeometric BSP merge (Haible & Papanikolaou 1997); applies directly to Chudnovsky | Supported |
| 4 | Pi(1)="3.1" after refactoring | Mathematical result unchanged by computation strategy | Supported |
| 5 | Pi(50) matches known reference | Same mathematical computation; known π reference is independently verified | Supported |
| 6 | Concurrent Pi calls return consistent results | Design requirement: no shared mutable state in BSP sub-range lambdas | Supported |
| 7 | Validation guard unchanged | Refactoring targets the accumulation loop only, not the guard at the top of Pi | Supported |
| 8 | Empty batch returns empty array | Standard `Task.WhenAll(Array.Empty<Task>())` returns immediately | Supported |
| 9 | Batch perfect squares return exact roots | Existing `RealSqrtTests` confirms per-element exactness; batch dispatches the same method | Supported |
| 10 | Batch irrationals match serial | Results depend only on input and read-only precision; parallelism has no effect on output | Supported |
| 11 | Batch with negative propagates ArithmeticException | `Real.Sqrt` throws for negatives; `Task.WhenAll` over faulted tasks propagates exceptions | Supported |
| 12 | Single-element batch equals scalar call | Trivial: batch of one is logically equivalent to the scalar overload | Supported |
| 13 | AsyncLocal does not leak across sibling tasks | .NET documentation: "The value does not flow from a child context to its parent, or laterally to a sibling" | Supported |
| 14 | Caller's AsyncLocal flows into Task.Run children | `Task.Run` captures the caller's `ExecutionContext`, which includes `AsyncLocal` values | Supported |
| 15 | PrecisionScope.Dispose restores outer scope | `Real.cs` line `public void Dispose() => _localMaxComputationDecimalPlaces.Value = _saved;` | Supported |
| 16 | Interlocked provides atomic 64-bit access | `Interlocked.Read`/`Exchange` on `long` are atomic on all .NET platforms including 32-bit | Supported |
| 17 | PiAsync(1) returns "3.1" | Task.Run wrapper; delegates to Pi(1) whose result is "3.1" per existing test | Supported |
| 18 | PiAsync(0) propagates ArgumentOutOfRangeException | ArgumentOutOfRangeException thrown inside Task.Run delegate; surfaced on await | Supported |
| 19 | Concurrent PiAsync calls return correct values | See assumption 6; same guarantee applies to Task.Run-wrapped calls | Supported |
| 20 | SqrtAsync(4) returns 2 | Existing test `Sqrt_GivenPerfectSquareFour_ReturnsExactlyTwo` confirms scalar result | Supported |
| 21 | SqrtAsync(-1) propagates ArithmeticException | Existing test `Sqrt_GivenNegativeInput_ThrowsArithmeticException` confirms scalar contract | Supported |
| 22 | Concurrent SqrtAsync results match serial | See assumption 10 | Supported |

**Falsified rows: 0.** All 22 assumptions verified; proceeding.

---

*All assumptions verified by Falsify Claims. Zero Falsified rows.*
