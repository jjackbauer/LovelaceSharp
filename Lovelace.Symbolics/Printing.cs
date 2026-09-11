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

    public enum PrintMode { Canonical, Pretty, Debug, Latex }

    /// <summary>
    /// Formatting options. <see cref="PrintMode.Canonical"/> is the byte-stable versioned
    /// S-expression form; <see cref="PrintMode.Pretty"/> is the human-oriented infix form
    /// (REPL default); <see cref="PrintMode.Debug"/> is a kind-annotated structural form for
    /// agents; <see cref="PrintMode.Latex"/> is the SAME human form rendered as LaTeX source —
    /// same printer, same precedence decisions (<see cref="Prec"/>, <see cref="Delimit"/>), only
    /// the notation differs. <c>Unicode</c> switches the human form to ∞ √ π ≤ ≥ ≠ (ASCII stays
    /// the default) and does not affect the LaTeX form.
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
        PrintMode.Latex => LatexRender(e, RootPrec),
        _ => Pretty(e, RootPrec, false, options.Unicode),
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

    // -----------------------------------------------------------------
    // Precedence: ONE table, ONE parenthesis decision, shared by every
    // infix rendering. Pretty and Latex both read these; there is no
    // second precedence table anywhere in this file.
    // -----------------------------------------------------------------

    private const int OrPrec = -1;
    private const int AndPrec = 0;
    private const int RelationPrec = 0;
    private const int AddPrec = 1;
    private const int MulPrec = 2;
    private const int PowPrec = 3;
    private const int AtomPrec = 4;

    /// <summary>The loosest rendering context: an operand nothing further is demanded of. It is
    /// the root context of both infix renderings, and the context of a child that LaTeX's own
    /// braces already group (a <c>rac</c> argument, a <c>sqrt</c> radicand, a superscript).</summary>
    private const int RootPrec = 0;

    private static int Prec(Expr e) => e switch
    {
        OrExpr => OrPrec,
        AndExpr => AndPrec,
        RelationExpr => RelationPrec,
        AddExpr => AddPrec,
        MultiplyExpr => MulPrec,
        PowerExpr => PowPrec,
        _ => AtomPrec,
    };

    /// <summary>The ONE parenthesis decision of the infix printers: a node whose own precedence
    /// is <paramref name="nodePrec"/> is wrapped exactly when the context
    /// (<paramref name="parentPrec"/>) demands something that binds tighter. Pretty and Latex
    /// both call this, so the two renderings cannot place a delimiter differently; only the glyph
    /// pair differs (<see cref="Delimit"/> vs <see cref="LatexDelimit"/>).</summary>
    private static bool NeedsDelimiter(int parentPrec, int nodePrec) => parentPrec > nodePrec;

    /// <summary>The shared decision applied in the ASCII infix notation.</summary>
    private static string Delimit(string text, int parentPrec, int nodePrec) =>
        NeedsDelimiter(parentPrec, nodePrec) ? "(" + text + ")" : text;

    /// <summary>The shared decision applied in LaTeX notation.</summary>
    private static string LatexDelimit(string text, int parentPrec, int nodePrec) =>
        NeedsDelimiter(parentPrec, nodePrec) ? "\\left(" + text + "\\right)" : text;

    /// <summary>The ONE power-base rule: a base that does not bind tighter than a power is
    /// delimited exactly once — <c>(x + 1)^2</c>, <c>(x^y)^2</c>; asking the child renderer for a
    /// precedence wrapped it twice (((x + 1))^12). Round 24.
    /// <para>
    /// Precedence is a property of VALUES and is not sufficient, because every constant is an
    /// atom in that table while a negative constant is not an atom as TEXT: <c>-1</c> is a unary
    /// minus applied to <c>1</c>, and a unary minus binds looser than <c>^</c>, so rendering
    /// <c>(-1)^x</c> as <c>-1^x</c> denotes <c>-(1^x)</c> — a different value. The rule therefore
    /// asks the text that is about to be raised as well, and delimits it when that text begins
    /// with a unary minus. This covers every constant shape that can print a sign (integer,
    /// rational, real, complex) without a second table, and cannot fire for a base that already
    /// binds tighter than a power, so no pair is ever doubled.
    /// </para>
    /// <para>
    /// The test is made against the rendered text rather than against the node because the two
    /// arms spell the same node differently where it matters: Pretty renders a complex constant
    /// as <c>-1 + i</c> and LaTeX as <c>(-1 + 1 i)</c>, so no single structural predicate is
    /// correct for both — it would either leave Pretty's base undelimited or wrap LaTeX's already
    /// delimited one a second time. Each arm passes the text it is about to emit, which is the
    /// same question — "does this bind looser than ^?" — in both notations. Cycle 5.
    /// </para>
    /// </summary>
    private static bool NeedsPowerBaseDelimiter(Expr b, string renderedBase) =>
        Prec(b) <= PowPrec || renderedBase.StartsWith("-", StringComparison.Ordinal);

    private static readonly Rat OneHalf = Rat.From(1, 2);
    private static readonly Rat MinusOneHalf = Rat.From(-1, 2);

    /// <summary>Precedence used for an operand that is already delimited by a call's parentheses
    /// and commas (function arguments, calculus operands). Nothing at or above additive
    /// precedence needs parentheses there; the logical operators keep theirs. Round 24.</summary>
    private const int ArgPrec = AddPrec;

    /// <summary>Precedence demanded of the magnitude of a subtracted term: strictly tighter than
    /// additive, so a sum on the right of a minus keeps its parentheses — <c>x - (y - a)</c> must
    /// never render as <c>x - y - a</c>, which denotes a different expression. Round 24.</summary>
    private const int SubtrahendPrec = MulPrec;

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
                        var single = "-" + Pretty(leadMag, AddPrec, false, unicode);
                        return Delimit(single, parentPrec, AddPrec);
                    }
                    sb.Append(Pretty(terms[1], AddPrec, false, unicode));
                    for (int i = 2; i < terms.Length; i++)
                        AppendSigned(sb, terms[i], unicode);
                    sb.Append(" - ");
                    sb.Append(Pretty(leadMag, AddPrec, false, unicode));
                }
                else
                {
                    sb.Append(Pretty(terms[0], AddPrec, false, unicode));
                    for (int i = 1; i < terms.Length; i++)
                        AppendSigned(sb, terms[i], unicode);
                }
                return Delimit(sb.ToString(), parentPrec, AddPrec);
            }
            case MultiplyExpr m:
            {
                var shape = ShapeProduct(m);
                var nums = new List<string>();
                var dens = new List<string>();
                foreach (var f in shape.Numerators)
                    nums.Add(Pretty(f, MulPrec, false, unicode));
                foreach (var b in shape.Denominators)
                {
                    var dText = Pretty(b, RootPrec, false, unicode);
                    dens.Add(Prec(b) < PowPrec ? "(" + dText + ")" : dText);
                }
                string text;
                if (dens.Count == 0)
                {
                    var body = string.Join("*", nums);
                    if (shape.Coefficient is { } c0 && !c0.IsOne)
                        text = nums.Count == 0 ? c0.ToString() : c0.ToString() + "*" + body;
                    else
                        text = nums.Count == 0 ? "1" : body;
                }
                else
                {
                    // fractions render as num/(den): a rational coefficient folds into the
                    // fraction instead of producing num/coef/den chains (-1/(2*(x + 1)))
                    var numParts = new List<string>();
                    if (shape.Coefficient is { } c1 && !c1.IsOne)
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
                if (shape.Negative) text = "-" + text;
                return Delimit(text, parentPrec, MulPrec);
            }
            case PowerExpr p:
            {
                // radical rendering: x^(1/2) → sqrt(x), x^(-1/2) → 1/sqrt(x) (human form only)
                if (p.Exponent is RationalConstantExpr sq)
                {
                    if (sq.Value == OneHalf)
                        return Delimit("sqrt(" + Pretty(p.Base, RootPrec, false, unicode) + ")", parentPrec, PowPrec);
                    if (sq.Value == MinusOneHalf)
                        return Delimit("1/sqrt(" + Pretty(p.Base, RootPrec, false, unicode) + ")", parentPrec, PowPrec);
                }
                // the base is rendered at the loosest precedence and wrapped exactly once by the
                // shared rule; asking Pretty for parentPrec 3 here wrapped it a second time
                // (((x + 1))^12). Round 24.
                var b = Pretty(p.Base, RootPrec, false, unicode);
                // a power base must be parenthesized: x^y^2 is ambiguous ((x^y)^2 vs x^(y^2)),
                // and -1^x is not (-1)^x
                if (NeedsPowerBaseDelimiter(p.Base, b))
                    b = "(" + b + ")";
                // 3, not 4: a power exponent is parenthesized by the explicit rule just below, so
                // asking for 4 wrapped it twice (x^((y^2))). Sums and products still get their
                // parentheses from this call. Round 24.
                var ex = Pretty(p.Exponent, PowPrec, true, unicode);
                if (p.Exponent is RationalConstantExpr rce && !rce.Value.IsInteger)
                    ex = "(" + ex + ")";
                else if (p.Exponent is PowerExpr)
                    ex = "(" + ex + ")";
                return Delimit(b + "^" + ex, parentPrec, PowPrec);
            }
            case FunctionExpr f:
            {
                // an argument is delimited by the comma or the closing paren, so it never needs
                // precedence parentheses; ArgPrec keeps the logical operators wrapped. Round 24.
                var args = string.Join(", ", f.Arguments.Select(a => Pretty(a, ArgPrec, false, unicode)));
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
                return "diff(" + Pretty(d.Operand, ArgPrec, false, unicode) + ", " + string.Join(", ", d.Variables.Select(v => v.Name)) + ")";
            case IntegralExpr i:
                return "integrate(" + Pretty(i.Operand, ArgPrec, false, unicode) + ", " + string.Join(", ", i.Variables.Select(v => v.Name)) + ")";
            case RootOfExpr r:
                return "rootof(" + Pretty(r.DefiningPolynomial.ToExpr(), ArgPrec, false, unicode) + ", " + r.RootIndex + ")";
            case AndExpr an:
            {
                var text = string.Join(" and ", an.Operands.Select(o => Pretty(o, AndPrec, false, unicode)));
                return Delimit(text, parentPrec, AndPrec);
            }
            case OrExpr or2:
            {
                var text = string.Join(" or ", or2.Operands.Select(o => Pretty(o, OrPrec, false, unicode)));
                return Delimit(text, parentPrec, OrPrec);
            }
            case NotExpr nt:
            {
                var text = "not " + Pretty(nt.Operand, AtomPrec, false, unicode);
                return Delimit(text, parentPrec, AtomPrec);
            }
            case OrderExpr o:
            {
                var (baseExpr, zeroPoint, degreeOne) = OrderShape(o);
                var baseText = Pretty(baseExpr, RelationPrec, false, unicode);
                if (degreeOne)
                    return "O(" + baseText + ")";
                var body = zeroPoint ? baseText : "(" + baseText + ")";
                return "O(" + body + "^" + Pretty(o.Degree, PowPrec, false, unicode) + ")";
            }
            default:
                return e.Kind.ToString();
        }
    }

    // -----------------------------------------------------------------
    // LaTeX rendering — the SAME expression model, the SAME precedence
    // table, a different notation. There is exactly one printer: this
    // arm shares Prec/Delimit/ShapeProduct/OrderTermsForDisplay/
    // IsNegativeTerm/OrderShape with the Pretty arm, so the two forms
    // cannot disagree about what an expression is.
    // -----------------------------------------------------------------

    /// <summary>
    /// The ONE product decomposition every infix rendering uses: an optional leading rational
    /// coefficient with its sign split off, the factors that stay in the numerator, and the bases
    /// of the factors raised to -1 (the denominator). A second copy of this rule is exactly what
    /// would let two renderings describe different expressions.
    /// </summary>
    private sealed record ProductShape(Rat? Coefficient, bool Negative, List<Expr> Numerators, List<Expr> Denominators);

    private static ProductShape ShapeProduct(MultiplyExpr m)
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
        var nums = new List<Expr>();
        var dens = new List<Expr>();
        for (int i = first; i < m.Factors.Length; i++)
        {
            var f = m.Factors[i];
            if (f is PowerExpr p && p.Exponent is RationalConstantExpr pe && pe.Value.IsMinusOne)
                dens.Add(p.Base);
            else
                nums.Add(f);
        }
        return new ProductShape(coef, negative, nums, dens);
    }

    /// <summary>The ONE O-term decomposition: the base expression around the expansion point, and
    /// whether the point is zero and the degree is one. Read by both infix renderings.</summary>
    private static (Expr Base, bool ZeroPoint, bool DegreeOne) OrderShape(OrderExpr o)
    {
        bool zeroPoint = o.Point is RationalConstantExpr rp && rp.Value.IsZero
                      || o.Point is IntegerConstantExpr ip && Int.IsZero(ip.Value);
        bool degreeOne = o.Degree is RationalConstantExpr rd && rd.Value.IsOne
                      || o.Degree is IntegerConstantExpr id && id.Value == Int.One;
        return (zeroPoint ? o.Variable : Exprs.Subtract(o.Variable, o.Point), zeroPoint, degreeOne);
    }

    /// <summary>LaTeX source for an expression of the same model. Every precedence decision goes
    /// through <see cref="Prec"/>/<see cref="Delimit"/> and every structural decision (display
    /// term order, what a fraction is, what a subtracted term is, which powers are radicals)
    /// through the helpers the Pretty arm calls; only the notation is new.
    /// <para>
    /// Grouping: where LaTeX already groups a child by construction (the braces of
    /// <c>\frac</c>, <c>\sqrt</c> and <c>^{}</c>) the child is rendered at
    /// <see cref="RootPrec"/> and no delimiter is emitted — the brace group IS the delimiter.
    /// Everywhere else the delimiters are exactly the ones <see cref="Delimit"/> asks for, and
    /// <c>LatexPrinterTests</c> proves both halves: the structural nesting equals the Pretty
    /// nesting, and removing any <c>\left...\right</c> pair changes the meaning.
    /// </para>
    /// </summary>
    private static string LatexRender(Expr e, int parentPrec)
    {
        switch (e)
        {
            case IntegerConstantExpr i: return i.Value.ToString();
            case RationalConstantExpr r: return LatexRational(r.Value);
            case RealConstantExpr rl: return rl.Value.ToString();
            case ComplexConstantExpr c: return "(" + c.Re + " + " + c.Im + " i)";
            case SymbolExpr s: return LatexName(s.Symbol.Name);
            case NamedConstantExpr n: return n.Constant switch
            {
                NamedConstant.Pi => "\\pi",
                NamedConstant.E => "e",
                NamedConstant.I => "i",
                _ => "\\infty",
            };
            case AddExpr a:
            {
                var terms = OrderTermsForDisplay(a.Terms);
                var sb = new StringBuilder();
                if (terms.Length > 0 && IsNegativeConstant(terms[0], out var leadMag))
                {
                    if (terms.Length == 1)
                        return LatexDelimit("-" + LatexRender(leadMag, AddPrec), parentPrec, AddPrec);
                    sb.Append(LatexRender(terms[1], AddPrec));
                    for (int i = 2; i < terms.Length; i++)
                        AppendSignedLatex(sb, terms[i]);
                    sb.Append(" - ");
                    sb.Append(LatexRender(leadMag, AddPrec));
                }
                else
                {
                    sb.Append(LatexRender(terms[0], AddPrec));
                    for (int i = 1; i < terms.Length; i++)
                        AppendSignedLatex(sb, terms[i]);
                }
                return LatexDelimit(sb.ToString(), parentPrec, AddPrec);
            }
            case MultiplyExpr m:
            {
                var shape = ShapeProduct(m);
                string text;
                if (shape.Denominators.Count == 0)
                {
                    var body = string.Join(" \\cdot ", shape.Numerators.Select(f => LatexRender(f, MulPrec)));
                    if (shape.Coefficient is { } c0 && !c0.IsOne)
                        text = shape.Numerators.Count == 0
                            ? LatexRational(c0)
                            : LatexRational(c0) + " \\cdot " + body;
                    else
                        text = shape.Numerators.Count == 0 ? "1" : body;
                }
                else
                {
                    // The same coefficient fold the Pretty arm performs: an integer coefficient
                    // joins the numerator as a numeral, a fractional one contributes its
                    // numerator there and its denominator to the denominator side — never a
                    // num*coef/den chain (-1/(2*(x + 1))).
                    var numParts = new List<Func<int, string>>();
                    var denParts = new List<Func<int, string>>();
                    if (shape.Coefficient is { } c1 && !c1.IsOne)
                    {
                        if (c1.IsInteger)
                        {
                            numParts.Add(_ => c1.ToInteger().ToString());
                        }
                        else
                        {
                            numParts.Add(_ => c1.Numerator.ToString());
                            denParts.Add(_ => c1.Denominator.ToString());
                        }
                    }
                    foreach (var f in shape.Numerators)
                    {
                        var factor = f;
                        numParts.Add(prec => LatexRender(factor, prec));
                    }
                    foreach (var b in shape.Denominators)
                    {
                        var basis = b;
                        denParts.Add(prec => LatexRender(basis, prec));
                    }

                    // The braces of \frac ARE the delimiter of each side, so a side that is a
                    // single part is rendered at the loosest precedence and carries no
                    // \left...\right of its own; a side of several parts is a text-level \cdot
                    // product whose Add factors do need them (2 > 1), which is why
                    // \frac{1}{2 \cdot \left(x + 1\right)} groups exactly as 1/(2*(x + 1)).
                    string Side(List<Func<int, string>> parts, string whenEmpty) =>
                        parts.Count == 0
                            ? whenEmpty
                            : string.Join(" \\cdot ", parts.Select(p => p(parts.Count > 1 ? MulPrec : RootPrec)));

                    text = "\\frac{" + Side(numParts, "1") + "}{" + Side(denParts, "1") + "}";
                }
                if (shape.Negative) text = "-" + text;
                return LatexDelimit(text, parentPrec, MulPrec);
            }
            case PowerExpr p:
            {
                // the same radical decision the Pretty arm makes (exponent ±1/2)
                if (p.Exponent is RationalConstantExpr sq)
                {
                    if (sq.Value == OneHalf)
                        return LatexDelimit("\\sqrt{" + LatexRender(p.Base, RootPrec) + "}", parentPrec, PowPrec);
                    if (sq.Value == MinusOneHalf)
                        return LatexDelimit("\\frac{1}{\\sqrt{" + LatexRender(p.Base, RootPrec) + "}}", parentPrec, PowPrec);
                }
                var b = LatexRender(p.Base, RootPrec);
                if (NeedsPowerBaseDelimiter(p.Base, b))
                    b = "\\left(" + b + "\\right)";
                // the exponent's braces are LaTeX's own grouping, so a sum/product/rational
                // exponent needs no priority delimiters of its own: x^{y + 1}, x^{\frac{1}{2}}
                return LatexDelimit(b + "^{" + LatexRender(p.Exponent, RootPrec) + "}", parentPrec, PowPrec);
            }
            case FunctionExpr f:
            {
                if (f.Function.Name == "abs" && f.Arguments.Length == 1)
                    return "\\left|" + LatexRender(f.Arguments[0], RootPrec) + "\\right|";
                var args = string.Join(", ", f.Arguments.Select(a => LatexRender(a, ArgPrec)));
                return LatexCallName(f.Function.Name) + "(" + args + ")";
            }
            case RelationExpr r:
            {
                var op = r.Op switch
                {
                    RelOp.Eq => " = ",
                    RelOp.Ne => " \\ne ",
                    RelOp.Lt => " < ",
                    RelOp.Le => " \\le ",
                    RelOp.Gt => " > ",
                    _ => " \\ge ",
                };
                return LatexRender(r.Left, RelationPrec) + op + LatexRender(r.Right, RelationPrec);
            }
            case PiecewiseExpr pw:
            {
                var parts = pw.Branches.Select(b =>
                    LatexRender(b.Value, RootPrec) + " & \\text{if } " + LatexRender(b.Guard, RootPrec));
                return "\\begin{cases} " + string.Join(" \\\\ ", parts) +
                       " & \\text{otherwise} \\end{cases}";
            }
            case DerivativeExpr d:
                return "\\operatorname{diff}(" + LatexRender(d.Operand, ArgPrec) + ", " +
                       string.Join(", ", d.Variables.Select(v => LatexName(v.Name))) + ")";
            case IntegralExpr i2:
                return "\\operatorname{integrate}(" + LatexRender(i2.Operand, ArgPrec) + ", " +
                       string.Join(", ", i2.Variables.Select(v => LatexName(v.Name))) + ")";
            case RootOfExpr r2:
                return "\\operatorname{rootof}(" + LatexRender(r2.DefiningPolynomial.ToExpr(), ArgPrec) + ", " +
                       r2.RootIndex + ")";
            case AndExpr an:
                return LatexDelimit(string.Join(" \\land ", an.Operands.Select(o => LatexRender(o, AndPrec))),
                    parentPrec, AndPrec);
            case OrExpr or2:
                return LatexDelimit(string.Join(" \\lor ", or2.Operands.Select(o => LatexRender(o, OrPrec))),
                    parentPrec, OrPrec);
            case NotExpr nt:
                return LatexDelimit("\\neg " + LatexRender(nt.Operand, AtomPrec), parentPrec, AtomPrec);
            case OrderExpr o:
            {
                var (baseExpr, zeroPoint, degreeOne) = OrderShape(o);
                var baseText = LatexRender(baseExpr, RelationPrec);
                if (degreeOne)
                    return "O(" + baseText + ")";
                var body = zeroPoint ? baseText : "\\left(" + baseText + "\\right)";
                return "O(" + body + "^{" + LatexRender(o.Degree, RootPrec) + "})";
            }
            default:
                return e.Kind.ToString();
        }
    }

    /// <summary>A rational constant as LaTeX: an integer stays a numeral, everything else is a
    /// fraction. The sign belongs to the numeral, so a negative rational is <c>-\frac{1}{2}</c>.</summary>
    private static string LatexRational(Rat value)
    {
        if (value.IsInteger)
            return value.ToInteger().ToString();
        var abs = Rat.Abs(value);
        var body = "\\frac{" + abs.Numerator + "}{" + abs.Denominator + "}";
        return value.IsNegative ? "-" + body : body;
    }

    /// <summary>The LaTeX spelling of a symbol NAME. A name is opaque to the kernel — its identity
    /// is the string, ordinal — so the LaTeX arm renders it as literal characters, never as
    /// structure: <c>x_1</c> is <c>x\_1</c>, not <c>x_{1}</c>. The subscript form would be prettier
    /// for the common case but would invent structure the kernel does not have and is undefined
    /// for a leading, trailing or doubled underscore; the literal form is never wrong.
    /// <para>
    /// All ten characters LaTeX gives a meaning of its own (# $ % &amp; _ { } ~ ^ \) are escaped
    /// with the LaTeX2e spellings, so a name can neither change what it says nor break the
    /// surrounding document: a bare <c>%</c> comments out the rest of the line, a bare <c>}</c>
    /// closes the caller's group, a bare <c>\</c> starts a control sequence. The three characters
    /// with no control-symbol spelling (~ ^ \) use the text-symbol commands and carry the empty
    /// group <c>{}</c> that terminates the control word before a letter.
    /// </para>
    /// A name with no special character is returned unchanged, so ordinary renderings are
    /// byte-identical to the ones this arm produced before names were escaped.</summary>
    private static string LatexName(string name)
    {
        if (name.AsSpan().IndexOfAny("#$%&_{}~^\\") < 0)
            return name;
        var sb = new StringBuilder(name.Length + 8);
        foreach (char ch in name)
        {
            _ = ch switch
            {
                '#' => sb.Append("\\#"),
                '$' => sb.Append("\\$"),
                '%' => sb.Append("\\%"),
                '&' => sb.Append("\\&"),
                '_' => sb.Append("\\_"),
                '{' => sb.Append("\\{"),
                '}' => sb.Append("\\}"),
                '~' => sb.Append("\\textasciitilde{}"),
                '^' => sb.Append("\\textasciicircum{}"),
                '\\' => sb.Append("\\textbackslash{}"),
                _ => sb.Append(ch),
            };
        }
        return sb.ToString();
    }

    /// <summary>LaTeX spelling of a function name: the control sequences TeX already defines keep
    /// their spelling, everything else is an <c>\operatorname{...}</c>. Argument parentheses stay
    /// plain parentheses — they are call syntax, never a precedence decision.</summary>
    private static string LatexCallName(string name) => name switch
    {
        "sin" or "cos" or "tan" or "sinh" or "cosh" or "tanh" or "exp" or "log"
            or "min" or "max" or "det" or "lim" => "\\" + name,
        _ => "\\operatorname{" + name + "}",
    };

    /// <summary>The LaTeX twin of <see cref="AppendSigned"/>: the same sign decision
    /// (<see cref="IsNegativeTerm"/>) and the same <see cref="SubtrahendPrec"/>, which is why
    /// <c>x - (y - a)</c> keeps its parentheses in both renderings.</summary>
    private static void AppendSignedLatex(StringBuilder sb, Expr t)
    {
        if (IsNegativeTerm(t, out var inner))
        {
            sb.Append(" - ");
            sb.Append(LatexRender(inner, SubtrahendPrec));
        }
        else
        {
            sb.Append(" + ");
            sb.Append(LatexRender(t, AddPrec));
        }
    }

    private static void AppendSigned(StringBuilder sb, Expr t, bool unicode)
    {
        if (IsNegativeTerm(t, out var inner))
        {
            sb.Append(" - ");
            // the right operand of a minus binds tighter than a sum: x - (y - a) must not
            // render as x - y - a, which reparses to a different expression. Round 24.
            sb.Append(Pretty(inner, SubtrahendPrec, false, unicode));
        }
        else
        {
            sb.Append(" + ");
            sb.Append(Pretty(t, AddPrec, false, unicode));
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
