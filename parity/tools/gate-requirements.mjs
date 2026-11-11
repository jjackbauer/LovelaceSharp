#!/usr/bin/env node
// Verifies that parity/requirements.json is a complete, honest ledger over every requirement ID in
// docs/parity/contract-v2.md. Fails closed: a requirement that disappeared, a "met" row with no
// evidence, or an exclusion without a user-approved owner is a failure.
import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';

const root = path.resolve(import.meta.dirname, '..', '..');
const contractPath = path.join(root, 'docs', 'parity', 'contract-v2.md');
const ledgerPath = path.join(root, 'parity', 'requirements.json');
const VALID_STATUS = new Set(['met', 'partial', 'unmet', 'excluded']);
const failures = [];

if (!fs.existsSync(contractPath)) failures.push('missing contract: docs/parity/contract-v2.md');
if (!fs.existsSync(ledgerPath)) failures.push('missing ledger: parity/requirements.json');
if (failures.length) { console.error(failures.join('\n')); process.exit(2); }

const ledger = JSON.parse(fs.readFileSync(ledgerPath, 'utf8'));
const rows = ledger.rows || [];
const byId = new Map(rows.map(r => [r.id, r]));

// 1. Contract drift: regenerate in check mode and require the same ID set.
const gen = spawnSync(process.execPath, [path.join(root, 'parity', 'gen-requirements.mjs'), '--check'], { encoding: 'utf8' });
if (gen.status !== 0) {
  failures.push('ledger is not in sync with the contract (run: node parity/gen-requirements.mjs)');
  if (gen.stdout) console.error(gen.stdout);
}

// 2. No duplicate ids.
const seen = new Set();
for (const r of rows) {
  if (seen.has(r.id)) failures.push('duplicate ledger row: ' + r.id);
  seen.add(r.id);
  if (!VALID_STATUS.has(r.status)) failures.push(r.id + ': invalid status ' + JSON.stringify(r.status));
  if (r.status === 'met' && (!Array.isArray(r.evidence) || r.evidence.length === 0))
    failures.push(r.id + ': status met with no evidence');
  if (r.status === 'partial' && (!Array.isArray(r.evidence) || r.evidence.length === 0))
    failures.push(r.id + ': status partial with no evidence');
  if (r.status === 'excluded') {
    if (!r.note) failures.push(r.id + ': excluded without a reason');
    if (r.owner !== 'user') failures.push(r.id + ': excluded without user approval (owner=' + r.owner + ')');
  }
  if (!r.title || r.title.trim().length === 0) failures.push(r.id + ': empty title');
}

// 3. Every family in the contract is represented.
const families = new Set(rows.map(r => r.family));
if (families.size < 25) failures.push('expected at least 25 requirement families, found ' + families.size);

const obligations = ledger.obligations || [];
if (obligations.length < 5) failures.push('expected the ID-less obligations (property families, nightly run, corpus, ...) in the ledger');
for (const o of obligations) {
  if (!VALID_STATUS.has(o.status)) failures.push(o.id + ': invalid status ' + JSON.stringify(o.status));
  if (!o.owner) failures.push(o.id + ': no owner');
  if (!o.phase && o.phase !== 0) failures.push(o.id + ': no target phase');
  if (o.status === 'met' && (!o.evidence || !o.evidence.length)) failures.push(o.id + ': met with no evidence');
}

const counts = {};
for (const s of ['met', 'partial', 'unmet', 'excluded']) counts[s] = rows.filter(r => r.status === s).length;
const summary = { requirements: rows.length, obligations: obligations.length, counts, families: families.size, failures };
console.log(JSON.stringify(summary, null, 2));
if (failures.length) { console.error('\nFAIL: ' + failures.length + ' issue(s)'); process.exit(1); }
console.log('PASS: ' + rows.length + ' requirements accounted for; ' + counts.met + ' met, ' + counts.partial + ' partial, ' + counts.unmet + ' unmet.');
