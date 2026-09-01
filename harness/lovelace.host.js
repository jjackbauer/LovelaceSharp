// LovelaceSharp DSH harness — the `lovelace` model tool.
//
// This file is the `code.host` body for a Dynamic Cordis Plugin in DeepSeek
// Harness. It registers a `lovelace` tool that evaluates a Lovelace script
// through the non-interactive `Lovelace.Run` runner (see harness/README.md) and
// returns the result, variables, functions, and any plot as JSON.
//
// Load it in any DSH session with the `cordis_define` / `cordis_run` tools:
//   - cordis_define (kind "new", idPrefix "lovel"), code.host = this file's body
//   - cordis_run  (mode "run")
//
// The runner must be published first (Native AOT binary):
//   dotnet publish Lovelace.Run/Lovelace.Run.csproj -c Release -f net10.0 \
//     -p:PublishAot=true -p:InvariantGlobalization=true \
//     -o Lovelace.Run/bin/Release/net10.0/publish
//   (or: make runner)

return {
  name: 'lovelace-runner',
  apply(ctx) {
    function errMsg(e) {
      return e && e.message ? e.message : String(e)
    }

    harness.registerTool(ctx, harness.defineTool({
      name: 'lovelace',
      description:
        'Evaluate a LovelaceSharp script (the arbitrary-precision math scripting language) and return the result, ' +
        'variables, functions, and any plot. Scripts may be newline- or semicolon-separated. ' +
        'Example: "x = 1..10; y = 1 / x^2; plot(x, y, \"1/x^2\")". ' +
        'Use this to author and test Lovelace scripts quickly instead of the interactive REPL.',
      parameters: {
        script: { type: 'string', required: true, description: 'The Lovelace script source (newline- or semicolon-separated).' },
        plotDir: { type: 'string', description: 'Directory for plot() SVG output. Defaults to the workspace root.' },
        plotFile: { type: 'string', description: 'Filename for plot() SVG output. Defaults to plot.svg.' },
        runner: { type: 'string', description: 'Path to the Lovelace.Run executable. Defaults to the published apphost under the session workspace.' },
      },
      output: {
        schema: { type: 'json' },
        render: (_args, value) => {
          if (!value || value.ok === false) {
            return [{ type: 'text', text: JSON.stringify(value, null, 2) }]
          }
          const lines = []
          lines.push(value.result ? ('result: ' + value.result.typed) : 'result: (void)')
          const vars = value.variables || []
          const shown = vars.filter(v => v.name !== '_')
          for (const v of shown) lines.push('  ' + v.name + ' = ' + v.display + '  (' + v.kind + ')')
          if (shown.length === 0 && vars.length > 0) lines.push('  (no named variables)')
          if (value.plot) {
            lines.push('plot: ' + value.plot.path)
            lines.push('  title: ' + JSON.stringify(value.plot.title))
            lines.push('  svg bytes: ' + value.plot.svg.length)
          } else {
            lines.push('plot: none')
          }
          return [{ type: 'text', text: lines.join('\n') }]
        },
      },
      async execute(args, exec) {
        const sub = ctx.get('subprocess')
        if (sub === undefined) {
          return { ok: false, error: 'the subprocess service is not available in this host composition' }
        }

        // The session workspace is the calling agent's cwd (the same source the
        // harness's own fs/lsp tools use: exec.agent.session.header.cwd).
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
          return { ok: false, error: 'could not resolve the session workspace cwd; pass the "runner" argument explicitly' }
        }

        const runner = (args.runner !== undefined && args.runner !== '') ? args.runner : ws + '\\Lovelace.Run\\bin\\Release\\net10.0\\publish\\Lovelace.Run.exe'
        const argv = [runner, '--stdin', '--json']
        if (args.plotDir !== undefined && args.plotDir !== '') argv.push('--plot-dir', args.plotDir)
        if (args.plotFile !== undefined && args.plotFile !== '') argv.push('--plot-file', args.plotFile)

        let handle
        try {
          handle = sub.spawn({
            argv,
            cwd: ws,
            stdio: {
              stdin: { data: args.script },
              stdout: { maxBytes: 4000000, spill: { maxBytes: 8000000 } },
              stderr: { maxBytes: 200000 },
            },
            graceMs: 10000,
            signal: exec.signal,
          })
        } catch (e) {
          return { ok: false, error: 'spawn failed: ' + errMsg(e) }
        }

        let outcome
        try {
          outcome = await handle.done
        } catch (e) {
          return { ok: false, error: 'runner failed to start: ' + errMsg(e) + '. Build it with: dotnet publish -c Release Lovelace.Run' }
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
            error: 'runner did not emit valid JSON',
          }
        }

        parsed.exitCode = outcome.exitCode
        if (stderr !== '') parsed.stderr = stderr
        return parsed
      },
    }))
  },
}
