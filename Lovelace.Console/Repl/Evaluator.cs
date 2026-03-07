using Nat = global::Lovelace.Natural.Natural;
using Int = global::Lovelace.Integer.Integer;
using Rl = global::Lovelace.Real.Real;

namespace Lovelace.Console.Repl;

/// <summary>
/// Walks an <see cref="Expr"/> AST and produces a <see cref="Value"/>.
/// Maintains a variable store and a registry of built-in functions.
/// </summary>
public sealed class Evaluator
{
    // -----------------------------------------------------------------
    // Variable store
    // -----------------------------------------------------------------

    private readonly Dictionary<string, Value> _variables = new();

    // -----------------------------------------------------------------
    // Public API
    // -----------------------------------------------------------------

    /// <summary>
    /// A read-only view of the current variable store.
    /// </summary>
    public IReadOnlyDictionary<string, Value> Variables => _variables;

    /// <summary>
    /// Removes all variables from the store.
    /// </summary>
    public void Clear() => _variables.Clear();

    /// <summary>
    /// Removes the variable with the given <paramref name="name"/> from the store.
    /// Returns <see langword="true"/> if the variable existed, <see langword="false"/> otherwise.
    /// </summary>
    public bool Remove(string name) => _variables.Remove(name);

    /// <summary>
    /// Evaluates <paramref name="expr"/> and returns the resulting <see cref="Value"/>.
    /// </summary>
    public Value Evaluate(Expr expr) => expr switch
    {
        LiteralExpr lit => EvaluateLiteral(lit),
        VariableExpr var => EvaluateVariable(var),
        AssignExpr assign => EvaluateAssign(assign),
        BinaryExpr bin => EvaluateBinary(bin),
        UnaryExpr unary => EvaluateUnary(unary),
        PostfixExpr postfix => EvaluatePostfix(postfix),
        CallExpr call => EvaluateCall(call),
        _ => throw new NotImplementedException($"Unsupported expression type: {expr.GetType().Name}"),
    };

    // -----------------------------------------------------------------
    // Literal evaluation — type inference by content
    // -----------------------------------------------------------------

    /// <summary>
    /// Infers the narrowest type from raw text:
    /// <list type="bullet">
    ///   <item>Contains <c>'.'</c> or <c>'('</c> → <see cref="Rl"/> via <c>Real.Parse</c></item>
    ///   <item>Otherwise → <see cref="Nat"/> via <c>Natural.Parse</c></item>
    /// </list>
    /// </summary>
    private static Value EvaluateLiteral(LiteralExpr lit)
    {
        var text = lit.RawText;

        if (text.Contains('.') || text.Contains('('))
            return new Value(Rl.Parse(text, null));

        return new Value(Nat.Parse(text, null));
    }

    // -----------------------------------------------------------------
    // Variable store
    // -----------------------------------------------------------------

    private Value EvaluateVariable(VariableExpr var)
    {
        if (_variables.TryGetValue(var.Name, out var value))
            return value;

        throw new InvalidOperationException($"Undefined variable '{var.Name}'.");
    }

    private Value EvaluateAssign(AssignExpr assign)
    {
        var value = Evaluate(assign.Value);
        _variables[assign.Name] = value;
        return value;
    }

    // -----------------------------------------------------------------
    // Binary arithmetic
    // -----------------------------------------------------------------

    private Value EvaluateBinary(BinaryExpr bin)
    {
        var left = Evaluate(bin.Left);
        var right = Evaluate(bin.Right);

        // Comparison operators: widen pair, CompareTo, return Boolean.
        if (bin.Op is BinaryOp.Equal or BinaryOp.NotEqual
                   or BinaryOp.Greater or BinaryOp.Less
                   or BinaryOp.GreaterEqual or BinaryOp.LessEqual)
            return EvaluateComparison(left, right, bin.Op);

        // Widen both operands to the wider of the two kinds, then dispatch.
        (left, right) = Value.WidenPair(left, right);

        return (bin.Op, left.Kind) switch
        {
            // ---- Natural arithmetic ----
            (BinaryOp.Add,      ValueKind.Natural) => new Value(left.AsNatural() + right.AsNatural()),
            (BinaryOp.Subtract, ValueKind.Natural) => SubtractNatural(left, right),
            (BinaryOp.Multiply, ValueKind.Natural) => new Value(left.AsNatural() * right.AsNatural()),
            (BinaryOp.Divide,   ValueKind.Natural) => new Value(left.AsNatural() / right.AsNatural()),
            (BinaryOp.Modulo,   ValueKind.Natural) => new Value(left.AsNatural() % right.AsNatural()),
            (BinaryOp.Power,    ValueKind.Natural) => new Value(left.AsNatural().Pow(right.AsNatural())),

            // ---- Integer arithmetic ----
            (BinaryOp.Add,      ValueKind.Integer) => new Value(left.AsInteger() + right.AsInteger()),
            (BinaryOp.Subtract, ValueKind.Integer) => new Value(left.AsInteger() - right.AsInteger()),
            (BinaryOp.Multiply, ValueKind.Integer) => new Value(left.AsInteger() * right.AsInteger()),
            (BinaryOp.Divide,   ValueKind.Integer) => new Value(left.AsInteger() / right.AsInteger()),
            (BinaryOp.Modulo,   ValueKind.Integer) => new Value(left.AsInteger() % right.AsInteger()),
            (BinaryOp.Power,    ValueKind.Integer) => new Value(left.AsInteger().Pow(right.AsInteger())),

            // ---- Real arithmetic ----
            (BinaryOp.Add,      ValueKind.Real) => new Value(left.AsReal() + right.AsReal()),
            (BinaryOp.Subtract, ValueKind.Real) => new Value(left.AsReal() - right.AsReal()),
            (BinaryOp.Multiply, ValueKind.Real) => new Value(left.AsReal() * right.AsReal()),
            (BinaryOp.Divide,   ValueKind.Real) => new Value(left.AsReal() / right.AsReal()),
            (BinaryOp.Modulo,   ValueKind.Real) => new Value(left.AsReal() % right.AsReal()),
            (BinaryOp.Power,    ValueKind.Real) => new Value(left.AsReal().Pow(right.AsReal())),

            _ => throw new InvalidOperationException(
                $"Operator '{bin.Op}' is not supported for type '{left.Kind}'."),
        };
    }

    // -----------------------------------------------------------------
    // Comparison operators → Boolean Value
    // -----------------------------------------------------------------

    /// <summary>
    /// Widens both operands to the wider of the two kinds, calls <c>CompareTo</c>
    /// on the inner numeric values, then maps the result to a <see cref="bool"/>.
    /// </summary>
    private static Value EvaluateComparison(Value left, Value right, BinaryOp op)
    {
        (left, right) = Value.WidenPair(left, right);

        int cmp = left.Kind switch
        {
            ValueKind.Natural => left.AsNatural().CompareTo(right.AsNatural()),
            ValueKind.Integer => left.AsInteger().CompareTo(right.AsInteger()),
            ValueKind.Real    => left.AsReal().CompareTo(right.AsReal()),
            _ => throw new InvalidOperationException(
                    $"Cannot compare values of kind '{left.Kind}'."),
        };

        bool result = op switch
        {
            BinaryOp.Equal        => cmp == 0,
            BinaryOp.NotEqual     => cmp != 0,
            BinaryOp.Greater      => cmp > 0,
            BinaryOp.Less         => cmp < 0,
            BinaryOp.GreaterEqual => cmp >= 0,
            BinaryOp.LessEqual    => cmp <= 0,
            _ => throw new InvalidOperationException(
                    $"Unknown comparison operator '{op}'."),
        };

        return new Value(result);
    }

    // -----------------------------------------------------------------
    // Natural subtraction with underflow → Integer fallback
    // -----------------------------------------------------------------

    /// <summary>
    /// Attempts Natural subtraction. If <see cref="InvalidOperationException"/> is thrown
    /// (i.e. the result would be negative), widens both operands to <see cref="Int"/> and retries.
    /// </summary>
    private static Value SubtractNatural(Value left, Value right)
    {
        try
        {
            return new Value(left.AsNatural() - right.AsNatural());
        }
        catch (InvalidOperationException)
        {
            var leftInt  = left.Widen(ValueKind.Integer);
            var rightInt = right.Widen(ValueKind.Integer);
            return new Value(leftInt.AsInteger() - rightInt.AsInteger());
        }
    }

    // -----------------------------------------------------------------
    // Unary operators
    // -----------------------------------------------------------------

    /// <summary>
    /// Evaluates a unary expression:
    /// <list type="bullet">
    ///   <item><c>+operand</c> — returns the operand unchanged.</item>
    ///   <item><c>-Natural</c> — widens to <see cref="Int"/> then negates.</item>
    ///   <item><c>-Integer</c> — negates directly.</item>
    ///   <item><c>-Real</c>    — negates directly.</item>
    /// </list>
    /// </summary>
    private Value EvaluateUnary(UnaryExpr unary)
    {
        var operand = Evaluate(unary.Operand);

        return unary.Op switch
        {
            UnaryOp.Plus => operand,

            UnaryOp.Negate => operand.Kind switch
            {
                // Natural does not support negation — widen to Integer first.
                ValueKind.Natural => new Value(-operand.Widen(ValueKind.Integer).AsInteger()),
                ValueKind.Integer => new Value(-operand.AsInteger()),
                ValueKind.Real    => new Value(-operand.AsReal()),
                _ => throw new InvalidOperationException(
                    $"Unary negation is not supported for type '{operand.Kind}'."),
            },

            _ => throw new InvalidOperationException(
                $"Unary operator '{unary.Op}' is not supported."),
        };
    }

    // -----------------------------------------------------------------
    // Postfix operators
    // -----------------------------------------------------------------

    /// <summary>
    /// Evaluates a postfix expression:
    /// <list type="bullet">
    ///   <item><c>Natural!</c> — calls <see cref="Nat.Factorial()"/>; returns <see cref="Nat"/>.</item>
    ///   <item><c>Integer!</c> — calls <see cref="Int.Factorial()"/>; throws <see cref="InvalidOperationException"/> for negatives.</item>
    ///   <item><c>Real!</c>    — throws <see cref="InvalidOperationException"/> with descriptive message.</item>
    /// </list>
    /// </summary>
    private Value EvaluatePostfix(PostfixExpr postfix)
    {
        var operand = Evaluate(postfix.Operand);

        return postfix.Op switch
        {
            PostfixOp.Factorial => operand.Kind switch
            {
                ValueKind.Natural => new Value(operand.AsNatural().Factorial()),
                ValueKind.Integer => new Value(operand.AsInteger().Factorial()),
                ValueKind.Real    => throw new InvalidOperationException(
                    "Factorial is not supported for Real numbers."),
                _ => throw new InvalidOperationException(
                    $"Factorial is not supported for type '{operand.Kind}'."),
            },

            _ => throw new InvalidOperationException(
                $"Postfix operator '{postfix.Op}' is not supported."),
        };
    }

    // -----------------------------------------------------------------
    // Built-in function calls
    // -----------------------------------------------------------------

    private Value EvaluateCall(CallExpr call) =>
        call.FunctionName switch
        {
            "abs"     => BuiltinAbs(call),
            "inv"     => BuiltinInv(call),
            "divrem"  => BuiltinDivRem(call),
            "is_even" => BuiltinIsEven(call),
            "is_odd"  => BuiltinIsOdd(call),
            "sign"    => BuiltinSign(call),
            _ => throw new InvalidOperationException(
                $"Unknown built-in function '{call.FunctionName}'."),
        };

    /// <summary>
    /// <c>inv(x)</c> — returns the multiplicative inverse (1 / x) of the argument.
    /// Widens the argument to <see cref="Rl"/> unconditionally, then calls <see cref="Rl.Invert"/>.
    /// </summary>
    /// <exception cref="DivideByZeroException">Thrown when the argument is zero.</exception>
    private Value BuiltinInv(CallExpr call)
    {
        if (call.Arguments.Count != 1)
            throw new InvalidOperationException(
                $"inv() expects exactly 1 argument, but got {call.Arguments.Count}.");

        var arg = Evaluate(call.Arguments[0]);
        var real = arg.Widen(ValueKind.Real).AsReal();
        return new Value(real.Invert());
    }

    /// <summary>
    /// <c>divrem(a, b)</c> — performs integer division and returns a formatted
    /// <see cref="Value"/> of kind <see cref="ValueKind.Text"/> containing
    /// <c>"quotient = Q, remainder = R"</c>.
    /// Supported for <see cref="ValueKind.Natural"/> and <see cref="ValueKind.Integer"/> only.
    /// </summary>
    /// <exception cref="InvalidOperationException">Thrown for <see cref="ValueKind.Real"/> arguments.</exception>
    private Value BuiltinDivRem(CallExpr call)
    {
        if (call.Arguments.Count != 2)
            throw new InvalidOperationException(
                $"divrem() expects exactly 2 arguments, but got {call.Arguments.Count}.");

        var a = Evaluate(call.Arguments[0]);
        var b = Evaluate(call.Arguments[1]);

        // Widen both to the wider of the two numeric kinds.
        (a, b) = Value.WidenPair(a, b);

        return a.Kind switch
        {
            ValueKind.Natural =>
                new Value(FormatDivRem(
                    Nat.DivRem(a.AsNatural(), b.AsNatural(), out var natRem),
                    natRem)),

            ValueKind.Integer =>
                new Value(FormatDivRem(
                    a.AsInteger().DivRem(b.AsInteger(), out var intRem),
                    intRem)),

            _ => throw new InvalidOperationException(
                $"divrem() is not supported for values of kind '{a.Kind}'. Use Natural or Integer operands."),
        };
    }

    private static string FormatDivRem(object quotient, object remainder) =>
        $"quotient = {quotient}, remainder = {remainder}";

    /// <summary>
    /// <c>abs(x)</c> — returns the absolute value of <paramref name="call"/>'s single argument.
    /// Dispatches to <see cref="Nat.Abs"/>, <see cref="Int.Abs"/>, or <see cref="Rl.Abs"/>
    /// depending on kind.
    /// </summary>
    private Value BuiltinAbs(CallExpr call)
    {
        if (call.Arguments.Count != 1)
            throw new InvalidOperationException(
                $"abs() expects exactly 1 argument, but got {call.Arguments.Count}.");

        var arg = Evaluate(call.Arguments[0]);

        return arg.Kind switch
        {
            ValueKind.Natural => new Value(Nat.Abs(arg.AsNatural())),
            ValueKind.Integer => new Value(Int.Abs(arg.AsInteger())),
            ValueKind.Real    => new Value(Rl.Abs(arg.AsReal())),
            _ => throw new InvalidOperationException(
                $"abs() is not supported for values of kind '{arg.Kind}'."),
        };
    }

    /// <summary>
    /// <c>sign(x)</c> — returns <c>-1</c>, <c>0</c>, or <c>1</c> as an
    /// <see cref="ValueKind.Integer"/> <see cref="Value"/> by widening the argument
    /// to at least <see cref="ValueKind.Integer"/> and reading <see cref="Int.Sign"/>.
    /// </summary>
    /// <exception cref="InvalidOperationException">
    /// Thrown when the argument is of kind <see cref="ValueKind.Real"/>, which has no
    /// integer <c>Sign</c> property, or when the wrong number of arguments is supplied.
    /// </exception>
    private Value BuiltinSign(CallExpr call)
    {
        if (call.Arguments.Count != 1)
            throw new InvalidOperationException(
                $"sign() expects exactly 1 argument, but got {call.Arguments.Count}.");

        var arg = Evaluate(call.Arguments[0]);

        // Widen Natural → Integer; Integer stays. Real is not supported.
        var intArg = arg.Kind switch
        {
            ValueKind.Natural => arg.Widen(ValueKind.Integer).AsInteger(),
            ValueKind.Integer => arg.AsInteger(),
            _ => throw new InvalidOperationException(
                $"sign() is not supported for values of kind '{arg.Kind}'. Use Natural or Integer operands."),
        };

        return new Value(new Int(intArg.Sign));
    }

    /// <summary>
    /// <c>is_even(x)</c> — returns <see langword="true"/> when the argument is an even integer.
    /// Dispatches to the static <c>IsEvenInteger</c> method on <see cref="Nat"/>,
    /// <see cref="Int"/>, or <see cref="Rl"/> depending on the argument's kind.
    /// </summary>
    private Value BuiltinIsEven(CallExpr call)
    {
        if (call.Arguments.Count != 1)
            throw new InvalidOperationException(
                $"is_even() expects exactly 1 argument, but got {call.Arguments.Count}.");

        var arg = Evaluate(call.Arguments[0]);

        bool result = arg.Kind switch
        {
            ValueKind.Natural => Nat.IsEvenInteger(arg.AsNatural()),
            ValueKind.Integer => Int.IsEvenInteger(arg.AsInteger()),
            ValueKind.Real    => Rl.IsEvenInteger(arg.AsReal()),
            _ => throw new InvalidOperationException(
                $"is_even() is not supported for values of kind '{arg.Kind}'."),
        };

        return new Value(result);
    }

    /// <summary>
    /// <c>is_odd(x)</c> — returns <see langword="true"/> when the argument is an odd integer.
    /// Dispatches to the static <c>IsOddInteger</c> method on <see cref="Nat"/>,
    /// <see cref="Int"/>, or <see cref="Rl"/> depending on the argument's kind.
    /// </summary>
    private Value BuiltinIsOdd(CallExpr call)
    {
        if (call.Arguments.Count != 1)
            throw new InvalidOperationException(
                $"is_odd() expects exactly 1 argument, but got {call.Arguments.Count}.");

        var arg = Evaluate(call.Arguments[0]);

        bool result = arg.Kind switch
        {
            ValueKind.Natural => Nat.IsOddInteger(arg.AsNatural()),
            ValueKind.Integer => Int.IsOddInteger(arg.AsInteger()),
            ValueKind.Real    => Rl.IsOddInteger(arg.AsReal()),
            _ => throw new InvalidOperationException(
                $"is_odd() is not supported for values of kind '{arg.Kind}'."),
        };

        return new Value(result);
    }
}
