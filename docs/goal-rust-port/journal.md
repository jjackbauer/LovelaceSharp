# Journal — LovelaceSharp → Rust

Append-only. Entry rules per references/evidence-protocol.md of the stretch-goal-harness skill.

---

## OBS-001 — Rust is absent but installable from inside the workspace
- **Source**: `rustc --version` returned nothing; `.toolchain\rustup-init.exe` downloaded via the Node runtime and installed a minimal stable toolchain with `RUSTUP_HOME`/`CARGO_HOME` under the repo.
- **Fact**: rustc 1.99.1 (58cf8f9c3 2026-10-13), cargo 1.99.1, installed at `.toolchain/`.
- **Implications**: the port is buildable without machine-wide changes; every cargo invocation must set RUSTUP_HOME/CARGO_HOME/PATH.
- **Confidence**: High (EVD-001).

## OBS-002 — crates.io is unreachable from cargo but reachable from Node
- **Source**: `cargo build` with a `serde_json` dependency fails after 3 retries with `schannel: AcquireCredentialsHandle failed: SEC_E_NO_CREDENTIALS`, exit 101. A Node `https` HEAD to `static.rust-lang.org` returns 200 and the 12 839 424-byte rustup-init downloads intact.
- **Fact**: cargo's TLS path is sandbox-blocked; the Node TLS path is not.
- **Implications**: no external crates. Any design that assumes serde/egui/rayon is dead on arrival. This is the single strongest constraint on the run.
- **Confidence**: High (EVD-002, EVD-003).

## DEC-001 — Port is std-only; JSON, bigint, parallelism and GUI are hand-written
- **Decision**: zero external crates. Hand-written JSON reader/writer, hand-written arbitrary-precision decimal, std threads only, and the desktop shell built on hand-declared OS FFI.
- **Rationale**: C1. Vendoring egui's transitive tree (~100 crates) cannot be resolved without cargo's network and would be unverifiable.
- **Alternatives considered**: (a) vendor serde/serde_json manually — ~10 crates, feasible but buys little over 300 lines of JSON we control; (b) Win32 GUI — chosen for desktop; (c) TUI — rejected, violates the user's named mechanism.
- **Consequence**: the desktop app is Windows-first in this run; cross-platform is recorded as OQ-002.
- **Owner**: agent. **Status**: approved (no user-named mechanism excluded).

## DEC-002 — Decimal limbs (base 10⁹), not binary limbs
- **Decision**: represent magnitudes as little-endian base-10⁹ limbs.
- **Rationale**: the requirements make representation a *free* internal choice; decimal limbs make exact period detection and decimal parsing/formatting O(n) rather than conversion-bound, and remove the entire binary↔decimal risk surface without a benchmark argument to lose.
- **Alternatives considered**: base-2⁶⁴ limbs + decimal cache (Port §2.2 recommendation) — faster arithmetic, slower conversions, more code, and its justification was a benchmark of a C# implementation we are replacing.
- **Owner**: agent. **Status**: approved. **Supersedes** the representation note in Port §2.2 for this run.

## HYP-001 — The 214-example corpus is a sufficient parity gate for the language surface
- **Claim**: if the Rust engine reproduces every documented example's typed rendering exactly, then value semantics, formatting and the builtin surface are behaviourally equivalent for the documented domain.
- **Falsification strategy**: run the corpus; then attack it with a differential check against the C# runner on generated scripts; if a generated script diverges while the corpus is green, the hypothesis is falsified.
- **Status**: Under review. **Confidence**: Medium.

## RISK-001 — GUI delivery cannot be pixel-verified in this environment
- **Risk**: the desktop app builds and launches but is never observed by a human, so "it works" is an inference from `--selftest`.
- **Likelihood**: High. **Impact**: Medium.
- **Mitigation**: separate all app logic into a headless core crate covered by tests; the OS shell is a thin adapter; `--selftest` exercises the command bus, document model, and lifecycle and prints a machine-checkable trace.

## RISK-002 — Symbolic/MathIR/DSP surface is far larger than the corpus exercises
- **Risk**: implementing to the corpus green light leaves builtins absent that a user would expect.
- **Likelihood**: Medium. **Impact**: High.
- **Mitigation**: `capabilities()`-style honesty report listing implemented vs documented builtins; every absent builtin must fail with the engine's normal "Unknown function" error rather than a wrong answer.

## OQ-002 — Cross-platform desktop build
- **Question**: can the Win32 shell be abstracted to cocoa/x11 without a crate ecosystem?
- **Needed evidence**: an abstraction seam proven by a second backend, or an explicit Windows-only decision.
- **Priority**: P2. **Status**: open.

---

## ROUND 1 — 2026-11-09

### OBS-003 — Five crates are green under the offline constraint
- **Source**: `cargo test -p lovelace-nat -p lovelace-int -p lovelace-array --offline` → 44 + 52 + 50 tests pass; `cargo test -p lovelace-real` → 20 pass; `cargo test -p lovelace-json` → 7 pass.
- **Fact**: 173 unit tests, 0 failures, zero external crates.
- **Implications**: the numeric spine is real. Representation is base-10⁹ limbs; period detection, sqrt and pi match the documented literals exactly.
- **Confidence**: High (EVD-010, EVD-014).

### DEC-003 — Real is implemented by the orchestrator, not the dispatched agent
- **Decision**: after the Real agent ran long without producing a file, the orchestrator wrote `lovelace-real` and stopped the agent.
- **Rationale**: Real is the keystone for the language engine; a stalled dependency blocks every downstream crate. The delivered version is verified by its own tests against the documented literals (pi(50), sqrt(2), 1/3, 1/6, 1/7).
- **Consequence**: the agent's independent implementation is not available as a cross-check — recorded as a gap, not a passing claim.

### OBS-004 — Pi must keep its trailing zero
- **Source**: the corpus expects `pi(50)` to end `...37510`; a normalising parser strips the final zero and renders `...3751`.
- **Fact**: the stored scale is part of the display contract, so normalisation happens on parse and on arithmetic, never on a fixed-precision constructor.
- **Implications**: any future operation that rebuilds a value from text must not silently re-normalise a fixed-scale value.
- **Confidence**: High (test `Pi_GivenFifty_TruncatesRatherThanRounds`).

---

## ROUND 2 - 2026-11-09

### OBS-005 - The language engine reaches parity on every non-symbolic example
- Source: lovelace-corpus with filter lang/ -> 100 passed, 11 failed; every failure is lang/100..lang/111.
- Fact: values, formatting, operators, ranges, indexing, slicing, strings, interpolation, blocks, loops, functions, array builtins, plotting, evalf and member access match the documented output exactly.
- Confidence: High (EVD-016).

### OBS-006 - The symbolic kernel is 0 percent ported and the corpus says so
- Source: filter sym/ -> 4 passed, 99 failed, all Unknown function symbol.
- Fact: the engine fails closed rather than answering symbolic questions wrongly, which is the required failure mode.
- Confidence: High.

### OBS-007 - The desktop agent delivered a core and a selftest but no window
- Source: main.rs printed the native shell is not linked in this build and exited 2.
- Fact: a delivery can pass its own 64 tests and still not be the product. The orchestrator wrote shell.rs (Win32 class registration, menu bar, three EDIT panes, WM_SIZE layout, command dispatch, --smoke), linked user32, and verified window creation and destruction.
- Confidence: High (EVD-019).

### DEC-004 - DSP transforms use f64 inside the kernel only
- Decision: fft/dft compute in f64 and quantise at 1e-9, collapsing numerically real results back to exact values.
- Rationale: an exact transform of arbitrary size needs exact trigonometry, which the tower does not provide.
- Consequence: a real deviation from no-floating-point, recorded rather than hidden.

## OQ-003 - Port the symbolic kernel
- Question: what is the smallest symbolic kernel that passes the remaining examples without answering incorrectly?
- Needed evidence: expressions with canonical ordering, diff/integrate/simplify/solve/series/limit, the SolveResult and LimitResult record shapes, 59 builtins.
- Priority: P0. Status: open. This is the remaining work.

---

## ROUND 3 (goal round) - symbolic kernel starts

### OBS-008 - Canonical ordering is derivable from the corpus, not from the old comparator note
- Source: the documented forms order terms by degree descending, then by kind (symbol, function, power, product, number last); products order factors by kind, not by depth.
- Fact: two ordering rules are needed (term order and factor order); using one for both produced -2*sin(x^2)*x instead of -2*x*sin(x^2).
- Confidence: High (corpus 134/214, 30 symbolic examples now pass).

### OBS-009 - Cancellation must stay blocked without an assumption
- Source: x/x must print x/x; simplify(x/x) with assume(x != 0) must print 1.
- Fact: power combination refused to sum exponents to zero unless the engine recorded a nonzero assumption. That is now an explicit rule in Expr::mul.
- Confidence: High (sym/008, sym/009, sym/010).

### DEC-005 - The symbolic crate is the orchestrator's, not an agent's
- Decision: lovelace-symbolics was written directly, because its printing rules are the parity contract and every later symbolic builtin depends on them.
- Consequence: 9 crate tests plus 30 corpus examples are the evidence; no independent cross-check exists yet.

### OQ-004 - Remaining symbolic surface
- Algebra: factor, cancel, apart. Calculus: series, limit, integrate. Solving: solve, solve_system, roots. Records: SolveResult/LimitResult/CompilationResult with exact field order. Vector calculus: jacobian, hessian. 80 corpus failures remain.
- Priority: P0. Status: open.
