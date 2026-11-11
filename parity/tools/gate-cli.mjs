#!/usr/bin/env node
// CLI surface gate (RUN-001..RUN-014).
//
// Each row is executed through BOTH runners with identical argv, identical stdin, and the same
// materialised files. The oracle's own output is checked against the row's semantic asserts first,
// so a row that does not exercise its capability fails instead of passing vacuously. Then the Rust
// runner must match the oracle: same exit code, and the same normalised JSON (or the same text).
import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import { spawnSync } from 'node:child_process';

const root = path.resolve(import.meta.dirname, '..', '..');
const matrix = JSON.parse(fs.readFileSync(path.join(root, 'parity', 'cli-matrix.json'), 'utf8'));
const VOLATILE = new Set(['revision', 'elapsed', 'elapsedTime', 'timings']);
const args = process.argv.slice(2);
const filterArg = args.find(a => a.startsWith('--filter='));
const filter = filterArg ? filterArg.slice('--filter='.length) : null;
const required = filter ? [] : Array.from({ length: 14 }, (_, i) => 'RUN-' + String(i + 1).padStart(3, '0'));

const oracle = (() => {
  const candidates = [
    path.join(root, 'Lovelace.Run', 'bin', 'Release', 'net10.0', 'Lovelace.Run.dll'),
    path.join(root, 'Lovelace.Run', 'bin', 'Debug', 'net10.0', 'Lovelace.Run.dll'),
  ];
  const dll = candidates.find(p => fs.existsSync(p));
  return dll ? { command: 'dotnet', args: ['exec', dll] } : null;
})();
const rustBin = path.join(root, 'rust', 'target', 'release', process.platform === 'win32' ? 'lovelace-run.exe' : 'lovelace-run');

function normalise(node, ignore) {
  if (Array.isArray(node)) return node.map(n => normalise(n, ignore));
  if (node && typeof node === 'object') {
    const out = {};
    for (const key of Object.keys(node).sort()) out[key] = VOLATILE.has(key) ? '<volatile>' : normalise(node[key], ignore);
    return out;
  }
  return node;
}

// A row may declare string patterns that quote measured quantities (e.g. a diagnostic that embeds
// elapsed milliseconds). Only the patterns declared on that row are rewritten, and the row's note
// says why; everything else compares exactly.
function applyPatterns(node, patterns) {
  if (!patterns || patterns.length === 0) return node;
  if (Array.isArray(node)) return node.map(n => applyPatterns(n, patterns));
  if (node && typeof node === 'object') {
    const out = {};
    for (const key of Object.keys(node)) out[key] = applyPatterns(node[key], patterns);
    return out;
  }
  if (typeof node !== 'string') return node;
  let text = node;
  for (const p of patterns) text = text.replace(new RegExp(p.find, 'g'), p.replace);
  return text;
}

function setPath(obj, dotted, value) {
  const parts = dotted.replace(/^\$\./, '').split('.');
  let cur = obj;
  for (let i = 0; i < parts.length - 1; i++) {
    if (cur === null || typeof cur !== 'object' || !(parts[i] in cur)) return false;
    cur = cur[parts[i]];
  }
  if (cur === null || typeof cur !== 'object' || !(parts[parts.length - 1] in cur)) return false;
  cur[parts[parts.length - 1]] = value;
  return true;
}

function getPath(obj, dotted) {
  const parts = dotted.replace(/^\$\./, '').split('.');
  let cur = obj;
  for (const part of parts) {
    if (cur === null || cur === undefined) return undefined;
    if (/^\d+$/.test(part)) {
      if (!Array.isArray(cur)) return undefined;
      cur = cur[Number(part)];
    } else {
      if (typeof cur !== 'object' || !(part in cur)) return undefined;
      cur = cur[part];
    }
  }
  return cur;
}

function same(a, b, where, out) {
  if (out.length >= 5) return;
  if (a === null || b === null) { if (a !== b) out.push(where + ': ' + JSON.stringify(a) + ' != ' + JSON.stringify(b)); return; }
  if (typeof a !== typeof b) { out.push(where + ': type ' + typeof a + ' != ' + typeof b); return; }
  if (typeof a !== 'object') { if (a !== b) out.push(where + ': ' + JSON.stringify(a) + ' != ' + JSON.stringify(b)); return; }
  if (Array.isArray(a) !== Array.isArray(b)) { out.push(where + ': array/object mismatch'); return; }
  if (Array.isArray(a)) {
    if (a.length !== b.length) out.push(where + ': length ' + a.length + ' != ' + b.length);
    for (let i = 0; i < Math.min(a.length, b.length); i++) same(a[i], b[i], where + '[' + i + ']', out);
    return;
  }
  const ak = Object.keys(a).sort(), bk = Object.keys(b).sort();
  if (ak.join('\u0000') !== bk.join('\u0000')) out.push(where + ': keys [' + ak.join(',') + '] != [' + bk.join(',') + ']');
  for (const k of ak) if (bk.includes(k)) same(a[k], b[k], where + '.' + k, out);
}

function run(binary, row, tmp) {
  const argv = row.argv.map(a => a.replaceAll('{tmp}', tmp));
  const res = spawnSync(binary.command, [...binary.args, ...argv], {
    input: row.stdin !== undefined ? Buffer.from(row.stdin, 'utf8') : undefined,
    encoding: 'utf8', timeout: 300000, maxBuffer: 64 * 1024 * 1024,
  });
  return { exitCode: res.status, stdout: res.stdout || '', stderr: res.stderr || '', error: res.error };
}

function checkAsserts(row, envelope, exitCode, stdout, stderr) {
  const reasons = [];
  const PSEUDO = new Set(['stdoutStartsWith', 'stdoutContains', 'stderrContains', 'outputContains']);
  for (const a of row.assert || []) {
    const value = a.path === 'exitCode' ? exitCode : PSEUDO.has(a.path) ? undefined : a.path.startsWith('envelope.') ? getPath(envelope, a.path.slice('envelope.'.length)) : getPath(envelope, a.path);
    if (Object.prototype.hasOwnProperty.call(a, 'equals') && !PSEUDO.has(a.path)) {
      if (Array.isArray(value) || (value && typeof value === 'object')) {
        if (JSON.stringify(value) !== JSON.stringify(a.equals)) reasons.push('assert ' + a.path + ' expected ' + JSON.stringify(a.equals) + ', got ' + JSON.stringify(value));
      } else if (value !== a.equals) reasons.push('assert ' + a.path + ' expected ' + JSON.stringify(a.equals) + ', got ' + JSON.stringify(value));
    }
    if (Object.prototype.hasOwnProperty.call(a, 'atLeast')) {
      if (typeof value !== 'number') reasons.push('assert ' + a.path + ' is not a number: ' + JSON.stringify(value));
      else if (value < a.atLeast) reasons.push('assert ' + a.path + ' expected >= ' + a.atLeast + ', got ' + value);
    }
    if (a.present && (value === undefined || value === null)) reasons.push('assert ' + a.path + ' is absent');
    if (a.endsWith && (typeof value !== 'string' || !value.endsWith(a.endsWith))) reasons.push('assert ' + a.path + ' does not end with ' + a.endsWith + ': ' + JSON.stringify(value));
    if (a.startsWith && (typeof value !== 'string' || !value.startsWith(a.startsWith))) reasons.push('assert ' + a.path + ' does not start with ' + a.startsWith);
    if (a.path === 'stdoutStartsWith' && !stdout.replace(/\r/g, '').startsWith(a.equals)) reasons.push('stdout does not start with ' + JSON.stringify(a.equals) + ': ' + JSON.stringify(stdout.slice(0, 80)));
    if (a.path === 'stdoutContains' && !stdout.includes(a.equals)) reasons.push('stdout does not contain ' + JSON.stringify(a.equals));
    if (a.path === 'stderrContains' && !stderr.includes(a.equals)) reasons.push('stderr does not contain ' + JSON.stringify(a.equals) + ': ' + JSON.stringify(stderr.slice(0, 120)));
    if (a.path === 'outputContains' && !(stdout + stderr).toLowerCase().includes(String(a.equals).toLowerCase())) reasons.push('stdout+stderr do not contain ' + JSON.stringify(a.equals) + ' (case-insensitive)');
  }
  return reasons;
}

const failures = [];
const rows = (matrix.rows || []).filter(r => !filter || r.id.includes(filter));
for (const id of required) if (!rows.some(r => r.id === id)) failures.push('missing matrix row ' + id);

const results = [];
for (const row of rows) {
  // A workspace-local directory, not os.tmpdir(): the sandbox denies the Rust child writes under
  // %TEMP%, which would make RUN-012 fail for an environment reason rather than a parity one.
  const tmpRoot = path.join(root, 'parity', '.tmp', 'cli');
  fs.mkdirSync(tmpRoot, { recursive: true });
  const tmp = fs.mkdtempSync(path.join(tmpRoot, row.id + '-'));
  for (const f of row.files || []) {
    const target = path.join(tmp, f.name);
    fs.mkdirSync(path.dirname(target), { recursive: true });
    fs.writeFileSync(target, f.content);
  }
  const reasons = [];
  const o = run(oracle, row, tmp);
  const r = run({ command: rustBin, args: [] }, row, tmp);
  if (o.error) reasons.push('oracle spawn failed: ' + o.error.message);
  if (r.error) reasons.push('rust spawn failed: ' + r.error.message);

  let oEnvelope = null, rEnvelope = null;
  if (row.compare === 'json') {
    try { oEnvelope = JSON.parse(o.stdout); } catch { reasons.push('oracle stdout is not JSON'); }
    try { rEnvelope = JSON.parse(r.stdout); } catch { reasons.push('rust stdout is not JSON'); }
  }
  // A defective row is a gate failure in its own right.
  const oracleAsserts = checkAsserts(row, oEnvelope, o.exitCode, o.stdout, o.stderr);
  for (const a of oracleAsserts) reasons.push('DEFECTIVE ROW (oracle): ' + a);

  const rustReasons = checkAsserts(row, rEnvelope, r.exitCode, r.stdout, r.stderr);
  for (const a of rustReasons) reasons.push('rust: ' + a);

  if (o.exitCode !== r.exitCode) reasons.push('exit ' + o.exitCode + ' != ' + r.exitCode);

  if (row.compare === 'json' && oEnvelope && rEnvelope) {
    const oo = applyPatterns(normalise(JSON.parse(JSON.stringify(oEnvelope)), row.ignorePaths || []), row.normalizePatterns);
    const rr = applyPatterns(normalise(JSON.parse(JSON.stringify(rEnvelope)), row.ignorePaths || []), row.normalizePatterns);
    for (const p of row.ignorePaths || []) {
      if (!setPath(oo, p, '<ignored>')) reasons.push('ignore path missing on oracle: ' + p);
      if (!setPath(rr, p, '<ignored>')) reasons.push('ignore path missing on rust: ' + p);
    }
    same(oo, rr, '$', reasons);
  } else if (row.compare === 'stdout') {
    const a = o.stdout.replace(/\r\n/g, '\n').trimEnd();
    const b = r.stdout.replace(/\r\n/g, '\n').trimEnd();
    if (a !== b) reasons.push('stdout differs: ' + JSON.stringify(a.slice(0, 120)) + ' != ' + JSON.stringify(b.slice(0, 120)));
  }
  results.push({ id: row.id, pass: reasons.length === 0, reasons });
}

const passed = results.filter(r => r.pass).length;
console.log('CLI matrix: ' + passed + '/' + results.length);
for (const r of results) {
  if (!r.pass) {
    console.log('  FAIL ' + r.id);
    for (const reason of r.reasons.slice(0, 6)) console.log('    ' + reason.slice(0, 300));
  }
}
if (failures.length) { console.error(failures.join('\n')); process.exit(2); }
if (passed !== results.length) { console.error('FAIL: ' + (results.length - passed) + ' CLI row(s) do not match'); process.exit(1); }
console.log('PASS: every RUN-001..RUN-014 row behaves like the oracle');
