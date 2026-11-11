#!/usr/bin/env node
// Generates parity/requirements.json from docs/parity/contract-v2.md.
//
// The contract document is the source of truth for WHICH requirements exist.
// This generator preserves the curated status/owner/evidence of rows that still
// exist, so regenerating after a contract edit neither drops a requirement nor
// resets progress. Run with --check to fail when the ledger and the contract
// disagree without rewriting anything.
import fs from 'node:fs';
import path from 'node:path';

const root = path.resolve(import.meta.dirname, '..');
const contractPath = path.join(root, 'docs', 'parity', 'contract-v2.md');
const ledgerPath = path.join(root, 'parity', 'requirements.json');

const FAMILIES = ['REQ','RUN','WIRE','ECO','NUM','REAL','CPLX','ARRAY','LANG','ENG','PLUG','SYM','MIR','DSP','STAT','CLI','KNOW','DESK','TEST','DIFF','PROOF','GATE','PERF','RUST','HARNESS'];
const ID_RE = new RegExp('^\\s*[-|]\\s*((?:' + FAMILIES.join('|') + ')-\\d{3})\\b\\s*(.*)$');

function cleanTitle(rest) {
  let s = rest.trim();
  if (s.startsWith('|')) {
    const cells = s.split('|').map(x => x.trim()).filter(x => x.length > 0);
    s = cells.join(' — ');
  }
  s = s.replace(/^[—:\-]\s*/, '').trim();
  return s.replace(/\s+/g, ' ');
}

const text = fs.readFileSync(contractPath, 'utf8');
const lines = text.split(/\r?\n/);
const found = new Map();
lines.forEach((line, i) => {
  const m = line.match(ID_RE);
  if (!m) return;
  const id = m[1];
  const title = cleanTitle(m[2]);
  if (!found.has(id)) found.set(id, { id, title, sourceLine: i + 1 });
});

// Duplicate detection: an id mentioned twice with different text is a contract defect.
const seen = new Map();
lines.forEach((line, i) => {
  const m = line.match(ID_RE);
  if (!m) return;
  const prior = seen.get(m[1]);
  seen.set(m[1], (prior || 0) + 1);
});

let previous = { rows: [] };
if (fs.existsSync(ledgerPath)) {
  try { previous = JSON.parse(fs.readFileSync(ledgerPath, 'utf8')); } catch { previous = { rows: [] }; }
}
const prevById = new Map((previous.rows || []).map(r => [r.id, r]));

// Hoisted so the merge below can use them: every requirement gets an owner and a target phase from
// its family, so "deferred" is never an unnamed absence. The phases are the requester's §15 order.
function phaseFor(family) {
  const PHASE = { REQ: 0, ECO: 0, TEST: 0, HARNESS: 0, RUN: 1, WIRE: 1, DIFF: 2, GATE: 2,
    NUM: 3, REAL: 3, CPLX: 3, ARRAY: 3, LANG: 3, ENG: 3, PLUG: 3, SYM: 4, MIR: 4,
    DSP: 5, STAT: 5, CLI: 5, KNOW: 5, DESK: 6, PROOF: 7, PERF: 7, RUST: 7 };
  return PHASE[family] ?? 0;
}
function ownerFor(family) {
  const CYCLE = new Set(['REQ', 'RUN', 'WIRE', 'ECO', 'TEST', 'DIFF', 'GATE', 'HARNESS']);
  return CYCLE.has(family) ? 'dsh-cycle-8' : 'phase-' + phaseFor(family);
}

const rows = [...found.values()].sort((a, b) => a.id.localeCompare(b.id, 'en'));
const merged = rows.map(r => {
  const prev = prevById.get(r.id);
  return {
    id: r.id,
    family: r.id.split('-')[0],
    title: r.title,
    sourceLine: r.sourceLine,
    status: prev?.status ?? 'unmet',
    owner: (prev?.owner && prev.owner !== 'unassigned') ? prev.owner : ownerFor(r.id.split('-')[0]),
    phase: prev?.phase !== undefined ? prev.phase : phaseFor(r.id.split('-')[0]),
    evidence: prev?.evidence ?? [],
    note: prev?.note ?? '',
  };
});

// Obligations the specification states without an ID (REQ-005: they must still be visible).
const OBLIGATIONS = [
  { id: 'OBL-001', text: '214/214 documentation corpus', phase: 4, owner: 'phase-4' },
  { id: 'OBL-002', text: '100,000-case nightly differential run', phase: 2, owner: 'phase-2' },
  { id: 'OBL-003', text: 'the twelve property families of §12.4', phase: 3, owner: 'phase-3' },
  { id: 'OBL-004', text: 'Lovelace.Run.Tests integration tests reproduced or matched', phase: 1, owner: 'dsh-cycle-8' },
  { id: 'OBL-005', text: 'docs/symbolics/dsh-protocol.md normative alongside the fixtures', phase: 1, owner: 'dsh-cycle-8' },
  { id: 'OBL-006', text: 'desktop platform decision: Windows-only vs Windows/Linux/macOS', phase: 6, owner: 'user' },
  { id: 'OBL-007', text: 'zero-dependency strategy as an explicit engineering decision', phase: 7, owner: 'phase-7' },
];
const prevObligations = new Map((previous.obligations || []).map(o => [o.id, o]));
const obligations = OBLIGATIONS.map(o => {
  const prev = prevObligations.get(o.id) || {};
  return { id: o.id, text: o.text, phase: o.phase, owner: o.owner, status: prev.status || 'unmet', evidence: prev.evidence || [], note: prev.note || '' };
});

const out = {
  schemaVersion: 1,
  contract: 'docs/parity/contract-v2.md',
  generatedBy: 'parity/gen-requirements.mjs',
  totals: {
    requirements: merged.length,
    byFamily: Object.fromEntries(FAMILIES.map(f => [f, merged.filter(r => r.family === f).length]).filter(([, n]) => n > 0)),
    byStatus: Object.fromEntries(['met','partial','unmet','excluded'].map(s => [s, merged.filter(r => r.status === s).length])),
  },
  duplicateMentions: [...seen.entries()].filter(([, n]) => n > 1).map(([id, n]) => ({ id, mentions: n })),
  obligations,
  rows: merged,
};

if (process.argv.includes('--check')) {
  const same = JSON.stringify(previous.rows?.map(r => r.id)) === JSON.stringify(merged.map(r => r.id));
  console.log(JSON.stringify({ ok: same, previous: previous.rows?.length ?? 0, generated: merged.length }, null, 2));
  process.exit(same ? 0 : 1);
}

fs.writeFileSync(ledgerPath, JSON.stringify(out, null, 1) + '\n');
const missing = [...prevById.keys()].filter(id => !found.has(id));
console.log(JSON.stringify({ requirements: merged.length, byFamily: out.totals.byFamily, byStatus: out.totals.byStatus, droppedSincePrevious: missing }, null, 2));
