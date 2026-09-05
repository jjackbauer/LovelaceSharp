using Xunit;

// Several Real tests mutate the process-global Real.MaxComputationDecimalPlaces /
// Real.DisplayDecimalPlaces statics directly (save/restore, patterns §14). Those statics are
// process-wide, so the suite must run single-threaded to avoid a precision race across classes
// (e.g. a division test running while a sqrt test temporarily lowers the cap). The AsyncLocal
// scope (Real.WithPrecision) is the race-free mechanism; the global setter requires serialization.
[assembly: CollectionBehavior(DisableTestParallelization = true)]
