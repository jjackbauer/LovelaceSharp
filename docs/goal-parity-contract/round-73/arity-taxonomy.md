# Round 73 — arity taxonomy: the five rows close

**Status: complete.** The reference enforces every builtin's DECLARED argument-count range at the
call site and classifies a count outside it as one recoverable argument error
(\`InvalidArgument\`/\`TypeMismatch\`, exit 1 on the wire, the builtin, both counts and the declared
signature in the envelope). The Rust dispatcher enforced that declaration per builtin, so
\`min()\`, \`sum()\`, the four \`limit*\` builtins, \`compile_full(f)\`, \`plot()\`, \`not()\`, the
variadic constructors and 20 other entries answered a different classification — a domain error,
a parse error, a successful call, or a silent truncation. One registry-driven validator now sits
at the call boundary, and the five deferred census rows are backed by five new tests.

Measured 2026-11-12 (America/Sao_Paulo) with the pinned toolchain. Oracle:
\`dotnet exec Lovelace.Run/bin/Release/net10.0/Lovelace.Run.dll --file <script> --omit-functions
--omit-variables\`; Rust: \`rust/target/release/lovelace-run.exe\` with the same argv.

## 1. The reference contract

The declaration is metadata on the registered builtin: \`Parameters\` (the declared parameter
names, hence the upper bound), \`MinArity\` (the lower bound) and \`Variadic\` (removes the upper
bound). A call outside that range is rejected before the body runs:

| Field | Oracle value |
|---|---|
| \`ok\` | \`false\` |
| \`code\` / \`category\` | \`InvalidArgument\` / \`TypeMismatch\` |
| \`recoverable\` | \`true\` |
| exit | \`1\` |
| \`message\` | \`<name>(): expected <range>; got <n>.\` — \`N argument(s)\`, \`min to max arguments\`, or \`at least N argument(s)\` |
| \`signature\` | \`function\`, \`parameters\`, \`expectedMin\`, \`expectedMax\` (absent when variadic), \`variadic\`, \`actual\` |
| \`diagnostics[0].position\` | the call site |

The five census rows are the property, the too-few example, both wrong-shape sweeps and the
signature-agreement probe. Rows 3 and 4 also drive \`min\`/\`max\`/\`sum\`/\`prod\` and
\`compile\`/\`compile_full\` with the wrong SHAPE, which is where their measured divergence lived.

## 2. Before / after per row

Every probe below was run through both runners; "position" is the first diagnostic's position.
\`--\` marks a field the envelope did not publish.

### Row 1 — \`ArityPropertyTests/EveryBuiltin_RefusesEveryCountOutsideItsDeclaredArity_WithTheOneDocumentedShape#0\`

The row drives every registered builtin with every count in \`0..Parameters.Count+2\` and requires
the documented arity refusal outside the declared range and no arity-shaped refusal inside it.
Across the live registry that is 558 Rust probes (408 outside the declared range, 150 inside).

| Probe | Oracle | Rust before | Rust after |
|---|---|---|---|
| \`min()\` | \`InvalidArgument\`/\`TypeMismatch\`, exit 1, \`min(): expected 1 to 2 arguments; got 0.\`, position 0, signature \`{a,axis; 1..2; actual 0}\` | \`InvalidOperation\`/\`DomainError\`, exit 1, \`Reduction needs a value.\`, position 0, no signature | identical to the oracle |
| \`min(1, 2, 3)\` | \`InvalidArgument\`/\`TypeMismatch\`, \`min(): expected 1 to 2 arguments; got 3.\` | exit 0, \`Natural 1\` | identical to the oracle |
| \`not()\` | \`InvalidArgument\`/\`TypeMismatch\`, \`not(): expected 1 argument; got 0.\`, signature actual 0 | \`ParseError\`, \`Unexpected token ')' at position 4: expected a number, string, identifier, '[', or '('.\`, position 4 | identical to the oracle |
| \`not(1, 2)\` | \`InvalidArgument\`/\`TypeMismatch\`, \`not(): expected 1 argument; got 2.\` | \`ParseError\`, \`Expected 'RParen' but found ',' at position 5.\` | identical to the oracle |
| \`zeros()\` | \`InvalidArgument\`/\`TypeMismatch\`, \`zeros(): expected at least 1 argument; got 0.\`, signature \`expectedMin 1\`, \`variadic true\`, no \`expectedMax\`, actual 0 | exit 0, \`Natural 0\` | identical to the oracle |
| \`ones()\` | \`InvalidArgument\`/\`TypeMismatch\`, \`ones(): expected at least 1 argument; got 0.\` | exit 0, \`Natural 1\` | identical to the oracle |
| \`x = symbol("x"); compile_full(x^2 + 1, [x], 3)\` | \`InvalidArgument\`/\`TypeMismatch\`, \`compile_full(): expected 2 arguments; got 3.\`, position 17, actual 3 | exit 0, \`CompilationResult\` | identical to the oracle |
| \`x = symbol("x"); dft([0,1,0,0], 4, 1)\` | \`InvalidArgument\`/\`TypeMismatch\`, \`dft(): expected 1 to 2 arguments; got 3.\` | exit 0 | identical to the oracle |

**Sweep result.** Before: **111 of the 408 outside-range probes** over **32 builtins** did not
produce the documented shape (\`and\`, \`apart\`, \`assume_clear\`, \`assumptions\`, \`compile\`,
\`compile_full\`, \`dft\`, \`fft\`, \`inf\`, \`limit\`, \`limit_full\`, \`limit_left\`, \`limit_right\`,
\`lower\`, \`max\`, \`min\`, \`not\`, \`ones\`, \`optimize\`, \`optimize_full\`, \`or\`, \`plot\`, \`prod\`,
\`reshape\`, \`series\`, \`solve\`, \`solve_full\`, \`solve_system\`, \`solve_system_full\`, \`sum\`,
\`symbol\`, \`zeros\`). After: **0 outside-range violations**, and **0 arity-shaped refusals inside
the declared range**. Every one of the 408 outside-range refusals now matches the oracle on
\`ok\`, \`code\`, \`category\`, \`message\` and (after normalising JSON key order) the whole
\`signature\` object; \`print\` remains the one builtin whose declaration covers every probed count
(\`MinArity 0\` + \`Variadic\`).

### Row 2 — \`ArityValidationTests/TooFewArguments_IsATypedStructuralError_NeverAnInternalFailure#0\`

Probe: \`x = symbol("x"); compile_full(x^2 + 1)\` (the call is the second statement, at UTF-16
offset 17).

| | Oracle | Rust before | Rust after |
|---|---|---|---|
| exit | 1 | 1 | 1 |
| \`ok\` | false | false | false |
| \`code\` / \`category\` | \`InvalidArgument\` / \`TypeMismatch\` | \`InvalidOperation\` / \`DomainError\` | \`InvalidArgument\` / \`TypeMismatch\` |
| \`message\` | \`compile_full(): expected 2 arguments; got 1.\` | \`compile needs an expression and parameters.\` | identical to the oracle |
| position | 17 | 17 | 17 |
| signature | \`{f,params; 2..2; variadic false; actual 1}\` | -- | identical to the oracle |

### Row 3 — \`BuiltinArgumentShapeSweepTests/EveryWrongShapedArgument_IsNeverAnInternalFailure_AndEveryArgumentRefusalNamesTheBuiltinAndThePosition#0\`

The row drives every builtin with its declared parameter count filled from three shapes (a bare
scalar, a four-element vector, a symbolic value). Its unconditional assertions — never an
internal invariant failure, every \`InvalidArgument\` refusal names the builtin and a position
inside the declaration — already held in Rust for the 345-call sweep. The measured divergence was
in the classifications the sweep's own calls produce:

| Probe | Oracle | Rust before | Rust after |
|---|---|---|---|
| \`min(1, 1)\` | \`InvalidArgument\`/\`TypeMismatch\`, \`min(): argument 1 must be an array or vector; got Natural.\`, position 0, signature \`position 1\`, \`parameter a\`, \`expectedShape an array or vector\`, \`arrivedKind Natural\` | exit 0, \`Natural 1\`, no refusal | identical to the oracle |
| \`compile_full(x, x)\` | \`InvalidOperation\`/\`DomainError\`, \`compile_full(): argument 2 must be a list of parameter symbols, e.g. [x]; got Symbolic.\`, position 17 | exit 0, \`CompilationResult\` | identical to the oracle |

After the fix the six builtins this family's probes name (\`min\`, \`max\`, \`sum\`, \`prod\`,
\`compile\`, \`compile_full\`) match the oracle on **all three shapes** (18/18 calls identical on
\`ok\`, \`code\`, \`category\`, \`message\` and signature). The sweep still has other
shape-coercion differences (\`append\`/\`dot\`/\`fft\` scalar coercions and the DomainError message
wording of other builtins); those are not arity classifications, and the row's own assertions do
not constrain them.

### Row 4 — \`BuiltinArgumentShapeSweepTests/EveryWrongShapedArgument_OverAWiderShapeLattice_IsStillNeverAnInternalFailure#0\`

The wider lattice adds a record, a 2x2 matrix, text, a Boolean and the empty vector. Same
unconditional contract; the classifications this family touches:

| Probe | Oracle | Rust before | Rust after |
|---|---|---|---|
| \`min(inspect(1), inspect(1))\` | \`InvalidArgument\`/\`TypeMismatch\`, \`min(): argument 1 must be an array or vector; got Record.\`, position 0, signature \`position 1\`, \`arrivedKind Record\` | \`InvalidArgument\`/\`TypeMismatch\` but \`min(): argument 2 (axis) must be a Natural or Integer that denotes an exact integer; got the Record.\` | identical to the oracle |
| \`compile_full("x", "x")\` | \`InvalidOperation\`/\`DomainError\`, \`Expected a symbolic expression.\`, position 0 | \`InvalidOperation\`/\`DomainError\`, \`compile needs an expression.\`, position 0 | identical to the oracle |

The new test drives the full lattice: 115 positional builtins × 8 shapes = 920 probes, none of
which may cross as an internal invariant failure or carry raw framework text.

### Row 5 — \`CallSignatureDiagnosticsTests/EveryArityRefusalAgreesWithTheRegistryEntry#0\`

The row's five probes already agreed with the oracle before this round — the family was deferred
as a group, not because this row's calls diverged. They are now pinned by a test and produced by
the same registry-driven validator:

| Probe | Oracle = Rust before = Rust after |
|---|---|
| \`matmul([1, 2])\` | \`InvalidArgument\`/\`TypeMismatch\`, \`matmul(): expected 2 arguments; got 1.\`, signature \`{a,b; 2..2; variadic false; actual 1}\` |
| \`cross([1, 2, 3])\` | \`cross(): expected 2 arguments; got 1.\`, signature actual 1 |
| \`append([1, 2])\` | \`append(): expected 2 arguments; got 1.\`, signature actual 1 |
| \`conv([1, 2])\` | \`conv(): expected 2 arguments; got 1.\`, signature actual 1 |
| \`filter([1], [1])\` | \`filter(): expected 3 arguments; got 2.\`, signature actual 2 |

## 3. Production changes

All in \`rust/lovelace-lang/src\`:

1. \`builtins.rs\`, \`builtins::call\`: one registry-driven guard before the dispatch \`match\` (and
   before any builtin body). It reads \`registry::find(name)\` — the same declaration the envelope's
   \`functions\` array and \`--functions\` publish — and refuses
   \`args.len() < min_arity || (!variadic && args.len() > parameter_count)\` with
   \`arity_failure\` (message, \`InvalidArgument\`/\`TypeMismatch\`, signature), positioned at the call.
   This closes the whole family at once instead of per arm: \`min\`/\`sum\`/\`max\`/\`prod\`,
   \`compile\`/\`compile_full\`, \`dft\`/\`fft\`, \`limit*\`, \`solve*\`, \`plot\`, \`series\`, \`lower\`,
   \`inf\`, \`and\`/\`or\`/\`not\`, \`zeros\`/\`ones\`/\`reshape\`, \`symbol\`, \`assume_clear\`, \`assumptions\`,
   \`optimize*\`, \`apart\` and every future registry entry.
2. \`builtins.rs\`, \`reduce\`: position 1 of \`min\`/\`max\`/\`sum\`/\`prod\` must be an array or vector
   even with no axis; a scalar is \`shape_failure(name, 1, "an array or vector", …)\` — the same
   sentence and signature \`mean\`/\`norm\` already publish — instead of a one-element reduction.
3. \`builtins.rs\`, \`compile\`/\`compile_full\`: position 1 must be a \`Symbolic\` expression
   (\`Expected a symbolic expression.\`, \`InvalidOperation\`/\`DomainError\`); position 2 must be a
   vector/array of symbols (\`<name>(): argument 2 must be a list of parameter symbols, e.g. [x];
   got <kind>.\` for the list shape and \`… must contain parameter symbols; got <kind>.\` for an
   element). This reproduces the oracle's classification for the wrong-shape sweep.
4. \`builtins.rs\`, \`and\`/\`or\`/\`not\`: a single Boolean argument to \`not\` negates it (the prefix
   operator's semantics preserved); everything else builds the logical expression node.
5. \`syntax.rs\`, \`Parser::unary\`: \`not\` followed by \`(\` parses as the reference's CALL form, so
   \`not()\` and \`not(a, b)\` reach the arity validator instead of dying in the parser. The bare
   prefix form (\`not x\`) is unchanged.
6. \`lib.rs\`: registers the new \`arity_taxonomy_round73\` test module (five tests, one per row).

## 4. The five census ids and their tests

| Census row | Rust test |
|---|---|
| \`Lovelace.Run.Tests/Lovelace.Run.Tests.ArityPropertyTests/EveryBuiltin_RefusesEveryCountOutsideItsDeclaredArity_WithTheOneDocumentedShape#0\` | \`lovelace-lang::arity_taxonomy_round73::every_builtin_refuses_every_count_outside_its_declared_arity\` |
| \`Lovelace.Run.Tests/Lovelace.Run.Tests.ArityValidationTests/TooFewArguments_IsATypedStructuralError_NeverAnInternalFailure#0\` | \`lovelace-lang::arity_taxonomy_round73::too_few_arguments_is_a_typed_structural_error\` |
| \`Lovelace.Run.Tests/Lovelace.Run.Tests.BuiltinArgumentShapeSweepTests/EveryWrongShapedArgument_IsNeverAnInternalFailure_AndEveryArgumentRefusalNamesTheBuiltinAndThePosition#0\` | \`lovelace-lang::arity_taxonomy_round73::every_wrong_shaped_argument_is_a_typed_refusal_naming_the_position\` |
| \`Lovelace.Run.Tests/Lovelace.Run.Tests.BuiltinArgumentShapeSweepTests/EveryWrongShapedArgument_OverAWiderShapeLattice_IsStillNeverAnInternalFailure#0\` | \`lovelace-lang::arity_taxonomy_round73::every_wrong_shaped_argument_over_the_wider_lattice_is_still_typed\` |
| \`Lovelace.Run.Tests/Lovelace.Run.Tests.CallSignatureDiagnosticsTests/EveryArityRefusalAgreesWithTheRegistryEntry#0\` | \`lovelace-lang::arity_taxonomy_round73::every_arity_refusal_agrees_with_the_registry_entry\` |

\`parity/manifest-map.json\` is outside this round's permitted scope and was not edited; the five
test ids above are the mapping targets.

## 5. Evidence

* **Arity sweep.** Every builtin × every count \`0..Parameters.Count+2\`: 558 probes (408 outside the
  declared range, 150 inside). Oracle and Rust envelopes captured for each; compared on
  \`ok\`, \`code\`, \`category\`, \`message\`, \`recoverable\`, exit and \`signature\` (JSON key order
  normalised). Before: 111 outside-range violations over 32 builtins. After: **0 violation in
  either direction**; the 408 outside-range refusals match the oracle's messages and whole
  signatures, and no inside-range call is refused as a wrong-arity call.
* **Three-shape sweep.** 122 positional builtins × 3 audit shapes = 345 probes, re-run after the
  fix. 0 internal-failure / unrecoverable / grammar / position violations; \`min\`, \`max\`, \`sum\`,
  \`prod\`, \`compile\` and \`compile_full\` match the oracle on all 18 calls.
* **Targeted calls.** 17 key calls (the two family probes plus \`not\`, \`zeros\`, \`ones\`, the
  reductions, \`compile_full\` arity and the row-5 five) were captured before and after; the
  before/after table above is that capture.
* **Wide lattice.** 115 × 8 = 920 in-process probes in the new test; no internal failure and no
  raw framework vocabulary.

All probes are the differential's argv and normalisation: same \`--file\`/\`--omit-functions\`/
\`--omit-variables\`, volatile fields ignored, key presence structural.

## 6. Gates

| Gate | Command | Observed | Exit |
|---|---|---|---:|
| Focused | \`cargo test -p lovelace-lang arity_taxonomy_round73\` | \`5 passed; 0 failed; 0 ignored; 1549 filtered out\` | 0 |
| Workspace | \`cargo test --workspace\` | \`5,165 passed; 0 failed; 1 ignored\` over 21 result blocks (baseline 5,160 + the 5 new tests) | 0 |
| Corpus | \`cargo run --release -p lovelace-corpus\` | \`corpus: 214 passed, 0 failed, 214 total\` | 0 |
| Fixtures | \`node parity/tools/gate-fixtures.mjs\` | \`oracle: 20/20\`, \`rust: 20/20\`, \`PASS: oracle 20/20, rust 20/20\` | 0 |
| CLI | \`node parity/tools/gate-cli.mjs\` | \`CLI matrix: 16/16\` | 0 |
| Differential | \`node parity/diff/run.mjs --cases 200 --seed 1\` | generated **200 match / 0 mismatch / 0 error**; replayed **1758 match / 0 mismatch / 0 error**; \`stillMismatching=0\`; harness exit 0 | 0 |
| Clippy | \`cargo clippy --workspace --all-targets -- -D warnings\` | no diagnostics; \`Finished\` | 0 |
| Format | \`cargo fmt --all -- --check\` | clean | 0 |

The one ignored test is the known
\`lovelace-corpus::property_assertions::mathir_evaluates_like_symbolic\` row, unchanged from the
round-72 baseline.

## 7. Scope

Changed files: \`rust/lovelace-lang/src/builtins.rs\`, \`rust/lovelace-lang/src/syntax.rs\`,
\`rust/lovelace-lang/src/lib.rs\`, \`rust/lovelace-lang/src/arity_taxonomy_round73.rs\` (new) and
this report. No \`.cs\` file, fixture, corpus file, \`parity/\` file or other document was edited.
