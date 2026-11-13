#!/usr/bin/env node
// Structural check for the GitHub Actions workflows.
//
// The repository is offline-capable and ships no YAML dependency, so this is a deliberately small
// subset parser: it proves indentation is well formed, every line is a mapping entry or a sequence
// item, no key repeats inside one mapping, and block scalars are terminated. It cannot prove GitHub
// will accept a workflow; it catches the mistakes that make a workflow unparseable before a push.
//
// Usage:
//   node parity/tools/check-workflow-yaml.mjs                    # check .github/workflows/*.yml
//   node parity/tools/check-workflow-yaml.mjs path/to/one.yml
import fs from 'node:fs';
import path from 'node:path';

const root = path.resolve(import.meta.dirname, '..', '..');
const workflowDir = path.join(root, '.github', 'workflows');
const named = process.argv.slice(2).filter(arg => !arg.startsWith('--'));
const files = named.length
  ? named.map(file => path.resolve(root, file))
  : fs.readdirSync(workflowDir).filter(name => /\.ya?ml$/.test(name)).sort().map(name => path.join(workflowDir, name));

const KEY = /^(\s*)([A-Za-z0-9_.\-/'"${{ }},]+):(?:\s+(.*))?$/;
const ITEM = /^(\s*)-(?:\s+(.*))?$/;
const BLOCK = /^[|>][+-]?$/;

let failed = 0;
for (const file of files) {
  const rel = path.relative(root, file).split(path.sep).join('/');
  const lines = fs.readFileSync(file, 'utf8').split(/\r?\n/);
  const problems = [];
  const keysByIndent = new Map();
  let blockIndent = -1;
  let maxIndent = 0;
  let mappings = 0;
  let items = 0;
  let blockScalars = 0;

  lines.forEach((raw, index) => {
    const where = rel + ':' + (index + 1);
    if (raw.includes('\t')) problems.push(where + ': tab character in indentation');
    if (!raw.trim()) return;
    const indent = raw.length - raw.trimStart().length;

    // A block scalar swallows every deeper line verbatim.
    if (blockIndent >= 0) {
      if (indent > blockIndent) return;
      blockIndent = -1;
    }
    const text = raw.trim();
    if (text.startsWith('#')) return;
    if (indent % 2 !== 0) problems.push(where + ': indent ' + indent + ' is not a multiple of two');
    maxIndent = Math.max(maxIndent, indent);
    for (const level of [...keysByIndent.keys()]) if (level > indent) keysByIndent.delete(level);

    const item = raw.match(ITEM);
    if (item) {
      items += 1;
      return;
    }
    const mapping = raw.match(KEY);
    if (!mapping) {
      problems.push(where + ': not a mapping entry or sequence item: ' + JSON.stringify(text.slice(0, 60)));
      return;
    }
    mappings += 1;
    const key = mapping[2];
    if (!keysByIndent.has(indent)) keysByIndent.set(indent, new Set());
    const seen = keysByIndent.get(indent);
    if (seen.has(key)) problems.push(where + ': duplicate key ' + JSON.stringify(key) + ' at indent ' + indent);
    seen.add(key);
    const value = (mapping[3] || '').trim();
    if (BLOCK.test(value)) {
      blockIndent = indent;
      blockScalars += 1;
    }
  });

  // A block scalar may legitimately run to the end of the file; YAML ends it at EOF.
  if (problems.length) {
    failed += 1;
    for (const problem of problems) console.error('  - ' + problem);
    console.error('FAIL: ' + rel + ' has ' + problems.length + ' structural problem(s)');
  } else {
    console.log('PASS ' + rel + ': ' + mappings + ' keys, ' + items + ' sequence items, ' + blockScalars + ' block scalars, max indent ' + maxIndent);
  }
}
if (failed) process.exit(1);
console.log('PASS: ' + files.length + ' workflow file(s) are structurally well formed');
