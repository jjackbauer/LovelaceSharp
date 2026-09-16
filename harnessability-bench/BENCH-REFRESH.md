# Refreshing and re-measuring the harnessability benchmark

This directory is a *working copy* of the published `lovelace-harnessability-public-branch`
package, made runnable and then actually run on this machine. `RESULTS.md` is the execution
record. This file is the part `RESULTS.md` does not carry: how to bring the measurement back
up to date with the repository, and what has silently gone stale before.

## The one artefact that must be refreshed BY HAND

`setup.ps1` publishes `runner/Lovelace.Run.exe` **only when it is missing**
(`setup.ps1:47-55`: `elseif (Test-Path $runner) { 'present' }`). It is therefore a frozen
COPY, and it does not track the repository:

| when | runner/Lovelace.Run.exe | what the repository had |
|---|---|---|
| 2026-09-16 10:52 | 5 815 808 B, sha256 `2E3E365BF9AA056E` | the pre-cycle-7 tree |
| 2026-09-16 19:56 | 6 020 096 B, sha256 `8005C6C74349D8EA` | HEAD `ce30fc3`, with ADV-01..09 |

The older binary **does not know `--functions`** — measured, before it was replaced:
`Unknown argument '--functions'`, exit 2 — and every harnessability measurement taken in that
window (`RESULTS.md` §6, §7) was taken through it. Nothing in the package reports this: a stale
runner validates, scores and benchmarks exactly like a fresh one.

Refresh it from the repository root:

```powershell
make runner                      # -> Lovelace.Run/bin/Release/net10.0/publish/Lovelace.Run.exe
Copy-Item Lovelace.Run/bin/Release/net10.0/publish/* harnessability-bench/runner/ -Force

# prove it is the build you think it is, BEFORE measuring:
& harnessability-bench/runner/Lovelace.Run.exe --functions det
#   exit 0 and a 276-byte envelope on a cycle-7 build;
#   "Unknown argument '--functions'", exit 2, on anything older.
```

`--functions` is the cheapest provenance probe the runner has: it is answered without reading a
script, so it distinguishes a build with the registry surface from one without, and nothing else.

## Re-validating the corpus, which is cheap and should be done every time

```powershell
$py     = 'harnessability-bench/tools/python/python.exe'
$runner = 'harnessability-bench/runner/Lovelace.Run.exe'
$branch = 'harnessability-bench/lovelace-harnessability-public-branch'

& $py $branch/private/build_corpus.py --check --runner $runner   # 20 goldens, 48 probes
& $py $branch/bench.py prepare --runner $runner                 # oracle fingerprints
& $py $branch/bench.py validate --runner $runner                # full corpus/protocol validation
```

`--check` verifies every golden executes and every hidden probe discriminates its shipped seed.
`validate` is the whole protocol check and is the one that must stay green: it re-derives the
oracles, proves no shipped seed accidentally satisfies them, and projects all 80 observations.

## Measuring the observation surface

```
python checks/surface_delta.py --runner runner/Lovelace.Run.exe \
    --json-out runs/surface-delta.json --report-out runs/validation-report-HEAD.json
```

`surface_delta.py` is the checker `RESULTS.md` did not have. It re-measures both halves of the
surface claim against the *current* runner and prints the deltas against the published baseline:

* per-seed observation bytes for all four modes, plus per-mode totals, means and maxima;
* the L3 on-demand registry question answered two ways — the `inspect` path the package shipped
  (one full evaluation, ~41.6 KB of envelope, 171 bytes printed) against `--functions <name>`
  (275 bytes, no evaluation).

It writes `runs/validation-report-HEAD.json` in the published report's exact shape, so the two
can be diffed field by field. **It does not overwrite the published report** — that file is the
package's own baseline and is left alone.

## What has gone stale, and what it does NOT mean

Measured on HEAD `ce30fc3` against `lovelace-harnessability-public-branch/validation-report.json`
(the in-repo baseline):

| mode | baseline total | current total | baseline mean | current mean |
|---|---:|---:|---:|---:|
| L0 | 1 348 | 1 105 | 67.4 | 55.2 |
| L1 | 2 311 | 2 001 | 115.5 | 100.0 |
| L2 | 20 823 | 13 520 | 1 041.2 | 676.0 |
| L3 | 199 108 | 17 741 | 9 955.4 | 887.0 |

and the shipped `validation-summary.json` disagrees with *its own* per-task report
(`mean_observation_bytes` L1 94.0 vs 115.5, L3 852.5 vs 9 955.4). The two files were produced on
different runners; the summary is the older of the two.

**This is not evidence about cycle 7.** The L3 gap is the pre-existing one: those 20 baselines
were projected from envelopes that still carried the full function registry (~16 KB each), which
`--omit-functions` stopped emitting before this cycle; no envelope is 16 KB any more. The
mirror-image check settles the other half: the CURRENT runner reproduces the shipped summary's
`max_observation_bytes` exactly (L0 440, L1 440, L2 4 424, L3 4 718 against 4 716 — 2 B), which it
could not do if the new wire fields (`artifacts[]`, the error `signature`, the widened refusal
messages) reached a projection. Each new field was probed individually and removed the same number
of bytes whether it was present or not: the projections do not carry them.

## The rest of the loop

```powershell
./run-bench.ps1 -Modes L0,L1,L2,L3 -Replicates 3 -Tag matrix-r3   # the agent matrix
python checks/measure_surface.py --runs runs/runs-matrix-r3.jsonl --label head
```

`run-bench.ps1` boots one real DSH headless session per cell (20 tasks x 4 modes x replicates),
points each at `runner/Lovelace.Run.exe`, and scores the merged trajectories with `bench.py score`.
`-Replicates 3` is the current requirement: a single replicate could not separate the modes from
their own noise (gate run A: L0 came in 0.075 below its own baseline).
