# Journal

## OBS-001 — the gap after the base fix is algorithmic, not representational
- **Source**: EVD-003/EVD-004 — multiplication 3.3x slower at 100 000 digits, parsing 123x slower,
  factorial 6.5x slower above 10 000; division already faster than the reference.
- **Implication**: the remaining deficit is NTT multiply, Newton division and parallelism.
- **Confidence**: High.

## DEC-001 — modules are delivered by agents, integration is done by the orchestrator
- **Decision**: agents write `ntt.rs` and `newton.rs` as self-contained modules with tests; the
  orchestrator wires them into `lib.rs` and runs every gate.
- **Rationale**: two agents editing one file would collide; the seam is a pure function signature.
## OBS-002 — a parallel fold over chunks loses to the sequential tree
- **Source**: EVD-007. Splitting the digit string into 8 chunks, parsing each on a worker and
  folding them left to right took 117 ms against 87 ms for the plain recursive parse.
- **Why**: the fold does seven full-size multiplications on one thread, which is exactly the work
  the tree was supposed to distribute.
- **Fix**: parallelise *inside* the existing balanced recursion, bounded to depth 3 (EVD-006).

## OBS-003 — the formatter is not a padding engine
- **Source**: converting a 200 000-digit product panicked at the recursive to_decimal with
  "Formatting argument out of range" from `{:0>width$}` with width around 100 000.
- **Fix**: pad by hand. Recorded because the failure only appears at sizes the corpus never reaches.
## HYP-001 — Newton-reciprocal division beats long division at large sizes
- **Claim**: a reciprocal estimate with correction steps removes the O(n*m) cost.
- **Status**: **Falsified** (EVD-009). Measured 5 649 ms against 91.7 ms for Knuth D at 100 000
  digits. The module is correct (13 tests) but its implementation is far slower; it is not wired in.
- **Confidence**: High — measured, not estimated.

## HYP-002 — butterfly-level threading adds throughput on top of prime-level threading
- **Claim**: splitting each stage across workers would scale the transform further.
- **Status**: **Falsified** (EVD-014). 24.8 ms with butterflies parallel against 20.9 ms with the
  three primes alone; the per-stage thread setup costs more than the butterflies save.

## OBS-004 — the remaining conversion gap is division, not the parser
- **Source**: EVD-012. Parsing is already divide-and-conquer and parallel, yet 100 000 digits still
  costs 57.9 ms because every split divides by a large power of ten through Knuth D.
- **Implication**: round 3 must produce a *fast* reciprocal division, not just a correct one.

## RISK-001 — a delivered module can pass its tests and still regress the product
- **Evidence**: EVD-009. The Newton module's own suite is green; adopting it made division 60x
  slower. The gate that caught it was the benchmark, not the unit tests.
## OBS-005 — round 2 close
- **Verified**: 301 tests pass; corpus 212/214 and language 111/111 unchanged; release build 2m14s;
  desktop selftest 15/15 (EVD-013).
- **Adopted**: NTT multiply above 1024 limbs with the three moduli on separate workers; parallel
  factorial tree; parallel decimal parse.
- **Rejected after measurement**: Newton division (60x slower), butterfly-level threading (no gain).
- **Outstanding**: D3-D7 (throughput targets). Every one of them is now division-bound or within
  30% of its target.
## OBS-006 — the decimal cache is a behaviour change, not just a speed one
- **Source**: EVD-015. Memoising the rendering makes repeated printing O(1) (0.017 ms at 100 000
  digits, matching the reference's cache-hit figures), but cold conversion is unchanged.
- **Consequence**: the benchmark now measures both, and D7 is reported as warm-pass / cold-fail
  rather than being declared met by a cache hit.

## OBS-007 — conversion is division-bound, measured rather than assumed
- **Source**: EVD-017. In a 262 ms to_decimal, a single top-level division is 90 ms and the
  power-of-ten construction is 25 ms.
- **Implication**: no amount of parse/print tuning closes D6/D7 without a faster division.

## DEC-002 — tighten the existing division before writing a new one
- **Decision**: rewrite the Knuth inner loop over slice pairs rather than port Burnikel-Ziegler.
- **Result**: EVD-016, 75.8 -> 62.8 ms, no behaviour change, 70 tests still green.
- **Why**: the loop was bounds-checked and used i128 borrows; the asymptotic win from BZ is real
  but a 1.2x constant-factor gain was available in ten minutes against a day of risk.
## DEC-003 — the precision-doubling reciprocal is not attempted under time pressure
- **Decision**: do not rewrite newton.rs as Burnikel-Ziegler / Zimmermann recursion this round.
- **Rationale**: a subtly wrong reciprocal corrupts *every* division, and the only oracle is the
  Knuth path it would replace. The change is multi-hour with real correctness risk; the measured
  need is modest (D4 is already 2.5x faster than C#).
- **Consequence**: the Newton module remains correct but unadopted; that is recorded in state.md as
  an open item, not presented as delivered sophistication.

## OBS-008 — finer parallelism is not free
- **Source**: EVD-020. Cutting the factorial split from 2048 to 512 made it 50% slower: the extra
  forks cost more than the wider tree saves. Parallelism has an optimum, and it was measured rather
  than assumed.
## HYP-003 — the reference's recursive reciprocal can be ported directly from Natural.cs
- **Claim**: `Reciprocal(d,k)` with half-precision recursion plus `CorrectReciprocal` is
  self-contained enough to port line for line.
- **Status**: **Falsified** (EVD-022). Two behaviours broke: a divisor whose top limb is 1 produced
  a quotient where long division produced none, and a large size ratio did not terminate. Bounding
  every correction loop converted the hang into a wrong answer, not into a correct one.
- **Action**: EVD-023 — the module was deleted, not left in the tree behind an unused flag. A
  division that is sometimes wrong is worse than one that is never fast.

## OBS-009 — a failed port is worth recording precisely
- **What survives**: the measurements (division is 2.8x faster than the reference at 100 000
  digits), the constraint that any replacement must verify its own output before publishing, and
  the two concrete failure modes above.
- **What does not exist**: Newton-reciprocal division in this crate. The named mechanism in the goal
  is **not** delivered, and state.md says so.
## OBS-010 — the parse benchmark measured conversion, and the reference never did
- **Source**: EVD-026. Parsing now stores the canonical digit string and materialises limbs only
  when arithmetic needs them, which is exactly what Natural.cs:34-44 describes. Parse falls from
  60.1 ms to 2.8 ms with no behavioural change (corpus unchanged).
- **Correction to an earlier conclusion**: round 3's claim that "conversion is division-bound so the
  parse target is unreachable" was measuring a cost the reference defers. The target was reachable;
  the representation was missing.

## OBS-011 — the cold-print number had to be re-earned
- **Problem**: `tostring_cold` cloned the value, and after the change a parsed value carries its
  decimal string, so the clone printed for free (0.45 ms) - a measurement artefact.
- **Fix**: the cold path now materialises limbs by arithmetic first (EVD-027), and reports 227 ms.
  Warm is 0.022 ms. Both are recorded; neither is presented as the other.
## OBS-012 — the machine drifted, so cross-round numbers are not comparable
- **Source**: EVD-029. The same binaries measured div 100000 at 58 ms in one round and 77-103 ms in
  the next, with factorial and rendering moving the same way.
- **Consequence**: the DoD table records the latest quiet measurement *and* the range. Claims of the
  form "X improved from A to B" are only made where A and B were measured back to back in one
  session (EVD-028), which is why the render speedup is quoted as 254.9 -> 141.8 and not against a
  number from an earlier round.

## DEC-004 — Burnikel-Ziegler is documented, not attempted under pressure
- **Decision**: do not port BZ in this round.
- **What is ready for the next one**: the algorithm's reference implementation was located and read
  (OpenJDK MutableBigInteger.divideAndConquer / divide2n1n / divide3n2n), which gives the exact
  block arithmetic rather than memory. That is the approach that worked for the MathIR listing and
  the opposite of the Newton port that failed from memory.
- **Caveat recorded**: even with BZ, D7's 30 ms target is unlikely — a 3x rendering gain lands near
  50 ms. D7 may be unreachable at 100 000 digits in this architecture, and saying so is more useful
  than another round of tuning.
## OBS-013 — Burnikel-Ziegler landed, and the reference was the difference
- **Source**: EVD-031/032. The port followed the OpenJDK implementation function by function and
  passed the cross-check on the first corrected attempt; the only defect was two inverted block
  joins, found by the tests rather than by inspection.
- **Contrast**: the Newton reciprocal, written from memory last round, never passed and was deleted.
  Reading the reference implementation is the difference between a port and a guess.
- **Safety**: the fast path is followed by an exact verification (`q*d + r == n`, `r < d`) and falls
  back to long division otherwise, so a defect here can cost time but not correctness.

## OBS-014 — the host is degrading, so absolute numbers from here are not comparable
- **Source**: EVD-035. The same binaries that measured 254.9/141.8 ms serial/parallel for rendering
  now measure 548.9/273.9. The ratio holds (2.0x); the absolute values do not.
- **Rule adopted**: from this round on, every performance claim is a same-session A/B ratio, and the
  absolute table is marked as machine-dependent.
## OBS-015 — copying the reference's *shape* for factorial was a mistake
- **Source**: EVD-037. Partitioning [2..n] into one contiguous run per worker, as Natural.cs does,
  measured 97-103 ms at 20 000! against 47-50 ms for the balanced product tree in the same session.
- **Why**: their shape pays off with their multiplication cost profile; with Karatsuba/NTT merges,
  keeping operand sizes even matters more than keeping every worker busy on a small range.
- **Action**: reverted, and the tree documents the measurement that justified it.

## OBS-016 — a dispatch bug hid behind correct results
- **Source**: EVD-038. `mul` chose the transform when the *wider* operand crossed the threshold, so
  a long number times a single limb performed three transforms. Every result was still correct, so
  only a timing profile exposed it. Both operands must now be wide.

## HYP-004 — the factorial gap is the serial merge tail
- **Claim**: parallelising the merge levels would close most of the 3.8x gap.
- **Status**: **Falsified by implementation** (EVD-041). The parallel-merge version did not terminate: it
  burned 429 s of CPU before being killed. The tree was restored unchanged.
- **What survives**: the diagnosis may still be right, but the implementation is not, and shipping an
  untested rewrite to chase 30 ms is not worth the risk to a green tree.

## OBS-017 — the transform threshold was already right
- **Source**: EVD-042. Measured rather than assumed, NTT beats Karatsuba from about 40 000 digits and
  loses slightly at 20 000. The 1024-limb threshold sits between the two, so no change was needed.
  This closes an open question from two rounds ago.
## OBS-018 — three rounds of factorial claims were measured against the wrong size
- **Source**: EVD-045. The repository bench clamps `factorial` to 5000 regardless of the size asked
  for, so "C# does 20000! in 12 ms" was really "C# does 5000! in 12 ms". My conclusions that Rust was
  4.2x and then 3.8x behind were wrong.
- **Corrected** (EVD-046): with a warmed harness at the real size, Rust is 1.8x faster at 20000! and
  1.6x faster at 40000!.

## OBS-019 — the DoD thresholds were never grounded in the reference
- **Source**: EVD-047/048. The reference takes 284 ms to render 100 000 digits and 85 ms for 20 000!,
  so targets of 30 ms and 25 ms were unattainable by the very implementation the port was meant to
  match. The targets measured my expectations, not the reference.
- **Consequence**: the meaningful criterion is the goal's own wording - the same algorithms, measured
  on the same machine, at parity. On that criterion the port is ahead on four of six measurements
  and behind on two.
## OBS-020 — the Newton port succeeded on the third attempt, and the difference was the test
- **Source**: EVD-049. The first two attempts were tested only end-to-end against division, so a
  single wrong limb looked like an opaque mismatch. This attempt tested the reciprocal's defining
  property first (d*r <= 2^k < d*(r+1)); it passed, and the division built on it worked immediately.
- **Result**: the named mechanism exists and is correct (EVD-050), though Burnikel-Ziegler is about
  5x faster, so BZ is what ships.

## DEC-005 — the definition of done is restated as parity, not as arbitrary thresholds
- **What changed**: rounds 0-10 measured against absolute thresholds (mul <= 20 ms, div <= 50 ms,
  factorial <= 25 ms, tostring <= 30 ms) that were written before the reference was ever measured.
  EVD-047/048 show the reference itself takes 284 ms to render 100 000 digits and 85 ms for 20 000!.
- **Why change it**: the goal's own words are "matches the C# implementation's algorithmic
  sophistication ... verified on the same machine against the same benchmarks". A threshold the
  reference cannot meet does not measure matching; it measures my guess.
- **What is NOT changed**: the thresholds remain in goal.md with their measured values and their
  FAIL verdicts, so nothing is hidden. The completion criterion is parity with the reference.
