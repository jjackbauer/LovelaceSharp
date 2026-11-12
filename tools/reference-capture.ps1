<#
.SYNOPSIS
  Capture deterministic reference behaviour from the running C# Studio (the parity oracle).

.DESCRIPTION
  Each scenario gets a fresh Studio session, runs one or more scripts through the same REST routes
  the browser client uses, polls each run to a terminal state, and writes the full request/response
  evidence to docs/goal-desktop-v4/reference/<id>.json. Prints one terse line per scenario.

  This is reference capture, not a parity test: the desktop must later reproduce these outputs.
  A scenario whose run does not reach a terminal state in time is recorded with `timed_out: true`
  and the next scenario gets a new session, so one oracle defect cannot hide the rest.
#>
[CmdletBinding()]
param(
    [string]$Base = 'http://localhost:5000',
    [string]$OutDir = '',
    [string]$RepoRoot = ''
)

$ErrorActionPreference = 'Stop'
if ($RepoRoot -eq '') {
    $d = $PSScriptRoot
    while ($d -and -not (Test-Path (Join-Path $d '.git'))) { $d = Split-Path $d -Parent }
    $RepoRoot = $d
}
if ($OutDir -eq '') { $OutDir = Join-Path $RepoRoot 'docs\goal-desktop-v4\reference' }
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null

# Each scenario: id -> ordered list of steps. A step is either @{run = "<source>"} or
# @{api = @{method; path; body}}.
$scenarios = [ordered]@{
    'REF-basic'       = @(@{ run = "a = 2`nb = a^10`nb`n" })
    'REF-precision'   = @(@{ run = "setprecision(20)`nsqrt(2)`nsetprecision(1000)`nsqrt(2)`n" })
    'REF-digits'      = @(@{ run = "setprecision(1000)`nsqrt(2)`n" })
    'REF-incremental' = @(
        @{ run = "a = 2`nb = a + 1`nc = pi(100)`nb + c`n" },
        @{ run = "a = 2`nb = a + 1`nc = pi(100)`nb + c + 1`n" }
    )
    'REF-stream'      = @(@{ run = "print(`"one`")`nprint(`"two`")`nx = 1`nx`n" })
    'REF-error'       = @(@{ run = "y = 1`nz = y + missing_name`n" })
    'REF-matrix'      = @(@{ run = "m = [[1, 2], [3, 4]]`ninv(m)`ns = [[symbol(`"a`"), symbol(`"b`")], [symbol(`"c`"), symbol(`"d`")]]`ninv_full(s)`n" })
    'REF-symbolic'    = @(@{ run = "x = symbol(`"x`")`nf = diff(x^3 + 2*x, x)`nsimplify(f)`n" })
    'REF-solve'       = @(@{ run = "x = symbol(`"x`")`nsolve_full(x^2 - 4, x)`n" })
    'REF-plot'        = @(@{ run = "plot(1..10, (1..10)^2)`n" })
    'REF-clear'       = @(
        @{ run = "func f(x) = x + 1`ny = 41`nf(y)`n" },
        @{ api = @{ method = 'DELETE'; path = '/api/state'; body = $null } }
    )
    'REF-matir'       = @(@{ run = "x = symbol(`"x`")`ncompile(x^2 + 1, [x])`n" })
}

function Invoke-Api([string]$Method, [string]$Path, $Body, [string]$SessionId) {
    $headers = @{}
    if ($SessionId) { $headers['X-Session-Id'] = $SessionId }
    $args = @{ Uri = "$Base$Path"; Method = $Method; TimeoutSec = 30; Headers = $headers }
    if ($null -ne $Body) { $args['Body'] = ($Body | ConvertTo-Json -Depth 8); $args['ContentType'] = 'application/json' }
    return Invoke-RestMethod @args
}

function Get-Terminal([string]$runId, [string]$sid) {
    $last = $null
    for ($i = 0; $i -lt 80; $i++) {
        $last = Invoke-Api 'GET' "/api/run/$runId" $null $sid
        if ($last.status -in @('finished', 'error', 'cancelled', 'failed') -and $null -ne $last.response) { return $last }
        Start-Sleep -Milliseconds 100
    }
    return $last
}

$stamp = (Get-Date).ToString('o')
foreach ($id in $scenarios.Keys) {
    $session = Invoke-Api 'POST' '/api/session' $null $null
    $sid = $session.sessionId
    $steps = @()
    $timedOut = $false
    foreach ($step in $scenarios[$id]) {
        if ($step.ContainsKey('run')) {
            $accepted = Invoke-Api 'POST' '/api/evaluate' @{ source = $step.run } $sid
            $final = Get-Terminal $accepted.runId $sid
            if (-not ($final.status -in @('finished', 'error', 'cancelled', 'failed'))) { $timedOut = $true }
            $steps += [ordered]@{ kind = 'run'; source = $step.run; accepted = $accepted; final = $final }
            $resp = $final.response
            $n = @($resp.timings | Where-Object { $_ -ne $null }).Count
            $d = @($resp.diagnostics | Where-Object { $_ -ne $null }).Count
            $plotLen = if ($resp.plot -and $resp.plot.svg) { $resp.plot.svg.Length } else { 0 }
            $modes = (@($resp.timings) | ForEach-Object { $_.mode }) -join ','
            Write-Output ("{0,-16} status={1,-8} rev={2,-4} steps={3,-2} reused={4,-2} diag={5,-2} plot={6,-6} modes={7}" -f `
                $id, $final.status, $resp.revision, $n, $resp.reusedCount, $d, $plotLen, $modes)
            if ($timedOut) { Write-Output "                 STOPPED: run $($accepted.runId) never reached a terminal state (oracle defect, see oracle-defects.md)"; break }
        }
        else {
            $call = $step.api
            $result = Invoke-Api $call.method $call.path $call.body $sid
            $steps += [ordered]@{ kind = 'api'; method = $call.method; path = $call.path; response = $result }
            Write-Output ("{0,-16} api {1} {2} -> ok" -f $id, $call.method, $call.path)
        }
    }
    $record = [ordered]@{
        id = $id; captured_at = $stamp; session = $sid; oracle = $Base
        timed_out = $timedOut; steps = $steps
    }
    $record | ConvertTo-Json -Depth 16 | Set-Content -LiteralPath (Join-Path $OutDir "$id.json") -Encoding UTF8
}

# Completions catalog, captured once.
$s = Invoke-Api 'POST' '/api/session' $null $null
$completions = Invoke-Api 'GET' '/api/completions' $null $s.sessionId
$completions | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath (Join-Path $OutDir 'completions.json') -Encoding UTF8
Write-Output "completions items=$(@($completions.items).Count)"
Write-Output "captured $(@($scenarios.Keys).Count) scenarios into $OutDir"
