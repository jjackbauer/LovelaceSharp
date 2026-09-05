using Xunit;

// Several Suite tests mutate the process-global Real.MaxComputationDecimalPlaces /
// Real.DisplayDecimalPlaces statics directly (save/restore, patterns §14), and the executable
// Language.md doctest reads those same statics to format its expected output. Those statics are
// process-wide, so the suite must run single-threaded to avoid a precision race across classes
// (e.g. a pi() example rendering while a pi-precision test temporarily lowers the cap). The
// AsyncLocal scope (Real.WithPrecision) is the race-free mechanism; the global setter requires
// serialization.
[assembly: CollectionBehavior(DisableTestParallelization = true)]
