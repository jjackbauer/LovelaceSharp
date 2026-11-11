#!/usr/bin/env node
// Census gate (TEST-006, TEST-009): the census is complete, internally consistent, and reconciled
// with the frozen solution rather than with a hardcoded number.
import fs from 'node:fs';
import path from 'node:path';
import { readJson } from './read.mjs';

const root = path.resolve(import.meta.dirname, '..', '..');
const failures = [];
const censusPath = path.join(root, 'parity', 'census.json');
if (!fs.existsSync(censusPath)) { console.error('missing parity/census.json'); process.exit(2); }
const census = readJson(censusPath);

const testProjects = fs.readdirSync(root, { withFileTypes: true })
  .filter(d => d.isDirectory() && d.name.endsWith('.Tests'))
  .map(d => d.name)
  .sort();
const censusProjects = (census.projects || []).map(p => p.name).sort();

if (census.schemaVersion !== 1) failures.push('census.schemaVersion is not 1');
if (!census.oracle || !/^[0-9a-f]{40}$/.test(census.oracle.commit || '')) failures.push('census.oracle.commit is not a 40-hex SHA');
if (census.oracle.commit !== JSON.parse(fs.readFileSync(path.join(root, 'parity', 'oracle.json'), 'utf8').replace(/^\uFEFF/, '')).commit) failures.push('census oracle differs from parity/oracle.json');

for (const p of testProjects) if (!censusProjects.includes(p)) failures.push('test project missing from census: ' + p);
for (const p of censusProjects) if (!testProjects.includes(p)) failures.push('census lists a project that is not a *.Tests directory: ' + p);

const ids = new Set();
const perProject = {};
const kinds = {};
for (const c of census.cases || []) {
  if (ids.has(c.id)) failures.push('duplicate census id: ' + c.id);
  ids.add(c.id);
  perProject[c.project] = (perProject[c.project] || 0) + 1;
  kinds[c.dataKind] = (kinds[c.dataKind] || 0) + 1;
  if (!c.method || !c.class) failures.push('case without class/method: ' + c.id);
  if (c.dataKind === 'unavailable' && !c.dataUnavailable) failures.push('unavailable case without a reason: ' + c.id);
}
for (const p of census.projects || []) {
  if (p.cases !== perProject[p.name]) failures.push(p.name + ': projects[].cases ' + p.cases + ' != enumerated ' + (perProject[p.name] || 0));
}
const totals = census.totals || {};
if (totals.testCases !== (census.cases || []).length) failures.push('totals.testCases ' + totals.testCases + ' != cases.length ' + (census.cases || []).length);
if (totals.projects !== censusProjects.length) failures.push('totals.projects disagrees with the project list');

// The census's own totals must agree with what it actually emitted. A totals block that no longer
// describes its case list is a falsified claim, not a cosmetic difference.
const inline = kinds.inline || 0, member = kinds.member || 0, unavailable = kinds.unavailable || 0, fact = kinds.fact || 0;
if (totals.inlineDataRows !== inline) failures.push('totals.inlineDataRows ' + totals.inlineDataRows + ' != ' + inline + ' inline cases');
if (totals.memberDataRows !== member) failures.push('totals.memberDataRows ' + totals.memberDataRows + ' != ' + member + ' member cases');
if ((totals.dataUnavailable || 0) !== unavailable) failures.push('totals.dataUnavailable ' + totals.dataUnavailable + ' != ' + unavailable + ' unavailable cases');
if ((totals.factMethods || 0) !== fact) failures.push('totals.factMethods ' + totals.factMethods + ' != ' + fact + ' fact cases');
if ((totals.theoryMethods || 0) + (totals.factMethods || 0) !== totals.testMethods) failures.push('factMethods + theoryMethods != testMethods');

const sumCases = (census.projects || []).reduce((n, p) => n + p.cases, 0);
if (sumCases !== (census.cases || []).length) failures.push('sum(projects[].cases) != cases.length');

// Reconciliation with the specification's REFERENCE figure (2,087 test facts). The reference is a
// method count from an older baseline; the census counts cases. Both numbers are reported, and a
// material divergence must be explained rather than hidden.
const referenceFacts = 2087;
const methods = totals.testMethods || 0;
const summary = {
  oracleCommit: census.oracle.commit,
  projects: censusProjects.length,
  testMethods: methods,
  testCases: (census.cases || []).length,
  dataKinds: kinds,
  referenceFacts,
  methodsVsReference: methods - referenceFacts,
  casesPerMethod: methods ? +((census.cases || []).length / methods).toFixed(2) : 0,
  unavailable: (census.cases || []).filter(c => c.dataKind === 'unavailable').map(c => c.id),
  failures,
};
console.log(JSON.stringify(summary, null, 2));
if (failures.length) { console.error('\nFAIL: ' + failures.length + ' issue(s)'); process.exit(1); }
console.log('PASS: ' + summary.testMethods + ' methods / ' + summary.testCases + ' cases across ' + summary.projects + ' projects (' + summary.methodsVsReference + ' methods vs the 2,087 reference).');
