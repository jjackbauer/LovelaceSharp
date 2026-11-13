#!/usr/bin/env node
// ECO-008 format validator.
//
// Two claims are checked against persisted data, not prose:
//
//   1. The knowledge-graph schema emitted by ecosystem-inventory.mjs is the schema the checked-in
//      example actually follows: version 1, the exact top-level field set, the exact config field
//      set, and a stable field set for every element of samples/planes/boundaries/frontiers.
//   2. Every additional --graph <path> validates against the same schema, so an existing graph file
//      remains readable by a reader that implements it.
//
// Exit codes: 0 every checked file conforms; 1 a file does not conform; 2 the schema or a file is
// missing/unparseable (an unverifiable check is a failure, never a pass).
//
// Usage:
//   node parity/tools/format-check.mjs                     # validate the checked-in example
//   node parity/tools/format-check.mjs --check             # same, fail-closed (CI)
//   node parity/tools/format-check.mjs --graph path.json   # validate one more graph file
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';

const root = path.resolve(import.meta.dirname, '..', '..');
const schemaPath = path.join(root, 'docs', 'parity', 'knowledge-graph-v1.schema.json');
const examplePath = path.join(root, 'knowledge-graph-demo.json');
const failures = [];

if (!fs.existsSync(schemaPath)) {
  console.error('format-check: missing ' + path.relative(root, schemaPath));
  process.exit(2);
}
const schema = JSON.parse(fs.readFileSync(schemaPath, 'utf8'));

const extra = [];
const argv = process.argv.slice(2);
for (let i = 0; i < argv.length; i++) if (argv[i] === '--graph' && argv[i + 1]) extra.push(argv[++i]);

function keySet(values) { return [...new Set(values)].sort(); }
function sha256(buf) { return crypto.createHash('sha256').update(buf).digest('hex'); }

function validateGraphFile(abs, label) {
  if (!fs.existsSync(abs)) {
    failures.push(label + ': file is missing');
    return null;
  }
  const bytes = fs.readFileSync(abs);
  let graph;
  try { graph = JSON.parse(bytes.toString('utf8')); } catch (e) {
    failures.push(label + ': not JSON: ' + String(e).slice(0, 160));
    return null;
  }
  const keys = Object.keys(graph).sort();
  const required = [...schema.requiredTopLevel].sort();
  for (const k of required) if (!keys.includes(k)) failures.push(label + ': missing top-level field ' + k);
  for (const k of keys) if (!required.includes(k)) failures.push(label + ': unknown top-level field ' + k);

  const checkConst = (field, expected) => {
    if (!(field in graph)) return;
    if (graph[field] !== expected) failures.push(label + ': ' + field + ' is ' + JSON.stringify(graph[field]) + ', expected ' + JSON.stringify(expected));
  };
  checkConst('version', 1);
  if (typeof graph.seed !== 'number' || !Number.isInteger(graph.seed)) failures.push(label + ': seed is not an integer');

  const config = graph.config;
  if (!config || typeof config !== 'object' || Array.isArray(config)) {
    failures.push(label + ': config is not an object');
  } else {
    const configKeys = Object.keys(config).sort();
    const expected = [...schema.properties.config.requiredKeys].sort();
    for (const k of expected) if (!configKeys.includes(k)) failures.push(label + ': config is missing ' + k);
    for (const k of configKeys) if (!expected.includes(k)) failures.push(label + ': config has unknown field ' + k);
  }

  const checkElements = (field, requiredElementKeys) => {
    const arr = graph[field];
    if (!Array.isArray(arr)) { failures.push(label + ': ' + field + ' is not an array'); return; }
    const expected = [...requiredElementKeys].sort();
    arr.forEach((item, index) => {
      if (!item || typeof item !== 'object' || Array.isArray(item)) { failures.push(label + ': ' + field + '[' + index + '] is not an object'); return; }
      const got = Object.keys(item).sort();
      const missing = expected.filter(k => !got.includes(k));
      const unknown = got.filter(k => !expected.includes(k));
      if (missing.length) failures.push(label + ': ' + field + '[' + index + '] is missing ' + missing.join(','));
      if (unknown.length) failures.push(label + ': ' + field + '[' + index + '] has unknown ' + unknown.join(','));
    });
  };
  checkElements('samples', schema.properties.samples.requiredElementKeys);
  checkElements('planes', schema.properties.planes.requiredElementKeys);
  checkElements('boundaries', schema.properties.boundaries.requiredElementKeys);
  checkElements('frontiers', schema.properties.frontiers.requiredElementKeys);

  const metrics = graph.metrics;
  if (metrics !== null && metrics !== undefined) {
    if (typeof metrics !== 'object' || Array.isArray(metrics)) failures.push(label + ': metrics is not an object');
    else {
      const got = Object.keys(metrics).sort();
      const expected = [...schema.properties.metrics.requiredKeys].sort();
      for (const k of expected) if (!got.includes(k)) failures.push(label + ': metrics is missing ' + k);
      for (const k of got) if (!expected.includes(k)) failures.push(label + ': metrics has unknown field ' + k);
    }
  }

  return {
    label,
    sha256: sha256(bytes),
    bytes: bytes.length,
    version: graph.version,
    seed: graph.seed,
    counts: {
      samples: (graph.samples || []).length,
      planes: (graph.planes || []).length,
      boundaries: (graph.boundaries || []).length,
      frontiers: (graph.frontiers || []).length,
      configKeys: config ? Object.keys(config).length : 0,
    },
  };
}

const results = [];
results.push(validateGraphFile(examplePath, 'knowledge-graph-demo.json'));
for (const rel of extra) results.push(validateGraphFile(path.resolve(root, rel), rel));

const summary = {
  schema: path.relative(root, schemaPath).split(path.sep).join('/'),
  version: schema.properties.version ? schema.properties.version.const : null,
  checked: results.filter(Boolean),
  failures,
};
console.log(JSON.stringify(summary, null, 2));
if (failures.length) { console.error('\nFAIL: ' + failures.length + ' format issue(s)'); process.exit(1); }
console.log('PASS: ' + results.filter(Boolean).length + ' graph file(s) conform to knowledge-graph v1');
