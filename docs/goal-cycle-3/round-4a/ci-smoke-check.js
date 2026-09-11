#!/usr/bin/env node
// Local proxy for the aot-smoke job's protocol assertions (.github/workflows/ci.yml).
//   node docs/goal-cycle-3/round-4a/ci-smoke-check.js
// GitHub Actions cannot run here (no python3), so this runs the SAME published AOT binary over
// the SAME five scenario scripts and performs the SAME JSON assertions with node instead of
// python3. It is a local proxy for the assertions, not a run of the workflow.
'use strict';
const { spawnSync } = require('child_process');
const fs = require('fs'), os = require('os'), path = require('path');

const exe = path.resolve('out/aot/Lovelace.Run.exe');
const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'ci-smoke-'));
let failures = 0;

function run(name, source, expectExit) {
  const scriptPath = path.join(tmp, name + '.ls');
  fs.writeFileSync(scriptPath, source + '\n', 'utf8');
  const r = spawnSync(exe, ['--file', scriptPath, '--omit-functions'], { encoding: 'utf8', maxBuffer: 64 * 1024 * 1024 });
  if (r.status !== expectExit) {
    console.log('FAIL ' + name + ': exit ' + r.status + ', expected ' + expectExit + '\n' + r.stderr);
    failures++;
  }
  return JSON.parse(r.stdout);
}
function check(label, condition, detail) {
  if (condition) { console.log('ok   ' + label); return; }
  console.log('FAIL ' + label + ' -> ' + JSON.stringify(detail));
  failures++;
}
const fields = (structured) => Object.fromEntries(structured.fields.map(f => [f.name, f.value]));

// 1. a complete solve reports Solved / Complete with per-solution conditions
{
  const env = run('solve', 'x = symbol("x")\nsolve_full(x^2 - 4 == 0, x)', 0);
  check('solve: ok/protocol/versions', env.ok === true && env.protocolVersion === 1 &&
    env.symbolicFormatVersion.startsWith('#!lovelace-sym') && env.mathIrVersion >= 1, env);
  const s = env.result.structured;
  check('solve: record shape', s.kind === 'Record' && s.type === 'SolveResult', s);
  const f = fields(s);
  check('solve: status value', f.status.value === 'Solved', f.status);
  check('solve: status kind is Enum', f.status.kind === 'Enum', f.status);
  check('solve: status type is SolveStatus', f.status.type === 'SolveStatus', f.status);
  check('solve: completeness kind is Enum', f.completeness.kind === 'Enum', f.completeness);
  check('solve: completeness type is Completeness', f.completeness.type === 'Completeness', f.completeness);
  check('solve: complete kind is Boolean', f.complete.kind === 'Boolean', f.complete);
  check('solve: complete is "true"', f.complete.value === 'true', f.complete);
  check('solve: domain', f.domain.kind === 'Domain' && f.domain.domain === 'complex', f.domain);
  check('solve: solutions shape', f.solutions.kind === 'Array' && JSON.stringify(f.solutions.shape) === '[2]', f.solutions);
  const sf = fields(f.solutions.elements[0]);
  check('solve: first solution symbolic', sf.value.kind === 'Symbolic' && sf.value.canonical.startsWith('(rat'), sf.value);
  check('solve: exactness value', ['Exact', 'AlgebraicExact'].includes(sf.exactness.value), sf.exactness);
  check('solve: exactness kind is Enum', sf.exactness.kind === 'Enum', sf.exactness);
  check('solve: exactness type', sf.exactness.type === 'SolutionExactness', sf.exactness);
}

// 2. an incomplete solve must NOT claim completeness
{
  const env = run('partial', 'x = symbol("x")\nsolve_full(x^4 - x^2 - 1 == 0, x)', 0);
  const f = fields(env.result.structured);
  check('partial: status Partial', f.status.value === 'Partial', f.status);
  check('partial: status kind is Enum', f.status.kind === 'Enum', f.status);
  check('partial: status type is SolveStatus', f.status.type === 'SolveStatus', f.status);
  check('partial: completeness kind is Enum', f.completeness.kind === 'Enum', f.completeness);
  check('partial: completeness Partial', f.completeness.value === 'Partial', f.completeness);
  check('partial: complete kind is Boolean', f.complete.kind === 'Boolean', f.complete);
  check('partial: complete is "false"', f.complete.value === 'false', f.complete);
  check('partial: unrepresented_count 2', Number(f.unrepresented_count.value) === 2, f.unrepresented_count);
}

// 3. arrays preserve shape
{
  const env = run('jac', 'x = symbol("x")\ny = symbol("y")\njacobian([x*y, x+y], [x, y])', 0);
  const s = env.result.structured;
  check('jacobian: rank-2 shape', s.kind === 'Array' && JSON.stringify(s.shape) === '[2,2]' && s.elements.length === 4, s);
  check('jacobian: first element', s.elements[0].kind === 'Symbolic' && s.elements[0].canonical === '(sym y)', s.elements[0]);
}

// 4. stdout purity
{
  const env = run('print', 'print("hello from the script")\n1 + 1', 0);
  check('print: output captured', JSON.stringify(env.output) === '["hello from the script"]', env.output);
  check('print: result is 2', env.result.structured.value === '2', env.result.structured);
}

// 5. errors are structural
{
  const env = run('err', 'x = symbol("x")\nsolve(x^2 + 1 == 0, x, integer)', 1);
  check('error: ok false', env.ok === false, env.ok);
  check('error: message names the domain', String(env.message).includes('got integer'), env.message);
  check('error: code and category', Boolean(env.code) && Boolean(env.category), env);
}

console.log(failures === 0 ? 'ALL CI SMOKE ASSERTIONS PASS (local node proxy)' : failures + ' assertion(s) FAILED');
process.exit(failures === 0 ? 0 : 1);
