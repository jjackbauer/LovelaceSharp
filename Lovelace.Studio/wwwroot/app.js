import { EditorView, keymap, Decoration, StateEffect, StateField, basicSetup, autocompletion, completionKeymap, oneDark } from "./vendor/codemirror.js";

const $ = (sel) => document.querySelector(sel);

const SESSION_KEY = "lovelace.studio.session";
const EDITOR_KEY = "lovelace.studio.editor";

let sessionId = sessionStorage.getItem(SESSION_KEY) || null;
let completionCatalog = [];
let currentRunId = null;
let pollTimer = null;
let renderedSteps = 0;

const logsEl = $("#logs");
const variablesTbody = $("#variables-table tbody");
const variablesEmpty = $("#variables-empty");
const functionsTbody = $("#functions-table tbody");
const functionsEmpty = $("#functions-empty");
const graphContainer = $("#graph-container");
const graphPlaceholder = $("#graph-placeholder");
const statusEl = $("#status");
const precisionReadout = $("#precision-readout");
const precisionInput = $("#precision-input");
const quickEval = $("#quick-eval");
const cursorPos = $("#cursor-pos");

// ---------------------------------------------------------------------------
// API helper (session-scoped)
// ---------------------------------------------------------------------------

async function api(path, options) {
  const opts = options || {};
  opts.headers = Object.assign({}, opts.headers || {}, { "X-Session-Id": sessionId || "" });
  return fetch(path, opts);
}

// ---------------------------------------------------------------------------
// Session
// ---------------------------------------------------------------------------

function setPrecisionReadout(precision) {
  precisionReadout.textContent = precision + " digits";
  precisionInput.placeholder = String(precision);
}

async function ensureSession() {
  if (sessionId) {
    const res = await api("/api/session");
    if (res.ok) {
      const data = await res.json();
      setPrecisionReadout(data.precision);
      return;
    }
    sessionId = null;
    sessionStorage.removeItem(SESSION_KEY);
  }
  const res = await fetch("/api/session", { method: "POST" });
  if (!res.ok) throw new Error("could not create session");
  const data = await res.json();
  sessionId = data.sessionId;
  sessionStorage.setItem(SESSION_KEY, sessionId);
  setPrecisionReadout(data.precision);
}

// ---------------------------------------------------------------------------
// CodeMirror editor + autocomplete
// ---------------------------------------------------------------------------

const setErrorLine = StateEffect.define();
const errorLineField = StateField.define({
  create: () => Decoration.none,
  update: (deco, tr) => {
    deco = deco.map(tr.changes);
    for (const e of tr.effects) if (e.is(setErrorLine)) deco = e.value;
    return deco;
  },
  provide: (f) => EditorView.decorations.from(f)
});

function completionType(kind) {
  if (kind === "builtin" || kind === "function") return "function";
  if (kind === "variable") return "variable";
  return "keyword";
}

function completionSource(context) {
  const word = context.matchBefore(/[A-Za-z_][A-Za-z0-9_]*/);
  if (!word || (word.from === word.to && !context.explicit)) return null;
  const prefix = word.text.toLowerCase();
  return {
    from: word.from,
    options: completionCatalog
      .filter((c) => c.label.toLowerCase().startsWith(prefix))
      .map((c) => ({ label: c.label, type: completionType(c.kind), detail: c.detail, apply: c.label }))
  };
}

const editor = new EditorView({
  parent: $("#editor"),
  extensions: [
    basicSetup,
    oneDark,
    keymap.of([...completionKeymap, { key: "Ctrl-Enter", run: () => { run(); return true; } }]),
    autocompletion({ override: [completionSource] }),
    errorLineField,
    EditorView.updateListener.of((u) => {
      if (u.selectionSet || u.docChanged) {
        const pos = u.state.selection.main.head;
        const line = u.state.doc.lineAt(pos);
        cursorPos.textContent = "Ln " + line.number + ", Col " + (pos - line.from + 1);
      }
      if (u.docChanged) localStorage.setItem(EDITOR_KEY, u.state.doc.toString());
    })
  ]
});

function editorValue() { return editor.state.doc.toString(); }
function setEditorValue(text) {
  editor.dispatch({ changes: { from: 0, to: editor.state.doc.length, insert: text } });
}

function highlightError(line) {
  const doc = editor.state.doc;
  const ln = Math.max(1, Math.min(line, doc.lines));
  const from = doc.line(ln).from;
  const to = doc.line(ln).to;
  editor.dispatch({ effects: setErrorLine.of(Decoration.line({ class: "cm-error-line" }).range(from, to)) });
}
function clearError() {
  editor.dispatch({ effects: setErrorLine.of(Decoration.none) });
}

// ---------------------------------------------------------------------------
// Logs
// ---------------------------------------------------------------------------

function appendLog(text, kind) {
  if (text === undefined || text === null || text === "") return;
  const div = document.createElement("div");
  div.className = "log-line" + (kind ? " " + kind : "");
  div.textContent = text;
  logsEl.appendChild(div);
  logsEl.scrollTop = logsEl.scrollHeight;
}

// ---------------------------------------------------------------------------
// Rendering
// ---------------------------------------------------------------------------

function renderVariables(vars) {
  variablesTbody.innerHTML = "";
  const list = vars || [];
  variablesEmpty.hidden = list.length > 0;
  for (const v of list) {
    const tr = document.createElement("tr");
    const nameTd = document.createElement("td"); nameTd.textContent = v.name;
    const valueTd = document.createElement("td"); valueTd.textContent = v.display; valueTd.title = v.display;
    const kindTd = document.createElement("td"); kindTd.textContent = v.kind; kindTd.className = "kind";
    const delTd = document.createElement("td");
    const delBtn = document.createElement("button");
    delBtn.textContent = "×"; delBtn.className = "row-delete"; delBtn.title = "Delete " + v.name;
    delBtn.addEventListener("click", () => deleteVariable(v.name));
    delTd.appendChild(delBtn);
    tr.append(nameTd, valueTd, kindTd, delTd);
    variablesTbody.appendChild(tr);
  }
}

function renderFunctions(fns) {
  functionsTbody.innerHTML = "";
  const list = fns || [];
  functionsEmpty.hidden = list.length > 0;
  for (const f of list) {
    const tr = document.createElement("tr");
    const nameTd = document.createElement("td"); nameTd.textContent = f.name;
    const paramsTd = document.createElement("td"); paramsTd.textContent = (f.parameters || []).join(", ");
    const kindTd = document.createElement("td");
    kindTd.textContent = f.plugin || (f.isBuiltin ? "builtin" : "user");
    kindTd.className = "kind " + (f.plugin ? "plugin" : f.isBuiltin ? "builtin" : "user");
    tr.append(nameTd, paramsTd, kindTd);
    functionsTbody.appendChild(tr);
  }
}

function renderPlot(plot) {
  const existing = graphContainer.querySelector(".graph-svg");
  if (existing) existing.remove();
  if (plot && plot.svg) {
    graphPlaceholder.style.display = "none";
    const wrap = document.createElement("div");
    wrap.className = "graph-svg";
    wrap.innerHTML = plot.svg;
    graphContainer.appendChild(wrap);
  } else {
    graphPlaceholder.style.display = "";
  }
}

function setStatus(revision) { statusEl.textContent = "rev " + revision; }

// Apply the workspace portion of a response (variables/functions/plot/status/precision).
function applyWorkspace(data) {
  if (!data) return;
  renderVariables(data.variables);
  renderFunctions(data.functions);
  renderPlot(data.plot);
  setStatus(data.revision);
  if (data.precision !== undefined) setPrecisionReadout(data.precision);
}

// Append only newly-seen step outputs (dedupes across polls).
function appendNewSteps(steps) {
  for (let i = renderedSteps; i < steps.length; i++) {
    if (steps[i].output) appendLog(steps[i].output, "output");
  }
  renderedSteps = steps.length;
}

function applyFinal(data) {
  if (data.result && data.result.kind !== "Void") renderStructuredResult(data.result);
  if (data.diagnostics && data.diagnostics.length > 0) {
    const d = data.diagnostics[0];
    appendLog("error: " + d.message + " (line " + d.line + ", col " + d.column + ")", "error");
    if (d.line) highlightError(d.line);
  }
  if (data.elapsed) {
    const t = data.elapsedTime ? " (" + data.elapsedTime.value + " " + data.elapsedTime.unit + ")" : "";
    appendLog("done: " + data.elapsed + t + " (" + (data.reusedCount || 0) + " reused)", "timing");
  }
  renderSymbolicPanel(data.result ? data.result.structured : null);
}

// ---------------------------------------------------------------------------
// Structured results. Everything below renders from the kernel's own structured
// payload (the same DTO the DSH runner emits); nothing is recovered by parsing a
// display string, and the inspected subject is the run's result — never a
// re-evaluation of the editor buffer.
// ---------------------------------------------------------------------------

// Field lookup on a structured record: { kind: "Record", type, fields: [{name, value}] }.
function field(sv, name) {
  if (!sv || sv.kind !== "Record" || !sv.fields) return null;
  const hit = sv.fields.find(f => f.name === name);
  return hit ? hit.value : null;
}

function scalarText(sv) {
  if (!sv) return "—";
  if (sv.kind === "Null") return "—";
  if (sv.value !== undefined && sv.value !== null) return String(sv.value);
  if (sv.pretty !== undefined && sv.pretty !== null) return sv.pretty;
  return sv.kind;
}

function el(tag, className, text) {
  const node = document.createElement(tag);
  if (className) node.className = className;
  if (text !== undefined && text !== null) node.textContent = text;
  return node;
}

// Structural rendering of an array: nested brackets from Shape/Elements, never flat JSON.
function renderArray(sv, into) {
  const shape = sv.shape || [];
  const elements = sv.elements || [];
  if (shape.length <= 1) {
    const row = el("div", "sv-row");
    row.appendChild(el("span", "sv-bracket", "["));
    elements.forEach((e, i) => {
      if (i) row.appendChild(el("span", "sv-sep", ", "));
      row.appendChild(structuredNode(e));
    });
    row.appendChild(el("span", "sv-bracket", "]"));
    into.appendChild(row);
    return;
  }
  const width = shape[shape.length - 1];
  const rows = width ? elements.length / width : 0;
  const table = el("div", "sv-matrix");
  for (let r = 0; r < rows; r++) {
    const row = el("div", "sv-row");
    row.appendChild(el("span", "sv-bracket", r === 0 ? "[" : " "));
    for (let c = 0; c < width; c++) {
      if (c) row.appendChild(el("span", "sv-sep", ", "));
      row.appendChild(structuredNode(elements[r * width + c]));
    }
    row.appendChild(el("span", "sv-bracket", r === rows - 1 ? "]" : " "));
    table.appendChild(row);
  }
  into.appendChild(table);
}

// One structured value as DOM. Records recurse, arrays render structurally, symbolic
// values show both print forms plus their domain/exactness, scalars show their kind.
function structuredNode(sv) {
  if (!sv) return el("span", "sv-null", "—");
  switch (sv.kind) {
    case "Record": {
      const box = el("div", "sv-record");
      box.appendChild(el("div", "sv-type", sv.type || "Record"));
      for (const f of sv.fields || []) {
        const line = el("div", "sv-field");
        line.appendChild(el("span", "sv-name", f.name + ": "));
        line.appendChild(structuredNode(f.value));
        box.appendChild(line);
      }
      return box;
    }
    case "Array": {
      const box = el("div", "sv-array");
      box.appendChild(el("span", "sv-meta", (sv.type || "Array") + " shape [" + (sv.shape || []).join(", ") + "]"));
      renderArray(sv, box);
      return box;
    }
    case "Symbolic": {
      const box = el("span", "sv-symbolic");
      box.appendChild(el("span", "sv-pretty", sv.pretty));
      if (sv.truncated) {
        box.appendChild(el("span", "sv-warn",
          " [abbreviated: " + sv.truncationReason + ", " + sv.nodeCount + " nodes, budget " + sv.budget + "]"));
      }
      if (sv.canonical) {
        const det = el("details", "sv-canonical");
        det.appendChild(el("summary", null, "canonical"));
        det.appendChild(el("code", null, sv.canonical));
        box.appendChild(det);
      }
      const meta = [];
      if (sv.domain) meta.push("domain " + sv.domain);
      if (sv.exact !== undefined && sv.exact !== null) meta.push(sv.exact ? "exact" : "approximate");
      if (sv.nodeCount !== undefined && sv.nodeCount !== null) meta.push(sv.nodeCount + " nodes");
      if (sv.freeSymbols && sv.freeSymbols.length) meta.push("free " + sv.freeSymbols.join(","));
      if (meta.length) box.appendChild(el("span", "sv-meta", " (" + meta.join(" · ") + ")"));
      return box;
    }
    default:
      return el("span", "sv-scalar sv-" + sv.kind.toLowerCase(), scalarText(sv));
  }
}

// The symbolic/detail panel. It reads the SAME structured payload as the result view:
// provenance from TransformResult.steps, the IR from CompilationResult.ir, conditions and
// solutions from the record's own fields. Nothing here re-evaluates the editor buffer.
function renderSymbolicPanel(sv) {
  const el0 = $("#symbolic-inspect");
  el0.textContent = "";
  if (!sv) { el0.textContent = "no structured result yet"; return; }
  if (sv.kind === "Symbolic") {
    el0.appendChild(named("pretty", el("code", null, sv.pretty)));
    if (sv.canonical) el0.appendChild(named("canonical", el("code", null, sv.canonical)));
    const meta = [];
    if (sv.domain) meta.push("domain " + sv.domain);
    if (sv.exact !== undefined && sv.exact !== null) meta.push(sv.exact ? "exact" : "approximate");
    if (sv.nodeCount !== undefined && sv.nodeCount !== null) meta.push(sv.nodeCount + " nodes");
    if (meta.length) el0.appendChild(el("div", "sv-meta", meta.join(" · ")));
    return;
  }
  if (sv.kind !== "Record") { el0.appendChild(named(sv.kind.toLowerCase(), structuredNode(sv))); return; }

  el0.appendChild(named("type", el("span", "sv-type", sv.type || "Record")));
  for (const [label, name] of [["pretty", "original"], ["value", "value"], ["expression", "expression"]]) {
    const v = field(sv, name);
    if (v && (v.kind === "Symbolic")) el0.appendChild(named(label, el("code", null, v.pretty)));
  }
  appendSection(el0, sv, "conditions", "conditions");
  appendSection(el0, sv, "common_conditions", "common conditions");
  appendSection(el0, sv, "left_conditions", "left conditions");
  appendSection(el0, sv, "right_conditions", "right conditions");
  appendSection(el0, sv, "assumptions", "assumptions");
  appendSection(el0, sv, "families", "families");

  const solutions = field(sv, "solutions");
  if (solutions && solutions.kind === "Array" && (solutions.elements || []).length) {
    el0.appendChild(named("solutions (" + solutions.elements.length + ")", structuredNode(solutions)));
  }
  const steps = field(sv, "steps");
  if (steps && steps.kind === "Array" && (steps.elements || []).length) {
    const box = el("div", "sv-provenance");
    box.appendChild(el("div", "sv-name", "provenance (" + steps.elements.length + " steps)"));
    for (const step of steps.elements) {
      const line = el("div", "sv-step");
      line.appendChild(el("span", "sv-rule", scalarText(field(step, "rule_id")) + " "));
      line.appendChild(el("span", "sv-class", scalarText(field(step, "classification")) + " "));
      line.appendChild(el("span", null,
        scalarText(field(step, "before")) + " → " + scalarText(field(step, "after"))));
      const required = field(step, "required_conditions");
      if (required && required.kind === "Array" && (required.elements || []).length) {
        line.appendChild(el("span", "sv-meta", " requires " + required.elements.map(scalarText).join(", ")));
      }
      box.appendChild(line);
    }
    el0.appendChild(box);
  }
  const ir = field(sv, "ir");
  if (ir) el0.appendChild(named("mathir", el("code", null, scalarText(ir))));
  appendSection(el0, sv, "diagnostics", "diagnostics");
  appendSection(el0, sv, "members", "members");
}

function named(label, node) {
  const box = el("div", "sv-section");
  box.appendChild(el("span", "sv-name", label + ": "));
  box.appendChild(node);
  return box;
}

function appendSection(host, sv, name, label) {
  const v = field(sv, name);
  if (!v || v.kind === "Null") return;
  if (v.kind === "Array" && !(v.elements || []).length) return;
  host.appendChild(named(label, v.kind === "Array" ? arrayInline(v) : structuredNode(v)));
}

function arrayInline(sv) {
  const box = el("span", "sv-inline-array");
  (sv.elements || []).forEach((e, i) => {
    if (i) box.appendChild(el("span", "sv-sep", "; "));
    box.appendChild(e.kind === "Symbolic" ? el("code", null, e.pretty) : structuredNode(e));
  });
  return box;
}

// The result line: type name, display form, and the structural view underneath.
function renderStructuredResult(result) {
  const line = el("div", "log-line result");
  line.appendChild(el("span", "sv-typename", (result.typeName || result.structured && result.structured.type || result.kind) + " "));
  line.appendChild(el("span", null, "= " + result.typed));
  logsEl.appendChild(line);
  if (result.structured) {
    const box = el("div", "log-line structured");
    box.appendChild(structuredNode(result.structured));
    logsEl.appendChild(box);
  }
  logsEl.scrollTop = logsEl.scrollHeight;
}

// ---------------------------------------------------------------------------
// Progress (inline in the toolbar)
// ---------------------------------------------------------------------------

function showProgress() { $("#toolbar-progress").hidden = false; }
function hideProgress() { $("#toolbar-progress").hidden = true; }

function updateProgress(data) {
  const total = data.totalStatements || 0;
  const done = data.completedStatements || 0;
  const sub = (data.subProgress !== null && data.subProgress !== undefined) ? data.subProgress : 0;
  const pct = total > 0 ? Math.round(((done + sub) / total) * 100) : 0;
  $("#progress-fill").style.width = pct + "%";
  let label = "";
  if (data.currentLabel) label = "step " + (data.currentIndex + 1) + "/" + total + ": " + data.currentLabel;
  if (data.subLabel && data.subProgress !== null && data.subProgress !== undefined) {
    label += " — " + data.subLabel + " " + Math.round(data.subProgress * 100) + "%";
  }
  if (data.reusedCount) label += " · " + data.reusedCount + " reused";
  $("#progress-label").textContent = label;
  $("#progress-label").title = label;
}

function finishPoll() {
  currentRunId = null;
  if (pollTimer) { clearTimeout(pollTimer); pollTimer = null; }
  setTimeout(hideProgress, 500);
}

// ---------------------------------------------------------------------------
// Run (async: start then poll)
// ---------------------------------------------------------------------------

async function pollRun(runId) {
  try {
    const res = await api("/api/run/" + runId);
    if (res.status === 404) { finishPoll(); return; }
    const data = await res.json();
    applyWorkspace(data.response);
    appendNewSteps(data.response.timings || []);
    updateProgress(data);
    if (data.status === "finished" || data.status === "error" || data.status === "cancelled") {
      applyFinal(data.response);
      loadCompletions();
      finishPoll();
      return;
    }
    pollTimer = setTimeout(() => pollRun(runId), 150);
  } catch (err) {
    appendLog("poll failed: " + err, "error");
    finishPoll();
  }
}

async function runSource(source) {
  if (currentRunId) return;
  if (!source.trim()) return;
  try {
    const res = await api("/api/evaluate", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ source })
    });
    if (res.status === 409) { appendLog("a run is already in progress", "error"); return; }
    if (!res.ok) { appendLog("HTTP " + res.status, "error"); return; }
    const data = await res.json();
    currentRunId = data.runId;
    renderedSteps = 0;
    logsEl.innerHTML = "";
    clearError();
    showProgress();
    updateProgress({ totalStatements: 0, completedStatements: 0, reusedCount: 0 });
    pollRun(data.runId);
  } catch (err) {
    appendLog("run failed: " + err, "error");
  }
}

function run() { runSource(editorValue()); }

async function cancelRun() {
  if (!currentRunId) return;
  try { await api("/api/run/" + currentRunId + "/cancel", { method: "POST" }); }
  catch (err) { appendLog("cancel failed: " + err, "error"); }
}

// ---------------------------------------------------------------------------
// Precision
// ---------------------------------------------------------------------------

async function applyPrecision() {
  const digits = parseInt(precisionInput.value, 10);
  if (!digits || digits <= 0) { appendLog("precision must be a positive integer", "error"); return; }
  try {
    const res = await api("/api/precision", {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ digits })
    });
    if (!res.ok) { appendLog("set precision failed: HTTP " + res.status, "error"); return; }
    const data = await res.json();
    renderVariables(data.variables);
    renderFunctions(data.functions);
    setStatus(data.revision);
    setPrecisionReadout(data.precision);
    appendLog("precision set to " + data.precision, "info");
  } catch (err) { appendLog("set precision failed: " + err, "error"); }
}

// ---------------------------------------------------------------------------
// Value rendering mode (ASCII default / Unicode glyphs). The engine owns the
// setting; every value Studio renders goes through it.
// ---------------------------------------------------------------------------

async function applyUnicode(unicode) {
  try {
    const res = await api("/api/format", {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ unicode })
    });
    if (!res.ok) { appendLog("set format failed: HTTP " + res.status, "error"); return; }
    const data = await res.json();
    applyWorkspace(data);
    appendLog("rendering: " + (unicode ? "unicode" : "ascii"), "info");
  } catch (err) { appendLog("set format failed: " + err, "error"); }
}

// ---------------------------------------------------------------------------
// Workspace + completions API
// ---------------------------------------------------------------------------

async function deleteVariable(name) {
  try {
    const res = await api("/api/variables/" + encodeURIComponent(name), { method: "DELETE" });
    if (!res.ok) { appendLog("delete failed: HTTP " + res.status, "error"); return; }
    const data = await res.json();
    renderVariables(data.variables);
    renderFunctions(data.functions);
    setStatus(data.revision);
    appendLog("deleted " + name, "info");
  } catch (err) { appendLog("delete failed: " + err, "error"); }
}

async function clearWorkspace() {
  try {
    const res = await api("/api/state", { method: "DELETE" });
    if (!res.ok) { appendLog("clear failed: HTTP " + res.status, "error"); return; }
    const data = await res.json();
    renderVariables(data.variables);
    renderFunctions(data.functions);
    setStatus(data.revision);
    appendLog("workspace cleared (variables)", "info");
  } catch (err) { appendLog("clear failed: " + err, "error"); }
}

async function loadState() {
  try {
    const res = await api("/api/state");
    if (!res.ok) return;
    const data = await res.json();
    renderVariables(data.variables);
    renderFunctions(data.functions);
    setStatus(data.revision);
    setPrecisionReadout(data.precision);
  } catch (err) { /* ignore */ }
}

async function loadCompletions() {
  try {
    const res = await api("/api/completions");
    if (res.ok) completionCatalog = (await res.json()).items;
  } catch (err) { /* ignore */ }
}

// ---------------------------------------------------------------------------
// Editor persistence
// ---------------------------------------------------------------------------

function saveEditor() {
  localStorage.setItem(EDITOR_KEY, editorValue());
  appendLog("editor saved to localStorage", "info");
}

function loadEditor() {
  const value = localStorage.getItem(EDITOR_KEY);
  setEditorValue(value !== null ? value : "");
  appendLog(value !== null ? "editor loaded from localStorage" : "no saved editor content", "info");
}

// ---------------------------------------------------------------------------
// Wiring
// ---------------------------------------------------------------------------

$("#run-btn").addEventListener("click", () => run());
$("#clear-btn").addEventListener("click", clearWorkspace);
$("#save-btn").addEventListener("click", saveEditor);
$("#load-btn").addEventListener("click", loadEditor);
$("#precision-apply").addEventListener("click", applyPrecision);
$("#unicode-toggle").addEventListener("change", (e) => applyUnicode(e.target.checked));
$("#progress-cancel").addEventListener("click", cancelRun);

quickEval.addEventListener("keydown", (e) => {
  if (e.key === "Enter") {
    e.preventDefault();
    const src = quickEval.value.trim();
    quickEval.value = "";
    if (!src) return;
    appendLog(">> " + src, "echo");
    runSource(src);
  }
});

// ---------------------------------------------------------------------------
// Resizable dividers + minimize/maximize panes
// ---------------------------------------------------------------------------

const SIZES_KEY = "lovelace.studio.sizes";
const panes = Array.from(document.querySelectorAll(".pane"));
const workspacePane = document.getElementById("workspace-pane");
const graphPane = document.getElementById("graph-pane");
const logsPane = document.getElementById("logs-pane");

const MIN_WORKSPACE = 260;
const MIN_GRAPH = 120;
const MIN_LOGS = 80;

function paneIsMinimized(p) { return p.classList.contains("minimized"); }
function paneIsMaximized(p) { return p.classList.contains("maximized"); }

function syncPaneButtons(p) {
  const min = p.querySelector(".pane-min");
  const max = p.querySelector(".pane-max");
  if (min) {
    min.textContent = paneIsMinimized(p) ? "+" : "–";
    min.title = paneIsMinimized(p) ? "Restore" : "Minimize";
  }
  if (max) {
    max.textContent = paneIsMaximized(p) ? "■" : "□";
    max.title = paneIsMaximized(p) ? "Restore" : "Maximize";
  }
}

function toggleMinimize(p) {
  if (paneIsMaximized(p)) return;
  p.classList.toggle("minimized");
  syncPaneButtons(p);
}

function toggleMaximize(p) {
  if (paneIsMaximized(p)) {
    p.classList.remove("maximized");
    document.body.classList.remove("has-maximized", "max-group-main", "max-group-bottom");
  } else {
    p.classList.remove("minimized");
    p.classList.add("maximized");
    document.body.classList.add("has-maximized");
    document.body.classList.toggle("max-group-bottom", p.dataset.group === "bottom");
    document.body.classList.toggle("max-group-main", p.dataset.group === "main");
  }
  panes.forEach(syncPaneButtons);
}

for (const p of panes) {
  p.querySelector(".pane-min")?.addEventListener("click", (e) => { e.stopPropagation(); toggleMinimize(p); });
  p.querySelector(".pane-max")?.addEventListener("click", (e) => { e.stopPropagation(); toggleMaximize(p); });
  p.querySelector(".pane-header")?.addEventListener("click", (e) => {
    if (paneIsMinimized(p) && !e.target.closest("button") && e.target.tagName !== "INPUT") {
      toggleMinimize(p);
    }
  });
}

function startResize(divider, getSize, apply) {
  divider.addEventListener("mousedown", (e) => {
    if (document.body.classList.contains("has-maximized")) return;
    e.preventDefault();
    const startX = e.clientX;
    const startY = e.clientY;
    const start = getSize();
    document.body.classList.add("resizing");

    function onMove(ev) { apply(start, ev.clientX - startX, ev.clientY - startY); }
    function onUp() {
      document.body.classList.remove("resizing");
      document.removeEventListener("mousemove", onMove);
      document.removeEventListener("mouseup", onUp);
      saveSizes();
    }
    document.addEventListener("mousemove", onMove);
    document.addEventListener("mouseup", onUp);
  });
}

startResize(
  document.getElementById("divider-main"),
  () => workspacePane.getBoundingClientRect().width,
  (startW, dx) => { workspacePane.style.width = Math.max(MIN_WORKSPACE, startW - dx) + "px"; }
);

startResize(
  document.getElementById("divider-graph"),
  () => graphPane.getBoundingClientRect().height,
  (startH, _dx, dy) => { graphPane.style.height = Math.max(MIN_GRAPH, startH - dy) + "px"; }
);

startResize(
  document.getElementById("divider-logs"),
  () => logsPane.getBoundingClientRect().height,
  (startH, _dx, dy) => { logsPane.style.height = Math.max(MIN_LOGS, startH - dy) + "px"; }
);

function saveSizes() {
  localStorage.setItem(SIZES_KEY, JSON.stringify({
    workspace: workspacePane.style.width || workspacePane.getBoundingClientRect().width + "px",
    graph: graphPane.style.height || graphPane.getBoundingClientRect().height + "px",
    logs: logsPane.style.height || logsPane.getBoundingClientRect().height + "px"
  }));
}

function restoreSizes() {
  try {
    const sizes = JSON.parse(localStorage.getItem(SIZES_KEY) || "null");
    if (!sizes) return;
    if (sizes.workspace) workspacePane.style.width = sizes.workspace;
    if (sizes.graph) graphPane.style.height = sizes.graph;
    if (sizes.logs) logsPane.style.height = sizes.logs;
  } catch { /* ignore */ }
}

restoreSizes();

// ---------------------------------------------------------------------------
// Boot
// ---------------------------------------------------------------------------

async function boot() {
  const saved = localStorage.getItem(EDITOR_KEY);
  if (saved !== null) setEditorValue(saved);
  renderPlot(null);
  try {
    await ensureSession();
    await loadCompletions();
    await loadState();
    appendLog("Lovelace.Studio ready — Run to evaluate.", "info");
  } catch (err) {
    appendLog("boot failed: " + err, "error");
  }
}

boot();
