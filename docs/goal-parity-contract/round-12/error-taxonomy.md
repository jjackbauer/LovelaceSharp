# Round 12 — error taxonomy and the failure envelope

Scope: `rust/lovelace-lang/src` (classification, engine state) and `rust/lovelace-run/src`
(envelope). No `.cs`, fixture, `parity/` or other doc was changed.

Oracle for every probe below: `dotnet exec Lovelace.Run/bin/Release/net10.0/Lovelace.Run.dll
--eval '<script>' --omit-functions --omit-variables` (the pinned reference). Rust: the release
`lovelace-run`. Volatile durations are omitted from the comparisons; they are not the subject.

## 1. What changed

| Layer | Change |
|---|---|
| `lovelace-lang/src/lib.rs` | `LangError` now carries `signature`, `suggestions`, `hints`, `registry_size`; `LangError::depth(stage, depth, limit, measure)` builds the reference's budget refusal. \
| | Parse-depth guard (statement/expression/power/sign nesting, limit 256), iterative AST-tree guard (512, first over-budget level), iterative value-depth guard (1024), symbolic-depth guard (256, enforced while `abs` builds a node and at the end of evaluation) and the evaluation-depth guard (6 units per user call level, base at the statement's user-code entry; f(85) answers, f(86) is refused, a live wrapper costs one level). |
| | A failing top-level statement now produces its own timing entry (`resultKind: "Void"`, `hasOutput` from the print delta) instead of vanishing with the error. |
| `lovelace-lang/src/syntax.rs` | Tokenizer/parser messages and taxonomy aligned: primary-token refusal text, `Unexpected character ... at position N`, unterminated-string `InvalidInput/ParseError` at the opening quote, and the four nesting scopes counted on the way down. |
| `lovelace-lang/src/registry.rs` | Ported the reference's `NameSuggestions`: synonyms, syntax hints, rank/edit-distance ordering, max 3. |
| `lovelace-lang/src/builtins.rs` | `UnknownFunction/UnsupportedOperation` with suggestions/hints/registrySize and the reference's message grammar; arity refusals are `InvalidArgument/TypeMismatch` with the declared signature and `name(): expected N arguments; got M.`; shape refusals for `det`/`trace` with position/parameter/expectedShape/arrivedKind; `append`/`concat` arity; non-integer exponents are `UnsupportedOperation`; plot write failures are `PlotFileError/TypeMismatch`. |
| `lovelace-lang/src/structured.rs` | Over-budget symbolic renderings now publish `truncated`, `truncationReason: "node-budget"` and `budget`; the fields are absent when the value fits. |
| `lovelace-run/src/main.rs` | Failure envelope publishes `suggestions`/`hints`/`registrySize`/`signature` only for the failure that has them; `recoverable` is false only for `InternalInvariantFailure`; the cancelled envelope carries the committed lines in both `output` and `partialOutput`; pre-engine failures (`FileReadError`, `PlotDirectoryError`) cross as `TypeMismatch` envelopes instead of stderr/exit 2. |
| `lovelace-run/src/error_taxonomy_batch.rs` (new) | 24 tests repeating the C# assertions: taxonomy, suggestions/hints, arity/shape signatures, parser/tokenizer, division/unsupported/domain, failing timings, print output on failure, print-budget metadata, all four depth classes, pre-engine failures and the cancelled blocks. |

## 2. Before/after probes (oracle vs Rust)

`before` is the measured Rust output before this round; `after` is the measured output now. All
`after` rows equal the oracle except where noted.

| Class / probe | before (Rust) | after (Rust = oracle) |
|---|---|---|
| Unknown name `determinant([[1,2],[3,4]])` | `InvalidOperation/DomainError`, "Unknown function 'determinant'.", no `suggestions`/`registrySize` | `UnknownFunction/UnsupportedOperation`, "Unknown function 'determinant'. Did you mean 'det'?", `suggestions:["det"]`, `registrySize:123` |
| Arity `dot(1)` | `InvalidOperation/DomainError`, "'dot' expects 2 argument(s), but 1 were given.", no `signature` | `InvalidArgument/TypeMismatch`, "dot(): expected 2 arguments; got 1.", `signature{function:dot,parameters:[a,b],expectedMin:2,expectedMax:2,variadic:false,actual:1}` |
| Shape `det(1)` | `InvalidOperation/DomainError`, "det needs a square matrix, but got 'Natural'.", no `signature` | `InvalidArgument/TypeMismatch`, "det(): argument 1 must be a square matrix; got Natural.", `signature{position:1,parameter:m,expectedShape:a square matrix,arrivedKind:Natural,...}` |
| Unsupported `2 ^ 0.5` | `InvalidOperation/DomainError`, "Exponent must be an Integer or a Natural." | `UnsupportedOperation/UnsupportedOperation`, "Non-integer exponents are not yet supported." |
| Division `1/0` | `DivisionByZero/DomainError` (control, already aligned) | `DivisionByZero/DomainError`, "Cannot divide by zero." |
| Parse `1+1;\n2+2;\n)` | `ParseError/ParseError`, "Unexpected token Punct(\"))." | `ParseError/ParseError`, "Unexpected token ')' at position 10: expected a number, string, identifier, '[', or '('." + empty timings |
| Tokenizer `"abc` | `ParseError/ParseError`, "Unterminated string literal.", position 1 | `InvalidInput/ParseError`, "Unterminated string literal at position 0.", position 0 |
| Parse depth (3,000 nested parens) | **succeeded** (no guard) | `DepthExceeded/BudgetExceeded`, "expression nesting depth 257 exceeds the maximum supported nesting depth of 256. …" |
| Evaluation depth `f(86)` | **succeeded** (no guard; f(448) also answered) | `DepthExceeded/BudgetExceeded`, "evaluation depth 513 exceeds the maximum supported evaluation depth of 512. …"; `f(85)` still answers |
| Value depth (1,100 `x=[x]` wraps) | `ok:true` | `DepthExceeded/BudgetExceeded`, "value depth 1101 exceeds the maximum supported nesting depth of 1024. …" |
| Symbolic depth (1,000 `abs` wraps) | `ok:true` | `DepthExceeded/BudgetExceeded`, "symbolic expression depth 257 exceeds the maximum supported nesting depth of 256. …" |
| Failing statement timing `x = symbol("x"); solve(x^2+1==0, x, integer)` | `timings: []` (the failing statement was not timed) | `timings: [{position:0,resultKind:"Symbolic",hasOutput:false},{position:17,resultKind:"Void",hasOutput:false}]` (oracle identical) |
| Print before failure `print("before"); print("also before"); det(1); print("UNREACHED")` | output kept, 2 timings | output `["before","also before"]`, 3 timings, last `hasOutput:false` (oracle identical) |
| Print budget `x^2` at `--print-budget 1` | `truncated`/`truncationReason`/`budget` **absent** although the text was abbreviated | `truncated:true`, `truncationReason:"node-budget"`, `budget:1`, `pretty:"x …"`, `canonical:"(pow ( …"` (oracle identical) |
| Missing script file | exit 2, stderr "error: cannot read …", no envelope | exit 1, `FileReadError/TypeMismatch`, empty diagnostics/timings, structural zero duration pair (message text is OS-specific; oracle says the .NET file-not-found sentence) |

The JSON captured for all of these is in `rust/target/tmp/after-probes.json` (working tree,
temporary). Every `after` code/category/message/timing/signature/suggestion/truncation row matched
the oracle exactly except the `FileReadError` OS message.

## 3. Verification

```text
$ cargo test -p lovelace-run error_taxonomy_batch
test result: ok. 24 passed; 0 failed; 0 ignored

$ node parity/tools/gate-fixtures.mjs
oracle: 20/20
rust:   20/20
PASS: oracle 20/20, rust 20/20        (error_envelope green)

$ cargo run --release -p lovelace-corpus
corpus: 212 passed, 2 failed, 214 total   (sym/071, sym/076 — unchanged)

$ cargo test --workspace --no-fail-fast
lovelace-array 58 ok; lovelace-bench 0; lovelace-corpus 0; lovelace-desktop 67 ok;
lovelace-int 125 ok; lovelace-json 7 ok; lovelace-lang 718 ok; lovelace-nat 134 ok;
lovelace-real 2,334 ok; lovelace-run 162 ok (+1 integration test ok); lovelace-symbolics 33 ok;
3,639 tests collected, 0 failed, exit 0.
```

The suite grew from the round's 3,541-test baseline because another implementer added ported
`suite_parity_batch` tests during this round. Intermediate runs observed failures in those
in-flight Symbolics ports (`acceptancetests_diff_productandchain_work`,
`acceptancetests_hessian_computes` and, earlier, nine solve/exactness rows); they were not caused
by, and were not touched by, this round, and the final run above is green.

## 4. Remaining divergences

1. **In-kernel cancellation is still post-hoc.** `sum(1..10000000)` refuses with
   `InvalidOperation`/"Range is too large." before the deadline can fire, and
   `matmul(eye(400),eye(400))` runs to completion (~18 s) and only then publishes the cancelled
   envelope. The ledger fields exist and now describe the run truthfully, but `stopped` means "the
   total elapsed exceeded the budget", not "a token stopped the kernel", and the C# promptness
   tests (`CancellationBudgetTests`, `PrintOutputOnFailureTests.TheCancelledPath`) are not met.
2. **Argument/shape coverage is not total.** `det`/`trace`/`append`/`concat` and every `arity()`
   call now carry `InvalidArgument/TypeMismatch` plus the signature, but hundreds of ad-hoc
   `LangError::plain` shape refusals elsewhere still cross as `InvalidOperation/DomainError`
   without `signature` (the 93-row integer-boundary matrix and the per-builtin shape sweeps).
3. **Other exception classes still share the default.** `EvaluationError/DomainError`,
   `UnsatisfiableAssumptions/DomainError`, `AllocationRefused/BudgetExceeded`,
   `ReentrancyNotSupported/UnsupportedOperation` and `NoSolution` have no Rust counterpart on the
   error path; Rust keeps `InvalidOperation/DomainError`, `UnsupportedOperation` or its own
   message. `filter` and several plugin builtins are still absent, so they answer
   `UnknownFunction`.
4. **`FileReadError` message prose** carries the OS text ("The system cannot find the file
   specified.") instead of .NET's "Could not find file '…'". Code/category/shape match.
5. **Timing durations are zero.** Each `timings[]` entry has the right position/resultKind/hasOutput
   but `elapsed:{value:"0",unit:"ns"}`; the reference measures each statement.
6. **Symbolic depth is enforced at the `abs` construction site plus end-of-evaluation, not at every
   canonical construction funnel**, so a deep symbolic value built by another builtin is refused
   only when the statement/run ends (same code/category, possibly later position).
7. **Envelope key order** is alphabetical in Rust (the JSON value is logically identical); the
   fixture gate and the C# assertions compare structure, not order.
