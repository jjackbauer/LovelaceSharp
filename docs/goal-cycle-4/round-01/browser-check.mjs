// Cycle-4 Blocker 3: drive the real Studio UI in a real browser over the Chrome DevTools
// Protocol. Node 24 has a global WebSocket and fetch, so no packages are needed.
//
// What it proves, in order:
//   1. the page loads and app.js (a module) executes without an uncaught exception;
//   2. CodeMirror mounted (#editor -> .cm-editor), i.e. the editor pane really initialised;
//   3. a real quick-eval typed into #quick-eval and submitted with Enter travels
//      app.js -> fetch -> /api/evaluate -> engine -> back into the #logs DOM;
//   4. every console message and page exception is captured verbatim;
//   5. a screenshot is written so a human can look at the rendered panels.
import { writeFileSync } from 'node:fs';

const PORT = process.argv[2] || '9223';
const URL_UNDER_TEST = process.argv[3] || 'http://127.0.0.1:5199/';
const OUT = process.argv[4] || '.';
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

async function waitForDevtools() {
  for (let i = 0; i < 80; i++) {
    try {
      const r = await fetch(`http://127.0.0.1:${PORT}/json/version`);
      if (r.ok) return await r.json();
    } catch { /* chrome not up yet */ }
    await sleep(250);
  }
  throw new Error('devtools endpoint never came up on port ' + PORT);
}

const version = await waitForDevtools();
const list = await (await fetch(`http://127.0.0.1:${PORT}/json/list`)).json();
let page = list.find((t) => t.type === 'page');
if (!page) throw new Error('no page target');

const ws = new WebSocket(page.webSocketDebuggerUrl);
await new Promise((res, rej) => { ws.onopen = res; ws.onerror = rej; });

let nextId = 1;
const pending = new Map();
const consoleMessages = [];
const exceptions = [];
const logEntries = [];

ws.onmessage = (ev) => {
  const msg = JSON.parse(ev.data);
  if (msg.id && pending.has(msg.id)) {
    const { resolve, reject } = pending.get(msg.id);
    pending.delete(msg.id);
    msg.error ? reject(new Error(JSON.stringify(msg.error))) : resolve(msg.result);
    return;
  }
  if (msg.method === 'Runtime.consoleAPICalled') {
    consoleMessages.push({
      type: msg.params.type,
      text: (msg.params.args || []).map((a) => a.value ?? a.description ?? a.type).join(' '),
    });
  } else if (msg.method === 'Runtime.exceptionThrown') {
    const d = msg.params.exceptionDetails;
    exceptions.push({
      text: d.text,
      description: d.exception?.description ?? '',
      url: d.url, line: d.lineNumber,
    });
  } else if (msg.method === 'Log.entryAdded') {
    logEntries.push({ level: msg.params.entry.level, text: msg.params.entry.text, source: msg.params.entry.source });
  }
};

function send(method, params = {}) {
  const id = nextId++;
  return new Promise((resolve, reject) => {
    pending.set(id, { resolve, reject });
    ws.send(JSON.stringify({ id, method, params }));
  });
}

async function evaluate(expression) {
  const r = await send('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true });
  if (r.exceptionDetails) throw new Error('eval threw: ' + JSON.stringify(r.exceptionDetails));
  return r.result.value;
}

await send('Runtime.enable');
await send('Log.enable');
await send('Page.enable');

await send('Page.navigate', { url: URL_UNDER_TEST });
await sleep(4000);   // module fetch + CodeMirror mount + the app's own startup calls

const afterLoad = await evaluate(`(() => {
  const q = (s) => document.querySelector(s);
  const panes = Array.from(document.querySelectorAll('.pane')).map((p) => ({
    id: p.id,
    visible: !!(p.offsetWidth || p.offsetHeight),
  }));
  return {
    title: document.title,
    readyState: document.readyState,
    codeMirrorMounted: !!q('#editor .cm-editor'),
    editorPaneVisible: panes.find((p) => p.id === 'editor-pane')?.visible ?? false,
    panes,
    status: q('#status')?.textContent ?? null,
    precision: q('#precision-readout')?.textContent ?? null,
    runButton: !!q('#run-btn'),
    variablesTable: !!q('#variables-table'),
    functionsTable: !!q('#functions-table'),
    graphPlaceholder: q('#graph-placeholder')?.textContent ?? null,
    logsText: q('#logs')?.innerText ?? null,
    stylesheetApplied: getComputedStyle(document.body).margin !== '',
    toolbarBg: getComputedStyle(q('#toolbar')).backgroundColor,
  };
})()`);

// A real quick-eval: focus the input, type into it, press Enter.
await evaluate(`document.querySelector('#quick-eval').focus()`);
await send('Input.insertText', { text: '2^10 + 1' });
const typed = await evaluate(`document.querySelector('#quick-eval').value`);
await send('Input.dispatchKeyEvent', { type: 'keyDown', key: 'Enter', code: 'Enter', windowsVirtualKeyCode: 13, nativeVirtualKeyCode: 13 });
await send('Input.dispatchKeyEvent', { type: 'keyUp', key: 'Enter', code: 'Enter', windowsVirtualKeyCode: 13, nativeVirtualKeyCode: 13 });
await sleep(6000);

const afterEval = await evaluate(`(() => {
  const q = (s) => document.querySelector(s);
  const logs = q('#logs');
  return {
    logsText: logs?.innerText ?? null,
    logChildCount: logs?.children.length ?? 0,
    status: q('#status')?.textContent ?? null,
    variablesBody: q('#variables-table tbody')?.innerText ?? null,
    variablesEmptyHidden: q('#variables-empty') ? q('#variables-empty').hidden : null,
  };
})()`);

// A second, symbolic one-liner, to reach the kernel rather than the numeric core.
await evaluate(`(() => { const i = document.querySelector('#quick-eval'); i.value = ''; i.focus(); return true; })()`);
await send('Input.insertText', { text: 'x = symbol("x"); factor(x^2 - 1)' });
await send('Input.dispatchKeyEvent', { type: 'keyDown', key: 'Enter', code: 'Enter', windowsVirtualKeyCode: 13, nativeVirtualKeyCode: 13 });
await send('Input.dispatchKeyEvent', { type: 'keyUp', key: 'Enter', code: 'Enter', windowsVirtualKeyCode: 13, nativeVirtualKeyCode: 13 });
await sleep(6000);
const afterSymbolic = await evaluate(`(() => {
  const q = (s) => document.querySelector(s);
  return {
    logsText: q('#logs')?.innerText ?? null,
    variablesBody: q('#variables-table tbody')?.innerText ?? null,
    symbolicInspect: q('#symbolic-inspect')?.textContent?.slice(0, 400) ?? null,
  };
})()`);

const shot = await send('Page.captureScreenshot', { format: 'png', captureBeyondViewport: true });
writeFileSync(`${OUT}/studio-screenshot.png`, Buffer.from(shot.data, 'base64'));

const report = {
  browser: version.Browser,
  protocolVersion: version['Protocol-Version'],
  url: URL_UNDER_TEST,
  afterLoad,
  quickEval: { typed, ...afterEval },
  symbolicEval: afterSymbolic,
  consoleMessages,
  exceptions,
  logEntries,
  consoleErrorCount: consoleMessages.filter((m) => m.type === 'error').length + logEntries.filter((l) => l.level === 'error').length,
  exceptionCount: exceptions.length,
};
writeFileSync(`${OUT}/browser-check.json`, JSON.stringify(report, null, 2));
console.log(JSON.stringify({
  browser: report.browser,
  codeMirrorMounted: afterLoad.codeMirrorMounted,
  panes: afterLoad.panes.length,
  quickEvalTyped: typed,
  quickEvalLogs: afterEval.logsText,
  symbolicLogs: afterSymbolic.logsText,
  variablesBody: afterSymbolic.variablesBody,
  consoleErrorCount: report.consoleErrorCount,
  exceptionCount: report.exceptionCount,
  consoleMessages,
  exceptions,
}, null, 2));
ws.close();
