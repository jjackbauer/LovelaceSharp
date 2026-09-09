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

    private static Symbol[] CollectSymbols(Expr e)
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
            }
        }
        Walk(e);
        return set.ToArray();
    }

    // -----------------------------------------------------------------
    // Pretty print (display only — never used for identity)
    // -----------------------------------------------------------------

    public static string PrettyPrint(Expr e) => Pretty(e, 0, false);

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

    private static string Pretty(Expr e, int parentPrec, bool rightOfPower)
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
                NamedConstant.Pi => "pi",
                NamedConstant.E => "e",
                NamedConstant.I => "i",
                _ => "inf",
            };
            case AddExpr a:
            {
                var sb = new StringBuilder();
                for (int i = 0; i < a.Terms.Length; i++)
                {
                    var t = a.Terms[i];
                    if (i == 0)
                    {
                        sb.Append(Pretty(t, 1, false));
                        continue;
                    }
                    if (IsNegativeTerm(t, out var inner))
                    {
                        sb.Append(" - ");
                        sb.Append(Pretty(inner, 1, false));
                    }
                    else
                    {
                        sb.Append(" + ");
                        sb.Append(Pretty(t, 1, false));
                    }
                }
                var s = sb.ToString();
                return parentPrec > 1 ? "(" + s + ")" : s;
            }
            case MultiplyExpr m:
            {
                var nums = new List<string>();
                var dens = new List<string>();
                bool negative = false;
                int first = 0;
                if (m.Factors.Length > 0 && TermOrder.IsNumericConstant(m.Factors[0]) && m.Factors[0] is RationalConstantExpr rc && rc.Value.IsMinusOne)
                {
                    negative = true;
                    first = 1;
                }
                for (int i = first; i < m.Factors.Length; i++)
                {
                    var f = m.Factors[i];
                    if (f is PowerExpr p && p.Exponent is RationalConstantExpr pe && pe.Value.IsMinusOne)
                        dens.Add(Pretty(p.Base, 2, false));
                    else
                        nums.Add(Pretty(f, 2, false));
                }
                string text;
                if (dens.Count == 0)
                {
                    text = string.Join("*", nums);
                }
                else
                {
                    var numText = nums.Count == 0 ? "1" : string.Join("*", nums);
                    var denText = string.Join("*", dens.Select(d => d.Contains('+') || d.Contains('-') || d.Contains('*') ? "(" + d + ")" : d));
                    text = numText + "/" + denText;
                }
                if (negative) text = "-" + text;
                return parentPrec > 2 ? "(" + text + ")" : text;
            }
            case PowerExpr p:
            {
                var b = Pretty(p.Base, 3, false);
                // a power base must be parenthesized: x^y^2 is ambiguous ((x^y)^2 vs x^(y^2))
                if (Prec(p.Base) <= 3)
                    b = "(" + b + ")";
                var ex = Pretty(p.Exponent, 4, true);
                if (p.Exponent is RationalConstantExpr rce && !rce.Value.IsInteger)
                    ex = "(" + ex + ")";
                else if (p.Exponent is PowerExpr)
                    ex = "(" + ex + ")";
                var text = b + "^" + ex;
                return parentPrec > 3 ? "(" + text + ")" : text;
            }
            case FunctionExpr f:
            {
                var args = string.Join(", ", f.Arguments.Select(a => Pretty(a, 3, false)));
                return f.Function.Name + "(" + args + ")";
            }
            case RelationExpr r:
            {
                var op = r.Op switch
                {
                    RelOp.Eq => " = ",
                    RelOp.Ne => " != ",
                    RelOp.Lt => " < ",
                    RelOp.Le => " <= ",
                    RelOp.Gt => " > ",
                    _ => " >= ",
                };
                return Pretty(r.Left, 0, false) + op + Pretty(r.Right, 0, false);
            }
            case PiecewiseExpr pw:
            {
                var parts = pw.Branches.Select(b => Pretty(b.Value, 0, false) + " if " + Pretty(b.Guard, 0, false));
                return "piecewise(" + string.Join(", ", parts) + ", " + Pretty(pw.Otherwise, 0, false) + ")";
            }
            case DerivativeExpr d:
                return "diff(" + Pretty(d.Operand, 4, false) + ", " + string.Join(", ", d.Variables.Select(v => v.Name)) + ")";
            case IntegralExpr i:
                return "integrate(" + Pretty(i.Operand, 4, false) + ", " + string.Join(", ", i.Variables.Select(v => v.Name)) + ")";
            case RootOfExpr r:
                return "rootof(" + Pretty(r.DefiningPolynomial.ToExpr(), 4, false) + ", " + r.RootIndex + ")";
            case AndExpr an:
            {
                var text = string.Join(" and ", an.Operands.Select(o => Pretty(o, 0, false)));
                return parentPrec > 0 ? "(" + text + ")" : text;
            }
            case OrExpr or2:
            {
                var text = string.Join(" or ", or2.Operands.Select(o => Pretty(o, -1, false)));
                return parentPrec > -1 ? "(" + text + ")" : text;
            }
            case NotExpr nt:
            {
                var text = "not " + Pretty(nt.Operand, 4, false);
                return parentPrec > 4 ? "(" + text + ")" : text;
            }
            case OrderExpr o:
            {
                bool zeroPoint = o.Point is RationalConstantExpr rp && rp.Value.IsZero
                              || o.Point is IntegerConstantExpr ip && Int.IsZero(ip.Value);
                var baseExpr = zeroPoint ? o.Variable : Exprs.Subtract(o.Variable, o.Point);
                var baseText = Pretty(baseExpr, 3, false);
                bool degreeOne = o.Degree is RationalConstantExpr rd && rd.Value.IsOne
                              || o.Degree is IntegerConstantExpr id && id.Value == Int.One;
                if (degreeOne)
                    return "O(" + baseText + ")";
                var body = zeroPoint ? baseText : "(" + baseText + ")";
                return "O(" + body + "^" + Pretty(o.Degree, 3, false) + ")";
            }
            default:
                return e.Kind.ToString();
        }
    }

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
