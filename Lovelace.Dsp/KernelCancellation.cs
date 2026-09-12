using System.Runtime.CompilerServices;

namespace Lovelace.Dsp;

/// <summary>
/// Strided cancellation polling for the DSP kernels. The suite's numeric kernels are polled in
/// <c>Lovelace.Suite.TypedArrayOps</c>; the DSP kernels are reachable from the same run
/// (<c>Lovelace.Run</c> loads <c>DspPlugin</c>) and used to enter a multi-second transform without
/// ever reading the caller's deadline, so they carry the same poll. The helper is duplicated per
/// assembly because <c>Lovelace.Abstractions</c> is frozen this round and is the only project all
/// of them reference.
/// </summary>
/// <remarks>
/// The expensive part of a poll is the ambient-token lookup
/// (<see cref="Lovelace.Abstractions.Cancellation.Token"/> is an <c>AsyncLocal</c> read), so a
/// kernel captures the token ONCE before entering its loop and then reads the captured token.
/// <see cref="Poll"/> strides for per-sample loops; <see cref="PollNow"/> checks on every call for
/// outer loops whose body is a full inner sweep.
/// </remarks>
internal readonly struct KernelCancellation
{
    /// <summary>Iterations between checks; see the Suite helper for the same constant.</summary>
    public const int Interval = 64;

    private readonly CancellationToken _token;

    private KernelCancellation(CancellationToken token) => _token = token;

    /// <summary>Captures the ambient token: call ONCE per kernel invocation, outside the loop.</summary>
    public static KernelCancellation Capture() => new(Lovelace.Abstractions.Cancellation.Token);

    /// <summary>Checks the caller's token every <see cref="Interval"/> iterations.</summary>
    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public void Poll(long iteration)
    {
        if ((iteration & (Interval - 1)) == 0)
            Throw();
    }

    /// <summary>Checks the caller's token on every call, for a loop whose body is a full sweep.</summary>
    [MethodImpl(MethodImplOptions.AggressiveInlining)]
    public void PollNow()
    {
        if (_token.IsCancellationRequested)
            Throw();
    }

    /// <summary>The cold half: kept out of line so the poll's hot path is one predictable branch.</summary>
    [MethodImpl(MethodImplOptions.NoInlining)]
    private void Throw() => _token.ThrowIfCancellationRequested();
}
