# Protocol parity — wave 2 report

Scope: the Rust workspace only (`rust/`). No .cs, .csproj, fixture, parity or docs file was
modified. Wave 1's tree was built on (Enum value variant, structured projection, envelope,
builtins helper).

Environment used for every command below:

    $env:RUSTUP_HOME="$repo\.toolchain\rustup"; $env:CARGO_HOME="$repo\.toolchain\cargo"
    $env:PATH="$repo\.toolchain\cargo\bin;" + $env:PATH
    $env:TMP="$repo\rust\target\tmp"; $env:TEMP=$env:TMP

## Fixture gate after each milestone (observed lines)

| # | Milestone | Observed gate line |
|---|---|---|
| 0 | baseline (wave-1 tree, build failed `E0425 UNREPRESENTED_ROOTS_REASON`) | build failed, gate not runnable |
| 1 | compile restored: `UNREPRESENTED_ROOTS_REASON` defined with the reference wording | `rust: 4/20` |
| 2 | Void projects as `Null`; `format_typed` publishes a record's own type name | `rust: 13/20` |
| 3 | solve domain guard (integer/rational refused), denominator NoSolutions, call-site diagnostic position | `rust: 14/20` |
| 4 | `simplify_full` publishes TransformResult with RewriteStep provenance | `rust: 16/20` |
| 5 | `capabilities()` record (19 refusal classes) + exact linear-system solver | `rust: 18/20` |
| 6 | degree >= 4 solver publishes `rootof(...)` (Sturm real-root count) | `rust: 19/20` |
| 7 | complex (DFT) — not solved | `rust: 19/20` |

Command: `powershell -NoProfile -ExecutionPolicy Bypass -File parity/gate-fixtures.ps1 -RustOnly`

## Fixtures still failing

`complex` (`dft([1, 2, 3])`). Expected `[6, -1.5 + 0.866…i, -1.5 - 0.866…i]` as a Vector of
three **Complex** structured elements whose imaginary parts carry 31 fractional digits
(`0.8660254037844386467637231707525`). Rust's DFT is an f64 transform quantised at 1e-9 and
publishes element 0 as a Natural; C# computes gcd-reduced roots of unity with
`Rl.Cos/Rl.Sin` at arbitrary precision and its typed array widens every element to Complex.
Matching needs (a) arbitrary-precision trigonometric functions in `lovelace-real` (it only has
`pi()` today), (b) a complex-aware DFT over that precision, and (c) typed-array element
widening. That is a numeric-kernel rework, not a projection fix, and was not attempted.

## Corpus

- Before: `corpus: 209 passed, 3 failed, 214 total` (regressions found by the orchestrator:
  `type()` answered `Record`, `evalir` rejected a CompilationResult).
- After: `corpus: 212 passed, 2 failed, 214 total` — only the two pre-existing failures
  `sym/071` (optimize/lower IR text) and `sym/076` (linsolve display) remain.
- Fixes: `type()` returns a record's declared type name (the wire projection keeps
  `kind:"Record"` + `type`); `evalir`/`evalir_batch` accept both IR text and a
  `CompilationResult` record (its `ir` field).

## Tests

`cd rust; cargo test --workspace` → **293 tests passed, 0 failed, exit 0**
(50+67+52+7+8+62+20+27 unit tests; all doc-tests pass).

## CLI surface (RUN rows)

`powershell -NoProfile -ExecutionPolicy Bypass -File parity/gate-cli.ps1`
→ `CLI matrix: 14/16` (was 2/16). Green: RUN-001..009, 005T, 011, 013, 014, 014B.
(The gate script in the tree initially threw `ReferenceError: args is not defined`; the
orchestrator fixed it. Until then I verified with a byte-identical copy that only added that
one missing `process.argv` line.)

Implemented for the CLI: the 123-entry builtin registry (name/parameters/arity/build/
plugin/category/summary/returnKind/related/examples) feeding both the success envelope and the
`--functions`/`--function`/`--function-limit` queries; `--print-budget`; `--cancel-after`
(cancellation ledger and overrun diagnostic); `--plot-file` + plot directory creation;
artifacts for the committed plot; `--text` variable listing; exact-Real
numerator/denominator; DivisionByZero coded errors; statement positions on failures; BOM
refusal with `ParseError`; `--precision` removed (the reference refuses it).

### Rows still failing (both blocked by non-protocol facts)

- **RUN-010** `--cancel-after 5` with `integrate(sin(x^2), x)`: the oracle's C# evaluation
  takes ~27 ms and its 5 ms deadline fires (`exceeded:true, stopped:true, ok:false/Cancelled`).
  The Rust run finishes in ~0.4 ms, so no deadline is observed and it reports a normal success.
  The ledger, overrun diagnostic and Cancelled envelope are implemented and are produced when
  the measured run really exceeds the budget; fabricating an overrun for a sub-budget run would
  be wrong. Parity here would require the Rust integrator to be as slow as the C# one — a
  timing-dependent row, not a protocol property.
- **RUN-012** plot placement: two independent blockers. (1) In this environment the Rust
  process is denied writes under `%TEMP%` (`create_dir_all: Access is denied (os error 5)`,
  reproduced standalone while pwsh/Node create the same directories); with `TMP` inside the
  workspace the run succeeds and publishes `artifacts:[{kind:"plot", path:"…\p.svg"}]`.
  (2) The gate compares `plot.svg` byte-for-byte: the reference's `SvgPlotRenderer` emits a
  14,636-byte 800×600 chart with axes, nice ticks, spline interpolation and invariant
  `0.#####` formatting, while Rust's renderer is a 326-byte polyline. A faithful port of
  `Lovelace.Suite/Plotting.cs` is required; that is a renderer port, not a flag fix.

## What could not be done

- `complex` fixture: arbitrary-precision trig + complex DFT + element widening (above).
- RUN-010: timing-dependent cancellation expectation (above).
- RUN-012: C# SVG renderer port (above) and the sandbox write denial under %TEMP%.

## Principal code changes (all under rust/)

- `lovelace-lang/src/lib.rs`: `Enum`/record `typed()`, parse-layer error codes, statement
  positions on failure, BOM refusal, plot file name.
- `lovelace-lang/src/structured.rs`: `Void` → `Null`, exact Real numerator/denominator,
  registry descriptor/listing projections.
- `lovelace-lang/src/registry.rs` (new): the 123 builtin descriptors as data.
- `lovelace-lang/src/builtins.rs`: `UNREPRESENTED_ROOTS_REASON`, solve domain guard,
  denominator NoSolutions, TransformResult + traced simplification (RewriteStep),
  `capabilities()` data table, exact linear-system solver, RootOf degree ≥ 4 branch,
  DivisionByZero coding, plot path normalisation and directory creation.
- `lovelace-symbolics/src/lib.rs`: RootOf printing/canonical/node-count/domain/free-symbols
  rules, Sturm `real_root_count`, degree ≥ 4 RootOf outcome.
- `lovelace-real/src/lib.rs`: exact `to_rational()` (reduced numerator/denominator).
- `lovelace-run/src/main.rs`: registry queries, functions array, artifacts, cancellation,
  text variable listing, new/removed flags.
