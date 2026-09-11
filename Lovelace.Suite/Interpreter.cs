using System.Diagnostics;
using System.Globalization;
using System.Text;
using Lovelace.Abstractions;
// the Round-5 completeness mapping and the solver's status enum live in the symbolic layer, which
// Suite already references; aliased so the AST type Expr stays unambiguous in this file
using SolveStatus = Lovelace.Symbolics.SolveStatus;
// the WIRE diagnostic (Round 4b), aliased because Lovelace.Suite.Diagnostic is the engine's own
// parse/engine diagnostic record — different type, different layer
using WireDiagnostic = Lovelace.Abstractions.Diagnostic;
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
    private readonly Dictionary<string, BuiltinDescriptor> _builtinDescriptors = new(StringComparer.Ordinal);
    private long _revision;
    private readonly List<OperationTiming> _timings = [];

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

    /// <summary>Computation precision (Real decimal places). Default 1000.</summary>
    public long ComputationDecimalPlaces
    {
        get => _computationDecimalPlaces;
        set { _computationDecimalPlaces = value; PrecisionExplicitlySet = true; }
    }

    /// <summary>Display precision (Real fractional digits shown). Default 100.</summary>
    public long DisplayDecimalPlaces
    {
        get => _displayDecimalPlaces;
        set { _displayDecimalPlaces = value; PrecisionExplicitlySet = true; }
    }

    /// <summary>
    /// True once the precision knob has been explicitly changed from its defaults (via
    /// <see cref="SetPrecision"/>, <see cref="ComputationDecimalPlaces"/>, or
    /// <see cref="DisplayDecimalPlaces"/>). <see cref="ModusHost"/> reads it to decide whether
    /// plugin-registered builtins keep their fast default budget or follow the engine knob.
    /// </summary>
    public bool PrecisionExplicitlySet { get; private set; }

    private long _computationDecimalPlaces = 1000L;
    private long _displayDecimalPlaces = 100L;

    /// <summary>Sets both computation and display precision (the single "precision" knob).</summary>
    public void SetPrecision(long decimalPlaces)
    {
        ComputationDecimalPlaces = decimalPlaces;
        DisplayDecimalPlaces = decimalPlaces;
    }

    /// <summary>Optional sink for sub-operation progress (sqrt/pi/factorial), set by the host.</summary>
    public IProgress<OperationProgress>? ProgressReporter { get; set; }

    private IProgress<double>? SubProgress(string label) =>
        ProgressReporter is null
            ? null
            : new SyncProgress<double>(f => ProgressReporter.Report(new OperationProgress(label, Math.Clamp(f, 0.0, 1.0))));

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

    /// <summary>Discoverability metadata for builtins registered with a descriptor
    /// (plugin-provided entries; core builtins resolve through <see cref="CoreBuiltinMetadata"/>).</summary>
    public IReadOnlyDictionary<string, BuiltinDescriptor> BuiltinDescriptors => _builtinDescriptors;

    /// <summary>The symbolic-matrix bridge (registered by the Symbolics plugin) for
    /// determinant/rank/inverse/solve dispatch; null on a bare engine.</summary>
    internal Lovelace.Abstractions.ISymbolicMatrixBridge? SymbolicMatrixBridge { get; set; }

    /// <summary>Optional introspection bridge: supplies the structured assumptions that constrain
    /// an expression's symbols for <c>inspect()</c>.</summary>
    internal Lovelace.Abstractions.ISymbolicInspectionBridge? SymbolicInspectionBridge { get; set; }

    /// <summary>Whether display formatting prefers Unicode (∞ √ π ≤ ≥ ≠). ASCII is the default;
    /// set via the REPL's <c>set pretty unicode|ascii</c> command.</summary>
    public bool UnicodeOutput { get; set; }

    /// <summary>Monotonic revision counter bumped on every state mutation.</summary>
    public long Revision => _revision;

    /// <summary>Per-statement timings from the most recent program execution, in statement order.</summary>
    public IReadOnlyList<OperationTiming> OperationTimings => _timings;

    /// <summary>Clears the accumulated per-statement timings (called before each evaluation).</summary>
    internal void ClearOperationTimings() => _timings.Clear();

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
    public void RegisterBuiltin(string name, IReadOnlyList<string> parameters, Func<IReadOnlyList<Value>, Value> implementation) =>
        RegisterBuiltin(name, parameters, implementation, descriptor: null);

    /// <summary>Registers a builtin; when a <see cref="BuiltinDescriptor"/> is supplied it is
    /// stored in <see cref="BuiltinDescriptors"/> for the help/funcs/completion surfaces.</summary>
    public void RegisterBuiltin(string name, IReadOnlyList<string> parameters, Func<IReadOnlyList<Value>, Value> implementation, BuiltinDescriptor? descriptor)
    {
        BuiltinFunction impl = args => Task.FromResult(implementation(args));
        _functions[name] = new FunctionDefinition(name, parameters, impl);
        if (descriptor is not null)
            _builtinDescriptors[name] = descriptor;
        RaiseFunctionDefined(_functions[name]);
    }

    // -----------------------------------------------------------------
    // Entry points
    // -----------------------------------------------------------------

    /// <summary>Evaluates a single expression in the global scope, scoped to this engine's precision.</summary>
    public async Task<Value> EvaluateAsync(Expr expr)
    {
        using var scope = Rl.WithPrecision(ComputationDecimalPlaces, DisplayDecimalPlaces);
        return await EvaluateAsync(expr, _global);
    }

    /// <summary>
    /// Executes a program (list of statements) in the global scope, timing each
    /// top-level statement and recording the elapsed time with its source position.
    /// </summary>
    public async Task<Value> ExecuteAsync(Program program)
    {
        _timings.Clear();
        Value last = Value.Void;

        try
        {
            for (int i = 0; i < program.Statements.Count; i++)
            {
                // statement granularity: a cancelled script stops between statements, so the
                // result it already produced (variables, captured output) stays readable
                Lovelace.Abstractions.Cancellation.ThrowIfCancellationRequested();
                var statement = program.Statements[i];
                int position = i < program.StatementPositions.Count ? program.StatementPositions[i] : 0;

                // Re-enter the precision scope each statement so a mid-script setprecision
                // takes effect immediately for subsequent statements.
                using var statementScope = Rl.WithPrecision(ComputationDecimalPlaces, DisplayDecimalPlaces);

                var stopwatch = Stopwatch.StartNew();
                var capture = new StringWriter();
                var previousOutput = Output;
                Output = capture;
                Value result = Value.Void;
                try
                {
                    result = await ExecuteAsync(statement, _global);
                    last = result;
                }
                finally
                {
                    Output = previousOutput;
                    string output = capture.ToString();
                    if (output.Length > 0)
                        previousOutput.Write(output);
                    stopwatch.Stop();
                    _timings.Add(new OperationTiming(position, result, output, stopwatch.Elapsed));
                }
            }

            return last;
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
            case MemberExpr member: return await EvaluateMemberAsync(member, scope);
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

        // Mathematical constants, computed once at the configured maximum precision (lazily).
        // A user assignment shadows these (TryGet above wins).
        if (var.Name == "pi")
            return new Value(Rl.Pi);
        if (var.Name == "e")
            return new Value(Rl.E);
        if (var.Name == "inf")
            return new Value(Lovelace.Symbolics.Exprs.Infinity);

        // First-class mathematical domains (solve/symbol options; also callable as builtins).
        if (var.Name == "real")
            return new Value(MathDomain.Real);
        if (var.Name == "complex")
            return new Value(MathDomain.Complex);
        if (var.Name == "integer")
            return new Value(MathDomain.Integer);
        if (var.Name == "rational")
            return new Value(MathDomain.Rational);

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

        if (IsArrayLike(left) || IsArrayLike(right))
            return EvaluateArrayBinary(bin.Op, left, right);

        return ApplyScalarBinary(bin.Op, left, right);
    }

    /// <summary>Widens both operands to the wider numeric kind, then dispatches.</summary>
    private static Value ApplyScalarBinary(BinaryOp op, Value left, Value right) =>
        NumericOps.Apply(op, left, right);

    /// <summary>Element-wise array/vector arithmetic with scalar broadcast and whole-array promotion.</summary>
    private static Value EvaluateArrayBinary(BinaryOp op, Value left, Value right)
    {
        if (IsArrayLike(left) && IsArrayLike(right))
        {
            var a = left.AsArrayValue();
            var b = right.AsArrayValue();

            if (a.Rank == b.Rank && a.Shape.Span.SequenceEqual(b.Shape.Span))
            {
                var results = new List<Value>(checked((int)a.Numel));
                for (long i = 0; i < a.Numel; i++)
                    results.Add(ApplyScalarBinary(op, (Value)a.GetElement(i), (Value)b.GetElement(i)));
                return WrapPromotedArray(op, results, a.Shape.ToArray());
            }

            return EvaluateBroadcastBinary(op, a, b);
        }

        if (IsArrayLike(left))
        {
            var a = left.AsArrayValue();
            var results = new List<Value>(checked((int)a.Numel));
            for (long i = 0; i < a.Numel; i++)
                results.Add(ApplyScalarBinary(op, (Value)a.GetElement(i), right));
            return WrapPromotedArray(op, results, a.Shape.ToArray());
        }

        var b2 = right.AsArrayValue();
        var results2 = new List<Value>(checked((int)b2.Numel));
        for (long i = 0; i < b2.Numel; i++)
            results2.Add(ApplyScalarBinary(op, left, (Value)b2.GetElement(i)));
        return WrapPromotedArray(op, results2, b2.Shape.ToArray());
    }

    /// <summary>Right-aligned N-dimensional broadcast elementwise op (BDC-001).</summary>
    private static Value EvaluateBroadcastBinary(BinaryOp op, ArrayValue a, ArrayValue b)
    {
        int maxRank = Math.Max(a.Rank, b.Rank);
        var aShape = a.Shape.ToArray();
        var bShape = b.Shape.ToArray();
        var aAligned = new long[maxRank];
        var bAligned = new long[maxRank];
        for (int d = 0; d < maxRank; d++)
        {
            aAligned[d] = d < maxRank - a.Rank ? 1 : aShape[d - (maxRank - a.Rank)];
            bAligned[d] = d < maxRank - b.Rank ? 1 : bShape[d - (maxRank - b.Rank)];
        }

        var resultShape = new long[maxRank];
        for (int d = 0; d < maxRank; d++)
        {
            long sa = aAligned[d];
            long sb = bAligned[d];
            if (sa == sb) resultShape[d] = sa;
            else if (sa == 1) resultShape[d] = sb;
            else if (sb == 1) resultShape[d] = sa;
            else throw new InvalidOperationException(
                $"Operands could not be broadcast together with shapes [{string.Join(", ", aShape)}] and [{string.Join(", ", bShape)}].");
        }

        long numel = 1;
        foreach (var d in resultShape)
            numel = checked(numel * d);

        var results = new List<Value>(checked((int)numel));
        var coords = new long[maxRank];
        var aCoords = new long[a.Rank];
        var bCoords = new long[b.Rank];
        for (long flat = 0; flat < numel; flat++)
        {
            long rem = flat;
            for (int d = maxRank - 1; d >= 0; d--)
            {
                coords[d] = rem % resultShape[d];
                rem /= resultShape[d];
            }

            for (int d = 0; d < a.Rank; d++)
            {
                int rd = maxRank - a.Rank + d;
                aCoords[d] = aAligned[rd] == 1 ? 0 : coords[rd];
            }
            for (int d = 0; d < b.Rank; d++)
            {
                int rd = maxRank - b.Rank + d;
                bCoords[d] = bAligned[rd] == 1 ? 0 : coords[rd];
            }

            results.Add(ApplyScalarBinary(op,
                (Value)a.GetElement(aCoords.AsSpan(0, a.Rank)),
                (Value)b.GetElement(bCoords.AsSpan(0, b.Rank))));
        }
        return WrapPromotedArray(op, results, resultShape);
    }

    private static bool IsArrayLike(Value v) => v.Kind is ValueKind.Array or ValueKind.Vector;

    /// <summary>Applies the whole-array promotion rule (D1/D2) and wraps as Vector (rank 1) or Array (rank ≥ 2).</summary>
    private static Value WrapPromotedArray(BinaryOp op, IReadOnlyList<Value> results, long[] shape)
    {
        var promoted = op == BinaryOp.Divide ? PromoteDivision(results) : PromoteNumeric(results);
        var av = TypedArrayAdapter.FromValues(promoted, shape);
        return new Value(av, shape.Length == 1 ? ValueKind.Vector : ValueKind.Array);
    }

    /// <summary>Division narrow-back (D2): narrow to Natural/Integer when every quotient is an exact integer.</summary>
    private static IReadOnlyList<Value> PromoteDivision(IReadOnlyList<Value> results)
    {
        bool allExactInteger = true;
        bool allNonNegative = true;

        foreach (var r in results)
        {
            switch (r.Kind)
            {
                case ValueKind.Natural:
                    break;
                case ValueKind.Integer:
                    if (r.AsInteger().Sign < 0) allNonNegative = false;
                    break;
                case ValueKind.Real:
                    if (Rl.IsInteger(r.AsReal()))
                    {
                        if (r.AsReal().Sign < 0) allNonNegative = false;
                    }
                    else
                    {
                        allExactInteger = false;
                    }
                    break;
                default:
                    return results;
            }
            if (!allExactInteger) break;
        }

        if (!allExactInteger)
            return PromoteNumeric(results);

        var target = allNonNegative ? ValueKind.Natural : ValueKind.Integer;
        var narrowed = new List<Value>(results.Count);
        foreach (var r in results)
        {
            narrowed.Add(r.Kind switch
            {
                ValueKind.Integer => allNonNegative ? new Value(r.AsInteger().ToNatural()) : r,
                ValueKind.Real => NarrowReal(r.AsReal(), target),
                _ => r,
            });
        }
        return narrowed;
    }

    /// <summary>Narrows an exact-integer Real to Natural (non-negative) or Integer (signed).</summary>
    private static Value NarrowReal(Rl r, ValueKind target)
    {
        var magnitude = r.ToNatural() * new Nat(10UL).Pow(new Nat((ulong)r.Exponent));
        return target == ValueKind.Natural
            ? new Value(magnitude)
            : new Value(new Int(magnitude, Int.IsNegative(r)));
    }

    private static Value EvaluateComparison(Value left, Value right, BinaryOp op)
    {
        if (left.Kind == ValueKind.Symbolic || right.Kind == ValueKind.Symbolic)
            return NumericOps.Apply(op, left, right);

        int cmp = NumericOps.Compare(left, right);

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
                ValueKind.Symbolic => NumericOps.Negate(operand),
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
                ValueKind.Natural => new Value(operand.AsNatural().Factorial(SubProgress("factorial"))),
                ValueKind.Integer => new Value(operand.AsInteger().Factorial(SubProgress("factorial"))),
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

        var specs = new List<IndexSpec>(idx.Indices.Count);
        foreach (var ie in idx.Indices)
        {
            if (ie is SliceExpr se)
            {
                long? start = se.Start is null ? null : ToLong(await EvaluateAsync(se.Start, scope));
                long? stop = se.Stop is null ? null : ToLong(await EvaluateAsync(se.Stop, scope));
                long? step = se.Step is null ? null : ToLong(await EvaluateAsync(se.Step, scope));
                specs.Add(IndexSpec.Range(start, stop, step));
            }
            else
            {
                var iv = await EvaluateAsync(ie, scope);
                specs.Add(IndexSpec.Scalar(ToLong(iv)));
            }
        }

        return IndexValue(target, specs);
    }

    /// <summary>Member access on structured record results (e.g. <c>r.solutions</c>).</summary>
    private async Task<Value> EvaluateMemberAsync(MemberExpr member, Scope scope)
    {
        var target = await EvaluateAsync(member.Target, scope);
        if (target.Kind != ValueKind.Record)
        {
            string hint = target.Kind == ValueKind.Symbolic
                ? " Symbolic expressions have no members; call the *_full builtin (e.g. solve_full) for structured results."
                : string.Empty;
            throw new InvalidOperationException(
                $"member '{member.MemberName}' is not available on type '{target.Kind}': member access requires a record result.{hint}");
        }
        var record = target.AsRecord();
        if (record.TryGetField(member.MemberName, out var field))
            return (Value)field!;
        var names = string.Join(", ", record.Fields.Select(f => f.Name));
        throw new InvalidOperationException(
            $"record '{record.TypeName}' has no member '{member.MemberName}'. Available members: {names}.");
    }

    /// <summary>Indexes a vector or N-D array with scalar coordinates and/or slices.</summary>
    private static Value IndexValue(Value target, IReadOnlyList<IndexSpec> specs)
    {
        if (!IsArrayLike(target))
            throw new InvalidOperationException($"Indexing is not supported for type '{target.Kind}'.");

        var arr = target.AsArrayValue();

        // All-scalar index: existing element / partial sub-array semantics.
        if (specs.All(s => s.Index is not null))
        {
            var indices = specs.Select(s => s.Index!.Value).ToList();

            if (arr.Rank == 1)
            {
                if (indices.Count != 1)
                    throw new InvalidOperationException($"Vector indexing expects exactly 1 index, but got {indices.Count}.");
                long i = indices[0];
                if (i < 0 || i >= arr.Numel)
                    throw new InvalidOperationException($"Index {i} is out of range for vector of length {arr.Numel}.");
                return (Value)arr.GetElement(i);
            }

            if (indices.Count > arr.Rank)
                throw new InvalidOperationException($"A rank-{arr.Rank} array cannot be indexed with {indices.Count} indices.");

            if (indices.Count == arr.Rank)
                return (Value)arr.GetElement(indices.ToArray());

            // Partial index: zero-copy view over the trailing dimensions.
            return WrapArrayValue(arr.Slice(indices.Select(IndexSpec.Scalar).ToList()));
        }

        // Slice path: zero-copy strided view.
        return WrapArrayValue(arr.Slice(specs));
    }

    private async Task<Value> EvaluateListAsync(ListExpr list, Scope scope)
    {
        var elements = new List<Value>(list.Elements.Count);
        foreach (var e in list.Elements)
            elements.Add(await EvaluateAsync(e, scope));
        return BuildList(elements);
    }

    /// <summary>
    /// Wraps evaluated list elements as a Vector (rank 1) or Array (rank ≥ 2). Numeric
    /// elements are promoted to the max numeric kind so every literal is homogeneous (D3).
    /// </summary>
    private static Value BuildList(IReadOnlyList<Value> elements)
    {
        if (elements.Count > 0 && elements.All(IsContainer))
        {
            long[] firstShape = ShapeOf(elements[0]);
            if (!elements.All(e => ShapeOf(e).SequenceEqual(firstShape)))
                throw new InvalidOperationException(
                    "Ragged nested list literal: every row must have the same shape.");

            long[] shape = new long[firstShape.Length + 1];
            shape[0] = elements.Count;
            for (int i = 0; i < firstShape.Length; i++)
                shape[i + 1] = firstShape[i];

            var data = new List<Value>();
            foreach (var e in elements)
                data.AddRange(ContainerData(e));

            return new Value(TypedArrayAdapter.FromValues(PromoteNumeric(data), shape));
        }

        return new Value(PromoteNumeric(elements));
    }

    /// <summary>Widens a numeric list to its max numeric kind (D3); non-numeric lists stay heterogeneous.</summary>
    private static IReadOnlyList<Value> PromoteNumeric(IReadOnlyList<Value> elements)
    {
        ValueKind? max = null;
        foreach (var e in elements)
        {
            if (e.Kind is not (ValueKind.Natural or ValueKind.Integer or ValueKind.Real))
                return elements;
            max = max is null ? e.Kind : (ValueKind)Math.Max((int)max.Value, (int)e.Kind);
        }

        if (max is null || max == ValueKind.Natural)
            return elements;

        var widened = new List<Value>(elements.Count);
        foreach (var e in elements)
            widened.Add(e.Widen(max.Value));
        return widened;
    }

    private static bool IsContainer(Value v) => v.Kind is ValueKind.Vector or ValueKind.Array;

    private static long[] ShapeOf(Value v) =>
        v.Kind == ValueKind.Vector ? new[] { (long)v.AsVector().Count } : v.AsArray().Shape;

    private static IEnumerable<Value> ContainerData(Value v) =>
        v.Kind == ValueKind.Vector ? v.AsVector() : v.AsArray().Data;

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

    /// <summary>Flattens an array value to plugin payloads (row-major) with its shape — the
    /// <see cref="Lovelace.Abstractions.ISymbolicMatrixBridge"/> argument convention.</summary>
    private static (IReadOnlyList<object?> Elements, long[] Shape) PayloadElements(ArrayValue av)
    {
        var elements = TypedArrayAdapter.ToElements(av);
        var payloads = new object?[elements.Count];
        for (int i = 0; i < elements.Count; i++)
            payloads[i] = PayloadMap.Unwrap(elements[i]);
        return (payloads, av.Shape.ToArray());
    }

    /// <summary>
    /// Builds a matrix result record in the SAME vocabulary as every other solve-shaped record —
    /// <c>status</c> as an Enum of the declared type <c>SolveStatus</c>, the derived
    /// <c>complete</c> Boolean, the <c>completeness</c> Enum, the payload, the <c>conditions</c>,
    /// and the Diagnostic ARRAY last. The (complete, completeness) pair comes from the ONE Round-5
    /// mapping <see cref="Lovelace.Symbolics.SolveCompletenessMapping"/> that SolveResult and
    /// SystemSolveResult publish, so a singular system reports NoSolutions/true/Complete exactly as
    /// the scalar solver does. Diagnostics are built from the Round-4b vocabulary: an EMPTY array
    /// when nothing is wrong — never an empty string and never Null.
    /// </summary>
    private static RecordValue MatrixResultRecord(
        string typeName, string payloadField, SolveStatus status,
        object? payload, object? conditions, IReadOnlyList<WireDiagnostic> diagnostics)
    {
        var (complete, completeness) = Lovelace.Symbolics.SolveCompletenessMapping.Of(status);
        return new RecordValue(typeName,
            new RecordField("status", new EnumValue("SolveStatus", status.ToString())),
            new RecordField("complete", complete),
            new RecordField("completeness", new EnumValue("Completeness", completeness.ToString())),
            new RecordField(payloadField, PayloadMap.Wrap(payload)),
            new RecordField("conditions", PayloadMap.Wrap(conditions)),
            new RecordField("diagnostics", DiagnosticProjection.ToRecordValues(diagnostics)));
    }

    /// <summary>The ONE diagnostic a singular matrix carries: a stable code a consumer matches,
    /// the ErrorCategory member, the human sentence as the MESSAGE (never as free text beside the
    /// record), no source location at this layer (Null, never "") and empty details.</summary>
    private static WireDiagnostic[] SingularMatrixDiagnostics(string message) =>
        new[] { WireDiagnostic.Of("matrix.singular", ErrorCategory.NoSolution, message) };

    /// <summary>
    /// The one type-name convention: the <see cref="ValueKind"/> name, except that a record
    /// reports its record type name (<c>SolveResult</c>) and an enum reports its declared enum
    /// type name (<c>SolveStatus</c>) — because that is what a consumer switches on, and the
    /// transport kind ("Enum") would only say how the value travels. A domain value is the kind
    /// <c>Domain</c> — its domain (<c>real</c>, <c>complex</c>) is available from
    /// <c>inspect(...).domain</c>, not from <c>type()</c>. Never a lowercased domain name:
    /// <c>type(complex)</c> is <c>Domain</c>, not <c>complex</c>. So
    /// <c>type(solve_full(x^2 - 4 == 0, x).status)</c> is <c>SolveStatus</c>.
    /// </summary>
    private static string TypeNameOf(Value v) => v.Kind switch
    {
        ValueKind.Record => v.AsRecord().TypeName,
        ValueKind.Enum => v.AsEnum().TypeName,
        _ => v.Kind.ToString(),
    };

    /// <summary>
    /// Builds the structural <c>Inspection</c> record. The shape is uniform across kinds — every
    /// field is present, with <c>Null</c> where it does not apply — so a consumer never has to
    /// probe which fields a kind happens to carry. <c>domain</c> is a Domain value (never text),
    /// <c>canonical</c>/<c>pretty</c> are both forms, arrays report <c>shape</c>/<c>rank</c>, and
    /// <c>assumptions</c> carries the structured conditions that constrain the free symbols.
    /// </summary>
    private RecordValue Inspect(Value v)
    {
        var fields = new List<RecordField>
        {
            new("type", new Value(TypeNameOf(v))),
            new("domain", DomainValueOf(v)),
            new("exact", ExactOf(v)),
            new("free_symbols", new Value(FreeSymbolsOf(v))),
            new("node_count", NodeCountOf(v)),
            new("canonical", CanonicalOf(v)),
            new("pretty", PrettyOf(v)),
            new("shape", InspectionShapeOf(v)),
            new("rank", RankOf(v)),
            new("element_domain", ElementDomainOf(v)),
            new("assumptions", new Value(RelevantAssumptions(v))),
            new("members", MembersOf(v)),
        };
        return new RecordValue("Inspection", fields.ToArray());
    }

    private static Value DomainValueOf(Value v) => v.Kind switch
    {
        ValueKind.Domain => v,
        ValueKind.Symbolic => new Value(Lovelace.Symbolics.Domains.ToMathDomain(
            Lovelace.Symbolics.Domains.DomainOf(v.AsSymbolic(), Lovelace.Symbolics.Exprs.Current))),
        _ => Value.Void,
    };

    private static Value ExactOf(Value v) => v.Kind switch
    {
        ValueKind.Symbolic => new Value(v.AsSymbolic().IsExact),
        ValueKind.Natural or ValueKind.Integer => new Value(true),
        _ => Value.Void,
    };

    private static Value[] FreeSymbolsOf(Value v)
    {
        if (v.Kind != ValueKind.Symbolic)
            return Array.Empty<Value>();
        var names = new List<string>();
        CollectSymbols(v.AsSymbolic(), names);
        return names.Distinct().Select(n => new Value(n)).ToArray();
    }

    private static Value NodeCountOf(Value v) => v.Kind == ValueKind.Symbolic
        ? new Value(new global::Lovelace.Natural.Natural(v.AsSymbolic().NodeCount))
        : Value.Void;

    private static Value CanonicalOf(Value v) => v.Kind == ValueKind.Symbolic
        ? new Value(Lovelace.Symbolics.Printing.CanonicalPrint(v.AsSymbolic()))
        : Value.Void;

    private static Value PrettyOf(Value v) => v.Kind == ValueKind.Symbolic
        ? new Value(Lovelace.Symbolics.Printing.PrettyPrint(v.AsSymbolic()))
        : Value.Void;

    private static Value InspectionShapeOf(Value v) => v.Kind is ValueKind.Vector or ValueKind.Array
        ? new Value(v.AsArrayValue().Shape.ToArray()
            .Select(s => new Value(new global::Lovelace.Natural.Natural((ulong)s))).ToArray())
        : Value.Void;

    private static Value RankOf(Value v) => v.Kind is ValueKind.Vector or ValueKind.Array
        ? new Value(new global::Lovelace.Natural.Natural(v.AsArrayValue().Rank))
        : Value.Void;

    /// <summary>The domain every element shares (arrays) or the expression's own domain.</summary>
    private static Value ElementDomainOf(Value v)
    {
        if (v.Kind is ValueKind.Vector or ValueKind.Array)
        {
            var elements = v.AsVector();
            if (elements.Count == 0)
                return Value.Void;
            var first = elements[0];
            return first.Kind switch
            {
                ValueKind.Symbolic => new Value(Lovelace.Symbolics.Domains.ToMathDomain(
                    Lovelace.Symbolics.Domains.DomainOf(first.AsSymbolic(), Lovelace.Symbolics.Exprs.Current))),
                ValueKind.Natural or ValueKind.Integer => new Value(
                    Lovelace.Abstractions.MathDomain.Integer),
                ValueKind.Real => new Value(Lovelace.Abstractions.MathDomain.Real),
                ValueKind.Complex => new Value(Lovelace.Abstractions.MathDomain.Complex),
                _ => Value.Void,
            };
        }
        return DomainValueOf(v);
    }

    private Value[] RelevantAssumptions(Value v)
    {
        if (v.Kind != ValueKind.Symbolic || SymbolicInspectionBridge is not { } bridge)
            return Array.Empty<Value>();
        var leaves = bridge.RelevantAssumptions(v.AsSymbolic());
        var wrapped = new Value[leaves.Length];
        for (int i = 0; i < leaves.Length; i++)
            wrapped[i] = PayloadMap.Wrap(leaves[i]);
        return wrapped;
    }

    private static Value MembersOf(Value v) => v.Kind == ValueKind.Record
        ? new Value(v.AsRecord().Fields.Select(f => new Value(f.Name)).ToArray())
        : Value.Void;

    private static void CollectSymbols(Lovelace.Symbolics.Expr e, List<string> into)
    {
        switch (e)
        {
            case Lovelace.Symbolics.SymbolExpr s:
                into.Add(s.Symbol.Name);
                break;
            case Lovelace.Symbolics.AddExpr a:
                foreach (var t in a.Terms) CollectSymbols(t, into);
                break;
            case Lovelace.Symbolics.MultiplyExpr m:
                foreach (var f in m.Factors) CollectSymbols(f, into);
                break;
            case Lovelace.Symbolics.PowerExpr p:
                CollectSymbols(p.Base, into);
                CollectSymbols(p.Exponent, into);
                break;
            case Lovelace.Symbolics.FunctionExpr f:
                foreach (var arg in f.Arguments) CollectSymbols(arg, into);
                break;
            case Lovelace.Symbolics.RelationExpr r:
                CollectSymbols(r.Left, into);
                CollectSymbols(r.Right, into);
                break;
            case Lovelace.Symbolics.PiecewiseExpr pw:
                foreach (var b in pw.Branches)
                {
                    CollectSymbols(b.Guard, into);
                    CollectSymbols(b.Value, into);
                }
                CollectSymbols(pw.Otherwise, into);
                break;
            case Lovelace.Symbolics.DerivativeExpr d:
                CollectSymbols(d.Operand, into);
                break;
            case Lovelace.Symbolics.IntegralExpr i:
                CollectSymbols(i.Operand, into);
                break;
            case Lovelace.Symbolics.AndExpr an:
                foreach (var o in an.Operands) CollectSymbols(o, into);
                break;
            case Lovelace.Symbolics.OrExpr or2:
                foreach (var o in or2.Operands) CollectSymbols(o, into);
                break;
            case Lovelace.Symbolics.NotExpr nt:
                CollectSymbols(nt.Operand, into);
                break;
            case Lovelace.Symbolics.OrderExpr o:
                CollectSymbols(o.Variable, into);
                break;
        }
    }

    private void Register(string name, IReadOnlyList<string> parameters, BuiltinFunction impl) =>
        _functions[name] = new FunctionDefinition(name, parameters, impl);

    private static void RequireArity(string name, IReadOnlyList<Value> args, int expected)
    {
        if (args.Count != expected)
            throw new InvalidOperationException($"{name}() expects exactly {expected} argument(s), but got {args.Count}.");
    }

    /// <summary>Elementwise magnitude for complex arrays (the magnitude-spectrum idiom).</summary>
    private static Value AbsArray(ArrayValue array)
    {
        if (array.DType != DType.Complex)
            throw new InvalidOperationException(
                $"abs() on arrays requires a Complex array (got DType.{array.DType}); use abs() on individual scalars for Natural/Integer/Real.");
        var elements = TypedArrayAdapter.ToElements(array);
        var magnitudes = new Value[elements.Count];
        for (int i = 0; i < elements.Count; i++)
            magnitudes[i] = new Value(elements[i].AsComplex().Magnitude);
        return WrapArrayValue(TypedArrayAdapter.FromElements(magnitudes));
    }

    private void RegisterBuiltins()
    {
        // type(x): the value-kind name; structured records report their type name
        Register("type", ["x"], args =>
        {
            RequireArity("type", args, 1);
            return Task.FromResult<Value>(new Value(TypeNameOf(args[0])));
        });

        // inspect(x): structural introspection as a record (machine-readable — no prose parsing)
        Register("inspect", ["x"], args =>
        {
            RequireArity("inspect", args, 1);
            return Task.FromResult<Value>(new Value(Inspect(args[0])));
        });

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
                ValueKind.Complex => new Value(arg.AsComplex().Magnitude),
                ValueKind.Vector or ValueKind.Array => AbsArray(arg.AsArrayValue()),
                // Round 23: a value the engine CREATES must be expressible back into it. The
                // rewriter emits abs(x) as a FunctionExpr over the registered "abs" kernel
                // function (Lovelace.Symbolics/Simplify.cs:251, :321, :335 — there is no
                // Absolute node kind), so the symbolic payload builds that very node instead of
                // being rejected. Mirrors the sqrt() symbolic arm below (Exprs.Power): the
                // kernel's own constructor, no wrapper node, no new NodeKind.
                ValueKind.Symbolic => new Value(Lovelace.Symbolics.Exprs.Function(
                    Lovelace.Symbolics.Exprs.Current.Function("abs"), arg.AsSymbolic())),
                _ => throw new InvalidOperationException($"abs() is not supported for values of kind '{arg.Kind}'."),
            });
        });

        // inv(x) / inv(matrix)
        Register("inv", ["x"], args =>
        {
            RequireArity("inv", args, 1);
            var arg = args[0];
            if (arg.Kind == ValueKind.Array)
            {
                var av = arg.AsArrayValue();
                var (elements, shape) = PayloadElements(av);
                var bridge = SymbolicMatrixBridge;
                if (bridge is not null && bridge.IsSymbolicMatrix(elements, shape))
                {
                    var inv = bridge.TryInverse(elements, shape, out _);
                    if (inv is null)
                        throw new InvalidOperationException("Matrix is singular (det = 0).");
                    return Task.FromResult<Value>(PayloadMap.Wrap(inv));
                }
                return Task.FromResult(WrapArrayValue(TypedArrayOps.Inverse(av)));
            }
            var real = arg.Widen(ValueKind.Real).AsReal();
            return Task.FromResult<Value>(new Value(real.Invert()));
        });

        // inv_full(A): structured inverse with the det != 0 condition
        Register("inv_full", ["x"], args =>
        {
            RequireArity("inv_full", args, 1);
            var av = args[0].AsArrayValue();
            var (elements, shape) = PayloadElements(av);
            var bridge = SymbolicMatrixBridge;
            if (bridge is null || !bridge.IsSymbolicMatrix(elements, shape))
                throw new InvalidOperationException("inv_full() requires a symbolic matrix.");
            var inv = bridge.TryInverse(elements, shape, out var conditions);
            if (inv is null)
                // a singular matrix is a PROVABLY empty solution set, which the frozen contract (and
                // the Round-5 mapping) calls a complete answer — and the reason is a Diagnostic
                return Task.FromResult<Value>(new Value(MatrixResultRecord(
                    "MatrixInverseResult", "inverse", SolveStatus.NoSolutions,
                    Array.Empty<object?>(), Array.Empty<object?>(),
                    SingularMatrixDiagnostics("matrix is singular"))));
            return Task.FromResult<Value>(new Value(MatrixResultRecord(
                "MatrixInverseResult", "inverse", SolveStatus.Solved,
                inv, conditions ?? Array.Empty<object?>(),
                Array.Empty<WireDiagnostic>())));
        });

        // matrix_rank(A): generic rank of a symbolic matrix (an exact integer constant)
        Register("matrix_rank", ["a"], args =>
        {
            RequireArity("matrix_rank", args, 1);
            var av = args[0].AsArrayValue();
            var (elements, shape) = PayloadElements(av);
            var bridge = SymbolicMatrixBridge;
            if (bridge is not null && bridge.IsSymbolicMatrix(elements, shape))
            {
                var rank = bridge.TryRank(elements, shape);
                if (rank is int r)
                    return Task.FromResult<Value>(new Value(new Lovelace.Integer.Integer(r)));
            }
            throw new InvalidOperationException("matrix_rank() requires a symbolic matrix.");
        });

        // linsolve(A, b): exact linear system solve over a symbolic matrix (Bareiss),
        // valid under det(A) != 0 (the condition is implicit in the returned entries)
        Register("linsolve", ["a", "b"], args =>
        {
            RequireArity("linsolve", args, 2);
            var av = args[0].AsArrayValue();
            var bv = args[1].AsArrayValue();
            var (elements, shape) = PayloadElements(av);
            var bridge = SymbolicMatrixBridge;
            if (bridge is not null && bridge.IsSymbolicMatrix(elements, shape))
            {
                var solution = bridge.TrySolve(elements, shape, PayloadElements(bv).Elements, out _);
                if (solution is null)
                    throw new InvalidOperationException("System is singular or underdetermined.");
                return Task.FromResult<Value>(PayloadMap.Wrap(solution));
            }
            throw new InvalidOperationException("linsolve() requires a symbolic matrix A.");
        });

        // linsolve_full(A, b): structured linear solve with the det(A) != 0 condition
        Register("linsolve_full", ["a", "b"], args =>
        {
            RequireArity("linsolve_full", args, 2);
            var av = args[0].AsArrayValue();
            var bv = args[1].AsArrayValue();
            var (elements, shape) = PayloadElements(av);
            var bridge = SymbolicMatrixBridge;
            if (bridge is null || !bridge.IsSymbolicMatrix(elements, shape))
                throw new InvalidOperationException("linsolve_full() requires a symbolic matrix A.");
            var solution = bridge.TrySolveFull(elements, shape, PayloadElements(bv).Elements,
                out var conditions, out var note, out var singular);
            return Task.FromResult<Value>(new Value(MatrixResultRecord(
                "MatrixSolveResult", "solutions", singular ? SolveStatus.NoSolutions : SolveStatus.Solved,
                solution ?? Array.Empty<object?>(), conditions ?? Array.Empty<object?>(),
                // the kernel's human note moves INTO the diagnostic message; a solved system
                // reports an EMPTY diagnostics array
                singular
                    ? SingularMatrixDiagnostics(note ?? "matrix is singular")
                    : Array.Empty<WireDiagnostic>())));
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
            if (arg.Kind == ValueKind.Symbolic)
                return new Value(Lovelace.Symbolics.Exprs.Power(arg.AsSymbolic(), Lovelace.Symbolics.Exprs.Rational(1, 2)));
            var real = arg.Widen(ValueKind.Real).AsReal();
            return new Value(await Rl.SqrtAsync(real, SubProgress("sqrt")));
        });

        // pi() / pi(digits)
        Register("pi", ["digits"], async args =>
        {
            switch (args.Count)
            {
                case 0:
                    return new Value(Rl.Pi);

                case 1:
                {
                    var arg = args[0];
                    long digits = arg.Kind switch
                    {
                        ValueKind.Natural => long.Parse(arg.AsNatural().ToString(), CultureInfo.InvariantCulture),
                        ValueKind.Integer => long.Parse(arg.AsInteger().ToString(), CultureInfo.InvariantCulture),
                        _ => throw new InvalidOperationException($"pi() expects a Natural or Integer digit count, but got '{arg.Kind}'."),
                    };
                    return new Value(await Rl.PiToAsync(digits, SubProgress("pi")));
                }

                default:
                    throw new InvalidOperationException($"pi() expects 0 or 1 argument, but got {args.Count}.");
            }
        });

        // e() / e(digits)
        Register("e", ["digits"], async args =>
        {
            switch (args.Count)
            {
                case 0:
                    return new Value(Rl.E);

                case 1:
                {
                    var arg = args[0];
                    long digits = arg.Kind switch
                    {
                        ValueKind.Natural => long.Parse(arg.AsNatural().ToString(), CultureInfo.InvariantCulture),
                        ValueKind.Integer => long.Parse(arg.AsInteger().ToString(), CultureInfo.InvariantCulture),
                        _ => throw new InvalidOperationException($"e() expects a Natural or Integer digit count, but got '{arg.Kind}'."),
                    };
                    return new Value(await Rl.EToAsync(digits, SubProgress("e")));
                }

                default:
                    throw new InvalidOperationException($"e() expects 0 or 1 argument, but got {args.Count}.");
            }
        });

        // setprecision(n) — raise both the computation cap and the display precision.
        Register("setprecision", ["digits"], args =>
        {
            RequireArity("setprecision", args, 1);
            var arg = args[0];
            long n = arg.Kind switch
            {
                ValueKind.Natural => long.Parse(arg.AsNatural().ToString(), CultureInfo.InvariantCulture),
                ValueKind.Integer => long.Parse(arg.AsInteger().ToString(), CultureInfo.InvariantCulture),
                _ => throw new InvalidOperationException($"setprecision() expects a Natural or Integer digit count, but got '{arg.Kind}'."),
            };
            if (n <= 0)
                throw new InvalidOperationException($"setprecision() expects a positive digit count, but got {n}.");

            SetPrecision(n);
            return Task.FromResult(Value.Void);
        });

        // print(values...)
        Register("print", ["values"], args =>
        {
            Output.WriteLine(string.Join(" ", args.Select(v => ValueFormatter.Format(v, UnicodeOutput))));
            return Task.FromResult(Value.Void);
        });

        // len(v) / len(array)
        Register("len", ["v"], args =>
        {
            RequireArity("len", args, 1);
            var arg = args[0];
            return arg.Kind switch
            {
                ValueKind.Vector => Task.FromResult<Value>(new Value(new Nat(arg.AsVector().Count))),
                ValueKind.Array  => Task.FromResult<Value>(Natural(arg.AsArray().Shape[0])),
                _ => throw new InvalidOperationException($"len() expects a vector or array, but got '{arg.Kind}'."),
            };
        });

        // plot(...)
        Register("plot", ["x", "y", "title"], args => Task.FromResult(BuiltinPlot(args)));

        RegisterArrayBuiltins();
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
            series.Points.Add(new PlotPoint(PlotValue.ToReal(xv[i]), PlotValue.ToReal(yv[i])));
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
    // Array / vector built-in helpers
    // -----------------------------------------------------------------

    /// <summary>Wraps an <see cref="ArrayValue"/> as a Vector (rank 1) or Array (rank ≥ 2).</summary>
    private static Value WrapArrayValue(ArrayValue av) =>
        new Value(av, av.Rank == 1 ? ValueKind.Vector : ValueKind.Array);

    /// <summary>Builds a Natural-seeded homogeneous dense array of the given shape (D7; supports zero dims per D5).</summary>
    private static ArrayValue FillArrayValue(long[] shape, Value value)
    {
        long total = 1;
        foreach (var d in shape)
            total = checked(total * d);
        var buffer = new Value[checked((int)total)];
        Array.Fill(buffer, value);
        return new DenseArray<Value>(shape, buffer, DType.Natural, new Precision(0));
    }

    /// <summary>Removes size-1 dimensions (a shape of all singletons collapses to rank 1).</summary>
    private static ArrayValue SqueezeArrayValue(ArrayValue av)
    {
        var dims = av.Shape.ToArray().Where(d => d != 1).ToArray();
        if (dims.Length == 0)
            dims = new long[] { 1 };
        return av.Reshape(dims);
    }

    /// <summary>Builds a Natural value from a non-negative long.</summary>
    private static Value Natural(long n) => new Value(Nat.Parse(n.ToString(), null));

    /// <summary>Parses trailing arguments as dimension sizes.</summary>
    private static long[] ParseShape(IReadOnlyList<Value> args, int start, string name)
    {
        if (args.Count <= start)
            throw new InvalidOperationException($"{name}() requires at least one dimension.");
        var dims = new long[args.Count - start];
        for (int i = start; i < args.Count; i++)
            dims[i - start] = ToLong(args[i]);
        return dims;
    }

    /// <summary>Converts a vector of indices to a long array.</summary>
    private static long[] ToLongArray(Value v)
    {
        if (v.Kind != ValueKind.Vector)
            throw new InvalidOperationException("Expected a vector of axis indices.");
        return v.AsVector().Select(ToLong).ToArray();
    }

    /// <summary>Shared dispatcher for reduce-all (1 arg) vs reduce-along-axis (2 args) built-ins.</summary>
    private static Task<Value> ReduceBuiltin(
        IReadOnlyList<Value> args,
        Func<Value> empty,
        Func<ArrayValue, Value> all,
        Func<ArrayValue, long, ArrayValue> axis)
    {
        return args.Count switch
        {
            1 => Task.FromResult(ReduceAllOrEmpty(args[0], empty, all)),
            2 => Task.FromResult(ReduceAxisResult(args[0], ToLong(args[1]), axis)),
            _ => throw new InvalidOperationException($"Expected 1 or 2 arguments, but got {args.Count}."),
        };
    }

    /// <summary>Reduce-all with the D6 empty rule: an empty array yields the builtin's identity or error.</summary>
    private static Value ReduceAllOrEmpty(Value input, Func<Value> empty, Func<ArrayValue, Value> all)
    {
        var av = input.AsArrayValue();
        if (av.Numel == 0)
            return empty();
        return all(av);
    }

    private static Func<Value> EmptyReduceError(string name) =>
        () => throw new InvalidOperationException($"{name}() cannot reduce an empty array.");

    /// <summary>Reduces along an axis; a rank-1 input reduces to a scalar.</summary>
    private static Value ReduceAxisResult(Value input, long axis, Func<ArrayValue, long, ArrayValue> reduce)
    {
        var av = input.AsArrayValue();
        var result = reduce(av, axis);
        if (av.Rank == 1)
            return (Value)result.GetElement(0);
        return WrapArrayValue(result);
    }

    private void RegisterArrayBuiltins()
    {
        // zeros(d1, …, dn) — Natural-seeded, no dtype arg (D7); supports zero-length dims (D5)
        Register("zeros", ["dims"], args =>
            Task.FromResult(WrapArrayValue(FillArrayValue(ParseShape(args, 0, "zeros"), NumericOps.Zero))));

        // ones(d1, …, dn)
        Register("ones", ["dims"], args =>
            Task.FromResult(WrapArrayValue(FillArrayValue(ParseShape(args, 0, "ones"), NumericOps.One))));

        // eye(n) / eye(r, c)
        Register("eye", ["rows", "cols"], args =>
        {
            if (args.Count != 1 && args.Count != 2)
                throw new InvalidOperationException($"eye() expects 1 or 2 arguments, but got {args.Count}.");
            long rows = ToLong(args[0]);
            long cols = args.Count == 2 ? ToLong(args[1]) : rows;
            if (rows < 1 || cols < 1)
                throw new ArgumentException("eye() dimensions must be positive.");

            long total = rows * cols;
            var buffer = new Value[checked((int)total)];
            Array.Fill(buffer, NumericOps.Zero);
            for (long i = 0; i < Math.Min(rows, cols); i++)
                buffer[(int)(i * cols + i)] = NumericOps.One;
            return Task.FromResult(WrapArrayValue(TypedArrayAdapter.FromValues(buffer, new long[] { rows, cols })));
        });

        // reshape(a, d1, …, dn)
        Register("reshape", ["a", "dims"], args =>
        {
            if (args.Count < 2)
                throw new InvalidOperationException("reshape() requires an array and one or more dimensions.");
            var av = args[0].AsArrayValue();
            return Task.FromResult(WrapArrayValue(av.Reshape(ParseShape(args, 1, "reshape"))));
        });

        // shape(a)
        Register("shape", ["a"], args =>
        {
            RequireArity("shape", args, 1);
            return Task.FromResult<Value>(new Value(args[0].AsArrayValue().Shape.ToArray().Select(Natural).ToList()));
        });

        // rank(a) / ndims(a)
        Register("rank", ["a"], args =>
        {
            RequireArity("rank", args, 1);
            return Task.FromResult<Value>(Natural(args[0].AsArrayValue().Rank));
        });
        Register("ndims", ["a"], args =>
        {
            RequireArity("ndims", args, 1);
            return Task.FromResult<Value>(Natural(args[0].AsArrayValue().Rank));
        });

        // numel(a)
        Register("numel", ["a"], args =>
        {
            RequireArity("numel", args, 1);
            return Task.FromResult<Value>(Natural(args[0].AsArrayValue().Numel));
        });

        // flatten(a)
        Register("flatten", ["a"], args =>
        {
            RequireArity("flatten", args, 1);
            var av = args[0].AsArrayValue();
            return Task.FromResult(WrapArrayValue(av.Reshape(new[] { av.Numel })));
        });

        // transpose(a) / transpose(a, perm)
        Register("transpose", ["a", "perm"], args =>
        {
            var av = args[0].AsArrayValue();
            return args.Count switch
            {
                1 => Task.FromResult(WrapArrayValue(av.Transpose(null))),
                2 => Task.FromResult(WrapArrayValue(av.Transpose(ToLongArray(args[1])))),
                _ => throw new InvalidOperationException($"transpose() expects 1 or 2 arguments, but got {args.Count}."),
            };
        });

        // squeeze(a)
        Register("squeeze", ["a"], args =>
        {
            RequireArity("squeeze", args, 1);
            return Task.FromResult(WrapArrayValue(SqueezeArrayValue(args[0].AsArrayValue())));
        });

        // reductions: sum / prod / min / max / mean / norm (all + axis)
        Register("sum",  ["a", "axis"], args => ReduceBuiltin(args, () => NumericOps.Zero,     TypedArrayOps.SumAll,  TypedArrayOps.SumAxis));
        Register("prod", ["a", "axis"], args => ReduceBuiltin(args, () => NumericOps.One,      TypedArrayOps.ProdAll, TypedArrayOps.ProdAxis));
        Register("min",  ["a", "axis"], args => ReduceBuiltin(args, EmptyReduceError("min"),   TypedArrayOps.MinAll,  TypedArrayOps.MinAxis));
        Register("max",  ["a", "axis"], args => ReduceBuiltin(args, EmptyReduceError("max"),   TypedArrayOps.MaxAll,  TypedArrayOps.MaxAxis));
        Register("mean", ["a", "axis"], args => ReduceBuiltin(args, EmptyReduceError("mean"),  TypedArrayOps.MeanAll, TypedArrayOps.MeanAxis));
        Register("norm", ["a", "axis"], args => ReduceBuiltin(args, EmptyReduceError("norm"),  TypedArrayOps.NormAll, TypedArrayOps.NormAxis));

        // dot(a, b)
        Register("dot", ["a", "b"], args =>
        {
            RequireArity("dot", args, 2);
            return Task.FromResult<Value>(TypedArrayOps.Dot(args[0].AsArrayValue(), args[1].AsArrayValue()));
        });

        // cross(a, b)
        Register("cross", ["a", "b"], args =>
        {
            RequireArity("cross", args, 2);
            return Task.FromResult(WrapArrayValue(TypedArrayOps.Cross(args[0].AsArrayValue(), args[1].AsArrayValue())));
        });

        // matmul(a, b)
        Register("matmul", ["a", "b"], args =>
        {
            RequireArity("matmul", args, 2);
            var a = args[0].AsArrayValue();
            var b = args[1].AsArrayValue();
            if (a.Rank == 1 && b.Rank == 1)
                return Task.FromResult<Value>(TypedArrayOps.Dot(a, b));
            return Task.FromResult(WrapArrayValue(TypedArrayOps.MatMul(a, b)));
        });

        // det(m)
        Register("det", ["m"], args =>
        {
            RequireArity("det", args, 1);
            var av = args[0].AsArrayValue();
            var (elements, shape) = PayloadElements(av);
            var bridge = SymbolicMatrixBridge;
            if (bridge is not null && bridge.IsSymbolicMatrix(elements, shape))
            {
                var det = bridge.TryDet(elements, shape);
                if (det is not null)
                    return Task.FromResult<Value>(PayloadMap.Wrap(det));
            }
            return Task.FromResult<Value>(TypedArrayOps.Det(av));
        });

        // trace(m)
        Register("trace", ["m"], args =>
        {
            RequireArity("trace", args, 1);
            return Task.FromResult<Value>(TypedArrayOps.Trace(args[0].AsArrayValue()));
        });

        // concat(a, b) / concat(a, b, axis)
        Register("concat", ["a", "b", "axis"], args =>
        {
            if (args.Count != 2 && args.Count != 3)
                throw new InvalidOperationException($"concat() expects 2 or 3 arguments, but got {args.Count}.");
            long axis = args.Count == 3 ? ToLong(args[2]) : 0;
            return Task.FromResult(WrapArrayValue(TypedArrayOps.Concat(args[0].AsArrayValue(), args[1].AsArrayValue(), axis)));
        });

        // append(a, b) — vectors only
        Register("append", ["a", "b"], args =>
        {
            RequireArity("append", args, 2);
            var a = args[0].AsArrayValue();
            var b = args[1].AsArrayValue();
            if (a.Rank != 1 || b.Rank != 1)
                throw new InvalidOperationException("append() expects two vectors.");
            return Task.FromResult(WrapArrayValue(TypedArrayOps.Concat(a, b, 0)));
        });
    }

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
