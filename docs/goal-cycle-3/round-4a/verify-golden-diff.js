// Structural audit of the regenerated goldens (round-4a Step C).
//   node docs/goal-cycle-3/round-4a/verify-golden-diff.js
// Prints EVERY leaf difference between the pre-change fixtures and the regenerated ones, so a
// change nobody intended cannot hide behind the enum revision.
'use strict';
const fs = require('fs'), path = require('path');
const beforeDir = 'docs/goal-cycle-3/round-4a/fixtures-before';
const afterDir = 'Lovelace.Run.Tests/fixtures';

function walk(a, b, p, out) {
  if (a === undefined && b !== undefined) { out.push(p + ' ADDED = ' + JSON.stringify(b)); return; }
  if (b === undefined) { out.push(p + ' REMOVED (was ' + JSON.stringify(a) + ')'); return; }
  if (a && b && typeof a === 'object' && typeof b === 'object') {
    for (const k of new Set([...Object.keys(a), ...Object.keys(b)])) walk(a[k], b[k], p + '.' + k, out);
    return;
  }
  if (JSON.stringify(a) !== JSON.stringify(b)) out.push(p + ': ' + JSON.stringify(a) + ' -> ' + JSON.stringify(b));
}

let total = 0, changedFiles = [], kinds = {};
for (const name of fs.readdirSync(beforeDir).sort()) {
  const a = JSON.parse(fs.readFileSync(path.join(beforeDir, name), 'utf8'));
  const b = JSON.parse(fs.readFileSync(path.join(afterDir, name), 'utf8'));
  const out = [];
  walk(a, b, '$', out);
  if (out.length) {
    changedFiles.push(name);
    console.log('--- ' + name + ' (' + out.length + ' leaf changes) ---');
    out.forEach(d => console.log('  ' + d));
  }
  total += out.length;
}
console.log('files changed: ' + changedFiles.join(', '));
console.log('total leaf changes: ' + total);
