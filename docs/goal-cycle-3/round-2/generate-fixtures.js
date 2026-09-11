#!/usr/bin/env node
// Generates the Lovelace.Run envelope fixture corpus.
//
//   node docs/goal-cycle-3/round-2/generate-fixtures.js
//
// For every corpus row this writes Lovelace.Run.Tests/fixtures/<name>.ls (the script) and
// <name>.json (the envelope the REAL runner emits for it, with the volatile values — revision,
// elapsed, elapsedTime, timings — replaced by "<volatile>", pretty-printed with 2-space
// indentation and a trailing newline). Re-running it after an intentional protocol change is how
// the goldens are refreshed; an unintentional change makes the test fail instead.
'use strict';
const fs = require('fs');
const path = require('path');
const { spawnSync } = require('child_process');

const repo = path.resolve(__dirname, '..', '..', '..');
const runnerDll = path.join(repo, 'Lovelace.Run', 'bin', 'Release', 'net10.0', 'Lovelace.Run.dll');
const fixtures = path.join(repo, 'Lovelace.Run.Tests', 'fixtures');

// name -> script, in the order the corpus is documented. Every row is run with
// --omit-functions, exactly as the golden test runs it.
const CORPUS = {
  symbolic: 'x = symbol("x"); x^2 + 1',
  vector: 'x = symbol("x"); [x, x, x]',
  array: 'x = symbol("x"); y = symbol("y"); [[x, y], [y, x]]',
  record: 'x = symbol("x"); solve_full(x^2 - 4 == 0, x)',
  nested_record: 'x = symbol("x"); solve_full(sin(x) == 0, x)',
  solve_result: 'x = symbol("x"); solve_full(x^4 - x^2 - 1 == 0, x)',
  transform_result: 'x = symbol("x"); simplify_full(sin(x)^2 + cos(x)^2)',
  limit_result: 'x = symbol("x"); limit_full(sin(x)/x, x, 0)',
  system_solve: 'x = symbol("x"); y = symbol("y"); solve_system_full([x + y == 1, x - y == 3], [x, y])',
  compilation: 'x = symbol("x"); compile_full(x^2 + 1, [x])',
  // The language has no complex literal (Lovelace.Suite/docs/Language.md:806) and 'i' is not
  // defined, so the complex row uses the documented route: a DSP builtin (Language.md:787-807).
  complex: 'dft([1, 2, 3])',
  absent_field: 'x = symbol("x"); inspect(x^2 + 1)',
  error_envelope: 'x = symbol("x"); solve(2*x == 1, x, integer)',
  print_purity: 'print("hello from the script"); 1 + 1',
};
const EXPECTED_EXIT = { error_envelope: 1 };

const VOLATILE = new Set(['revision', 'elapsed', 'elapsedTime', 'timings']);

function normalise(value) {
  if (Array.isArray(value)) return value.map(normalise);
  if (value && typeof value === 'object') {
    const out = {};
    for (const key of Object.keys(value)) out[key] = VOLATILE.has(key) ? '<volatile>' : normalise(value[key]);
    return out;
  }
  return value;
}

fs.mkdirSync(fixtures, { recursive: true });
let rows = 0;
for (const [name, script] of Object.entries(CORPUS)) {
  const scriptPath = path.join(fixtures, name + '.ls');
  fs.writeFileSync(scriptPath, script + '\n', 'utf8');

  const run = spawnSync('dotnet', ['exec', runnerDll, '--file', scriptPath, '--omit-functions'],
    { encoding: 'utf8', maxBuffer: 64 * 1024 * 1024 });
  const expectedExit = EXPECTED_EXIT[name] || 0;
  if (run.status !== expectedExit) {
    console.error(name + ': runner exited ' + run.status + ', expected ' + expectedExit + '\n' + run.stderr);
    process.exit(1);
  }

  const envelope = JSON.parse(run.stdout); // throws unless stdout is exactly one JSON document
  fs.writeFileSync(path.join(fixtures, name + '.json'),
    JSON.stringify(normalise(envelope), null, 2) + '\n', 'utf8');
  console.log(name.padEnd(17) + ' exit=' + run.status +
    '  ok=' + envelope.ok +
    '  revision=' + envelope.revision +
    '  elapsed=' + JSON.stringify(envelope.elapsed));
  rows++;
}
if (rows !== 14) {
  console.error('corpus row count ' + rows + ' != 14');
  process.exit(1);
}
console.log('wrote ' + rows + ' script + ' + rows + ' golden fixtures to ' + fixtures);
