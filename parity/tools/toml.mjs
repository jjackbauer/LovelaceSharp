// Minimal TOML reader for the ledger files in this directory.
//
// Supported subset (deliberately small, so the parser cannot silently mis-read):
//   comments (# ...), [table], [[array.of.tables]], key = "string" | number | true/false |
//   ["a", "b"] with double-quoted strings, basic escapes (\\, \", \n, \t), and blank lines.
// Anything else is an error. The ledgers are generated and reviewed, so a narrow reader is safer
// than a permissive one.
export function parseToml(text) {
  const root = {};
  let table = root;
  const lines = text.split(/\r?\n/);
  for (let i = 0; i < lines.length; i++) {
    const raw = lines[i];
    const line = stripComment(raw).trim();
    if (!line) continue;
    const at = 'line ' + (i + 1) + ': ';
    if (line.startsWith('[[') && line.endsWith(']]')) {
      const name = line.slice(2, -2).trim();
      if (!isBareKey(name)) throw new Error(at + 'invalid array-table name ' + JSON.stringify(name));
      const parts = name.split('.');
      let parent = root;
      for (const p of parts.slice(0, -1)) {
        if (!(p in parent)) parent[p] = {};
        parent = parent[p];
      }
      const last = parts[parts.length - 1];
      if (!Array.isArray(parent[last])) parent[last] = [];
      const entry = {};
      parent[last].push(entry);
      table = entry;
      continue;
    }
    if (line.startsWith('[') && line.endsWith(']')) {
      const name = line.slice(1, -1).trim();
      if (!isBareKey(name)) throw new Error(at + 'invalid table name ' + JSON.stringify(name));
      const parts = name.split('.');
      let parent = root;
      for (const p of parts.slice(0, -1)) {
        if (!(p in parent)) parent[p] = {};
        parent = parent[p];
      }
      const last = parts[parts.length - 1];
      if (!parent[last]) parent[last] = {};
      table = parent[last];
      continue;
    }
    const eq = indexOfUnquoted(line, '=');
    if (eq < 0) throw new Error(at + 'expected key = value, got ' + JSON.stringify(raw));
    const key = line.slice(0, eq).trim();
    if (!isBareKey(key)) throw new Error(at + 'invalid key ' + JSON.stringify(key));
    table[key] = parseValue(line.slice(eq + 1).trim(), at);
  }
  return root;
}

function isBareKey(s) { return /^[A-Za-z0-9_-]+$/.test(s); }

function stripComment(line) {
  let inString = false;
  for (let i = 0; i < line.length; i++) {
    const c = line[i];
    if (c === '"' && line[i - 1] !== '\\') inString = !inString;
    if (c === '#' && !inString) return line.slice(0, i);
  }
  return line;
}

function indexOfUnquoted(line, target) {
  let inString = false;
  for (let i = 0; i < line.length; i++) {
    const c = line[i];
    if (c === '"' && line[i - 1] !== '\\') inString = !inString;
    if (c === target && !inString) return i;
  }
  return -1;
}

function parseValue(text, at) {
  if (text.startsWith('"')) {
    if (!text.endsWith('"') || text.length < 2) throw new Error(at + 'unterminated string');
    return unescape(text.slice(1, -1));
  }
  if (text.startsWith('[')) {
    if (!text.endsWith(']')) throw new Error(at + 'unterminated array');
    const inner = text.slice(1, -1).trim();
    if (!inner) return [];
    const out = [];
    let depth = 0, start = 0, inString = false;
    for (let i = 0; i < inner.length; i++) {
      const c = inner[i];
      if (c === '"' && inner[i - 1] !== '\\') inString = !inString;
      if (!inString && c === '[') depth++;
      if (!inString && c === ']') depth--;
      if (!inString && depth === 0 && c === ',') { out.push(parseValue(inner.slice(start, i).trim(), at)); start = i + 1; }
    }
    out.push(parseValue(inner.slice(start).trim(), at));
    return out;
  }
  if (text === 'true') return true;
  if (text === 'false') return false;
  if (/^-?\d+$/.test(text)) return Number(text);
  throw new Error(at + 'unsupported value ' + JSON.stringify(text));
}

function unescape(s) {
  return s.replace(/\\(["\\nrt])/g, (_, c) => (c === 'n' ? '\n' : c === 'r' ? '\r' : c === 't' ? '\t' : c));
}
