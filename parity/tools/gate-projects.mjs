#!/usr/bin/env node
// ECO-001/ECO-002 gate: every C# production project in the contract has exactly one disposition,
// and every non-absent disposition points at a Rust path that exists.
import fs from 'node:fs';
import path from 'node:path';
import { parseToml } from './toml.mjs';

const root = path.resolve(import.meta.dirname, '..', '..');
const contract = fs.readFileSync(path.join(root, 'docs', 'parity', 'contract-v2.md'), 'utf8');
const projects = parseToml(fs.readFileSync(path.join(root, 'parity', 'projects.toml'), 'utf8'));

// The contract's section 4 table is the authority for which projects exist.
const contractNames = new Set();
for (const line of contract.split(/\r?\n/)) {
  const m = line.match(/^\|\s*(Lovelace\.[A-Za-z.]+)\s*\|/);
  if (m) contractNames.add(m[1]);
}
const failures = [];
if (contractNames.size !== 19) failures.push('expected 19 C# production projects in the contract, found ' + contractNames.size);

const rows = projects.project || [];
const seen = new Set();
const VALID = new Set(['port', 'consolidated', 'partial', 'absent']);
const BY_STATUS = { met: new Set(), partial: new Set(), absent: new Set() };
for (const p of rows) {
  if (seen.has(p.name)) failures.push('duplicate project row: ' + p.name);
  seen.add(p.name);
  if (!contractNames.has(p.name)) failures.push('project not in the contract: ' + p.name);
  if (!VALID.has(p.status)) failures.push(p.name + ': invalid status ' + JSON.stringify(p.status));
  const rust = p.rust || [];
  if (p.status === 'absent') {
    if (!p.reason) failures.push(p.name + ': absent without a reason');
    if (!p.phase) failures.push(p.name + ': absent without a target phase');
    if (rust.length) failures.push(p.name + ': absent but lists rust paths');
  } else {
    if (!rust.length) failures.push(p.name + ': ' + p.status + ' with no rust path');
    for (const rel of rust) {
      if (!fs.existsSync(path.join(root, rel))) failures.push(p.name + ': rust path does not exist: ' + rel);
    }
    if (!p.note) failures.push(p.name + ': ' + p.status + ' without a disposition note');
  }
}
for (const name of contractNames) if (!seen.has(name)) failures.push('missing disposition for ' + name);

const counts = { port: 0, consolidated: 0, partial: 0, absent: 0 };
for (const p of rows) counts[p.status]++;
console.log(JSON.stringify({ projects: rows.length, contractProjects: contractNames.size, counts, failures }, null, 2));
if (failures.length) { console.error('\nFAIL: ' + failures.length + ' issue(s)'); process.exit(1); }
console.log('PASS: ' + rows.length + ' C# projects dispositioned (' + counts.port + ' port, ' + counts.consolidated + ' consolidated, ' + counts.partial + ' partial, ' + counts.absent + ' absent).');
