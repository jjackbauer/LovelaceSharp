using Lovelace.Abstractions;

namespace Lovelace.Suite;

/// <summary>
/// The depth of a parsed program or expression, measured with an explicit stack.
/// <para>
/// The parser's own counter bounds how deep the DESCENT goes, but a left-associative chain
/// (<c>1-1-1-…</c>, <c>1*1*…</c>, <c>a.b.c…</c>) is built by a LOOP: the parser never descends, so
/// its counter stays flat while the tree it returns grows one level per operator. Evaluating or
/// printing that tree is a recursion of exactly that depth, which is why the tree is measured here,
/// once, before anything walks it.
/// </para>
/// <para>
/// The walk is iterative by construction. A recursive depth measure would itself overflow on the
/// input it is supposed to protect against — the same reason this guard exists.
/// </para>
/// </summary>
internal static class AstDepth
{
    /// <summary>Throws <see cref="InputDepthExceededException"/> when <paramref name="program"/>
    /// nests deeper than <see cref="InputDepth.MaxParsedTreeDepth"/>.</summary>
    internal static void EnsureWithin(string stage, Program program) => Walk(stage, program);

    /// <summary>Throws <see cref="InputDepthExceededException"/> when <paramref name="expr"/>
    /// nests deeper than <see cref="InputDepth.MaxParsedTreeDepth"/>.</summary>
    internal static void EnsureWithin(string stage, Expr expr) => Walk(stage, expr);

    private static void Walk(string stage, object root)
    {
        var stack = new Stack<(object Node, int Depth)>();
        stack.Push((root, 1));

        while (stack.Count > 0)
        {
            var (node, depth) = stack.Pop();
            if (depth > InputDepth.MaxParsedTreeDepth)
                throw new InputDepthExceededException(stage, depth, InputDepth.MaxParsedTreeDepth);

            PushChildren(stack, node, depth);
        }
    }

    /// <summary>Pushes every child of <paramref name="node"/> one level deeper. A node type that is
    /// neither a known container nor a known leaf is a hard error: silently treating it as a leaf
    /// would let a future grammar addition bypass the budget.</summary>
    private static void PushChildren(Stack<(object Node, int Depth)> stack, object node, int depth)
    {
        int next = depth + 1;
        switch (node)
        {
            // ---- program / statement containers ----
            case Program p:
                foreach (var s in p.Statements) stack.Push((s, next));
                break;
            case BlockStatement b:
                foreach (var s in b.Statements) stack.Push((s, next));
                break;
            case ExpressionStatement s:
                stack.Push((s.Expression, next));
                break;
            case IfStatement s:
                stack.Push((s.Condition, next));
                stack.Push((s.Then, next));
                if (s.Else is { } otherwise) stack.Push((otherwise, next));
                break;
            case WhileStatement s:
                stack.Push((s.Condition, next));
                stack.Push((s.Body, next));
                break;
            case ForStatement s:
                stack.Push((s.Range, next));
                stack.Push((s.Body, next));
                break;
            case ReturnStatement s:
                if (s.Value is { } returned) stack.Push((returned, next));
                break;
            case FunctionStatement s:
                foreach (var body in s.Definition.Body) stack.Push((body, next));
                break;

            // ---- expressions ----
            case BinaryExpr e:
                stack.Push((e.Left, next));
                stack.Push((e.Right, next));
                break;
            case UnaryExpr e:
                stack.Push((e.Operand, next));
                break;
            case PostfixExpr e:
                stack.Push((e.Operand, next));
                break;
            case AssignExpr e:
                stack.Push((e.Value, next));
                break;
            case CallExpr e:
                foreach (var argument in e.Arguments) stack.Push((argument, next));
                break;
            case ListExpr e:
                foreach (var element in e.Elements) stack.Push((element, next));
                break;
            case IndexExpr e:
                stack.Push((e.Target, next));
                foreach (var index in e.Indices) stack.Push((index, next));
                break;
            case MemberExpr e:
                stack.Push((e.Target, next));
                break;
            case RangeExpr e:
                stack.Push((e.Start, next));
                if (e.Step is { } step) stack.Push((step, next));
                stack.Push((e.End, next));
                break;
            case SliceExpr e:
                if (e.Start is { } sliceStart) stack.Push((sliceStart, next));
                if (e.Stop is { } sliceStop) stack.Push((sliceStop, next));
                if (e.Step is { } sliceStep) stack.Push((sliceStep, next));
                break;
            case InterpolatedStringExpr e:
                foreach (var part in e.Parts)
                    if (part is ExpressionPart embedded) stack.Push((embedded.Expression, next));
                break;

            // ---- leaves: nothing below them ----
            case LiteralExpr or VariableExpr or StringExpr or BreakStatement or ContinueStatement:
                break;

            default:
                throw new InvalidOperationException(
                    $"AstDepth: unhandled node type '{node.GetType().Name}' — a depth budget cannot be "
                    + "guaranteed for a grammar node this walk does not know.");
        }
    }
}
