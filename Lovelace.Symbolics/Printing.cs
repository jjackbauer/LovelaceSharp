using System.Collections.Immutable;
using System.Text;
using Int = global::Lovelace.Integer.Integer;
using Rat = global::Lovelace.Rational.Rational;

namespace Lovelace.Symbolics;

/// <summary>
/// Canonical, versioned, round-trippable text form (§6.7 of the architecture). The canonical
/// form is a typed prefix (S-expression) format — deterministic, whitespace-insensitive, and
/// independent of the pretty-printer.
/// </summary>
public static class Printing
{
    public const string FormatHeader = "#!lovelace-sym 1";

    // -----------------------------------------------------------------
    // Canonical print
    // -----------------------------------------------------------------

    public static string CanonicalPrint(Expr e) => PrintExpr(e);

    private static string PrintExpr(Expr e) => e switch
    {
        IntegerConstantExpr i => "(int " + i.Value + ")",
        RationalConstantExpr r => "(rat " + r.Value.Numerator + " " + r.Value.Denominator + ")",
        RealConstantExpr rl => "(real " + rl.Value + ")",
        ComplexConstantExpr c => "(cplx " + c.Re.Numerator + " " + c.Re.Denominator + " " + c.Im.Numerator + " " + c.Im.Denominator + ")",
        SymbolExpr s => "(sym " + s.Symbol.Name + ")",
        NamedConstantExpr n => n.Constant switch
        {
            NamedConstant.Pi => "(pi)",
            NamedConstant.E => "(e)",
            NamedConstant.I => "(i)",
            _ => "(inf)",
        },
        AddExpr a => "(add " + string.Join(" ", a.Terms.Select(PrintExpr)) + ")",
        MultiplyExpr m => "(mul " + string.Join(" ", m.Factors.Select(PrintExpr)) + ")",
        PowerExpr p => "(pow " + PrintExpr(p.Base) + " " + PrintExpr(p.Exponent) + ")",
        FunctionExpr f => "(fn " + f.Function.Name + " " + string.Join(" ", f.Arguments.Select(PrintExpr)) + ")",
        RelationExpr r => "(" + OpTag(r.Op) + " " + PrintExpr(r.Left) + " " + PrintExpr(r.Right) + ")",
        PiecewiseExpr pw => "(pw " + string.Join(" ", pw.Branches.Select(b => "(" + PrintExpr(b.Guard) + " " + PrintExpr(b.Value) + ")")) + " " + PrintExpr(pw.Otherwise) + ")",
        DerivativeExpr d => "(der (" + string.Join(" ", d.Variables.Select(v => v.Name)) + ") " + PrintExpr(d.Operand) + ")",
        IntegralExpr i => "(integ (" + string.Join(" ", i.Variables.Select(v => v.Name)) + ") " + PrintExpr(i.Operand) + ")",
        RootOfExpr r => "(rootof " + PrintExpr(r.DefiningPolynomial.ToExpr()) + " " + r.RootIndex + ")",
        AndExpr an => "(and " + string.Join(" ", an.Operands.Select(PrintExpr)) + ")",
        OrExpr or2 => "(or " + string.Join(" ", or2.Operands.Select(PrintExpr)) + ")",
        NotExpr nt => "(not " + PrintExpr(nt.Operand) + ")",
        OrderExpr o => "(order " + PrintExpr(o.Variable) + " " + PrintExpr(o.Point) + " " + PrintExpr(o.Degree) + ")",
        _ => throw new InvalidOperationException($"Unknown node kind {e.Kind}."),
    };

    private static string OpTag(RelOp op) => op switch
    {
        RelOp.Eq => "eq",
        RelOp.Ne => "ne",
        RelOp.Lt => "lt",
        RelOp.Le => "le",
        RelOp.Gt => "gt",
        RelOp.Ge => "ge",
        _ => throw new InvalidOperationException(),
    };

    // -----------------------------------------------------------------
    // Canonical parse
    // -----------------------------------------------------------------

    public static Expr CanonicalParse(string text, ExprContext ctx)
    {
        var tokens = Tokenize(text);
        int pos = 0;
        var result = ParseExpr(tokens, ref pos, ctx);
        if (pos != tokens.Count)
            throw new FormatException($"Trailing tokens at position {pos}.");
        return result;
    }

    private static List<string> Tokenize(string text)
    {
        var tokens = new List<string>();
        var sb = new StringBuilder();
        void Flush()
        {
            if (sb.Length > 0)
            {
                tokens.Add(sb.ToString());
                sb.Clear();
            }
        }
        foreach (var ch in text)
        {
            if (char.IsWhiteSpace(ch) || ch == '(' || ch == ')')
            {
                Flush();
                if (ch == '(') tokens.Add("(");
                else if (ch == ')') tokens.Add(")");
            }
            else
            {
                sb.Append(ch);
            }
        }
        Flush();
        return tokens;
    }

    private static Expr ParseExpr(List<string> tokens, ref int pos, ExprContext ctx)
    {
        if (pos >= tokens.Count)
            throw new FormatException("Unexpected end of input.");
        var tok = tokens[pos++];
        if (tok != "(")
        {
            // bare atoms are not part of the canonical grammar (everything is tagged)
            throw new FormatException($"Expected '(', got '{tok}'.");
        }

        var tag = tokens[pos++];
        switch (tag)
        {
            case "int":
            {
                var e = Exprs.Integer(Int.Parse(tokens[pos++], null));
                ExpectClose(tokens, ref pos);
                return e;
            }
            case "rat":
            {
                var n = Int.Parse(tokens[pos++], null);
                var d = Int.Parse(tokens[pos++], null);
                var e = Exprs.Rational(Rat.From(n, d));
                ExpectClose(tokens, ref pos);
                return e;
            }
            case "real":
            {
                var e = Exprs.Real(RealLiteral.Parse(tokens[pos++]));
                ExpectClose(tokens, ref pos);
                return e;
            }
            case "cplx":
            {
                var rn = Int.Parse(tokens[pos++], null);
                var rd = Int.Parse(tokens[pos++], null);
                var imn = Int.Parse(tokens[pos++], null);
                var imd = Int.Parse(tokens[pos++], null);
                var e = Exprs.Complex(Rat.From(rn, rd), Rat.From(imn, imd));
                ExpectClose(tokens, ref pos);
                return e;
            }
            case "sym":
            {
                var e = Exprs.Symbol(tokens[pos++]);
                ExpectClose(tokens, ref pos);
                return e;
            }
            case "pi": ExpectClose(tokens, ref pos); return Exprs.Pi;
            case "e": ExpectClose(tokens, ref pos); return Exprs.E;
            case "i": ExpectClose(tokens, ref pos); return Exprs.I;
            case "inf": ExpectClose(tokens, ref pos); return Exprs.Infinity;
            case "add":
            {
                var terms = new List<Expr>();
                while (tokens[pos] != ")")
                    terms.Add(ParseExpr(tokens, ref pos, ctx));
                pos++;
                return Exprs.Add(terms);
            }
            case "mul":
            {
                var factors = new List<Expr>();
                while (tokens[pos] != ")")
                    factors.Add(ParseExpr(tokens, ref pos, ctx));
                pos++;
                return Exprs.Multiply(factors);
            }
            case "pow":
            {
                var b = ParseExpr(tokens, ref pos, ctx);
                var e = ParseExpr(tokens, ref pos, ctx);
                ExpectClose(tokens, ref pos);
                return Exprs.Power(b, e);
            }
            case "fn":
            {
                var name = tokens[pos++];
                var args = new List<Expr>();
                while (tokens[pos] != ")")
                    args.Add(ParseExpr(tokens, ref pos, ctx));
                pos++;
                return Exprs.Function(ctx.Function(name), args.ToArray());
            }
            case "eq" or "ne" or "lt" or "le" or "gt" or "ge":
            {
                var op = tag switch
                {
                    "eq" => RelOp.Eq,
                    "ne" => RelOp.Ne,
                    "lt" => RelOp.Lt,
                    "le" => RelOp.Le,
                    "gt" => RelOp.Gt,
                    _ => RelOp.Ge,
                };
                var l = ParseExpr(tokens, ref pos, ctx);
                var r = ParseExpr(tokens, ref pos, ctx);
                ExpectClose(tokens, ref pos);
                return Exprs.Relation(op, l, r);
            }
            case "pw":
            {
                var branches = new List<PiecewiseBranch>();
                while (tokens[pos] != ")")
                {
                    if (tokens[pos] != "(")
                        throw new FormatException("Piecewise branch must be a pair.");
                    pos++;
                    var g = ParseExpr(tokens, ref pos, ctx);
                    var v = ParseExpr(tokens, ref pos, ctx);
                    ExpectClose(tokens, ref pos);
                    branches.Add(new PiecewiseBranch(g, v));
                }
                pos++;
                var otherwise = ParseExpr(tokens, ref pos, ctx);
                return Exprs.Piecewise(branches, otherwise);
            }
            case "der" or "integ":
            {
                var vars = new List<Symbol>();
                if (tokens[pos] != "(")
                    throw new FormatException("Expected variable list.");
                pos++;
                while (tokens[pos] != ")")
                    vars.Add(ctx.Symbol(tokens[pos++]));
                pos++;
                var operand = ParseExpr(tokens, ref pos, ctx);
                ExpectClose(tokens, ref pos);
                return tag == "der"
                    ? Exprs.Derivative(operand, vars.ToArray())
                    : Exprs.Integral(operand, vars.ToArray());
            }
            case "rootof":
            {
                var polyExpr = ParseExpr(tokens, ref pos, ctx);
                var index = int.Parse(tokens[pos++], System.Globalization.CultureInfo.InvariantCulture);
                ExpectClose(tokens, ref pos);
                var vars = CollectSymbols(polyExpr);
                if (!Polynomial.TryFromExpr(polyExpr, ctx, vars, out var poly, out _))
                    throw new FormatException("RootOf operand is not a polynomial.");
                return Exprs.RootOf(poly, index);
            }
            case "and" or "or":
            {
                var ops = new List<Expr>();
                while (tokens[pos] != ")")
                    ops.Add(ParseExpr(tokens, ref pos, ctx));
                pos++;
                return tag == "and" ? Exprs.And(ops) : Exprs.Or(ops);
            }
            case "not":
            {
                var operand = ParseExpr(tokens, ref pos, ctx);
                ExpectClose(tokens, ref pos);
                return Exprs.Not(operand);
            }
            case "order":
            {
                var variable = ParseExpr(tokens, ref pos, ctx);
                var point = ParseExpr(tokens, ref pos, ctx);
                var degree = ParseExpr(tokens, ref pos, ctx);
                ExpectClose(tokens, ref pos);
                return Exprs.Order(variable, point, degree);
            }
            default:
                throw new FormatException($"Unknown tag '{tag}'.");
        }
    }

    private static void ExpectClose(List<string> tokens, ref int pos)
    {
        if (pos >= tokens.Count || tokens[pos] != ")")
            throw new FormatException("Expected ')'.");
        pos++;
    }

    /// <summary>Every free symbol of an expression, in deterministic name order. Total over the
    /// node kinds: relations, piecewise guards, derivatives, integrals, RootOf, logic and order
    /// terms are all traversed (agent-facing introspection must not under-report).</summary>
    public static Symbol[] CollectSymbols(Expr e)
    {
        var set = new SortedSet<Symbol>();
        void Walk(Expr x)
        {
            switch (x)
            {
                case SymbolExpr s: set.Add(s.Symbol); break;
                case AddExpr a: foreach (var t in a.Terms) Walk(t); break;
                case MultiplyExpr m: foreach (var f in m.Factors) Walk(f); break;
                case PowerExpr p: Walk(p.Base); Walk(p.Exponent); break;
                case FunctionExpr f: foreach (var a in f.Arguments) Walk(a); break;
                case RelationExpr r: Walk(r.Left); Walk(r.Right); break;
                case PiecewiseExpr pw:
                    foreach (var b in pw.Branches) { Walk(b.Guard); Walk(b.Value); }
                    Walk(pw.Otherwise);
                    break;
                case DerivativeExpr d: Walk(d.Operand); break;
                case IntegralExpr i: Walk(i.Operand); break;
                case AndExpr an: foreach (var o in an.Operands) Walk(o); break;
                case OrExpr or: foreach (var o in or.Operands) Walk(o); break;
                case NotExpr nt: Walk(nt.Operand); break;
                case OrderExpr od: Walk(od.Variable); Walk(od.Point); Walk(od.Degree); break;
            }
        }
        Walk(e);
        return set.ToArray();
    }

    /// <summary>Free symbol names of an expression, in deterministic order.</summary>
    public static IReadOnlyList<string> FreeSymbolNames(Expr e) =>
        CollectSymbols(e).Select(s => s.Name).ToArray();

    // -----------------------------------------------------------------
    // Pretty print (display only — never used for identity)
    // -----------------------------------------------------------------

    public enum PrintMode { Canonical, Pretty, Debug }

    /// <summary>
    /// Formatting options. <see cref="PrintMode.Canonical"/> is the byte-stable versioned
    /// S-expression form; <see cref="PrintMode.Pretty"/> is the human-oriented infix form
    /// (REPL default); <see cref="PrintMode.Debug"/> is a kind-annotated structural form for
    /// agents. <c>Unicode</c> switches the human form to ∞ √ π ≤ ≥ ≠ (ASCII stays the default).
    /// </summary>
    public sealed record PrintOptions(
        PrintMode Mode = PrintMode.Pretty, bool Unicode = false, PrintBudget? Budget = null);

    /// <summary>
    /// A display budget. <c>null</c> on either limit means "no limit", so the default behaviour is
    /// unbounded and byte-identical to before. A rendering that exceeds the budget is abbreviated
    /// (never silently truncated mid-token) and the caller receives a
    /// <see cref="PrintTruncation"/> describing exactly what was dropped.
    /// </summary>
    public sealed record PrintBudget(int? MaxNodes = null, int? MaxDepth = null);

    /// <summary>Why a rendering was abbreviated, how big the expression actually is, the budget
    /// that stopped it, and the abbreviated text (the partial result the caller may show).</summary>
    public sealed record PrintTruncation(string Reason, int NodeCount, int Budget, string PartialResult);

    /// <summary>The outcome of a budgeted rendering: the text plus, when the budget stopped a full
    /// rendering, the structured reason. Canonical full output stays available by asking for no
    /// budget.</summary>
    public sealed record PrintOutcome(string Text, bool Truncated, PrintTruncation? Truncation);

    /// <summary>
    /// Renders <paramref name="e"/> under <paramref name="options"/>' budget. Without a budget this
    /// is exactly <see cref="PrettyPrint(Expr, PrintOptions)"/> and reports no truncation.
    /// </summary>
    public static PrintOutcome Print(Expr e, PrintOptions options)
    {
        if (options.Budget is not { } budget)
            return new PrintOutcome(PrettyPrint(e, options), false, null);

        int nodes = e.NodeCount;
        string? reason = null;
        int limit = 0;
        if (budget.MaxNodes is { } maxNodes && nodes > maxNodes)
        {
            reason = "node-budget";
            limit = maxNodes;
        }
        else if (budget.MaxDepth is { } maxDepth && ExprDepth(e) > maxDepth)
        {
            reason = "depth-limit";
            limit = maxDepth;
        }

        if (reason is null)
            return new PrintOutcome(PrettyPrint(e, options), false, null);

        // The rendering always goes through the normal printer, so ordering, precedence and
        // operator spacing are exactly the ones the caller would otherwise see; the budget bounds
        // how much of it is returned. A prefix is never passed off as the whole expression: the
        // trailing ellipsis is emitted here and the structured diagnostic says what was dropped.
        var full = PrettyPrint(e, options);
        int charBudget = Math.Max(48, limit * CharsPerNode);
        // nothing was dropped, so nothing is reported: "truncated" means the caller is NOT looking
        // at the whole expression
        if (full.Length <= charBudget)
            return new PrintOutcome(full, false, null);

        string text = full[..SafeCut(full, charBudget)] + " …";
        return new PrintOutcome(text, true, new PrintTruncation(reason, nodes, limit, text));
    }

    /// <summary>Characters of rendering allowed per budgeted node. A budget is expressed in nodes
    /// because that is the property of the expression; this is the documented conversion used to
    /// bound the returned text.</summary>
    private const int CharsPerNode = 6;

    /// <summary>Cuts at a token boundary so a number or identifier is never split mid-token.</summary>
    private static int SafeCut(string text, int cut)
    {
        int i = Math.Min(cut, text.Length);
        while (i > 0 && (char.IsLetterOrDigit(text[i - 1]) || text[i - 1] == '.'))
            i--;
        return i > 0 ? i : cut;
    }

    private static int ExprDepth(Expr e) => e switch
    {
        AddExpr a => a.Terms.Length == 0 ? 1 : 1 + a.Terms.Max(ExprDepth),
        MultiplyExpr m => m.Factors.Length == 0 ? 1 : 1 + m.Factors.Max(ExprDepth),
        PowerExpr p => 1 + Math.Max(ExprDepth(p.Base), ExprDepth(p.Exponent)),
        FunctionExpr f => f.Arguments.Length == 0 ? 1 : 1 + f.Arguments.Max(ExprDepth),
        RelationExpr r => 1 + Math.Max(ExprDepth(r.Left), ExprDepth(r.Right)),
        _ => 1,
    };



    public static string PrettyPrint(Expr e) => PrettyPrint(e, new PrintOptions());

    public static string PrettyPrint(Expr e, PrintOptions options) => options.Mode switch
    {
        PrintMode.Canonical => CanonicalPrint(e),
        PrintMode.Debug => DebugPrint(e),
        _ => Pretty(e, 0, false, options.Unicode),
    };

    /// <summary>Structural (kind-annotated) print for agent debugging: every node carries its
    /// kind, so agents never have to infer structure from the infix form.</summary>
    public static string DebugPrint(Expr e) => e switch
    {
        IntegerConstantExpr i => "int(" + i.Value + ")",
        RationalConstantExpr r => "rat(" + r.Value + ")",
        RealConstantExpr rl => "real(" + rl.Value + ")",
        ComplexConstantExpr c => "cplx(" + c.Re + ", " + c.Im + ")",
        SymbolExpr s => "sym(" + s.Symbol.Name + ")",
        NamedConstantExpr n => "named(" + n.Constant + ")",
        AddExpr a => "add[" + string.Join(", ", a.Terms.Select(DebugPrint)) + "]",
        MultiplyExpr m => "mul[" + string.Join(", ", m.Factors.Select(DebugPrint)) + "]",
        PowerExpr p => "pow[" + DebugPrint(p.Base) + ", " + DebugPrint(p.Exponent) + "]",
        FunctionExpr f => "fn:" + f.Function.Name + "(" + string.Join(", ", f.Arguments.Select(DebugPrint)) + ")",
        RelationExpr r => "rel:" + r.Op + "(" + DebugPrint(r.Left) + ", " + DebugPrint(r.Right) + ")",
        PiecewiseExpr pw => "piecewise[" +
            string.Join(", ", pw.Branches.Select(b => DebugPrint(b.Guard) + " -> " + DebugPrint(b.Value))) +
            "; " + DebugPrint(pw.Otherwise) + "]",
        DerivativeExpr d => "der[" + DebugPrint(d.Operand) + ", " + string.Join(", ", d.Variables.Select(v => v.Name)) + "]",
        IntegralExpr i2 => "integral[" + DebugPrint(i2.Operand) + ", " + string.Join(", ", i2.Variables.Select(v => v.Name)) + "]",
        RootOfExpr r2 => "rootof[" + DebugPrint(r2.DefiningPolynomial.ToExpr()) + ", " + r2.RootIndex + "]",
        AndExpr an => "and[" + string.Join(", ", an.Operands.Select(DebugPrint)) + "]",
        OrExpr or2 => "or[" + string.Join(", ", or2.Operands.Select(DebugPrint)) + "]",
        NotExpr nt => "not[" + DebugPrint(nt.Operand) + "]",
        OrderExpr o => "order[" + DebugPrint(o.Variable) + ", " + DebugPrint(o.Point) + ", " + DebugPrint(o.Degree) + "]",
        _ => "?" + e.Kind,
    };

    private static int Prec(Expr e) => e switch
    {
        OrExpr => -1,
        AndExpr => 0,
        RelationExpr => 0,
        AddExpr => 1,
        MultiplyExpr => 2,
        PowerExpr => 3,
        _ => 4,
    };

    private static readonly Rat OneHalf = Rat.From(1, 2);
    private static readonly Rat MinusOneHalf = Rat.From(-1, 2);

    private static string Pretty(Expr e, int parentPrec, bool rightOfPower, bool unicode)
    {
        switch (e)
        {
            case IntegerConstantExpr i: return i.Value.ToString();
            case RationalConstantExpr r: return r.Value.ToString();
            case RealConstantExpr rl: return rl.Value.ToString();
            case ComplexConstantExpr c: return ComplexToText(c);
            case SymbolExpr s: return s.Symbol.Name;
            case NamedConstantExpr n: return n.Constant switch
            {
                NamedConstant.Pi => unicode ? "π" : "pi",
                NamedConstant.E => "e",
                NamedConstant.I => "i",
                _ => unicode ? "∞" : "inf",
            };
            case AddExpr a:
            {
                var terms = OrderTermsForDisplay(a.Terms);
                var sb = new StringBuilder();
                if (terms.Length > 0 && IsNegativeConstant(terms[0], out var leadMag))
                {
                    // leading negative constant renders as "rest - |c|" (x - 1, not -1 + x)
                    if (terms.Length == 1)
                    {
                        var single = "-" + Pretty(leadMag, 1, false, unicode);
                        return parentPrec > 1 ? "(" + single + ")" : single;
                    }
                    sb.Append(Pretty(terms[1], 1, false, unicode));
                    for (int i = 2; i < terms.Length; i++)
                        AppendSigned(sb, terms[i], unicode);
                    sb.Append(" - ");
                    sb.Append(Pretty(leadMag, 1, false, unicode));
                }
                else
                {
                    sb.Append(Pretty(terms[0], 1, false, unicode));
                    for (int i = 1; i < terms.Length; i++)
                        AppendSigned(sb, terms[i], unicode);
                }
                var s = sb.ToString();
                return parentPrec > 1 ? "(" + s + ")" : s;
            }
            case MultiplyExpr m:
            {
                Rat? coef = null;
                bool negative = false;
                int first = 0;
                if (m.Factors.Length > 0 && m.Factors[0] is RationalConstantExpr rc0)
                {
                    coef = rc0.Value;
                    first = 1;
                    if (coef.IsNegative)
                    {
                        negative = true;
                        coef = Rat.Negate(coef);
                    }
                }
                var nums = new List<string>();
                var dens = new List<string>();
                for (int i = first; i < m.Factors.Length; i++)
                {
                    var f = m.Factors[i];
                    if (f is PowerExpr p && p.Exponent is RationalConstantExpr pe && pe.Value.IsMinusOne)
                    {
                        var dText = Pretty(p.Base, 0, false, unicode);
                        dens.Add(Prec(p.Base) < 3 ? "(" + dText + ")" : dText);
                    }
                    else
                    {
                        nums.Add(Pretty(f, 2, false, unicode));
                    }
                }
                string text;
                if (dens.Count == 0)
                {
                    var body = string.Join("*", nums);
                    if (coef is { } c0 && !c0.IsOne)
                        text = nums.Count == 0 ? c0.ToString() : c0.ToString() + "*" + body;
                    else
                        text = nums.Count == 0 ? "1" : body;
                }
                else
                {
                    // fractions render as num/(den): a rational coefficient folds into the
                    // fraction instead of producing num/coef/den chains (-1/(2*(x + 1)))
                    var numParts = new List<string>();
                    if (coef is { } c1 && !c1.IsOne)
                    {
                        if (c1.IsInteger)
                        {
                            numParts.Add(c1.ToInteger().ToString());
                        }
                        else
                        {
                            numParts.Add(c1.Numerator.ToString());
                            dens.Insert(0, c1.Denominator.ToString());
                        }
                    }
                    numParts.AddRange(nums);
                    var numText = numParts.Count == 0 ? "1" : string.Join("*", numParts);
                    var denText = dens.Count == 1 ? dens[0] : "(" + string.Join("*", dens) + ")";
                    text = numText + "/" + denText;
                }
                if (negative) text = "-" + text;
                return parentPrec > 2 ? "(" + text + ")" : text;
            }
            case PowerExpr p:
            {
                // radical rendering: x^(1/2) → sqrt(x), x^(-1/2) → 1/sqrt(x) (human form only)
                if (p.Exponent is RationalConstantExpr sq)
                {
                    if (sq.Value == OneHalf)
                    {
                        var rt = "sqrt(" + Pretty(p.Base, 0, false, unicode) + ")";
                        return parentPrec > 3 ? "(" + rt + ")" : rt;
                    }
                    if (sq.Value == MinusOneHalf)
                    {
                        var rt = "1/sqrt(" + Pretty(p.Base, 0, false, unicode) + ")";
                        return parentPrec > 3 ? "(" + rt + ")" : rt;
                    }
                }
                var b = Pretty(p.Base, 3, false, unicode);
                // a power base must be parenthesized: x^y^2 is ambiguous ((x^y)^2 vs x^(y^2))
                if (Prec(p.Base) <= 3)
                    b = "(" + b + ")";
                var ex = Pretty(p.Exponent, 4, true, unicode);
                if (p.Exponent is RationalConstantExpr rce && !rce.Value.IsInteger)
                    ex = "(" + ex + ")";
                else if (p.Exponent is PowerExpr)
                    ex = "(" + ex + ")";
                var text = b + "^" + ex;
                return parentPrec > 3 ? "(" + text + ")" : text;
            }
            case FunctionExpr f:
            {
                var args = string.Join(", ", f.Arguments.Select(a => Pretty(a, 3, false, unicode)));
                return f.Function.Name + "(" + args + ")";
            }
            case RelationExpr r:
            {
                var op = r.Op switch
                {
                    RelOp.Eq => " = ",
                    RelOp.Ne => unicode ? " ≠ " : " != ",
                    RelOp.Lt => " < ",
                    RelOp.Le => unicode ? " ≤ " : " <= ",
                    RelOp.Gt => " > ",
                    _ => unicode ? " ≥ " : " >= ",
                };
                return Pretty(r.Left, 0, false, unicode) + op + Pretty(r.Right, 0, false, unicode);
            }
            case PiecewiseExpr pw:
            {
                var parts = pw.Branches.Select(b => Pretty(b.Value, 0, false, unicode) + " if " + Pretty(b.Guard, 0, false, unicode));
                return "piecewise(" + string.Join(", ", parts) + ", " + Pretty(pw.Otherwise, 0, false, unicode) + ")";
            }
            case DerivativeExpr d:
                return "diff(" + Pretty(d.Operand, 4, false, unicode) + ", " + string.Join(", ", d.Variables.Select(v => v.Name)) + ")";
            case IntegralExpr i:
                return "integrate(" + Pretty(i.Operand, 4, false, unicode) + ", " + string.Join(", ", i.Variables.Select(v => v.Name)) + ")";
            case RootOfExpr r:
                return "rootof(" + Pretty(r.DefiningPolynomial.ToExpr(), 4, false, unicode) + ", " + r.RootIndex + ")";
            case AndExpr an:
            {
                var text = string.Join(" and ", an.Operands.Select(o => Pretty(o, 0, false, unicode)));
                return parentPrec > 0 ? "(" + text + ")" : text;
            }
            case OrExpr or2:
            {
                var text = string.Join(" or ", or2.Operands.Select(o => Pretty(o, -1, false, unicode)));
                return parentPrec > -1 ? "(" + text + ")" : text;
            }
            case NotExpr nt:
            {
                var text = "not " + Pretty(nt.Operand, 4, false, unicode);
                return parentPrec > 4 ? "(" + text + ")" : text;
            }
            case OrderExpr o:
            {
                bool zeroPoint = o.Point is RationalConstantExpr rp && rp.Value.IsZero
                              || o.Point is IntegerConstantExpr ip && Int.IsZero(ip.Value);
                var baseExpr = zeroPoint ? o.Variable : Exprs.Subtract(o.Variable, o.Point);
                var baseText = Pretty(baseExpr, 3, false, unicode);
                bool degreeOne = o.Degree is RationalConstantExpr rd && rd.Value.IsOne
                              || o.Degree is IntegerConstantExpr id && id.Value == Int.One;
                if (degreeOne)
                    return "O(" + baseText + ")";
                var body = zeroPoint ? baseText : "(" + baseText + ")";
                return "O(" + body + "^" + Pretty(o.Degree, 3, false, unicode) + ")";
            }
            default:
                return e.Kind.ToString();
        }
    }

    private static void AppendSigned(StringBuilder sb, Expr t, bool unicode)
    {
        if (IsNegativeTerm(t, out var inner))
        {
            sb.Append(" - ");
            sb.Append(Pretty(inner, 1, false, unicode));
        }
        else
        {
            sb.Append(" + ");
            sb.Append(Pretty(t, 1, false, unicode));
        }
    }

    private static bool IsNegativeConstant(Expr t, out Expr magnitude)
    {
        if (t is RationalConstantExpr r && r.Value.IsNegative)
        {
            magnitude = Exprs.Rational(Rat.Negate(r.Value));
            return true;
        }
        if (t is IntegerConstantExpr i && Int.IsNegative(i.Value))
        {
            magnitude = Exprs.Integer(-i.Value);
            return true;
        }
        magnitude = t;
        return false;
    }

    /// <summary>
    /// Human-facing term ordering: a sum containing an Order term renders in ascending powers
    /// around the expansion point (O-term last); a single-symbol polynomial renders in
    /// descending degree; anything else keeps the canonical order. Bounded: sums above 64
    /// terms are left canonical (determinism and cost).
    /// </summary>
    private static ImmutableArray<Expr> OrderTermsForDisplay(ImmutableArray<Expr> terms)
    {
        if (terms.Length is 0 or > 64)
            return terms;

        // series: the O-term declares the expansion variable and point
        foreach (var t in terms)
        {
            if (t is OrderExpr o && o.Variable is SymbolExpr sv)
            {
                var offset = IsZeroExpr(o.Point) ? (Expr)sv : Exprs.Subtract(sv, o.Point);
                return ImmutableArray.CreateRange(terms
                    .OrderBy(t => t, Comparer<Expr>.Create((x, y) => CompareSeriesTerms(x, y, offset))));
            }
        }

        // polynomial: every non-constant term must be a monomial in one common symbol
        Symbol? polySymbol = null;
        foreach (var t in terms)
        {
            if (t is RationalConstantExpr or IntegerConstantExpr or RealConstantExpr)
                continue;
            if (!TryMonomial(t, out var sym, out _))
                return terms;
            if (polySymbol is null)
                polySymbol = sym;
            else if (polySymbol.Value.Name != sym.Name)
                return terms;
        }
        if (polySymbol is { } ps)
        {
            return ImmutableArray.CreateRange(terms
                .OrderByDescending(t => DegreeOf(t, ps))
                .ThenBy(t => t, Comparer<Expr>.Create(TermOrder.Compare)));
        }
        return terms;
    }

    private static int CompareSeriesTerms(Expr x, Expr y, Expr offset)
    {
        bool xo = x is OrderExpr, yo = y is OrderExpr;
        if (xo || yo)
            return xo == yo ? 0 : (xo ? 1 : -1);   // the O-term is always last
        var dx = TryDegreeOf(x, offset, out var xd);
        var dy = TryDegreeOf(y, offset, out var yd);
        if (dx && dy && xd != yd)
            return xd < yd ? -1 : 1;               // ascending powers around the point
        return TermOrder.Compare(x, y);
    }

    private static bool TryDegreeOf(Expr t, Expr offset, out Rat degree)
    {
        switch (t)
        {
            case RationalConstantExpr or IntegerConstantExpr or RealConstantExpr:
                degree = Rat.Zero;
                return true;
            case SymbolExpr s when s.Equals(offset):
                degree = Rat.One;
                return true;
            case PowerExpr p when p.Base.Equals(offset) && p.Exponent is RationalConstantExpr pr:
                degree = pr.Value;
                return true;
            case MultiplyExpr m:
            {
                Rat total = Rat.Zero;
                bool any = false;
                foreach (var f in m.Factors)
                {
                    if (f is PowerExpr fp && fp.Base.Equals(offset) && fp.Exponent is RationalConstantExpr fr)
                    {
                        total = total + fr.Value;
                        any = true;
                    }
                    else if (f is SymbolExpr fs && fs.Equals(offset))
                    {
                        total = total + Rat.One;
                        any = true;
                    }
                    else if (f is not (RationalConstantExpr or IntegerConstantExpr or RealConstantExpr))
                    {
                        degree = Rat.Zero;
                        return false;
                    }
                }
                degree = any ? total : Rat.Zero;
                return true;
            }
            default:
                degree = Rat.Zero;
                return false;
        }
    }

    private static bool TryMonomial(Expr t, out Symbol symbol, out Rat degree)
    {
        switch (t)
        {
            case SymbolExpr s:
                symbol = s.Symbol;
                degree = Rat.One;
                return true;
            case PowerExpr p when p.Base is SymbolExpr sb && p.Exponent is RationalConstantExpr pr:
                symbol = sb.Symbol;
                degree = pr.Value;
                return true;
            case MultiplyExpr m:
            {
                Symbol? found = null;
                Rat total = Rat.Zero;
                foreach (var f in m.Factors)
                {
                    if (f is SymbolExpr ms)
                    {
                        found ??= ms.Symbol;
                        if (ms.Symbol.Name != found.Value.Name)
                            goto fail;
                        total = total + Rat.One;
                    }
                    else if (f is PowerExpr mp && mp.Base is SymbolExpr mb && mp.Exponent is RationalConstantExpr mr)
                    {
                        found ??= mb.Symbol;
                        if (mb.Symbol.Name != found.Value.Name)
                            goto fail;
                        total = total + mr.Value;
                    }
                    else if (f is not (RationalConstantExpr or IntegerConstantExpr or RealConstantExpr))
                    {
                        goto fail;
                    }
                }
                symbol = found!.Value;
                degree = total;
                return true;
            fail:
                symbol = default;
                degree = Rat.Zero;
                return false;
            }
            default:
                symbol = default;
                degree = Rat.Zero;
                return false;
        }
    }

    private static Rat DegreeOf(Expr t, Symbol s) => t switch
    {
        RationalConstantExpr or IntegerConstantExpr or RealConstantExpr => Rat.Zero,
        _ => TryMonomial(t, out var sym, out var d) && sym.Name == s.Name ? d : Rat.Zero,
    };

    private static bool IsZeroExpr(Expr e) => e switch
    {
        RationalConstantExpr r => r.Value.IsZero,
        IntegerConstantExpr i => Int.IsZero(i.Value),
        _ => false,
    };

    private static string ComplexToText(ComplexConstantExpr c)
    {
        var im = c.Im;
        var sign = im.IsNegative ? "-" : "+";
        var abs = Rat.Abs(im);
        var imText = abs.IsOne ? "" : abs.ToString() + "*";
        return c.Re + " " + sign + " " + imText + "i";
    }

    private static bool IsNegativeTerm(Expr t, out Expr inner)
    {
        // a negative term renders as " - <positive-magnitude term>"; the magnitude is preserved
        if (t is RationalConstantExpr r && r.Value.IsNegative)
        {
            inner = Exprs.Rational(Rat.Negate(r.Value));
            return true;
        }
        if (t is MultiplyExpr m && m.Factors.Length > 0 && m.Factors[0] is RationalConstantExpr rc && rc.Value.IsNegative)
        {
            var rest = m.Factors.Skip(1).ToArray();
            var posCoef = Exprs.Rational(Rat.Negate(rc.Value));
            if (posCoef is RationalConstantExpr pc && pc.Value.IsOne)
            {
                inner = rest.Length == 1 ? rest[0] : Exprs.Multiply(rest);
            }
            else
            {
                var factors = new List<Expr> { posCoef };
                factors.AddRange(rest);
                inner = Exprs.Multiply(factors);
            }
            return true;
        }
        inner = t;
        return false;
    }
}
