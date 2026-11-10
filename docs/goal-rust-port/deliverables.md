# Deliverables

| Artifact | Path | Evidence | Confidence |
|---|---|---|---|
| Requirements lift (3 documents) | .github/requirements/Lovelace.Rust.{Port,Parity,Desktop}.md | measured in turn 1 | High |
| Harness memory | docs/goal-rust-port/{goal,journal,evidence,state,scorecard,deliverables}.md | this run | High |
| Corpus extractor | parity/extract-corpus.mjs | 214 examples extracted | High |
| Corpus corpus | parity/corpus/{lang,symbolics}.json | 111 + 103 examples | High |
| Rust workspace | rust/ (8 crates) | cargo build --workspace --release exit 0 | High |
| Numeric tower | rust/lovelace-{nat,int,real}/ | 116 tests | High |
| Array library | rust/lovelace-array/ | 50 tests | High |
| JSON | rust/lovelace-json/ | 7 tests | High |
| Language engine | rust/lovelace-lang/ | 100/111 documented examples | High |
| Wire runner | rust/lovelace-run/ | envelope + exit codes verified | High |
| Parity gate | rust/lovelace-corpus/ | 104/214, fails on mismatch and on missing corpus | High |
| Desktop app | rust/lovelace-desktop/ | native window smoke + 14 selftest checks + 64 unit tests | High |
| Scorecard | docs/goal-rust-port/scorecard.md | measured | High |

## Not delivered

| Item | Why | Consequence |
|---|---|---|
| Symbolic kernel (59 builtins, 137 types) | 13 980 lines of C#; a half-built version would answer symbolic questions wrongly | 110 of 214 corpus examples fail, all in the symbolic domain; the engine fails closed with Unknown function |
| REPL binary (lovelace-cli) | the desktop app and the runner cover both interactive and scripted use | no line-editing REPL; `lovelace-run --text` evaluates one script |
| Lean proof-to-Rust theorem instances | needs the port finished | proofs still constrain the model, unexercised by tests |
| Differential harness against the C# oracle | not reached in the round budget | parity is proven by the corpus, not by generated differential cases |
