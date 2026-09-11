using Lovelace.Abstractions;
using Cplx = global::Lovelace.Complex.Complex;
using Int = global::Lovelace.Integer.Integer;
using Nat = global::Lovelace.Natural.Natural;
using Rl = global::Lovelace.Real.Real;

namespace Lovelace.Suite;

/// <summary>
/// Hosts Modus plugins: holds the kernel registry, exposes fallible dispatch, and adapts
/// builtins to the interpreter. A plugin depends only on <c>Lovelace.Abstractions</c>; this
/// host is the only place that touches the interpreter, and it owns the <c>Value</c> ↔ payload
/// mapping for the general builtin channel (the core owns the mapping; plugins never see
/// <c>Value</c>).
/// </summary>
public sealed class ModusHost : IModusContext
{
    private readonly Interpreter _interpreter;
    private readonly List<object> _kernels = [];
    private readonly HashSet<Type> _loadedPlugins = [];
    private string? _loadingPluginName;

    internal ModusHost(Interpreter interpreter) => _interpreter = interpreter;

    public void Load(IModusPlugin plugin)
    {
        ArgumentNullException.ThrowIfNull(plugin);
        if (!_loadedPlugins.Add(plugin.GetType()))
            throw new InvalidOperationException($"Plugin '{plugin.Name}' ({plugin.GetType().Name}) is already loaded.");
        string? previous = _loadingPluginName;
        _loadingPluginName = plugin.Name;
        try
        {
            plugin.Register(this);
        }
        finally
        {
            _loadingPluginName = previous;
        }
    }

    /// <summary>Attributes a just-registered builtin to the plugin currently loading.</summary>
    private void StampPlugin(string name)
    {
        if (_loadingPluginName is not null && _interpreter.Functions.TryGetValue(name, out var definition))
            definition.PluginName = _loadingPluginName;
    }

    /// <summary>Computation budget for plugin builtins while the engine knob is untouched.</summary>
    private const long PluginStandardComputationPrecision = 30L;

    /// <summary>Display budget for plugin builtins while the engine knob is untouched.</summary>
    private const long PluginStandardDisplayPrecision = 15L;

    public void RegisterArrayBuiltin(string name, Func<ArrayValue, ArrayValue> implementation)
    {
        GuardName(name);
        _interpreter.RegisterBuiltin(name, new[] { "a" }, args =>
        {
            using var scope = PluginPrecisionScope();
            if (args.Count != 1)
                throw new InvalidOperationException($"{name}() expects exactly 1 argument, but got {args.Count}.");
            var result = implementation(args[0].AsArrayValue());
            return new Value(result, result.Rank == 1 ? ValueKind.Vector : ValueKind.Array);
        });
        StampPlugin(name);
    }

    public void RegisterBuiltin(string name, IReadOnlyList<string> parameters, Func<IReadOnlyList<object?>, object?> implementation)
    {
        ArgumentNullException.ThrowIfNull(name);
        ArgumentNullException.ThrowIfNull(parameters);
        ArgumentNullException.ThrowIfNull(implementation);
        GuardName(name);
        _interpreter.RegisterBuiltin(name, parameters, args =>
        {
            CheckArity(name, parameters, variadic: false, minArity: -1, args);
            using var scope = PluginPrecisionScope();
            return WrapResult(implementation(UnwrapArguments(args)));
        });
        StampPlugin(name);
    }

    /// <summary>Descriptor-carrying registration: the metadata is stored next to the function
    /// so help/funcs/Studio/DSH all derive from the same source of truth.</summary>
    public void RegisterBuiltin(BuiltinDescriptor descriptor, Func<IReadOnlyList<object?>, object?> implementation)
    {
        ArgumentNullException.ThrowIfNull(descriptor);
        ArgumentNullException.ThrowIfNull(implementation);
        GuardName(descriptor.Name);
        _interpreter.RegisterBuiltin(descriptor.Name, descriptor.Parameters, args =>
        {
            CheckArity(descriptor.Name, descriptor.Parameters, descriptor.Variadic, descriptor.MinArity, args);
            using var scope = PluginPrecisionScope();
            return WrapResult(implementation(UnwrapArguments(args)));
        }, descriptor);
        StampPlugin(descriptor.Name);
    }

    /// <summary>Descriptor-carrying registration over the typed ScalarResult channel.</summary>
    public void RegisterBuiltin(BuiltinDescriptor descriptor, Func<IReadOnlyList<object?>, ScalarResult> implementation)
    {
        ArgumentNullException.ThrowIfNull(descriptor);
        ArgumentNullException.ThrowIfNull(implementation);
        GuardName(descriptor.Name);
        _interpreter.RegisterBuiltin(descriptor.Name, descriptor.Parameters, args =>
        {
            CheckArity(descriptor.Name, descriptor.Parameters, descriptor.Variadic, descriptor.MinArity, args);
            using var scope = PluginPrecisionScope();
            return WrapResult(implementation(UnwrapArguments(args)).Payload);
        }, descriptor);
        StampPlugin(descriptor.Name);
    }

    /// <summary>
    /// Enforces the DECLARED arity once, before an implementation body can index an argument that
    /// was never supplied: <c>Parameters.Count</c> is the upper bound, <c>MinArity</c> the lower
    /// bound (<c>-1</c> means exactly the declared count) and <c>Variadic</c> removes the upper
    /// bound because the last declared parameter may repeat. A shorter call is legal only when the
    /// descriptor says so through <c>MinArity</c> — a metadata correction, never a name special
    /// case.
    /// </summary>
    private static void CheckArity(string name, IReadOnlyList<string> parameters, bool variadic,
        int minArity, IReadOnlyList<Value> args)
    {
        int declared = parameters.Count;
        int min = minArity >= 0 ? Math.Min(minArity, declared) : declared;
        int max = variadic ? int.MaxValue : declared;
        if (args.Count >= min && args.Count <= max)
            return;
        throw new BuiltinArityException(name, min, declared, variadic, args.Count);
    }

    private void GuardName(string name)
    {
        if (_interpreter.Functions.ContainsKey(name))
            throw new InvalidOperationException(
                $"A builtin named '{name}' is already registered; plugin builtins cannot shadow existing functions.");
    }

    /// <summary>
    /// Plugin builtins run at a fast default budget (30 computation / 15 display digits) while
    /// the engine's precision knob is untouched; the moment the user raises the knob (including
    /// explicitly to the default value), plugin builtins silently follow it. Promotion only ever
    /// increases precision — no plugin result is ever downgraded below what the engine asks for.
    /// </summary>
    private IDisposable? PluginPrecisionScope() =>
        _interpreter.PrecisionExplicitlySet
            ? null
            : Rl.WithPrecision(PluginStandardComputationPrecision, PluginStandardDisplayPrecision);

    public void RegisterKernel<T>(IFieldKernel<T> kernel) => _kernels.Add(kernel);

    /// <summary>Stores the symbolic-matrix bridge and hands it to the interpreter for
    /// inv/linsolve/matrix_rank/det dispatch (the D14 layering seam).</summary>
    public void RegisterSymbolicInspectionBridge(ISymbolicInspectionBridge bridge)
    {
        _interpreter.SymbolicInspectionBridge = bridge;
    }

    public void RegisterSymbolicMatrixBridge(ISymbolicMatrixBridge bridge)
    {
        ArgumentNullException.ThrowIfNull(bridge);
        _interpreter.SymbolicMatrixBridge = bridge;
    }

    /// <summary>
    /// Fallible dispatch: tries each registered kernel for <typeparamref name="T"/>, injecting
    /// the exact field identity (<see cref="Lovelace.Real.RealField"/> etc.). Returns
    /// <see langword="false"/> when no kernel handles the request, signalling the caller to
    /// run the reference backend.
    /// </summary>
    public bool TryDispatch<T>(ArrayOp op, ReadOnlySpan<T> left, ReadOnlySpan<T> right, Span<T> result)
    {
        IField<T>? field = FieldFor<T>();
        if (field is null)
            return false;   // no exact field identity for T — the reference backend owns it
        foreach (var k in _kernels)
        {
            if (k is IFieldKernel<T> kernel && kernel.TryElementwise(op, left, right, result, field))
                return true;
        }
        return false;
    }

    /// <summary>
    /// The exact field identity the core injects, selected by the element type. Static
    /// type comparisons only — AOT-safe, no reflection over plugin or field types.
    /// </summary>
    private static IField<T>? FieldFor<T>() =>
        typeof(T) == typeof(Rl) ? (IField<T>)(object)Lovelace.Real.RealField.Instance
        : typeof(T) == typeof(Int) ? (IField<T>)(object)Lovelace.Integer.IntegerField.Instance
        : typeof(T) == typeof(Nat) ? (IField<T>)(object)Lovelace.Natural.NaturalField.Instance
        : null;

    // -------------------------------------------------------------------------
    // Value ↔ payload mapping (the general builtin channel)
    // -------------------------------------------------------------------------

    private static IReadOnlyList<object?> UnwrapArguments(IReadOnlyList<Value> args)
    {
        var payloads = new object?[args.Count];
        for (int i = 0; i < args.Count; i++)
            payloads[i] = PayloadMap.Unwrap(args[i]);
        return payloads;
    }

    private static Value WrapResult(object? result) => PayloadMap.Wrap(result);
}

/// <summary>
/// A call-site arity failure: typed and structural rather than a framework index error. It derives
/// from <see cref="ArgumentException"/> so the runner's taxonomy classifies it as a RECOVERABLE
/// argument error (code <c>InvalidArgument</c>, category <c>TypeMismatch</c>) instead of an internal
/// invariant failure, and it names the builtin and both counts so the caller can fix the call
/// without parsing prose:
/// <c>compile_full(): expected 2 arguments; got 1.</c>
/// </summary>
public sealed class BuiltinArityException : ArgumentException
{
    public BuiltinArityException(string builtin, int expectedMin, int expectedMax, bool variadic, int actual)
        : base(Describe(builtin, expectedMin, expectedMax, variadic, actual))
    {
        Builtin = builtin;
        ExpectedMin = expectedMin;
        ExpectedMax = expectedMax;
        Variadic = variadic;
        Actual = actual;
    }

    /// <summary>The builtin whose declared arity the call violated.</summary>
    public string Builtin { get; }

    /// <summary>The declared minimum argument count.</summary>
    public int ExpectedMin { get; }

    /// <summary>The declared upper bound (<see cref="int.MaxValue"/> when <see cref="Variadic"/>).</summary>
    public int ExpectedMax { get; }

    /// <summary>True when the last declared parameter may repeat.</summary>
    public bool Variadic { get; }

    /// <summary>The argument count the call actually supplied.</summary>
    public int Actual { get; }

    private static string Describe(string builtin, int min, int max, bool variadic, int actual)
    {
        string expected = variadic
            ? $"at least {min} {Argument(min)}"
            : min == max
                ? $"{min} {Argument(min)}"
                : $"{min} to {max} arguments";
        return $"{builtin}(): expected {expected}; got {actual}.";
    }

    private static string Argument(int count) => count == 1 ? "argument" : "arguments";
}
