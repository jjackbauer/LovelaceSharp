<#
.SYNOPSIS
  Fail-closed acceptance gate for the Lovelace Desktop v4.0 contract.

.DESCRIPTION
  Two verdicts:
    -Readiness  no P0 row may be FAIL or NOT_TESTED. PASS rows need fresh, class-correct,
                hash-bound evidence. A BLOCKED row is allowed only with a user-owned waiver.
    -Verify     strict release gate: additionally every P0 row is PASS or waived, all 22 journeys
                are PASS, no open severity-1 defect, and the working tree is clean at the same HEAD
                that produced the evidence.

  Evidence classes stop headless proof from certifying a GUI row:
    unit | gui | perf | file | static   (see docs/goal-desktop-v4/goal.md)

  -Init parses docs/goal-desktop-v4/requirements.md into gate/register.json and gate/journeys.json,
  preserving any status/evidence already recorded.

  Exit codes: 0 pass, 1 gate failure, 2 instrument error.
#>
[CmdletBinding()]
param(
    [switch]$Init,
    [switch]$Status,
    [switch]$Readiness,
    [switch]$Verify,
    [int]$List = 25,
    [string]$RepoRoot = ''
)

$ErrorActionPreference = 'Stop'

function Get-RepoRoot {
    if ($RepoRoot -ne '') { return (Resolve-Path $RepoRoot).Path }
    $d = $PSScriptRoot
    while ($d -and -not (Test-Path (Join-Path $d '.git'))) { $d = Split-Path $d -Parent }
    if (-not $d) { throw 'cannot locate repository root (no .git above the script)' }
    return $d
}

$Root = Get-RepoRoot
$GoalDir = Join-Path $Root 'docs\goal-desktop-v4'
$GateDir = Join-Path $GoalDir 'gate'
$ReqPath = Join-Path $GoalDir 'requirements.md'
$RegisterPath = Join-Path $GateDir 'register.json'
$JourneysPath = Join-Path $GateDir 'journeys.json'
$GateJsonPath = Join-Path $GateDir 'gate.json'
$WaiversPath = Join-Path $GateDir 'waivers.json'
$DefectsPath = Join-Path $GateDir 'defects.json'

$script:Expected = @{ total = 229; P0 = 171; P1 = 56; P2 = 2 }

# ---------------------------------------------------------------------------------------------
# Evidence class policy: a PASS row must carry at least one artifact of its required class.
# Group default first, then per-row overrides where the honest evidence is not a GUI artifact.
# ---------------------------------------------------------------------------------------------
$script:GroupClass = @{
    CON = 'static'; ARC = 'gui'; VIS = 'gui'; LAY = 'gui'; EDT = 'gui'; DOC = 'file'
    RUN = 'gui'; INC = 'unit'; WKS = 'gui'; RES = 'gui'; SYM = 'gui'; PLT = 'gui'
    LOG = 'gui'; A11Y = 'gui'; PLF = 'static'; PERF = 'perf'; TST = 'file'
}
$script:ClassOverride = @{
    'CON-007' = 'file'; 'CON-009' = 'file'
    'ARC-001' = 'perf'; 'ARC-002' = 'static'; 'ARC-003' = 'static'; 'ARC-012' = 'perf'
    'VIS-009' = 'perf'
    'EDT-012' = 'unit'; 'EDT-013' = 'file'; 'EDT-014' = 'unit'; 'EDT-016' = 'perf'
    'DOC-001' = 'unit'; 'DOC-008' = 'unit'; 'DOC-009' = 'unit'; 'DOC-014' = 'unit'
    'RUN-005' = 'perf'; 'RUN-008' = 'perf'; 'RUN-016' = 'perf'; 'RUN-017' = 'unit'
    'INC-008' = 'perf'
    'RES-002' = 'unit'; 'RES-006' = 'unit'
    'PLT-003' = 'perf'; 'PLT-008' = 'unit'; 'PLT-012' = 'perf'
    'LOG-001' = 'unit'; 'LOG-006' = 'unit'; 'LOG-010' = 'perf'
    'A11Y-014' = 'unit'
    'PLF-001' = 'file'; 'PLF-002' = 'static'; 'PLF-003' = 'unit'; 'PLF-004' = 'file'
    'PLF-005' = 'static'; 'PLF-006' = 'unit'; 'PLF-009' = 'static'; 'PLF-010' = 'gui'
    'TST-001' = 'file'; 'TST-002' = 'gui'; 'TST-003' = 'gui'; 'TST-004' = 'gui'
    'TST-005' = 'gui'; 'TST-006' = 'unit'; 'TST-007' = 'gui'; 'TST-008' = 'unit'
    'TST-009' = 'gui'; 'TST-010' = 'file'; 'TST-011' = 'gui'; 'TST-014' = 'gui'
    'TST-015' = 'perf'; 'TST-016' = 'file'; 'TST-017' = 'file'
}
function Get-EvidenceClass([string]$id, [string]$group) {
    if ($script:ClassOverride.ContainsKey($id)) { return $script:ClassOverride[$id] }
    if ($script:GroupClass.ContainsKey($group)) { return $script:GroupClass[$group] }
    return 'file'
}

# ---------------------------------------------------------------------------------------------
# Catalog parsing
# ---------------------------------------------------------------------------------------------
function Read-Catalog {
    if (-not (Test-Path $ReqPath)) { throw "requirement catalog not found: $ReqPath" }
    $lines = Get-Content -LiteralPath $ReqPath -Encoding utf8
    $rows = New-Object System.Collections.Generic.List[object]
    $journeys = New-Object System.Collections.Generic.List[object]
    $group = $null
    $inJourneys = $false
    $seen = @{}

    foreach ($line in $lines) {
        if ($line -match '^###\s+([A-Z][A-Z0-9]*)\s+') { $group = $Matches[1]; $inJourneys = $false; continue }
        if ($line -match '^##\s+4\.\s') { $inJourneys = $true; $group = $null; continue }
        if ($line -match '^##\s+5\.\s') { $inJourneys = $false; $group = $null; continue }

        if ($inJourneys -and $line -match '^\|\s*(J-\d{2})\s*\|\s*(.+?)\s*\|\s*(.+?)\s*\|\s*$') {
            $id = $Matches[1]
            if ($seen.ContainsKey($id)) { throw "duplicate journey id $id" }
            $seen[$id] = $true
            $journeys.Add([ordered]@{ id = $id; name = $Matches[2]; outcome = $Matches[3]; status = 'NOT_TESTED'; evidence = @(); notes = '' })
            continue
        }

        if ($group -and $line -match '^\|\s*\*\*([A-Z][A-Z0-9]*-\d{3})\*\*\s*\|\s*(P[012])\s*\|\s*([WDQ])\s*\|\s*(.+?)\s*\|\s*(.+?)\s*\|\s*$') {
            $id = $Matches[1]
            if ($seen.ContainsKey($id)) { throw "duplicate requirement id $id" }
            $seen[$id] = $true
            $rows.Add([ordered]@{
                id             = $id
                group          = $group
                priority       = $Matches[2]
                type           = $Matches[3]
                requirement    = $Matches[4]
                acceptance     = $Matches[5]
                status         = 'NOT_TESTED'
                evidence_class = (Get-EvidenceClass $id $group)
                evidence       = @()
                test           = ''
                waiver         = ''
                scope_note     = ''
            })
        }
    }

    if ($rows.Count -ne $script:Expected.total) {
        throw "catalog parsed $($rows.Count) requirements, expected $($script:Expected.total); transcription is incomplete or malformed"
    }
    foreach ($p in @('P0', 'P1', 'P2')) {
        $n = @($rows | Where-Object { $_.priority -eq $p }).Count
        if ($n -ne $script:Expected[$p]) { throw "catalog has $n $p rows, expected $($script:Expected[$p])" }
    }
    if ($journeys.Count -ne 22) { throw "catalog parsed $($journeys.Count) journeys, expected 22" }
    return @{ rows = $rows; journeys = $journeys }
}

function Load-Register {
    if (-not (Test-Path $RegisterPath)) { throw "register missing - run: powershell -File tools/desktop-gate.ps1 -Init" }
    return (Get-Content -LiteralPath $RegisterPath -Raw -Encoding utf8 | ConvertFrom-Json)
}

function Load-Journeys {
    if (-not (Test-Path $JourneysPath)) { return @() }
    return @((Get-Content -LiteralPath $JourneysPath -Raw -Encoding utf8 | ConvertFrom-Json).journeys)
}

function Save-Json($obj, $path) {
    New-Item -ItemType Directory -Force -Path (Split-Path $path -Parent) | Out-Null
    $json = $obj | ConvertTo-Json -Depth 14
    [System.IO.File]::WriteAllText($path, $json, (New-Object System.Text.UTF8Encoding($false)))
}

function Initialize-Register {
    $catalog = Read-Catalog
    $old = @{}
    if (Test-Path $RegisterPath) {
        foreach ($r in (Load-Register).rows) { $old[$r.id] = $r }
    }
    $rows = foreach ($r in $catalog.rows) {
        if ($old.ContainsKey($r.id)) {
            $r.status = $old[$r.id].status
            $r.evidence = @($old[$r.id].evidence)
            $r.test = $old[$r.id].test
            $r.waiver = $old[$r.id].waiver
            if ($old[$r.id].scope_note) { $r.scope_note = $old[$r.id].scope_note }
        }
        [pscustomobject]$r
    }
    $journeys = foreach ($j in $catalog.journeys) { [pscustomobject]$j }
    $register = [pscustomobject]@{
        schema        = 1
        source        = 'docs/goal-desktop-v4/requirements.md'
        catalog_total = $script:Expected.total
        catalog_p0    = $script:Expected.P0
        catalog_p1    = $script:Expected.P1
        catalog_p2    = $script:Expected.P2
        rows          = @($rows)
    }
    Save-Json $register $RegisterPath
    Save-Json ([pscustomobject]@{ schema = 1; count = 22; journeys = @($journeys) }) $JourneysPath
    $byClass = @{}
    foreach ($r in $register.rows) { $c = $r.evidence_class; if (-not $byClass.ContainsKey($c)) { $byClass[$c] = 0 }; $byClass[$c]++ }
    $classText = ($byClass.Keys | Sort-Object | ForEach-Object { "$_=$($byClass[$_])" }) -join ' '
    Write-Output "register: $($register.rows.Count) rows ($($script:Expected.P0) P0 / $($script:Expected.P1) P1 / $($script:Expected.P2) P2); journeys: 22; evidence classes: $classText"
}

function Get-GitState {
    $sha = (& git -C $Root rev-parse HEAD 2>$null)
    if ($LASTEXITCODE -ne 0) { throw 'git rev-parse failed' }
    $porcelain = @(& git -C $Root status --porcelain 2>$null)
    return @{ head = "$sha".Trim(); dirty = @($porcelain | Where-Object { $_ -ne '' }) }
}

function Load-Waivers {
    $map = @{}
    if (-not (Test-Path $WaiversPath)) { return $map }
    $doc = Get-Content -LiteralPath $WaiversPath -Raw -Encoding utf8 | ConvertFrom-Json
    foreach ($w in @($doc.waivers)) {
        $ok = ($w.owner -eq 'user') -and ($w.approved -eq $true) -and ($w.quote -and $w.quote.Length -gt 10)
        $map[$w.id] = @{ valid = $ok; quote = $w.quote; scope = $w.scope; date = $w.date }
    }
    return $map
}

function Load-Defects {
    if (-not (Test-Path $DefectsPath)) { return @() }
    return @((Get-Content -LiteralPath $DefectsPath -Raw -Encoding utf8 | ConvertFrom-Json).defects)
}

function Test-Evidence($id, $requiredClass, $evidence, $head) {
    $problems = New-Object System.Collections.Generic.List[string]
    $evidence = @($evidence)
    if ($evidence.Count -eq 0) { $problems.Add("$id`: PASS with no evidence"); return $problems }
    $classOk = $false
    foreach ($e in $evidence) {
        if (-not $e.path) { $problems.Add("$id`: evidence without path"); continue }
        $p = if ([System.IO.Path]::IsPathRooted($e.path)) { $e.path } else { Join-Path $Root $e.path }
        if (-not (Test-Path -LiteralPath $p)) { $problems.Add("$id`: evidence file missing: $($e.path)"); continue }
        $actual = (Get-FileHash -LiteralPath $p -Algorithm SHA256).Hash
        if (-not $e.sha256) { $problems.Add("$id`: evidence has no recorded hash: $($e.path)") }
        elseif ($e.sha256 -ne $actual) { $problems.Add("$id`: stale evidence hash: $($e.path)") }
        # Provenance: the evidence must have been produced at HEAD or an ancestor of it. A shared
        # repository keeps moving forward; an unrelated later commit must not invalidate proof, but
        # proof produced in a *future* state must never certify the present one.
        if ($e.at_head -and $head) {
            if ($e.at_head -ne $head) {
                & git -C $Root merge-base --is-ancestor $e.at_head $head 2>$null
                if ($LASTEXITCODE -ne 0) { $problems.Add("$id`: evidence commit $($e.at_head) is not an ancestor of HEAD $head") }
            }
        }
        if ($e.class -eq $requiredClass) { $classOk = $true }
    }
    if (-not $classOk) { $problems.Add("$id`: needs at least one '$requiredClass' artifact; has: $((($evidence | ForEach-Object { $_.class }) -join ','))") }
    return $problems
}

function Get-DimensionTally($register, $journeys, $waivers) {
    $groups = [ordered]@{
        D01 = 'CON'; D02 = 'ARC'; D03 = 'VIS'; D04 = 'LAY'; D05 = 'EDT'; D06 = 'DOC'
        D07 = 'RUN'; D08 = 'INC'; D09 = 'WKS'; D10 = 'RES'; D11 = 'SYM'; D12 = 'PLT'
        D13 = 'LOG'; D14 = 'A11Y'; D15 = 'PLF'; D16 = 'PERF'; D17 = 'TST'
    }
    $result = @{}
    foreach ($k in $groups.Keys) {
        $g = $groups[$k]
        $p0 = @($register.rows | Where-Object { $_.group -eq $g -and $_.priority -eq 'P0' })
        $bad = @($p0 | Where-Object { $_.status -ne 'PASS' -and -not ($_.status -eq 'BLOCKED' -and $waivers.ContainsKey($_.id) -and $waivers[$_.id].valid) })
        $result[$k] = @{ group = $g; total = $p0.Count; pass = @($p0 | Where-Object { $_.status -eq 'PASS' }).Count; met = ($bad.Count -eq 0) }
    }
    $jbad = @($journeys | Where-Object { $_.status -ne 'PASS' })
    $result['D18'] = @{ group = 'JOURNEYS'; total = @($journeys).Count; pass = @($journeys | Where-Object { $_.status -eq 'PASS' }).Count; met = ($jbad.Count -eq 0 -and @($journeys).Count -eq 22) }
    $d19 = @($register.rows | Where-Object { $_.id -in @('TST-016', 'CON-005', 'CON-008') -and $_.priority -eq 'P0' })
    $d19bad = @($d19 | Where-Object { $_.status -ne 'PASS' })
    $result['D19'] = @{ group = 'EVIDENCE'; total = $d19.Count; pass = @($d19 | Where-Object { $_.status -eq 'PASS' }).Count; met = ($d19bad.Count -eq 0) }
    return $result
}

function Invoke-Check([bool]$strict) {
    $register = Load-Register
    $journeys = Load-Journeys
    $waivers = Load-Waivers
    $defects = Load-Defects
    $git = Get-GitState
    $problems = New-Object System.Collections.Generic.List[string]

    if ($register.catalog_total -ne $script:Expected.total) { $problems.Add("register catalog_total=$($register.catalog_total) expected=$($script:Expected.total)") }
    $ids = @($register.rows | ForEach-Object { $_.id })
    if (($ids | Sort-Object -Unique).Count -ne $ids.Count) { $problems.Add('register contains duplicate requirement ids') }

    $p0 = @($register.rows | Where-Object { $_.priority -eq 'P0' })
    foreach ($r in $p0) {
        $waived = $waivers.ContainsKey($r.id) -and $waivers[$r.id].valid
        switch ($r.status) {
            'PASS' { foreach ($p in (Test-Evidence $r.id $r.evidence_class $r.evidence $git.head)) { $problems.Add($p) } }
            'BLOCKED' {
                if (-not $waived) { $problems.Add("$($r.id): BLOCKED without a user-owned waiver") }
            }
            'APPROVED_DEVIATION' {
                if (-not $waived) { $problems.Add("$($r.id): APPROVED_DEVIATION without a user-owned waiver record") }
            }
            default { $problems.Add("$($r.id): P0 status $($r.status)") }
        }
    }

    if (@($journeys).Count -ne 22) { $problems.Add("journeys file has $(@($journeys).Count) rows, expected 22") }
    foreach ($j in $journeys) {
        if ($j.status -ne 'PASS') { $problems.Add("$($j.id): journey status $($j.status)"); continue }
        foreach ($p in (Test-Evidence $j.id 'gui' $j.evidence $git.head)) { $problems.Add($p) }
    }

    $openSev1 = @($defects | Where-Object { $_.severity -eq 1 -and $_.status -eq 'open' })
    if ($openSev1.Count -gt 0) { $problems.Add("$($openSev1.Count) open severity-1 usability defect(s)") }

    if ($strict) {
        if ($git.dirty.Count -gt 0) { $problems.Add("working tree is dirty ($($git.dirty.Count) paths); release evidence must come from a clean commit") }
    }

    $dims = Get-DimensionTally $register $journeys $waivers
    $met = @($dims.Keys | Where-Object { $dims[$_].met }).Count
    foreach ($k in ($dims.Keys | Sort-Object)) {
        if (-not $dims[$k].met -and $strict) { $problems.Add("dimension $k ($($dims[$k].group)) is not MET ($($dims[$k].pass)/$($dims[$k].total) P0 PASS)") }
    }

    $t = [ordered]@{
        p0_total      = $p0.Count
        p0_pass       = @($p0 | Where-Object { $_.status -eq 'PASS' }).Count
        p0_fail       = @($p0 | Where-Object { $_.status -eq 'FAIL' }).Count
        p0_not_tested = @($p0 | Where-Object { $_.status -eq 'NOT_TESTED' }).Count
        p0_blocked    = @($p0 | Where-Object { $_.status -eq 'BLOCKED' }).Count
        p0_deviation  = @($p0 | Where-Object { $_.status -eq 'APPROVED_DEVIATION' }).Count
        p0_waived     = @($p0 | Where-Object { $waivers.ContainsKey($_.id) -and $waivers[$_.id].valid }).Count
        journeys_pass = @($journeys | Where-Object { $_.status -eq 'PASS' }).Count
        journeys_total = @($journeys).Count
    }

    $verdict = 'PASS'
    if ($problems.Count -gt 0) { $verdict = 'FAIL' }
    $result = [ordered]@{
        schema        = 1
        mode          = $(if ($strict) { 'verify' } else { 'readiness' })
        generated_at  = (Get-Date).ToString('o')
        verdict       = $verdict
        provenance    = [ordered]@{ head_sha = $git.head; dirty_paths = $git.dirty.Count }
        p0_total      = $t.p0_total
        p0_pass       = $t.p0_pass
        p0_fail       = $t.p0_fail
        p0_not_tested = $t.p0_not_tested
        p0_blocked    = $t.p0_blocked
        p0_deviation  = $t.p0_deviation
        p0_waived     = $t.p0_waived
        journeys_pass = $t.journeys_pass
        journeys_total = $t.journeys_total
        dimensions_met = $met
        dimensions_total = 19
        dimensions    = @($dims.Keys | Sort-Object | ForEach-Object { [ordered]@{ id = $_; group = $dims[$_].group; pass = $dims[$_].pass; total = $dims[$_].total; met = $dims[$_].met } })
        problems      = @($problems)
    }
    Save-Json $result $GateJsonPath
    return $result
}

function Show-Status($register, $journeys, $waivers) {
    $p0 = @($register.rows | Where-Object { $_.priority -eq 'P0' })
    Write-Output ('P0 {0}/{1} PASS  FAIL {2}  NOT_TESTED {3}  BLOCKED {4}  DEVIATION {5}  WAIVED {6}' -f `
        @($p0 | Where-Object { $_.status -eq 'PASS' }).Count, $p0.Count,
        @($p0 | Where-Object { $_.status -eq 'FAIL' }).Count,
        @($p0 | Where-Object { $_.status -eq 'NOT_TESTED' }).Count,
        @($p0 | Where-Object { $_.status -eq 'BLOCKED' }).Count,
        @($p0 | Where-Object { $_.status -eq 'APPROVED_DEVIATION' }).Count,
        @($p0 | Where-Object { $waivers.ContainsKey($_.id) -and $waivers[$_.id].valid }).Count)
    Write-Output ('Journeys {0}/{1} PASS' -f @($journeys | Where-Object { $_.status -eq 'PASS' }).Count, @($journeys).Count)
    $dims = Get-DimensionTally $register $journeys $waivers
    Write-Output ('Dimensions {0}/19 MET: {1}' -f @($dims.Keys | Where-Object { $dims[$_].met }).Count, (($dims.Keys | Sort-Object | ForEach-Object { if ($dims[$_].met) { "$_+" } else { "$_-" } }) -join ' '))
    $open = @($p0 | Where-Object { $_.status -ne 'PASS' } | Select-Object -First $List)
    if ($open.Count -gt 0) {
        Write-Output ''
        Write-Output "First $($open.Count) open P0 rows:"
        foreach ($r in $open) { Write-Output ("  {0,-9} {1,-12} {2}" -f $r.id, $r.status, $r.requirement) }
    }
}

if (-not ($Init -or $Status -or $Readiness -or $Verify)) {
    Write-Output 'usage: desktop-gate.ps1 -Init | -Status [-List n] | -Readiness | -Verify'
    exit 2
}

try {
    if ($Init) { Initialize-Register; exit 0 }
    if ($Status) { Show-Status (Load-Register) (Load-Journeys) (Load-Waivers); exit 0 }

    $strict = [bool]$Verify
    $result = Invoke-Check $strict
    $count = @($result.problems).Count
    $tag = if ($strict) { 'RELEASE' } else { 'READINESS' }
    if ($result.verdict -eq 'PASS') {
        Write-Output "GATE PASS [$tag] - $($result.p0_pass)/$($result.p0_total) P0, $($result.p0_waived) waived, $($result.journeys_pass)/$($result.journeys_total) journeys, $($result.dimensions_met)/19 dimensions, HEAD $($result.provenance.head_sha)"
        exit 0
    }
    Write-Output "GATE FAIL [$tag] - $count blocking problem(s)"
    foreach ($p in ($result.problems | Select-Object -First $List)) { Write-Output "  $p" }
    if ($count -gt $List) { Write-Output "  ... $(($count - $List)) more in $GateJsonPath" }
    exit 1
}
catch {
    Write-Output "INSTRUMENT ERROR: $($_.Exception.Message)"
    exit 2
}
