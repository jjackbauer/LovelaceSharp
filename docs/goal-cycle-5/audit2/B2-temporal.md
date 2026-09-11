# B2 — temporal and repetition audit of `Lovelace.Run.exe`

**Audit 2 of goal cycle 5 — independent adversarial review, temporal dimension only**
(previous cycle-5 audits: A1–A4 in `docs/goal-cycle-5/audit1/`; this file deliberately does not re-litigate
their ground).

| fact | value |
|---|---|
| product under test | `out/aot/Lovelace.Run.exe` (Native AOT, not rebuilt) |
| binary SHA-256 | `937C0FDDE2991DDD7B7A84AF042FB484392FF02619EC45350EEA5AFF3DBA060F` |
| binary timestamp | 2026-09-11 15:18:20 |
| repository HEAD at test time | `9e761ba` — "docs(cycle-5): T0-3 closed, the wire cluster verified, and the golden that moved for a measured reason" |
| host | Windows 10.0.26100, 24 logical processors |
| contract consulted | `docs/symbolics/dsh-protocol.md` (210 lines) + `Lovelace.Run.exe --help` |
| independent ground truth | CPython 3.12.14 (`math.factorial`, exact integers) and the formulas for the probed sums/determinants |

## Method

* Every probe is a script file under `%TEMP%\b2\` invoked as
  `out\aot\Lovelace.Run.exe --file <path>.ls --omit-functions [extra flags]`.
* **Volatile-field scrubbing.** Two envelopes are compared after recursively deleting exactly
  `revision`, `elapsed`, `elapsedTime`, `timings` (the fields the protocol calls per-run timing/state
  information). The scrubbed object is serialised with sorted keys and hashed with SHA-256 (first 16 hex
  characters shown as `hash=`). Two runs with the same `hash` are byte-identical in every field that
  decides meaning.
* Driver: `%TEMP%\b2\h.py` (spec-driven, repeats, concurrency, CPU-load burners),
  `%TEMP%\b2\prec.py`, `%TEMP%\b2\final.py`, `%TEMP%\b2\gt.py`, `%TEMP%\b2\show.py`.
  Raw envelopes are kept run-by-run in `%TEMP%\b2\out\<id>-NN.json`.
* Every FINDING below was reproduced at least twice from a freshly written script file (the driver
  rewrites `<id>.ls` before each probe). Reproductions are listed per finding.
* Expected observables come from the contract or from mathematics, never from what the binary printed.

---

## 1. Probe table

### 1.1 Repeatability, determinism and state leakage (T)

| id | what was probed | command | raw output (abridged) | verdict | severity |
|---|---|---|---|---|---|
| T01 | same solve script, 20 sequential runs | `--file T01.ls --omit-functions` | 20× `hash=f4a231ae3d0eca05 rev=82 ok=true kind=Record timings=2` | HELD | — |
| T02 | print-only script `print("alpha"); print(1/3); print(42)`, 20 runs | as above | 20× `hash=1d27b63f15bb3060 rev=80 out=["alpha","0.(3)","42"]` | HELD | — |
| T03 | 9-statement mixed script (solve + setprecision + sqrt + simplify), 12 runs | as above | 12× `hash=c9bd36679c7b977f rev=84 timings=9` | HELD | — |
| T04 | 4 runners **concurrently**, same solve script | 4 parallel processes | 4× `hash=f4a231ae3d0eca05` | HELD | — |
| T05 | 8 runners **concurrently**, print-only script | 8 parallel processes | 8× `hash=1d27b63f15bb3060` | HELD | — |
| T06 | T03 script under 8 CPU-burner processes | as T03 + load | 3× `hash=c9bd36679c7b977f` — **identical to the unloaded T03 hash** | HELD | — |
| T07 | script of 10 000 `print(i)` statements, 3 runs | `--omit-functions --omit-variables` | 3× `items=10000 first="0" last="9999"` and identical output hash `946b139552a9db9a` | HELD | — |
| T08 | `sum(1..2000000)` with `--cancel-after 60000` vs with no flag | `--cancel-after 60000` | both `hash=43f9aba014e8b67c` (`result=2000001000000`) — a flag that never fires does not perturb the envelope | HELD | — |
| T09 | `matmul(eye(400),eye(400))` with `--cancel-after 100` vs no flag | `--cancel-after 100` | both `hash=1d55175a93e0ebf1` (`kind=Array`, 7.5 s) — *same envelope, but see F2* | HELD | — |
| T10 | 2 concurrent `sum(1..10000000)` | 2 parallel processes | both `hash=04e2bd6bfed75cdd`, `result=50000005000000` | HELD | — |
| T11 | 2 concurrent light runs `print("A"); print("B"); 1/3` with `--cancel-after 5` | 2 parallel | both `hash=e6973eb95926b94c ok=true` — no spurious cancel | HELD | — |
| T12 | assumption ordering: 6 variants (`assume(x>0)`, `assume_clear()`, re-`symbol("x")`, two identical solves) | `--file T12.ls` | `assume_clear == base ? True`; `assume_pos_clear_reassume_neg == assume_neg_only ? True`; two solves after one assume produce identical output | HELD | — |

### 1.2 Precision scopes and first-run/later-run caches (P)

Each row ran both orders in **separate processes** and compared the printed digits (and, for P02, the whole
`result` object including `exact`): "fresh" = computed at precision N with nothing before it;
"primed" = `setprecision(300); q = pi(); r = e();` first, then precision N.

| id | what was probed | command | raw output (abridged) | verdict | severity |
|---|---|---|---|---|---|
| P01 | `pi()` and `e()` at N ∈ {1,2,3,4,5,6,7,8,9,10,12,15,20,25,30,40,50,80,100}, fresh vs primed | `setprecision(N); print(pi()); print(e())` | **0/19 differ**. e.g. N=3 both `["3.141","2.718"]`; N=100 identical 100-digit strings for both constants | HELD | — |
| P02 | the `result` object of `pi()` (value + `exact` + structured form), fresh vs primed, N ∈ {1,2,3,5,10,20,50} | `setprecision(N); pi()` | 7/7 `SAME` | HELD | — |
| P03 | deep value of a low-precision `pi` re-evaluated at 30 digits (`evalf(x,30)`), N ∈ {3,5,10} | `setprecision(N); x = pi(); setprecision(30); print(evalf(x,30))` | N=3 both `["3.141"]`, N=5 both `["3.14159"]` — the cached-value truncation is exactly a fresh low-precision computation | HELD | — |
| P04 | repeated identical calls in one process | `setprecision(3); print(pi()); print(pi()); print(pi())` / `setprecision(50); a=pi(); b=pi(); print(a-b); print(a-b)` / sqrt(2) at 20 → 3 → 20 / sin(pi()/6) at 3 → 60 → 3 | `["3.141","3.141","3.141"]`; `["0","0"]`; `["1.4142135623730950488","1.414","1.4142135623730950488"]` (re-upgrading restores the 20-digit value); sin 3→60→3 symmetric | HELD | — |
| P05 | special-angle trig (`sin(pi()/6)`, `cos(pi()/3)`) at N ∈ {1,2,3,4,5,6,8,10,20,40}, fresh vs primed through the `Sqrt2Half/Sqrt3Half` precision-keyed caches | `setprecision(N); print(sin(pi()/6)); print(cos(pi()/3))` | **0/10 differ** (values are symbolic at low N, e.g. `sin(1047/2000)`, and become `0.499…9` at N=40 — identical in both orders) | HELD | — |

### 1.3 Cancellation (CB / CC)

| id | what was probed | command | raw output (abridged) | verdict | severity |
|---|---|---|---|---|---|
| CB1 | `sum(1..10000000)` (≈5.7 s solo) with a 100 ms budget, ×3 | `--cancel-after 100` | 3× `ok=true rc=0 result=50000005000000 elapsed=4.27/5.90/4.98 s` | **FINDING (F2)** | P1 |
| CB2 | `prod(1..200000)` (973 351-digit factorial, ≈50 s) with a 100 ms budget, ×2 | `--cancel-after 100` | `ok=true rc=0 elapsed=57.73 s / 47.1 s`; result identical to the un-budgeted run | **FINDING (F2)** | P1 |
| CB3 | `det(eye(400)*2)` (2.7 s) with a 100 ms budget, ×2 | `--cancel-after 100` | `ok=true rc=0 elapsed=2.70/2.81 s result=2^400` | **FINDING (F2)** | P1 |
| CB4 | 300 000-statement flat script with a 50 ms budget, ×2 | `--cancel-after 50` | `rc=1 code=Cancelled elapsed=218.9/215.2 ms timings=[] partialOutput=[]`, wall 305/297 ms | **FINDING (F3)** | P1 |
| CB5 | `sum(1..2000000)` with a 60 000 ms budget (budget ≫ work), ×2 | `--cancel-after 60000` | 2× `ok=true rc=0 hash=43f9aba014e8b67c` (same hash as T08's un-budgeted run) | HELD | — |
| CB6 | trivial `1+1` with a 1 ms budget, ×20 | `--cancel-after 1` | 20× `ok=true rc=0 elapsed=62–99 µs` — no spurious cancellation | HELD | — |
| CB7 | `print("A"); print("B"); sum(1..10000000)` with a 150 ms budget, ×10 | `--cancel-after 150` | 10× `ok=true rc=0 out=["A","B"] elapsed≈5 s` | **FINDING (F2)** | P1 |
| CB8 | `a=1; b=2; c=sum(1..10000000)` with a 150 ms budget, ×10 | `--cancel-after 150` | 10× `ok=true rc=0 vars=['_','a','b','c'] elapsed≈5 s` | **FINDING (F2)** | P1 |
| CB9a | `--cancel-after 0` | `--cancel-after 0` | `rc=2`, stdout **empty**, stderr `Error: --cancel-after requires a positive millisecond count.` + usage | HELD | — |
| CB9b | `--cancel-after -1` | `--cancel-after -1` | as CB9a | HELD | — |
| CB9c | `--cancel-after abc` | `--cancel-after abc` | as CB9a | HELD | — |
| CB9d | `--cancel-after 2147483647` (Int32.MaxValue) | `--cancel-after 2147483647` | `rc=0 ok=true hash=c15d39bd475c88d2` | HELD | — |
| CB9e | `--cancel-after 2147483648` (one past Int32) | `--cancel-after 2147483648` | `rc=2`, empty stdout, same usage error | HELD | — |
| CB9f | `--cancel-after` with no value | `--cancel-after` | `rc=2`, empty stdout, `requires a millisecond argument.` | HELD | — |
| CB10 | `print("s1"); sum(1..2000000); print("s2"); sum(1..2000000); print("s3")` with a 1200 ms budget, ×8 | `--cancel-after 1200` | 1× `timings=2 partialOutput=["s1"]`; 7× `timings=4 partialOutput=["s1","s2"]`. Output is a **prefix** of the statement prints and `len(timings)` equals the number of statements started in every run | INCONCLUSIVE (cut point varies run to run — inherent to a wall-clock budget; each envelope is internally consistent) | P3? |
| CB11 | `a=1; b=2; c=sum(1..10000000); print("done")` with a 1000 ms budget, ×3 | `--cancel-after 1000` | 3× `rc=1 code=Cancelled elapsed=5.03/5.08/5.05 s timings=3 partialVariables=[a=1,b=2,c=50000005000000]` — cancellable only **after** the uncancellable 5 s sum | **FINDING (F3)** | P1 |
| CB12 | `matmul(eye(400),eye(400))` (7.5 s) with a 100 ms budget, ×2 | `--cancel-after 100` | `ok=true rc=0 elapsed=7.48/7.20 s`, `hash=1d55175a93e0ebf1` | **FINDING (F2)** | P1 |
| CB12b | same script, no budget (control for CB12) | (no cancel flag) | `ok=true elapsed=7.28 s`, `hash=1d55175a93e0ebf1` — **the budgeted envelope is byte-identical to the un-budgeted one** | HELD (control) | — |
| CB13 | `inv(eye(300)*2)` (30 ms, shorter than the budget) with a 100 ms budget, ×2 | `--cancel-after 100` | `ok=true rc=0 elapsed=29.8/33.4 ms` | HELD | — |
| CB14 | `conv(1..50000, 1..1000)` (8.2 s) with a 100 ms budget, ×2 | `--cancel-after 100` | `ok=true rc=0 elapsed=8.04/8.40 s` | **FINDING (F2)** | P1 |
| CB15 | `prod(1..20000)` (355 ms) with a 50 ms budget, ×2 | `--cancel-after 50` | `ok=true rc=0 elapsed=354.5/369.1 ms` | **FINDING (F2)** | P1 |
| CB16 | 300 000-statement flat script with a 1 ms budget, ×2 | `--cancel-after 1` | `rc=1 code=Cancelled elapsed="0 ns" elapsedTime={"value":0,"unit":"ns"} timings=[]`, wall **93.6 / 105.8 ms** | **FINDING (F4)** | P2 |
| CB17 | syntax error `1+` with a 1 ms budget, ×6 | `--cancel-after 1` | 6× `rc=1 code=InvalidOperation category=DomainError elapsed≈150 µs` | HELD | — |
| CB18 | syntax error `1+`, no budget, ×6 | (no flag) | 6× `hash=f958aa097def2d52` — identical to CB17: a parse error is never masked by a deadline | HELD | — |
| CB19 | `sum(1..10000000)` with a **1 ms** budget, ×8 | `--cancel-after 1` | 8× `ok=true rc=0 result=50000005000000 elapsed=4.88–6.02 s` — 5000× the requested budget | **FINDING (F2)** | P1 |
| CC1 | `expand((x+1)^4000)` with a 5 ms budget, ×2 | `--cancel-after 5` | `rc=1 code=Cancelled elapsed=22.2/18.1 ms timings=2` — a kernel that polls *does* stop | HELD | — |
| CC2 | same `expand` workload with no budget (control) | (no flag) | `rc=1 code=BudgetExceeded elapsed=2.47 s` (the symbolic engine's own budget) | HELD (control) | — |

### 1.4 Deep nesting, runtime-built depth, resource (D / RT / MEM)

| id | what was probed | command | raw output (abridged) | verdict | severity |
|---|---|---|---|---|---|
| D250 | nested parentheses, depths 250–254 (boundary bisect) | `--file D250.ls` … `DN254.ls` | 5× `rc=0 ok=true hash=d9942521ac8ba59f` | HELD | — |
| D256 | 256 nested parentheses, and the cliff itself | `--file D256.ls`, `DN255.ls` | **254 accepted, 255 refused**: `rc=1 code=DepthExceeded category=BudgetExceeded message="expression nesting depth 257 exceeds the maximum supported nesting depth of 256…"` | HELD | — |
| D257 | one past the paren budget | `--file D257.ls` | as D256 | HELD | — |
| D258 | two past | `--file D258.ls` | as D256 | HELD | — |
| D300 | 300 nested parentheses | `--file D300.ls` | `rc=1 code=DepthExceeded`, wall 33 ms | HELD | — |
| D2000 | 2 000 nested parentheses | `--file D2000.ls` | `rc=1 code=DepthExceeded`, wall 33 ms | HELD | — |
| D100000 | 100 000 nested parentheses | `--file D100000.ls` | `rc=1 code=DepthExceeded`, wall 46 ms — no crash | HELD | — |
| A513 | 513 nested array literals `[[[…1…]]]` | `--file A513.ls` | `rc=1 code=DepthExceeded` | HELD | — |
| A514 | 514 nested array literals | `--file A514.ls` | `rc=1 code=DepthExceeded` | HELD | — |
| A50000 | 50 000 nested array literals | `--file A50000.ls` | `rc=1 code=DepthExceeded`, wall 38 ms | HELD | — |
| C600 | 600 nested `sin(` calls | `--file C600.ls` | `rc=1 code=DepthExceeded` | HELD | — |
| C5000 | 5 000 nested calls | `--file C5000.ls` | `rc=1 code=DepthExceeded` | HELD | — |
| U257 | 257 unary minus signs | `--file U257.ls` | `rc=1 code=DepthExceeded` | HELD | — |
| U5000 | 5 000 unary minus signs | `--file U5000.ls` | `rc=1 code=DepthExceeded` | HELD | — |
| RT2000 | 2 000 runtime wraps `x = 1; x = [x]; ×2000` | `--omit-variables` | `rc=0 ok=true kind=Array timings=2001 elapsed=208 ms`, wall 358 ms | HELD | — |
| RT4k | 4 000 runtime wraps | as above | `rc=0 ok=true timings=4001`, wall 504 ms | HELD | — |
| RT8k | 8 000 runtime wraps | as above | `rc=0 ok=true timings=8001`, wall 1 525 ms | HELD | — |
| RT12k | 12 000 runtime wraps | as above | `rc=3221225725`, **stdout 0 bytes**, stderr `Process is terminating due to StackOverflowException.` | **FINDING (F1)** | P0 |
| RT20000 | 20 000 runtime wraps | as above | `rc=3221225725`, **stdout 0 bytes**, stderr `Process is terminating due to StackOverflowException.`, wall 18.8 s | **FINDING (F1)** | P0 |
| RT2a | 20 000 runtime wraps, clean file, confirmation 1 | as above | `rc=3221225725`, stdout 0 bytes, wall 16.1 s | **FINDING (F1)** | P0 |
| RT2b | 20 000 runtime wraps, clean file, confirmation 2 | as above | `rc=3221225725`, stdout 0 bytes, wall 15.5 s | **FINDING (F1)** | P0 |
| RTG2000 | 2 000 runtime `x = x + k` (deep add tree) | as above | `rc=0 ok=true kind=Natural`, wall 72 ms | HELD | — |
| RTC2000 | 2 000 runtime `x = sin(x)` (deep call tree) | as above | `rc=1 code=DepthExceeded elapsed=10.55 ms timings=257` — the runtime depth guard *does* cover expression trees | HELD | — |
| RT20k-void | 20 000 wraps then `print(1)` (last value Void, variables omitted) | as above | `rc=3221225725`, stdout 0 bytes, wall 13.7 s | **FINDING (F1)** | P0 |
| RT20k-print | 20 000 wraps then `print(x)` | as above | `rc=3221225725`, stdout 0 bytes, wall 14.8 s | **FINDING (F1)** | P0 |
| MEM1 | peak working set of `sum(1..10000000)` (10 M-element vector) | `--omit-functions --omit-variables`, sampled every 50 ms | `peakWorkingSet = 1 221.7 MB`, wall 8 427 ms | INCONCLUSIVE (no documented memory limit to compare against) | — |

### 1.5 Mathematics against independent ground truth (GT) and timing-field integrity (TI)

| id | what was probed | command | raw output (abridged) | verdict | severity |
|---|---|---|---|---|---|
| GT1 | `sum(1..10000000)` vs n(n+1)/2 | CB1/CB19 envelope | `got=50000005000000 want=50000005000000` | HELD | — |
| GT2 | `sum(1..2000000)` vs n(n+1)/2 | W3 envelope | `got=2000001000000 want=2000001000000` | HELD | — |
| GT3 | `det(eye(400)*2)` vs Python `2**400` | CB3 envelope | `got=2582249878086908589655919172003011874329705792829223512830659356540647…` identical 121-digit string | HELD | — |
| GT4 | `matmul(eye(400),eye(400))` shape | CB12b envelope | `got=[400, 400] want=[400, 400]` | HELD | — |
| GT5 | `prod(1..20000)` vs Python `math.factorial(20000)` | W4 envelope | exact string equality | HELD | — |
| GT6 | `prod(1..200000)` digit count vs Python | W6 envelope | `got=973351 want=973351` | HELD | — |
| GT7 | `prod(1..200000)` full value vs Python `math.factorial(200000)` | W6 envelope | `exact equality = True`; first 30 digits `142022534547031440496694633368` and last 30 all zeros match | HELD | — |
| TI1 | contract invariant 4: `elapsed` string and `elapsedTime` structural pair "can never disagree" | all 201 saved envelopes | `envelopes checked=201 unparsable=16 elapsed/elapsedTime mismatches=0` (the 16 unparsable are the 10 usage-error runs with empty stdout + the 6 stack-overflow deaths) | HELD | — |
| TI2 | unit-selector boundary: no value should be emitted in a unit where it would exceed 1000 | all `timings[].elapsed` across 201 envelopes (416 703 entries) | `ns n=381062 min=100 max=900`; `µs n=35447 min=1 max=974.1`; `ms n=134 min=1.06 max=995.3`; `s n=60`; violations (value ≥ 1000) = **0** | HELD | — |
| TI3 | corpus scan for structurally-zero durations | all envelopes | exactly 2 envelopes carry `"elapsed": "0 ns"` / `{"value":0,"unit":"ns"}` — both are CB16 (1 ms budget), whose processes each ran ≈100 ms | **FINDING (F4)** | P2 |

---

## 2. FINDINGS

### F1 (P0) — A deeply nested array value built by repetition kills the process with a stack overflow and **no envelope at all**.

Twelve thousand or more successive `x = [x]` statements are valid input with no documented nesting limit
(the only documented limit is the 256-deep *input* expression guard, and the runtime depth guard covers
expression trees — see RTC2000), yet the process dies with `STATUS_STACK_OVERFLOW` and writes **zero bytes**
to stdout, so the machine contract "stdout carries the envelope and nothing else" degrades to "stdout carries
nothing".

Exact reproduction (script file content, then command):

```
x = 1; x = [x]; x = [x]; ... (20000 times, separated by "; ")
```
```
out\aot\Lovelace.Run.exe --file %TEMP%\b2\out\RT20000.ls --omit-functions --omit-variables
```
Observed (raw):

```
exit code      : 3221225725  (0xC00000FD, STATUS_STACK_OVERFLOW)
stdout         : 0 bytes
stderr         : Process is terminating due to StackOverflowException.
wall           : 18813 ms
```

Second and third confirmations from freshly written files (`RT2a.ls`, `RT2b.ls`): identical — exit
`3221225725`, stdout `0 bytes`, wall 16 081 ms / 15 541 ms (stderr empty in those two runs; the death
signal is the exit code). The same script with `; print(1)` appended (last value Void, variables omitted)
and with `; print(x)` appended also die the same way, so the overflow is **not** only in the final result
projection — the variables' display/state capture on the way out recurses into the nested array.

Threshold (measured): 4 000 wraps → `ok=true` (0.50 s); 8 000 wraps → `ok=true` (1.53 s);
12 000 wraps → death; 20 000 wraps → death. The 12 000–20 000 range is reproducible; the exact cliff was
not bisected further. Cost before death grows super-linearly (8 000 → 1.5 s, 12 000 → 6.8 s,
20 000 → 16 s), so an agent that trips this loses both the answer and the time.

### F2 (P1) — `--cancel-after` is silently ignored inside array/numeric kernels: the run finishes, returns `"ok": true`, and no field says the budget was exceeded.

`--help` documents `--cancel-after <ms>` as "cancel the evaluation after the given time, returning the
partial result". For a single statement whose cost lives in an array/DSP kernel, the deadline is never
consulted, so the evaluation runs to completion and the envelope reports success.

Exact reproduction (the strongest of nine independent rows):

```
script : sum(1..10000000)
command: out\aot\Lovelace.Run.exe --file %TEMP%\b2\out\CB19.ls --omit-functions --cancel-after 1
```
Raw envelope (CB1 run #0, the same shape with a 100 ms budget):

```json
{"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":true,"revision":81,
 "result":{"kind":"Natural","display":"50000005000000","typed":"50000005000000 (Natural)",
           "structured":{"kind":"Natural","value":"50000005000000","exact":true}},
 "output":[],"variables":[{"name":"_","kind":"Natural","display":"50000005000000"}],"functions":[],
 "elapsed":"4.27 s","elapsedTime":{"value":4.27,"unit":"s"},
 "timings":[{"position":0,"elapsed":{"value":4.27,"unit":"s"},"resultKind":"Natural","hasOutput":false}]}
```
Eight runs at `--cancel-after 1` (CB19) all returned `ok=true` with the full answer after 4.88–6.02 s —
**up to 5 000× the requested budget**. The other reproductions (nine finding rows in total): CB1
(`sum(1..10000000)`, budget 100 ms, 3 runs, 4.27–5.90 s), CB2 (`prod(1..200000)`, budget 100 ms, 2 runs,
47.1 s and 57.7 s, 973 351-digit factorial delivered complete), CB3 (`det(eye(400)*2)`, budget 100 ms,
2 runs, 2.70/2.81 s), CB7 (budget 150 ms, 10 runs, all `out=["A","B"] ok=true` after ≈5 s), CB8
(same, 10 runs), CB12/CB14/CB15 (`matmul`, `conv`, `prod`). CB12 with `--cancel-after 100` produced
**the byte-identical scrubbed envelope of the un-budgeted control** CB12b
(`hash=1d55175a93e0ebf1`, 7.5 s), and CB5 vs T08 likewise.

Root cause (source, not inference): the ambient token is polled only where a loop opts in. The only
`Cancellation.ThrowIfCancellationRequested` call sites are `Lovelace.Suite/Interpreter.cs:243` (statement
loop), `Lovelace.Natural` (4 sites), `Lovelace.Symbolics` (Expand, Simplify, Rewriting, Solve, Diff,
Integrate, Groebner) and `Lovelace.MathIR/Evaluator.cs`. There is **no reference to `Cancellation` /
`CancellationToken` anywhere in `Lovelace.Array`, `Lovelace.Dsp`, `Lovelace.Real`, `Lovelace.Integer`,
`Lovelace.Rational`, `Lovelace.Complex` or `Lovelace.Statistics`. CC1 confirms the other side of the
contrast: `expand((x+1)^4000)` with `--cancel-after 5` returns `Cancelled` in 18–22 ms.

Consequence for a consumer: the envelope contains no budget field and no "budget exceeded" marker, so an
agent loop that relies on the deadline for liveness cannot distinguish "finished" from "the deadline was
never enforced".

### F3 (P1) — The deadline is only honoured at poll points, so cancellation latency is bounded by the longest unpolled region (tokenising/parsing a large script, or one uncancellable statement); the same clause as F2, a different unpolled region.

Exact reproduction A (parse phase, 300 000-statement script, budget 50 ms):

```
script : a0 = 1; a1 = 1; ... (300000 times)
command: out\aot\Lovelace.Run.exe --file %TEMP%\b2\out\CB4.ls --omit-functions --omit-variables --cancel-after 50
raw    : {"ok":false,"code":"Cancelled","category":"BudgetExceeded",
          "message":"the evaluation was cancelled by the caller.","recoverable":true,"diagnostics":[],
          "elapsed":"218.9 ms","elapsedTime":{"value":218.9,"unit":"ms"},"timings":[],
          "partialOutput":[],"partialVariables":[]}
wall   : 305 ms      (budget 50 ms → 4.4–6.1× over; the second run: elapsed 215.2 ms, wall 297 ms)
```
`"timings":[]` proves no statement ran: the whole 218.9 ms was tokenise + parse + depth guard, none of
which polls. The overrun grows with script size (a 3 MB script parses proportionally longer).

Exact reproduction B (post-hoc cancellation, budget 1000 ms, ≈5 s uncancellable sum):

```
script : a = 1; b = 2; c = sum(1..10000000); print("done")
command: ... --cancel-after 1000
raw    : {"ok":false,"code":"Cancelled","elapsed":"5.03 s","timings":[3 entries],
          "partialOutput":[], "partialVariables":[{"name":"a","display":"1"},{"name":"b","display":"2"},
          {"name":"c","display":"50000005000000"}]}
3 runs : elapsed 5.03 / 5.08 / 5.05 s, identical scrubbed envelope
```
The promised "partial result" is kept intact here (a, b and even the completed c are present, and
`partialOutput` is a correct prefix), but the cancellation arrives 5× late: the deadline is applied only
when the *next* statement starts polling.

### F4 (P2) — A cancelled run can report `"elapsed": "0 ns"` for a process that took ≈100 ms, because the elapsed field is never assigned on the pre-gate cancellation path.

Exact reproduction (300 000-statement script, budget 1 ms):

```
command: out\aot\Lovelace.Run.exe --file %TEMP%\b2\out\CB16.ls --omit-functions --omit-variables --cancel-after 1
raw run 1: {"ok":false,"code":"Cancelled","category":"BudgetExceeded","elapsed":"0 ns",
            "elapsedTime":{"value":0,"unit":"ns"},"timings":[],"partialOutput":[],"partialVariables":[]}   wall 93.6 ms
raw run 2: identical envelope                                                                              wall 105.8 ms
```
The very same script with a 50 ms budget reports `"elapsed":"218.9 ms"` (CB4), so the duration field
flips between a truthful 218.9 ms and a structural 0 ns depending on where the deadline lands — the timer
can fire before `SuiteEngine.EvaluateAsync` acquires its gate, and `LastElapsed` (assigned only in the
inner `finally`) is then still the initial `TimeSpan.Zero`. A consumer reading `elapsedTime` to account
for the run is told the engine spent no time at all. Corpus scan TI3 found exactly these 2 envelopes with
a zero duration out of 201.

---

## 3. COULD NOT DECIDE

* **CB10 — where a wall-clock budget cuts a multi-statement script.** The same script + 1200 ms budget
  produced `timings=2 partialOutput=["s1"]` once and `timings=4 partialOutput=["s1","s2"]` seven times.
  The *outcome* is nondeterministic (a function of machine speed against a wall clock), but every envelope
  was internally consistent (prints are a prefix, `len(timings)` = statements started, variables present
  exactly for completed statements). The protocol promises nothing about the cut point, so I scored the
  individual probes as HELD/INCONCLUSIVE rather than a finding; the observation stands as P3 material.
* **MEM1 — 1.22 GB peak working set** for `sum(1..10000000)` (a 10 M-element vector, ≈122 bytes per
  element). There is no documented memory limit or expected footprint in the protocol, so I cannot say
  whether this is a defect or the intended cost model.
* **Orphan processes.** One sample taken immediately after a heavy concurrency probe saw a single lingering
  `Lovelace.Run` process; two follow-up checks (5 completed + 5 cancelled runs, sampled at +0 s, +2 s and
  +5 s) all reported **0** runner processes, so I could not reproduce a process leak and do not claim one.
* **Load sensitivity beyond 8-way contention.** T06 ran the same script under 8 CPU burners on a 24-core
  box with an unchanged envelope; a fully saturated machine (24+ burners, or an artificially throttled
  process) was not swept. Only timing fields changed under load in every probe I ran.
* **The normative status of `--cancel-after`.** `docs/symbolics/dsh-protocol.md` does not mention
  `--cancel-after` at all (audit1 F4 already recorded this); F2/F3 rest on the runner's own `--help`
  sentence. If the GUI/CLI help is judged non-normative, F2/F3 drop from P1 to P2.
* **Correctness of the deep values that *did* survive.** RT4k/RT8k returned `ok=true` with
  `--omit-variables`, so I verified only status and timing, not that the 4 000-deep array's structured
  projection is the expected nesting.
* **Out-of-scope observation, recorded only because it touches "state leakage".** In the T12 family,
  `x = symbol("x"); assume(x > 0); solve_full(x^2 - 4 == 0, x)` still returns `status=Solved,
  completeness=Complete, solutions=[-2, 2]` — i.e. the assumption neither contracts the set nor survives
  into the answer. That is a symbolic-mathematics question, not a temporal one, and audit1's remit; the
  temporal conclusion I can state is only that nothing leaks *between* statements or runs.

---

## 4. Summary

I ran **86 probe rows / 307 runner invocations** (plus a re-validation pass over all 201 saved envelopes),
attacking the runner over time and repetition: 20-run and 12-run repetitions of solve, print-only and mixed
scripts; 4-way and 8-way concurrent runners; the same script under 8 CPU burners; 10 000-print scripts; a
19-point precision sweep of `pi` and `e` in both orders to expose first-run/later-run caches; the
`Sqrt2Half/Sqrt3Half` special-angle caches; assumption ordering; a full budget-argument boundary sweep;
the 27 cancellation rows (budgets, boundary arguments, controls); the documented 256-deep input guard in four shapes; and runtime-built
nesting. **65 rows HELD, 19 rows FINDING (grouped into 4 findings: one P0, two P1, one P2), 2 INCONCLUSIVE.**
The determinism story is genuinely strong — the same script produces byte-identical envelopes after
scrubbing `revision`/`elapsed`/`elapsedTime`/`timings` across 20 sequential runs, 8 concurrent
processes, and an 8-way loaded machine, `pi`/`e`/trig are identical whether computed fresh at precision N
or truncated from a 300-digit cache, timing fields never disagree with their human form and never cross a
unit boundary, and `200000!` matches Python exactly at 973 351 digits. What breaks is the **time** axis:
`--cancel-after` is enforced only in the kernels that opted into polling, so nine probes show budgets
ignored with `ok:true` (up to 5 000× over), three rows (CB4, CB11, CB16) show cancellation arriving late or
mis-reported because the parse phase, the array kernels and the pre-gate path never poll, two envelopes
show a `0 ns` elapsed for a ≈100 ms process — and twelve
thousand repetitions of `x = [x]` kill the process with a stack overflow and **no envelope on stdout at
all**, which is the one result the protocol cannot express and an agent cannot recover from.

*Audit tooling and all raw envelopes: `%TEMP%\b2\` (spec files, `h.py`, `prec.py`, `final.py`,
`gt.py`, `show.py`, `out\<id>-NN.json`).*
