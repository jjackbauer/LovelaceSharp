using System.Collections;
using System.Collections.Immutable;
using Lovelace.Abstractions;
using Rat = global::Lovelace.Rational.Rational;
using Int = global::Lovelace.Integer.Integer;
using Nat = global::Lovelace.Natural.Natural;
using Rl = global::Lovelace.Real.Real;
using Cplx = global::Lovelace.Complex.Complex;

namespace Lovelace.Symbolics;

/// <summary>
/// The ONE projection from an effective <see cref="SolveStatus"/> onto the (complete,
/// completeness) pair the solver publishes. It lives in this file, next to the two record
/// builders that call it (<c>SolveResult</c> and <c>SystemSolveResult</c> are both constructed in
/// <see cref="SymbolicsPlugin"/>), so the two fields are always read off a single expression and
/// the two records cannot drift apart.
/// <para>
/// The frozen contract (alignment plan D) is that <see cref="SolveStatus.NoSolutions"/> means the
/// solution set over the requested domain is PROVABLY EMPTY: a provably empty set is a complete
/// answer, so it pairs with <c>complete: true</c> / <see cref="Completeness.Complete"/>. Only a
/// genuinely partial, unevaluated or budget-stopped solve is incomplete.
/// </para>
/// </summary>
public static class SolveCompletenessMapping
{
    /// <summary>The published pair for <paramref name="status"/>.
    /// <paramref name="partialSubset"/> is the kernel's own <see cref="Completeness"/> for the
    /// subset it did represent when a budget stopped the search; it is the answer for
    /// <see cref="SolveStatus.BudgetExceeded"/> and is ignored for every other status.</summary>
    public static (bool Complete, Completeness Completeness) Of(
        SolveStatus status, Completeness partialSubset = Completeness.Unknown) => status switch
    {
        SolveStatus.Solved => (true, Completeness.Complete),
        SolveStatus.NoSolutions => (true, Completeness.Complete),
        SolveStatus.Partial => (false, Completeness.Partial),
        SolveStatus.Unevaluated => (false, Completeness.Unknown),
        SolveStatus.BudgetExceeded => (false, partialSubset),
        _ => (false, Completeness.Unknown),
    };
}

/// <summary>
/// The language-facing surface of the symbolic kernel: a compile-time-linked Modus plugin
/// (the DspPlugin pattern) that registers the CAS builtins. Symbolic values cross the Modus
/// payload boundary as <see cref="Expr"/> objects via the Suite core bridge
/// (ValueKind.Symbolic).
/// </summary>
public sealed class SymbolicsPlugin : IModusPlugin, ISymbolicMatrixBridge, ISymbolicInspectionBridge
{
    public string Name => "Lovelace.Symbolics";

    public ExprContext Context { get; } = new();

    /// <summary>The elementary symbolic functions exposed one-argument builtins.</summary>
    internal static readonly string[] ElementaryFunctions =
        { "exp", "log", "sin", "cos", "tan", "asin", "acos", "atan",
          "sinh", "cosh", "tanh", "asinh", "acosh", "atanh" };

    private static string ElementarySummary(string fn) => fn switch
    {
        "exp" => "The exponential function, exp(x).",
        "log" => "The natural logarithm (principal branch).",
        "sin" => "The sine function.",
        "cos" => "The cosine function.",
        "tan" => "The tangent function.",
        "asin" => "The principal arcsine.",
        "acos" => "The principal arccosine.",
        "atan" => "The principal arctangent.",
        "sinh" => "The hyperbolic sine.",
        "cosh" => "The hyperbolic cosine.",
        "tanh" => "The hyperbolic tangent.",
        "asinh" => "The inverse hyperbolic sine, principal branch.",
        "acosh" => "The inverse hyperbolic cosine, principal branch (x >= 1).",
        "atanh" => "The inverse hyperbolic tangent, principal branch (-1 < x < 1).",
        _ => "The " + fn + " function.",
    };

    /// <summary>See-also pairs for the elementary family: every hyperbolic inverse names the
    /// hyperbolic it inverts (and vice versa), so the two halves of the family are reachable from
    /// either end — help's "See also" is the only route between them.</summary>
    private static string[] ElementaryRelated(string fn) => fn switch
    {
        "sinh" => new[] { "cosh", "tanh", "asinh" },
        "cosh" => new[] { "sinh", "tanh", "acosh" },
        "tanh" => new[] { "sinh", "cosh", "atanh" },
        "asinh" => new[] { "sinh", "acosh", "atanh" },
        "acosh" => new[] { "cosh", "asinh", "atanh" },
        "atanh" => new[] { "tanh", "asinh", "acosh" },
        _ => new[] { "simplify", "diff", "integrate" },
    };

    /// <summary>The example every elementary builtin ships: a call at a point where the value is
    /// exact, so the snippet is a doctest a reader can run and check.</summary>
    private static string ElementaryExample(string fn) => fn switch
    {
        "asinh" => "asinh(0)",
        "acosh" => "acosh(1)",
        "atanh" => "atanh(0)",
        _ => fn + "(x)",
    };

    /// <summary>Help metadata for the symbolic builtins that have no bespoke descriptor: no
    /// user-facing function ships without a summary, parameters and a return kind.</summary>
    private static readonly Dictionary<string, BuiltinDescriptor> Metadata = BuildMetadata();

    private static Dictionary<string, BuiltinDescriptor> BuildMetadata()
    {
        var m = new Dictionary<string, BuiltinDescriptor>(StringComparer.Ordinal)
        {
            ["and"] = new("and", new[] { "a", "b" }, BuiltinCategories.Symbolics,
                "Logical conjunction of two conditions.", ["assume(x > 0); and(x > 0, x < 1)"], "Boolean | Symbolic", ["or", "not"]),
            ["or"] = new("or", new[] { "a", "b" }, BuiltinCategories.Symbolics,
                "Logical disjunction of two conditions.", ["or(x > 0, x < -1)"], "Boolean | Symbolic", ["and", "not"]),
            ["not"] = new("not", new[] { "a" }, BuiltinCategories.Symbolics,
                "Logical negation of a condition.", ["not(x > 0)"], "Boolean | Symbolic", ["and", "or"]),
            ["assume_positive"] = new("assume_positive", new[] { "x" }, BuiltinCategories.Symbolics,
                "Assumes x > 0 for the rest of the session.", ["assume_positive(x)"], "Symbolic", ["assume", "assumptions", "assume_clear"]),
            ["assume_nonnegative"] = new("assume_nonnegative", new[] { "x" }, BuiltinCategories.Symbolics,
                "Assumes x >= 0 for the rest of the session.", ["assume_nonnegative(x)"], "Symbolic", ["assume", "assumptions"]),
            ["assume_negative"] = new("assume_negative", new[] { "x" }, BuiltinCategories.Symbolics,
                "Assumes x < 0 for the rest of the session.", ["assume_negative(x)"], "Symbolic", ["assume", "assumptions"]),
            ["assume_real"] = new("assume_real", new[] { "x" }, BuiltinCategories.Symbolics,
                "Assumes x is real-valued for the rest of the session.", ["assume_real(x)"], "Symbolic", ["assume", "real"]),
            ["assume_integer"] = new("assume_integer", new[] { "x" }, BuiltinCategories.Symbolics,
                "Assumes x is an integer for the rest of the session.", ["assume_integer(x)"], "Symbolic", ["assume", "integer"]),
            ["assume_clear"] = new("assume_clear", Array.Empty<string>(), BuiltinCategories.Symbolics,
                "Clears every active assumption.", ["assume_clear()"], "Text", ["assume", "assumptions"]),
            ["assumptions"] = new("assumptions", Array.Empty<string>(), BuiltinCategories.Symbolics,
                "Lists the active assumptions.", ["assumptions()"], "Text", ["assume", "assume_clear"]),
            ["limit_left"] = new("limit_left", new[] { "f", "x", "x0" }, BuiltinCategories.Calculus,
                "One-sided limit from the left as x approaches x0.", ["limit_left(1/x, x, 0)"], "Symbolic | Text", ["limit", "limit_full"]),
            ["limit_right"] = new("limit_right", new[] { "f", "x", "x0" }, BuiltinCategories.Calculus,
                "One-sided limit from the right as x approaches x0.", ["limit_right(1/x, x, 0)"], "Symbolic | Text", ["limit", "limit_full"]),
            ["solve_system_full"] = new("solve_system_full", new[] { "eqs", "vars" }, BuiltinCategories.Solving,
                "Structured system solve: a SystemSolveResult with status, domain, complete, per-solution Binding records (name, value) and conditions.",
                ["solve_system_full([x^2 + y^2 - 1 == 0, x*y == 0], [x, y])"], "SystemSolveResult", ["solve_system", "solve_full"]),
            ["optimize_full"] = new("optimize_full", new[] { "f", "params" }, BuiltinCategories.Optimization,
                "Structured optimization: an OptimizationResult with the original and optimized expressions, estimated costs before/after, and the applied transformations.",
                ["optimize_full(x^5 + 2*x^4 + 3*x^3 + x^2 + x + 1, [x])"], "OptimizationResult", ["optimize", "compile"]),
        };
        foreach (var fn in ElementaryFunctions)
        {
            m[fn] = new BuiltinDescriptor(fn, new[] { "x" }, BuiltinCategories.Symbolics,
                ElementarySummary(fn), new[] { ElementaryExample(fn) }, "Symbolic",
                ElementaryRelated(fn));
        }
        return m;
    }

    private AssumptionSet _assumptions = AssumptionSet.Empty;

    public void Register(IModusContext c)
    {
        // the D14 seam: core inv/linsolve/matrix_rank/det dispatch symbolic matrices here
        c.RegisterSymbolicMatrixBridge(this);
        c.RegisterSymbolicInspectionBridge(this);

        void Add(string name, string[] parameters, Func<IReadOnlyList<object?>, object?> impl, BuiltinDescriptor? descriptor = null)
        {
            var resolved = descriptor
                ?? (Metadata.TryGetValue(name, out var known)
                    ? known
                    : new BuiltinDescriptor(name, parameters, BuiltinCategories.Symbolics,
                        "(no summary registered)", Array.Empty<string>(), "Symbolic"));
            c.RegisterBuiltin(resolved, args =>
            {
                // a coercion failure is reported with the function, the argument position and the
                // actual payload kind: "diff(): argument 2 must be a symbolic variable; got Integer."
                var tracked = args as BuiltinArgs ?? new BuiltinArgs(args);
                try
                {
                    return Run(() => impl(tracked));
                }
                catch (BuiltinShapeError ex)
                {
                    // a wrong SHAPE is a call-site mistake, not a domain refusal: it crosses as the
                    // documented recoverable argument error (InvalidArgument / TypeMismatch) while
                    // the message still names the builtin, the position, the expectation and the
                    // payload kind that was actually supplied
                    throw new ArgumentException(DescribeArgument(name, tracked, ex.Expected));
                }
                catch (BuiltinArgumentError ex)
                {
                    throw new InvalidOperationException(DescribeArgument(name, tracked, ex.Expected));
                }
            });
        }

        Add("symbol", new[] { "name", "domain" }, args =>
            args.Count >= 2 && args[1] is MathDomain md
                ? SymbolWithDomain((string)args[0]!, md)
                : Exprs.Symbol((string)args[0]!),
            new BuiltinDescriptor("symbol", new[] { "name", "domain" }, BuiltinCategories.Symbolics,
                "Creates a symbolic variable; an optional domain (integer/rational/real/complex) is assumed for it.",
                ["symbol(\"x\")", "symbol(\"x\", real)"], "Symbolic", ["assume", "real", "complex"], MinArity: 1));
        Add("inf", Array.Empty<string>(), _ => Exprs.Infinity,
            new BuiltinDescriptor("inf", Array.Empty<string>(), BuiltinCategories.Symbolics,
                "Positive infinity (the symbolic constant).", ["inf"], "Symbolic"));

        // first-class mathematical domains (solver/assumption options — never magic strings)
        Add("real", Array.Empty<string>(), _ => MathDomain.Real,
            new BuiltinDescriptor("real", Array.Empty<string>(), BuiltinCategories.Symbolics,
                "The real domain value (solve(..., real), symbol(name, real)).", ["solve(x^2 + 1 == 0, x, real)"], "Domain",
                ["complex", "integer", "rational"]));
        Add("complex", Array.Empty<string>(), _ => MathDomain.Complex,
            new BuiltinDescriptor("complex", Array.Empty<string>(), BuiltinCategories.Symbolics,
                "The complex domain value (the solve default).", ["solve(x^2 + 1 == 0, x, complex)"], "Domain",
                ["real", "integer", "rational"]));
        Add("integer", Array.Empty<string>(), _ => MathDomain.Integer,
            new BuiltinDescriptor("integer", Array.Empty<string>(), BuiltinCategories.Symbolics,
                "The integer domain value (symbol(name, integer)).", ["symbol(\"n\", integer)"], "Domain",
                ["rational", "real", "complex"]));
        Add("rational", Array.Empty<string>(), _ => MathDomain.Rational,
            new BuiltinDescriptor("rational", Array.Empty<string>(), BuiltinCategories.Symbolics,
                "The rational domain value (symbol(name, rational)).", ["symbol(\"q\", rational)"], "Domain",
                ["integer", "real", "complex"]));

        // Round 22: the runtime's own capability statement. An agent must be able to LEARN the
        // runtime's limits from structure — today the only way to discover that 2^(1/2),
        // (-8)^(1/3) and integer/rational solving are unsupported is to trip over each one.
        // Round 03 moved sqrt(-1) and (-1)^(1/2) OUT of this list by making them answerable.
        Add("capabilities", Array.Empty<string>(), _ => CapabilitiesRecord(),
            new BuiltinDescriptor("capabilities", Array.Empty<string>(), BuiltinCategories.Introspection,
                "Reports the domains this runtime solves over and the known-unsupported operation classes, each with the exact error code and ErrorCategory member its live call produces.",
                ["capabilities()"], "CapabilitiesResult", ["solve", "solve_full", "symbol", "type"]));

        // Round 28 (item 13): the LaTeX rendering of the SAME expression model. It is a print
        // MODE, not a second printer: Printing routes it through the one precedence table and the
        // one set of structural decisions the pretty form uses (see Printing.LatexRender).
        Add("latex", new[] { "expr" }, args =>
            Printing.PrettyPrint(AsExpr(args[0]), new Printing.PrintOptions(Printing.PrintMode.Latex)),
        new BuiltinDescriptor("latex", new[] { "expr" }, BuiltinCategories.Introspection,
            "Renders an expression as LaTeX source. This is the printer's LaTeX mode, not a second printer: it shares the precedence table and the structural decisions of the pretty form, so the two can never describe different expressions.",
            ["latex((x + 1)/y)"], "Text", ["print", "type"]));

        // elementary functions as symbolic builtins (numeric versions do not exist in the core)
        foreach (var fn in ElementaryFunctions)
        {
            var name = fn;
            Add(name, new[] { "x" }, args => Exprs.Function(Context.Function(name), AsExpr(args[0])));
        }
        Add("assume", new[] { "relation" }, args =>
        {
            var rel = (Expr)args[0]!;
            if (!AssumeRecursive(rel))
                throw new InvalidOperationException("assume() accepts relations and their conjunctions/negations with constant bounds.");
            return rel;
        },
        new BuiltinDescriptor("assume", new[] { "relation" }, BuiltinCategories.Symbolics,
            "Assumes a relation (or conjunction of relations) with a constant bound for the session, e.g. x > 5.",
            ["assume(x > 5)", "assume(and(x > 0, x < 10))"], "Symbolic",
            ["assume_positive", "assumptions", "assume_clear"]));
        Add("and", new[] { "a", "b" }, args => LogicalOp(args, isAnd: true));
        Add("or", new[] { "a", "b" }, args => LogicalOp(args, isAnd: false));
        Add("not", new[] { "a" }, args =>
        {
            if (args[0] is bool b)
                return !b;
            var e = AsExpr(args[0]);
            return Exprs.Not(e);
        });
        Add("assume_positive", new[] { "x" }, args => AssumePred(args, SymbolPredicate.Positive));
        Add("assume_nonnegative", new[] { "x" }, args => AssumePred(args, SymbolPredicate.NonNegative));
        Add("assume_negative", new[] { "x" }, args => AssumePred(args, SymbolPredicate.Negative));
        Add("assume_real", new[] { "x" }, args => AssumeDomain(args, Domain.Real));
        Add("assume_integer", new[] { "x" }, args => AssumeDomain(args, Domain.Integer));
        Add("assume_clear", Array.Empty<string>(), _ =>
        {
            _assumptions = AssumptionSet.Empty;
            return "assumptions cleared";
        });
        Add("assumptions", Array.Empty<string>(), _ =>
            string.Join("; ", _assumptions.Atoms.Select(a => a switch
            {
                SymbolRelationAssumption r => Printing.PrettyPrint(Exprs.Relation(r.Op, Exprs.Symbol(r.S), r.Bound)),
                SymbolDomainAssumption d => d.S.Name + " in " + d.D,
                SymbolPropertyAssumption p => p.S.Name + " is " + p.P,
                _ => AssumptionSet.Describe(a),
            })));

        Add("diff", new[] { "f", "x" }, args =>
            Calculus.Diff(AsExpr(args[0]), AsSymbol(args[1]), Context),
        new BuiltinDescriptor("diff", new[] { "f", "x" }, BuiltinCategories.Calculus,
            "Derivative of f with respect to x.", ["diff(x^3 + 2*x^2 + 5*x + 7, x)"], "Symbolic",
            ["integrate", "jacobian", "hessian"]));
        Add("integrate", new[] { "f", "x" }, args =>
            Integration.Integrate(AsExpr(args[0]), AsSymbol(args[1]), Context),
        new BuiltinDescriptor("integrate", new[] { "f", "x" }, BuiltinCategories.Calculus,
            "Indefinite integral of f with respect to x; every closed form is differentiated and verified.",
            ["integrate(2*x*cos(x^2), x)"], "Symbolic", ["diff", "integrate_full"]));
        Add("integrate_full", new[] { "f", "x" }, args =>
        {
            var r = Integration.IntegrateResult(AsExpr(args[0]), AsSymbol(args[1]), Context);
            return new RecordValue("IntegrationResult",
                new RecordField("status", EnumField("IntegrationStatus", r.Kind)),
                new RecordField("expression", r.Expression),
                new RecordField("conditions", ConditionExprs(r.Conditions)),
                new RecordField("verified", r.Kind != IntegrationKind.Unevaluated),
                new RecordField("method", r.Method ?? ""),
                new RecordField("verification_method", r.VerificationMethod ?? ""),
                new RecordField("exactness", EnumField("SolutionExactness", r.Expression.IsExact
                    ? SolutionExactness.Exact
                    : SolutionExactness.Approximate)),
                new RecordField("diagnostics", Diagnostics(IntegrationDiagnostic(r))));
        },
        new BuiltinDescriptor("integrate_full", new[] { "f", "x" }, BuiltinCategories.Calculus,
            "Structured integration: an IntegrationResult record with status (SolvedExact/SolvedConditional/Unevaluated), the antiderivative, its conditions, and the self-verification flag.",
            ["integrate_full(x^2, x)"], "IntegrationResult", ["integrate"]));
        Add("simplify", new[] { "f" }, args =>
        {
            var e = AsExpr(args[0]);
            // a relation the active assumptions decide folds to a Boolean: the bound lattice is
            // already sound, so refusing to expose it here would hide a decidable answer
            if (e is RelationExpr rel && TryDecideRelation(rel, out bool truth))
                return truth;
            return Simplify.SimplifyExpr(e, Context);
        },
        new BuiltinDescriptor("simplify", new[] { "f" }, BuiltinCategories.Symbolics,
            "Safe simplification: applies only universal rules and rules whose side conditions are already provable from the active assumptions (conditions are never silently discarded).",
            ["simplify(sin(x)^2 + cos(x)^2)", "simplify(x/x)"], "Symbolic", ["simplify_full", "expand", "factor"]));
        Add("simplify_full", new[] { "f" }, args =>
        {
            var e = AsExpr(args[0]);
            var r = Simplify.Transform(e, Context, new Simplify.Options(Trace: true));
            return new RecordValue("TransformResult",
                new RecordField("status", EnumField("TransformStatus", r.Status)),
                new RecordField("original", r.Original),
                new RecordField("expression", r.Expression),
                new RecordField("changed", !r.Expression.Equals(e)),
                new RecordField("conditions", ConditionExprs(r.Conditions)),
                new RecordField("steps", r.Steps.Select(s => (object)new RecordValue("RewriteStep",
                    new RecordField("rule_id", s.RuleId),
                    new RecordField("classification", EnumField("RuleClassification", s.Classification)),
                    new RecordField("before", s.Before),
                    new RecordField("after", s.After),
                    new RecordField("required_conditions", ConditionExprs(s.Conditions)))).ToArray()),
                new RecordField("budget_exceeded", r.BudgetExceeded),
                new RecordField("budget_kind", r.BudgetKind ?? ""),
                new RecordField("diagnostics", Diagnostics(TransformDiagnostic(r))));
        },
        new BuiltinDescriptor("simplify_full", new[] { "f" }, BuiltinCategories.Symbolics,
            "Structured simplify: a TransformResult record with the rewritten expression, required side conditions, and the applied rule steps (stable rule ids and classifications).",
            ["simplify_full(x/x)", "simplify_full(exp(log(x)))"], "TransformResult",
            ["simplify"]));
        Add("expand", new[] { "f" }, args =>
            Algebra.Expand(AsExpr(args[0]), Context),
        new BuiltinDescriptor("expand", new[] { "f" }, BuiltinCategories.Symbolics,
            "Expands products and powers into a sum of monomials.", ["expand((x+1)^3)"], "Symbolic", ["collect", "factor", "simplify"]));
        Add("factor", new[] { "f" }, args =>
            Factoring.Factor(AsExpr(args[0]), Context),
        new BuiltinDescriptor("factor", new[] { "f" }, BuiltinCategories.Symbolics,
            "Factors a polynomial over the rationals.", ["factor(x^4 - 5*x^2 + 4)"], "Symbolic", ["expand", "cancel"]));
        Add("collect", new[] { "f", "x" }, args =>
            Algebra.Collect(AsExpr(args[0]), AsSymbol(args[1]), Context),
        new BuiltinDescriptor("collect", new[] { "f", "x" }, BuiltinCategories.Symbolics,
            "Collects coefficients of powers of x.", ["collect(x*y + x + y + 1, x)"], "Symbolic", ["expand"]));
        Add("cancel", new[] { "f" }, args =>
            RationalFunctions.Cancel(AsExpr(args[0]), Context),
        new BuiltinDescriptor("cancel", new[] { "f" }, BuiltinCategories.Symbolics,
            "Cancels common polynomial factors in a rational function. The result is a bare value and " +
            "CANNOT carry the side conditions the cancellation requires: removing a common factor g " +
            "extends the domain (the input is undefined where g = 0), so the value holds only off that " +
            "pole. Use cancel_full when the definedness delta matters.",
            ["cancel((x^2 - 1)/(x - 1))"], "Symbolic", ["cancel_full", "apart", "factor"]));
        Add("cancel_full", new[] { "f" }, args =>
        {
            var r = RationalFunctions.CancelWithConditions(AsExpr(args[0]), Context);
            return new RecordValue("CancelResult",
                new RecordField("status", EnumField("CancelStatus", r.Status)),
                new RecordField("original", r.Original),
                new RecordField("expression", r.Expression),
                new RecordField("changed", r.Changed),
                new RecordField("conditions", ConditionExprs(r.Conditions)));
        },
        new BuiltinDescriptor("cancel_full", new[] { "f" }, BuiltinCategories.Symbolics,
            "Structured cancel: a CancelResult record with status (Exact/Conditional), the reduced " +
            "expression, and the conditions it requires (the removed common factor must be nonzero — " +
            "the same condition the simplify rewrite path attaches to rat.cancel-x-over-x). An Exact " +
            "reduction reports no conditions at all.",
            ["cancel_full(x/x)", "cancel_full((x^2 - 1)/(x - 1))"], "CancelResult",
            ["cancel", "simplify_full"]));
        Add("apart", new[] { "f", "x" }, args =>
            RationalFunctions.Apart(AsExpr(args[0]), AsSymbol(args[1]), Context),
        new BuiltinDescriptor("apart", new[] { "f", "x" }, BuiltinCategories.Symbolics,
            "Partial fraction decomposition of a rational function in x.", ["apart(1/(x^2 - 1), x)"], "Symbolic", ["cancel"]));
        Add("series", new[] { "f", "x", "x0", "order" }, args =>
            Series.Of(AsExpr(args[0]), AsSymbol(args[1]), AsExpr(args[2]), AsInt(args[3]), Context).ToExpression(),
        new BuiltinDescriptor("series", new[] { "f", "x", "x0", "order" }, BuiltinCategories.Calculus,
            "Power series of f around x = x0 to the given order (with an O-term).",
            ["series(sin(x)/x, x, 0, 8)"], "Symbolic", ["limit", "diff"]));
        Add("limit", new[] { "f", "x", "x0" }, args =>
            LimitToExpr(Limits.Limit(AsExpr(args[0]), AsSymbol(args[1]), AsExpr(args[2]), LimitDirection.TwoSided, Context)),
        new BuiltinDescriptor("limit", new[] { "f", "x", "x0" }, BuiltinCategories.Calculus,
            "Two-sided limit of f as x → x0. Use limit_full for the structured result (existence, left/right values).",
            ["limit(sin(x)/x, x, 0)", "limit(1/x, x, 0)"], "Symbolic | Text",
            ["limit_left", "limit_right", "limit_full"]));
        Add("limit_full", new[] { "f", "x", "x0" }, args =>
        {
            var x = AsSymbol(args[1]);
            var point = AsExpr(args[2]);
            var r = Limits.Limit(AsExpr(args[0]), x, point, LimitDirection.TwoSided, Context);
            // N19: "exists" is THREE-VALUED, not derived from "a value came back or not".
            // true  — a limit was determined;
            // false — non-existence was PROVEN (DoesNotExist: the two sides disagree);
            // null  — the engine did not determine it (Unevaluated, Failed), emitted as the
            //         protocol Null ({kind:Null}), which is neither false nor "". Asserting
            //         "does not exist" here would be a false mathematical claim (e.g. the
            //         squeezed limit x*sin(1/x) -> 0, which the engine cannot yet solve).
            bool? exists = r.Status switch
            {
                LimitStatus.Value or LimitStatus.PlusInfinity or LimitStatus.MinusInfinity => true,
                LimitStatus.DoesNotExist => false,
                _ => null,
            };
            return new RecordValue("LimitResult",
                new RecordField("status", EnumField("LimitStatus", r.Status)),
                new RecordField("exists", exists),
                new RecordField("value", LimitSide(r)),
                new RecordField("left", LimitSide(r.FromLeft)),
                new RecordField("left_conditions", SideConditions(x, point, fromRight: false, r.FromLeft, exists is true)),
                new RecordField("right", LimitSide(r.FromRight)),
                new RecordField("right_conditions", SideConditions(x, point, fromRight: true, r.FromRight, exists is true)),
                new RecordField("conditions", ConditionExprs(r.Conditions)),
                new RecordField("exactness", EnumField("SolutionExactness", r.Exactness)),
                new RecordField("diagnostics", Diagnostics(LimitDiagnostic(r))));
        },
        new BuiltinDescriptor("limit_full", new[] { "f", "x", "x0" }, BuiltinCategories.Calculus,
            "Structured limit: a LimitResult record with status, exists, the two-sided value and its conditions, the left/right one-sided values each with the side constraint they hold under, exactness, and diagnostics.",
            ["limit_full(1/x, x, 0)"], "LimitResult", ["limit", "limit_left", "limit_right"]));
        Add("limit_left", new[] { "f", "x", "x0" }, args =>
            LimitToExpr(Limits.Limit(AsExpr(args[0]), AsSymbol(args[1]), AsExpr(args[2]), LimitDirection.FromLeft, Context)));
        Add("limit_right", new[] { "f", "x", "x0" }, args =>
            LimitToExpr(Limits.Limit(AsExpr(args[0]), AsSymbol(args[1]), AsExpr(args[2]), LimitDirection.FromRight, Context)));
        Add("solve_system", new[] { "eqs", "vars" }, args =>
        {
            var eqs = ExprVector(args[0], EquationVectorExpectation);
            var vs = SymbolVector(args[1], ParameterVectorExpectation);
            return SystemSolveRecord(SystemSolvers.Solve(eqs, vs, Context), vs);
        },
        new BuiltinDescriptor("solve_system", new[] { "eqs", "vars" }, BuiltinCategories.Solving,
            "Solves a polynomial equation system (Gröbner elimination into triangular form, verified solutions) and returns the SAME SystemSolveResult record solve_system_full does.",
            ["solve_system([x^2 + y^2 - 1 == 0, x*y == 0], [x, y])"], "SystemSolveResult",
            ["solve_system_full", "solve"]));
        Add("solve_system_full", new[] { "eqs", "vars" }, args =>
        {
            var eqs = ExprVector(args[0], EquationVectorExpectation);
            var vs = SymbolVector(args[1], ParameterVectorExpectation);
            return SystemSolveRecord(SystemSolvers.Solve(eqs, vs, Context), vs);
        });
        Add("solve", new[] { "f", "x", "domain" }, args =>
        {
            var fx = AsExpr(args[0]);
            var sx = AsSymbol(args[1]);
            var domain = SolveDomainOf(args);
            var set = Solvers.Solve(fx, sx, Context, domain);
            var kept = AcceptedSolutions(set, sx);
            if (set.Status == SolveStatus.Solved && kept.Count > 0)
                return (object)kept.Select(s => s.Value).ToArray();
            if (set.Status == SolveStatus.Solved && set.Families.Count > 0)
                return string.Join("; ", set.Families.Select(f =>
                    Printing.PrettyPrint(f.Template) + " for integer " + f.Parameter.Name));
            if (set.Status == SolveStatus.Solved)
                return "no solutions";
            if (set.Status == SolveStatus.NoSolutions)
                return set.Note ?? "no solutions";
            // A partial or unevaluated result must never be presented as a complete vector.
            if (set.Status == SolveStatus.Partial)
            {
                var shown = kept.Count > 0
                    ? string.Join(", ", kept.Select(s => Printing.PrettyPrint(s.Value)))
                    : "none";
                return $"partially representable ({set.UnrepresentedCount} root(s) missing: " +
                       $"{set.UnrepresentedReason ?? "not representable"}); representable: [{shown}]";
            }
            return "unevaluated: " + (set.Note ?? "no solver for this structure");
        },
        new BuiltinDescriptor("solve", new[] { "f", "x", "domain" }, BuiltinCategories.Solving,
            "Solves an equation for x. The default domain is Complex. A result is returned as a vector only when the solver can represent the COMPLETE solution set over the requested domain; partial results are reported as text (use solve_full for the structured form); pass real for real solutions only.",
            ["solve(x^2 - 4 == 0, x)", "solve(x^2 + 1 == 0, x, real)"], "Vector | Text",
            ["solve_full", "solve_system", "linsolve"], MinArity: 2));
        Add("solve_full", new[] { "f", "x", "domain" }, args =>
        {
            var fx = AsExpr(args[0]);
            var sx = AsSymbol(args[1]);
            var domain = SolveDomainOf(args);
            var set = Solvers.Solve(fx, sx, Context, domain);
            var accepted = AcceptedSolutions(set, sx);

            // rejections after solving are what make a "solved" set empty
            var status = set.Status;
            if (status == SolveStatus.Solved && accepted.Count == 0 && set.Families.Count == 0)
                status = SolveStatus.NoSolutions;

            var solutions = accepted.Select(s => (object)new RecordValue("Solution",
                new RecordField("value", s.Value),
                new RecordField("conditions", ConditionExprs(s.Conditions)),
                new RecordField("multiplicity", (long)s.Multiplicity),
                new RecordField("exactness", EnumField("SolutionExactness", s.Exactness)))).ToArray();

            var families = set.Families.Select(f => (object)new RecordValue("SolutionFamily",
                new RecordField("template", f.Template),
                new RecordField("parameter", Exprs.Symbol(f.Parameter)),
                new RecordField("period", f.Period),
                new RecordField("parameter_domain", ParameterDomainOf(f.Domain)),
                new RecordField("conditions", ConditionExprs(f.Conditions)),
                new RecordField("exactness", EnumField("SolutionExactness", f.Exactness)))).ToArray();

            // ONE function derives BOTH published fields from the effective status, so
            // complete and completeness can never disagree (RISK-003) and NoSolutions reports the
            // complete answer the frozen contract says it is
            var (complete, completeness) = SolveCompletenessMapping.Of(status, set.Complete);

            return new RecordValue("SolveResult",
                new RecordField("status", EnumField("SolveStatus", status)),
                new RecordField("variable", Exprs.Symbol(sx)),
                new RecordField("domain", DomainOf(domain)),
                new RecordField("complete", complete),
                new RecordField("completeness", EnumField("Completeness", completeness)),
                new RecordField("solutions", solutions),
                new RecordField("families", families),
                new RecordField("common_conditions", ConditionExprs(CommonConditions(accepted))),
                new RecordField("represented_count", (long)accepted.Count),
                new RecordField("unrepresented_count", (long)set.UnrepresentedCount),
                new RecordField("unrepresented_reason", set.UnrepresentedReason ?? ""),
                new RecordField("diagnostics", Diagnostics(SolveDiagnostic(set, status))));
        },
        new BuiltinDescriptor("solve_full", new[] { "f", "x", "domain" }, BuiltinCategories.Solving,
            "Structured solve: a SolveResult record with status, domain (a Domain value), complete flag, per-solution conditions/multiplicity/exactness, parametric families, and diagnostics. status is Solved only when the represented set is complete over the requested domain.",
            ["solve_full(x^2 - 4 == 0, x)", "solve_full(x^4 - x^2 - 1 == 0, x)"], "SolveResult",
            ["solve", "solve_system_full"], MinArity: 2));
        Add("subs", new[] { "f", "x", "value" }, args =>
            Evaluation.Substitute(AsExpr(args[0]), Context,
                new Dictionary<Symbol, Expr> { [AsSymbol(args[1])] = AsExpr(args[2]) }),
        new BuiltinDescriptor("subs", new[] { "f", "x", "value" }, BuiltinCategories.Symbolics,
            "Substitutes value for the symbol x in f.", ["subs(x^2 + 1, x, 3)"], "Symbolic", ["evalf"]));
        Add("evalf", new[] { "f", "digits" }, args =>
        {
            var f = AsExpr(args[0]);
            var digits = (int)AsLong(args[1]);
            using (Rl.WithPrecision(digits, Math.Min(digits, 50)))
            {
                var num = Evaluation.EvaluateToNum(f, Context, new Dictionary<Symbol, Num>());
                return NumToPayload(num);
            }
        },
        new BuiltinDescriptor("evalf", new[] { "f", "digits" }, BuiltinCategories.Numerics,
            "Numerically evaluates a symbolic expression to the given number of digits.",
            ["evalf(sqrt(2), 30)"], "Real | Complex", ["subs"]));
        Add("hessian", new[] { "f", "vars" }, args =>
        {
            var f = AsExpr(args[0]);
            var vars = NameList(args[1]).Select(Context.Symbol).ToArray();
            return (object)MatrixPayload(SymbolicMatrix.Hessian(f, vars, Context));
        },
        new BuiltinDescriptor("hessian", new[] { "f", "vars" }, BuiltinCategories.Calculus,
            "Hessian matrix (second derivatives) of f in the given variables.", ["hessian(x*y, [x, y])"], "Array",
            ["jacobian", "diff"]));
        Add("jacobian", new[] { "fs", "vars" }, args =>
        {
            var fs = ExprVector(args[0], FunctionVectorExpectation);
            var vars = SymbolVector(args[1], ParameterVectorExpectation);
            return (object)MatrixPayload(SymbolicMatrix.Jacobian(fs, vars, Context));
        },
        new BuiltinDescriptor("jacobian", new[] { "fs", "vars" }, BuiltinCategories.Calculus,
            "Jacobian matrix (first derivatives) of the function vector in the given variables.", ["jacobian([x*y, x+y], [x, y])"], "Array",
            ["hessian", "diff"]));
        Add("optimize", new[] { "f", "params" }, args =>
        {
            var f = AsExpr(args[0]);
            var ps = NameList(args[1]).Select(Context.Symbol).ToArray();
            return Optimizer.Optimize(f, ps, Context).Expression;
        },
        new BuiltinDescriptor("optimize", new[] { "f", "params" }, BuiltinCategories.Optimization,
            "Equivalence-preserving optimization for evaluation (CSE accounting, policy-gated Hornerization). Use optimize_full for the cost trace.",
            ["optimize(x^5 + 2*x^4 + 3*x^3 + x^2 + x + 1, [x])"], "Symbolic", ["optimize_full", "compile"]));
        Add("optimize_full", new[] { "f", "params" }, args =>
        {
            var f = AsExpr(args[0]);
            var ps = NameList(args[1]).Select(Context.Symbol).ToArray();
            var r = Optimizer.OptimizeDetailed(f, ps, Context);
            var transformations = new List<object?>();
            if (r.SharedSubtrees > 0) transformations.Add("cse");
            if (r.HornerRewrites > 0) transformations.Add("horner");
            return new RecordValue("OptimizationResult",
                new RecordField("original", f),
                new RecordField("optimized", r.Expression),
                new RecordField("estimated_cost_before", r.NodesBefore),
                new RecordField("estimated_cost_after", r.NodesAfter),
                new RecordField("shared_subtrees", r.SharedSubtrees),
                new RecordField("horner_rewrites", r.HornerRewrites),
                new RecordField("transformations", transformations.ToArray()),
                new RecordField("target", "mathir"),
                new RecordField("policy", PolicyToken(r.Options)),
                // an optimization either succeeded or threw: there is nothing to report (D1)
                new RecordField("diagnostics", Diagnostics()));
        });
    }

    // -----------------------------------------------------------------
    // Structured diagnostics (D1: the wire carries Diagnostic records, never prose)
    // -----------------------------------------------------------------

    /// <summary>Projects an ordered diagnostic list for a record's "diagnostics" field: ALWAYS an
    /// Array of Diagnostic records, EMPTY when nothing happened — never Text, never Null and never
    /// an empty string. A null element means "this result has nothing to report".</summary>
    private static object?[] Diagnostics(params Diagnostic?[] diagnostics)
    {
        var present = new List<Diagnostic>(diagnostics.Length);
        foreach (var d in diagnostics)
            if (d is not null)
                present.Add(d);
        return DiagnosticProjection.ToRecordValues(present);
    }

    /// <summary>The diagnostic a solve result carries: the kernel note becomes its MESSAGE, or —
    /// when the note is null and roots are missing — the unrepresented remainder is reported.
    /// A complete solve with nothing missing yields null, projected as an empty array.</summary>
    private static Diagnostic? SolveDiagnostic(SolutionSet set, SolveStatus status)
    {
        if (set.Note is { } note)
            return Diagnostic.Of(SolveDiagnosticCode(status), SolveDiagnosticCategory(status), note);
        if (set.UnrepresentedCount > 0)
            return Diagnostic.Of("solve.unrepresented-roots", ErrorCategory.UnsupportedOperation,
                set.UnrepresentedReason ?? "part of the solution set is not representable");
        return null;
    }

    /// <summary>Stable code per disposition: a consumer matches the CODE, never the message.</summary>
    private static string SolveDiagnosticCode(SolveStatus status) => status switch
    {
        SolveStatus.NoSolutions => "solve.no-solutions",
        SolveStatus.Unevaluated => "solve.unevaluated",
        SolveStatus.BudgetExceeded => "solve.budget-exceeded",
        SolveStatus.Partial => "solve.partial",
        _ => "solve.note",
    };

    private static ErrorCategory SolveDiagnosticCategory(SolveStatus status) => status switch
    {
        SolveStatus.NoSolutions => ErrorCategory.NoSolution,
        SolveStatus.BudgetExceeded => ErrorCategory.BudgetExceeded,
        SolveStatus.Partial => ErrorCategory.UnsupportedOperation,
        _ => ErrorCategory.UnsupportedOperation,
    };

    /// <summary>A system solve reports only its kernel note: null means an EMPTY array.</summary>
    private static Diagnostic? SystemSolveDiagnostic(SystemSolveResult result) => result.Note is { } note
        ? Diagnostic.Of(
            result.Status == SolveStatus.NoSolutions ? "system-solve.no-solutions" : "system-solve.unevaluated",
            result.Status == SolveStatus.NoSolutions ? ErrorCategory.NoSolution : ErrorCategory.UnsupportedOperation,
            note)
        : null;

    /// <summary>The ONE SystemSolveResult builder. Both call forms (<c>solve_system</c> and
    /// <c>solve_system_full</c>) publish this record — the frozen contract declares ONE record type
    /// for a system solve — so the two forms cannot drift, and neither can degrade the result to the
    /// joined prose it used to return.
    /// <para>
    /// <c>complete</c> and <c>completeness</c> are read off the SAME projection
    /// <c>SolveResult</c> uses (<see cref="SolveCompletenessMapping.Of"/>, with the kernel's own
    /// completeness claim as the budget-stop subset), so the two records cannot disagree either.</para>
    /// </summary>
    private static RecordValue SystemSolveRecord(SystemSolveResult result, IReadOnlyList<Symbol> vars)
    {
        var solutions = result.Solutions.Select(sol => (object)new RecordValue("SystemSolution",
            new RecordField("bindings", vars.Select(v => (object)BindingRecord(new Binding(v.Name, sol.Assignment[v]))).ToArray()),
            new RecordField("conditions", ConditionExprs(sol.Conditions)),
            new RecordField("exactness", EnumField("SolutionExactness", sol.Exactness)))).ToArray();

        // the SAME projection SolveResult uses: an inconsistent system is a provably empty solution
        // set and therefore a COMPLETE answer (a truncated enumeration stays Partial)
        var (complete, completeness) = SolveCompletenessMapping.Of(result.Status, result.Complete);

        return new RecordValue("SystemSolveResult",
            new RecordField("status", EnumField("SolveStatus", result.Status)),
            new RecordField("domain", DomainOf(result.Domain)),
            new RecordField("complete", complete),
            new RecordField("completeness", EnumField("Completeness", completeness)),
            new RecordField("solutions", solutions),
            new RecordField("diagnostics", Diagnostics(SystemSolveDiagnostic(result))));
    }

    /// <summary>A limit reports its kernel failure reason; null means an EMPTY array.</summary>
    private static Diagnostic? LimitDiagnostic(LimitResult result) => result.FailureReason is { } reason
        ? Diagnostic.Of("limit.unevaluated", ErrorCategory.UnsupportedOperation, reason)
        : null;

    /// <summary>An integration reports its kernel note; null means an EMPTY array. Round 10: a
    /// REFUSED integration is no longer silent. The kernel always supplies a note for
    /// <see cref="IntegrationKind.Unevaluated"/>, and it crosses as a STRUCTURED diagnostic with
    /// a stable code, the <see cref="ErrorCategory.UnsupportedOperation"/> category, a
    /// CLASS-LEVEL message that is identical for every refused integrand, and the input-specific
    /// reason in <c>details</c> (a nested Diagnostic). Before round 10 this path emitted
    /// <c>status = Unevaluated</c> with an EMPTY diagnostics array: "did anything go wrong?" was
    /// not answerable from structure, which is exactly the property the protocol promises.</summary>
    private static Diagnostic? IntegrationDiagnostic(IntegrationResult result) => result.Note is { } note
        ? WithDetails("integration.unevaluated", ErrorCategory.UnsupportedOperation,
            IntegrationUnevaluatedMessage,
            Diagnostic.Of("integration.no-closed-form", ErrorCategory.UnsupportedOperation, note))
        : null;

    /// <summary>A kernel-level diagnostic that carries NESTED details. <see cref="Diagnostic.Of"/>
    /// deliberately produces an empty details list; this is the one construction that does not,
    /// and it keeps every other invariant of the six-field form: no source span at this layer, so
    /// the location stays Null.</summary>
    private static Diagnostic WithDetails(
        string code, ErrorCategory category, string message, params Diagnostic[] details) =>
        new(code, category, message, Recoverable: true, Location: null, Details: details);

    /// <summary>The diagnostic a transformation reports where the record previously reported none:
    /// a budget stop (a larger budget recovers it) and a condition set with no model (retrying
    /// cannot) are the two states a caller must be able to branch on without re-deriving them from
    /// budget_exceeded/conditions. The ordinary case reports nothing.</summary>
    private static Diagnostic? TransformDiagnostic(TransformResult result) => result.Status switch
    {
        TransformStatus.BudgetExceeded => Diagnostic.Of("transform.budget-exceeded", ErrorCategory.BudgetExceeded,
            "the rewrite budget '" + (result.BudgetKind ?? "rewrite_steps") +
            "' was exhausted; the returned expression is a partial result."),
        TransformStatus.Unsatisfiable => Diagnostic.Of("transform.unsatisfiable-conditions",
            ErrorCategory.DomainError,
            "the applied rules require contradictory conditions, so this branch has no model.",
            recoverable: false),
        _ => null,
    };

    /// <summary>One enum-valued result field: the DECLARED enum type name plus the member name.
    /// Every enum emission goes through here so an enum field can never be spelled as text by
    /// accident (decision D2, alignment addendum §9.1) and so the type name is never invented at
    /// the call site.</summary>
    private static EnumValue EnumField<TEnum>(string typeName, TEnum member) where TEnum : struct, Enum =>
        new(typeName, member.ToString());

    // -----------------------------------------------------------------
    // capabilities(): the runtime's real-only limits, discoverable from structure
    // -----------------------------------------------------------------

    /// <summary>
    /// The structured capability statement behind <c>capabilities()</c>. Every advertised entry
    /// carries the code, <see cref="ErrorCategory"/> member and message that the LIVE call named in
    /// its <c>trigger</c> produces — these strings are transcriptions of observed output, not a
    /// second vocabulary, and <c>CapabilitiesBuiltinTests</c> RUNS every trigger through the
    /// published <c>Lovelace.Run</c> envelope and fails if the advertisement drifts from the kernel.
    /// <para>
    /// TWO CARRIER SHAPES exist, and every class added in round 10 says in its name which one
    /// applies:
    /// </para>
    /// <list type="bullet">
    /// <item><b>the refusal IS the error envelope</b> (exit 1, <c>ok:false</c>, with
    /// <c>code</c>/<c>category</c>/<c>message</c> at the top level): the power guards, the solver
    /// domain guard, the non-symbolic argument guards of solve/diff/integrate/limit, the plot
    /// guards, the DSP element guard, the linsolve matrix guard and the FFT length guard.</item>
    /// <item><b>the refusal rides in a result record's <c>diagnostics</c> while the call SUCCEEDS</b>
    /// (exit 0, <c>ok:true</c>): the class name ends in <c>-in-record-diagnostics</c>, the record's
    /// own <c>status</c> enum reports the refusal (<c>Unevaluated</c>/<c>Partial</c>), and the
    /// advertised code/category/message are transcribed from the FIRST element of that record's
    /// <c>diagnostics</c> array. The test asserts that carrier, not just the strings.</item>
    /// </list>
    /// <para>
    /// The marker is a GUARANTEE the marked classes make, not a partition of the whole list: the
    /// four entries that predate round 10 keep their published <c>operation_class</c> ids, so
    /// <c>rootof.complex-algebraic</c> is record-carried (audit P6a-3 noted the advertisement never
    /// claimed which carrier) without carrying the marker. The honesty test reports the carrier it
    /// observed for every entry, marked or not.
    /// </para>
    /// <para>
    /// <c>message</c> is what the advertised <c>trigger</c> produces, byte for byte. Some classes
    /// have more than one message: <c>limit.unevaluated</c> reports the kernel's input-specific
    /// reason ("coefficient does not evaluate at the point" for <c>limit_full(sin(x), x, inf)</c>,
    /// "leading coefficient is symbolic" for <c>limit_full(a/x, x, 0)</c>), the FFT guard embeds the
    /// offending length, and the argument guards embed the builtin and the payload kind. For those
    /// the CODE and CATEGORY are class-level and the MESSAGE is trigger-level — which is exactly the
    /// pair the honesty test asserts. <c>integration.unevaluated</c> is deliberately NOT one of
    /// them: its outer message is class-level (identical for every refused integrand) and the
    /// input-specific reason rides in the diagnostic's <c>details</c> array, so one advertised
    /// message is true for the whole class.
    /// </para>
    /// <para>
    /// The four entries that predate round 10, unchanged:
    /// </para>
    /// <list type="bullet">
    /// <item>a non-integer exponent of a positive base, e.g. <c>2^(1/2)</c>: a
    /// <c>NotImplementedException</c> from <c>Lovelace.Real.Real.Pow</c>, classified as
    /// <c>UnsupportedOperation</c> / <c>UnsupportedOperation</c>. The result is a real irrational
    /// the Real type will not approximate, so this stays a genuine unsupported class.</item>
    /// <item>a negative base raised to an exponent whose denominator is &gt;= 3, e.g.
    /// <c>(-8)^(1/3)</c>: the SAME exception and therefore the same code, category and message.
    /// It is a distinct operation class because the principal value is a root of unity that is not
    /// a complex constant, so it is enumerated separately even though its live envelope is
    /// identical — an agent matching on the code sees one class, one matching on
    /// <c>operation_class</c> sees both.</item>
    /// <item>solving over integer/rational: an <c>InvalidOperationException</c> raised by the
    /// plugin's domain guard, classified as <c>InvalidOperation</c> / <c>DomainError</c>.</item>
    /// <item>complex algebraic roots of degree &gt;= 4: the kernel's own stable diagnostic code
    /// <c>solve.unrepresented-roots</c> / <c>UnsupportedOperation</c> (from
    /// <see cref="SolveDiagnostic"/>), which is non-fatal and rides in a solve record.</item>
    /// </list>
    /// <para>
    /// <c>sqrt</c> of a negative real USED to be the first entry (<c>complex.sqrt-negative</c>,
    /// <c>ArithmeticError</c> / <c>DomainError</c>). Round 03 moved it into the supported set: the
    /// exact closed form of <c>sqrt(-a)</c> is <c>i·sqrt(a)</c>, so the live call now answers (see
    /// <c>Lovelace.Suite.Interpreter</c>'s sqrt builtin, which routes the input the real-domain
    /// operation rejects to the symbolic complex path). The <c>Lovelace.Real</c> contract is
    /// unchanged — <c>Real.Sqrt(-1)</c> still throws.
    /// </para>
    /// <para>
    /// <c>exactness</c> remains <c>BestEffort</c>, and deliberately. What the enumeration DOES
    /// cover: <b>every refusal class the round-09 adversarial audit found unlisted
    /// (docs/goal-cycle-4/round-09/audit-P2P6-wire.md, part 6(b))</b> — the symbolic-limit refusal,
    /// the integration refusal, the symbolic plot path, the non-symbolic solve/diff/integrate/limit
    /// arguments, the DSP symbolic-element guard, the linsolve matrix guard and the FFT length
    /// constraint — together with the classes earlier rounds advertised; 16 entries, each
    /// live-verified by <c>CapabilitiesBuiltinTests</c>. What is still NOT enumerated, and why:
    /// </para>
    /// <list type="bullet">
    /// <item>the remaining kernel-note diagnostics whose message is the INPUT-SPECIFIC reason —
    /// <c>solve.unevaluated</c>, <c>solve.partial</c>, <c>solve.budget-exceeded</c>,
    /// <c>solve.no-solutions</c>, <c>system-solve.unevaluated</c>,
    /// <c>system-solve.no-solutions</c>, <c>matrix.singular</c>. They are refusals, but one
    /// advertised message could not state them truthfully; they need the same details-carrying
    /// shape <c>integration.unevaluated</c> now has.</item>
    /// <item><c>transform.budget-exceeded</c> and <c>transform.unsatisfiable-conditions</c>: both
    /// have stable messages, but neither is an unsupported OPERATION — a retryable budget stop and a
    /// contradictory-branch domain error. They belong to the transform contract, not to this
    /// list.</item>
    /// <item>the host-level error taxonomy no plugin owns: <c>ParseError</c> for a malformed script,
    /// <c>FileReadError</c>, the arity errors (<c>abs(1,2)</c>, <c>sin(1,2)</c>,
    /// <c>sum(1,2,3)</c>), the <c>Cancelled</c>/<c>BudgetExceeded</c> cancellation envelope, and
    /// <c>UnsatisfiableAssumptions</c> from <c>assume()</c>. They are real refusals, but they are
    /// not per-operation capability boundaries of this kernel.</item>
    /// <item>the caller-side argument errors that are still BUGS rather than capabilities:
    /// <c>mean(1)</c>, <c>max(1,2)</c> and <c>sum(x, 5)</c> surface as
    /// <c>InternalError</c>/<c>InternalInvariantFailure</c> carrying the raw CLR message "Specified
    /// cast is not valid." (audit finding F4), and a 3000-deep script overflows the native stack and
    /// emits no envelope at all (F11). Advertising any of them would claim a capability boundary
    /// where the truth is a defect; they stay unadvertised until they are fixed or typed.</item>
    /// </list>
    /// <para>
    /// So the honest reading of this record is: <b>exhaustive over the refusal classes the round-09
    /// audit exhibited, and explicitly not exhaustive over the runtime's whole error taxonomy</b>.
    /// <c>BestEffort</c> is the member that says exactly that, and
    /// <c>CapabilitiesBuiltinTests.ExactnessBestEffort_IsBackedByLiveRefusalsTheListDoesNotCarry</c>
    /// keeps it falsifiable by driving refusals the list does not enumerate.
    /// </para>
    /// </summary>
    private static RecordValue CapabilitiesRecord() => new(
        "CapabilitiesResult",
        // solver domain options: what solve(..., domain) accepts, and what it rejects
        new RecordField("supported_domains", new object?[] { MathDomain.Real, MathDomain.Complex }),
        new RecordField("unsupported_domains", new object?[] { MathDomain.Integer, MathDomain.Rational }),
        new RecordField("unsupported_operations", new object?[]
        {
            // ---- the four entries that predate round 10 (byte-identical, still live-verified) ----
            UnsupportedCapability("pow.non-integer-exponent", "UnsupportedOperation",
                ErrorCategory.UnsupportedOperation,
                "Non-integer exponents are not yet supported.", "2^(1/2)"),
            UnsupportedCapability("pow.negative-base-unrepresentable-exponent", "UnsupportedOperation",
                ErrorCategory.UnsupportedOperation,
                "Non-integer exponents are not yet supported.", "(-8)^(1/3)"),
            UnsupportedCapability("solve.unsupported-domain", "InvalidOperation", ErrorCategory.DomainError,
                "solve(): currently supports domains real and complex; got integer.",
                "x = symbol(\"x\"); solve(x^2 - 2 == 0, x, integer)"),
            UnsupportedCapability("rootof.complex-algebraic", "solve.unrepresented-roots",
                ErrorCategory.UnsupportedOperation,
                "complex algebraic roots not supported (RootOf is real-only in v1).",
                "x = symbol(\"x\"); solve_full(x^4 - x^2 - 1 == 0, x)"),

            // ---- round 10: the refusal classes the round-09 audit found unlisted (part 6(b)) ----
            // P6b-1: the refusal rides in a LimitResult's diagnostics while the call SUCCEEDS, so the
            // class name marks the carrier. `limit.unevaluated` has more than one message (this one
            // and "leading coefficient is symbolic" for limit_full(a/x, x, 0)); code and category are
            // class-level, the message is the one THIS trigger produces.
            UnsupportedCapability("limit.unevaluated-in-record-diagnostics", "limit.unevaluated",
                ErrorCategory.UnsupportedOperation,
                "coefficient does not evaluate at the point",
                "x = symbol(\"x\"); limit_full(sin(x), x, inf)"),
            // P6b-2/P6b-3: the integration refusal used to be a status with an EMPTY diagnostics
            // array. The kernel now always supplies a reason, which crosses as this diagnostic's
            // details; the outer message is the same for every refused integrand, so it is
            // class-level and true for all of them.
            UnsupportedCapability("integration.unevaluated-in-record-diagnostics", "integration.unevaluated",
                ErrorCategory.UnsupportedOperation,
                IntegrationUnevaluatedMessage,
                "x = symbol(\"x\"); integrate_full(exp(x^2), x)"),
            // P6b-4: plot() has no sampling range and no evaluator, so a symbolic argument is
            // refused. It used to die as InternalError/InternalInvariantFailure ("Specified cast is
            // not valid.", recoverable:false) because the single-vector form cast its argument before
            // checking its kind; the refusal is now the typed envelope below.
            UnsupportedCapability("plot.symbolic-expression", "InvalidOperation", ErrorCategory.DomainError,
                "plot() argument 1 must be a vector, but got 'Symbolic'.",
                "x = symbol(\"x\"); plot(sin(x))"),
            // the same defect on the element axis: a vector whose ELEMENTS are symbolic is refused by
            // the plot value conversion (this one was already typed, merely unlisted)
            UnsupportedCapability("plot.symbolic-element", "InvalidOperation", ErrorCategory.DomainError,
                "Cannot convert value of kind 'Symbolic' to a number for plotting.",
                "x = symbol(\"x\"); plot([x, 1, 2])"),
            // P6b-5: the argument-coercion guards. One entry per refusing surface, because the message
            // names the builtin and the payload kind; the code and category are shared.
            UnsupportedCapability("solve.non-symbolic-variable", "InvalidOperation", ErrorCategory.DomainError,
                "solve(): argument 2 must be a symbolic variable; got Natural.",
                "x = symbol(\"x\"); solve(x^2 - 2 == 0, 1)"),
            UnsupportedCapability("solve.non-symbolic-expression", "InvalidOperation", ErrorCategory.DomainError,
                "solve(): argument 1 must be a symbolic expression; got Vector.",
                "x = symbol(\"x\"); solve([x == 1], x)"),
            UnsupportedCapability("diff.non-symbolic-variable", "InvalidOperation", ErrorCategory.DomainError,
                "diff(): argument 2 must be a symbolic variable; got Natural.",
                "x = symbol(\"x\"); diff(x^2, 1)"),
            UnsupportedCapability("integrate.non-symbolic-variable", "InvalidOperation", ErrorCategory.DomainError,
                "integrate(): argument 2 must be a symbolic variable; got Natural.",
                "x = symbol(\"x\"); integrate(x^2, 1)"),
            UnsupportedCapability("limit.non-symbolic-variable", "InvalidOperation", ErrorCategory.DomainError,
                "limit(): argument 2 must be a symbolic variable; got Natural.",
                "x = symbol(\"x\"); limit(sin(x), 1, 0)"),
            // P6b-6: the DSP element guard. The message leaks the kernel's internal CLR type name
            // ('SymbolExpr') — Lovelace.Dsp is outside this round's scope, so it is transcribed as
            // observed rather than reworded.
            UnsupportedCapability("dsp.symbolic-element", "InvalidOperation", ErrorCategory.DomainError,
                "DSP builtins expect numeric/complex elements, but got 'SymbolExpr'.",
                "x = symbol(\"x\"); dft([x, 1, 2, 3])"),
            // P6b-7: linsolve() needs a symbolic matrix; a numeric one is refused by the host bridge.
            UnsupportedCapability("linsolve.non-symbolic-matrix", "InvalidOperation", ErrorCategory.DomainError,
                "linsolve() requires a symbolic matrix A.",
                "A = [[1,2],[2,4]]; b = [[1],[3]]; linsolve(A, b)"),
            // P6b-8: the FFT length constraint. The message carries the .NET ArgumentException
            // artifact "(Parameter 'x')" and the offending length, both transcribed as observed.
            UnsupportedCapability("fft.non-power-of-two-length", "InvalidArgument", ErrorCategory.TypeMismatch,
                "FFT length must be a power of two, but got 3. (Parameter 'x')",
                "x = symbol(\"x\"); fft([1,2,3])"),
        }),
        new RecordField("exactness", new EnumValue("CapabilitiesExactness", "BestEffort")));

    /// <summary>The CLASS-LEVEL message of the <c>integration.unevaluated</c> refusal: identical for
    /// every refused integrand, with the input-specific reason in the diagnostic's <c>details</c>.
    /// It is a constant because the advertisement and the kernel must not be able to drift.</summary>
    internal const string IntegrationUnevaluatedMessage =
        "No closed form was found, so the integral is returned unevaluated; the reason for the refusal is carried in this diagnostic's details.";

    /// <summary>One unsupported operation class: the class identifier an agent enumerates on, the
    /// EXACT wire code and category its live call produces, the human message, and a runnable
    /// snippet that trips it (statements separated by ';', per the language contract).</summary>
    private static RecordValue UnsupportedCapability(
        string operationClass, string code, ErrorCategory category, string message, string trigger) =>
        new("UnsupportedCapability",
            new RecordField("operation_class", operationClass),
            new RecordField("code", code),
            new RecordField("category", EnumField("ErrorCategory", category)),
            new RecordField("message", message),
            new RecordField("trigger", trigger));

    private static IEnumerable<string> NameList(object? o)
    {
        if (o is not IReadOnlyList<object?> list)
            throw new BuiltinArgumentError("a list of parameter symbols, e.g. [x]");
        foreach (var n in list)
        {
            if (n is string s)
                yield return s;
            else if (n is SymbolExpr sx)
                yield return sx.Symbol.Name;
            else
                throw new BuiltinArgumentError("a list of parameter symbols, e.g. [x]");
        }
    }

    // the declared expectation text of the three system-solving vector positions: one constant per
    // position, so the message a caller reads and the descriptor's parameter name cannot drift
    private const string EquationVectorExpectation = "a list of equations, e.g. [x + y == 2, x - y == 0]";
    private const string FunctionVectorExpectation = "a list of expressions, e.g. [x*y, x + y]";
    private const string ParameterVectorExpectation = "a list of parameter symbols, e.g. [x, y]";

    /// <summary>The EQUATION / FUNCTION vector position. The direct cast this replaces threw
    /// <c>InvalidCastException</c> for a relation or a bare expression and the whole call crossed as
    /// an internal invariant failure; a wrong shape here is the documented recoverable argument
    /// error instead, naming the builtin and what the position wanted. An element that is not an
    /// expression at all is the same class of mistake, not a second one.</summary>
    private static Expr[] ExprVector(object? o, string expected)
    {
        if (o is not IReadOnlyList<object?> list)
            throw new BuiltinShapeError(expected);
        try
        {
            return list.Select(AsExpr).ToArray();
        }
        catch (BuiltinArgumentError)
        {
            throw new BuiltinShapeError(expected);
        }
    }

    /// <summary>The PARAMETER vector position, the second-argument twin of
    /// <see cref="ExprVector"/>: a bare symbol where a list is declared is a call-shape violation
    /// and crosses the same way.</summary>
    private Symbol[] SymbolVector(object? o, string expected)
    {
        try
        {
            return NameList(o).Select(Context.Symbol).ToArray();
        }
        catch (BuiltinArgumentError)
        {
            throw new BuiltinShapeError(expected);
        }
    }

    /// <summary>Adds assumption atoms: relations, conjunctions of relations, and negations of
    /// relations (mapped to their complement relation). Disjunctions are rejected: the atom
    /// lattice has no disjunction.</summary>
    /// <summary>The same relation read from the other side: a &lt; b is b &gt; a.</summary>
    private static RelOp? Flipped(RelOp op) => op switch
    {
        RelOp.Lt => RelOp.Gt,
        RelOp.Gt => RelOp.Lt,
        RelOp.Le => RelOp.Ge,
        RelOp.Ge => RelOp.Le,
        RelOp.Eq => RelOp.Eq,
        RelOp.Ne => RelOp.Ne,
        _ => null,
    };

    private bool AssumeRecursive(Expr rel)
    {
        switch (rel)
        {
            case RelationExpr r:
            {
                if (r.Left is SymbolExpr sx && r.Right is (RationalConstantExpr or IntegerConstantExpr or RealConstantExpr))
                {
                    _assumptions = _assumptions.Add(new SymbolRelationAssumption(sx.Symbol, r.Op, r.Right));
                    return true;
                }
                // "1/2 < x" states the same fact as "x > 1/2": normalise the mirrored spelling.
                // The atom lattice stores symbol-op-constant only, so without this the surface is
                // shape-sensitive and rejects a perfectly ordinary way of writing a bound.
                if (r.Right is SymbolExpr rs &&
                    r.Left is (RationalConstantExpr or IntegerConstantExpr or RealConstantExpr) &&
                    Flipped(r.Op) is { } flipped)
                {
                    _assumptions = _assumptions.Add(new SymbolRelationAssumption(rs.Symbol, flipped, r.Left));
                    return true;
                }
                return false;
            }
            case AndExpr an:
            {
                var ok = true;
                foreach (var o in an.Operands)
                    ok &= AssumeRecursive(o);
                return ok;
            }
            case NotExpr nt when nt.Operand is RelationExpr nr && nr.Left is SymbolExpr nx &&
                                   nr.Right is (RationalConstantExpr or IntegerConstantExpr or RealConstantExpr):
            {
                var negated = AssumptionSet.Negate(new SymbolRelationAssumption(nx.Symbol, nr.Op, nr.Right));
                if (negated is SymbolRelationAssumption sra)
                {
                    _assumptions = _assumptions.Add(sra);
                    return true;
                }
                return false;
            }
            default:
                return false;
        }
    }

    private object LogicalOp(IReadOnlyList<object?> args, bool isAnd)
    {
        if (args[0] is bool l0 && args[1] is bool r0)
            return isAnd ? l0 && r0 : l0 || r0;
        return isAnd ? Exprs.And(AsExpr(args[0]), AsExpr(args[1])) : Exprs.Or(AsExpr(args[0]), AsExpr(args[1]));
    }

    /// <summary>Maps the requested domain value onto the solver's domain. Unsupported domains are
    /// REJECTED: silently widening integer/rational requests to Complex would answer a different
    /// question than the caller asked.</summary>
    private static SolveDomain SolveDomainOf(IReadOnlyList<object?> args)
    {
        if (args.Count < 3 || args[2] is null)
            return SolveDomain.Complex;
        if (args[2] is MathDomain md)
        {
            return md switch
            {
                MathDomain.Real => SolveDomain.Real,
                MathDomain.Complex => SolveDomain.Complex,
                _ => throw new InvalidOperationException(
                    "solve(): currently supports domains real and complex; got " +
                    md.ToString().ToLowerInvariant() + "."),
            };
        }
        throw new InvalidOperationException(
            "solve(): argument 3 (domain) must be a domain value such as real or complex.");
    }

    /// <summary>Decides a single relation of the form symbol OP constant against the active
    /// assumptions. Returns false when the relation is not of that shape or is not decided.</summary>
    private bool TryDecideRelation(RelationExpr rel, out bool truth)
    {
        truth = false;
        if (rel.Left is not SymbolExpr sx)
            return false;
        var answer = Context.Assumptions.Ask(new SymbolRelationAssumption(sx.Symbol, rel.Op, rel.Right));
        if (answer == Tristate.True)
        {
            truth = true;
            return true;
        }
        if (answer == Tristate.False)
            return true;
        return false;
    }

    private static MathDomain DomainOf(SolveDomain d) =>
        d == SolveDomain.Real ? MathDomain.Real : MathDomain.Complex;

    /// <summary>Family parameter domains cross the wire as first-class Domain values. Both
    /// declared arms project onto the kernel lattice: a NON-NEGATIVE parameter's constraint is
    /// carried by the family's conditions (an <see cref="AssumptionSet"/>), never by the domain,
    /// so <see cref="ParameterDomain.NonNegativeIntegers"/> is the integer domain plus a
    /// condition. The switch is total: an undeclared parameter domain throws instead of silently
    /// inheriting Integers, so adding one is a decision made here rather than an accident.
    /// Internal (not private) because Lovelace.Symbolics.csproj already grants
    /// Lovelace.Symbolics.Tests access, and an arm the language cannot reach must stay pinnable.</summary>
    internal static MathDomain ParameterDomainOf(ParameterDomain domain) => domain switch
    {
        ParameterDomain.Integers => MathDomain.Integer,
        ParameterDomain.NonNegativeIntegers => MathDomain.Integer,
        _ => throw new ArgumentOutOfRangeException(
            nameof(domain), domain, "No wire Domain is declared for this parameter domain."),
    };

    /// <summary>Projects the kernel's one structural binding contract onto the wire as a
    /// <c>Binding</c> record. The NAME is the symbol itself — a Symbolic value with a canonical
    /// form — never the <c>"x = …"</c> string <see cref="Binding"/> exists to forbid.</summary>
    private static RecordValue BindingRecord(Binding binding) =>
        new("Binding",
            new RecordField("name", Exprs.Symbol(binding.Name)),
            new RecordField("value", binding.Value));

    /// <summary>Solutions that survive their own conditions (a value violating a provable
    /// excluded-domain condition, e.g. the pole of a cancelled denominator, is not a solution).</summary>
    private static List<Solution> AcceptedSolutions(SolutionSet set, Symbol x)
    {
        var kept = new List<Solution>();
        foreach (var sol in set.Solutions)
        {
            if (sol.Conditions.IsUnsatisfiable || ViolatesConditions(sol, x))
                continue;
            kept.Add(sol);
        }
        return kept;
    }

    /// <summary>The conditions every solution shares (an intersection, never a union: a union of
    /// branch conditions is not a condition any single branch satisfies).</summary>
    private static AssumptionSet CommonConditions(IReadOnlyList<Solution> solutions)
    {
        if (solutions.Count == 0)
            return AssumptionSet.Empty;
        var common = solutions[0].Conditions.Atoms;
        for (int i = 1; i < solutions.Count && common.Length > 0; i++)
        {
            var other = solutions[i].Conditions;
            common = common.Where(other.Contains).ToImmutableArray();
        }
        return AssumptionSet.FromAtoms(common);
    }

    // -----------------------------------------------------------------
    // ISymbolicMatrixBridge (the D14 Suite↔Symbolics seam)
    // -----------------------------------------------------------------

    /// <summary>Any symbolic element makes the matrix symbolic (numeric elements convert to
    /// constant expressions) — the same dispatch contract the core used before the seam.</summary>
    public bool IsSymbolicMatrix(IReadOnlyList<object?> elements, long[] shape) =>
        shape.Length == 2 && elements.Count > 0 && elements.Any(e => e is Expr);

    private static SymbolicMatrix MatrixOf(IReadOnlyList<object?> elements, long[] shape)
    {
        int rows = checked((int)shape[0]);
        int cols = checked((int)shape[1]);
        var m = new Expr[rows, cols];
        for (int r = 0; r < rows; r++)
            for (int c = 0; c < cols; c++)
                m[r, c] = AsExpr(elements[r * cols + c]!);
        return SymbolicMatrix.From(m);
    }

    private static Expr[] RhsOf(IReadOnlyList<object?> rhs) => rhs.Select(AsExpr).ToArray();

    private static object[][] MatrixPayload(SymbolicMatrix m)
    {
        var rows = new object[m.Rows][];
        for (int r = 0; r < m.Rows; r++)
        {
            rows[r] = new object[m.Columns];
            for (int c = 0; c < m.Columns; c++)
                rows[r][c] = m[r, c];
        }
        return rows;
    }

    /// <summary>ISymbolicInspectionBridge: the active assumptions that constrain the symbols free
    /// in the inspected expression, projected as structured condition leaves (the same projection
    /// every other condition array uses).</summary>
    public object?[] RelevantAssumptions(object expression)
    {
        if (expression is not Expr e)
            return Array.Empty<object?>();
        var names = new HashSet<string>(Printing.FreeSymbolNames(e), StringComparer.Ordinal);
        if (names.Count == 0)
            return Array.Empty<object?>();
        var relevant = _assumptions.Atoms.Where(a => MentionsAny(a, names)).ToArray();
        return relevant.Length == 0 ? Array.Empty<object?>() : ConditionExprs(AssumptionSet.FromAtoms(relevant));
    }

    private static bool MentionsAny(Assumption a, HashSet<string> names) => a switch
    {
        SymbolRelationAssumption r => names.Contains(r.S.Name),
        SymbolPropertyAssumption p => names.Contains(p.S.Name),
        SymbolDomainAssumption d => names.Contains(d.S.Name),
        ExpressionPropertyAssumption e => Printing.FreeSymbolNames(e.E).Any(names.Contains),
        IntervalAssumption i => Printing.FreeSymbolNames(i.E).Any(names.Contains),
        _ => false,
    };

    public object? TryDet(IReadOnlyList<object?> elements, long[] shape) =>
        MatrixOf(elements, shape).Det(Context);

    public object? TryRank(IReadOnlyList<object?> elements, long[] shape) =>
        MatrixOf(elements, shape).Rank(Context);

    public object? TryInverse(IReadOnlyList<object?> elements, long[] shape, out object? conditions)
    {
        var result = MatrixOf(elements, shape).InverseWithConditions(Context);
        conditions = ConditionExprs(result.Conditions);
        return result.Matrix is null ? null : MatrixPayload(result.Matrix);
    }

    public object? TrySolve(IReadOnlyList<object?> elements, long[] shape, IReadOnlyList<object?> rhs, out object? conditions)
    {
        var result = MatrixOf(elements, shape).SolveWithConditions(RhsOf(rhs), Context);
        conditions = ConditionExprs(result.Conditions);
        return result.Vector?.Select(e => (object)e).ToArray();
    }

    public object? TrySolveFull(
        IReadOnlyList<object?> elements, long[] shape, IReadOnlyList<object?> rhs,
        out object? conditions, out string? note, out bool singular)
    {
        var result = MatrixOf(elements, shape).SolveWithConditions(RhsOf(rhs), Context);
        conditions = ConditionExprs(result.Conditions);
        note = result.Note;
        singular = result.Vector is null;
        return result.Vector?.Select(e => (object)e).ToArray();
    }

    private object SymbolWithDomain(string name, MathDomain domain)
    {
        var s = Context.Symbol(name);
        _assumptions = _assumptions.Add(new SymbolDomainAssumption(s, Domains.ToKernelDomain(domain)));
        return Exprs.Symbol(s);
    }

    /// <summary>Marker payload for a provably contradictory condition set (no model exists).
    /// Distinct from an empty condition array, which means "unconditional" — the two must never
    /// be confused, so the contradiction is a value rather than the string "unsatisfiable".</summary>
    internal static RecordValue UnsatisfiableConditions { get; } = new("UnsatisfiableConditions");

    /// <summary>Projects condition atoms into structured payload leaves — never strings. A relation
    /// stays a symbolic relation; a predicate without a relation form becomes an explicit
    /// <c>FiniteCondition</c>/<c>PredicateCondition</c> record; a domain restriction becomes a
    /// <c>DomainCondition</c> carrying a Domain value; an interval becomes an
    /// <c>IntervalCondition</c>; a contradictory set becomes <see cref="UnsatisfiableConditions"/>.
    /// The human-readable <c>finite(x)</c> spelling belongs to the pretty printer, not the payload.</summary>
    private static object?[] ConditionExprs(AssumptionSet conditions)
    {
        if (conditions.IsUnsatisfiable)
            return new object?[] { UnsatisfiableConditions };
        var list = new List<object?>();
        foreach (var a in conditions.Atoms)
            list.Add(ConditionLeaf(a));
        return list.ToArray();
    }

    private static object ConditionLeaf(Assumption a) => a switch
    {
        SymbolRelationAssumption sr => Exprs.Relation(sr.Op, Exprs.Symbol(sr.S), sr.Bound),
        SymbolPropertyAssumption sp =>
            PredicateLeaf(PropertyRelation(Exprs.Symbol(sp.S), sp.P), sp.P, Exprs.Symbol(sp.S)),
        ExpressionPropertyAssumption ep =>
            PredicateLeaf(PropertyRelation(ep.E, ep.P), ep.P, ep.E),
        SymbolDomainAssumption sd => new RecordValue("DomainCondition",
            new RecordField("variable", Exprs.Symbol(sd.S)),
            new RecordField("domain", Domains.ToMathDomain(sd.D))),
        IntervalAssumption iv => new RecordValue("IntervalCondition",
            new RecordField("expression", iv.E),
            new RecordField("lower", iv.Lower),
            new RecordField("lower_open", iv.LowerOpen),
            new RecordField("upper", iv.Upper),
            new RecordField("upper_open", iv.UpperOpen)),
        _ => throw new InvalidOperationException(
            $"Condition atom '{a.GetType().Name}' has no structured projection. Add one to " +
            "SymbolicsPlugin.ConditionLeaf instead of letting a string reach the machine API."),
    };

    /// <summary>A predicate leaf: the equivalent relation when one exists (NonZero ⇒ <c>x != 0</c>),
    /// otherwise an explicit predicate record, so the payload never degrades to text.</summary>
    private static object PredicateLeaf(Expr? relation, SymbolPredicate predicate, Expr subject)
    {
        if (relation is not null)
            return relation;
        if (predicate == SymbolPredicate.Finite)
            return new RecordValue("FiniteCondition", new RecordField("expression", subject));
        return new RecordValue("PredicateCondition",
            new RecordField("predicate", predicate.ToString()),
            new RecordField("expression", subject));
    }

    /// <summary>Rendering form of a predicate: the equivalent relation when one exists, so the
    /// pretty view reads <c>x != 0</c>; the payload itself stays a structured predicate record
    /// (<see cref="ConditionLeaf"/>). The operator mapping is shared with the kernel.</summary>
    private static Expr? PropertyRelation(Expr e, SymbolPredicate p) =>
        AssumptionSet.RelationOp(p) is { } op ? Exprs.Relation(op, e, Exprs.Zero) : null;

    /// <summary>The constraint a one-sided value holds under — x &lt; x0 from the left,
    /// x &gt; x0 from the right. Attached only when the two-sided limit does not exist, because
    /// then the value genuinely holds on that side alone; a two-sided value needs no condition.</summary>
    private static object?[] SideConditions(
        Symbol x, Expr point, bool fromRight, LimitResult? side, bool twoSidedExists)
    {
        if (twoSidedExists || side is null || side.Status == LimitStatus.Unevaluated)
            return Array.Empty<object?>();
        return new object?[]
        {
            Exprs.Relation(fromRight ? RelOp.Gt : RelOp.Lt, Exprs.Symbol(x), point),
        };
    }

    /// <summary>Stable optimization-policy token: the enabled flags in a fixed order.</summary>
    private static string PolicyToken(OptimizeOptions o)
    {
        var parts = new List<string>();
        if (o.CommonSubexpressionElimination) parts.Add("cse");
        if (o.Horner) parts.Add("horner");
        if (o.PrecisionAware) parts.Add("precision-aware");
        if (o.PowerReduction) parts.Add("power-reduction");
        return parts.Count == 0 ? "none" : string.Join("+", parts);
    }

    private static object? LimitSide(LimitResult? r) => r?.Status switch
    {
        LimitStatus.Value => r.Value,
        LimitStatus.PlusInfinity => Exprs.Infinity,
        LimitStatus.MinusInfinity => Exprs.Negate(Exprs.Infinity),
        _ => null,
    };

    private object Run(Func<object?> impl)
    {
        var previous = Exprs.Current;
        Exprs.Current = Context;
        try
        {
            // install the pre-call set for the builtin's own reads, then re-sync after the call
            // so assumption updates made during the call (assume/assume_*) become visible
            // immediately — including to direct (non-builtin) kernel callers like Studio
            Context.Assumptions = _assumptions;
            var result = impl() ?? throw new InvalidOperationException("Symbolic builtin returned null.");
            Context.Assumptions = _assumptions;
            return result;
        }
        finally
        {
            Exprs.Current = previous;   // restore the ambient context: never leak across calls
        }
    }

    private object AssumePred(IReadOnlyList<object?> args, SymbolPredicate pred)
    {
        if (args[0] is SymbolExpr or Expr)
        {
            var e = AsExpr(args[0]);
            if (e is SymbolExpr sx)
            {
                _assumptions = _assumptions.Add(new SymbolPropertyAssumption(sx.Symbol, pred));
                return e;
            }
            _assumptions = _assumptions.Add(new ExpressionPropertyAssumption(e, pred));
            return e;
        }
        var s = Context.Symbol((string)args[0]!);
        _assumptions = _assumptions.Add(new SymbolPropertyAssumption(s, pred));
        return Exprs.Symbol(s);
    }

    private object AssumeDomain(IReadOnlyList<object?> args, Domain domain)
    {
        if (args[0] is Expr e && e is SymbolExpr sx)
        {
            _assumptions = _assumptions.Add(new SymbolDomainAssumption(sx.Symbol, domain));
            return e;
        }
        var s = Context.Symbol((string)args[0]!);
        _assumptions = _assumptions.Add(new SymbolDomainAssumption(s, domain));
        return Exprs.Symbol(s);
    }

    private static object NumToPayload(Num n) => n switch
    {
        NumInt i => i.V,
        // the Modus payload vocabulary carries Real (exact periodic), not Rational
        NumRat r => r.V.IsInteger ? r.V.ToInteger() : RationalReal.ToReal(r.V, (int)Math.Min(Rl.MaxComputationDecimalPlaces, 1000)),
        NumReal rl => rl.V,
        NumComplex c => c.V,
        _ => throw new InvalidOperationException(),
    };

    private static object LimitToExpr(LimitResult r) => r.Status switch
    {
        LimitStatus.Value => r.Value!,
        LimitStatus.PlusInfinity => Exprs.Infinity,
        LimitStatus.MinusInfinity => Exprs.Negate(Exprs.Infinity),
        LimitStatus.DoesNotExist =>
            $"does not exist (left: {SideText(r.FromLeft)}, right: {SideText(r.FromRight)})",
        _ => "unevaluated: " + (r.FailureReason ?? "no limit"),
    };

    private static string SideText(LimitResult? r) => r?.Status switch
    {
        LimitStatus.PlusInfinity => "+inf",
        LimitStatus.MinusInfinity => "-inf",
        LimitStatus.Value => Printing.PrettyPrint(r.Value!),
        _ => "unknown",
    };

    /// <summary>True when the solution provably violates one of its excluded-domain conditions.</summary>
    private static bool ViolatesConditions(Solution sol, Symbol x)
    {
        foreach (var atom in sol.Conditions.Atoms)
        {
            if (atom is ExpressionPropertyAssumption { P: SymbolPredicate.NonZero } ep)
            {
                var sub = Evaluation.Substitute(ep.E, Exprs.Current,
                    new Dictionary<Symbol, Expr> { [x] = sol.Value });
                if (Evaluation.ConstantToNum(sub) is { } nv && NumOps.IsZero(nv))
                    return true;
            }
        }
        return false;
    }

    /// <summary>Argument list that records which position a coercion helper last read, so a
    /// failed coercion can be reported as
    /// <c>diff(): argument 2 must be a symbolic variable; got Integer.</c> without threading the
    /// index through every builtin implementation.</summary>
    private sealed class BuiltinArgs : IReadOnlyList<object?>
    {
        private readonly IReadOnlyList<object?> _inner;
        public BuiltinArgs(IReadOnlyList<object?> inner) => _inner = inner;

        /// <summary>Index most recently read through the indexer, or -1.</summary>
        public int LastIndex { get; private set; } = -1;

        public object? this[int index]
        {
            get { LastIndex = index; return _inner[index]; }
        }

        public int Count => _inner.Count;
        public IEnumerator<object?> GetEnumerator() => _inner.GetEnumerator();
        IEnumerator IEnumerable.GetEnumerator() => GetEnumerator();
    }

    /// <summary>Coercion failure carrying only the expectation; the registration wrapper adds the
    /// function name, the argument position and the actual payload kind.</summary>
    private sealed class BuiltinArgumentError(string expected) : Exception
    {
        public string Expected { get; } = expected;
    }

    /// <summary>A wrong-SHAPED argument: the value supplied is not the KIND of thing the builtin's
    /// declared parameter names — a relation where a list of equations is required, a bare symbol
    /// where a parameter list is required, an element that is not an expression. It carries the same
    /// expectation text as <see cref="BuiltinArgumentError"/>, but derives from
    /// <see cref="ArgumentException"/> so the runner's taxonomy classifies the call as the
    /// documented RECOVERABLE argument error (<c>code: InvalidArgument</c>,
    /// <c>category: TypeMismatch</c>) rather than a domain error or — as the direct cast this
    /// replaced did — an internal invariant failure.</summary>
    private sealed class BuiltinShapeError(string expected) : ArgumentException
    {
        public string Expected { get; } = expected;
    }

    private static string DescribeArgument(string name, BuiltinArgs args, string expected)
    {
        int index = args.LastIndex;
        object? actual = index >= 0 && index < args.Count ? args[index] : null;
        return $"{name}(): argument {index + 1} must be {expected}; got {DescribePayload(actual)}.";
    }

    /// <summary>The payload kind as an agent sees it: the value-kind vocabulary, not a CLR type name.</summary>
    private static string DescribePayload(object? payload) => payload switch
    {
        null => "nothing",
        Expr => "Symbolic",
        Rl => "Real",          // Real inherits Integer: check before Int
        Int => "Integer",
        Nat => "Natural",
        Cplx => "Complex",
        bool => "Boolean",
        string => "Text",
        MathDomain => "Domain",
        RecordValue => "Record",
        IReadOnlyList<object?> => "Vector",
        _ => payload.GetType().Name,
    };

    private static Symbol AsSymbol(object? o)
    {
        if (o is Expr e && e is SymbolExpr sx)
            return sx.Symbol;
        if (o is string s)
            return Exprs.Current.Symbol(s);
        throw new BuiltinArgumentError("a symbolic variable");
    }

    private static Expr AsExpr(object? o) => o switch
    {
        Expr e => e,
        Rl r => RealToExpr(r),                 // Real inherits Integer: check before Int!
        Int i => Exprs.Integer(i),
        Nat n => Exprs.Integer(new Int(n)),
        Cplx c => Exprs.Add(RealToExpr(c.Re), Exprs.Multiply(RealToExpr(c.Im), Exprs.I)),
        string s => Exprs.Symbol(s),
        _ => throw new BuiltinArgumentError("a symbolic expression"),
    };

    private static Expr RealToExpr(Rl r)
    {
        if (r.IsPeriodic || -r.Exponent <= 18)
            return Exprs.Rational(RationalReal.FromReal(r));
        return Exprs.Real(RealLiteral.FromRealExact(r));
    }

    private static int AsInt(object? o) => (int)AsLong(o);

    private static long AsLong(object? o) => o switch
    {
        Rl r => long.Parse(r.ToString().Split('.')[0]),   // Real inherits Integer: check first
        Nat n => long.Parse(n.ToString()),
        Int i => long.Parse(i.ToString()),
        long l => l,
        _ => throw new BuiltinArgumentError("an integer"),
    };
}
