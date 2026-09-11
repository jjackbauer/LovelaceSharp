using Lovelace.Abstractions;

namespace Lovelace.Suite;

/// <summary>
/// Discoverability metadata for the core (non-plugin) builtins. Plugin builtins carry their
/// own <see cref="BuiltinDescriptor"/>s at registration; this static table is the single
/// source of truth for the core set, consumed by <see cref="HelpService"/>, Studio
/// completions, and the DSH tool schema.
/// </summary>
public static class CoreBuiltinMetadata
{
    private static readonly Dictionary<string, BuiltinDescriptor> Table = new(StringComparer.Ordinal);

    static CoreBuiltinMetadata()
    {
        void Add(string name, string[] parameters, string category, string summary, string[] examples,
            string returnKind, string[]? related = null) =>
            Table[name] = new BuiltinDescriptor(name, parameters, category, summary, examples, returnKind, related);

        // ---- Introspection ----
        Add("type", ["x"], BuiltinCategories.Introspection,
            "The value-kind name of x; structured results report their record type name.",
            ["type(42)", "type(solve_full(x^2 - 4 == 0, x))"], "Text");
        Add("inspect", ["x"], BuiltinCategories.Introspection,
            "Structural introspection of x as a record (type, domain, exactness, free symbols, shape).",
            ["inspect(x^2 + 1)"], "Inspection");
        Add("print", ["values"], BuiltinCategories.Introspection,
            "Writes the rendered value(s) to the engine output.", ["print(\"hi\", 42)"], "Void");

        // ---- Numerics ----
        Add("abs", ["x"], BuiltinCategories.Numerics,
            "Absolute value (magnitude for complex values).", ["abs(-3)", "abs(fft([0, 1, 0, 0]))"], "Natural | Integer | Real");
        Add("sqrt", ["x"], BuiltinCategories.Numerics,
            "Principal square root; symbolic arguments produce Power(x, 1/2).", ["sqrt(4)", "sqrt(x)"], "Real | Symbolic");
        Add("sign", ["x"], BuiltinCategories.Numerics, "Sign of a real value: -1, 0, or 1.", ["sign(-7)"], "Integer");
        Add("divrem", ["a", "b"], BuiltinCategories.Numerics,
            "Quotient and remainder pair of integer division.", ["divrem(17, 5)"], "DivRemResult");
        Add("is_even", ["x"], BuiltinCategories.Numerics, "Whether the integer value is even.", ["is_even(4)"], "Boolean");
        Add("is_odd", ["x"], BuiltinCategories.Numerics, "Whether the integer value is odd.", ["is_odd(4)"], "Boolean");
        Add("pi", ["digits"], BuiltinCategories.Numerics,
            "π to the requested precision (or the engine default).", ["pi()", "pi(50)"], "Real", ["e"]);
        Add("e", ["digits"], BuiltinCategories.Numerics,
            "e to the requested precision (or the engine default).", ["e()"], "Real", ["pi"]);
        Add("setprecision", ["digits"], BuiltinCategories.Numerics,
            "Sets the engine computation and display precision (Real decimal places).", ["setprecision(100)"], "Void");

        // ---- Arrays / linear algebra ----
        Add("dot", ["a", "b"], BuiltinCategories.Arrays, "Dot product of two vectors.", ["dot([1, 2], [3, 4])"], "Natural | Integer | Real", ["cross", "matmul"]);
        Add("cross", ["a", "b"], BuiltinCategories.Arrays, "Cross product of two 3-vectors.", ["cross([1, 0, 0], [0, 1, 0])"], "Vector", ["dot"]);
        Add("matmul", ["a", "b"], BuiltinCategories.Arrays, "Matrix multiplication (also elementwise for same-shape arrays).", ["matmul([[1, 2], [3, 4]], [[5], [6]])"], "Vector | Array", ["dot"]);
        Add("det", ["m"], BuiltinCategories.LinearAlgebra, "Determinant (fraction-free for symbolic matrices).", ["det([[1, 2], [3, 4]])", "det([[x, 1], [y, x]])"], "Real | Symbolic", ["inv", "matrix_rank"]);
        Add("trace", ["m"], BuiltinCategories.LinearAlgebra, "Sum of the diagonal entries.", ["trace([[1, 2], [3, 4]])"], "Natural | Integer | Real");
        Add("inv", ["x"], BuiltinCategories.LinearAlgebra, "Reciprocal of a scalar or the inverse of a square matrix.", ["inv(2)", "inv([[1, 2], [3, 4]])"], "Real | Array", ["det", "linsolve"]);
        Add("matrix_rank", ["a"], BuiltinCategories.LinearAlgebra, "Exact rank of a symbolic matrix.", ["matrix_rank([[x, 1], [1, x]])"], "Integer", ["det", "linsolve"]);
        Add("linsolve", ["a", "b"], BuiltinCategories.LinearAlgebra, "Exact linear system solve over a symbolic matrix (Bareiss); valid under det(a) != 0.", ["linsolve([[x, 1], [1, x]], [5, 4])"], "Vector", ["matrix_rank", "solve_system", "linsolve_full"]);
        Add("linsolve_full", ["a", "b"], BuiltinCategories.LinearAlgebra, "Structured linear solve: a MatrixSolveResult with status, the solution vector, and the det(a) != 0 side condition.", ["linsolve_full([[x, 1], [1, x]], [5, 4])"], "MatrixSolveResult", ["linsolve", "inv_full"]);
        Add("inv_full", ["a"], BuiltinCategories.LinearAlgebra, "Structured matrix inverse: a MatrixInverseResult with status, the inverse, and the det(a) != 0 side condition.", ["inv_full([[x, 1], [1, x]])"], "MatrixInverseResult", ["inv", "linsolve_full"]);
        Add("ndims", ["a"], BuiltinCategories.Arrays, "Number of dimensions of an array (its rank).", ["ndims([[1, 2], [3, 4]])"], "Natural", ["shape", "rank"]);
        Add("zeros", ["dims"], BuiltinCategories.Arrays, "All-zero array of the given shape.", ["zeros(3)", "zeros(2, 2)"], "Vector | Array", ["ones", "eye"]);
        Add("ones", ["dims"], BuiltinCategories.Arrays, "All-one array of the given shape.", ["ones(3)"], "Vector | Array", ["zeros", "eye"]);
        Add("eye", ["rows", "cols"], BuiltinCategories.Arrays, "Identity matrix.", ["eye(3)"], "Array", ["zeros"]);
        Add("reshape", ["a", "dims"], BuiltinCategories.Arrays, "Reshape an array to new dimensions.", ["reshape([1, 2, 3, 4], 2, 2)"], "Vector | Array", ["flatten"]);
        Add("flatten", ["a"], BuiltinCategories.Arrays, "Flatten an N-D array to a rank-1 vector.", ["flatten([[1, 2], [3, 4]])"], "Vector", ["reshape"]);
        Add("transpose", ["a", "perm"], BuiltinCategories.Arrays, "Transpose (or permute the axes) of an array.", ["transpose([[1, 2], [3, 4]])"], "Vector | Array");
        Add("squeeze", ["a"], BuiltinCategories.Arrays, "Remove length-1 axes.", ["squeeze([[[1]]])"], "Vector | Array");
        Add("concat", ["a", "b", "axis"], BuiltinCategories.Arrays, "Concatenate arrays along an axis.", ["concat([1, 2], [3, 4])"], "Vector | Array", ["append"]);
        Add("append", ["a", "b"], BuiltinCategories.Arrays, "Append the values of one vector to another.", ["append([1, 2], [3])"], "Vector", ["concat"]);
        Add("sum", ["a", "axis"], BuiltinCategories.Arrays, "Sum of elements (optionally along an axis).", ["sum([1, 2, 3])"], "Natural | Integer | Real", ["prod", "mean"]);
        Add("prod", ["a", "axis"], BuiltinCategories.Arrays, "Product of elements (optionally along an axis).", ["prod([1, 2, 3])"], "Natural | Integer | Real", ["sum"]);
        Add("min", ["a", "axis"], BuiltinCategories.Arrays, "Minimum of elements (optionally along an axis).", ["min([3, 1, 2])"], "Natural | Integer | Real", ["max"]);
        Add("max", ["a", "axis"], BuiltinCategories.Arrays, "Maximum of elements (optionally along an axis).", ["max([3, 1, 2])"], "Natural | Integer | Real", ["min"]);
        Add("mean", ["a", "axis"], BuiltinCategories.Arrays, "Mean of elements (optionally along an axis).", ["mean([1, 2, 3])"], "Real", ["sum"]);
        Add("norm", ["a", "axis"], BuiltinCategories.Arrays, "Euclidean norm (optionally along an axis).", ["norm([3, 4])"], "Real");
        Add("shape", ["a"], BuiltinCategories.Arrays, "Dimensions of an array as a vector.", ["shape([[1, 2], [3, 4]])"], "Vector", ["rank", "numel"]);
        Add("rank", ["a"], BuiltinCategories.Arrays, "Number of array dimensions.", ["rank([[1, 2], [3, 4]])"], "Natural", ["shape"]);
        Add("numel", ["a"], BuiltinCategories.Arrays, "Total number of elements.", ["numel([[1, 2], [3, 4]])"], "Natural", ["shape"]);
        Add("len", ["v"], BuiltinCategories.Arrays, "Length of a vector (first dimension of an array).", ["len([1, 2, 3])"], "Natural", ["shape"]);

        // ---- Language / IO ----
        Add("plot", ["x", "y", "title"], BuiltinCategories.Language,
            "Renders a 2D line plot to an SVG file; returns the output path.", ["plot([1, 2, 3])", "plot([1, 2, 3], [1, 4, 9], \"title\")"], "Text");
    }

    /// <summary>Looks up the descriptor for a core builtin, or null when none is declared.</summary>
    public static BuiltinDescriptor? TryGet(string name) =>
        Table.TryGetValue(name, out var d) ? d : null;

    /// <summary>
    /// Every core descriptor, ordered by name. Unlike <see cref="TryGet"/> this does not depend
    /// on the function being registered, so consumers that must see the whole declared table —
    /// the descriptor doctests, generated documentation, tool schemas — enumerate here.
    /// </summary>
    public static IReadOnlyList<BuiltinDescriptor> Descriptors =>
        Table.OrderBy(kv => kv.Key, StringComparer.Ordinal).Select(kv => kv.Value).ToArray();
}
