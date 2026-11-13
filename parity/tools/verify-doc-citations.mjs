#!/usr/bin/env node
// PROOF-002/PROOF-003 citation check.
//
// The proof documents cite implementation paths and test names. A citation that points at a file
// that no longer exists, or at a test that was renamed away, is a document that lies about its own
// evidence. This verifier resolves every citation against the tree:
//
//   * markdown links resolve relative to the document (no dangling ../../ paths);
//   * backticked repository paths (rust/..., parity/..., .github/..., Lovelace...) exist;
//   * every test cited in a mapping table's Test column, and every proof_* name in the prose,
//     resolves to a fn <name> in the Rust tree or a theorem <name> in the Lean tree.
//
// Usage:
//   node parity/tools/verify-doc-citations.mjs            # check the default document set
//   node parity/tools/verify-doc-citations.mjs a.md b.md  # check the named documents
import fs from 'node:fs';
import path from 'node:path';

const root = path.resolve(import.meta.dirname, '..', '..');
const defaults = [
  'docs/parity/proof-theorem-map.md',
  'docs/parity/numeric-representation.md',
  'docs/parity/release-targets.md',
  'docs/parity/oracle-artifacts.md',
];
const named = process.argv.slice(2).filter(arg => !arg.startsWith('--'));
const targets = (named.length ? named : defaults).map(t => t.split(path.sep).join('/'));

function walk(dir, suffixes, out) {
  if (!fs.existsSync(dir)) return out;
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    if (entry.name === 'target' || entry.name === 'node_modules' || entry.name === '.git' || entry.name === '.lake') continue;
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) walk(full, suffixes, out);
    else if (suffixes.some(suffix => entry.name.endsWith(suffix))) out.push(full);
  }
  return out;
}

// Read the Rust and Lean sources once: the name check is then a lookup, not a grep per token.
const rustSources = walk(path.join(root, 'rust'), ['.rs'], [])
  .map(file => ({ file: path.relative(root, file).split(path.sep).join('/'), text: fs.readFileSync(file, 'utf8') }));
const leanSources = walk(path.join(root, 'Lovelace.Proofs'), ['.lean'], [])
  .map(file => ({ file: path.relative(root, file).split(path.sep).join('/'), text: fs.readFileSync(file, 'utf8') }));

// Citation names are snake_case identifiers, so no regex metacharacter escaping is needed; anything
// else is not a test citation and is skipped rather than guessed at.
const NAME = /^[a-z][a-z0-9_]*$/;

function resolveRustTest(name) {
  const anywhere = new RegExp('fn\\s+' + name + '(?![A-Za-z0-9_])');
  const declaration = new RegExp('fn\\s+' + name + '\\s*\\(');
  for (const source of rustSources) {
    if (!anywhere.test(source.text)) continue;
    return { file: source.file, exact: declaration.test(source.text) };
  }
  return null;
}

function resolveLeanTheorem(name) {
  const pattern = new RegExp('theorem\\s+' + name + '(?![A-Za-z0-9_])');
  for (const source of leanSources) if (pattern.test(source.text)) return { file: source.file };
  return null;
}

const failures = [];
const checked = { documents: 0, links: 0, paths: 0, testCitations: 0 };

function isExternal(link) {
  return /^[a-z]+:/i.test(link) || link.startsWith('#') || link.startsWith('mailto:');
}

function checkDocument(rel) {
  const file = path.join(root, rel);
  if (!fs.existsSync(file)) {
    failures.push(rel + ': document is missing');
    return;
  }
  checked.documents += 1;
  const lines = fs.readFileSync(file, 'utf8').split(/\r?\n/);
  lines.forEach((line, index) => {
    const where = rel + ':' + (index + 1);

    // 1. Markdown links resolve relative to the document.
    for (const match of line.matchAll(/\[[^\]]*\]\(([^)\s]+)\)/g)) {
      const link = match[1];
      if (isExternal(link)) continue;
      const target = path.resolve(path.dirname(file), decodeURIComponent(link.split('#')[0]));
      checked.links += 1;
      if (!fs.existsSync(target)) failures.push(where + ': dangling link ' + link);
    }

    // 2. Backticked repository paths exist.
    for (const match of line.matchAll(/`([^`]+)`/g)) {
      const token = match[1].trim();
      if (!token.includes('/') || isExternal(token)) continue;
      if (!/\.(rs|lean|mjs|ps1|json|jsonl|yml|yaml|md|toml|cs|csproj|slnx|svg|txt|pdf|exe|dll|lock)$/.test(token)) continue;
      checked.paths += 1;
      if (!fs.existsSync(path.join(root, token))) failures.push(where + ': cited path does not exist: ' + token);
    }

    // 3. Test names: the Test column of a mapping table, plus every proof_* name in the prose.
    const isTableRow = /^\s*\|/.test(line) && (line.match(/\|/g) || []).length >= 5;
    const testCell = isTableRow ? (line.split('|')[4] || '') : '';
    const citations = new Set();
    for (const match of testCell.matchAll(/`([^`]+)`/g)) {
      for (const piece of match[1].split(/[,;]/)) {
        const name = piece.trim().replace(/^\(|\)$/g, '');
        const last = name.includes('::') ? name.split('::').pop() : name;
        if (NAME.test(last)) citations.add(last);
      }
    }
    for (const match of line.matchAll(/`(proof_[a-z0-9_]+)`/g)) citations.add(match[1]);

    for (const name of citations) {
      if (name.length < 6) continue;
      checked.testCitations += 1;
      const rust = resolveRustTest(name);
      if (rust) {
        if (!rust.exact) failures.push(where + ': ' + name + ' matches only as a name prefix in ' + rust.file);
        continue;
      }
      if (resolveLeanTheorem(name)) continue;
      failures.push(where + ': cited test ' + name + ' is not a fn in rust/**/*.rs nor a theorem in Lovelace.Proofs');
    }
  });
}

for (const rel of targets) checkDocument(rel);

console.log(JSON.stringify({ ...checked, failures: failures.length }, null, 2));
if (failures.length) {
  for (const failure of failures) console.error('  - ' + failure);
  console.error('FAIL: ' + failures.length + ' unresolvable citation(s)');
  process.exit(1);
}
console.log('PASS: every cited path, link and test resolves against the tree');
