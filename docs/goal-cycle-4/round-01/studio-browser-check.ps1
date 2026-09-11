# Cycle-4 Blocker 3: load the Studio UI in a real browser and prove it works.
# Starts the Studio host, drives it with headless Chrome over CDP (see browser-check.mjs),
# captures console output, page exceptions, DOM state and a screenshot, then tears everything down.
$ErrorActionPreference = 'Continue'
$repo = 'C:\Users\ricar\dev\LovelaceSharp'
Set-Location $repo
$outDir = Join-Path $repo 'docs\goal-cycle-4\round-01\studio-browser'
New-Item -ItemType Directory -Force -Path $outDir | Out-Null
$port = 5199
$cdp = 9223
$chrome = 'C:\Program Files\Google\Chrome\Application\chrome.exe'
$profile = Join-Path $env:TEMP ('lovelace-chrome-' + [guid]::NewGuid().ToString('N').Substring(0, 8))

function Stop-Mine($pattern) {
  Get-CimInstance Win32_Process -Filter "Name = 'chrome.exe' OR Name = 'Lovelace.Studio.exe' OR Name = 'dotnet.exe'" -ErrorAction SilentlyContinue |
    Where-Object { $_.CommandLine -and ($_.CommandLine -like "*$pattern*") } |
    ForEach-Object { try { Stop-Process -Id $_.ProcessId -Force -ErrorAction Stop; "killed pid $($_.ProcessId)" } catch { } }
}

"=== starting Studio host on http://127.0.0.1:$port ==="
$studio = Start-Process -FilePath 'dotnet' -ArgumentList @('run', '--project', 'Lovelace.Studio', '-c', 'Release', '--no-build', '--', '--urls', "http://127.0.0.1:$port") -PassThru -NoNewWindow -RedirectStandardOutput (Join-Path $outDir 'studio-stdout.log') -RedirectStandardError (Join-Path $outDir 'studio-stderr.log')
"studio dotnet pid=$($studio.Id)"

$ready = $false
for ($i = 0; $i -lt 120; $i++) {
  Start-Sleep -Milliseconds 500
  try {
    $r = Invoke-WebRequest -Uri "http://127.0.0.1:$port/" -UseBasicParsing -TimeoutSec 5
    if ($r.StatusCode -eq 200) { $ready = $true; "HTTP 200 after $([math]::Round($i * 0.5, 1))s; index.html bytes=$($r.RawContentLength)"; break }
  } catch { }
}
if (-not $ready) {
  "STUDIO NEVER BECAME READY - stdout/stderr:"
  Get-Content (Join-Path $outDir 'studio-stdout.log') -Tail 30 -ErrorAction SilentlyContinue
  Get-Content (Join-Path $outDir 'studio-stderr.log') -Tail 30 -ErrorAction SilentlyContinue
  Stop-Mine 'Lovelace.Studio'
  exit 1
}

# API-level control probe, so a UI failure can be attributed to the browser or to the host.
try {
  $session = Invoke-WebRequest -Uri "http://127.0.0.1:$port/api/session" -Method Post -UseBasicParsing -TimeoutSec 20
  "api/session HTTP $($session.StatusCode)"
} catch { "api/session FAILED: $($_.Exception.Message)" }

$node = 'C:\Program Files\nodejs\node.exe'
$script = 'docs\goal-cycle-4\round-01\browser-check.mjs'
$browserOk = $false
foreach ($mode in @('--headless=new', '--headless')) {
  "=== chrome $mode ==="
  Remove-Item -Recurse -Force $profile -ErrorAction SilentlyContinue
  $c = Start-Process -FilePath $chrome -ArgumentList @($mode, '--disable-gpu', '--no-first-run', '--no-default-browser-check', '--disable-extensions', '--hide-scrollbars', '--window-size=1600,1200', "--remote-debugging-port=$cdp", "--user-data-dir=$profile", 'about:blank') -PassThru
  "chrome pid=$($c.Id)"
  $nodeOut = & $node $script $cdp "http://127.0.0.1:$port/" $outDir 2>&1 | Out-String
  $nodeExit = $LASTEXITCODE
  "node exit=$nodeExit"
  $nodeOut
  if ($nodeExit -eq 0) { $browserOk = $true; break }
  "retrying with a different headless mode"
  Stop-Mine $profile
  Start-Sleep -Seconds 2
}
if (-not $browserOk) { "BROWSER CHECK FAILED IN BOTH MODES" }

"=== teardown ==="
Stop-Mine $profile
Stop-Mine 'Lovelace.Studio'
Start-Sleep -Seconds 1
"artifacts in $outDir"
Get-ChildItem $outDir | ForEach-Object { "{0,10}  {1}" -f $_.Length, $_.Name }
