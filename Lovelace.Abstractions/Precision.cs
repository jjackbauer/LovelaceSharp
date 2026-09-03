namespace Lovelace.Abstractions;

/// <summary>
/// The precision associated with an array's element arithmetic, expressed as a
/// significant-digit count. Carried as first-class metadata (ARR-004). The value is
/// derived by the language layer from the process-global precision knobs; this type is
/// deliberately independent of the scalar numeric projects.
/// </summary>
public readonly record struct Precision(int SignificantDigits)
{
    public override string ToString() => $"{SignificantDigits} sig";
}
