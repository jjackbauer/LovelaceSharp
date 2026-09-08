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
            using var scope = PluginPrecisionScope();
            return WrapResult(implementation(UnwrapArguments(args)));
        });
        StampPlugin(name);
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
            payloads[i] = Unwrap(args[i]);
        return payloads;
    }

    private static object? Unwrap(Value value) => value.Kind switch
    {
        ValueKind.Natural => value.AsNatural(),
        ValueKind.Integer => value.AsInteger(),
        ValueKind.Real => value.AsReal(),
        ValueKind.Complex => value.AsComplex(),
        ValueKind.Symbolic => value.AsSymbolic(),
        ValueKind.Boolean => value.AsBoolean(),
        ValueKind.Text => value.AsText(),
        ValueKind.Vector or ValueKind.Array => UnwrapArray(value.AsArrayValue()),
        _ => throw new InvalidOperationException($"A plugin builtin received an unsupported argument kind '{value.Kind}'."),
    };

    private static IReadOnlyList<object?> UnwrapArray(ArrayValue array)
    {
        var elements = TypedArrayAdapter.ToElements(array);
        var payloads = new object?[elements.Count];
        for (int i = 0; i < elements.Count; i++)
            payloads[i] = Unwrap(elements[i]);
        return payloads;
    }

    private static Value WrapResult(object? result) => result switch
    {
        // Real derives from Integer, so match the narrowest reference types first.
        Rl real => new Value(real),
        Int integer => new Value(integer),
        Nat natural => new Value(natural),
        Cplx complex => new Value(complex),
        Lovelace.Symbolics.Expr expr => new Value(expr),
        bool boolean => new Value(boolean),
        string text => new Value(text),
        ArrayValue array => new Value(array, array.Rank == 1 ? ValueKind.Vector : ValueKind.Array),
        IReadOnlyList<object?> elements => WrapArray(elements),
        _ => throw new InvalidOperationException($"A plugin builtin returned an unsupported result type '{result?.GetType().Name ?? "null"}'."),
    };

    private static Value WrapArray(IReadOnlyList<object?> elements)
    {
        var boxed = new Value[elements.Count];
        for (int i = 0; i < elements.Count; i++)
        {
            boxed[i] = elements[i] switch
            {
                Rl real => new Value(real),
                Int integer => new Value(integer),
                Nat natural => new Value(natural),
                Cplx complex => new Value(complex),
                Lovelace.Symbolics.Expr expr => new Value(expr),
                _ => throw new InvalidOperationException($"A plugin builtin returned an unsupported array element type '{elements[i]?.GetType().Name ?? "null"}'."),
            };
        }
        return new Value(boxed);
    }
}
