using Lovelace.Abstractions;

namespace Lovelace.Suite;

/// <summary>
/// Hosts Modus plugins: holds the kernel registry, exposes fallible dispatch, and adapts
/// array→array builtins to the interpreter. A plugin depends only on
/// <c>Lovelace.Abstractions</c>; this host is the only place that touches the interpreter.
/// </summary>
public sealed class ModusHost : IModusContext
{
    private readonly Interpreter _interpreter;
    private readonly List<object> _kernels = [];

    internal ModusHost(Interpreter interpreter) => _interpreter = interpreter;

    public void Load(IModusPlugin plugin) => plugin.Register(this);

    public void RegisterArrayBuiltin(string name, Func<ArrayValue, ArrayValue> implementation)
    {
        _interpreter.RegisterBuiltin(name, new[] { "a" }, args =>
        {
            if (args.Count != 1)
                throw new InvalidOperationException($"{name}() expects exactly 1 argument, but got {args.Count}.");
            var result = implementation(args[0].AsArrayValue());
            return new Value(result, result.Rank == 1 ? ValueKind.Vector : ValueKind.Array);
        });
    }

    public void RegisterKernel<T>(IArrayKernel<T> kernel) where T : unmanaged => _kernels.Add(kernel);

    /// <summary>
    /// Fallible dispatch: tries each registered kernel for <typeparamref name="T"/>. Returns
    /// <see langword="false"/> when no kernel handles the request, signalling the caller to
    /// run the reference backend.
    /// </summary>
    public bool TryDispatch<T>(ArrayOp op, ReadOnlySpan<T> left, ReadOnlySpan<T> right, Span<T> result)
        where T : unmanaged
    {
        foreach (var k in _kernels)
        {
            if (k is IArrayKernel<T> kernel && kernel.TryElementwise(op, left, right, result))
                return true;
        }
        return false;
    }
}
