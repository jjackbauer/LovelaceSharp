using System.Globalization;
using System.Text;
using Nat = global::Lovelace.Natural.Natural;
using Int = global::Lovelace.Integer.Integer;
using Rl = global::Lovelace.Real.Real;

namespace Lovelace.Suite;

/// <summary>
/// Tree-walking backend that evaluates an <see cref="Expr"/> or executes a
/// <see cref="Program"/> over a lexical <see cref="Scope"/>. Owns the global
/// scope and the function registry, and raises state-change notifications.
/// </summary>
public sealed class Interpreter
{
    // -----------------------------------------------------------------
    // Control-flow signals (internal; caught by the interpreter)
    // -----------------------------------------------------------------

    private sealed class ReturnSignal(Value value) : Exception { public Value Value { get; } = value; }
    private sealed class BreakSignal : Exception { }
    private sealed class ContinueSignal : Exception { }

    // -----------------------------------------------------------------
    // State
    // -----------------------------------------------------------------

    private readonly Scope _global = new();
    private readonly Dictionary<string, FunctionDefinition> _functions = new();
    private long _revision;

    // -----------------------------------------------------------------
    // Host-configurable settings
    // -----------------------------------------------------------------

    /// <summary>Where <c>print</c> writes. Defaults to the console.</summary>
    public TextWriter Output { get; set; } = Console.Out;

    /// <summary>Directory into which <c>plot</c> writes its SVG file.</summary>
    public string PlotOutputDirectory { get; set; } = Directory.GetCurrentDirectory();

    /// <summary>File name used by <c>plot</c>.</summary>
    public string PlotFileName { get; set; } = "plot.svg";

    /// <summary>The SVG and title of the most recently rendered plot, if any.</summary>
    public PlotCapture? LastPlot { get; private set; }

    /// <summary>Clears the last-plot capture.</summary>
    public void ResetPlotCapture() => LastPlot = null;

    // -----------------------------------------------------------------
    // Events
    // -----------------------------------------------------------------

    /// <summary>Raised when a global variable is defined, reassigned, or removed.</summary>
    public event EventHandler<VariableChangedEventArgs>? VariableChanged;

    /// <summary>Raised when a function is defined.</summary>
    public event EventHandler<FunctionDefinedEventArgs>? FunctionDefined;

    // -----------------------------------------------------------------
    // Constructor
    // -----------------------------------------------------------------

    public Interpreter() => RegisterBuiltins();

    // -----------------------------------------------------------------
    // Introspection surface
    // -----------------------------------------------------------------

    /// <summary>A read-only view of the global variable store.</summary>
    public IReadOnlyDictionary<string, Value> Variables => _global.Values;

    /// <summary>A read-only view of all functions (user + built-in).</summary>
    public IReadOnlyDictionary<string, FunctionDefinition> Functions => _functions;

    /// <summary>Monotonic revision counter bumped on every state mutation.</summary>
    public long Revision => _revision;

    // -----------------------------------------------------------------
    // State mutation
    // -----------------------------------------------------------------

    /// <summary>Defines or overwrites a global variable.</summary>
    public void SetVariable(string name, Value value)
    {
        _global.Define(name, value);
        RaiseVariableChanged(name, value);
    }

    /// <summary>Removes a global variable. Returns <see langword="true"/> if it existed.</summary>
    public bool Remove(string name)
    {
        if (_global.TryGet(name, out var old))
        {
            _global.Remove(name);
            _revision++;
            VariableChanged?.Invoke(this, new VariableChangedEventArgs(name, old, removed: true));
            return true;
        }
        return false;
    }

    /// <summary>Clears all global variables (functions and built-ins remain).</summary>
    public void Clear()
    {
        _global.Clear();
        _revision++;
    }

    /// <summary>Registers a user or built-in function definition.</summary>
    public void DefineFunction(FunctionDefinition definition)
    {
        _functions[definition.Name] = definition;
        RaiseFunctionDefined(definition);
    }

    /// <summary>Registers a host-provided native function.</summary>
    public void RegisterBuiltin(string name, IReadOnlyList<string> parameters, Func<IReadOnlyList<Value>, Value> implementation)
    {
        BuiltinFunction impl = args => Task.FromResult(implementation(args));
        _functions[name] = new FunctionDefinition(name, parameters, impl);
        RaiseFunctionDefined(_functions[name]);
    }

    // -----------------------------------------------------------------
    // Entry points
    // -----------------------------------------------------------------

    /// <summary>Evaluates a single expression in the global scope.</summary>
    public Task<Value> EvaluateAsync(Expr expr) => EvaluateAsync(expr, _global);

    /// <summary>Executes a program (list of statements) in the global scope.</summary>
    public async Task<Value> ExecuteAsync(Program program)
    {
        try
        {
            return await ExecuteStatementListAsync(program.Statements, _global);
        }
        catch (ReturnSignal rs)
        {
            return rs.Value;
        }
        catch (BreakSignal)
        {
            throw new InvalidOperationException("'break' is only valid inside a loop.");
        }
        catch (ContinueSignal)
        {
            throw new InvalidOperationException("'continue' is only valid inside a loop.");
        }
    }

    // -----------------------------------------------------------------
    // Expression evaluation
    // -----------------------------------------------------------------

    private async Task<Value> EvaluateAsync(Expr expr, Scope scope)
    {
        switch (expr)
        {
            case LiteralExpr lit: return EvaluateLiteral(lit);
            case VariableExpr var: return EvaluateVariable(var, scope);
            case AssignExpr assign: return await EvaluateAssignAsync(assign, scope);
            case BinaryExpr bin: return await EvaluateBinaryAsync(bin, scope);
            case UnaryExpr unary: return await EvaluateUnaryAsync(unary, scope);
            case PostfixExpr postfix: return await EvaluatePostfixAsync(postfix, scope);
            case CallExpr call: return await EvaluateCallAsync(call, scope);
            case StringExpr str: return new Value(str.Value);
            case RangeExpr range: return await EvaluateRangeAsync(range, scope);
            case IndexExpr idx: return await EvaluateIndexAsync(idx, scope);
            case ListExpr list: return await EvaluateListAsync(list, scope);
            case InterpolatedStringExpr interp: return await EvaluateInterpolatedAsync(interp, scope);
            default: throw new NotImplementedException($"Unsupported expression type: {expr.GetType().Name}");
        }
    }

    private static Value EvaluateLiteral(LiteralExpr lit)
    {
        var text = lit.RawText;
        if (text.Contains('.') || text.Contains('('))
            return new Value(Rl.Parse(text, null));
        return new Value(Nat.Parse(text, null));
    }

    private static Value EvaluateVariable(VariableExpr var, Scope scope)
    {
        if (scope.TryGet(var.Name, out var value))
            return value;
        throw new InvalidOperationException($"Undefined variable '{var.Name}'.");
    }

    private async Task<Value> EvaluateAssignAsync(AssignExpr assign, Scope scope)
    {
        var value = await EvaluateAsync(assign.Value, scope);
        DefineInScope(scope, assign.Name, value);
        return value;
    }

    private void DefineInScope(Scope scope, string name, Value value)
    {
        var target = scope.Assign(name, value);
        if (ReferenceEquals(target, _global))
            RaiseVariableChanged(name, value);
    }

    // -----------------------------------------------------------------
    // Binary operators
    // -----------------------------------------------------------------

    private async Task<Value> EvaluateBinaryAsync(BinaryExpr bin, Scope scope)
    {
        var left = await EvaluateAsync(bin.Left, scope);
        var right = await EvaluateAsync(bin.Right, scope);

        if (bin.Op is BinaryOp.Equal or BinaryOp.NotEqual
                   or BinaryOp.Greater or BinaryOp.Less
                   or BinaryOp.GreaterEqual or BinaryOp.LessEqual)
            return EvaluateComparison(left, right, bin.Op);

        if (left.Kind == ValueKind.Vector || right.Kind == ValueKind.Vector)
            return EvaluateVectorBinary(bin.Op, left, right);

        return ApplyScalarBinary(bin.Op, left, right);
    }

    /// <summary>Widens both operands to the wider numeric kind, then dispatches.</summary>
    private static Value ApplyScalarBinary(BinaryOp op, Value left, Value right)
    {
        (left, right) = Value.WidenPair(left, right);

        return (op, left.Kind) switch
        {
            // ---- Natural arithmetic ----
            (BinaryOp.Add,      ValueKind.Natural) => new Value(left.AsNatural() + right.AsNatural()),
            (BinaryOp.Subtract, ValueKind.Natural) => SubtractNatural(left, right),
            (BinaryOp.Multiply, ValueKind.Natural) => new Value(left.AsNatural() * right.AsNatural()),
            (BinaryOp.Divide,   ValueKind.Natural) => DivideNatural(left, right),
            (BinaryOp.Modulo,   ValueKind.Natural) => new Value(left.AsNatural() % right.AsNatural()),
            (BinaryOp.Power,    ValueKind.Natural) => new Value(left.AsNatural().Pow(right.AsNatural())),

            // ---- Integer arithmetic ----
            (BinaryOp.Add,      ValueKind.Integer) => new Value(left.AsInteger() + right.AsInteger()),
            (BinaryOp.Subtract, ValueKind.Integer) => new Value(left.AsInteger() - right.AsInteger()),
            (BinaryOp.Multiply, ValueKind.Integer) => new Value(left.AsInteger() * right.AsInteger()),
            (BinaryOp.Divide,   ValueKind.Integer) => DivideInteger(left, right),
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
                $"Operator '{op}' is not supported for type '{left.Kind}'."),
        };
    }

    /// <summary>Element-wise vector arithmetic with scalar broadcast.</summary>
    private static Value EvaluateVectorBinary(BinaryOp op, Value left, Value right)
    {
        if (left.Kind == ValueKind.Vector && right.Kind == ValueKind.Vector)
        {
            var a = left.AsVector();
            var b = right.AsVector();
            if (a.Count != b.Count)
                throw new InvalidOperationException($"Vector operands must have the same length ({a.Count} vs {b.Count}).");

            var result = new List<Value>(a.Count);
            for (int i = 0; i < a.Count; i++)
                result.Add(ApplyScalarBinary(op, a[i], b[i]));
            return new Value(result);
        }

        if (left.Kind == ValueKind.Vector)
        {
            var result = new List<Value>(left.AsVector().Count);
            foreach (var e in left.AsVector())
                result.Add(ApplyScalarBinary(op, e, right));
            return new Value(result);
        }

        var result2 = new List<Value>(right.AsVector().Count);
        foreach (var e in right.AsVector())
            result2.Add(ApplyScalarBinary(op, left, e));
        return new Value(result2);
    }

    private static Value EvaluateComparison(Value left, Value right, BinaryOp op)
    {
        (left, right) = Value.WidenPair(left, right);

        int cmp = left.Kind switch
        {
            ValueKind.Natural => left.AsNatural().CompareTo(right.AsNatural()),
            ValueKind.Integer => left.AsInteger().CompareTo(right.AsInteger()),
            ValueKind.Real    => left.AsReal().CompareTo(right.AsReal()),
            _ => throw new InvalidOperationException($"Cannot compare values of kind '{left.Kind}'."),
        };

        bool result = op switch
        {
            BinaryOp.Equal        => cmp == 0,
            BinaryOp.NotEqual     => cmp != 0,
            BinaryOp.Greater      => cmp > 0,
            BinaryOp.Less         => cmp < 0,
            BinaryOp.GreaterEqual => cmp >= 0,
            BinaryOp.LessEqual    => cmp <= 0,
            _ => throw new InvalidOperationException($"Unknown comparison operator '{op}'."),
        };

        return new Value(result);
    }

    /// <summary>Natural subtraction that auto-widens to Integer on underflow.</summary>
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

    /// <summary>
    /// Natural division. An exact quotient stays <see cref="ValueKind.Natural"/>; a non-zero
    /// remainder widens to <see cref="ValueKind.Real"/> so the exact rational result is
    /// preserved with period detection (e.g. <c>1 / 3 = 0.(3)</c>) instead of truncating.
    /// </summary>
    private static Value DivideNatural(Value left, Value right)
    {
        var quotient = Nat.DivRem(left.AsNatural(), right.AsNatural(), out var remainder);
        if (Nat.IsZero(remainder))
            return new Value(quotient);

        return new Value(Rl.Divide(
            left.Widen(ValueKind.Real).AsReal(),
            right.Widen(ValueKind.Real).AsReal()));
    }

    /// <summary>
    /// Integer division. An exact quotient stays <see cref="ValueKind.Integer"/>; otherwise the
    /// result widens to <see cref="ValueKind.Real"/> (e.g. <c>-7 / 2 = -3.5</c>).
    /// </summary>
    private static Value DivideInteger(Value left, Value right)
    {
        var quotient = left.AsInteger().DivRem(right.AsInteger(), out var remainder);
        if (Int.IsZero(remainder))
            return new Value(quotient);

        return new Value(Rl.Divide(
            left.Widen(ValueKind.Real).AsReal(),
            right.Widen(ValueKind.Real).AsReal()));
    }

    // -----------------------------------------------------------------
    // Unary / postfix
    // -----------------------------------------------------------------

    private async Task<Value> EvaluateUnaryAsync(UnaryExpr unary, Scope scope)
    {
        var operand = await EvaluateAsync(unary.Operand, scope);

        return unary.Op switch
        {
            UnaryOp.Plus => operand,

            UnaryOp.Negate => operand.Kind switch
            {
                ValueKind.Natural => new Value(-operand.Widen(ValueKind.Integer).AsInteger()),
                ValueKind.Integer => new Value(-operand.AsInteger()),
                ValueKind.Real    => new Value(-operand.AsReal()),
                _ => throw new InvalidOperationException($"Unary negation is not supported for type '{operand.Kind}'."),
            },

            _ => throw new InvalidOperationException($"Unary operator '{unary.Op}' is not supported."),
        };
    }

    private async Task<Value> EvaluatePostfixAsync(PostfixExpr postfix, Scope scope)
    {
        var operand = await EvaluateAsync(postfix.Operand, scope);

        return postfix.Op switch
        {
            PostfixOp.Factorial => operand.Kind switch
            {
                ValueKind.Natural => new Value(operand.AsNatural().Factorial()),
                ValueKind.Integer => new Value(operand.AsInteger().Factorial()),
                ValueKind.Real    => throw new InvalidOperationException("Factorial is not supported for Real numbers."),
                _ => throw new InvalidOperationException($"Factorial is not supported for type '{operand.Kind}'."),
            },

            _ => throw new InvalidOperationException($"Postfix operator '{postfix.Op}' is not supported."),
        };
    }

    // -----------------------------------------------------------------
    // Calls
    // -----------------------------------------------------------------

    private async Task<Value> EvaluateCallAsync(CallExpr call, Scope scope)
    {
        var args = new List<Value>(call.Arguments.Count);
        foreach (var a in call.Arguments)
            args.Add(await EvaluateAsync(a, scope));

        if (_functions.TryGetValue(call.FunctionName, out var fn))
        {
            if (fn.IsBuiltin)
                return await fn.Builtin!(args);
            return await CallUserFunctionAsync(fn, args);
        }

        throw new InvalidOperationException($"Unknown function '{call.FunctionName}'.");
    }

    private async Task<Value> CallUserFunctionAsync(FunctionDefinition fn, IReadOnlyList<Value> args)
    {
        if (args.Count != fn.Parameters.Count)
            throw new InvalidOperationException(
                $"Function '{fn.Name}' expects {fn.Parameters.Count} argument(s), but got {args.Count}.");

        var frame = new Scope(_global);
        for (int i = 0; i < fn.Parameters.Count; i++)
            frame.Define(fn.Parameters[i], args[i]);

        try
        {
            return await ExecuteStatementListAsync(fn.Body, frame);
        }
        catch (ReturnSignal rs)
        {
            return rs.Value;
        }
    }

    // -----------------------------------------------------------------
    // Ranges, indexing, lists, interpolation
    // -----------------------------------------------------------------

    private async Task<Value> EvaluateRangeAsync(RangeExpr range, Scope scope)
    {
        var start = await EvaluateAsync(range.Start, scope);
        var end = await EvaluateAsync(range.End, scope);
        Value? step = range.Step is null ? null : await EvaluateAsync(range.Step, scope);
        return BuildRange(start, step, end);
    }

    private async Task<Value> EvaluateIndexAsync(IndexExpr idx, Scope scope)
    {
        var target = await EvaluateAsync(idx.Target, scope);
        var index = await EvaluateAsync(idx.Index, scope);

        if (target.Kind != ValueKind.Vector)
            throw new InvalidOperationException($"Indexing is not supported for type '{target.Kind}'.");

        long i = ToLong(index);
        var vec = target.AsVector();
        if (i < 0 || i >= vec.Count)
            throw new InvalidOperationException($"Index {i} is out of range for vector of length {vec.Count}.");

        return vec[(int)i];
    }

    private async Task<Value> EvaluateListAsync(ListExpr list, Scope scope)
    {
        var elements = new List<Value>(list.Elements.Count);
        foreach (var e in list.Elements)
            elements.Add(await EvaluateAsync(e, scope));
        return new Value(elements);
    }

    private async Task<Value> EvaluateInterpolatedAsync(InterpolatedStringExpr interp, Scope scope)
    {
        var sb = new StringBuilder();
        foreach (var part in interp.Parts)
        {
            switch (part)
            {
                case TextPart t:
                    sb.Append(t.Text);
                    break;
                case ExpressionPart e:
                    sb.Append(ValueFormatter.Format(await EvaluateAsync(e.Expression, scope)));
                    break;
            }
        }
        return new Value(sb.ToString());
    }

    private static Value BuildRange(Value start, Value? step, Value end)
    {
        Int s = ToInteger(start);
        Int e = ToInteger(end);
        Int st = step is null ? new Int(1) : ToInteger(step);

        if (st.Sign == 0)
            throw new InvalidOperationException("Range step must not be zero.");

        bool natural = step is null && start.Kind == ValueKind.Natural && end.Kind == ValueKind.Natural;

        var elements = new List<Value>();
        Int current = s;

        if (st.Sign > 0)
        {
            while (current.CompareTo(e) <= 0)
            {
                elements.Add(natural ? new Value(current.ToNatural()) : new Value(current));
                current = current + st;
            }
        }
        else
        {
            while (current.CompareTo(e) >= 0)
            {
                elements.Add(natural ? new Value(current.ToNatural()) : new Value(current));
                current = current + st;
            }
        }

        return new Value(elements);
    }

    private static Int ToInteger(Value value) => value.Kind switch
    {
        ValueKind.Natural => new Int(value.AsNatural()),
        ValueKind.Integer => value.AsInteger(),
        _ => throw new InvalidOperationException($"Range bounds must be Natural or Integer, but got '{value.Kind}'."),
    };

    private static long ToLong(Value value) => value.Kind switch
    {
        ValueKind.Natural => long.Parse(value.AsNatural().ToString(), CultureInfo.InvariantCulture),
        ValueKind.Integer => long.Parse(value.AsInteger().ToString(), CultureInfo.InvariantCulture),
        _ => throw new InvalidOperationException($"Index must be Natural or Integer, but got '{value.Kind}'."),
    };

    // -----------------------------------------------------------------
    // Statement execution
    // -----------------------------------------------------------------

    private async Task<Value> ExecuteStatementListAsync(IReadOnlyList<Statement> statements, Scope scope)
    {
        Value last = Value.Void;
        foreach (var s in statements)
            last = await ExecuteAsync(s, scope);
        return last;
    }

    private async Task<Value> ExecuteAsync(Statement stmt, Scope scope)
    {
        switch (stmt)
        {
            case ExpressionStatement es:
                return await EvaluateAsync(es.Expression, scope);

            case BlockStatement block:
                return await ExecuteBlockAsync(block, scope);

            case IfStatement ifStmt:
                return await ExecuteIfAsync(ifStmt, scope);

            case WhileStatement whileStmt:
                return await ExecuteWhileAsync(whileStmt, scope);

            case ForStatement forStmt:
                return await ExecuteForAsync(forStmt, scope);

            case ReturnStatement returnStmt:
                return await ExecuteReturnAsync(returnStmt, scope);

            case BreakStatement:
                throw new BreakSignal();

            case ContinueStatement:
                throw new ContinueSignal();

            case FunctionStatement funcStmt:
                DefineFunction(funcStmt.Definition);
                return Value.Void;

            default:
                throw new NotImplementedException($"Unsupported statement type: {stmt.GetType().Name}");
        }
    }

    private async Task<Value> ExecuteBlockAsync(BlockStatement block, Scope scope)
    {
        var child = new Scope(scope);
        return await ExecuteStatementListAsync(block.Statements, child);
    }

    private async Task<Value> ExecuteIfAsync(IfStatement stmt, Scope scope)
    {
        var condition = await EvaluateAsync(stmt.Condition, scope);
        if (condition.Kind != ValueKind.Boolean)
            throw new InvalidOperationException($"if condition must be Boolean, but got '{condition.Kind}'.");

        if (condition.AsBoolean())
            return await ExecuteAsync(stmt.Then, scope);

        if (stmt.Else is not null)
            return await ExecuteAsync(stmt.Else, scope);

        return Value.Void;
    }

    private async Task<Value> ExecuteWhileAsync(WhileStatement stmt, Scope scope)
    {
        Value last = Value.Void;

        while (true)
        {
            var condition = await EvaluateAsync(stmt.Condition, scope);
            if (condition.Kind != ValueKind.Boolean)
                throw new InvalidOperationException($"while condition must be Boolean, but got '{condition.Kind}'.");

            if (!condition.AsBoolean())
                break;

            try
            {
                last = await ExecuteAsync(stmt.Body, scope);
            }
            catch (BreakSignal)
            {
                break;
            }
            catch (ContinueSignal)
            {
                // continue to next iteration
            }
        }

        return last;
    }

    private async Task<Value> ExecuteForAsync(ForStatement stmt, Scope scope)
    {
        var range = await EvaluateAsync(stmt.Range, scope);
        if (range.Kind != ValueKind.Vector)
            throw new InvalidOperationException($"for loop range must be a vector, but got '{range.Kind}'.");

        Value last = Value.Void;

        foreach (var element in range.AsVector())
        {
            scope.Define(stmt.Variable, element);

            try
            {
                last = await ExecuteAsync(stmt.Body, scope);
            }
            catch (BreakSignal)
            {
                break;
            }
            catch (ContinueSignal)
            {
                // continue to next element
            }
        }

        return last;
    }

    private async Task<Value> ExecuteReturnAsync(ReturnStatement stmt, Scope scope)
    {
        var value = stmt.Value is null ? Value.Void : await EvaluateAsync(stmt.Value, scope);
        throw new ReturnSignal(value);
    }

    // -----------------------------------------------------------------
    // Built-in registration
    // -----------------------------------------------------------------

    private void Register(string name, IReadOnlyList<string> parameters, BuiltinFunction impl) =>
        _functions[name] = new FunctionDefinition(name, parameters, impl);

    private static void RequireArity(string name, IReadOnlyList<Value> args, int expected)
    {
        if (args.Count != expected)
            throw new InvalidOperationException($"{name}() expects exactly {expected} argument(s), but got {args.Count}.");
    }

    private void RegisterBuiltins()
    {
        // abs(x)
        Register("abs", ["x"], args =>
        {
            RequireArity("abs", args, 1);
            var arg = args[0];
            return Task.FromResult(arg.Kind switch
            {
                ValueKind.Natural => new Value(Nat.Abs(arg.AsNatural())),
                ValueKind.Integer => new Value(Int.Abs(arg.AsInteger())),
                ValueKind.Real    => new Value(Rl.Abs(arg.AsReal())),
                _ => throw new InvalidOperationException($"abs() is not supported for values of kind '{arg.Kind}'."),
            });
        });

        // inv(x)
        Register("inv", ["x"], args =>
        {
            RequireArity("inv", args, 1);
            var arg = args[0];
            var real = arg.Widen(ValueKind.Real).AsReal();
            return Task.FromResult<Value>(new Value(real.Invert()));
        });

        // divrem(a, b)
        Register("divrem", ["a", "b"], args =>
        {
            RequireArity("divrem", args, 2);
            var a = args[0];
            var b = args[1];
            (a, b) = Value.WidenPair(a, b);

            return Task.FromResult(a.Kind switch
            {
                ValueKind.Natural => new Value(FormatDivRem(Nat.DivRem(a.AsNatural(), b.AsNatural(), out var natRem), natRem)),
                ValueKind.Integer => new Value(FormatDivRem(a.AsInteger().DivRem(b.AsInteger(), out var intRem), intRem)),
                _ => throw new InvalidOperationException($"divrem() is not supported for values of kind '{a.Kind}'. Use Natural or Integer operands."),
            });
        });

        // is_even(x)
        Register("is_even", ["x"], args =>
        {
            RequireArity("is_even", args, 1);
            var arg = args[0];
            bool result = arg.Kind switch
            {
                ValueKind.Natural => Nat.IsEvenInteger(arg.AsNatural()),
                ValueKind.Integer => Int.IsEvenInteger(arg.AsInteger()),
                ValueKind.Real    => Rl.IsEvenInteger(arg.AsReal()),
                _ => throw new InvalidOperationException($"is_even() is not supported for values of kind '{arg.Kind}'."),
            };
            return Task.FromResult<Value>(new Value(result));
        });

        // is_odd(x)
        Register("is_odd", ["x"], args =>
        {
            RequireArity("is_odd", args, 1);
            var arg = args[0];
            bool result = arg.Kind switch
            {
                ValueKind.Natural => Nat.IsOddInteger(arg.AsNatural()),
                ValueKind.Integer => Int.IsOddInteger(arg.AsInteger()),
                ValueKind.Real    => Rl.IsOddInteger(arg.AsReal()),
                _ => throw new InvalidOperationException($"is_odd() is not supported for values of kind '{arg.Kind}'."),
            };
            return Task.FromResult<Value>(new Value(result));
        });

        // sign(x)
        Register("sign", ["x"], args =>
        {
            RequireArity("sign", args, 1);
            var arg = args[0];
            var intArg = arg.Kind switch
            {
                ValueKind.Natural => arg.Widen(ValueKind.Integer).AsInteger(),
                ValueKind.Integer => arg.AsInteger(),
                _ => throw new InvalidOperationException($"sign() is not supported for values of kind '{arg.Kind}'. Use Natural or Integer operands."),
            };
            return Task.FromResult<Value>(new Value(new Int(intArg.Sign)));
        });

        // sqrt(x)
        Register("sqrt", ["x"], async args =>
        {
            RequireArity("sqrt", args, 1);
            var arg = args[0];
            var real = arg.Widen(ValueKind.Real).AsReal();
            return new Value(await Rl.SqrtAsync(real));
        });

        // pi() / pi(digits)
        Register("pi", ["digits"], async args =>
        {
            switch (args.Count)
            {
                case 0:
                    return new Value(await Rl.PiAsync(Rl.DisplayDecimalPlaces));

                case 1:
                {
                    var arg = args[0];
                    long digits = arg.Kind switch
                    {
                        ValueKind.Natural => long.Parse(arg.AsNatural().ToString(), CultureInfo.InvariantCulture),
                        ValueKind.Integer => long.Parse(arg.AsInteger().ToString(), CultureInfo.InvariantCulture),
                        _ => throw new InvalidOperationException($"pi() expects a Natural or Integer digit count, but got '{arg.Kind}'."),
                    };
                    return new Value(await Rl.PiAsync(digits));
                }

                default:
                    throw new InvalidOperationException($"pi() expects 0 or 1 argument, but got {args.Count}.");
            }
        });

        // print(values...)
        Register("print", ["values"], args =>
        {
            Output.WriteLine(string.Join(" ", args.Select(ValueFormatter.Format)));
            return Task.FromResult(Value.Void);
        });

        // len(v)
        Register("len", ["v"], args =>
        {
            RequireArity("len", args, 1);
            var arg = args[0];
            if (arg.Kind != ValueKind.Vector)
                throw new InvalidOperationException($"len() expects a vector, but got '{arg.Kind}'.");
            return Task.FromResult<Value>(new Value(new Nat(arg.AsVector().Count)));
        });

        // plot(...)
        Register("plot", ["x", "y", "title"], args => Task.FromResult(BuiltinPlot(args)));
    }

    private Value BuiltinPlot(IReadOnlyList<Value> args)
    {
        Value xs;
        Value ys;
        string? title = null;

        switch (args.Count)
        {
            case 1:
                ys = args[0];
                xs = BuildIndexVector(ys);
                break;

            case 2:
                xs = args[0];
                ys = args[1];
                break;

            case 3:
                xs = args[0];
                ys = args[1];
                if (args[2].Kind != ValueKind.Text)
                    throw new InvalidOperationException($"plot() title must be a string, but got '{args[2].Kind}'.");
                title = args[2].AsText();
                break;

            default:
                throw new InvalidOperationException($"plot() expects 1 to 3 arguments, but got {args.Count}.");
        }

        if (xs.Kind != ValueKind.Vector || ys.Kind != ValueKind.Vector)
            throw new InvalidOperationException("plot() arguments must be vectors.");

        var xv = xs.AsVector();
        var yv = ys.AsVector();

        if (xv.Count != yv.Count)
            throw new InvalidOperationException($"plot() vectors must have the same length ({xv.Count} vs {yv.Count}).");

        if (xv.Count == 0)
            throw new InvalidOperationException("plot() cannot plot an empty vector.");

        var model = new PlotModel { Title = title };
        var series = new PlotSeries();
        for (int i = 0; i < xv.Count; i++)
            series.Points.Add(new PlotPoint(PlotValue.ToDouble(xv[i]), PlotValue.ToDouble(yv[i])));
        model.Series.Add(series);

        string path = Path.Combine(PlotOutputDirectory, PlotFileName);
        string full = Path.GetFullPath(path);
        string svg = new SvgPlotRenderer().Render(model);
        File.WriteAllText(full, svg);
        LastPlot = new PlotCapture(svg, title);

        return new Value(full);
    }

    private static Value BuildIndexVector(Value vector)
    {
        int count = vector.AsVector().Count;
        var elements = new List<Value>(count);
        for (int i = 1; i <= count; i++)
            elements.Add(new Value(new Nat(i)));
        return new Value(elements);
    }

    private static string FormatDivRem(object quotient, object remainder) =>
        $"quotient = {quotient}, remainder = {remainder}";

    // -----------------------------------------------------------------
    // Event helpers
    // -----------------------------------------------------------------

    private void RaiseVariableChanged(string name, Value value)
    {
        _revision++;
        VariableChanged?.Invoke(this, new VariableChangedEventArgs(name, value));
    }

    private void RaiseFunctionDefined(FunctionDefinition definition)
    {
        _revision++;
        FunctionDefined?.Invoke(this, new FunctionDefinedEventArgs(definition));
    }
}
