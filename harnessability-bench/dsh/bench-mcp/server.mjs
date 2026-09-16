#!/usr/bin/env node
// Benchmark evaluation surface for a DSH session (see DSH-INTEGRATION.md).
//
// This MCP server exposes exactly one candidate-evaluation tool plus, in L3
// only, the on-demand semantic registry lookup.  It never exposes goldens,
// hidden probes or oracle fingerprints: the evaluator's private corpus stays
// outside the agent workspace.
//
// Configuration is entirely env-driven so one server file serves every
// (task, mode, replicate) cell of the matrix:
//
//   BENCH_RUNNER   absolute path to the published Lovelace.Run binary
//   BENCH_TASKS    absolute path to tasks.json (seed + prompt)
//   BENCH_TASK     task id for this session
//   BENCH_MODE     L0 | L1 | L2 | L3
//   BENCH_BUDGET   maximum number of candidate evaluations
//   BENCH_EVENTS   append-only JSONL trajectory file for this run
//   BENCH_RUN_ID   run id stamped on the events
import { spawnSync } from 'node:child_process'
import { appendFileSync } from 'node:fs'
import { readFileSync } from 'node:fs'
import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js'
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js'
import { z } from 'zod'

const runner = process.env.BENCH_RUNNER
const tasksPath = process.env.BENCH_TASKS
const taskId = process.env.BENCH_TASK
const mode = process.env.BENCH_MODE ?? 'L2'
const budget = Number(process.env.BENCH_BUDGET ?? 10)
const eventsPath = process.env.BENCH_EVENTS
const runId = process.env.BENCH_RUN_ID ?? 'run'

const tasks = JSON.parse(readFileSync(tasksPath, 'utf8'))
const task = tasks.find((t) => t.id === taskId)
if (!task) throw new Error('unknown task ' + taskId)

let evaluations = 0
let finalScript = null

function log(event) {
  appendFileSync(eventsPath, JSON.stringify({ run_id: runId, ...event }) + '\n', 'utf8')
}

function runLovelace(script, includeFunctions) {
  const argv = [runner, '--stdin', '--json']
  if (!includeFunctions) argv.push('--omit-functions')
  const p = spawnSync(argv[0], argv.slice(1), { input: script, encoding: 'utf8', maxBuffer: 64 * 1024 * 1024 })
  let env
  try {
    env = JSON.parse(p.stdout)
  } catch {
    throw new Error('runner emitted invalid JSON (exit ' + p.status + '): ' + String(p.stdout).slice(0, 400))
  }
  env._processExitCode = p.status
  return env
}

// Mirrors bench.py::project — the projection IS the experimental variable, so it
// must stay byte-for-byte identical to the controller's implementation.
function project(env, which) {
  const ok = Boolean(env.ok)
  if (which === 'L0') {
    if (ok) return { ok: true, result: (env.result ?? {}).display }
    return { ok: false, error: 'failed' }
  }
  if (which === 'L1') {
    const out = { ok }
    if (ok) out.result = (env.result ?? {}).display
    else {
      out.error = env.message ?? 'failed'
      out.diagnostics = (env.diagnostics ?? []).map((d) => d.message)
    }
    return out
  }
  let out
  if (ok) {
    out = {
      ok: true,
      result: { kind: (env.result ?? {}).kind, structured: (env.result ?? {}).structured },
      variables: (env.variables ?? []).map((v) => ({ name: v.name, kind: v.kind, structured: v.structured })),
    }
  } else {
    out = {
      ok: false,
      code: env.code,
      category: env.category,
      message: env.message,
      recoverable: env.recoverable,
      diagnostics: (env.diagnostics ?? []).map((d) => ({ message: d.message, position: d.position, line: d.line, column: d.column })),
    }
    if (env.partialVariables !== undefined && env.partialVariables !== null) {
      out.partialVariables = env.partialVariables.map((v) => ({ name: v.name, kind: v.kind, structured: v.structured }))
    }
  }
  if (which === 'L3') {
    if (env.timings !== undefined && env.timings !== null) out.timings = env.timings
    if (env.cancellation !== undefined && env.cancellation !== null) out.cancellation = env.cancellation
  }
  return out
}

function functionProjection(f) {
  return {
    name: f.name,
    parameters: f.parameters,
    minArity: f.minArity,
    parameterCount: f.parameterCount,
    variadic: f.variadic,
    builtin: f.builtin,
    plugin: f.plugin,
  }
}

const server = new McpServer({ name: 'lovelace-harnessability-bench', version: '1.0.0' })

server.registerTool(
  'evaluate_candidate',
  {
    title: 'Evaluate a Lovelace Script candidate',
    description:
      'Execute one complete Lovelace Script program and return the feedback this session exposes. ' +
      'Pass the whole program, including any functions it defines and the calls they need.',
    inputSchema: { script: z.string().describe('the complete Lovelace Script program to execute') },
  },
  async ({ script }) => {
    if (evaluations >= budget) {
      const exhausted = { ok: false, error: 'evaluation budget exhausted (' + budget + ' evaluations used)' }
      log({ type: 'observation', observation: exhausted })
      return { content: [{ type: 'text', text: JSON.stringify(exhausted, null, 2) }], isError: true }
    }
    evaluations += 1
    log({ type: 'candidate', script, evaluation: evaluations })
    const env = runLovelace(script, false)
    const observation = project(env, mode)
    log({ type: 'observation', observation })
    return { content: [{ type: 'text', text: JSON.stringify(observation, null, 2) }] }
  },
)

if (mode === 'L3') {
  server.registerTool(
    'inspect_functions',
    {
      title: 'Look up runtime functions by name fragment',
      description: 'Search the runtime function registry by name fragment and return signature/arity metadata.',
      inputSchema: {
        query: z.string().describe('case-insensitive name fragment, e.g. "det"'),
        limit: z.number().int().positive().optional().describe('maximum functions to return (default 20)'),
      },
    },
    async ({ query, limit }) => {
      const env = runLovelace('0', true)
      if (!env.ok) return { content: [{ type: 'text', text: 'registry lookup failed: ' + env.message }], isError: true }
      const q = String(query).toLowerCase()
      const all = (env.functions ?? []).filter((f) => String(f.name).toLowerCase().includes(q))
      const funcs = all.slice(0, limit ?? 20).map(functionProjection)
      const payload = { query, totalMatches: all.length, returned: funcs.length, functions: funcs }
      log({ type: 'inspection', inspection: payload })
      return { content: [{ type: 'text', text: JSON.stringify(payload, null, 2) }] }
    },
  )
}

server.registerTool(
  'submit_final',
  {
    title: 'Submit the final repaired program',
    description:
      'Declare the repair finished and hand in the complete final Lovelace Script program. ' +
      'Call this only when you believe the program is correct.',
    inputSchema: { script: z.string().describe('the complete final Lovelace Script program') },
  },
  async ({ script }) => {
    finalScript = script
    log({ type: 'completion', script })
    return { content: [{ type: 'text', text: 'final program recorded' }] }
  },
)

process.on('SIGTERM', () => process.exit(0))
await server.connect(new StdioServerTransport())
