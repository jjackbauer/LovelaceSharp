// Shared JSON reader. Files in this repository may carry a UTF-8 BOM (a .NET writer adds one by
// default); JSON.parse rejects it, so strip exactly one leading BOM before parsing.
import fs from 'node:fs';

export function readJson(file) {
  const text = fs.readFileSync(file, 'utf8');
  return JSON.parse(text.charCodeAt(0) === 0xFEFF ? text.slice(1) : text);
}
