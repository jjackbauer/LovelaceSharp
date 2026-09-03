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
/// A pluggable elementwise kernel for a specific unmanaged element type. Returning
/// <see langword="false"/> declines the request so the dispatch falls back to the
/// reference backend.
/// </summary>
public interface IArrayKernel<T> where T : unmanaged
{
    DType DType { get; }

    bool TryElementwise(ArrayOp op, ReadOnlySpan<T> left, ReadOnlySpan<T> right, Span<T> result);
}

/// <summary>Registration surface a Modus plugin uses to extend the language.</summary>
public interface IModusContext
{
    /// <summary>Registers an array→array builtin callable from the language.</summary>
    void RegisterArrayBuiltin(string name, Func<ArrayValue, ArrayValue> implementation);

    /// <summary>Registers an optimized elementwise kernel.</summary>
    void RegisterKernel<T>(IArrayKernel<T> kernel) where T : unmanaged;
}

/// <summary>A Lovelace extension package: registers builtins and kernels.</summary>
public interface IModusPlugin
{
    string Name { get; }

    void Register(IModusContext context);
}
