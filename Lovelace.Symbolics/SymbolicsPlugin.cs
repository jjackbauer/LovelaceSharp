using Lovelace.Abstractions;
using Rat = global::Lovelace.Rational.Rational;
using Int = global::Lovelace.Integer.Integer;
using Nat = global::Lovelace.Natural.Natural;
using Rl = global::Lovelace.Real.Real;
using Cplx = global::Lovelace.Complex.Complex;

namespace Lovelace.Symbolics;

/// <summary>
/// The language-facing surface of the symbolic kernel: a compile-time-linked Modus plugin
/// (the DspPlugin pattern) that registers the CAS builtins. Symbolic values cross the Modus
/// payload boundary as <see cref="Expr"/> objects via the Suite core bridge
/// (ValueKind.Symbolic).
/// </summary>
public sealed class SymbolicsPlugin : IModusPlugin, ISymbolicMatrixBridge
{
    public string Name => "Lovelace.Symbolics";

    public ExprContext Context { get; } = new();

    private AssumptionSet _assumptions = AssumptionSet.Empty;

    public void Register(IModusContext c)
    {
        // the D14 seam: core inv/linsolve/matrix_rank/det dispatch symbolic matrices here
        c.RegisterSymbolicMatrixBridge(this);

        void Add(string name, string[] parameters, Func<IReadOnlyList<object?>, object?> impl, BuiltinDescriptor? descriptor = null)
            => c.RegisterBuiltin(
                descriptor ?? new BuiltinDescriptor(name, parameters, BuiltinCategories.Symbolics, "(no summary registered)", Array.Empty<string>(), "Symbolic"),
                args => Run(() => impl(args)));

        Add("symbol", new[] { "name", "domain" }, args =>
            args.Count >= 2 && args[1] is MathDomain md
                ? SymbolWithDomain((string)args[0]!, md)
                : Exprs.Symbol((string)args[0]!),
            new BuiltinDescriptor("symbol", new[] { "name", "domain" }, BuiltinCategories.Symbolics,
                "Creates a symbolic variable; an optional domain (integer/rational/real/complex) is assumed for it.",
                ["symbol(\"x\")", "symbol(\"x\", real)"], "Symbolic", ["assume", "real", "complex"]));
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

        // elementary functions as symbolic builtins (numeric versions do not exist in the core)
        foreach (var fn in new[] { "exp", "log", "sin", "cos", "tan", "asin", "acos", "atan", "sinh", "cosh", "tanh" })
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
            ["assume(x > 5)", "assume(x > 0 and x < 10)"], "Symbolic",
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
                _ => a.ToString(),
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
                new RecordField("status", r.Kind.ToString()),
                new RecordField("expression", r.Expression),
                new RecordField("conditions", ConditionExprs(r.Conditions)),
                new RecordField("verified", r.Kind != IntegrationKind.Unevaluated),
                new RecordField("diagnostics", r.Note ?? ""));
        },
        new BuiltinDescriptor("integrate_full", new[] { "f", "x" }, BuiltinCategories.Calculus,
            "Structured integration: an IntegrationResult record with status (SolvedExact/SolvedConditional/Unevaluated), the antiderivative, its conditions, and the self-verification flag.",
            ["integrate_full(x^2, x)"], "IntegrationResult", ["integrate"]));
        Add("simplify", new[] { "f" }, args =>
            Simplify.SimplifyExpr(AsExpr(args[0]), Context),
        new BuiltinDescriptor("simplify", new[] { "f" }, BuiltinCategories.Symbolics,
            "Safe simplification: applies only universal rules and rules whose side conditions are already provable from the active assumptions (conditions are never silently discarded).",
            ["simplify(sin(x)^2 + cos(x)^2)", "simplify(x/x)"], "Symbolic", ["simplify_full", "expand", "factor"]));
        Add("simplify_full", new[] { "f" }, args =>
        {
            var e = AsExpr(args[0]);
            var r = Simplify.Transform(e, Context, new Simplify.Options(Trace: true));
            return new RecordValue("TransformResult",
                new RecordField("expression", r.Expression),
                new RecordField("changed", !r.Expression.Equals(e)),
                new RecordField("conditions", ConditionExprs(r.Conditions)),
                new RecordField("steps", r.Steps.Select(s => (object)new RecordValue("RewriteStep",
                    new RecordField("rule_id", s.RuleId),
                    new RecordField("classification", s.Classification.ToString()),
                    new RecordField("before", s.Before),
                    new RecordField("after", s.After),
                    new RecordField("required_conditions", ConditionExprs(s.Conditions)))).ToArray()),
                new RecordField("budget_exceeded", r.BudgetExceeded));
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
            "Cancels common polynomial factors in a rational function.", ["cancel((x^2 - 1)/(x - 1))"], "Symbolic", ["apart", "factor"]));
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
            var r = Limits.Limit(AsExpr(args[0]), AsSymbol(args[1]), AsExpr(args[2]), LimitDirection.TwoSided, Context);
            bool exists = r.Status is LimitStatus.Value or LimitStatus.PlusInfinity or LimitStatus.MinusInfinity;
            return new RecordValue("LimitResult",
                new RecordField("status", r.Status.ToString()),
                new RecordField("exists", exists),
                new RecordField("value", LimitSide(r)),
                new RecordField("left", LimitSide(r.FromLeft)),
                new RecordField("right", LimitSide(r.FromRight)),
                new RecordField("diagnostics", r.FailureReason ?? ""));
        },
        new BuiltinDescriptor("limit_full", new[] { "f", "x", "x0" }, BuiltinCategories.Calculus,
            "Structured limit: a LimitResult record with status, exists, value, and the left/right one-sided values.",
            ["limit_full(1/x, x, 0)"], "LimitResult", ["limit", "limit_left", "limit_right"]));
        Add("limit_left", new[] { "f", "x", "x0" }, args =>
            LimitToExpr(Limits.Limit(AsExpr(args[0]), AsSymbol(args[1]), AsExpr(args[2]), LimitDirection.FromLeft, Context)));
        Add("limit_right", new[] { "f", "x", "x0" }, args =>
            LimitToExpr(Limits.Limit(AsExpr(args[0]), AsSymbol(args[1]), AsExpr(args[2]), LimitDirection.FromRight, Context)));
        Add("solve_system", new[] { "eqs", "vars" }, args =>
        {
            var eqs = ((IReadOnlyList<object?>)args[0]!).Select(AsExpr).ToArray();
            var vs = NameList(args[1]).Select(Context.Symbol).ToArray();
            var result = SystemSolvers.Solve(eqs, vs, Context);
            if (result.Solutions.Count == 0)
                return "no solutions" + (result.Note is { } note ? ": " + note : "");
            return string.Join("; ", result.Solutions.Select(sol =>
                string.Join(", ", vs.Select(v => v.Name + " = " + Printing.PrettyPrint(sol.Assignment[v])))));
        },
        new BuiltinDescriptor("solve_system", new[] { "eqs", "vars" }, BuiltinCategories.Solving,
            "Solves a polynomial equation system (Gröbner elimination into triangular form, verified solutions).",
            ["solve_system([x^2 + y^2 - 1 == 0, x*y == 0], [x, y])"], "Text",
            ["solve_system_full", "solve"]));
        Add("solve_system_full", new[] { "eqs", "vars" }, args =>
        {
            var eqs = ((IReadOnlyList<object?>)args[0]!).Select(AsExpr).ToArray();
            var vs = NameList(args[1]).Select(Context.Symbol).ToArray();
            var result = SystemSolvers.Solve(eqs, vs, Context);
            var solutions = result.Solutions.Select(sol => (object)new RecordValue("SystemSolution",
                new RecordField("assignment", vs.Select(v => (object)(v.Name + " = " + Printing.PrettyPrint(sol.Assignment[v]))).ToArray()),
                new RecordField("conditions", ConditionExprs(sol.Conditions)))).ToArray();
            return new RecordValue("SystemSolveResult",
                new RecordField("status", result.Solutions.Count > 0 ? "Solved" : (result.Note is null ? "NoSolutions" : "Unevaluated")),
                new RecordField("solutions", solutions),
                new RecordField("diagnostics", result.Note ?? ""));
        });
        Add("solve", new[] { "f", "x", "domain" }, args =>
        {
            var fx = AsExpr(args[0]);
            var sx = AsSymbol(args[1]);
            var domain = SolveDomainOf(args);
            var set = Solvers.Solve(fx, sx, Context, domain);
            if (set.Kind == SolutionKind.Exact)
            {
                // conditions are part of the solution: a value violating a provable
                // excluded-domain condition (e.g. the pole of a cancelled denominator) is dropped
                var kept = new List<Expr>();
                foreach (var sol in set.Solutions)
                {
                    if (ViolatesConditions(sol, sx))
                        continue;
                    kept.Add(sol.Value);
                }
                if (kept.Count > 0)
                    return (object)kept.ToArray();
                if (set.Families.Count > 0)
                    return string.Join("; ", set.Families.Select(f =>
                        Printing.PrettyPrint(f.Template) + " for integer " + f.Parameter.Name));
                return "no solutions";
            }
            if (set.Kind == SolutionKind.Empty)
                return set.Note ?? "no solutions";
            // Unevaluated: prefer the diagnostic note (e.g. the domain-honesty message for
            // deg ≥ 4 complex roots); fall back to the equation itself when there is no note
            if (set.Note is { } note)
                return "unevaluated: " + note;
            return Printing.PrettyPrint(fx);
        },
        new BuiltinDescriptor("solve", new[] { "f", "x", "domain" }, BuiltinCategories.Solving,
            "Solves an equation for x. The default domain is Complex (deg ≤ 3 radicals are complex-capable; deg ≥ 4 complex algebraic roots are reported unevaluated); pass real for real solutions only. Use solve_full for the structured result.",
            ["solve(x^2 - 4 == 0, x)", "solve(x^2 + 1 == 0, x, real)"], "Vector | Text",
            ["solve_full", "solve_system", "linsolve"]));
        Add("solve_full", new[] { "f", "x", "domain" }, args =>
        {
            var fx = AsExpr(args[0]);
            var sx = AsSymbol(args[1]);
            var domain = SolveDomainOf(args);
            var set = Solvers.Solve(fx, sx, Context, domain);
            var solutions = new List<object?>();
            if (set.Kind == SolutionKind.Exact)
            {
                foreach (var sol in set.Solutions)
                {
                    if (sol.Conditions.IsUnsatisfiable || ViolatesConditions(sol, sx))
                        continue;
                    solutions.Add(sol.Value);
                }
            }
            var families = set.Families.Select(f => (object)new RecordValue("SolutionFamily",
                new RecordField("template", f.Template),
                new RecordField("parameter", f.Parameter.Name),
                new RecordField("period", f.Period),
                new RecordField("parameter_domain", f.Domain.ToString()))).ToArray();
            return new RecordValue("SolveResult",
                new RecordField("status", set.Kind switch
                {
                    SolutionKind.Exact => "Solved",
                    SolutionKind.Empty => "NoSolutions",
                    _ => "Unevaluated",
                }),
                new RecordField("variable", sx.Name),
                new RecordField("domain", domain.ToString()),
                new RecordField("solutions", solutions.ToArray()),
                new RecordField("conditions", ConditionExprs(set.Kind == SolutionKind.Exact
                    ? UnionConditions(set.Solutions)
                    : AssumptionSet.Empty)),
                new RecordField("families", families),
                new RecordField("diagnostics", set.Note ?? ""));
        },
        new BuiltinDescriptor("solve_full", new[] { "f", "x", "domain" }, BuiltinCategories.Solving,
            "Structured solve: a SolveResult record with status, domain, solutions, side conditions, parametric families, and diagnostics.",
            ["solve_full(x^2 - 4 == 0, x)", "solve_full((x^2 - 1)/(x - 1) == 0, x)"], "SolveResult",
            ["solve", "solve_system_full"]));
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
            var fs = ((IReadOnlyList<object?>)args[0]!).Select(AsExpr).ToArray();
            var vars = NameList(args[1]).Select(Context.Symbol).ToArray();
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
            return new RecordValue("OptimizationResult",
                new RecordField("original", f),
                new RecordField("optimized", r.Expression),
                new RecordField("estimated_cost_before", r.NodesBefore),
                new RecordField("estimated_cost_after", r.NodesAfter),
                new RecordField("shared_subtrees", r.SharedSubtrees),
                new RecordField("horner_rewrites", r.HornerRewrites),
                new RecordField("target", "mathir"));
        });
    }

    private static IEnumerable<string> NameList(object? o)
    {
        var list = (IReadOnlyList<object?>)o!;
        foreach (var n in list)
        {
            if (n is string s)
                yield return s;
            else if (n is SymbolExpr sx)
                yield return sx.Symbol.Name;
            else
                throw new InvalidOperationException("Expected symbol names.");
        }
    }

    /// <summary>Adds assumption atoms: relations, conjunctions of relations, and negations of
    /// relations (mapped to their complement relation). Disjunctions are rejected: the atom
    /// lattice has no disjunction.</summary>
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

    private static SolveDomain SolveDomainOf(IReadOnlyList<object?> args) =>
        args.Count >= 3 && args[2] is MathDomain { } md && md == MathDomain.Real
            ? SolveDomain.Real
            : SolveDomain.Complex;

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

    private static Domain ToKernelDomain(MathDomain d) => d switch
    {
        MathDomain.Integer => Domain.Integer,
        MathDomain.Rational => Domain.Rational,
        MathDomain.Real => Domain.Real,
        _ => Domain.Complex,
    };

    private object SymbolWithDomain(string name, MathDomain domain)
    {
        var s = Context.Symbol(name);
        _assumptions = _assumptions.Add(new SymbolDomainAssumption(s, ToKernelDomain(domain)));
        return Exprs.Symbol(s);
    }

    /// <summary>Renders condition atoms as symbolic relations where representable (the
    /// structured view); non-relation atoms fall back to their text form.</summary>
    private static object?[] ConditionExprs(AssumptionSet conditions)
    {
        if (conditions.IsUnsatisfiable)
            return new object?[] { "unsatisfiable" };
        var list = new List<object?>();
        foreach (var a in conditions.Atoms)
        {
            switch (a)
            {
                case SymbolRelationAssumption sr:
                    list.Add(Exprs.Relation(sr.Op, Exprs.Symbol(sr.S), sr.Bound));
                    break;
                case SymbolPropertyAssumption { P: SymbolPredicate.Finite } sf:
                    list.Add("finite(" + sf.S.Name + ")");
                    break;
                case ExpressionPropertyAssumption { P: SymbolPredicate.Finite } ef:
                    list.Add("finite(" + Printing.PrettyPrint(ef.E) + ")");
                    break;
                case SymbolPropertyAssumption sp:
                    list.Add((object?)PropertyRelation(Exprs.Symbol(sp.S), sp.P) ?? a.ToString());
                    break;
                case ExpressionPropertyAssumption ep:
                    list.Add((object?)PropertyRelation(ep.E, ep.P) ?? a.ToString());
                    break;
                default:
                    list.Add(a.ToString());
                    break;
            }
        }
        return list.ToArray();
    }

    private static Expr? PropertyRelation(Expr e, SymbolPredicate p) => p switch
    {
        SymbolPredicate.NonZero => Exprs.Relation(RelOp.Ne, e, Exprs.Zero),
        SymbolPredicate.Positive => Exprs.Relation(RelOp.Gt, e, Exprs.Zero),
        SymbolPredicate.Negative => Exprs.Relation(RelOp.Lt, e, Exprs.Zero),
        SymbolPredicate.NonNegative => Exprs.Relation(RelOp.Ge, e, Exprs.Zero),
        SymbolPredicate.NonPositive => Exprs.Relation(RelOp.Le, e, Exprs.Zero),
        _ => null,
    };

    private static AssumptionSet UnionConditions(IEnumerable<Solution> solutions)
    {
        var result = AssumptionSet.Empty;
        foreach (var sol in solutions)
        {
            foreach (var atom in sol.Conditions.Atoms)
            {
                try { result = result.Add(atom); }
                catch (AssumptionContradictionException) { return AssumptionSet.Unsatisfiable; }
            }
        }
        return result;
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

    private static Symbol AsSymbol(object? o)
    {
        if (o is Expr e && e is SymbolExpr sx)
            return sx.Symbol;
        if (o is string s)
            return Exprs.Current.Symbol(s);
        throw new InvalidOperationException($"Expected a symbolic symbol, got {o?.GetType().Name ?? "null"}.");
    }

    private static Expr AsExpr(object? o) => o switch
    {
        Expr e => e,
        Rl r => RealToExpr(r),                 // Real inherits Integer: check before Int!
        Int i => Exprs.Integer(i),
        Nat n => Exprs.Integer(new Int(n)),
        Cplx c => Exprs.Add(RealToExpr(c.Re), Exprs.Multiply(RealToExpr(c.Im), Exprs.I)),
        string s => Exprs.Symbol(s),
        _ => throw new InvalidOperationException($"Cannot convert payload of type {o?.GetType().Name} to an expression."),
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
        _ => throw new InvalidOperationException("Expected an integer."),
    };
}
