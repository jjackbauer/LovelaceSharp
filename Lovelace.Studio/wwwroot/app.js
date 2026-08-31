"use strict";

const $ = (sel) => document.querySelector(sel);

const editor = $("#editor");
const cursorPos = $("#cursor-pos");
const statusEl = $("#status");
const variablesTbody = $("#variables-table tbody");
const variablesEmpty = $("#variables-empty");
const functionsTbody = $("#functions-table tbody");
const functionsEmpty = $("#functions-empty");
const graphContainer = $("#graph-container");
const graphPlaceholder = $("#graph-placeholder");
const logsEl = $("#logs");
const quickEval = $("#quick-eval");

const STORAGE_KEY = "lovelace.studio.editor";
let running = false;

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

    const nameTd = document.createElement("td");
    nameTd.textContent = v.name;

    const valueTd = document.createElement("td");
    valueTd.textContent = v.display;
    valueTd.title = v.display;

    const kindTd = document.createElement("td");
    kindTd.textContent = v.kind;
    kindTd.className = "kind";

    const delTd = document.createElement("td");
    const delBtn = document.createElement("button");
    delBtn.textContent = "×";
    delBtn.className = "row-delete";
    delBtn.title = "Delete " + v.name;
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

    const nameTd = document.createElement("td");
    nameTd.textContent = f.name;

    const paramsTd = document.createElement("td");
    paramsTd.textContent = (f.parameters || []).join(", ");

    const kindTd = document.createElement("td");
    kindTd.textContent = f.isBuiltin ? "builtin" : "user";
    kindTd.className = f.isBuiltin ? "kind builtin" : "kind user";

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
    wrap.innerHTML = plot.svg; // inline SVG, inspectable in the DOM
    graphContainer.appendChild(wrap);
  } else {
    graphPlaceholder.style.display = "";
  }
}

function setStatus(revision) {
  statusEl.textContent = "rev " + revision;
}

// ---------------------------------------------------------------------------
// Error highlighting
// ---------------------------------------------------------------------------

function highlightError(d) {
  const text = editor.value;
  let pos = typeof d.position === "number" ? d.position : 0;
  pos = Math.max(0, Math.min(pos, text.length));

  let lineStart = text.lastIndexOf("\n", pos - 1);
  lineStart = lineStart < 0 ? 0 : lineStart + 1;
  let lineEnd = text.indexOf("\n", pos);
  if (lineEnd < 0) lineEnd = text.length;

  // Scroll the caret into view, then highlight the offending line.
  editor.focus();
  editor.setSelectionRange(pos, pos);
  editor.setSelectionRange(lineStart, lineEnd);

  editor.classList.remove("editor-error");
  void editor.offsetWidth; // restart the CSS transition
  editor.classList.add("editor-error");
  setTimeout(() => editor.classList.remove("editor-error"), 2500);
}

// ---------------------------------------------------------------------------
// Response application
// ---------------------------------------------------------------------------

function applyResponse(data) {
  renderVariables(data.variables);
  renderFunctions(data.functions);
  renderPlot(data.plot);
  setStatus(data.revision);

  for (const line of data.logs || []) {
    appendLog(line, "output");
  }

  if (data.result && data.result.kind !== "Void") {
    appendLog("= " + data.result.typed, "result");
  }

  if (data.diagnostics && data.diagnostics.length > 0) {
    const d = data.diagnostics[0];
    appendLog("error: " + d.message + " (line " + d.line + ", col " + d.column + ")", "error");
    highlightError(d);
  }
}

// ---------------------------------------------------------------------------
// API calls
// ---------------------------------------------------------------------------

async function run(source) {
  if (running) return;
  running = true;
  try {
    const res = await fetch("/api/evaluate", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ source })
    });

    if (!res.ok) {
      const text = await res.text();
      appendLog("HTTP " + res.status + ": " + text, "error");
      return;
    }

    const data = await res.json();
    applyResponse(data);
  } catch (err) {
    appendLog("request failed: " + err, "error");
  } finally {
    running = false;
  }
}

async function deleteVariable(name) {
  try {
    const res = await fetch("/api/variables/" + encodeURIComponent(name), { method: "DELETE" });
    if (!res.ok) {
      appendLog("delete failed: HTTP " + res.status, "error");
      return;
    }
    const data = await res.json();
    renderVariables(data.variables);
    renderFunctions(data.functions);
    setStatus(data.revision);
    appendLog("deleted " + name, "info");
  } catch (err) {
    appendLog("delete failed: " + err, "error");
  }
}

async function clearWorkspace() {
  try {
    const res = await fetch("/api/state", { method: "DELETE" });
    if (!res.ok) {
      appendLog("clear failed: HTTP " + res.status, "error");
      return;
    }
    const data = await res.json();
    renderVariables(data.variables);
    renderFunctions(data.functions);
    setStatus(data.revision);
    appendLog("workspace cleared (variables)", "info");
  } catch (err) {
    appendLog("clear failed: " + err, "error");
  }
}

async function loadState() {
  try {
    const res = await fetch("/api/state");
    const data = await res.json();
    renderVariables(data.variables);
    renderFunctions(data.functions);
    setStatus(data.revision);
  } catch (err) {
    appendLog("could not load state: " + err, "error");
  }
}

// ---------------------------------------------------------------------------
// Editor helpers
// ---------------------------------------------------------------------------

function updateCursorPos() {
  const pos = editor.selectionStart;
  const before = editor.value.slice(0, pos);
  const line = before.split("\n").length;
  const col = pos - before.lastIndexOf("\n");
  cursorPos.textContent = "Ln " + line + ", Col " + col;
}

function saveEditor() {
  localStorage.setItem(STORAGE_KEY, editor.value);
  appendLog("editor saved to localStorage", "info");
}

function loadEditor() {
  const value = localStorage.getItem(STORAGE_KEY);
  editor.value = value !== null ? value : "";
  updateCursorPos();
  appendLog(value !== null ? "editor loaded from localStorage" : "no saved editor content", "info");
}

// ---------------------------------------------------------------------------
// Wiring
// ---------------------------------------------------------------------------

$("#run-btn").addEventListener("click", () => run(editor.value));
$("#clear-btn").addEventListener("click", clearWorkspace);
$("#save-btn").addEventListener("click", saveEditor);
$("#load-btn").addEventListener("click", loadEditor);

editor.addEventListener("input", () => {
  updateCursorPos();
  localStorage.setItem(STORAGE_KEY, editor.value);
});
editor.addEventListener("keyup", updateCursorPos);
editor.addEventListener("click", updateCursorPos);
editor.addEventListener("select", updateCursorPos);
editor.addEventListener("keydown", (e) => {
  if ((e.ctrlKey || e.metaKey) && e.key === "Enter") {
    e.preventDefault();
    run(editor.value);
  }
});

quickEval.addEventListener("keydown", (e) => {
  if (e.key === "Enter") {
    e.preventDefault();
    const src = quickEval.value.trim();
    quickEval.value = "";
    if (!src) return;
    appendLog(">> " + src, "echo");
    run(src);
  }
});

// ---------------------------------------------------------------------------
// Boot
// ---------------------------------------------------------------------------

const saved = localStorage.getItem(STORAGE_KEY);
if (saved !== null) editor.value = saved;
updateCursorPos();
renderPlot(null);
loadState();
appendLog("Lovelace.Studio ready — Run to evaluate.", "info");
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
  } catch { /* ignore malformed sizes */ }
}

restoreSizes();

