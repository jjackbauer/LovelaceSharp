using Lovelace.Abstractions;

namespace Lovelace.Suite;

/// <summary>
/// The depth of a run-time <see cref="Value"/>, measured with an explicit stack before any recursive
/// walk over it starts. The measure counts what the renderers recurse over: a rank-R array costs R
/// levels (the display walk descends one level per dimension), a record costs one, and a scalar is
/// level 1 — so <c>x = 1; x = [x] × N</c>, which builds a rank-N array holding one scalar, measures
/// N + 1.
/// <para>
/// The budgets in <see cref="InputDepth"/> bound the INPUT: the source's nesting, and the parsed
/// tree the interpreter walks. A value's nesting is a third thing, and a program can raise it one
/// level at a time without ever writing a deep expression or a deep statement —
/// <c>x = 1; x = [x]; x = [x]; …</c> is a flat script whose VALUE is N levels deep. Every render of
/// that value (the display string of each variable, the result's <c>display</c>, the structured
/// projection, <c>print</c>, interpolation) is a native recursion of exactly that depth, and native
/// recursion cannot be caught: once the process stack is exhausted the runtime terminates the process
/// with 0xC00000FD and no envelope, no exit code and nothing on stdout can be reported.
/// </para>
/// <para>
/// The walk is iterative for the same reason <see cref="AstDepth"/> is: a recursive measure would
/// itself overflow on the value it is supposed to protect against. It also stops at the first node
/// past the budget, so refusing a deep value costs one descent to the limit rather than a traversal
/// of the whole value.
/// </para>
/// </summary>
internal static class ValueDepth
{
    /// <summary>Throws <see cref="InputDepthExceededException"/> when <paramref name="value"/> nests
    /// deeper than <see cref="InputDepth.MaxValueDepth"/>.</summary>
    internal static void EnsureWithin(string stage, Value value) => Walk(stage, value);

    /// <summary>Throws <see cref="InputDepthExceededException"/> when <paramref name="array"/> nests
    /// deeper than <see cref="InputDepth.MaxValueDepth"/>.</summary>
    internal static void EnsureWithin(string stage, ArrayValue array) => Walk(stage, array);

    /// <summary>Throws <see cref="InputDepthExceededException"/> when <paramref name="record"/> nests
    /// deeper than <see cref="InputDepth.MaxValueDepth"/>.</summary>
    internal static void EnsureWithin(string stage, RecordValue record) => Walk(stage, record);

    private static void Walk(string stage, object root)
    {
        var stack = new Stack<(object Node, int Depth)>();
        stack.Push((root, 1));

        while (stack.Count > 0)
        {
            var (node, depth) = stack.Pop();
            if (depth > InputDepth.MaxValueDepth)
                throw new InputDepthExceededException(stage, depth, InputDepth.MaxValueDepth);

            PushChildren(stack, node, depth);
        }
    }

    /// <summary>
    /// Pushes every child of <paramref name="node"/> at the level a RENDERER would recurse to, or
    /// throws when the node is a container this walk does not know (an unbudgeted recursion).
    /// <para>
    /// The level is not simply "one per container":
    /// </para>
    /// <list type="bullet">
    /// <item>A <see cref="Value"/> of kind Array or Record costs one level (its
    /// <see cref="ArrayValue"/>/<see cref="RecordValue"/> payload is the SAME level, exactly as
    /// <c>Format</c> does not recurse when it unwraps it), and each record field is one level
    /// deeper.</item>
    /// <item>An ARRAY's elements are <c>Rank</c> levels deeper, not one: the display walk descends
    /// one level per DIMENSION (<c>FormatLevel</c> recurses along the shape, not along the element
    /// list), so a <c>rank-R</c> array is R native frames deep even when it holds a single scalar.
    /// This is exactly how a value built by repetition grows — <c>x = 1; x = [x]; x = [x]; …</c>
    /// makes a rank-N array with ONE element, so an element-counting measure would call it two
    /// levels and let the renderer below it overflow the stack (measured: the AOT binary dies at
    /// rank ≈ 10700).</item>
    /// </list>
    /// </summary>
    private static void PushChildren(Stack<(object Node, int Depth)> stack, object node, int depth)
    {
        switch (node)
        {
            case Value value:
                switch (value.Kind)
                {
                    case ValueKind.Vector or ValueKind.Array:
                        stack.Push((value.AsArrayValue(), depth));
                        break;
                    case ValueKind.Record:
                        stack.Push((value.AsRecord(), depth));
                        break;
                }
                break;

            case ArrayValue array:
            {
                int next = depth + array.Rank;
                for (long i = 0; i < array.Numel; i++)
                    stack.Push((array.GetElement(i), next));
                break;
            }

            case RecordValue record:
            {
                int next = depth + 1;
                foreach (var field in record.Fields)
                    if (field.Value is not null)
                        stack.Push((field.Value, next));
                break;
            }

            default:
                throw new InvalidOperationException(
                    $"ValueDepth: unhandled node type '{node.GetType().Name}' — a depth budget cannot be "
                    + "guaranteed for a value container this walk does not know.");
        }
    }
}
