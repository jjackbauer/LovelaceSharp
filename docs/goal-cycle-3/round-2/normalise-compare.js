#!/usr/bin/env node
// STEP 3 regression proof for the Lovelace.Run envelope extraction.
//
//   node docs/goal-cycle-3/round-2/normalise-compare.js <before.json> <after.json>
//
// Parses both envelopes as JSON, replaces the value of every volatile key
// (revision, elapsed, elapsedTime, timings — at any depth) with the literal
// string "<volatile>", and asserts the resulting trees are deep-equal. Exits 0
// and prints "IDENTICAL" on success, 1 with a JSON-path diff otherwise.
'use strict';
const fs = require('fs');

const VOLATILE = new Set(['revision', 'elapsed', 'elapsedTime', 'timings']);

function normalise(value) {
  if (Array.isArray(value)) return value.map(normalise);
  if (value && typeof value === 'object') {
    const out = {};
    for (const key of Object.keys(value)) {
      out[key] = VOLATILE.has(key) ? '<volatile>' : normalise(value[key]);
    }
    return out;
  }
  return value;
}

// Structural comparison that reports the first differing JSON path; key order is
// irrelevant on both sides (keys are compared as sets).
function diff(a, b, path) {
  const pa = path || '$';
  const ta = a === null ? 'null' : Array.isArray(a) ? 'array' : typeof a;
  const tb = b === null ? 'null' : Array.isArray(b) ? 'array' : typeof b;
  if (ta !== tb) return pa + ': type ' + ta + ' vs ' + tb;
  if (ta === 'array') {
    if (a.length !== b.length) return pa + ': length ' + a.length + ' vs ' + b.length;
    for (let i = 0; i < a.length; i++) {
      const d = diff(a[i], b[i], pa + '[' + i + ']');
      if (d) return d;
    }
    return null;
  }
  if (ta === 'object') {
    const ka = Object.keys(a).sort();
    const kb = Object.keys(b).sort();
    if (ka.join('\u0000') !== kb.join('\u0000')) {
      return pa + ': keys [' + ka.join(',') + '] vs [' + kb.join(',') + ']';
    }
    for (const k of ka) {
      const d = diff(a[k], b[k], pa + '.' + k);
      if (d) return d;
    }
    return null;
  }
  return a === b ? null : pa + ': ' + JSON.stringify(a) + ' vs ' + JSON.stringify(b);
}

const before = normalise(JSON.parse(fs.readFileSync(process.argv[2], 'utf8')));
const after = normalise(JSON.parse(fs.readFileSync(process.argv[3], 'utf8')));
const d = diff(before, after, '$');
if (d) {
  console.log('DIFFERENT: ' + d);
  process.exit(1);
}
console.log('IDENTICAL — normalised documents are deep-equal (' +
  JSON.stringify(before).length + ' chars normalised)');
