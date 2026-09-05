// Render a markdown file (whose ```mermaid fences become diagrams) to a PDF.
//
// VERIFIED RECIPE (what worked, 2026-09, Windows + Chrome 1xx + Node 24):
//   1. one-time:  npm install mermaid --prefix out/graph-render --no-audit --no-fund
//   2. node Lovelace.Knowledge/tools/render-graph-pdf.mjs <input.md> [output.pdf]
//      (or: make graph-pdf)
//
// How it works:
//   - Builds a SELF-CONTAINED html with mermaid.min.js inlined.
//   - HTML-escapes every < > & in the mermaid blocks so '<br/>' survives as literal
//     text for mermaid to parse (the #1 gotcha).
//   - mermaid.initialize({ startOnLoad:true, securityLevel:'loose', theme:'default' }).
//   - Prints the html with headless Chrome/Edge via --print-to-pdf, using
//     --virtual-time-budget=25000 so mermaid's async render finishes before capture.
//
// Usage: node render-graph-pdf.mjs <input.md> [output.pdf]
import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import { spawnSync } from 'node:child_process';
import { pathToFileURL } from 'node:url';

const NL = String.fromCharCode(10);
const FENCE = '```';

const input = process.argv[2] || 'Lovelace.Knowledge/BEHAVIOR-GRAPH.md';
const output = process.argv[3] || input.replace(/\.md$/, '.pdf');

function mermaidSource() {
  const candidates = [
    'out/graph-render/node_modules/mermaid/dist/mermaid.min.js',
    'node_modules/mermaid/dist/mermaid.min.js',
  ];
  for (const c of candidates) {
    if (fs.existsSync(c)) return fs.readFileSync(c, 'utf8');
  }
  throw new Error('mermaid.min.js not found. Run: npm install mermaid --prefix out/graph-render --no-audit --no-fund');
}

function chromePath() {
  const candidates = [
    'C:/Program Files/Google/Chrome/Application/chrome.exe',
    'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe',
    'C:/Program Files/Microsoft/Edge/Application/msedge.exe',
  ];
  for (const c of candidates) if (fs.existsSync(c)) return c;
  throw new Error('No Chrome/Edge found for headless PDF printing.');
}

function esc(s) {
  return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
}

function toHtml(md, mmd) {
  const lines = md.split(NL);
  const body = [];
  let i = 0;
  while (i < lines.length) {
    const line = lines[i];
    if (line.startsWith(FENCE + 'mermaid')) {
      const buf = [];
      i++;
      while (i < lines.length && !lines[i].startsWith(FENCE)) { buf.push(lines[i]); i++; }
      i++;
      body.push('<pre class="mermaid">' + esc(buf.join(NL)) + '</pre>');
      continue;
    }
    if (line.startsWith('### ')) { body.push('<h3>' + esc(line.slice(4)) + '</h3>'); i++; continue; }
    if (line.startsWith('## ')) { body.push('<h2>' + esc(line.slice(3)) + '</h2>'); i++; continue; }
    if (line.startsWith('# ')) { body.push('<h1>' + esc(line.slice(2)) + '</h1>'); i++; continue; }
    if (line.trim() === '') { i++; continue; }
    const buf = [];
    while (i < lines.length && lines[i].trim() !== '' && !lines[i].startsWith(FENCE) && lines[i][0] !== '#') {
      buf.push(lines[i]); i++;
    }
    body.push('<p>' + esc(buf.join(' ')) + '</p>');
  }

  const style =
    'body{font-family:"Segoe UI",-apple-system,Roboto,sans-serif;max-width:920px;margin:2rem auto;padding:0 1.4rem;color:#16181d;line-height:1.55;}' +
    'h1{font-size:1.9rem;border-bottom:3px solid #0a7a4a;padding-bottom:.45rem;}' +
    'h2{font-size:1.35rem;margin-top:2.3rem;color:#0a7a4a;}' +
    'h3{font-size:1.1rem;color:#333;}' +
    'pre.mermaid{background:#fbfcfd;border:1px solid #e3e7ec;border-radius:8px;padding:1.1rem 1.2rem;overflow:hidden;page-break-inside:avoid;}' +
    'svg{max-width:100%;height:auto;}' +
    'p{max-width:70ch;}';

  return (
    '<!DOCTYPE html><html><head><meta charset="utf-8"><title>Behavior Graph</title>' +
    '<style>' + style + '</style>' +
    '<script>' + mmd + '</script></head><body>' +
    body.join(NL) +
    '<script>mermaid.initialize({startOnLoad:true,theme:"default",securityLevel:"loose",flowchart:{useMaxWidth:true},pie:{useMaxWidth:true}});</script>' +
    '</body></html>'
  );
}

const md = fs.readFileSync(input, 'utf8');
const mmd = mermaidSource();
const html = toHtml(md, mmd);

const htmlPath = path.join(os.tmpdir(), 'behavior-graph-' + Date.now() + '.html');
fs.writeFileSync(htmlPath, html);

const chrome = chromePath();
const r = spawnSync(chrome, [
  '--headless=new',
  '--disable-gpu',
  '--no-sandbox',
  '--no-pdf-header-footer',
  '--virtual-time-budget=25000',
  '--print-to-pdf=' + path.resolve(output),
  pathToFileURL(htmlPath).href,
], { stdio: 'ignore' });

try { fs.unlinkSync(htmlPath); } catch (e) { /* temp file, ignore */ }

if (!fs.existsSync(output)) {
  console.error('PDF was not produced. Chrome exit status:', r.status);
  process.exit(1);
}
console.log('wrote ' + path.resolve(output) + ' (' + fs.statSync(output).size + ' bytes)');
