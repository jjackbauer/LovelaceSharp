using System.Globalization;
using Cplx = Lovelace.Complex.Complex;
using Rat = Lovelace.Rational.Rational;
using Rl = Lovelace.Real.Real;

namespace Lovelace.Symbolics.Tests.Falsification;

/// <summary>
/// ADV-11: the five observation classes a falsifier may draw from one sample. They are exactly the
/// classes the advanced verifier distinguishes, and each one maps onto a distinction the kernel
/// already makes: <see cref="Agree"/>/<see cref="Disagree"/> come from the numeric comparison,
/// <see cref="UndefinedLeft"/>/<see cref="UndefinedRight"/> from the kernel's own typed
/// <see cref="EvaluationException"/> on that side only, and <see cref="BudgetOrRefusal"/> from the
/// typed resource refusals the product already throws.
/// </summary>
public enum ObservationClass
{
    /// <summary>Both sides evaluated and their difference is inside the stated tolerance.</summary>
    Agree,

    /// <summary>Both sides evaluated and their difference reaches the stated tolerance.</summary>
    Disagree,

    /// <summary>The left-hand side is not defined at this input (the kernel's typed domain refusal).</summary>
    UndefinedLeft,

    /// <summary>The right-hand side is not defined at this input (the kernel's typed domain refusal).</summary>
    UndefinedRight,

    /// <summary>The sample hit a resource budget the product refuses with a typed exception.</summary>
    BudgetOrRefusal,
}

/// <summary>What one side of one sample did. <see cref="Threw"/> is deliberately NOT an
/// <see cref="ObservationClass"/>: an unexpected exception type is a defect in the kernel or in the
/// harness, and folding it into a class would let it pass as "undefined here" - exactly the mistake
/// <c>FalsificationTests</c> refuses to make.</summary>
public enum SideOutcome
{
    /// <summary>The side produced a value.</summary>
    Evaluated,

    /// <summary>The kernel's typed domain refusal: the expression is not defined at this input.</summary>
    Undefined,

    /// <summary>A typed resource refusal (budget, depth, allocation, cancellation).</summary>
    Refused,

    /// <summary>Any other exception. A defect, reported as itself.</summary>
    Threw,
}

/// <summary>The one tolerance a comparison uses. It is a parameter of the classifier, never a
/// constant buried inside it: the caller states the tolerance and the text it printed from, so two
/// runs (or two processes) can be compared on the number the comparison actually used.</summary>
/// <param name="Value">The tolerance as a kernel value.</param>
/// <param name="Text">The tolerance rendered deterministically.</param>
public readonly record struct ComparisonTolerance(Num Value, string Text)
{
    /// <summary>The gate's historical tolerance: 1e-11, rendered as it is always rendered.</summary>
    public static ComparisonTolerance TenToTheMinusEleven() =>
        new(NumOps.FromReal(Rl.Parse("0." + new string('0', 11) + "1", null)), "1/100000000000");

    /// <summary>An exact rational tolerance. Exact on purpose: the value is a kernel
    /// <c>NumInt</c>/<c>NumRat</c>, so the comparison never rounds the tolerance itself.</summary>
    public static ComparisonTolerance OfRational(Rat tol) =>
        new(NumOps.FromRat(tol), FalsificationText.Rational(tol));
}

/// <summary>One side's outcome plus whatever it has to say for itself.</summary>
public sealed record SideObservation(SideOutcome Outcome, Num? Value, string Detail)
{
    public bool IsUndefined => Outcome == SideOutcome.Undefined;
    public bool IsRefused => Outcome == SideOutcome.Refused;
    public bool Threw => Outcome == SideOutcome.Threw;
}

/// <summary>The full result of classifying one sample: the class the falsifier branches on, plus the
/// two sides' outcomes so a defect can be reported as itself instead of as a class.</summary>
public sealed record ClassifiedObservation(
    ObservationClass Class,
    SideObservation Left,
    SideObservation Right,
    string? DeltaText)
{
    /// <summary>True when this sample is a counterexample to the identity under test.</summary>
    public bool IsCounterexample => Class == ObservationClass.Disagree;

    /// <summary>True when the sample revealed a defect rather than a mathematical fact.</summary>
    public bool IsDefect => Left.Threw || Right.Threw;
}

/// <summary>
/// The classifier. It evaluates both sides under ONE scope, classifies each side independently, and
/// then folds the two side outcomes into the single class the falsifier branches on, with an explicit
/// precedence so a defect can never be masked by a milder class.
/// </summary>
public static class ObservationClassifier
{
    /// <summary>Evaluates and classifies one sample of an identity at one input.</summary>
    /// <param name="lhs">The left-hand expression.</param>
    /// <param name="rhs">The right-hand expression.</param>
    /// <param name="ctx">The expression context.</param>
    /// <param name="variable">The probe symbol.</param>
    /// <param name="value">The exact input value.</param>
    /// <param name="tolerance">The stated comparison tolerance.</param>
    public static ClassifiedObservation Classify(
        Expr lhs,
        Expr rhs,
        ExprContext ctx,
        Symbol variable,
        Num value,
        ComparisonTolerance tolerance)
    {
        var bindings = new Dictionary<Symbol, Num> { [variable] = value };
        var left = EvaluateSide(lhs, ctx, bindings);
        var right = EvaluateSide(rhs, ctx, bindings);

        // Precedence, stated once and asserted by a test:
        //   1. a typed resource refusal outranks everything (the sample never got to run);
        //   2. then definedness, left before right (the left side is the subject of the identity);
        //   3. only when both sides produced a value does the comparison decide.
        if (left.IsRefused || right.IsRefused)
            return new ClassifiedObservation(ObservationClass.BudgetOrRefusal, left, right, null);

        if (left.IsUndefined)
            return new ClassifiedObservation(ObservationClass.UndefinedLeft, left, right, null);

        if (right.IsUndefined)
            return new ClassifiedObservation(ObservationClass.UndefinedRight, left, right, null);

        if (left.Threw || right.Threw)
            return new ClassifiedObservation(ObservationClass.BudgetOrRefusal, left, right, null);

        var (agree, deltaText) = Agrees(left.Value!, right.Value!, tolerance, ctx);
        return new ClassifiedObservation(
            agree ? ObservationClass.Agree : ObservationClass.Disagree, left, right, deltaText);
    }

    /// <summary>
    /// |lhs - rhs| against the stated tolerance, with the same care the existing gate takes: a
    /// complex difference is decided on its exact squared magnitude re^2 + im^2 rather than through
    /// a square root, so the tolerance check never depends on the transcendental sqrt path.
    /// </summary>
    public static (bool Agrees, string DeltaText) Agrees(
        Num left, Num right, ComparisonTolerance tolerance, ExprContext ctx)
    {
        var delta = NumOps.Subtract(left, right);
        if (delta is NumComplex c)
        {
            var squared = NumOps.FromReal(c.V.MagnitudeSquared);
            var tolSquared = NumOps.Multiply(tolerance.Value, tolerance.Value);
            var ok = NumOps.Compare(squared, tolSquared) < 0;
            return (ok, "|delta|^2 = " + FalsificationText.Num(squared));
        }

        var abs = NumOps.Abs(delta, ctx);
        var agrees = NumOps.Compare(abs, tolerance.Value) < 0;
        return (agrees, "|delta| = " + FalsificationText.Num(abs));
    }

    private static SideObservation EvaluateSide(
        Expr expression, ExprContext ctx, IReadOnlyDictionary<Symbol, Num> bindings)
    {
        try
        {
            return new SideObservation(
                SideOutcome.Evaluated, Evaluation.EvaluateToNum(expression, ctx, bindings), "");
        }
        catch (EvaluationException ex)
        {
            // The kernel's typed domain refusal. This is the ONLY exception that licenses
            // "undefined at this input".
            return new SideObservation(SideOutcome.Undefined, null, ex.Message);
        }
        catch (Exception ex) when (IsTypedResourceRefusal(ex))
        {
            return new SideObservation(SideOutcome.Refused, null, Describe(ex));
        }
        catch (Exception ex)
        {
            return new SideObservation(SideOutcome.Threw, null, Describe(ex));
        }
    }

    /// <summary>Whether an exception is one of the typed resource refusals the product throws.
    /// Checked by type, never by message.</summary>
    public static bool IsTypedResourceRefusal(Exception ex) =>
        ex is Lovelace.Symbolics.BudgetExceededException
           or Lovelace.Abstractions.InputDepthExceededException
           or Lovelace.Suite.ArrayAllocationRefusedException
           or Lovelace.Suite.EvaluationCancelledException;

    private static string Describe(Exception ex) => ex.GetType().Name + ": " + ex.Message;
}

/// <summary>Deterministic renderings shared by the whole falsifier. Every rendering here is a
/// function of the value alone: no culture-dependent formatting, no clock, no hash-set order.</summary>
public static class FalsificationText
{
    /// <summary>An exact rational as <c>n/d</c>, or <c>n</c> when it is an integer.</summary>
    public static string Rational(Rat value) =>
        value.IsInteger ? value.Numerator.ToString() : value.Numerator + "/" + value.Denominator;

    /// <summary>A complex value as its two exact decimal parts.</summary>
    public static string Complex(Cplx value) =>
        "(" + Decimal(value.Re) + "," + Decimal(value.Im) + ")";

    /// <summary>An exact-or-inexact real as its own decimal rendering.</summary>
    public static string Decimal(Rl value) => value.ToString("G", CultureInfo.InvariantCulture);

    /// <summary>A kernel number in one canonical rendering, whichever tier it lives in.</summary>
    public static string Num(Num value) =>
        Printing.PrettyPrint(Evaluation.NumToExpr(value));

    /// <summary>The assumption set an observation was taken under, in canonical order.</summary>
    public static string Environment(AssumptionSet env) =>
        env.Atoms.Length == 0
            ? "{}"
            : "{" + string.Join("; ", env.Atoms.Select(a => a.ToString())) + "}";

    /// <summary>Escapes the four characters that would break the probe's line-delimited text form.</summary>
    public static string Escape(string text)
    {
        var result = new System.Text.StringBuilder(text.Length);
        foreach (var ch in text)
        {
            switch (ch)
            {
                case '\\': result.Append("\\\\"); break;
                case '|': result.Append("\\p"); break;
                case '\n': result.Append("\\n"); break;
                case '\r': result.Append("\\r"); break;
                case '\t': result.Append("\\t"); break;
                default: result.Append(ch); break;
            }
        }
        return result.ToString();
    }

    /// <summary>The inverse of <see cref="Escape"/>, one pass, so no escape is applied twice.</summary>
    public static string Unescape(string text)
    {
        var result = new System.Text.StringBuilder(text.Length);
        for (var i = 0; i < text.Length; i++)
        {
            if (text[i] != '\\' || i + 1 >= text.Length)
            {
                result.Append(text[i]);
                continue;
            }
            i++;
            result.Append(text[i] switch
            {
                'n' => '\n',
                'r' => '\r',
                't' => '\t',
                'p' => '|',
                '\\' => '\\',
                _ => text[i],
            });
        }
        return result.ToString();
    }
}
