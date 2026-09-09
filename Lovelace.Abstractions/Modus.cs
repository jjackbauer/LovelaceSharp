namespace Lovelace.Abstractions;

/// <summary>Whole-array elementwise operations a kernel can implement.</summary>
public enum ArrayOp
{
    Add,
    Subtract,
    Multiply,
    Divide,
}

/// <summary>
/// A pluggable elementwise kernel for a concrete scalar type (<c>Natural</c>/<c>Integer</c>/
/// <c>Real</c> — reference types, so no <c>unmanaged</c> constraint). Returning
/// <see langword="false"/> declines the request so the dispatch falls back to the
/// reference backend. The field is injected by the core (the real
/// <c>RealField</c>/<c>IntegerField</c>/<c>NaturalField</c>, inheriting the active precision
/// scope); plugins never construct or select a field.
/// </summary>
public interface IFieldKernel<T>
{
    DType DType { get; }

    bool TryElementwise(ArrayOp op, ReadOnlySpan<T> left, ReadOnlySpan<T> right, Span<T> result, IField<T> field);
}

/// <summary>Registration surface a Modus plugin uses to extend the language.</summary>
public interface IModusContext
{
    /// <summary>Registers an array→array builtin callable from the language.</summary>
    void RegisterArrayBuiltin(string name, Func<ArrayValue, ArrayValue> implementation);

    /// <summary>
    /// Registers a general multi-argument builtin callable from the language. The core unwraps
    /// each argument before the call and re-wraps the result, so no <c>Value</c>/interpreter
    /// type crosses the boundary. Payload contract: scalar arguments arrive as their public
    /// types (<c>Natural</c>, <c>Integer</c>, <c>Real</c>, or <c>Complex</c>); vector/array
    /// arguments arrive as a flat list of those scalars (row-major). The implementation returns
    /// one of those scalars or an array of them (a list of scalars); the core wraps arrays as a
    /// Vector (rank 1) with an inferred dtype.
    /// </summary>
    void RegisterBuiltin(string name, IReadOnlyList<string> parameters,
                         Func<IReadOnlyList<object?>, object?> implementation);

    /// <summary>
    /// Registers a general multi-argument builtin whose implementation returns a typed
    /// <see cref="ScalarResult"/>. The default implementation unwraps the payload and forwards
    /// to the raw-object channel, so existing hosts get the wrapper without code changes.
    /// </summary>
    void RegisterBuiltin(string name, IReadOnlyList<string> parameters,
                         Func<IReadOnlyList<object?>, ScalarResult> implementation) =>
        RegisterBuiltin(name, parameters, args => implementation(args).Payload);

    /// <summary>
    /// Registers a builtin together with its discoverability metadata (help, funcs listings,
    /// Studio autocomplete, DSH tool schemas — one source of truth). Hosts that do not store
    /// descriptors keep the default behavior: the registration proceeds, metadata is dropped.
    /// </summary>
    void RegisterBuiltin(BuiltinDescriptor descriptor,
                         Func<IReadOnlyList<object?>, object?> implementation) =>
        RegisterBuiltin(descriptor.Name, descriptor.Parameters, implementation);

    /// <summary>Descriptor-carrying registration over the typed <see cref="ScalarResult"/>
    /// channel (mirrors the raw-object descriptor overload).</summary>
    void RegisterBuiltin(BuiltinDescriptor descriptor,
                         Func<IReadOnlyList<object?>, ScalarResult> implementation) =>
        RegisterBuiltin(descriptor, args => implementation(args).Payload);

    /// <summary>Registers an optimized elementwise kernel over an exact scalar type.</summary>
    void RegisterKernel<T>(IFieldKernel<T> kernel);

    /// <summary>
    /// Registers the symbolic-matrix bridge (determinant/rank/inverse/solve dispatch). The
    /// core falls back to non-symbolic behavior when no bridge is registered; hosts that do
    /// not store bridges keep the default no-op.
    /// </summary>
    void RegisterSymbolicMatrixBridge(ISymbolicMatrixBridge bridge) { }
}

/// <summary>
/// A plugin-produced result for the general builtin channel. The payload carries the channel's
/// documented vocabulary — Natural/Integer/Real/Complex scalars, a flat list of them, or an
/// <see cref="ArrayValue"/> — plus booleans and text; the core maps it onto a Value at the
/// boundary. The typed factories give the channel's return side a stable growth point without
/// breaking the raw-object overload. Novel *element* types still require a core bridge.
/// </summary>
public readonly struct ScalarResult
{
    private ScalarResult(object? payload) => Payload = payload;

    /// <summary>Wraps an arbitrary payload from the documented vocabulary.</summary>
    public static ScalarResult From(object? payload) => new(payload);

    /// <summary>Wraps a boolean result.</summary>
    public static ScalarResult FromBoolean(bool value) => new(value);

    /// <summary>Wraps a text result.</summary>
    public static ScalarResult FromText(string text) => new(text);

    /// <summary>Wraps a typed array result.</summary>
    public static ScalarResult FromArray(ArrayValue array) => new(array);

    /// <summary>The wrapped payload; the host maps it onto its value type.</summary>
    public object? Payload { get; }
}

/// <summary>A Lovelace extension package: registers builtins and kernels.</summary>
public interface IModusPlugin
{
    string Name { get; }

    void Register(IModusContext context);
}
