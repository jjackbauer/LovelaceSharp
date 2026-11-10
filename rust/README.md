# Lovelace (Rust)

A dependency-free Rust implementation of the LovelaceSharp stack: arbitrary-precision exact
arithmetic, an N-dimensional array library, and the Lovelace scripting language — the engine that
backs the CLI, the script runner and the desktop IDE.

The workspace builds with **no external crates**. That is a deliberate constraint, not an
accident: the build host cannot reach a crate registry, so everything the product needs (JSON,
arbitrary precision, the desktop shell) is written here and auditable in one place.

## Layout

| Crate | Responsibility |
|---|---|
| `lovelace-json` | A small JSON reader/writer with the wire contract's escaping rules. |
| `lovelace-nat` | Arbitrary-precision naturals over base-10⁹ limbs. |
| `lovelace-int` | Signed integers: sign plus `Nat` magnitude, no negative zero. |
| `lovelace-real` | Reals with exact periodic decimals, `sqrt` and `pi`. |
| `lovelace-array` | Generic N-dimensional arrays: shape, indexing, slicing, reductions. |
| `lovelace-symbolics` | Canonical expressions, differentiation, series, solving, partial fractions. |
| `lovelace-lang` | Tokenizer, parser, interpreter, values, builtins, SVG plotting. |
| `lovelace-run` | The non-interactive runner and the agent-facing JSON envelope. |
| `lovelace-corpus` | The parity gate: runs every documented example and diffs the output. |
| `lovelace-desktop` | The native desktop IDE (no HTTP server, no browser runtime). |

## Build and test

    cargo build --workspace --release
    cargo test --workspace
    cargo run -p lovelace-corpus -- --verbose

The corpus gate reads `../parity/corpus/*.json`, which is generated from the C# documentation by
`parity/extract-corpus.mjs`. A missing corpus is a failure, never an empty pass.

## Design notes

- **Exactness first.** No floating point in any value path. Division produces an exact decimal with
  a repeating period (`1/3` is `0.(3)`), never a rounded approximation.
- **Representation is free, rendering is not.** Values are stored as base-10⁹ limbs; the contract is
  the rendered text, which the corpus pins example by example.
- **Positions are UTF-16 code units**, matching the published wire protocol, so a consumer can slice
  the script it submitted.