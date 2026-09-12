# Cycle-6 · round-21 · audit K — the product assemblies embedded as a library

**Claim under test:** the product assemblies (Lovelace.Symbolics, Suite, Real, Complex, Dsp, MathIR,
Abstractions, Natural, Integer, Rational, Array, Representation) behave correctly when embedded as a
library — many evaluations in one process, shared across threads, reused after cancellation, re-created
after disposal, at different precisions.

**Verdict: FALSIFIED.** Three **P1** findings and one **P2**. Every earlier wave in this project went
through the CLI, where each run is a *fresh process*; the three P1s are all engine state that a fresh
process cannot carry, which is exactly the delta this strategy was chosen to expose.

Severity ladder used below: **P0** = a wrong value or an abort/crash on valid input; **P1** = a false
claim in the machine API, a wrong refusal, or data the API says it carries being lost or altered;
**P2** = cosmetic/documentation.

---

## Method

* **Object under test — the built product assemblies**, referenced directly: no project reference, no
  CLI, no repo source in the test process.

  | assembly | bytes | written |
  |---|---|---|
  | Lovelace.Abstractions.dll | 35 840 | 2026-09-12 18:49:04 |
  | Lovelace.Array.dll | 21 504 | 2026-09-12 18:49:07 |
  | Lovelace.Natural.dll | 28 160 | 2026-09-12 18:49:11 |
  | Lovelace.Integer.dll | 14 848 | 2026-09-12 18:49:12 |
  | Lovelace.Rational.dll | 10 752 | 2026-09-12 18:49:12 |
  | Lovelace.Real.dll | 55 296 | 2026-09-12 18:49:12 |
  | Lovelace.Complex.dll | 24 064 | 2026-09-12 18:49:13 |
  | Lovelace.Symbolics.dll | 358 400 | 2026-09-12 18:49:14 |
  | Lovelace.Dsp.dll | 45 568 | 2026-09-12 18:49:21 |
  | Lovelace.MathIR.dll | 50 176 | 2026-09-12 18:49:22 |
  | Lovelace.Suite.dll | 244 736 | 2026-09-12 18:49:22 |
  | Lovelace.Run.dll | 203 776 | 2026-09-12 18:50:23 |

  All from Lovelace.Run/bin/Release/net10.0/. The CLI executable used **only as a reference oracle**
  (never as the system under test) is Lovelace.Run/bin/Release/net10.0/Lovelace.Run.exe.

* **Harness** — a scratch console project **outside the repo tree**, at
  C:\Users\ricar\dev\.lovelace-libaudit\ (SDK 10.0.103, net10.0, 24 logical processors, Windows
  10.0.26100). The repo tree was not modified. Host wiring is the Runner's own
  (Lovelace.Run/Runner.cs, the plugin wiring inside RunAsync — currently :180-185):

```
  static SuiteEngine New() {
    var e = new SuiteEngine();
    e.LoadPlugin(new Lovelace.Dsp.DspPlugin());
    var sym = new Lovelace.Symbolics.SymbolicsPlugin();
    e.LoadPlugin(sym);
    e.LoadPlugin(new Lovelace.MathIR.MathIRPlugin(sym));
    return e;
  }
  // exactly what Runner.cs publishes as "display" (engine.FormatValue(result), :256)
  static string V(SuiteEngine e, Value v) => e.FormatValue(v);
```

* **Ground truth for numeric claims:** $env:PATH='C:\Users\ricar\dev\.lovelace-tools\python;'+$env:PATH,
  then python with mpmath (SymPy 1.14.0 distribution), mp.dps = 1400.

* **Reproduction rule.** Every finding below was reproduced **at least twice**, in a fresh process each
  time: the harness binary is re-run, not merely re-called. Probe sources:
  C:\Users\ricar\dev\.lovelace-libaudit\Attacks3.cs (LE, PS, SC), Attacks5.cs (EXACT, CLI),
  Attacks6.cs (REPRO, NEST, OUT), Attacks7.cs (CQ, SWEEP).

* **The tree moved during the audit.** Lovelace.Run/Runner.cs and its DLL were rebuilt by another agent
  while I worked (Runner.cs 18:52:25, Lovelace.Run.dll 18:50:23 then 18:52:28); citations to Runner.cs are
  therefore given by SYMBOL as well as by the line number seen at audit time. This does not touch any
  finding: **Lovelace.Suite.dll (18:49:22), Lovelace.Symbolics.dll (18:49:14), Lovelace.Real.dll
  (18:49:12) and Lovelace.Abstractions.dll (18:49:04) were unchanged throughout**, and every finding
  K-1..K-4 lives in those assemblies. Lovelace.Run appears only as the CLI reference oracle.

---

## K-1 — **P1** — a prior evaluation's setprecision turns a legal pi(500) into a refusal, and does so nondeterministically under concurrency

**New?** **NEW.** Not in the known-open list and not in any prior audit: the known-open evalf item is the
*1000-place clamp on the value*; this is a *refusal of a different, valid script* caused by leftover
engine state.

### Exact program

C:\Users\ricar\dev\.lovelace-libaudit\Attacks3.cs, method LE (sequential form), and
C:\Users\ricar\dev\.lovelace-libaudit\Attacks7.cs, method CQ (concurrent form):

```
// LE — sequential, ONE engine, two evaluations
var fresh = H2.New();
var r1 = H.Eval(fresh, "pi(500)");                    // evaluation #1 only

var used = H2.New();
H.Eval(used, "setprecision(20); 1");                  // an UNRELATED evaluation
var r2 = H.Eval(used, "pi(500)");                     // evaluation #2: reads state, sets nothing

// CQ — the same thing with two concurrent callers on one engine
var e = H2.New();
var tA = Task.Run(() => H.Eval(e, "setprecision(20); pi(20)"));
var tB = Task.Run(() => { var r = H.Eval(e, "pi(500)"); /* record r */ });
Task.WaitAll(tA, tB);
```

### Observed output (verbatim, reproduced twice)

```
--- LE1 ---
  rep0 fresh engine  pi(500) -> ok digits=101
  rep0 after setprecision(20) in an UNRELATED evaluation, pi(500) -> ArgumentOutOfRangeException:
        Specified argument was out of the range of valid values. (Parameter 'digits')
  rep0 engine now: comp=20 display=20
  rep1 fresh engine  pi(500) -> ok digits=101
  rep1 after setprecision(20) in an UNRELATED evaluation, pi(500) -> ArgumentOutOfRangeException: ...
--- LE3 ---
  after setprecision(20): e(500) -> ArgumentOutOfRangeException: ...
  fresh engine           : e(500) -> ok digits=101
--- CQ (six consecutive fresh processes of the harness) ---
  rep0..rep5  B pi(500) -> ArgumentOutOfRangeException: Specified argument was out of the range of valid values. (Parameter 'digits')
--- CQ-control: the SAME engine, sequential, without the setprecision evaluation ---
  pi(500) -> ok digits=101
```

An earlier CQ run of the identical program produced rep0 ok digits=101, rep1 refused, rep2 refused — so
under concurrency the verdict depends on the interleaving. That is claim 1 ("interleaving two evaluations
must not change either") and claim 3 ("look for wrong values, exceptions, or silent corruption") failing
together: the exception is *not* the documented typed refusal, and **2 of 3** concurrent runs of a valid
script were refused while **3 of 3** sequential runs were.

### Correct behaviour, and how I know

* The CLI — where each run is a **fresh process** — has no such history. Three separate processes:

```
  CLI pi(500)                    -> ok display=3.1415926535897932384626433832...
  CLI setprecision(20); pi(20)   -> ok display=3.14159265358979323846
  CLI (2nd process) pi(500)      -> ok display=3.1415926535897932384626433832...
```

  So pi(500) is a **valid** script by the product's own published behaviour, and the library refuses it.
* The mechanism, from the source: setprecision(n) writes the engine field permanently
  (Lovelace.Suite/Interpreter.cs:1827-1841; SetPrecision at :164-168), and *every later* evaluation
  re-enters the precision scope from that field (Interpreter.cs:321,
  "using var statementScope = Rl.WithPrecision(ComputationDecimalPlaces, DisplayDecimalPlaces);"), while
  Real.PiTo refuses digits > MaxComputationDecimalPlaces (Lovelace.Real/Real.cs:1403-1406). The refusal
  is therefore *internally consistent* — the defect is that a later evaluation which **sets nothing** is
  governed by another evaluation's leftovers, and the caller has no way to detect that.
* The correct behaviour is one of: (a) a script's setprecision is evaluation-scoped rather than
  engine-persistent, or (b) the capability is documented and reportable at the library boundary. Today
  neither holds: SuiteEngine exposes the *numeric* consequence (ComputationDecimalPlaces) but no flag
  saying "a script changed this", so the host cannot even snapshot-and-restore the intended budget.

---

## K-2 — **P1** — SuiteEngine.ProjectValue silently truncates the structured Value to display precision, while the CLI publishes all of it

**New?** **NEW as a library finding.** The *CLI* half of this defect was found and fixed in an earlier
cycle (b3b74b8, "the structured payload's silent 100-decimal cut",
docs/symbolics/a-plus-cycle-6-report.md:23) — but the fix landed only on the Runner's own path.
SuiteEngine.ProjectValue, the façade the class documentation points hosts at, still has it.

### Exact program

C:\Users\ricar\dev\.lovelace-libaudit\Attacks6.cs, method REPRO (section A), and Attacks5.cs, method
EXACT:

```
var e = H2.New();
e.ComputationDecimalPlaces = 1100; e.DisplayDecimalPlaces = 100;   // the API allows these to differ
var v = e.EvaluateAsync("pi(1100)").GetAwaiter().GetResult();

string facade = JsonSerializer.Serialize(e.ProjectValue(v));      // SuiteEngine.cs:115-120
string runner;
using (var _ = Real.WithPrecision(e.ComputationDecimalPlaces, 1_000_000_000L))
  runner = JsonSerializer.Serialize(StructuredProjection.ToStructured(v, null));  // Runner.cs StructuredValue, :570-573
```

### Observed output (verbatim, reproduced twice, two independent engines)

```
--- REPRO-A ---
  rep0 ProjectValue Value len=102  Truncated=null | Runner-style len=1102 Truncated=null
  rep1 ProjectValue Value len=102  Truncated=null | Runner-style len=1102 Truncated=null
--- EXACT (default engine: comp=1000, display=100) ---
=== pi
  ProjectValue : Kind=[Real] Exact=False Trunc=null Reason=null Budget=null Num=null Den=null Value=[[3.1415926535897932384626433832795028841...](len 104)
  Runner-style : Kind=[Real] Exact=False Trunc=null Reason=null Budget=null Num=null Den=null Value=[[3.1415926535897932384626433832795028841...](len 1004)
  *** DIVERGE ***
=== sqrt(2)
  ProjectValue : ... Value=[[1.4142135623730950488016887242096980785...](len 104)
  Runner-style : ... Value=[[1.4142135623730950488016887242096980785...](len 1004)
  *** DIVERGE ***
```

(len counts the two bracket characters the probe adds, so the strings are "3." + 100 digits versus
"3." + 1000 digits, and "3." + 100 versus "3." + 1100 in REPRO-A.)

The two projections agree on Exact, on Numerator/Denominator, and on periodic values (1/7 → 0.(142857),
1/1009 → a 252-digit block, both scopes byte-identical) — the loss is **only** in the non-periodic Value
digits, which is precisely where the digits are expensive.

### Correct behaviour, and how I know

The product's own protocol contract, stated in the Runner's fix for this exact defect
(Lovelace.Run/Runner.cs, the StructuredValue comment above the StructuredDecimalDigits constant,
currently :557-573): "The display bound stays a DISPLAY bound: display/typed are rendered at
the engine's precision by the caller, **the structure is rendered in full**", and the truncation fields
exist so that "a bounded structured rendering [says] so". The Runner therefore projects the structure
under StructuredDecimalDigits = 1_000_000_000L and never loses a digit.

SuiteEngine.ProjectValue instead calls Rl.WithPrecision(ComputationDecimalPlaces, DisplayDecimalPlaces)
(Lovelace.Suite/SuiteEngine.cs:118) — the display bound — so a host that follows the façade loses 1000 of
the 1100 digits **the same value carries**, and the DTO reports Truncated: null,
TruncationReason: null, i.e. the machine API states that nothing was lost. Two public entry points in one
product publish different payloads for one computation. I also verified the loss end-to-end against the
CLI: "setprecision(1100); pi(1100)" publishes the full 1100-digit structured value on the wire.

Honest caveat: ProjectValue's own doc-comment does say "under this engine's display precision", so the
*behaviour* matches its own wording. The finding stands on the two grounds that survive that wording:
(i) the product publishes two disagreeing structured renderings of one value, and (ii) Truncated and
TruncationReason are null, which is a false statement in the machine API regardless of which precision
was intended.

---

## K-3 — **P1** — one plugin instance loaded into two engines leaks assumptions, and the second engine returns a wrong Boolean

**New?** **NEW.** Lovelace.Suite.Tests/CrossEngineIsolationTests.cs pins cross-engine isolation, but it
builds a **fresh plugin per engine** (:14-19), so it never exercises the shared-instance path.

### Exact program

C:\Users\ricar\dev\.lovelace-libaudit\Attacks3.cs, method PS; reproduction in Attacks6.cs, method REPRO
(section B):

```
var sym = new Lovelace.Symbolics.SymbolicsPlugin();   // ONE instance
var e1 = new SuiteEngine(); e1.LoadPlugin(sym);
var e2 = new SuiteEngine(); e2.LoadPlugin(sym);       // accepted, no warning

H.Eval(e1, "x = symbol(\"x\"); assume(x > 0)");       // e1 assumes; e2 never does
var b = H.Eval(e2, "x = symbol(\"x\"); simplify(x > 0)");   // e2's OWN answer
var c = H.Eval(e2, "x = symbol(\"x\"); simplify(x < 0)");

var ctl = H2.New();                                   // control: fresh plugin per engine
var d = H.Eval(ctl, "x = symbol(\"x\"); simplify(x > 0)");
```

### Observed output (verbatim, reproduced twice)

```
--- REPRO-B ---
  rep0 e1 setup=x > 0 | e2(shared plugin) simplify(x>0)=True | e2 simplify(x<0)=False | fresh-plugin control simplify(x>0)=x > 0
  rep1 e1 setup=x > 0 | e2(shared plugin) simplify(x>0)=True | e2 simplify(x<0)=False | fresh-plugin control simplify(x>0)=x > 0
--- PS3 ---
  shared DspPlugin accepted by a second engine
```

### Correct behaviour, and how I know

Engine e2 never assumed anything, so simplify(x > 0) must not answer True — the control run with a fresh
plugin per engine answers the unevaluated relation "x > 0" on the same script, and that is the answer the
product publishes everywhere else. True is a **wrong value** produced by another engine's assumption set,
and False for x < 0 is its mirror.

The API claims the opposite in writing: Lovelace.Suite/SuiteEngine.cs:206-212 — "Cross-engine isolation
is structural (each engine hosts its own plugin instances)." It is structural only if the host never
reuses an instance, and nothing enforces or reports that: the duplicate check is
_loadedPlugins.Add(plugin.GetType()) **per ModusHost** (Lovelace.Suite/ModusHost.cs:28-29), so the same
object passes into a second engine silently. The reference wiring (Runner.cs:180-184) happens to create
fresh instances (Runner.cs :180-185) and never meets this; an embedding host that caches one SymbolicsPlugin — the natural
thing to do when MathIRPlugin *requires* an instance to be handed around — gets it.

---

## K-4 — **P2** — PrecisionExplicitlySet is a one-way latch a host cannot reset

**New?** **NEW.** Behavioural consequence not exhibited; recorded as an API-shape defect.

SuiteEngine exposes PrecisionExplicitlySet with a getter only (SuiteEngine.cs:91; Interpreter.cs:158,
"{ get; private set; }"). Once any evaluation runs setprecision, the flag is permanently true, and
ModusHost.PluginPrecisionScope (ModusHost.cs:119-128) permanently stops using the fast default budget.
Restoring the numeric defaults does not restore the mode:

```
--- SWEEP2 ---
  after setprecision(1000) then restoring 1000/100: PrecisionExplicitlySet=True
  SuiteEngine public members mentioning 'Precision': SetPrecision, get_PrecisionExplicitlySet, PrecisionExplicitlySet
  PrecisionExplicitlySet setter is public: False
  Interpreter.PrecisionExplicitlySet has a public setter: False
```

**Honest scope:** I could not exhibit an observable *value* difference from the mode flag — a 33-probe
sweep of plugin/core builtins (simplify, expand, factor, diff, integrate_full, limit_full, solve_full,
nroot, det, inv, matrix_rank, linsolve_full, zeta, gamma, erf, besselj, sum_full, product_full,
trigexpand, cbrt, log, arg, conjugate, abs, real, imag, round, floor, gcd, isprime, …) reported
probes=33 differing=0. So this is **P2**: engine state a host cannot return to the fresh-engine mode,
with no observed consequence yet. It is listed because "re-created after disposal" has no in-library
answer (see *could not check*): discarding the engine is the only recovery, and there is no Dispose.

---

## What I attacked and could **not** falsify

| claim | probe | result |
|---|---|---|
| 1 · determinism, 100 calls, one engine | 7 script families (precision, polynomials, 1/7, sqrt(2), solve_full, arrays), 100 evaluations each | stable — 1 distinct result per family; same for 100 **fresh engines** per script |
| 1 · interleaving two evaluations | 6 unrelated scripts interleaved 20× on one engine (120 evaluations) against their solo results | mismatches=0 of 120 |
| 3 · one engine, N threads, same script | 8 threads × sqrt(2) | distinct=1 |
| 3 · one engine, N threads, different scripts | 8 threads × 8 distinguishable scripts × 3 reps | all correct, no corruption |
| 3 · N engines in parallel | 8 engines × sqrt(i) | each correct |
| 3 · concurrent print capture | 2 evaluations on one engine, distinct StringWriters | wA=[AAA\|], wB=[BBB\|] — no bleed |
| 3 · non-awaited second call from one flow | var t1 = e.EvaluateAsync(...); var t2 = e.EvaluateAsync(...); | both ran (2 and 4); the flow-local _inEvaluation marker does **not** leak to a sibling call — no false ReentrancyNotSupportedException |
| 4 · cancel, then reuse | "x = 1; y = pi(900); z = 3;" cancelled at 30 ms, 3 reps | EvaluationCancelledException; next evaluation correct (sqrt(2) = 61 digits, as asked); partial state exactly the statements that committed (vars=[_,x,y]), and CaptureState() agrees with engine.Revision |
| 4 · ambient cancellation leak | read Lovelace.Abstractions.Cancellation.Token in the caller's flow after a cancelled run | CanBeCanceled=False; ThrowIfCancellationRequested() does not throw — no leak |
| 4 · output writer restored | set engine.Output, cancel a run with a second writer | "engine.Output is w1 after cancel: True"; a later print reaches the writer set afterwards |
| 4 · reentrancy refusal | host builtin that calls back into its own engine | ReentrancyNotSupportedException (the documented typed refusal); engine still usable |
| 5 · re-creation resets state | fresh engine vs used engine | vars 0 vs 3, revision 80 vs 83, comp=1000 disp=100 explicit=False on both, diag=0; Real.DisplayDecimalPlaces / MaxComputationDecimalPlaces unchanged (100 / 1000) by any engine work |
| 5 · plugin double-load in one engine | second SymbolicsPlugin into one engine | refused: "Plugin 'Lovelace.Symbolics' (SymbolicsPlugin) is already loaded." — correct |
| 6 · exactness at the library boundary | StructuredProjection.ToStructured — 1/8 → Exact=true num=1 den=8; pi, sqrt(2) → Exact=false; sin(1) → Kind=Symbolic Exact=true Canonical=(fn sin (rat 1 1)) | matches the CLI's shape |
| 6 · numeric agreement with mpmath | sqrt(2), pi, e, sqrt(3), sqrt(2)+sqrt(3) — **all 1000 published digits**, mp.dps=1400 | match_mpmath=True for all five |
| 6 · periodic agreement with mpmath | 1/7 block 6, 2/3 block 1, 1/1009 block 252 | block length = **minimal** period computed by brute force (pow(10,p,d)), and block digits match mpmath |

**Checked and deliberately *not* reported:**

* pi(N) / e(N) for N greater than the engine's computation cap throws the bare
  ArgumentOutOfRangeException("digits") (pi(1001), pi(2000), pi(4000) on a default engine;
  setprecision(60); pi(61)). That is Real.PiTo's documented exception (Real.cs:1401-1406), and the CLI
  classifies the same exception identically — {"code":"InvalidArgument","category":"TypeMismatch"} for
  pi(1001) and for setprecision(60); pi(4000). Library and CLI agree, so this is **not** a divergence.
  Its *message* names neither the cap nor the engine state, which is what makes K-1 hard to diagnose,
  but the text is the same on both surfaces.
* sqrt(2)*sqrt(2) → 1.999…9 (2000 digits, Exact=false) where SymPy answers exactly 2. The engine's
  sqrt(2) is a 1000-digit Real, so the product is 2 − ε to the working precision and the engine flags it
  inexact. Honest numeric policy, not a wrong published value.
* sin(1) stays symbolic (Exact=true, no Value) while sqrt(2) becomes numeric — an internal
  representational asymmetry, not a claim in the contract.
* None of the **known-open** cluster was re-reported. The pole/branch-point series family, the
  --cancel-after stopped/exceeded pair, evalf's 1000-place clamp, --print-budget token splitting and
  non-monotonicity, and --omit-variables on the cancel path are all **CLI-surface** behaviours that this
  library-façade witness does not exercise; I saw none of them and claim no new instance of any.

---

## Findings table

| id | severity | new? | one-line repro |
|---|---|---|---|
| K-1 | **P1** | NEW | one engine: EvaluateAsync("setprecision(20); 1") then EvaluateAsync("pi(500)") → ArgumentOutOfRangeException; a fresh engine (and the CLI in a fresh process) answers pi(500); under two concurrent callers the same engine refused 2/3 in one run and 6/6 in another |
| K-2 | **P1** | NEW (library path) | e.ComputationDecimalPlaces=1100; e.DisplayDecimalPlaces=100; then e.ProjectValue(e.Evaluate("pi(1100)")) → Value has 100 digits with Truncated:null; the Runner's own projection of the same value has 1100, and the CLI publishes all 1100 |
| K-3 | **P1** | NEW | one SymbolicsPlugin instance into two SuiteEngines; engine A: assume(x > 0); engine B (never assumed): simplify(x > 0) → True, simplify(x < 0) → False; with a fresh plugin per engine the same script answers "x > 0" |
| K-4 | **P2** | NEW | EvaluateAsync("setprecision(1000); 1") then restore ComputationDecimalPlaces=1000, DisplayDecimalPlaces=100; e.PrecisionExplicitlySet stays True and has no public setter on SuiteEngine or Interpreter |

---

## Could not check

* **"Reused after disposal" is not expressible.** SuiteEngine implements no IDisposable and has no
  Dispose method (typeof(SuiteEngine).GetInterfaces() contains no IDisposable; GetMethod("Dispose")
  returns null), although it owns a SemaphoreSlim (SuiteEngine.cs:212) that is never disposed and it
  holds plugin instances whose lifetime the host cannot release. I substituted **re-creation** (a new
  engine) for disposal and report the missing dispose seam as an observation rather than a finding.
* **LastElapsed attribution under concurrency — suspicion, not reproduced.** SuiteEngine.cs:263-268
  assigns LastElapsed *after* _evaluationGate.Release(), outside the gate, and Runner.cs:264-265
  publishes it as the envelope's elapsed/elapsedTime (elapsedTime is built from engine.LastElapsedDisplay
  and the deadline ledger, Runner.cs :261-274). Two overlapping evaluations on one engine can
  therefore have the later one's LastElapsed overwritten by the earlier one's. My probe (Attacks2.T7)
  could not measure it: the heavy script I chose, pi(3000), is refused by the cap (K-1's relative), so
  the timing comparison never ran. **Unverified.**
* **The engine's diagnostics clear outside the gate on the cancel path — suspicion, not reproduced.**
  SuiteEngine.cs:270-277 clears the engine's diagnostics in the cancellation catch, which runs *after*
  the gate has been released; a concurrent evaluation's diagnostics could be wiped. A 12-rep probe of
  "cancelled evaluation concurrent with a failing one on one engine" (Attacks2.C5) found
  Diagnostics.Count=1 in 12/12, i.e. **0/12 lost**. The window is real in the source; I did not
  reproduce a loss.
* **Sustained concurrency beyond 8 callers / 12 reps.** K-1 is interleaving-sensitive; I characterised it
  at 2-8 concurrent callers and small repetition counts. Long soak tests were out of budget.
* **The other hosts.** Lovelace.Studio, Lovelace.Console and Lovelace.Knowledge.Run embed the same
  assemblies through the same SuiteEngine façade; I drove the façade directly and did not exercise those
  host processes, so any host-specific wiring defect is outside this audit.
* **Lovelace.Representation and Lovelace.Array beyond incidental use.** Both were referenced and loaded,
  but no finding rests on either and I did not drive their public types directly.
* **The known-open series / evalf / print-budget cluster.** Deliberately not re-attacked (the brief says
  not to report them as new, and they are CLI-surface behaviours); I also did not attempt to *confirm*
  them from the library side, so I make no claim about whether the library façade inherits each of them.
