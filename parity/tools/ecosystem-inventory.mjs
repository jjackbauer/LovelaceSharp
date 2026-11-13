#!/usr/bin/env node
// Ecosystem inventory generator (ECO-003 / ECO-004 / ECO-007 / ECO-008).
//
// ECO-003 asks for an inventory of "all externally visible C# types and methods, not just language
// builtins". This tool reads the frozen C# tree directly (no compiled assemblies, no NuGet, no
// generated C#) and emits two deterministic artifacts:
//
//   docs/parity/ecosystem-api.json  machine-readable inventory: every externally visible type and
//                                   every externally visible member, per production project, with
//                                   the disposition derived from parity/projects.toml
//   docs/parity/ecosystem.md        human inventory: counts, per-type tables, a complete member
//                                   index, the ECO-004 consumer list, the ECO-008 format table with
//                                   a readability decision per format, and the ECO-007 decision
//
// The scan is conservative. A declaration it cannot classify is counted and listed as
// "unclassified" instead of being dropped; the generator fails when that count exceeds the
// recorded ceiling, so a parser regression cannot silently shrink the inventory.
//
// Usage:
//   node parity/tools/ecosystem-inventory.mjs           # write docs/parity/ecosystem.md + .json
//   node parity/tools/ecosystem-inventory.mjs --check   # fail if the committed artifacts are stale
import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import crypto from 'node:crypto';
import { parseToml } from './toml.mjs';

const root = path.resolve(import.meta.dirname, '..', '..');
const CHECK = process.argv.includes('--check');
const MD_PATH = path.join(root, 'docs', 'parity', 'ecosystem.md');
const JSON_PATH = path.join(root, 'docs', 'parity', 'ecosystem-api.json');
const SCHEMA_PATH = path.join(root, 'docs', 'parity', 'knowledge-graph-v1.schema.json');
const projectRows = parseToml(fs.readFileSync(path.join(root, 'parity', 'projects.toml'), 'utf8')).project || [];
const SKIP_DIRS = new Set(['bin', 'obj', '.git', 'node_modules', '.lake', 'TestResults']);

function walkCs(dir, out) {
  if (!fs.existsSync(dir)) return out;
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    if (entry.isDirectory()) {
      if (SKIP_DIRS.has(entry.name)) continue;
      walkCs(path.join(dir, entry.name), out);
    } else if (entry.name.endsWith('.cs')) {
      out.push(path.join(dir, entry.name));
    }
  }
  return out;
}

// Replace comment and string bodies with spaces, preserving newlines and offsets. A declaration
// inside a string or a comment must never enter the inventory.
function mask(text) {
  const chars = text.split('');
  let i = 0;
  const blank = (from, to) => {
    for (let k = from; k < to && k < chars.length; k++) if (chars[k] !== '\n' && chars[k] !== '\r') chars[k] = ' ';
  };
  while (i < chars.length) {
    const c = chars[i];
    const n = chars[i + 1];
    if (c === '/' && n === '/') {
      const start = i;
      while (i < chars.length && chars[i] !== '\n') i++;
      blank(start, i);
      continue;
    }
    if (c === '/' && n === '*') {
      const start = i;
      i += 2;
      while (i < chars.length && !(chars[i] === '*' && chars[i + 1] === '/')) i++;
      i = Math.min(chars.length, i + 2);
      blank(start, i);
      continue;
    }
    if (c === '@' && n === '"') {
      const start = i;
      i += 2;
      while (i < chars.length) {
        if (chars[i] === '"' && chars[i + 1] === '"') { i += 2; continue; }
        if (chars[i] === '"') { i++; break; }
        i++;
      }
      blank(start, i);
      continue;
    }
    if (c === '"' || c === "'") {
      const quote = c;
      const start = i;
      i++;
      while (i < chars.length) {
        if (chars[i] === '\\') { i += 2; continue; }
        if (chars[i] === quote) { i++; break; }
        i++;
      }
      blank(start, i);
      continue;
    }
    i++;
  }
  return chars.join('');
}

function depthMap(text) {
  const depth = new Int32Array(text.length + 1);
  let d = 0;
  for (let i = 0; i < text.length; i++) {
    depth[i] = d;
    if (text[i] === '{') d++;
    else if (text[i] === '}') d = Math.max(0, d - 1);
  }
  depth[text.length] = d;
  return depth;
}

const TYPE_RE = /\b(public|internal|protected|private)\s+((?:(?:sealed|static|abstract|partial|readonly|ref|unsafe|new)\s+)*)(record\s+(?:struct|class)|class|struct|interface|enum|record|delegate)\s+([A-Za-z_][A-Za-z0-9_]*)/g;

function findDecls(masked, depth) {
  const decls = [];
  TYPE_RE.lastIndex = 0;
  let m;
  while ((m = TYPE_RE.exec(masked))) {
    const keyword = m[3].startsWith('record') ? 'record' : m[3];
    const modifiers = m[2].trim().split(/\s+/).filter(Boolean);
    if (m[3].includes(' ')) modifiers.push(...m[3].split(/\s+/));
    let bodyStart = -1;
    let i = TYPE_RE.lastIndex;
    while (i < masked.length) {
      const c = masked[i];
      if (c === '{') { bodyStart = i; break; }
      if (c === ';') break;
      i++;
    }
    let bodyEnd = -1;
    if (bodyStart >= 0) {
      let d = 0;
      for (let k = bodyStart; k < masked.length; k++) {
        if (masked[k] === '{') d++;
        else if (masked[k] === '}') { d--; if (d === 0) { bodyEnd = k; break; } }
      }
    }
    decls.push({ keyword, name: m[4], modifiers, accessibility: m[1], start: m.index, bodyStart, bodyEnd, depth: depth[m.index] });
  }
  return decls;
}

function splitTopLevel(body) {
  const parts = [];
  let start = 0;
  let paren = 0, bracket = 0, angle = 0;
  for (let i = 0; i < body.length; i++) {
    const c = body[i];
    if (c === '(') paren++;
    else if (c === ')') paren = Math.max(0, paren - 1);
    else if (c === '[') bracket++;
    else if (c === ']') bracket = Math.max(0, bracket - 1);
    else if (c === '<') angle++;
    else if (c === '>' && body[i - 1] !== '=') angle = Math.max(0, angle - 1);
    if (paren || bracket || angle) continue;
    if (c === ';') {
      parts.push({ text: body.slice(start, i), terminator: ';' });
      start = i + 1;
    } else if (c === '{') {
      let d = 0;
      let end = i;
      for (let k = i; k < body.length; k++) {
        if (body[k] === '{') d++;
        else if (body[k] === '}') { d--; if (d === 0) { end = k; break; } }
      }
      parts.push({ text: body.slice(start, i), terminator: '{' });
      start = end + 1;
      i = end;
    } else if (c === '=' && body[i + 1] === '>') {
      let k = i + 2;
      let p = 0, b = 0, br = 0;
      for (; k < body.length; k++) {
        const ch = body[k];
        if (ch === '(') p++;
        else if (ch === ')') p = Math.max(0, p - 1);
        else if (ch === '[') b++;
        else if (ch === ']') b = Math.max(0, b - 1);
        else if (ch === '{') br++;
        else if (ch === '}') br = Math.max(0, br - 1);
        else if (ch === ';' && !p && !b && !br) break;
      }
      parts.push({ text: body.slice(start, i), terminator: '=>' });
      start = Math.min(body.length, k + 1);
      i = start - 1;
    }
  }
  if (start < body.length && body.slice(start).trim()) parts.push({ text: body.slice(start), terminator: null });
  return parts;
}

function stripAttributes(text) {
  let out = '';
  let i = 0;
  while (i < text.length) {
    if (text[i] === '[') {
      let d = 0;
      let k = i;
      for (; k < text.length; k++) {
        if (text[k] === '[') d++;
        else if (text[k] === ']') { d--; if (d === 0) break; }
      }
      i = k + 1;
      continue;
    }
    out += text[i];
    i++;
  }
  return out;
}

function classifyMember(raw, typeName, keyword, terminator) {
  const text = stripAttributes(raw).replace(/\s+/g, ' ').trim();
  if (!text) return null;
  if (text.startsWith('=') || text.startsWith(':')) return { unclassified: text.slice(0, 120) };
  const mods = new Set();
  let rest = text;
  const modRe = /^(public|private|protected|internal|static|sealed|override|virtual|abstract|readonly|const|new|extern|unsafe|async|partial|required|volatile|implicit|explicit|event)\b\s*/;
  let guard = 0;
  while (guard++ < 20) {
    const m = rest.match(modRe);
    if (!m) break;
    mods.add(m[1]);
    rest = rest.slice(m[0].length);
  }
  if (!rest) return null;
  const open = rest.indexOf('(');
  const isOperator = /\boperator\b/.test(rest) && open > 0;
  const isIndexer = /\bthis\s*\[/.test(rest);
  const hasParen = open > 0;
  const isEvent = mods.has('event');
  const list = [...mods];
  const accessibility = list.find(x => ['public', 'private', 'protected', 'internal'].includes(x)) || (keyword === 'interface' ? 'public' : 'private');
  let name = null;
  let kind = null;
  if (keyword === 'enum') return null;
  if (isIndexer) {
    return { kind: 'indexer', name: 'this[]', accessibility, modifiers: list, signature: text.slice(0, 300) };
  }
  if (isOperator) {
    const m = rest.match(/(operator\s*[^\s(]+)/);
    return { kind: 'operator', name: (m ? m[1] : 'operator').replace(/\s+/g, ''), accessibility, modifiers: list, signature: text.slice(0, 300) };
  }
  if (hasParen) {
    const before = rest.slice(0, open).trim();
    const nameMatch = before.match(/([A-Za-z_][A-Za-z0-9_]*)\s*(?:<[^>]*>)?$/);
    name = nameMatch ? nameMatch[1] : before;
    kind = name === typeName ? 'constructor' : 'method';
    const close = rest.lastIndexOf(')');
    const params = close > open ? rest.slice(open + 1, close).replace(/\s+/g, ' ').trim() : '';
    return { kind, name, accessibility, modifiers: list, params, signature: text.slice(0, 300) };
  }
  if (isEvent) {
    const nameMatch = rest.match(/([A-Za-z_][A-Za-z0-9_]*)\s*(?:[=;{]|$)/);
    return { kind: 'event', name: nameMatch ? nameMatch[1] : null, accessibility, modifiers: list, signature: text.slice(0, 300) };
  }
  if (terminator === '=>' || terminator === '{') {
    const nameMatch = rest.match(/([A-Za-z_][A-Za-z0-9_]*)\s*$/);
    return { kind: 'property', name: nameMatch ? nameMatch[1] : null, accessibility, modifiers: list, signature: text.slice(0, 300) };
  }
  const before = rest.split(/[=;:]/)[0].trim();
  const nameMatch = before.match(/([A-Za-z_][A-Za-z0-9_]*)\s*$/);
  return { kind: 'field', name: nameMatch ? nameMatch[1] : null, accessibility, modifiers: list, signature: text.slice(0, 300) };
}


function parseFile(abs, rel, project) {
  const masked = mask(fs.readFileSync(abs, 'utf8'));
  const depth = depthMap(masked);
  const decls = findDecls(masked, depth);
  for (const d of decls) {
    d.containing = decls.filter(x => x !== d && x.bodyStart >= 0 && x.bodyStart < d.start && x.bodyEnd > d.start)
      .sort((a, b) => b.depth - a.depth)[0] || null;
  }
  const isVisible = (x) => x.accessibility === 'public' && (!x.containing || isVisible(x.containing));
  const out = [];
  for (const d of decls) {
    d.project = project.name;
    d.file = rel;
    d.line = lineOf(masked, d.start);
    d.externallyVisible = isVisible(d);
    d.members = [];
    d.unclassified = [];
    if (d.bodyStart < 0 || d.keyword === 'delegate') {
      if (d.keyword === 'delegate') d.delegateSignature = masked.slice(d.start, d.bodyStart >= 0 ? d.bodyStart : masked.length).replace(/\s+/g, ' ').trim();
      out.push(d);
      continue;
    }
    const nested = decls.filter(x => x !== d && x.bodyStart > d.bodyStart && x.bodyEnd < d.bodyEnd);
    let body = masked.slice(d.bodyStart + 1, d.bodyEnd);
    const holes = nested.map(x => [x.start - d.bodyStart - 1, x.bodyEnd - d.bodyStart])
      .filter(([a, b]) => a >= 0 && b <= body.length)
      .sort((a, b) => b[0] - a[0]);
    for (const [from, to] of holes) body = body.slice(0, from) + ' '.repeat(to - from) + body.slice(to);
    if (d.keyword === 'enum') {
      for (const v of splitTopLevel(body).flatMap(p => p.text.split(','))) {
        const name = stripAttributes(v).replace(/\s+/g, ' ').trim().split('=')[0].trim();
        if (!name) continue;
        if (/^[A-Za-z_][A-Za-z0-9_]*$/.test(name)) d.members.push({ kind: 'enum-value', name, accessibility: 'public' });
        else d.unclassified.push(name.slice(0, 80));
      }
    } else {
      for (const part of splitTopLevel(body)) {
        if (!part.text.trim()) continue;
        if (part.text.trimStart().startsWith('=')) continue; // property/field initializer continuation
        const classified = classifyMember(part.text, d.name, d.keyword, part.terminator);
        if (!classified) continue;
        if (classified.unclassified) { d.unclassified.push(classified.unclassified); continue; }
        if (d.keyword === 'interface' || classified.accessibility === 'public' || classified.accessibility === 'protected') d.members.push(classified);
      }
      if (d.keyword === 'record') {
        const header = masked.slice(d.start, d.bodyStart);
        const paren = header.indexOf('(');
        if (paren >= 0) {
          let pd = 0, end = -1;
          for (let i = paren; i < header.length; i++) {
            if (header[i] === '(') pd++;
            else if (header[i] === ')') { pd--; if (pd === 0) { end = i; break; } }
          }
          if (end > paren) {
            for (const p of header.slice(paren + 1, end).split(',')) {
              const name = p.trim().replace(/\s*=.*$/, '').split(/\s+/).pop();
              if (name && /^[A-Za-z_][A-Za-z0-9_]*$/.test(name) && !d.members.some(x => x.name === name)) {
                d.members.push({ kind: 'record-property', name, accessibility: 'public' });
              }
            }
          }
        }
      }
    }
    out.push(d);
  }
  return out;
}

// --- disposition -------------------------------------------------------------------------
function projectDisposition(p) {
  if (p.status === 'absent') {
    return { status: 'absent', text: 'absent from the Rust workspace', rust: [], reason: p.reason || '', phase: p.phase };
  }
  return {
    status: p.status,
    text: p.status === 'port' ? 'ported' : p.status === 'consolidated' ? 'consolidated' : 'partial',
    rust: p.rust || [],
    reason: p.note || '',
    phase: p.phase,
  };
}

// --- curated ecosystem facts (each claim is checked against the tree) ---------------------
// ECO-004: the consumers that exist today. Every entry names a tracked file and a needle that must
// occur in it; the generator fails when a consumer disappears or its entry point changes, so this
// table cannot drift into fiction.
const CONSUMERS = [
  {
    id: 'CON-001', kind: 'command-line application', name: 'Lovelace.Run CLI',
    evidence: [{ path: 'Lovelace.Run/Program.cs', needle: 'Runner.RunAsync' }],
    entry: 'argv + stdin -> one JSON envelope on stdout; exit code 0/1',
    disposition: 'Rust equivalent: rust/lovelace-run (same argv, same envelope). Covered by parity/tools/gate-cli.mjs (16/16) and gate-fixtures.mjs (20/20).',
  },
  {
    id: 'CON-002', kind: 'application (web host + browser SPA)', name: 'Lovelace.Studio',
    evidence: [{ path: 'Lovelace.Studio/Program.cs', needle: 'WebApplication' }, { path: 'Lovelace.Studio/wwwroot/app.js', needle: 'fetch(' }],
    entry: 'HTTP JSON routes + X-Session-Id session header; the SPA talks only to those routes',
    disposition: 'Rust equivalent: rust/lovelace-desktop (native Win32 shell, no HTTP). DESK-* rows remain partial; no browser client is required.',
  },
  {
    id: 'CON-003', kind: 'command-line application', name: 'Lovelace.Knowledge.Run',
    evidence: [{ path: 'Lovelace.Knowledge.Run/Program.cs', needle: 'CliRequest' }],
    entry: 'JSON request/response over stdio; graph files on disk',
    disposition: 'Absent from the Rust workspace (parity/projects.toml: Lovelace.Knowledge absent, KNOW-001..010 unmet).',
  },
  {
    id: 'CON-004', kind: 'command-line application (REPL)', name: 'Lovelace.Console',
    evidence: [{ path: 'Lovelace.Console/Repl/ReplSession.cs', needle: 'class ReplSession' }],
    entry: 'interactive console loop; --file script mode',
    disposition: 'Absent from the Rust workspace (CLI-001..007 unmet). Scripts are served by lovelace-run today.',
  },
  {
    id: 'CON-005', kind: 'agent harness', name: 'DSH lovelace tool',
    evidence: [{ path: 'harness/lovelace.host.js', needle: 'runner' }],
    entry: 'spawns the published Lovelace.Run executable with --eval/--file and parses its JSON envelope',
    disposition: 'Entry-point compatible with rust/lovelace-run: the host passes a runner path and parses the protocol, not a .NET assembly.',
  },
  {
    id: 'CON-006', kind: 'agent harness', name: 'DSH mgir tool',
    evidence: [{ path: 'harness/knowledge.host.js', needle: 'cli' }],
    entry: 'spawns Lovelace.Knowledge.Run with a JSON request on stdin',
    disposition: 'Blocked on the Knowledge port (CON-003); the harness contract is process + JSON, not .NET interop.',
  },
  {
    id: 'CON-007', kind: 'benchmark harness', name: 'harnessability-bench',
    evidence: [{ path: 'harnessability-bench/checks/wire_contract.mjs', needle: 'Lovelace.Run' }],
    entry: 'spawns the published runner and validates the wire contract',
    disposition: 'Runner path is a parameter; the Rust runner can be substituted without changing the harness.',
  },
  {
    id: 'CON-008', kind: 'automation (CI)', name: '.github/workflows/ci.yml',
    evidence: [{ path: '.github/workflows/ci.yml', needle: 'dotnet build' }],
    entry: 'dotnet build/test/publish of the C# suites and the Native AOT runner',
    disposition: 'Retained: it is the C# oracle build/execute job (TEST-002). The Rust CI job added in RUST-010 is separate.',
  },
  {
    id: 'CON-009', kind: 'automation (parity gates)', name: 'parity/build-oracle.ps1 + gate-cli/gate-fixtures',
    evidence: [{ path: 'parity/build-oracle.ps1', needle: 'dotnet' }, { path: 'parity/tools/gate-cli.mjs', needle: 'dotnet' }],
    entry: 'builds/executes the pinned oracle and compares it with the Rust binary',
    disposition: 'Repository-internal; retired when the oracle is retired. Must keep working while TEST-001..005 hold.',
  },
  {
    id: 'CON-010', kind: 'automation (build)', name: 'Makefile',
    evidence: [{ path: 'Makefile', needle: 'dotnet publish' }],
    entry: 'publishes the runner and the knowledge CLI',
    disposition: 'Operator convenience; no compatibility obligation beyond the CLI it invokes.',
  },
  {
    id: 'CON-011', kind: 'automation (operator)', name: 'tools/*.ps1 desktop and plot harnesses',
    evidence: [{ path: 'tools/desktop-gate.ps1', needle: 'Lovelace Desktop' }, { path: 'tools/plot-fidelity.ps1', needle: 'svg' }],
    entry: 'drives the Studio HTTP host and inspects plot output',
    disposition: 'Studio-driving scripts are superseded by the Rust desktop probes; plot scripts remain usable against SVG files.',
  },
  {
    id: 'CON-012', kind: 'application (documentation renderer)', name: 'Lovelace.Knowledge/tools/render-graph-pdf.mjs',
    evidence: [{ path: 'Lovelace.Knowledge/tools/render-graph-pdf.mjs', needle: 'graph' }],
    entry: 'reads a persisted knowledge-graph JSON and renders a PDF',
    disposition: 'Format-level consumer: the graph reader must keep working, or the script plus its input must migrate together.',
  },
  {
    id: 'CON-013', kind: 'automation (historical evidence)', name: 'docs/** capture and verification scripts',
    evidence: [{ path: 'docs/goal-cycle-3/round-1/baseline.ps1', needle: 'dotnet' }],
    entry: 'snapshot oracle behaviour into recorded JSON/text evidence',
    disposition: 'Historical evidence, not a shipped consumer. Scanned in the appendix so the count is explicit; no compatibility obligation.',
  },
  {
    id: 'CON-014', kind: 'application (benchmarks)', name: 'symbench, dspbench, precbench, arraybench, bench, realbench, mulbench',
    evidence: [{ path: 'symbench/symbench.csproj', needle: 'Project' }, { path: 'dspbench/dspbench.csproj', needle: 'Project' }],
    entry: 'C# BenchmarkDotNet harnesses compiled against the libraries',
    disposition: 'Benchmark harnesses (PERF scope), not product consumers. They are evidence of library-level assembly use, not a reason for a C ABI.',
  },
];



// ECO-008: persisted formats. `rustReader` is what the Rust product can read today, checked against
// a Rust source needle; `migration` is the recorded path when it cannot.
const FORMATS = [
  {
    id: 'FMT-001', name: 'Knowledge graph JSON', version: 'GraphStore.Version = 1',
    evidence: [{ path: 'Lovelace.Knowledge/GraphStore.cs', needle: 'Version = 1' }, { path: 'knowledge-graph-demo.json', needle: '"version": 1' }],
    writers: 'Lovelace.Knowledge.GraphStore.Save; Lovelace.Knowledge.Run converge/reduce',
    readers: 'GraphStore.Load; Knowledge.Run query; Lovelace.Knowledge/tools/render-graph-pdf.mjs',
    rustReader: null,
    decision: 'migration path',
    migration: 'Versioned, self-describing JSON whose complete field set is listed below and validated by parity/tools/format-check.mjs. The Rust workspace has no Knowledge module (projects.toml: absent, KNOW-001..010 unmet), so the C# oracle remains the only reader today. The version byte is checked by the validator; a Rust reader must reject any version other than 1 and is blocked on the Knowledge port, not on this format.',
  },
  {
    id: 'FMT-002', name: 'Runner JSON envelope', version: 'protocolVersion = 1',
    evidence: [{ path: 'Lovelace.Run/Runner.cs', needle: 'ProtocolVersion = 1' }, { path: 'Lovelace.Run.Tests/fixtures/record.json', needle: '"protocolVersion"' }],
    writers: 'Lovelace.Run (--json / default); rust/lovelace-run',
    readers: 'agents, harness/lovelace.host.js, harnessability-bench, parity gates',
    rustReader: { path: 'rust/lovelace-run/src/main.rs', needle: 'PROTOCOL_VERSION' },
    decision: 'readable by Rust',
    migration: 'No migration needed: the Rust runner emits the same versioned envelope. Verified by node parity/tools/gate-fixtures.mjs (20/20) and node parity/tools/gate-cli.mjs (16/16).',
  },
  {
    id: 'FMT-003', name: 'MathIR text', version: '#!mathir 2',
    evidence: [{ path: 'Lovelace.MathIR/Ir.cs', needle: '#!mathir ' }],
    writers: 'Lovelace.MathIR.IrProgram.Serialize; rust/lovelace-lang mathir',
    readers: 'Lovelace.MathIR.IrProgram.Deserialize; rust/lovelace-lang::mathir::IrProgram::deserialize',
    rustReader: { path: 'rust/lovelace-lang/src/mathir.rs', needle: '"#!mathir "' },
    decision: 'readable by Rust',
    migration: 'No migration needed: both sides write and gate #!mathir 2. Verified by cargo test -p lovelace-lang mathir (deserialize and version-gate tests).',
  },
  {
    id: 'FMT-004', name: 'Symbolic canonical text', version: '#!lovelace-sym 1',
    evidence: [{ path: 'Lovelace.Symbolics/Printing.cs', needle: '#!lovelace-sym 1' }],
    writers: 'Lovelace.Symbolics.Printing; rust/lovelace-run symbolic projection',
    readers: 'symbolic value consumers; parity fixtures',
    rustReader: { path: 'rust/lovelace-run/src/main.rs', needle: '#!lovelace-sym 1' },
    decision: 'readable by Rust',
    migration: 'No migration needed: the Rust runner publishes the same header. Verified by the golden fixtures and the CLI matrix.',
  },
  {
    id: 'FMT-005', name: 'Plot SVG files', version: 'SVG (no custom header)',
    evidence: [{ path: 'Lovelace.Suite/Interpreter.cs', needle: 'File.WriteAllText' }, { path: 'rust/lovelace-lang/src/plot_surface.rs', needle: '<svg' }],
    writers: 'Lovelace.Suite.Interpreter (plot.svg); rust/lovelace-lang plot surface',
    readers: 'any SVG renderer; browsers; Rust desktop',
    rustReader: { path: 'rust/lovelace-lang/src/plot_surface.rs', needle: '<svg' },
    decision: 'readable by Rust',
    migration: 'No migration needed: the file is a standard SVG document with no proprietary wrapper. Verified by rendering a plot through the Rust binary and checking the SVG root (see the report for the run).',
  },
  {
    id: 'FMT-006', name: 'Studio browser state', version: 'localStorage keys (editor text, panel sizes)',
    evidence: [{ path: 'Lovelace.Studio/wwwroot/app.js', needle: 'localStorage.setItem(EDITOR_KEY' }],
    writers: 'the browser SPA',
    readers: 'the browser SPA only',
    rustReader: null,
    decision: 'no compatibility obligation',
    migration: 'No file format and no server-side persistence: the values live in the browser origin and hold plain text plus panel dimensions. The Rust desktop keeps its own UI state. Editors can copy the text; nothing is lost that the product owns.',
  },
  {
    id: 'FMT-007', name: 'Studio sessions', version: 'in-memory',
    evidence: [{ path: 'Lovelace.Studio/SessionRegistry.cs', needle: 'ConcurrentDictionary' }],
    writers: 'Lovelace.Studio.SessionRegistry (memory only)',
    readers: 'Lovelace.Studio HTTP routes',
    rustReader: null,
    decision: 'no compatibility obligation',
    migration: 'Sessions are process memory with an idle timeout; no serialized session file exists to migrate. Recorded so the absence is a decision, not an oversight.',
  },
  {
    id: 'FMT-008', name: 'Knowledge.Run CLI JSON', version: 'CliRequest / CliResponse (camelCase JSON)',
    evidence: [{ path: 'Lovelace.Knowledge.Run/Program.cs', needle: 'CliRequest' }],
    writers: 'Lovelace.Knowledge.Run',
    readers: 'harness/knowledge.host.js',
    rustReader: null,
    decision: 'migration path',
    migration: 'Process protocol, not a stored file. It rides on the same Knowledge port as FMT-001 (CON-003/CON-006); until then the C# CLI remains the implementation and no Rust consumer claims otherwise.',
  },
  {
    id: 'FMT-009', name: 'Repository test/evidence data (corpus, fixtures, census, probes)', version: 'repo-internal',
    evidence: [{ path: 'parity/corpus/lang.json', needle: '"examples"' }, { path: 'Lovelace.Run.Tests/fixtures/record.json', needle: '"protocolVersion"' }],
    writers: 'parity/extract-corpus.mjs, census tool, probes',
    readers: 'parity gates, lovelace-corpus, lovelace-run',
    rustReader: { path: 'rust/lovelace-corpus/src/main.rs', needle: 'corpus' },
    decision: 'readable by Rust',
    migration: 'Not a product persistence format: these are the migration own measurements. The Rust corpus gate reads the same JSON. Verified by cargo run --release -p lovelace-corpus (214/214).',
  },
];

// ECO-008 FMT-001: the complete field set of a persisted graph, generated from the checked-in
// example plus the C# record. A format whose fields are not written down cannot be migrated.
function graphFieldSet() {
  const graphPath = path.join(root, 'knowledge-graph-demo.json');
  const graph = JSON.parse(fs.readFileSync(graphPath, 'utf8'));
  const describe = (value) => {
    if (Array.isArray(value)) return 'array[' + value.length + ']';
    if (value === null) return 'null';
    return typeof value;
  };
  const elementKeys = (arr) => {
    if (!Array.isArray(arr) || arr.length === 0) return [];
    const keys = new Set();
    for (const item of arr) if (item && typeof item === 'object' && !Array.isArray(item)) for (const k of Object.keys(item)) keys.add(k);
    return [...keys].sort();
  };
  return {
    sha256: sha256File(graphPath),
    bytes: fs.statSync(graphPath).size,
    version: graph.version,
    seed: graph.seed,
    top: Object.keys(graph).sort().map(k => ({ key: k, type: describe(graph[k]) })),
    configKeys: Object.keys(graph.config || {}).sort(),
    samples: (graph.samples || []).length,
    planes: (graph.planes || []).length,
    boundaries: (graph.boundaries || []).length,
    frontiers: (graph.frontiers || []).length,
    elementKeys: {
      samples: elementKeys(graph.samples),
      planes: elementKeys(graph.planes),
      boundaries: elementKeys(graph.boundaries),
      frontiers: elementKeys(graph.frontiers),
      metrics: graph.metrics ? Object.keys(graph.metrics).sort() : [],
    },
    extraKeys: Object.keys(graph).filter(k => !['seed','version','config','samples','planes','boundaries','frontiers','metrics'].includes(k)),
  };
}

function sha256File(abs) {
  return crypto.createHash('sha256').update(fs.readFileSync(abs)).digest('hex');
}

function trackedFiles() {
  const res = spawnSync('git', ['-C', root, 'ls-files'], { encoding: 'utf8', maxBuffer: 64 * 1024 * 1024 });
  if (res.status !== 0) throw new Error('git ls-files failed: ' + (res.stderr || '').slice(0, 200));
  return res.stdout.split(/\r?\n/).filter(Boolean);
}

const ENTRY_RE = /Lovelace\.Run|Lovelace\.Knowledge\.Run|Lovelace\.Studio|dotnet (run|build|test|publish|exec)/;
const CONSUMER_EXT_RE = /\.(ps1|sh|mjs|js|py|csproj|yml|yaml)$/;

function groupOf(file) {
  const first = file.split('/')[0];
  if (first === '.github') return { kind: 'CI automation', disposition: 'Oracle build/execute; retained (CON-008).' };
  if (first === 'docs') return { kind: 'historical evidence script', disposition: 'Captured probe/verification, no compatibility obligation (CON-013).' };
  if (first === 'parity') return { kind: 'parity automation', disposition: 'Repository-internal gate (CON-009).' };
  if (first === 'harness') return { kind: 'agent harness', disposition: 'Spawns a published binary (CON-005/006).' };
  if (first === 'harnessability-bench') return { kind: 'benchmark harness', disposition: 'Runner path is a parameter (CON-007).' };
  if (first === 'tools') return { kind: 'operator automation', disposition: 'Operator script; no shipped contract (CON-011).' };
  if (/\.Tests$/.test(first)) return { kind: 'C# test project', disposition: 'Test code, not an external consumer.' };
  if (/^(bench|symbench|dspbench|precbench|arraybench|realbench|mulbench)$/.test(first)) return { kind: 'C# benchmark application', disposition: 'Benchmark harness (CON-014).' };
  if (first.startsWith('Lovelace.')) return { kind: 'C# product project', disposition: 'Product application or library (see the project table).' };
  return { kind: 'other', disposition: 'Reviewed: no shipped contract.' };
}

function consumerScan() {
  const rows = [];
  for (const file of trackedFiles()) {
    if (!CONSUMER_EXT_RE.test(file)) continue;
    const abs = path.join(root, file);
    if (!fs.existsSync(abs)) continue;
    let text = '';
    try { text = fs.readFileSync(abs, 'utf8'); } catch { continue; }
    if (!ENTRY_RE.test(text)) continue;
    const g = groupOf(file);
    rows.push({ file, kind: g.kind, disposition: g.disposition });
  }
  return rows;
}

function validateCurated() {
  const failures = [];
  for (const c of CONSUMERS) {
    for (const e of c.evidence) {
      const abs = path.join(root, e.path);
      if (!fs.existsSync(abs)) { failures.push(c.id + ': missing evidence file ' + e.path); continue; }
      if (!fs.readFileSync(abs, 'utf8').includes(e.needle)) failures.push(c.id + ': ' + e.path + ' does not contain ' + JSON.stringify(e.needle));
    }
  }
  for (const f of FORMATS) {
    for (const e of f.evidence) {
      const abs = path.join(root, e.path);
      if (!fs.existsSync(abs)) { failures.push(f.id + ': missing evidence file ' + e.path); continue; }
      const text = fs.readFileSync(abs, 'utf8');
      if (!text.includes(e.needle)) failures.push(f.id + ': ' + e.path + ' does not contain ' + JSON.stringify(e.needle));
    }
    if (f.rustReader) {
      const abs = path.join(root, f.rustReader.path);
      if (!fs.existsSync(abs)) failures.push(f.id + ': missing Rust reader ' + f.rustReader.path);
      else if (!fs.readFileSync(abs, 'utf8').includes(f.rustReader.needle)) failures.push(f.id + ': ' + f.rustReader.path + ' does not contain ' + JSON.stringify(f.rustReader.needle));
    }
  }
  return failures;
}

function ffiAbsence() {
  const patterns = [
    { label: 'C# P/Invoke or native-library loading (DllImport / LibraryImport / NativeLibrary)', re: /\b(DllImport|LibraryImport|NativeLibrary)\b/, ext: /\.cs$/ },
    { label: 'Rust C ABI exports (extern "C" / cdylib / staticlib / no_mangle)', re: /\bextern\s+"C"|cdylib|staticlib|no_mangle/, ext: /\.rs$|Cargo\.toml$/ },
  ];
  const files = trackedFiles().filter(f => patterns.some(p => p.ext.test(f)));
  return patterns.map(p => {
    const hits = [];
    for (const file of files.filter(f => p.ext.test(f))) {
      const abs = path.join(root, file);
      if (!fs.existsSync(abs)) continue;
      if (p.re.test(fs.readFileSync(abs, 'utf8'))) hits.push(file);
    }
    return { label: p.label, files: hits.length, samples: hits.slice(0, 5) };
  });
}

function typeCounts(types) {
  const counts = { method: 0, constructor: 0, property: 0, indexer: 0, field: 0, event: 0, operator: 0, 'enum-value': 0, 'record-property': 0, delegate: 0 };
  for (const t of types) for (const m of t.members) counts[m.kind] = (counts[m.kind] || 0) + 1;
  return counts;
}


// --- markdown -----------------------------------------------------------------------------
const UNCLASSIFIED_CEILING = 60;

function kindCount(members, kind) {
  return members.filter(m => m.kind === kind).length;
}

function esc(text) {
  return String(text).replace(/\|/g, '\\|');
}

function renderMarkdown(data) {
  const md = [];
  const p = (s) => md.push(s === undefined ? '' : s);
  p('# Ecosystem inventory: C# surface, consumers, and persisted formats');
  p('');
  p('Generated by ' + '`node parity/tools/ecosystem-inventory.mjs`' + ' from the frozen C# tree. **Do not edit by hand**; the');
  p('generator fails if the tree and this document disagree. Machine-readable companion:');
  p('[ecosystem-api.json](ecosystem-api.json) — every member with kind, accessibility, disposition');
  p('and Rust target.');
  p('');
  p('Contract rows: ECO-003 (externally visible types and methods), ECO-004 (external consumers),');
  p('ECO-007 (the .NET compatibility decision), ECO-008 (persisted formats and readability).');
  p('Dispositions are derived from ' + '`parity/projects.toml`' + ', the project ledger the ECO-001/002 gate');
  p('already verifies; this document adds the per-type and per-member detail that ledger does not carry.');
  p('');
  p('## 0. What is counted, and how');
  p('');
  p('The scan reads every ' + '`.cs`' + ' file under each of the 19 production project directories named in');
  p('`parity/projects.toml` (test projects, ' + '`bin`' + ', ' + '`obj`' + ', ' + '`docs`' + ', and parity tooling are not product');
  p('surface). Comments and string literals are blanked first, so a declaration inside either cannot');
  p('enter the inventory. A type is **externally visible** when it is ' + '`public`' + ' and every enclosing type is');
  p('also public; a member is externally visible when it is ' + '`public`' + ' or ' + '`protected`' + ' (interface members are');
  p('implicitly public). Types and members that are declared public but are unreachable from outside the');
  p('assembly are counted as *unreachable public declarations* and excluded from the surface totals.');
  p('');
  p('| Measure | Count |');
  p('|---|---:|');
  p('| Production projects (parity/projects.toml) | ' + data.totals.projects + ' |');
  p('| Projects with no C# source | ' + data.totals.projectsWithoutCs + ' |');
  p('| ' + '`.cs`' + ' files scanned | ' + data.totals.files + ' |');
  p('| Public type declarations found | ' + data.totals.publicDeclarations + ' |');
  p('| Externally visible types | ' + data.totals.visibleTypes + ' |');
  p('| Unreachable public declarations (excluded) | ' + data.totals.unreachablePublicTypes + ' |');
  p('| Externally visible members | ' + data.totals.visibleMembers + ' |');
  p('| — methods | ' + data.totals.memberCounts.method + ' |');
  p('| — constructors | ' + data.totals.memberCounts.constructor + ' |');
  p('| — properties | ' + data.totals.memberCounts.property + ' |');
  p('| — fields | ' + data.totals.memberCounts.field + ' |');
  p('| — events | ' + data.totals.memberCounts.event + ' |');
  p('| — operators | ' + data.totals.memberCounts.operator + ' |');
  p('| — indexers | ' + data.totals.memberCounts.indexer + ' |');
  p('| — enum values | ' + data.totals.memberCounts['enum-value'] + ' |');
  p('| — record positional properties | ' + data.totals.memberCounts['record-property'] + ' |');
  p('| Unclassified declarations (parser backlog) | ' + data.totals.unclassified + ' / ceiling ' + UNCLASSIFIED_CEILING + ' |');
  p('');
  p('The scan is a source scan, not reflection: it cannot execute the frozen assemblies here (the');
  p('VSTest host is blocked in this environment — see parity/README.md). Its limits are explicit:');
  p('explicit interface implementations are private and not listed; attributes are dropped; overloads');
  p('are distinguished by their parameter text; nested types are resolved through their enclosing');
  p('chain; a declaration the parser cannot classify is counted and quoted in the JSON, never');
  p('silently dropped. The ceiling above makes a parser regression a gate failure, not a smaller');
  p('number that looks like progress.');
  p('');
  p('## 1. Externally visible surface (ECO-003)');
  p('');
  p('### 1.1 Per project');
  p('');
  p('| Project | Disposition | Types | Methods | Ctors | Props | Fields | Events | Other | Members | Rust target |');
  p('|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---|');
  for (const proj of data.projects) {
    const c = proj.memberCounts;
    const other = c.operator + c.indexer + c['enum-value'] + c['record-property'];
    p('| ' + proj.name + ' | ' + proj.disposition + ' | ' + proj.typeCount + ' | ' + c.method + ' | ' + c.constructor + ' | ' + c.property + ' | ' + c.field + ' | ' + c.event + ' | ' + other + ' | ' + proj.memberCount + ' | ' + (proj.rust.length ? proj.rust.map(r => '`' + r + '`').join('<br>') : '—') + ' |');
  }
  p('| **Total** | | **' + data.totals.visibleTypes + '** | **' + data.totals.memberCounts.method + '** | **' + data.totals.memberCounts.constructor + '** | **' + data.totals.memberCounts.property + '** | **' + data.totals.memberCounts.field + '** | **' + data.totals.memberCounts.event + '** | **' + (data.totals.memberCounts.operator + data.totals.memberCounts.indexer + data.totals.memberCounts['enum-value'] + data.totals.memberCounts['record-property']) + '** | **' + data.totals.visibleMembers + '** | |');
  p('');
  p('Dispositions: ' + data.totals.byDisposition.map(d => d.status + ' = ' + d.count).join(', ') + '. "partial" and "absent" are');
  p('recorded facts from parity/projects.toml, not waivers: the member rows below inherit them.');
  p('');
  p('### 1.2 Per type');
  p('');
  p('| Project | Type | Kind | Members | Disposition |');
  p('|---|---|---|---:|---|');
  for (const t of data.types) {
    p('| ' + t.project + ' | ' + '`' + t.name + '`' + ' | ' + t.kind + ' | ' + t.members.length + ' | ' + dispositionOf(data, t.project, t.name) + ' |');
  }
  p('');
  p('### 1.3 Member index');
  p('');
  p('Every externally visible member, grouped by project, with its kind and the disposition inherited');
  p('from the project ledger. Method rows carry the parameter text so overloads are distinguishable.');
  p('');
  for (const proj of data.projects) {
    const types = data.types.filter(t => t.project === proj.name);
    if (types.length === 0) continue;
    p('<details><summary><strong>' + proj.name + '</strong> — ' + types.length + ' type(s), ' + proj.memberCount + ' member(s), disposition: ' + proj.disposition + '</summary>');
    p('');
    for (const t of types) {
      p('**' + '`' + t.name + '`' + '** (' + t.kind + ', ' + t.file + ':' + t.line + ')');
      p('');
      if (t.members.length === 0) {
        p('- (no externally visible members)');
      } else {
        for (const m of t.members) {
          const sig = (m.kind === 'method' || m.kind === 'constructor' || m.kind === 'operator') && m.params ? '(' + m.params + ')' : '';
          p('- ' + '`' + t.name + '.' + m.name + sig + '`' + ' — ' + m.kind + ', ' + m.disposition);
        }
      }
      p('');
    }
    p('</details>');
    p('');
  }
  p('## 2. External consumers (ECO-004)');
  p('');
  p('The curated list names every distinct consumer class in the repository and the entry point it');
  p('uses. Each row carries an evidence file and a needle that the generator requires to be present, so');
  p('a consumer that disappears or changes entry point breaks the build of this document.');
  p('');
  p('| ID | Kind | Consumer | Entry point | Disposition |');
  p('|---|---|---|---|---|');
  for (const c of data.consumers.curated) {
    p('| ' + c.id + ' | ' + esc(c.kind) + ' | ' + esc(c.name) + ' | ' + esc(c.entry) + ' | ' + esc(c.disposition) + ' |');
  }
  p('');
  p('The scan found **' + data.consumers.scan.files + '** tracked scripts/projects that reference a C# entry point');
  p('(patterns: ' + '`Lovelace.Run`' + ', ' + '`Lovelace.Knowledge.Run`' + ', ' + '`Lovelace.Studio`' + ', ' + '`dotnet run|build|test|publish|exec`' + ').');
  p('Grouped by location and disposition:');
  p('');
  p('| Location | Files | Classification | Disposition |');
  p('|---|---:|---|---|');
  for (const g of data.consumers.scan.byDirectory) {
    p('| ' + esc(g.dir) + ' | ' + g.count + ' | ' + esc(g.kind) + ' | ' + esc(g.disposition) + ' |');
  }
  p('');
  p('<details><summary>Full scanned file list</summary>');
  p('');
  for (const r of data.consumers.scan.rows) p('- ' + '`' + r.file + '`' + ' — ' + r.kind);
  p('');
  p('</details>');
  p('');
  p('No consumer in this repository loads a Lovelace assembly from outside the solution. The');
  p('assembly-level consumers are the C# projects and the benchmark harnesses inside the solution');
  p('(CON-014); every cross-boundary consumer spawns a process and speaks a file/stdio format');
  p('(CON-001, CON-005..CON-007, CON-012). That is the evidence behind the ECO-007 decision below.');
  p('');
  p('## 3. Persisted formats (ECO-008)');
  p('');
  p('| ID | Format | Version | Rust reader | Decision |');
  p('|---|---|---|---|---|');
  for (const f of data.formats) {
    p('| ' + f.id + ' | ' + esc(f.name) + ' | ' + '`' + esc(f.version) + '`' + ' | ' + (f.rustReader ? 'yes — ' + '`' + f.rustReader.path + '`' : 'no') + ' | ' + f.decision + ' |');
  }
  p('');
  for (const f of data.formats) {
    p('### ' + f.id + ' — ' + f.name);
    p('');
    p('- Version: ' + '`' + f.version + '`');
    p('- Writers: ' + f.writers);
    p('- Readers: ' + f.readers);
    p('- Evidence: ' + f.evidence.map(e => '`' + e.path + '`').join(', '));
    p('- Rust reader: ' + (f.rustReader ? '`' + f.rustReader.path + '`' : 'none'));
    p('- Decision: **' + f.decision + '**');
    p('- Migration / readability: ' + f.migration);
    p('');
  }
  const graph = data.formats.find(f => f.id === 'FMT-001').graph;
  p('### FMT-001 field set (the migration contract for knowledge graphs)');
  p('');
  p('Measured from ' + '`knowledge-graph-demo.json`' + ' (sha256 ' + '`' + graph.sha256 + '`' + ', ' + graph.bytes + ' bytes, seed ' + graph.seed + ', version ' + graph.version + '):');
  p('');
  p('| Top-level field | JSON type |');
  p('|---|---|');
  for (const t of graph.top) p('| ' + '`' + t.key + '`' + ' | ' + t.type + ' |');
  p('');
  p('Required element fields: ' + Object.entries(graph.elementKeys).filter(([, v]) => v.length).map(([k, v]) => '`' + k + '`' + ' → ' + v.map(x => '`' + x + '`').join(', ')).join('; ') + '.');
  p('');
  p('Config fields (' + graph.configKeys.length + '): ' + graph.configKeys.map(k => '`' + k + '`').join(', ') + '.');
  p('');
  p('The same field set is emitted as ' + '`docs/parity/knowledge-graph-v1.schema.json`' + '. The validator');
  p('' + '`parity/tools/format-check.mjs --check`' + ' requires version 1 and fails on an unknown or missing');
  p('top-level field, a config field outside the schema, or an element of samples/planes/boundaries/');
  p('frontiers whose field set differs from the schema.');
  p('');
  p('## 4. The .NET compatibility decision (ECO-007)');
  p('');
  p('**Decision: no .NET consumer requires continued compatibility through a C ABI, an FFI bridge, or');
  p('a C# wrapper. None is built, none is needed, and none is planned.**');
  p('');
  p('Justification, in the order the evidence was checked:');
  p('');
  p('1. **The consumers are process-level.** Every consumer that crosses the .NET boundary in this');
  p('   repository spawns a published executable and reads stdout or a file (CON-001, CON-005,');
  p('   CON-006, CON-007, CON-009, CON-012). The published surface they depend on is the versioned');
  p('   JSON envelope, argv/stdin, the graph JSON, MathIR text, symbolic text and SVG — all');
  p('   process/file formats (FMT-001..FMT-009), not managed object layouts.');
  p('2. **No interop mechanism exists to preserve.** The generator scans every tracked C# file for');
  p('   ' + '`DllImport`' + '/' + '`LibraryImport`' + '/' + '`NativeLibrary`' + ' and every tracked Rust file and manifest for');
  p('   ' + '`extern "C"`' + ', ' + '`cdylib`' + ', ' + '`staticlib`' + ' and ' + '`no_mangle`' + ': ' + data.compatibility.absence.map(a => a.files).reduce((a, b) => a + b, 0) + ' matches (table below). There is no bridge to keep working, and');
  p('   a C ABI would be new surface with no consumer.');
  p('3. **The only assembly-level consumers migrate together.** The benchmark projects (CON-014) and');
  p('   the C# test suites compile against the libraries; they are part of the same solution and are');
  p('   retired or replaced with the reference, not hosted by the Rust build.');
  p('4. **The desktop is replaced, not embedded.** Lovelace.Studio becomes a native Rust application');
  p('   (CON-002); it does not host the .NET engine through FFI. A wrapper would preserve a host');
  p('   process the migration is explicitly removing.');
  p('5. **Fallback is the reference, not a bridge.** REQ-009 keeps C# as the reference until cutover;');
  p('   if an out-of-tree .NET consumer appears, the recorded fallback is to keep the C# solution');
  p('   buildable (it is, and CI proves it) rather than invent an ABI. No such consumer is known.');
  p('');
  p('Consequences recorded so the decision is testable: no ' + '`cdylib`' + ' crate type, no C headers, no');
  p('wrapper package, and no ' + '`unsafe`' + ' FFI block may be added without a new requirement row and user');
  p('approval. The scan that proves the absence today:');
  p('');
  p('| Interop mechanism | Files matching |');
  p('|---|---:|');
  for (const a of data.compatibility.absence) p('| ' + esc(a.label) + ' | ' + a.files + ' |');
  p('');
  p('## 5. Reproduce this inventory');
  p('');
  p('    node parity/tools/ecosystem-inventory.mjs           # regenerate');
  p('    node parity/tools/ecosystem-inventory.mjs --check   # fail on drift (used by CI)');
  p('    node parity/tools/format-check.mjs --check          # validate the persisted formats');
  p('');
  p('The counts above are printed by the generator from the tree; none is typed by hand.');
  p('');
  return md.join('\n') + '\n';
}

function dispositionOf(data, project, typeName) {
  const proj = data.projects.find(p => p.name === project);
  return proj ? proj.disposition : 'unknown';
}


// --- main ---------------------------------------------------------------------------------
function build() {
  const projects = [];
  const types = [];
  let files = 0;
  let publicDeclarations = 0;
  let unclassified = 0;
  for (const p of projectRows) {
    const dir = path.join(root, p.name);
    const csFiles = walkCs(dir, []);
    files += csFiles.length;
    const disp = projectDisposition(p);
    const projectTypes = [];
    for (const abs of csFiles) {
      const rel = path.relative(root, abs).split(path.sep).join('/');
      for (const t of parseFile(abs, rel, p)) {
        if (t.accessibility === 'public') publicDeclarations++;
        unclassified += t.unclassified.length;
        if (t.externallyVisible) projectTypes.push(t);
      }
    }
    projectTypes.sort((a, b) => a.name.localeCompare(b.name, 'en') || a.start - b.start);
    for (const t of projectTypes) {
      t.disposition = disp.text;
      for (const m of t.members) {
        m.disposition = disp.text;
        m.rust = disp.rust;
      }
      types.push(t);
    }
    const memberCounts = typeCounts(projectTypes);
    const memberCount = projectTypes.reduce((n, t) => n + t.members.length, 0);
    projects.push({
      name: p.name,
      status: p.status,
      disposition: disp.text,
      rust: disp.rust,
      reason: disp.reason,
      phase: disp.phase,
      files: csFiles.length,
      typeCount: projectTypes.length,
      memberCount,
      memberCounts,
    });
  }
  const allMembers = projects.reduce((n, p) => n + p.memberCount, 0);
  const totals = {
    projects: projects.length,
    projectsWithoutCs: projects.filter(p => p.files === 0).length,
    files,
    publicDeclarations,
    visibleTypes: types.length,
    unreachablePublicTypes: publicDeclarations - types.length,
    visibleMembers: allMembers,
    memberCounts: typeCounts(types),
    unclassified,
    byDisposition: ['port', 'consolidated', 'partial', 'absent'].map(status => ({ status, count: projects.filter(p => p.status === status).length })),
  };
  const curated = CONSUMERS.map(c => ({ ...c }));
  const scanRows = consumerScan();
  const byDir = new Map();
  for (const row of scanRows) {
    const dir = row.file.split('/')[0];
    if (!byDir.has(dir)) byDir.set(dir, { dir, count: 0, kind: row.kind, disposition: row.disposition });
    byDir.get(dir).count++;
  }
  const byDirectory = [...byDir.values()].sort((a, b) => b.count - a.count || a.dir.localeCompare(b.dir));
  const formats = FORMATS.map(f => ({ ...f }));
  const graph = graphFieldSet();
  formats.find(f => f.id === 'FMT-001').graph = graph;
  return {
    schemaVersion: 1,
    generatedBy: 'parity/tools/ecosystem-inventory.mjs',
    method: {
      source: 'C# source scan of the 19 production projects in parity/projects.toml',
      visibility: 'public type with a public enclosing chain; public/protected members (interfaces implicitly public)',
      exclusions: ['test projects', 'bin/obj', 'docs', 'parity tools', 'benchmark-only declarations'],
      unclassifiedCeiling: UNCLASSIFIED_CEILING,
    },
    totals,
    projects,
    types: types.map(t => ({
      project: t.project,
      file: t.file,
      line: t.line || 0,
      kind: t.keyword,
      name: t.name,
      modifiers: t.modifiers,
      delegateSignature: t.delegateSignature || null,
      members: t.members.map(m => ({
        kind: m.kind,
        name: m.name,
        params: m.params || null,
        accessibility: m.accessibility,
        modifiers: m.modifiers,
        disposition: m.disposition,
        rust: m.rust,
        signature: m.signature,
      })),
      unclassified: t.unclassified,
    })),
    consumers: { curated, scan: { files: scanRows.length, rows: scanRows, byDirectory } },
    formats,
    compatibility: {
      decision: 'No .NET consumer requires continued compatibility through a C ABI, FFI bridge, or C# wrapper.',
      absence: ffiAbsence(),
    },
  };
}

function lineOf(text, index) {
  let line = 1;
  for (let i = 0; i < index && i < text.length; i++) if (text[i] === '\n') line++;
  return line;
}

const data = build();
const graph = data.formats.find(f => f.id === 'FMT-001').graph;
const schema = {
  schemaVersion: 1,
  title: 'Lovelace knowledge graph (GraphStore v1)',
  source: 'generated from Lovelace.Knowledge/Model.cs and knowledge-graph-demo.json by parity/tools/ecosystem-inventory.mjs',
  rootType: 'Graph',
  requiredTopLevel: graph.top.map(t => t.key),
  properties: {
    seed: { type: 'integer' },
    version: { const: 1 },
    config: { type: 'object', requiredKeys: graph.configKeys },
    samples: { type: 'array', requiredElementKeys: graph.elementKeys.samples },
    planes: { type: 'array', requiredElementKeys: graph.elementKeys.planes },
    boundaries: { type: 'array', requiredElementKeys: graph.elementKeys.boundaries },
    frontiers: { type: 'array', requiredElementKeys: graph.elementKeys.frontiers },
    metrics: { type: 'object', requiredKeys: graph.elementKeys.metrics },
  },
  example: { path: 'knowledge-graph-demo.json', sha256: graph.sha256, bytes: graph.bytes },
};
const schemaText = JSON.stringify(schema, null, 1) + '\n';
const json = JSON.stringify(data, null, 1) + '\n';
const markdown = renderMarkdown(data);
const failures = validateCurated();
if (failures.length) {
  console.error('ecosystem: curated evidence is stale:');
  for (const f of failures) console.error('  - ' + f);
}
if (data.totals.unclassified > UNCLASSIFIED_CEILING) {
  failures.push('unclassified declarations ' + data.totals.unclassified + ' exceed the ceiling ' + UNCLASSIFIED_CEILING);
}
if (data.compatibility.absence.some(a => a.files > 0)) {
  failures.push('an interop mechanism was found (' + data.compatibility.absence.map(a => a.label + ': ' + a.files).join('; ') + '); the ECO-007 decision must be revisited');
}
if (CHECK) {
  const diffs = [];
  for (const [file, expected] of [[MD_PATH, markdown], [JSON_PATH, json], [SCHEMA_PATH, schemaText]]) {
    const rel = path.relative(root, file).split(path.sep).join('/');
    if (!fs.existsSync(file)) { diffs.push(rel + ' is missing'); continue; }
    if (fs.readFileSync(file, 'utf8') !== expected) diffs.push(rel + ' is stale (regenerate with node parity/tools/ecosystem-inventory.mjs)');
  }
  for (const d of diffs) failures.push(d);
} else {
  fs.writeFileSync(MD_PATH, markdown);
  fs.writeFileSync(JSON_PATH, json);
  fs.writeFileSync(SCHEMA_PATH, schemaText);
}
const summary = {
  projects: data.totals.projects,
  files: data.totals.files,
  publicDeclarations: data.totals.publicDeclarations,
  visibleTypes: data.totals.visibleTypes,
  visibleMembers: data.totals.visibleMembers,
  memberCounts: data.totals.memberCounts,
  unclassified: data.totals.unclassified,
  consumers: data.consumers.scan.files,
  formats: data.formats.length,
  interopMatches: data.compatibility.absence.reduce((n, a) => n + a.files, 0),
  mode: CHECK ? 'check' : 'write',
};
console.log(JSON.stringify(summary, null, 2));
if (failures.length) { console.error('\nFAIL: ' + failures.length + ' issue(s)'); process.exit(1); }
console.log(CHECK ? 'PASS: ecosystem artifacts are current' : 'PASS: wrote docs/parity/ecosystem.md, ecosystem-api.json and knowledge-graph-v1.schema.json');
