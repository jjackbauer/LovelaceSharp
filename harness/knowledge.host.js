// LovelaceSharp DSH harness — the 'mgir' model tool.
//
// A thin bridge over the Lovelace.Knowledge.Run CLI (MGIR observation-driven
// behavioral graph discovery). It contains NO sampling/reduction/graph logic —
// transport only: marshal a JSON request, spawn the CLI via the DSH subprocess
// service, parse the JSON response back (P6, R-IMPL-3).
//
// Load it in any DSH session with the cordis_define / cordis_run tools (same as
// harness/lovelace.host.js). Build the CLI first:
//   make knowledge
// and ensure the runner is published (make runner), since converge/sample
// spawn Lovelace.Run per sample.

return {
  name: 'lovelace-knowledge-runner',
  apply(ctx) {
    function errMsg(e) {
      return e && e.message ? e.message : String(e)
    }

    harness.registerTool(ctx, harness.defineTool({
      name: 'mgir',
      description:
        'Observation-driven behavioral graph discovery over the LovelaceSharp engine. ' +
        'Samples the input domain, executes each sample against Lovelace.Run, and reduces the ' +
        'observations into behavior planes, boundaries (with fitted guards), and frontiers, until ' +
        'convergence metrics C1-C4 are met. Commands: config, sample, reduce, converge, query. ' +
        'The graph is built purely from observations; the agent only sets the domain, proposal, and thresholds.',
      parameters: {
        command: { type: 'string', required: true, description: 'config | sample | reduce | converge | query (converge is the autonomous loop).' },
        config: { type: 'json', description: 'Optional full config override: seed, thresholds, value sets, operations, budget.' },
        graphPath: { type: 'string', description: 'Path for the persisted graph JSON. Defaults to knowledge-graph.json in the workspace.' },
        runner: { type: 'string', description: 'Path to the Lovelace.Run executable (defaults to the published apphost).' },
        seed: { type: 'number', description: 'Override the seeded RNG seed.' },
        batchSize: { type: 'number', description: 'Override batch size.' },
        maxSamples: { type: 'number', description: 'Override the per-run sample budget.' },
        query: { type: 'string', description: 'For query: summary | planes | boundaries | frontiers | metrics | graph.' },
        cli: { type: 'string', description: 'Path to the Lovelace.Knowledge.Run executable (defaults to the published apphost).' },
      },
      output: {
        schema: { type: 'json' },
        render: (_args, value) => {
          if (!value || value.ok === false) {
            return [{ type: 'text', text: JSON.stringify(value, null, 2) }]
          }
          const lines = []
          if (value.summary) lines.push('summary: ' + value.summary)
          if (value.metrics) {
            const m = value.metrics
            lines.push(
              'converged: ' + m.converged + (m.stopReason ? ' (' + m.stopReason + ')' : ''),
            )
            lines.push(
              'C1 planes=' + m.planeCount + ' newLastK=' + m.c1NewPlanesLastK +
              ' (rate ' + m.c1NewPlaneRate + ') saturated=' + m.c1Saturated,
            )
            lines.push(
              'C2 boundaries=' + m.c2TotalBoundaries + ' stable=' + m.c2StableBoundaries + '/' + m.c2TotalBoundaries,
            )
            lines.push(
              'C3 agreement=' + m.c3Agreement + ' (' + m.c3AgreedCount + '/' + m.c3HeldOutCount + ' held-out)',
            )
            lines.push('C4 covered=' + m.c4Covered)
          }
          if (value.planes) lines.push('planes: ' + value.planes.length)
          if (value.boundaries) lines.push('boundaries: ' + value.boundaries.length)
          if (value.frontiers) lines.push('frontiers: ' + value.frontiers.length)
          return [{ type: 'text', text: lines.join('
') }]
        },
      },
      async execute(args, exec) {
        const sub = ctx.get('subprocess')
        if (sub === undefined) {
          return { ok: false, error: 'the subprocess service is not available in this host composition' }
        }

        let ws = ''
        try {
          const a = exec && exec.agent
          if (a && a.session && a.session.header && typeof a.session.header.cwd === 'string') {
            ws = a.session.header.cwd
          }
        } catch (e) {
          ws = ''
        }
        if (ws === '') {
          return { ok: false, error: 'could not resolve the session workspace cwd; pass the "cli" argument explicitly' }
        }

        const cli = (args.cli !== undefined && args.cli !== '')
          ? args.cli
          : ws + '/Lovelace.Knowledge.Run/bin/Release/net10.0/publish/Lovelace.Knowledge.Run.exe'

        const request = {
          command: args.command,
          config: args.config,
          graphPath: args.graphPath,
          runner: args.runner,
          seed: args.seed,
          batchSize: args.batchSize,
          maxSamples: args.maxSamples,
          query: args.query,
        }

        let handle
        try {
          handle = sub.spawn({
            argv: [cli, '--stdin'],
            cwd: ws,
            stdio: {
              stdin: { data: JSON.stringify(request) },
              stdout: { maxBytes: 16000000, spill: { maxBytes: 32000000 } },
              stderr: { maxBytes: 400000 },
            },
            graceMs: args.command === 'converge' ? 1800000 : 120000,
            signal: exec.signal,
          })
        } catch (e) {
          return { ok: false, error: 'spawn failed: ' + errMsg(e) }
        }

        let outcome
        try {
          outcome = await handle.done
        } catch (e) {
          return { ok: false, error: 'cli failed: ' + errMsg(e) + '. Build it with: make knowledge' }
        }

        const stdout = handle.collected.stdout ? handle.collected.stdout.readFrom(0).text : ''
        const stderr = handle.collected.stderr ? handle.collected.stderr.readFrom(0).text : ''

        let parsed = null
        try { parsed = JSON.parse(stdout) } catch (e) { parsed = null }

        if (parsed === null || typeof parsed !== 'object' || Array.isArray(parsed)) {
          return {
            ok: false,
            exitCode: outcome.exitCode,
            stderr,
            stdoutTail: String(stdout).slice(0, 4000),
            error: 'cli did not emit valid JSON',
          }
        }

        parsed.exitCode = outcome.exitCode
        if (stderr !== '') parsed.stderr = stderr
        return parsed
      },
    }))
  },
}
