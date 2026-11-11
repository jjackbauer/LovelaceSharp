#!/usr/bin/env node
// Protocol fixture gate: the 20 golden envelopes in Lovelace.Run.Tests/fixtures.
//
// Two independent claims are checked:
//   oracle: the frozen C# runner still reproduces its own goldens (so the goldens are not stale)
//   rust:   the Rust runner reproduces the same goldens under the SAME normalisation
//
// The normalisation is the reference's own: the four volatile keys at any depth. Nothing else is
// excused. Exits 0 only when both sides pass every fixture.
import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';

const root = path.resolve(import.meta.dirname, '..', '..');
const fixturesDir = path.join(root, 'Lovelace.Run.Tests', 'fixtures');
const VOLATILE = new Set(['revision', 'elapsed', 'elapsedTime', 'timings']);
const args = process.argv.slice(2);
const onlyRust = args.includes('--rust-only');
const onlyOracle = args.includes('--oracle-only');
const verbose = args.includes('--verbose');
const filterArg = args.find(a => a.startsWith('--filter='));
const filter = filterArg ? filterArg.slice('--filter='.length) : null;

const oracle = (() => {
  const candidates = [
    path.join(root, 'Lovelace.Run', 'bin', 'Release', 'net10.0', 'Lovelace.Run.dll'),
    path.join(root, 'Lovelace.Run', 'bin', 'Debug', 'net10.0', 'Lovelace.Run.dll'),
  ];
  const dll = candidates.find(p => fs.existsSync(p));
  if (!dll) return null;
  return { command: process.platform === 'win32' ? 'dotnet' : 'dotnet', args: ['exec', dll], dll };
})();
const rust = path.join(root, 'rust', 'target', 'release', process.platform === 'win32' ? 'lovelace-run.exe' : 'lovelace-run');

function normalise(node) {
  if (Array.isArray(node)) return node.map(normalise);
  if (node && typeof node === 'object') {
    const out = {};
    for (const key of Object.keys(node).sort()) out[key] = VOLATILE.has(key) ? '<volatile>' : normalise(node[key]);
    return out;
  }
  return node;
}

function collect(expected, actual, where, out) {
  if (out.length >= 6) return;
  if (expected === null || actual === null) {
    if (expected !== actual) out.push(where + ': expected ' + JSON.stringify(expected) + ', got ' + JSON.stringify(actual));
    return;
  }
  const eo = typeof expected === 'object', ao = typeof actual === 'object';
  if (!eo || !ao) {
    if (JSON.stringify(expected) !== JSON.stringify(actual)) out.push(where + ': expected ' + JSON.stringify(expected) + ', got ' + JSON.stringify(actual));
    return;
  }
  if (Array.isArray(expected) !== Array.isArray(actual)) { out.push(where + ': array/object kind mismatch'); return; }
  if (Array.isArray(expected)) {
    if (expected.length !== actual.length) out.push(where + ': length ' + expected.length + ' != ' + actual.length);
    for (let i = 0; i < Math.min(expected.length, actual.length); i++) collect(expected[i], actual[i], where + '[' + i + ']', out);
    return;
  }
  const ek = Object.keys(expected).sort(), ak = Object.keys(actual).sort();
  if (ek.join('\u0000') !== ak.join('\u0000')) out.push(where + ': keys [' + ek.join(',') + '] != [' + ak.join(',') + ']');
  for (const k of ek) if (ak.includes(k)) collect(expected[k], actual[k], where + '.' + k, out);
}

function runOne(binary, fixture) {
  const scriptPath = path.join(fixturesDir, fixture + '.ls');
  const res = spawnSync(binary.command, [...binary.args, '--file', scriptPath, '--omit-functions'], {
    encoding: 'utf8', maxBuffer: 64 * 1024 * 1024, timeout: 300000,
  });
  return { exitCode: res.status, stdout: res.stdout || '', stderr: res.stderr || '', error: res.error };
}

function checkSide(label, binary) {
  const names = fs.readdirSync(fixturesDir).filter(f => f.endsWith('.ls')).map(f => f.slice(0, -3)).filter(n => !filter || n.includes(filter)).sort();
  const rows = [];
  for (const name of names) {
    const goldenPath = path.join(fixturesDir, name + '.json');
    const expectedExit = name === 'error_envelope' ? 1 : 0;
    const row = { name, pass: false, reasons: [] };
    if (!fs.existsSync(goldenPath)) { row.reasons.push('missing golden'); rows.push(row); continue; }
    const golden = JSON.parse(fs.readFileSync(goldenPath, 'utf8'));
    const result = runOne(binary, name);
    if (result.error) row.reasons.push('spawn failed: ' + result.error.message);
    if (result.exitCode !== expectedExit) row.reasons.push('exit ' + result.exitCode + ' expected ' + expectedExit);
    if (result.stderr.trim().length > 0) row.reasons.push('stderr not empty: ' + result.stderr.trim().slice(0, 160));
    let actual = null;
    try { actual = JSON.parse(result.stdout); } catch (e) { row.reasons.push('stdout is not one JSON document: ' + String(e).slice(0, 120)); }
    if (actual !== null) collect(normalise(golden), normalise(actual), '$', row.reasons);
    row.pass = row.reasons.length === 0;
    rows.push(row);
  }
  const passed = rows.filter(r => r.pass).length;
  console.log(label + ': ' + passed + '/' + rows.length);
  for (const r of rows) {
    if (!r.pass) {
      console.log('  FAIL ' + r.name);
      for (const reason of r.reasons.slice(0, verbose ? 99 : 3)) console.log('    ' + reason.slice(0, 300));
    }
  }
  return { label, passed, total: rows.length, rows };
}

const results = [];
if (!onlyRust) {
  if (!oracle) { console.error('oracle: FAIL (no Lovelace.Run.dll found)'); process.exit(2); }
  results.push(checkSide('oracle', oracle));
}
if (!onlyOracle) {
  if (!fs.existsSync(rust)) { console.error('rust: FAIL (missing ' + rust + ')'); process.exit(2); }
  results.push(checkSide('rust', { command: rust, args: [] }));
}

const failed = results.filter(r => r.passed !== r.total);
if (failed.length) {
  console.error('FAIL: ' + failed.map(r => r.label + ' ' + r.passed + '/' + r.total).join(', '));
  process.exit(1);
}
console.log('PASS: ' + results.map(r => r.label + ' ' + r.passed + '/' + r.total).join(', '));
