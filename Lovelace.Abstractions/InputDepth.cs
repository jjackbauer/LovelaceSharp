namespace Lovelace.Abstractions;

/// <summary>
/// The nesting budgets shared by every recursive walk over host input. Each walk is a native
/// recursion, and native recursion cannot be caught: once the process stack is exhausted the
/// runtime terminates the process (0xC00000FD) and NOTHING — no diagnostic, no envelope, no exit
/// code — can be reported. A budget enforced <em>before</em> the walk starts is the only way a deep
/// input can be refused instead of fatal.
/// <para>
/// There are two budgets rather than one because the walks cost very different amounts of stack per
/// level, and each budget is set from the measured death of the walk it protects (CoreCLR, Release,
/// 1 MB stack — the tighter of the two runtimes this engine ships on):
/// </para>
/// <list type="bullet">
/// <item><see cref="Max"/> (256) protects the expensive walks: the parser's descent (≈9 native
/// frames per nesting level; dies between 800 and 1024 nested levels) and every walk over a
/// canonical symbolic expression (printing dies between 800 and 900). Both therefore keep a
/// factor-3 margin.</item>
/// <item><see cref="MaxParsedTreeDepth"/> (512) protects the interpreter walking the PARSED tree,
/// whose per-level cost is the lowest of the three (dies between 1280 and 1536 for a flat chain).
/// It is separate because a flat associative chain — <c>exp(log(x+1)) + … + exp(log(x+420))</c>,
/// which the shipped suites use — is legitimate, wide input whose parsed tree is one level per
/// operator; at 256 the budget would have refused that shipped test. Anything deeper than 256 that
/// reaches canonical construction is still refused by <see cref="Max"/>.</item>
/// <item><see cref="MaxValueDepth"/> (1024) protects the walks over a run-time VALUE — the display
/// walk and the structured projection — neither of which the budgets above can see.</item>
/// </list>
/// </summary>
public static class InputDepth
{
    /// <summary>The greatest number of nested levels a parser descent or a canonical symbolic
    /// expression may reach.</summary>
    public const int Max = 256;

    /// <summary>The greatest depth the PARSED tree may have. Only the interpreter's tree walk
    /// reads it, and only a flat left-associative chain can get there without nesting.</summary>
    public const int MaxParsedTreeDepth = 512;

    /// <summary>The greatest number of levels a run-time VALUE may be deep when it is rendered.
    /// The measure counts what the renderers recurse over: a rank-R array costs R levels (the
    /// display walk descends one level per DIMENSION, not per element), a record costs one level,
    /// and a scalar is level 1. A flat 20000-element vector is therefore 2 levels; a rank-N array
    /// holding one scalar — <c>x = 1; x = [x]; x = [x]; …</c> — is N + 1.
    /// <para>
    /// This budget protects a third walk, and one that neither budget above can see. The display
    /// walk (<c>ValueFormatter.Format</c>: <c>print</c>, string interpolation, the variable capture,
    /// the result's <c>display</c>) and the wire projection
    /// (<c>StructuredProjection.ToStructured</c>: the result's and each variable's
    /// <c>structured</c> form) recurse over that shape. A value built by REPETITION is deep in
    /// NEITHER the source (every statement is flat) nor the parsed tree (every statement is
    /// shallow), and it never reaches canonical symbolic construction, so nothing above measures it.
    /// </para>
    /// <para>
    /// Measured on the shipped Native-AOT binary (Windows, Release, 1 MB stack, the tighter of the
    /// two runtimes this engine ships on): the display walk survives 10500 repetitions of
    /// <c>x = [x]</c> and dies at 11000 with 0xC00000FD and 0 bytes on stdout, and the variable
    /// capture reaches the same walk first. 1024 therefore keeps a factor-10 margin, while staying
    /// far above any depth a legitimate program builds — the shipped suites never nest a value more
    /// than a handful of levels, and a WIDE array is not deep (20000 elements are 2 levels).
    /// </para>
    /// </summary>
    public const int MaxValueDepth = 1024;
}

/// <summary>
/// A valid input (or a valid intermediate expression) whose nesting exceeds one of the
/// <see cref="InputDepth"/> budgets. This is a budget stop, not a defect: the caller can always
/// rewrite the input with less nesting, so it is reported as a recoverable
/// <see cref="ErrorCategory.BudgetExceeded"/> rather than as a domain error or an invariant
/// failure. It derives from <see cref="Exception"/> directly (not from
/// <see cref="InvalidOperationException"/>) so that a kernel which converts ordinary operation
/// failures into domain diagnostics cannot swallow a stack-safety refusal.
/// </summary>
public sealed class InputDepthExceededException : Exception
{
    /// <summary>The measured nesting depth that was refused.</summary>
    public int Depth { get; }

    /// <summary>The budget it exceeded.</summary>
    public int Limit { get; }

    public InputDepthExceededException(string stage, int depth, int limit)
        : base($"{stage} depth {depth} exceeds the maximum supported nesting depth of {limit}. "
               + "Rewrite the input with less nesting (the engine refuses it before a recursive walk "
               + "can exhaust the process stack).")
    {
        Depth = depth;
        Limit = limit;
    }
}
