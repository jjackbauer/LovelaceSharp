// The acceptance test for the cycle-7 harnessability surface (ADV-07/08/09), run against a runner
// binary rather than asserted in prose:
//
//     node checks/wire_contract.mjs [path/to/Lovelace.Run.exe]
//
// Defaults to the benchmark's own runner copy. Exits non-zero if any contract regresses.
import { spawnSync } from 'node:child_process';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const exe = resolve(
  process.argv[2] ?? fileURLToPath(new URL('../runner/Lovelace.Run.exe', import.meta.url)));
let pass = 0, fail = 0;
const check = (name, ok, detail) => { (ok ? pass++ : fail++); console.log((ok ? 'PASS  ' : 'FAIL  ') + name.padEnd(58) + (detail ?? '')); };
const run = (argv, input) => spawnSync(exe, argv, { input, encoding: 'utf8' });

// ---- ADV-08: the registry surface, answered without evaluating anything ----------------------
const q = run(['--functions', 'det']);
const qe = JSON.parse(q.stdout);
check('ADV-08 --functions det exits 0', q.status === 0, 'exit ' + q.status);
check('ADV-08 --functions det payload is small', Buffer.byteLength(q.stdout) < 300, Buffer.byteLength(q.stdout) + ' B');
check('ADV-08 listing carries no documentation', !('summary' in (qe.functions[0] || {})), JSON.stringify(Object.keys(qe.functions[0] || {})));
check('ADV-08 page reports total/returned/truncated', qe.total === 1 && qe.returned === 1 && qe.truncated === false, `total=${qe.total} returned=${qe.returned} truncated=${qe.truncated}`);
check('ADV-08 query evaluates nothing', !('result' in qe) && !('timings' in qe) && !('revision' in qe), Object.keys(qe).filter(k => ['result','timings','revision'].includes(k)).join(',') || 'none');

const detail = run(['--function', 'det']);
const de = JSON.parse(detail.stdout);
check('ADV-08 --function det carries the documentation', ['summary','category','returnKind','related','examples'].every(k => k in de.function), Object.keys(de.function).join(','));
check('ADV-08 --function det is small', Buffer.byteLength(detail.stdout) < 700, Buffer.byteLength(detail.stdout) + ' B');

const unknown = run(['--function', 'nosuchfn']);
const ue = JSON.parse(unknown.stdout);
check('ADV-08 an unknown NAME is a refusal, not an empty success', unknown.status !== 0 && ue.ok === false && typeof ue.registrySize === 'number', ue.code + ' registrySize=' + ue.registrySize);
const empty = run(['--functions', 'nosuchpattern']);
const ee = JSON.parse(empty.stdout);
check('ADV-08 an unknown PATTERN is an empty result', empty.status === 0 && ee.ok === true && ee.total === 0, 'total=' + ee.total);

// ---- ADV-07: a call-site refusal carries the signature it was measured against ---------------
const arity = JSON.parse(run(['--stdin'], 'sin(1, 2)').stdout);
check('ADV-07 an arity refusal carries signature', arity.ok === false && arity.signature && typeof arity.signature === 'object', JSON.stringify(arity.signature).slice(0, 90));
check('ADV-07 the signature names the builtin and its parameters', arity.signature?.function === 'sin' && JSON.stringify(arity.signature?.parameters) === '["x"]', JSON.stringify(arity.signature).slice(0, 120));
check('ADV-07 the signature carries the measured arity', arity.signature?.expectedMin === 1 && arity.signature?.expectedMax === 1 && arity.signature?.actual === 2, JSON.stringify(arity.signature).slice(0, 160));
check('ADV-07 the refusal is InvalidArgument/TypeMismatch', arity.code === 'InvalidArgument' && arity.category === 'TypeMismatch', arity.code + '/' + arity.category);
const notCall = JSON.parse(run(['--stdin'], 'undefined_name_here').stdout);
check('ADV-07 signature is omitted when it is not a call-site refusal', notCall.ok === false && !('signature' in notCall), 'signature present: ' + ('signature' in notCall));

// ---- ADV-09: the committed state is observable on EVERY failure -------------------------------
const failed = JSON.parse(run(['--stdin'], 'a = 1; b = 2; zeros(2.5)').stdout);
check('ADV-09 an ordinary failure carries partialVariables', failed.ok === false && Array.isArray(failed.partialVariables), JSON.stringify(failed.partialVariables).slice(0, 80));
check('ADV-09 the committed bindings are the ones bound before the failure', (failed.partialVariables || []).map(v => v.name).sort().join(',') === 'a,b', (failed.partialVariables || []).map(v => v.name).join(','));
const refused = JSON.parse(run(['--stdin'], 'zeros(2.5)').stdout);
check('ADV-09 the digit-count refusal speaks the contract grammar', /zeros\(\): argument 1 \(a dimension\)/.test(refused.message), refused.message.slice(0, 80));

// ---- success shape ----------------------------------------------------------------------------
const ok = JSON.parse(run(['--stdin'], 'print(scale([1,2,3], 2.5))').stdout);
check('success envelope carries artifacts[]', ok.ok === true && Array.isArray(ok.artifacts), JSON.stringify(ok.artifacts));
check('success envelope carries the sealed fields', ['protocolVersion','symbolicFormatVersion','mathIrVersion','timings','elapsedTime','variables','output'].every(k => k in ok), Object.keys(ok).sort().join(','));
check('scale multiplies by a scalar (ADV-03 descriptor fix)', JSON.stringify(ok.output) === '["[2.5, 5, 7.5]"]', JSON.stringify(ok.output));

console.log();
console.log(`${pass} passed, ${fail} failed`);
process.exit(fail === 0 ? 0 : 1);
