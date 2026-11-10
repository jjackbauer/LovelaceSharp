#!/usr/bin/env node
// Extracts the executable documentation corpus from the Markdown references into a
// language-neutral JSON file that both implementations consume.
//
//   node parity/extract-corpus.mjs
//
// A `lovelace` fence must be followed by a `result` fence; anything else is a hard error,
// because a silently skipped example shrinks the parity gate without anyone noticing.

import { readFileSync, writeFileSync, mkdirSync } from "node:fs";
import { dirname, resolve } from "node:path";

const SOURCES = [
  { path: "Lovelace.Suite/docs/Language.md", prefix: "lang", out: "parity/corpus/lang.json" },
  { path: "Lovelace.Symbolics/README.md", prefix: "sym", out: "parity/corpus/symbolics.json" },
];

function parse(text, prefix, docPath) {
  const lines = text.split(/\r?\n/);
  const out = [];
  let heading = "(top)";
  let i = 0;
  while (i < lines.length) {
    const t = lines[i].trim();
    if (t.startsWith("#")) {
      heading = t.replace(/^#+\s*/, "");
      i++;
      continue;
    }
    if (t === "```lovelace") {
      const scriptLines = [];
      i++;
      while (i < lines.length && lines[i].trim() !== "```") scriptLines.push(lines[i++]);
      if (i >= lines.length) throw new Error(docPath + ": unterminated lovelace fence");
      i++;
      let j = i;
      while (j < lines.length && lines[j].trim() === "") j++;
      if (j >= lines.length || lines[j].trim() !== "```result") {
        throw new Error(docPath + ": lovelace fence under '" + heading + "' has no result fence");
      }
      j++;
      const body = [];
      while (j < lines.length && lines[j].trim() !== "```") body.push(lines[j++]);
      if (j >= lines.length) throw new Error(docPath + ": unterminated result fence");
      i = j + 1;
      const expected = body.join("\n").trim();
      let kind = "value";
      if (expected.startsWith("error: ")) kind = "error";
      else if (expected.startsWith("plot: ")) kind = "plot";
      else if (expected.startsWith("prints: ")) kind = "prints";
      out.push({
        id: prefix + "/" + String(out.length + 1).padStart(3, "0"),
        doc: docPath,
        heading,
        script: scriptLines.join("\n").trim(),
        kind,
        text: kind === "value" ? expected : expected.slice(kind.length + 2),
      });
      continue;
    }
    i++;
  }
  return out;
}

let total = 0;
for (const src of SOURCES) {
  const text = readFileSync(resolve(src.path), "utf8");
  const examples = parse(text, src.prefix, src.path);
  if (examples.length === 0) throw new Error(src.path + ": produced an empty corpus");
  mkdirSync(dirname(resolve(src.out)), { recursive: true });
  writeFileSync(resolve(src.out), JSON.stringify({ version: 1, sources: [src.path], examples }, null, 1) + "\n");
  console.log(src.path + " -> " + src.out + " (" + examples.length + " examples)");
  total += examples.length;
}
if (total !== 214) {
  console.error("expected 214 examples across both documents, found " + total);
  process.exit(1);
}
console.log("total: " + total);
