// ---------------------------------------------------------------------------
// Lovelace.Run — a non-interactive script runner over Lovelace.Suite.
//
// Evaluates a script (from --eval, --file, or --stdin) and emits a single JSON
// envelope on stdout so it can be driven by scripts, tests, or the DSH plugin.
//
//   Lovelace.Run --eval "x = 1..10" --json
//   Lovelace.Run script.ls --plot-dir out
//
// Protocol contract (versioned):
//   * stdout carries the envelope and NOTHING else. Anything the script prints
//     with print() is captured and returned in the envelope's "output" array.
//   * Every value is serialized structurally — arrays carry their shape, symbolic
//     values carry BOTH their canonical and pretty forms, complex values carry
//     re/im, domains are domain values, and an absent field is null.
//   * JSON keys are camelCase; structured record FIELD NAMES are snake_case.
//
// Exit codes: 0 success, 1 script/diagnostic error, 2 usage error.
//
// The CLI itself lives in Lovelace.Run.Runner (Runner.cs) so the envelope can be
// captured in-process by Lovelace.Run.Tests; this file is only the entry point.
// ---------------------------------------------------------------------------

return await Lovelace.Run.Runner.RunAsync(args, Console.Out, Console.Error);
