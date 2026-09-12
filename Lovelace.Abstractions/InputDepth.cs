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

    /// <summary>The greatest depth the interpreter's own EVALUATION walk may reach. It bounds the
    /// recursion the budgets above cannot see: a user function that calls itself is deep in neither
    /// the source (every statement of the body is flat) nor the parsed tree (the body is a handful of
    /// levels) and returns a scalar — the depth that grows is the INTERPRETER's own native recursion,
    /// which is uncatchable when it exhausts the process stack.
    /// <para>
    /// The unit is one step of that walk (enforced in <c>Interpreter.EnterEvaluation</c>): one unit
    /// per expression node that descends (a leaf descends nowhere), one per statement executed (a
    /// BlockStatement is grouping and is covered by the statements inside it), and four per
    /// user-function call level (its frame, its scope, and the entry into its body). A script's own
    /// outermost call — its entry into user code — starts each top-level statement with a six-unit
    /// credit, so the depth that counts is the depth the script's function bodies add below their
    /// entry.
    /// </para>
    /// <para>
    /// The audit's shape — <c>func f(n) { if (n == 0) { return 0 }; return f(n - 1) }</c> plus
    /// <c>f(N)</c> — therefore costs six units per call level. Measured on the shipped runner
    /// (Windows, Release, 1 MB stack): the deepest accepted call is <c>f(85)</c> at exactly 512
    /// units and <c>f(86)</c> is refused as <c>DepthExceeded/BudgetExceeded</c>; before this budget
    /// the same script answered through <c>f(432)</c> and terminated the process at <c>f(436)</c>
    /// with 0xC00000FD, zero bytes on stdout and ~2 MB of "Stack overflow." on stderr.
    /// </para>
    /// <para>
    /// 512 units is a factor-5 margin below that death (f(436) is roughly 2 600 native frames). It is
    /// one number over the whole walk rather than a call-level cap because a deep expression inside a
    /// recursive body is walked at EVERY level: 120 nested <c>if</c>s in the body killed the process
    /// at call depth 5 (measured) and are now refused, while the same 120 nested <c>if</c>s at the top
    /// level still answer.
    /// </para>
    /// </summary>
    public const int MaxEvaluationDepth = 512;

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

    /// <param name="stage">What was being walked (e.g. "expression", "evaluation").</param>
    /// <param name="depth">The measured depth that was refused.</param>
    /// <param name="limit">The budget it exceeded.</param>
    /// <param name="measure">What the limit measures: "nesting" for the shape budgets, or
    /// "evaluation" for <see cref="InputDepth.MaxEvaluationDepth"/>. Only the word changes; the
    /// message shape is the same for every budget.</param>
    public InputDepthExceededException(string stage, int depth, int limit, string measure = "nesting")
        : base($"{stage} depth {depth} exceeds the maximum supported {measure} depth of {limit}. "
               + "Rewrite the input with less nesting (the engine refuses it before a recursive walk "
               + "can exhaust the process stack).")
    {
        Depth = depth;
        Limit = limit;
    }
}
