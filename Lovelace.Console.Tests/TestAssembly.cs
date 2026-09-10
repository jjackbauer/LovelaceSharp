// The REPL sessions share process-wide formatting/console affordances; run the
// transcript tests sequentially so precision and pretty-mode knobs cannot leak
// between concurrently executing sessions.
[assembly: CollectionBehavior(DisableTestParallelization = true)]
