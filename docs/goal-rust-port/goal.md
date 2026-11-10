# Goal — LovelaceSharp → Rust (work directory contract)

## One-sentence outcome

LovelaceSharp ships as a self-contained Rust workspace whose engine reproduces the C#
implementation's observable behaviour on the 214-example documentation corpus and a ported test
suite, with a native desktop application replacing the browser IDE.

## Verbatim request

> do a requirements lift to migrate the whole stack of lovelacesharp to RUST, with parity assurance
> and a desktop app instead of a web app

> /stretch-goal-harness build it all. do the best decisions you can, only get back to me once you
> `ve done it

## Named mechanisms — disposition

| Mechanism (user's noun) | Disposition | Where satisfied |
|---|---|---|
| "whole stack" (every project, not the engine only) | KEPT | crates 1..18 of the port requirements; this run builds the dependency spine and the plugin layer the corpus exercises |
| "RUST" | KEPT | workspace compiles with rustc 1.99.1 |
| "parity assurance" | KEPT | corpus gate over the real Language.md examples + ported unit tests + differential checks against the C# runner where runnable |
| "desktop app instead of a web app" | KEPT | `lovelace-desktop` crate: native OS window, no HTTP listener, no browser runtime |
| "requirements lift" (previous turn) | KEPT | the three requirement documents remain the specification this run is measured against |

No mechanism is REPLACED or REMOVED by scope. Two *agent-chosen* implementation mechanisms from the
requirements documents are replaced for environmental reasons, recorded as DEC-001 and DEC-002:

| Named by | Mechanism | Disposition | Reason |
|---|---|---|---|
| requirements doc (not the user) | external crates (egui, serde, rayon) | REPLACED BY std-only + hand-written equivalents | crates.io is unreachable from cargo (EVD-002); the desktop requirement survives as a native window built on hand-declared OS FFI |
| requirements doc (not the user) | binary-limb representation | REPLACED BY base-10⁹ limbs | representation is explicitly a *free* internal choice (Port §2.2); decimal limbs make period detection exact and are verifiable without a third-party bigint |

## Definition of done (falsifiable)

| ID | Check | Command | Expected observable |
|---|---|---|---|
| D1 | Workspace builds | `cargo build --workspace --release` | exit 0 |
| D2 | Unit tests pass | `cargo test --workspace` | exit 0, every crate reports >=1 test |
| D3 | Corpus parity | `cargo test -p lovelace-corpus -- --nocapture` | 214/214 examples match the documented output |
| D4 | CLI surface | `lovelace-run.exe --eval "1/3"` | prints `0.(3) (Real)` |
| D5 | Wire envelope | `lovelace-run.exe --stdin --json` | single JSON object, protocolVersion 1, exit 0 |
| D6 | Desktop app | `lovelace-desktop.exe --selftest` | exit 0 after exercising document/run/cancel/persist paths; opens no socket |
| D7 | No regression of the C# tree | `git status` on C# sources | unchanged except the three requirement docs and README map |

## Hard constraints (gates)

| ID | Constraint | Provenance |
|---|---|---|
| C1 | Builds and tests offline; no crate downloads | environment (EVD-002) |
| C2 | Numeric value paths contain no floating point | repo thesis + Port G7 |
| C3 | Diagnostics and published positions are UTF-16 code-unit offsets | Parity §2.1, risk R1 |
| C4 | The desktop binary opens no TCP listener and embeds no web runtime | Desktop §1 |
| C5 | Existing C# sources are not modified | freeze protocol PF-1 |

## Preferences (tie-breakers)

- Prefer exactness over speed; prefer a verified subset over an unverified whole.
- Prefer fewer crates with sharp boundaries over many thin ones where a boundary buys nothing.
- Prefer hand-written, auditable code over vendored code we cannot fetch.

## Stop criteria

- **Done**: D1–D7 all pass, reproduced by the orchestrator.
- **Round cap**: 12 rounds. Reaching it is not "done"; it is reported as such with the outstanding
  dimensions named.
- **Blocked**: three consecutive rounds blocked by the same concrete condition.
