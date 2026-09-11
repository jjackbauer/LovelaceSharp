# Cycle-4 adversarial audit plan

> Cycle 3's lesson: fifteen rounds of closing items reported green while four defects — including a
> documented feature that did not work at all — were live. The audit found them in minutes. This
> plan exists so Cycle 4 attacks the system *before* the final report, not after.

## Rules for every auditor

1. Run against the **re-published** AOT binary (`out/aot/Lovelace.Run.exe`) or a host built from the
   final tree. Never against a stale binary — Cycle 2 produced two false defects that way.
2. Every finding cites the **exact command** and its **observed output**. A finding without a
   reproduction is not a finding.
3. A wrong answer, a hang, a crash, an untyped failure, or a *claim the binary cannot back* is a
   finding. So is a documentation statement the binary contradicts.
4. Before reporting "the kernel is wrong", check the same expression against SymPy
   (`C:\Users\ricar\dev\.lovelace-tools\python\python3.exe`, sympy 1.14.0). The oracle corpus already
   covers six families; anything outside them is fair game.
5. Report the **negative** result too: "I attacked X with N inputs and found nothing" is a result and
   must name the inputs.

## Personas and their attack surfaces

### P1 — Numeric-boundary attacker
Attacks the arithmetic core, informed by what Cycle 4 already found (N21 Real division for divisors
just below 1, N22 `Real.ToString` on repeating expansions, N23 high-precision `cos` cost).
Probes: division by values in (0.9, 1) and near-zero; `0^-1`, `0^0`, `inf - inf`, `0 * inf`;
reciprocals of values whose expansion repeats with period 1/2/3/6; powers with negative and
fractional exponents over negative bases; the same expression evaluated at two precisions; very large
and very small exponents; values at the guard-digit boundary.

### P2 — Wire-contract attacker
Attacks the published JSON envelope. Probes: every record type the runner can emit; absent optional
fields; `null` vs omitted; the `Enum` kind on status/completeness/exactness/classification/category;
Boolean-as-string; array shape and row-major order; error envelopes (exit code, code, category);
stdout purity when the script prints; `--omit-functions` and other flags; two runs of the same script
must differ only in the volatile fields.

### P3 — Cancellation and budget attacker
Attacks long workloads (Cycle 3's OQ-002 stayed open for 30 rounds because every probe finished in
~2 ms — use a workload that runs for tens of seconds). Probes: `--cancel-after` at several
thresholds on a genuinely long computation; step budget exhaustion mid-rewrite; cancellation during
polling loops; does a cancelled run report a typed cancellation rather than a generic failure?

### P4 — Symbolic-correctness attacker
Attacks algebra, not the wire. Probes: `expand(factor(p)) == p` over hostile polynomials
(negative leading coefficients, repeated roots, rational coefficients, high degree);
`parse(pretty(e))` canonicalises identically for expressions with nested subtraction, division,
negative powers and function composition; solve-completeness claims (`status` vs `complete`) on
quadratics with negative discriminants, quartics, and provably empty sets over the requested domain;
limits at poles, branch cuts and discontinuities.

### P5 — Surface attacker
Attacks the entry points rather than the library. Probes: the REPL and the Studio HTTP API
(`/api/session`, `/api/evaluate`, `/api/run/{id}`, `/api/run/{id}/cancel`, `/api/completions`,
`/api/state`, `/api/symbolic/inspect`); the browser panels (CDP, as in `round-01/browser-check.mjs`);
hostile scripts (empty, deeply nested, unterminated strings, unicode identifiers, contradictory
assumptions, self-assignment).

### P6 — Capability-honesty attacker
Attacks `capabilities()`'s claim to be a complete statement. For every advertised unsupported
operation class, trip it live and compare code/category/message. Then attack the *other* direction:
find an operation the kernel refuses that the statement does NOT list — that is the falsifier for an
"exhaustive" claim, and it is the acceptance test for §4.1.

### P7 — Documentation-attacker (cheap, high yield)
Reads `docs/symbolics/dsh-protocol.md`, `Lovelace.Symbolics/README.md` and the alignment plan, and
tests each checkable statement against the binary. Cycle 3 found a documented cancellation claim
that was false.

## Scheduling

- P1, P4, P6 are the highest-yield (they attack what Cycle 4 changed and what the oracle exposed).
- P2, P3, P5 guard claims the final report will make.
- Run each persona as an independent agent so disagreement is informative; a finding from one persona
  is re-run by the orchestrator before it becomes an `EVD` row.
