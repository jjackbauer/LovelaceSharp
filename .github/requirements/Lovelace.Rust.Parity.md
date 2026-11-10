# Requirements: Lovelace.Rust.Parity — Behavioural Parity Assurance

> Scope: specify how the Rust port of LovelaceSharp proves it behaves like the C# implementation it
> replaces. This is the **gate document**: it defines the observable surface under parity, the
> oracles that already exist in the repository, the machine that turns them into one scorecard, the
> thresholds that scorecard must meet, and how the gate itself is falsified so that a green run
> means something. Companion to [Lovelace.Rust.Port.md](Lovelace.Rust.Port.md) and
> [Lovelace.Rust.Desktop.md](Lovelace.Rust.Desktop.md).

---

## 1. The parity problem, stated precisely

A port can fail in four ways, and each needs a different instrument:

| Failure mode | Example | Instrument |
|---|---|---|
| **Value drift** | `1/3` renders as `0.333…` instead of `0.(3)` | Fixture corpus + differential oracle |
| **Surface drift** | a diagnostic reports column 12 instead of 11 | Fixture corpus with positions |
| **Protocol drift** | a JSON key is present where it must be absent | Golden envelopes, structural comparison |
| **Silent omission** | a feature is simply not ported and nobody notices | Test census + manifest gate |

Human review catches none of these reliably. The repository already contains 2 087 test facts, 214
machine-verified documentation examples and 20 golden protocol envelopes; the parity programme's job
is to bind them to the Rust workspace so that **the C# tree is executable specification**, not
reference material that someone might read.

---

## 2. The observable surface

Parity is defined only at boundaries a user or a consuming process can observe. Everything below
the boundary is engineering freedom; this is what makes a Rust port worth doing rather than a
transliteration.

| Level | Name | Definition | Comparison rule |
|---|---|---|---|
| **P0** | Value semantics | Kind, exactness, digits, structure of every value the engine can produce | Exact structural equality |
| **P1** | Human rendering | `display` and `typed` strings, `print` output, Unicode/ASCII modes, period notation | Byte equality |
| **P2** | Diagnostics | `message` text, `code`, `category`, `position`/`line`/`column`, `suggestions` | Byte equality of message, exact equality of offsets |
| **P3** | Wire protocol | The `Lovelace.Run` JSON envelope, field presence/omission, exit codes, stdout purity | Structural equality with a documented volatile-field list |
| **P4** | Artifacts | Plot SVG, knowledge graph JSON, structured projections | Byte equality where deterministic, otherwise documented tolerance |
| **P5** | Performance | Throughput and latency on the recorded benchmark suite | Ratio against the recorded C# baseline |

**Explicitly outside parity:** internal representation (BCD vs binary limbs), allocation strategy,
thread usage, module layout, .NET interface implementations, and the Studio's HTTP surface (deleted
by design — see Desktop §1). These are documented as *free* in Port §3 so that no one ports
`DigitStore` byte-for-byte out of a mistaken sense of duty.

### 2.1 What "byte equality" means for strings

Three encoding traps sit under P1–P3 and each has a decided answer:

| Trap | C# behaviour | Rust default | Requirement |
|---|---|---|---|
| Non-ASCII in JSON | `System.Text.Json` escapes to `\uXXXX` | `serde_json` emits raw UTF-8 | **Match C#**: configure the serializer to escape non-ASCII, or normalise both sides before comparison. The published `\u00B5`-style escaping is part of the wire contract. |
| String offsets | UTF-16 code units | UTF-8 bytes / `char` (scalar values) | **Match C#**: every published `position` stays a UTF-16 code-unit offset; conversion happens at one boundary in the runner. |
| Line endings | `\r\n` on Windows | `\n` always | **Match the invariant, not the platform**: the envelope ends in exactly one newline; `position` accounts for CRLF as two units; `line`/`column` treat CRLF/CR/LF as one terminator. |

---

## 3. Oracle inventory (what already exists)

Parity does not start from zero. The repository already ships six independent oracles, and the port
inherits them rather than inventing replacements.

| # | Oracle | Where | Volume | What it pins |
|---|---|---|---|---|
| O1 | **Doctest corpus** | `Lovelace.Suite/docs/Language.md` (111 examples) and `Lovelace.Symbolics/README.md` (103 examples), executed by `LanguageDocumentationTests` | 214 examples | Language surface: values, operators, arrays, errors, prints, plots |
| O2 | **Test census** | 15 `*.Tests` projects | 2 087 `[Fact]/[Theory]` | Module behaviour at method granularity |
| O3 | **Golden envelopes** | `Lovelace.Run.Tests/fixtures` | 20 `.ls` + 20 `.json` | Wire protocol, structured projections, error shape |
| O4 | **Recorded benchmarks** | `benchmark-results.md`, `precbench-report.md`, `dspbench-report.md`, `benchmarks/symbench-*.md` | 7 harnesses | Performance envelope |
| O5 | **Lean proofs** | `Lovelace.Proofs` (core Lean 4.33.1) | Representation, addition, subtraction, multiplication, division | The digit-arithmetic model itself |
| O6 | **MGIR behavioural graph** | `Lovelace.Knowledge` + `knowledge-graph-demo.json` | Seeded sampling of the real runner | End-to-end behavioural fingerprint, hundreds of observed input/output planes |

O6 deserves emphasis: the knowledge tool samples the **real runner binary** as a black box, over a
seeded Monte Carlo domain, canonicalises every observation, and clusters outcomes into behaviour
planes. Running the same seed against the Rust runner and diffing the graph is the single strongest
whole-system signal available, and it costs nothing to add because the harness already exists.

**Additional oracles the C# suite already uses and the port must keep:** a SymPy differential oracle
in `Lovelace.Symbolics.Tests`, and accuracy gates in `precbench.Tests` comparing `Real` at 8 and 16
digits against `float`/`double`.

---

## 4. Freeze protocol

The C# tree is the specification, so it must stop moving.

| ID | Rule |
|---|---|
| PF-1 | At the start of Phase 0 the C# tree is tagged `oracle-v1`; all parity comparisons name that tag. |
| PF-2 | After the tag, `main` accepts only two kinds of C# change: a bug fix that is also mirrored into the corresponding Rust crate, or a port-blocking refactor that changes no observable output. |
| PF-3 | Every post-freeze C# change carries a `parity/amendments/NNN.md` record: what changed, which oracle moved, and the corpus diff it produced. |
| PF-4 | Any change that alters a P0–P3 observation **invalidates the oracle and must be re-baselined deliberately**, with the amendment recording both before and after values. |
| PF-5 | The Rust port may not "fix" a C# bug silently. Fixes are a two-step: mirror into C#, re-baseline the corpus, apply in Rust. Otherwise the port diverges and every future comparison is noise. |

---

## 5. Opus 1 — the language-neutral fixture corpus

The doctests are currently expressed as Markdown, consumed by one C# test class. The port needs them
as data.

**Requirement:** a generator, `xtask corpus extract`, reads the Markdown sources and emits
`parity/corpus/lang.json`:

| Field | Type | Meaning |
|---|---|---|
| `id` | string | Stable `lang/NNN` / `sym/NNN` / `desc/NNN`, assigned by document order |
| `source_doc` | string | Path and heading of the originating example |
| `script` | string | The `lovelace` fence body, verbatim |
| `expect` | object | `{ kind: "value" ❘ "error" ❘ "prints" ❘ "plot", text: string }` |
| `plugins` | string[] | Plugin set the example needs (`dsp`, `symbolics`, `mathir`) |
| `precision` | integer or null | Only where the example depends on a non-default knob |

The committed census side of the same ledger is a flat list of records shaped like:

    { "project": "Lovelace.Natural.Tests", "class": "NaturalTests",
      "method": "Add_GivenCarry_Propagates", "kind": "fact",
      "traits": ["Category=Heavy"] }

**Requirements on the corpus:**

- [ ] CF-1 Both implementations run the **same file**. The C# side gains a test that reads `lang.json` instead of parsing Markdown; the Rust side reads it directly. One file, two consumers, no drift. [prerequisite for every corpus test]
- [ ] CF-2 The corpus is generated, never hand-edited; a CI check re-runs extraction and fails if the committed file differs. [mandatory — prevents silent hand-tuning]
- [ ] CF-3 Extraction fails loudly if a `lovelace` fence lacks a `result` fence (the C# parser already enforces this; the generator must keep enforcing it). [depends on CF-1]
- [ ] CF-4 Each example runs in a **fresh engine** with the plugins it declares, matching `LanguageDocumentationTests` semantics exactly — including the plot-directory isolation. [depends on CF-1]
- [ ] CF-5 `error` expectations compare the exception **message**, not just the fact of failure. [mandatory — message drift is invisible otherwise]
- [ ] CF-6 `plot` expectations compare the title and assert a non-empty SVG; `prints` compare the captured writer output after trailing-newline normalisation, exactly as the C# test does. [depends on CF-4]

---

## 6. Opus 2 — test census and the parity manifest

2 087 tests cannot be trusted to "get ported" without a ledger.

**Requirement:** `xtask parity census` scans every C# test project and emits `parity/census.json`
with one record per `[Fact]`/`[Theory]` method (shape shown in §5). The census is committed.
`parity/manifest.toml` maps each entry to its disposition:

| `status` | Meaning | Required field |
|---|---|---|
| `ported` | A Rust test exists and runs | `rust` = exact module path of the test |
| `dropped` | Intentionally not ported | `reason` from a **closed vocabulary** |
| `new` | Rust-only addition with no C# counterpart | `why` |

Closed vocabulary for `dropped`: `http-surface-removed`, `net-interface-only`,
`superseded-by-X`, `oracle-internal`. Free text is rejected, because a free-text reason is how
omissions hide.

**Gate rules:**

- [ ] PM-1 Every census entry appears exactly once in the manifest. Unknown, duplicated or missing entries fail the build. [mandatory — the ledger is the guarantee]
- [ ] PM-2 Every `ported` entry resolves to a Rust test that exists, is not `#[ignore]`d, and **ran** in this CI invocation (zero tests matched is a failure, not a pass). [depends on PM-1]
- [ ] PM-3 The `dropped` count is ratcheted: it may fall but never rise without an explicit manifest-version bump reviewed in the PR. [mandatory — omission cannot creep]
- [ ] PM-4 Coverage percentages are reported per crate against the census, and the scorecard fails below 100 % of `ported + dropped` against the total. [depends on PM-1..2]
- [ ] PM-5 `[Theory]` data rows are counted individually where the C# test enumerates `MemberData`, so 111 doctest examples count as 111 obligations, not one. [depends on PM-1]

---

## 7. Opus 3 — the differential harness

The corpus and manifest prove the port matches on **known** inputs. The differential harness proves
it on generated ones.

| Component | Role |
|---|---|
| `parity/oracle-cs` | A C# console that reads JSONL requests `{id, script, precision, plugins}` and writes one JSONL response per line: kind, display, typed, structured, output, diagnostics, error code/category. Built from the frozen `oracle-v1` tree. |
| `parity/oracle-rs` | The Rust binary with the identical request/response contract, sharing the projection code with the real runner. |
| `parity/diff` | A comparator that applies the per-field comparison rules of §2 and reports a minimal diff with a reproducible seed. |

**Corpus generation:** a `proptest` strategy tree over the language grammar (literals with adversarial
digit lengths, period-forming divisions, nested arrays of mixed rank, string interpolation,
user functions, errors at every precedence level), plus a mutation pass over the existing corpus.

**Requirements:**

- [ ] DH-1 The oracle protocol is versioned and the two sides are built from the **same** protocol crate/assembly, so the harness cannot compare against a stale shape. [prerequisite for all differential runs]
- [ ] DH-2 The comparator ignores exactly the documented volatile fields (`revision`, `elapsed`, `timings`) and nothing else; any widening of that list requires a comment explaining why the field is not observable. [mandatory — a loose comparator is a silent pass]
- [ ] DH-3 Every mismatch is minimised (delta-debugged) to the smallest failing script before being reported, so the report is actionable. [depends on DH-1]
- [ ] DH-4 Nightly runs at ≥100 000 generated cases; PR runs a fixed 5 000-case regression slice for time. [depends on DH-1..2]
- [ ] DH-5 Precision is varied by the generator (defaults, 20, 1 000, 2 500) so precision scoping is exercised, not assumed. [depends on DH-3]
- [ ] DH-6 Cancellation and budget paths are generated too: every `--cancel-after`, `--print-budget` and allocation-budget refusal is a comparable observation. [depends on DH-1]

---

## 8. Opus 4 — property invariants

Properties catch the class of bug a finite corpus cannot: arithmetic that is wrong only for operands
nobody generated. Each law is asserted on the Rust side and, where it is cheap, on the oracle side
too.

| ID | Law | Applies to |
|---|---|---|
| PI-1 | `a + b = b + a`, `(a+b)+c = a+(b+c)` | Natural, Integer, Real |
| PI-2 | `divrem(a,b)` reconstructs: `q*b + r = a` and `0 ≤ r < ❘b❘` | Integer |
| PI-3 | `parse(format(x)) = x` for every representable form, including periodic | All numerics |
| PI-4 | A periodic rendering is **canonical**: the shortest period, no pre-period longer than the denominator requires, no trailing zeros | Real |
| PI-5 | `sqrt(x)·sqrt(x) ≈ x` within the requested digits, and the error never exceeds one unit in the last place of the working precision | Real |
| PI-6 | `det(A·B) = det(A)·det(B)` and `A·A⁻¹ = I` for nonsingular exact matrices | Array |
| PI-7 | Shape algebra: `reshape` preserves element count, `transpose` is an involution, `squeeze` then `expand` is identity on shape | Array |
| PI-8 | Interning is order-independent: the same expression built in any order has the same canonical form | Symbolics |
| PI-9 | Rewrite rules terminate and preserve value under the assumption set they are licensed by | Symbolics |
| PI-10 | `lower(expr)` then `evalir` agrees with direct numeric evaluation within precision policy | MathIR |
| PI-11 | Every value round-trips through the structured projection without losing its kind | Suite |
| PI-12 | The engine is deterministic: the same script with the same precision produces byte-identical output across runs and across thread counts | Suite |

---

## 9. Opus 5 — artifact parity

| Artifact | Determinism today | Requirement |
|---|---|---|
| Plot SVG | The renderer documents deterministic output | Compare **structure-identical and geometry within 1e-9**; byte equality is a bonus, not a gate. The renderer uses `double` for coordinates, so transcendental library differences are expected — hiding that behind a byte-gate would be dishonest. |
| Structured projection | Pure function of value + display precision | Byte-identical JSON after the non-ASCII escaping rule of §2.1 |
| Run envelope | Deterministic apart from `revision`/`elapsed`/`timings` | Structural equality; volatile fields replaced by a sentinel, as the existing fixtures already do |
| Knowledge graph | Seeded (SplitMix64, seed 20240617) over the real binary | Byte-identical when the runner matches; this is the end-to-end acceptance signal (O6) |
| Help/registry catalogue | Derived from `BuiltinDescriptor` metadata | Byte-identical; a missing `summary` or `related` entry is a P3 failure |

---

## 10. Opus 6 — proofs and their Rust counterpart

The Lean proofs are statements about **digit lists and `Nat`**, not about C#. They constrain the Rust
implementation exactly as much as the C# one — but only if the Rust code is *modelled* by them.

| ID | Requirement |
|---|---|
| PR-1 | The Lean project is carried over unchanged and continues to build with the pinned toolchain (Lean 4.33.1, core-only). [mandatory — the proofs are the strongest claim the project makes] |
| PR-2 | Each named theorem in `Lovelace.Proofs` has a Rust property test that instantiates it at the implementation boundary: `ofDigits_digits` → parse/format round-trip; `addDigits_correct` → add-with-carry reconstruction; `subDigits_correct` → subtract-with-borrow reconstruction; `mulDigits_correct` → convolution equality; `divDigits_correct` → quotient·divisor + remainder. [depends on PR-1] |
| PR-3 | The representation switch (BCD → binary limbs, see Port §3) is justified against the theorems: they are stated for general base `b`, so the Rust implementation must document which `b` it instantiates and prove the mapping in a comment plus the PR-2 tests. [depends on PR-2] |
| PR-4 | Exhaustive small-case enumeration (all operands below a threshold in base 10 and base 2⁶⁴) runs in CI and must pass for every theorem instance. [depends on PR-2] |

---

## 11. The parity scorecard

One number, computed by `xtask parity score`, published on every CI run and in the PR.

| Dimension | Metric | Gate |
|---|---|---|
| Corpus | `lang.json` pass rate | **100 %** |
| Census | `ported + dropped` ÷ total entries | **100 %** |
| Census | `ported` entries whose Rust test ran | **100 %** |
| Census | `dropped` count trend | **non-increasing** |
| Differential | mismatches in the nightly slice | **0** |
| Properties | failing laws | **0** |
| Artifacts | envelope / projection mismatches | **0** |
| Artifacts | SVG geometry failures beyond 1e-9 | **0** |
| Protocol | golden envelopes passing | **20/20** |
| End-to-end | MGIR graph identical under the recorded seed | **identical** |
| Proofs | PR-2/PR-4 instances passing | **100 %** |
| Performance | benchmark ratio vs recorded baseline | **≥ 0.80×** (Rust may be 20 % slower; never slower than that without a written exception) |

The scorecard is **fail-closed**: a dimension that fails to compute — missing corpus, zero tests
discovered, oracle binary not built — is reported as **red**, never skipped. Vacuous green is the
failure mode this section exists to prevent.

---

## 12. Falsifying the gate

A gate nobody has seen fail is a decoration. Before the gate is trusted, it is attacked.

| ID | Attack | Required outcome |
|---|---|---|
| FG-1 | Seed 12 mutations into a scratch Rust build (period detection off by one; `+` widening where it must not; a non-ASCII escape dropped; `artifacts` omitted on failure; a statement position off by one; term order using hashes; `sqrt` last-digit rounding; a diagnostic message reworded; an envelope key renamed; `print` output duplicated; a builtin `summary` dropped; `pi` precision cap ignored). | **All 12 are caught and attributed to a dimension.** |
| FG-2 | Run the scorecard against an empty crate with no tests. | Reported **red**, not green. |
| FG-3 | Delete one Rust test that a manifest entry claims. | `PM-2` fails. |
| FG-4 | Mark a test `dropped` with free-text reasoning. | Schema validation fails. |
| FG-5 | Add a tenth volatile field to the comparator's ignore list without justification. | Review-blocking check (a required comment) fails. |

The mutation results are recorded in `parity/gate-falsification.md` and re-run whenever the
comparator changes, because a comparator edit is exactly how a gate silently loses teeth.

---

## 13. Risks

| ID | Risk | Likelihood | Impact | Detection | Mitigation |
|---|---|---|---|---|---|
| R1 | UTF-16 `position` reimplemented as UTF-8 offsets, breaking agent tooling that slices caller text. | High | High | Corpus entries containing emoji before the error | One conversion boundary; differential cases with astral characters in every generated script (DH-5) |
| R2 | Precision scoping (C# `AsyncLocal`) leaks between concurrent evaluations in async Rust. | High | High | Concurrency stress test; PI-12 across thread counts | Task-local scope plus an explicit precision parameter on every entry point; test both |
| R3 | `System.Text.Json` escaping differs from `serde_json`. | High | Medium | Byte-compare of envelopes containing `µ`, `π`, `≤` | Custom serializer escaping non-ASCII, pinned by fixture |
| R4 | Dictionary ordering assumed stable. C# `Dictionary` preserves insertion order; `HashMap` does not. | High | Medium | `variables[]` order in fixtures | `IndexMap` everywhere order is published |
| R5 | `decimal` paths (`Real(decimal)`, `G29` formatting) have no Rust equivalent. | Medium | Medium | Corpus cases constructed from decimals | Implement the conversion over exact rationals; never route through `f64` |
| R6 | `char.IsWhiteSpace` / `IsAsciiDigit` Unicode edge sets differ from Rust's `char` predicates. | Medium | Medium | Tokenizer corpus over U+00A0, U+2028, U+0085 | Pin an explicit predicate table, not library defaults |
| R7 | Parallel reductions produce different rounding/associativity. | Low | High | PI-12, MGIR graph diff | All reductions must be exact or order-independent; forbid order-dependent parallel accumulation |
| R8 | Plot geometry differs due to platform math libraries. | High | Low | §9 geometry gate | Gate on tolerance, not bytes; document the difference |
| R9 | A "ported" test exists but asserts something weaker than the original. | Medium | High | Mutation attack FG-1 | Porting review: each ported test keeps its original name and its original assertions; weakening is a PR-blocking finding |
| R10 | The C# oracle cannot be built in CI forever (.NET 10 on every agent). | Medium | Medium | Oracle job failures | Oracle built once, published as an artifact, pinned by digest; rebuild job on a fixed runner image |
| R11 | Rust error handling (`Result`) drifts message text from C# exceptions. | High | Medium | P2 comparisons | Messages are data, not formatting: a single message catalogue tested against the corpus |
| R12 | Symbolics term ordering ported with any dependence on hashing. | Medium | High | PI-8, canonical-form fixtures | Port `TermOrder` as a total order with a documented comparator and no hash use |
| R13 | `noise` argument-position overloading (3-arg vs 4-arg) is ambiguous in the source. | Medium | Low | Dsp corpus | Resolve the ambiguity in Phase 0 as a recorded decision before porting the kernel |
| R14 | The migration succeeds technically but the C# tree rots before the port is done. | Medium | High | Freeze protocol | Freeze tag plus amendment records (PF-1..5) |

---

## 14. Completeness checklist

- [ ] `parity/corpus/lang.json` generated, committed, and consumed by both implementations (CF-1..6). [prerequisite for the corpus dimension]
- [ ] `parity/census.json` + `parity/manifest.toml` cover all 2 087 facts (PM-1..5). [prerequisite for the census dimension]
- [ ] Differential harness builds both oracles from the `oracle-v1` tag and runs nightly at ≥100 000 cases (DH-1..6). [prerequisite for the differential dimension]
- [ ] All twelve property laws have tests, including the determinism law (PI-1..12). [prerequisite for the properties dimension]
- [ ] Twenty golden envelopes pass structurally under the frozen volatile-field list. [prerequisite for the protocol dimension]
- [ ] MGIR graph reproduced byte-identically under seed 20240617. [prerequisite for end-to-end]
- [ ] Lean build green and every named theorem has a Rust instance (PR-1..4). [prerequisite for the proofs dimension]
- [ ] All twelve seeded mutations caught; empty-crate run red; manifest tampering rejected (FG-1..5). [mandatory — the gate is only trusted after it has failed correctly]
- [ ] Scorecard published per PR and fail-closed on any dimension error (§11). [mandatory]

---

## Test Plan

### Corpus extraction

1. `Extract_GivenLanguageMarkdown_ProducesOneEntryPerLovelaceFence`
   *Assumption*: the number of `lovelace` fences is the number of examples, since the C# parser already rejects an unpaired fence.
2. `Extract_GivenCommittedCorpus_RegeneratesByteIdenticalFile`
   *Assumption*: generation must be deterministic or the CI drift check is unusable.
3. `Extract_GivenFenceWithoutResult_FailsWithFilePathAndLine`
   *Assumption*: a silent skip would silently shrink the corpus, which is the vacuous-green failure.

### Census and manifest

4. `Census_GivenAllTestProjects_FindsEveryFactAndTheoryMethod`
   *Assumption*: 2 087 is the count from the current tree and any drift is a real change, not a scanner bug.
5. `Manifest_GivenEntryMissingFromManifest_FailsBuild`
   *Assumption*: new C# tests can appear after the census is taken, so the ledger must be checked both ways.
6. `Manifest_GivenPortedEntryWhoseRustTestDoesNotExist_FailsBuild`
   *Assumption*: a path typo is the most likely way a manifest lies.
7. `Manifest_GivenDroppedEntryWithFreeTextReason_RejectsSchema`
   *Assumption*: the closed vocabulary is what makes a drop reviewable.
8. `Manifest_GivenDroppedCountIncreased_RequiresVersionBump`
   *Assumption*: omission creep is gradual and only a ratchet stops it.

### Differential harness

9. `Diff_GivenUTF16PositionAgainstUTF8Position_ReportsMismatch`
   *Assumption*: the risk materialises as an offset difference, not a crash.
10. `Diff_GivenVolatileFieldDiffers_ReportsNoMismatch`
    *Assumption*: revision and elapsed time are genuinely unobservable, so ignoring them is correct rather than convenient.
11. `Diff_GivenMismatch_MinimisesScriptToSmallestFailingInput`
    *Assumption*: a 40-statement failing script is not diagnosable without reduction.
12. `Diff_GivenGeneratedCancellation_ComparesStructuredCancellationFields`
    *Assumption*: cancellation output is part of the contract because agents branch on `code`.

### Properties

13. `Properties_GivenRandomDivRem_ReconstructsDividendWithBoundedRemainder`
    *Assumption*: reconstruction is the theorem `divDigits_correct` at the API boundary.
14. `Properties_GivenRandomReal_PeriodRenderingIsCanonical`
    *Assumption*: canonicality is what makes `1/3` display stable across implementations.
15. `Properties_GivenSameScriptOnDifferentThreadCounts_ProducesIdenticalBytes`
    *Assumption*: determinism is a product guarantee, not an implementation accident.

### Gate falsification

16. `Gate_GivenSeededMutation_AttributesFailureToExpectedDimension`
    *Assumption*: attribution matters as much as detection, or a green/red bit teaches nothing.
17. `Gate_GivenEmptyWorkspace_ReportsRed`
    *Assumption*: the most likely real-world false pass is a misconfigured workspace, not a bug.
18. `Gate_GivenComparatorIgnoresNewFieldWithoutJustification_BlocksReview`
    *Assumption*: the ignore list is the thinnest part of the gate's armour.

---

*All assumptions verified against the implemented oracles — the doctest runner, the census counts, the twenty golden fixtures and the Lean theorem list were read from source; Zero Falsified rows.*
