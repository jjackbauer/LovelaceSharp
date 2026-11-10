# LovelaceSharp -> Rust: final report

Every number here was produced by a command recorded in `evidence.md`.

## The outcome in one table

| Dimension | Result |
|---|---|
| Workspace | 10 crates, ~21 000 lines of Rust, zero external dependencies |
| Build | `cargo build --workspace --release` exit 0 (2m11s clean) |
| Tests | 275 passing, 0 failing |
| Documentation corpus | **212 / 214** (99.1 %) |
| Language document | **111 / 111 (100 %)** |
| Symbolics document | 101 / 103 |
| Wire protocol | envelope, exit codes 0/1/2, stdout purity |
| Desktop | native window verified by title and clean exit; selftest 15/15 |
| C# tree | unmodified |

## What is complete

**The language** — every documented example reproduces the reference byte for byte: values and
widening, repeating decimals, ranges, indexing, slicing, broadcasting, strings and interpolation,
blocks, loops, recursion, closures, the array library, plotting, `print`, and the exact error text.

**The numeric tower** — exact arbitrary precision on base-10^9 limbs, repeating-period detection,
integer Newton `sqrt`, Machin `pi` with the published digits, no floating point in any value path.

**The symbolic kernel** — canonical expressions with an explicit non-collapsing division node,
differentiation, expansion, collection, substitution, polynomial solving (rational, complex
algebraic, nested radical), parametric solution families, limits and series through a truncated
Laurent engine, partial fractions, integration, equation systems by case splitting, assumption
intervals with contradiction detection, Horner optimisation, and a MathIR emitter/parser that
round-trips.

**The interfaces** — `lovelace-run` speaks the documented JSON envelope with exit codes 0/1/2 and
stdout purity; `lovelace-desktop` is a native Win32 IDE with editor, workspace, logs and plot panes,
file open/save, precision control, and no network surface.

## The two remaining examples

Both are documented here rather than fitted:

**`sym/071` — MathIR listing.** The emitter was re-ported from `Lovelace.MathIR/Ir.cs`: a power emits
its base, then its exponent as a constant node, then the power; sums and products emit their
children in order; repeated subexpressions reuse their index. The output is valid MathIR and
`evalir` parses it back (`sym/072` passes through that round trip). Byte-identity still differs in
two places: the reference emits a *second* constant node for a repeated literal because literal
identity in its expression model includes provenance (`RealLiteral` compares digits, exponent **and
exactness**), and its inner Horner sum stores its operands constant-first. Reproducing the first
would mean modelling literal provenance in the Rust expression tree; reproducing the second would
mean storing an order the printer then has to undo.

**`sym/076` — `linsolve` output form.** The reference prints `-x/x/(x*y)`; this implementation
prints the mathematically equal `-1/(x*y)`. The reference's form is an artifact of its pivot
selection. Matching it means imitating an algorithm's incidental intermediate, not its result.

## Why the number moved the way it did

Each gain came from a capability, not a string:

| Examples | Capability added |
|---|---|
| 066 | an explicit `Div` node that is never collapsed |
| 059, 060 | parametric solution families for periodic equations |
| 089 | biquadratic solving through y = x^2, with a real unrepresented-root count |
| 070 | Horner form for `optimize` |
| 072 | `lower` -> IR text -> `evalir` round trip with a real parser |
| 100 | `linsolve` with a determinant non-vanishing condition |
| 062 | equation systems by product case-splitting |

And the gate kept catching things a plausible port would have shipped: `exp(x)` read as a
degree-one polynomial (answering `x = 5`), imaginary roots returned in the real domain, a cancelled
quotient returning the root that nulls its own denominator, and a global ordering rule that
matched two examples and broke five.

## Running it

    cd rust
    cargo build --workspace --release
    cargo test --workspace
    .\target\release\lovelace-run.exe --eval "1/3" --text
    .\target\release\lovelace-corpus.exe
    .\target\release\lovelace-desktop.exe --selftest
    .\target\release\lovelace-desktop.exe        (opens the window)

The toolchain is local to the repository (`.toolchain/`); no machine-wide installation is required
and no crate is downloaded.
