using Lovelace.Abstractions;
using Lovelace.Arrays;

namespace Lovelace.Suite;

/// <summary>
/// Temporary Stage-2/3 bridge between the legacy boxed <see cref="NdArray{T}"/> and the new
/// typed <see cref="ArrayValue"/>. <c>ValueKind.Vector</c> and <c>ValueKind.Array</c> both
/// store a <c>DenseArray&lt;Value&gt;</c>; this adapter materializes the legacy views for
/// the interpreter/builtin call sites that still consume them. Retired in Stage 6.
/// </summary>
internal static class TypedArrayAdapter
{
    /// <summary>Builds a <c>DenseArray&lt;Value&gt;</c> from flat data and an explicit shape.</summary>
    public static ArrayValue FromValues(IReadOnlyList<Value> data, long[] shape)
    {
        var buffer = new Value[data.Count];
        for (int i = 0; i < data.Count; i++)
            buffer[i] = data[i];

        // Precision is inert metadata until Stage 3 wires it to the process-global knobs.
        return new DenseArray<Value>(shape, buffer, InferDType(data), new Precision(0));
    }

    /// <summary>Builds a rank-1 <c>DenseArray&lt;Value&gt;</c> from a list of elements.</summary>
    public static ArrayValue FromElements(IReadOnlyList<Value> elements) =>
        FromValues(elements, new[] { (long)elements.Count });

    public static ArrayValue FromNdArray(NdArray<Value> nd) => FromValues(nd.Data, nd.Shape);

    /// <summary>Materializes the elements of an <see cref="ArrayValue"/> as a rank-1 list.</summary>
    public static IReadOnlyList<Value> ToElements(ArrayValue av)
    {
        var data = new Value[checked((int)av.Numel)];
        for (long i = 0; i < av.Numel; i++)
            data[i] = (Value)av.GetElement(i);
        return data;
    }

    /// <summary>Materializes an <see cref="ArrayValue"/> as the legacy <see cref="NdArray{T}"/> view.</summary>
    public static NdArray<Value> ToNdArray(ArrayValue av)
    {
        var data = new List<Value>(checked((int)av.Numel));
        for (long i = 0; i < av.Numel; i++)
            data.Add((Value)av.GetElement(i));
        return new NdArray<Value>(av.Shape.ToArray(), data);
    }

    /// <summary>The max numeric kind over the elements, per the D3 promotion lattice.</summary>
    private static DType InferDType(IReadOnlyList<Value> data)
    {
        var max = ValueKind.Natural;
        foreach (var v in data)
        {
            if (v.Kind == ValueKind.Real)
                return DType.Real;
            if (v.Kind == ValueKind.Integer && max == ValueKind.Natural)
                max = ValueKind.Integer;
        }
        return max switch
        {
            ValueKind.Real => DType.Real,
            ValueKind.Integer => DType.Integer,
            _ => DType.Natural,
        };
    }
}
