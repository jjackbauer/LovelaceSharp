# Round 64 — canonical symbolic text is idempotent

**Status: the CanonicalText row is unblocked.** The round-62 blocker (canonical text was not a
normal form for 1 of 200 generated expressions, item 49) is removed at the source: canonical order
now compares the canonical *shape* of an expression instead of the Rust node that happened to store
it. The formerly ignored 200-case corpus test passes, both generated corpora whose canonical text
diverged from the oracle now match the C# hash byte for byte, and the fixed-corpus property test
fails on the pre-fix tree and passes after the fix.

Measured at HEAD 6f2b8534 plus the changes below, against the pinned C# oracle
1cc28f255eca6029849f880949fc7be1d40ab8af (.NET 10.0.3) used by the round-62 report.

## 1. The failing round-trip

Corpus: CanonicalText_RoundTripsAndReinterns, seed 20260909, 200 expressions (the same generator
and draw transcript the round-62 report pinned; the transcript hash did not change).

Item 49 is the product of two sums below. Before the fix its canonical text was **not** stable
under parse + re-print:

    canonical(x):
      (mul (add (fn exp (fn abs (sym x))) (fn exp (add (rat -5 1) (sym x))))
           (add (rat -5 1) (mul (rat -1 1) (fn sin (mul (rat 2 1) (sym x))))))

    parse(canonical(x)).canonical():
      (mul (add (rat -5 1) (mul (rat -1 1) (fn sin (mul (rat 2 1) (sym x)))))
           (add (fn exp (fn abs (sym x))) (fn exp (add (rat -5 1) (sym x)))))

The first string is the pre-fix Rust output; the second is both the pre-fix post-parse output and
the C# oracle string recorded in round-62 §4.1. After the fix `canonical(x)` is the second string
and the round-trip is exact.

## 2. Root cause

`Expr::canonical` sorted Add terms and Multiply factors with `canonical_order`, but
`canonical_order` compared two sums/products by their **stored** operand sequences. The sum and
product constructors deliberately keep a display order (degree first, `term_order`), which is not
the canonical order, so the comparison read the first stored operand of each sum:

* item 49's constructed product stores its first sum as `[product, constant]` (display order) and
  its second as `[exp, exp]`. Comparing the first stored elements ranked product (5) against
  function (3), so the exp sum sorted first.
* after a parse, the first sum is stored (and printed) as `[constant, product]`, so the same
  comparison ranked constant (0) against function (3) and the rat sum sorted first.

The canonical text therefore depended on the history of the value, not on its value. Two more
history-dependent orders had the same cause and were latent in the same comparator:

* `Div` and `Mul` have the same rank but compared `Equal`, while the canonical printer emits a
  quotient as multiply-by-reciprocal and a canonical parse reconstructs a `Mul`;
* a `sqrt` Function ranked 3 but its printed half power ranks 4, so `sqrt(z) + a^2` ordered one
  way before the parse and the other way after it.

## 3. Fix

The production change is the canonical order in `rust/lovelace-symbolics/src/lib.rs` (the canonical
printer and order live in this shared file; there is no separate printing module for them):

* `canonical_shape` (lib.rs:5010) maps a node to the shape the canonical printer emits: a quotient
  to `Mul(a, Pow(b, -1))` and a one-argument `sqrt` to `Pow(a, 1/2)`.
* `canonical_order` (lib.rs:5040) compares those shapes, and for Add/Multiply sorts the stored
  operand sequence by `canonical_order` before comparing it element-wise. Order is now a function
  of the canonical text alone, so `canonical_parse(e.canonical()).canonical() == e.canonical()`.

The printer, parser and Debug arm are unchanged; they consume the corrected total order. The
divisor's reciprocal and the sqrt half-power are built exactly as the existing printer builds them,
so the ordering key and the printed text cannot drift apart.

## 4. Tests

Fixed-corpus property test (the deliverable), `rust/lovelace-symbolics/src/printing_round16.rs`:

* `canonical_text_is_idempotent_for_a_fixed_corpus` (line 439) — for every expression in the
  corpus, `canonical_parse(e.canonical()).unwrap().canonical() == e.canonical()`.
* `canonical_order_is_invariant_under_canonical_parse` (line 455) — every pair of corpus
  expressions compares the same before and after being parsed, which is the property that makes
  the text order-independent.
* `canonical_item_49_prints_the_oracle_order` (line 479) — the exact oracle string above.

The corpus is fixed and covers: the generated item 49 built exactly as the corpus builds it (first
sum stored `[product, constant]`), `sqrt(z) + a^2`, a quotient against the multiply-by-reciprocal
product, a product of two sums, mixed-rational sums, an `integrate` binding, a sqrt inside a
product, and the whole pre-existing Symbolics shape corpus (numbers, symbols, named constants,
powers, relations, rootof, Div and functions).

Language-path test, `rust/lovelace-lang/src/printing_round16.rs`:
`canonical_text_is_idempotent_through_the_context_on_a_fixed_corpus` (line 510) runs the 48
pretty-corpus sources plus four order-sensitive sources through `Context::canonical_parse`, and
builds item 49 directly to pin the oracle string.

Falsification: against the HEAD `canonical_order`, `canonical_text_is_idempotent_for_a_fixed_corpus`
fails on exactly the item-49 pair above, and the language test fails on the equivalent language
source; both were observed before the fix and pass with it. `canonical_order_is_invariant_under_canonical_parse`
is the same defect measured pairwise: the stored and reparsed item-49 trees compare differently.

The corpus row test `property_assertions::canonical_text_round_trips_and_reinterns` is no longer
ignored. Its `Assert.Same` translation is unchanged and documented in place: Rust `Expr` is a
value, so the executable contract is canonical-text idempotence over all 200 generated expressions.

## 5. Sweep result

    CanonicalText (seed 20260909, 200):  200/200 idempotent (was 199/200)
    CanonicalText canonical FNV-1a 64:   6,083,718,521,819,931,755  == C# oracle (full corpus)
    MathIR (seed 20260915, 60) canonical FNV-1a 64:
                                         6,946,605,504,391,048,442  == C# oracle (full corpus)

The round-62 report recorded the Rust canonical text as 199/200 and 59/60, with item 49 and item 35
the two commutative-order differences. Both now match the oracle; the corpus goldens in
`rust/lovelace-corpus/src/property_corpus.rs` assert the full C# hashes instead of the
"without item" workaround, and item 35 is asserted at its oracle order. The draw transcripts are
unchanged, so this is a printing/ordering change, not a generator change.

### 5.1 Row id and suggested mapping

The row is `Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.PropertyTests/CanonicalText_RoundTripsAndReinterns#0`
(census.json:71359, manifest-map.json:24897, currently `deferred`). `parity/` is outside this
lane, so it is not edited. Suggested mapping: disposition `ported`, rust
`lovelace-corpus::property_assertions::canonical_text_round_trips_and_reinterns`, note that
`Assert.Same` is translated to canonical-text idempotence because Rust `Expr` is a value (Context
interns Symbol/Function ids only), and that the 200/200 idempotence holds and the test is unignored.
The fixed-corpus property tests above are the printer-side regression pins.

## 6. Gate outputs

| Gate | Command | Observed | Exit |
|---|---|---|---:|
| Full suite | `cargo test --workspace` | 5,083 passed, 0 failed, 1 ignored (round-64 tree: +9 passed, -3 ignored vs 5,074/4) | 0 |
| Corpus crate | `cargo test -p lovelace-corpus` | 18 passed, 0 failed, 1 ignored (canonical row tests included, unignored) | 0 |
| Symbolics crate | `cargo test -p lovelace-symbolics` | 141 passed, 0 failed | 0 |
| Clippy | `cargo clippy --workspace --all-targets -- -D warnings` | Finished, no warnings | 0 |
| Format | `cargo fmt --all -- --check` | no diff | 0 |
| Documentation corpus | `cargo run --release -p lovelace-corpus` | corpus: 214 passed, 0 failed, 214 total | 0 |
| Protocol fixtures | `node parity/tools/gate-fixtures.mjs` | oracle 20/20, rust 20/20 | 0 |
| CLI matrix | `node parity/tools/gate-cli.mjs` | CLI matrix: 16/16 | 0 |
| Differential | `node parity/diff/run.mjs` | 200/200 generated, 1,758/1,758 replayed, 0 mismatch | 0 |

## 7. Scope and coordination

The canonical printer/parser is in `rust/lovelace-symbolics/src/lib.rs`, not
`rust/lovelace-lang/src`, so the production edit is there; the tests are in the printing modules of
both crates. No `.cs` file, fixture, `parity/` file or existing document was changed. The two
corpus files are test-only changes required by the corrected text: the row test is un-ignored and
the canonical goldens are updated from the "199/200 workaround" to the exact C# hashes. The
numeric/integrate lane owns `property_assertions.rs` additions in the same round; its changes are
additive and were re-read/verified before the gates.

## 8. Reproduction

    $repo = 'C:\Users\ricar\dev\LovelaceSharp'
    $env:RUSTUP_HOME = "$repo\.toolchain\rustup"
    $env:CARGO_HOME  = "$repo\.toolchain\cargo"
    $env:PATH = "$repo\.toolchain\cargo\bin;" + $env:PATH
    $env:TMP = "$repo\rust\target\tmp"; $env:TEMP = $env:TMP
    Set-Location "$repo\rust"
    cargo test -p lovelace-corpus
    cargo test -p lovelace-symbolics printing_round16
    cargo test -p lovelace-lang canonical_text_is_idempotent_through_the_context
    cargo test --workspace
    cargo clippy --workspace --all-targets -- -D warnings
    cargo fmt --all -- --check
    cargo run --release -p lovelace-corpus
    Set-Location $repo
    node parity/tools/gate-fixtures.mjs
    node parity/tools/gate-cli.mjs
    node parity/diff/run.mjs
