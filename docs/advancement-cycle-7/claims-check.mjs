import { spawnSync } from 'node:child_process';
import { statSync, readFileSync, existsSync } from 'node:fs';
import { createHash } from 'node:crypto';

const ROOT = 'C:\\Users\\ricar\\dev\\LovelaceSharp';
const exe = ROOT + '\\Lovelace.Run\\bin\\Release\\net10.0\\publish\\Lovelace.Run.exe';
let pass = 0, fail = 0;
const claim = (text, ok, detail) => { ok ? pass++ : fail++; console.log((ok ? 'HOLDS ' : 'BROKEN') + '  ' + text.padEnd(64) + (detail ?? '')); };
const run = (argv, input) => spawnSync(exe, argv, { input, encoding: 'utf8' });
const bytes = (s) => Buffer.byteLength(s);
const sha = (p) => createHash('sha256').update(readFileSync(p)).digest('hex').slice(0, 16).toUpperCase();

// ---- ADV-08: the payload numbers cited in evidence.md and RESULTS.md -------------------------
const q = run(['--functions', 'det']);
claim('ADV-08 "--functions det" is 276 bytes', bytes(q.stdout) === 276, bytes(q.stdout) + ' B');
const d = run(['--function', 'det']);
claim('ADV-08 "--function det" is 459 bytes', bytes(d.stdout) === 459, bytes(d.stdout) + ' B');
const full = run(['--stdin'], '0');
claim('the full envelope is ~41.7 KB', bytes(full.stdout) > 41000 && bytes(full.stdout) < 42500, bytes(full.stdout) + ' B');
const ratio = bytes(full.stdout) / bytes(q.stdout);
claim('the query is at least 150x smaller than the envelope', ratio >= 150, ratio.toFixed(1) + 'x');
const help = run(['--help']);
claim('--help documents the registry flags', help.status === 0 && help.stdout.includes('--functions') && help.stdout.includes('--function-limit'), bytes(help.stdout) + ' B, mentions both flags');

// ---- ADV-07/09: the refusals and the committed state ----------------------------------------
const arity = JSON.parse(run(['--stdin'], 'sin(1, 2)').stdout);
claim('ADV-07 arity refusal is 650 bytes', bytes(run(['--stdin'], 'sin(1, 2)').stdout) === 650, bytes(run(['--stdin'], 'sin(1, 2)').stdout) + ' B');
claim('ADV-07 signature.actual is 2', arity.signature?.actual === 2, JSON.stringify(arity.signature));
const zeros = JSON.parse(run(['--stdin'], 'zeros(2.5)').stdout);
claim('ADV-03 zeros(2.5) refusal is 694 bytes', bytes(run(['--stdin'], 'zeros(2.5)').stdout) === 694, bytes(run(['--stdin'], 'zeros(2.5)').stdout) + ' B');
claim('ADV-03 the zeros refusal names the position and the parameter', /zeros\(\): argument 1 \(a dimension\)/.test(zeros.message), zeros.message.slice(0, 70));
claim('ADV-03 scale multiplies by a scalar', JSON.stringify(JSON.parse(run(['--stdin'], 'print(scale([1,2,3], 2.5))').stdout).output) === '["[2.5, 5, 7.5]"]');
const b = JSON.parse(run(['--stdin'], 'a=1;b=2;zeros(2.5)').stdout);
claim('ADV-09 partialVariables carries a,b', (b.partialVariables || []).map(v => v.name).sort().join(',') === 'a,b', JSON.stringify((b.partialVariables || []).map(v => v.name)));

// ---- ADV-04b: the width contract, at the language surface ------------------------------------
const w = (s) => { const e = JSON.parse(run(['--stdin'], s).stdout); const t = String((e.output || [''])[0] || ''); return t.includes('.') ? t.split('.')[1].length : 0; };
claim('width: evalf(sin(10), 30) publishes 30', w('print(evalf(sin(10), 30))') === 30, w('print(evalf(sin(10), 30))') + '');
claim('width: pi(30) publishes 30', w('print(pi(30))') === 30, w('print(pi(30))') + '');
claim('width: setprecision(30); sqrt(2) publishes 30', w('setprecision(30); print(sqrt(2))') === 30, w('setprecision(30); print(sqrt(2))') + '');
claim('width: setprecision(40); 1/(1009*10^1000) publishes 40', w('setprecision(40); print(1/(1009*10^1000))') === 40, w('setprecision(40); print(1/(1009*10^1000))') + '');

// ---- cited file sizes -------------------------------------------------------------------------
const P = (p) => ROOT + '\\' + p.replace(/\//g, '\\\\');
for (const [p, size] of [
  ['Lovelace.Abstractions/ExactIntegerArgument.cs', 8300],
  ['Lovelace.Abstractions/ArgumentContractException.cs', 10168],
  ['Lovelace.Run.Tests/IntegerArgumentBoundaryMatrixTests.cs', 30086],
  ['Lovelace.Real.Tests/RealTrigPrecisionOracleTests.cs', 13226],
  ['harnessability-bench/checks/wire_contract.mjs', 5312],
]) {
  const got = existsSync(P(p)) ? statSync(P(p)).size : -1;
  claim('cited size of ' + p.split('/').pop(), Math.abs(got - size) <= 64, got + ' B (cited ' + size + ')');
}

// ---- the published artefacts ------------------------------------------------------------------
claim('AOT publish path is 6 020 096 B, sha256 E4BA237A9153E6B6',
  statSync(exe).size === 6020096 && sha(exe) === 'E4BA237A9153E6B6', statSync(exe).size + ' B ' + sha(exe));
const bench = ROOT + '\\harnessability-bench\\runner\\Lovelace.Run.exe';
claim('the benchmark runner copy is byte-identical to it', sha(bench) === sha(exe), sha(bench));
claim('the AOT artefact is self-contained (no coreclr.dll beside it)', !existsSync(ROOT + '\\Lovelace.Run\\bin\\Release\\net10.0\\publish\\coreclr.dll'));
const evidenceText = readFileSync(ROOT + '\\docs\\advancement-cycle-7\\evidence.md', 'utf8');
claim('the docs record that out/aot is a framework-dependent apphost, not AOT',
  /out[\\/]aot[\s\S]{0,400}(framework-dependent apphost|NOT an AOT artefact)/.test(evidenceText));
// an AFFIRMATIVE claim only: "out/aot … is a Native AOT" must never appear. The notes that say
// "out/aot is NOT a Native AOT artefact" and the correction quoting the old string are the point of
// the correction, not a violation of it.
// Line-based rather than one regex, because the correction necessarily QUOTES the wrong claim: a line
// that also carries a negation or a correction marker is describing the mis-reading, not repeating it.
const aotAssertions = evidenceText
  .split(/\r?\n/)
  .filter((l) => /out[\\/]aot/i.test(l) && /Native AOT/i.test(l))
  .filter((l) => !/\bNOT\b|\bnot an\b|framework-dependent|SUPERSEDED|mis-read|misreading/i.test(l));
claim('the docs do not ASSERT out/aot is a Native AOT artefact', aotAssertions.length === 0,
  aotAssertions.length ? aotAssertions[0].slice(0, 120) : '0 assertions');

console.log();
console.log(pass + ' claims HOLD, ' + fail + ' BROKEN');
process.exit(fail === 0 ? 0 : 1);
