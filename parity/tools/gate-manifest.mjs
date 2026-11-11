#!/usr/bin/env node
// Manifest gate (TEST-007..TEST-010): every census case has exactly one honest disposition, every
// claimed Rust test exists in the measured inventory, and no exclusion lacks a user-approved reason.
import fs from 'node:fs';
import path from 'node:path';
import { parseToml } from './toml.mjs';
import { readJson } from './read.mjs';

const root = path.resolve(import.meta.dirname, '..', '..');
const failures = [];
const read = (p, what) => {
  if (!fs.existsSync(p)) { failures.push('missing ' + what + ': ' + p); return null; }
  return fs.readFileSync(p, 'utf8');
};

const censusText = read(path.join(root, 'parity', 'census.json'), 'census');
const rustText = read(path.join(root, 'parity', 'rust-tests.json'), 'rust test inventory');
const manifestText = read(path.join(root, 'parity', 'manifest.toml'), 'manifest');
const projectsText = read(path.join(root, 'parity', 'projects.toml'), 'project ledger');
if (failures.length) { console.error(failures.join('\n')); process.exit(2); }

const census = readJson(path.join(root, 'parity', 'census.json'));
const rust = readJson(path.join(root, 'parity', 'rust-tests.json'));
const manifest = parseToml(manifestText);
const projects = parseToml(projectsText);

const rustIds = new Set();
for (const c of rust.crates || []) for (const t of c.tests) rustIds.add(c.name + '::' + t);

const censusIds = new Set(census.cases.map(c => c.id));
const rows = manifest.test || [];
const seen = new Set();
const VALID = new Set(['ported', 'consolidated', 'excluded', 'deferred']);
const counts = { ported: 0, consolidated: 0, excluded: 0, deferred: 0 };
const perProject = {};
let claimedRust = 0;

for (const row of rows) {
  if (seen.has(row.id)) failures.push('duplicate manifest row: ' + row.id);
  seen.add(row.id);
  if (!VALID.has(row.disposition)) failures.push(row.id + ': invalid disposition ' + JSON.stringify(row.disposition));
  counts[row.disposition] = (counts[row.disposition] || 0) + 1;
  perProject[row.project] = perProject[row.project] || { ported: 0, consolidated: 0, excluded: 0, deferred: 0 };
  perProject[row.project][row.disposition]++;
  if (row.disposition === 'ported' || row.disposition === 'consolidated') {
    if (!row.rust) failures.push(row.id + ': ' + row.disposition + ' with no rust test id');
    else if (!rustIds.has(row.rust)) failures.push(row.id + ': rust test does not exist: ' + row.rust);
    else claimedRust++;
    if (rust.suite && rust.suite.exitCode !== 0) failures.push(row.id + ': claims a Rust test but the measured suite exit is ' + rust.suite.exitCode);
  }
  if (row.disposition === 'excluded') {
    if (!row.reason) failures.push(row.id + ': excluded without a reason');
    if (row.owner !== 'user') failures.push(row.id + ': excluded without user approval');
  }
  if (row.disposition === 'deferred' && !row.reason) failures.push(row.id + ': deferred without a reason');
}
for (const id of censusIds) if (!seen.has(id)) failures.push('census case has no disposition: ' + id);
for (const id of seen) if (!censusIds.has(id)) failures.push('manifest row is not in the census: ' + id);

// The inventory must be fresh: a test deleted since the last listing would otherwise resolve as
// "exists" for ever (TEST-008: resolve to a test that actually ran).
const rustRoot = path.join(root, 'rust');
const walkSources = (function makeWalker() {
  const files = [];
  const walk = (dir) => {
    for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
      if (entry.name === 'target' || entry.name.startsWith('.')) continue;
      const full = path.join(dir, entry.name);
      if (entry.isDirectory()) walk(full);
      else if (entry.name.endsWith('.rs')) files.push(full);
    }
  };
  return () => { files.length = 0; walk(rustRoot); return files; };
})();
const crypto = await import('node:crypto');
const lines = walkSources()
  .map(f => path.relative(rustRoot, f).split(path.sep).join('/') + ':' + crypto.createHash('sha256').update(fs.readFileSync(f)).digest('hex'))
  .sort();
const sourcesSha256 = crypto.createHash('sha256').update(lines.join(String.fromCharCode(10)), 'utf8').digest('hex');
if (!rust.sourcesSha256) failures.push('rust-tests.json has no sourcesSha256: regenerate it with parity/gen-rust-tests.ps1');
else if (rust.sourcesSha256 !== sourcesSha256) failures.push('rust-tests.json is stale: the Rust sources changed since the inventory was measured (run parity/gen-rust-tests.ps1)');

if (manifest.oracle !== census.oracle.commit) failures.push('manifest oracle ' + manifest.oracle + ' != census oracle ' + census.oracle.commit);
if (projects.oracle !== census.oracle.commit) failures.push('project ledger oracle differs from census oracle');
if (census.totals && census.totals.testCases !== census.cases.length) failures.push('census totals.testCases disagrees with its own case list');

const total = rows.length;
const covered = counts.ported + counts.consolidated;
const summary = {
  censusCases: censusIds.size,
  manifestRows: total,
  dispositions: counts,
  coveragePercent: total ? +(100 * covered / total).toFixed(2) : 0,
  claimedRustTests: claimedRust,
  projectsInCensus: Object.keys(perProject).length,
  failures,
};
console.log(JSON.stringify(summary, null, 2));
if (failures.length) { console.error('\nFAIL: ' + failures.length + ' issue(s)'); process.exit(1); }
console.log('PASS (structural): ' + total + ' test cases dispositioned; ' + covered + ' claim Rust coverage (' + summary.coveragePercent + '%).');
