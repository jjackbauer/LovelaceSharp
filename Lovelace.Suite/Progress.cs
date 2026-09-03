namespace Lovelace.Suite;

/// <summary>A progress update from a long-running operation: a label and a 0..1 fraction.</summary>
public readonly record struct OperationProgress(string Label, double Fraction);
