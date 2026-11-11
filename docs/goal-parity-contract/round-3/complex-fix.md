# Complex fixture — fix report (round 3)

Scope: the Rust workspace only (`rust/`). No .cs, .csproj, fixture or parity file was
modified; this new report is the only file written outside `rust/`.

Environment for every command below:

    $repo='C:\Users\ricar\dev\LovelaceSharp'
    $env:RUSTUP_HOME="$repo\.toolchain\rustup"; $env:CARGO_HOME="$repo\.toolchain\cargo"
    $env:PATH="$repo\.toolchain\cargo\bin;" + $env:PATH
    $env:TMP="$repo\rust\target\tmp"; $env:TEMP=$env:TMP
    cargo build --release --manifest-path "$repo\rust\Cargo.toml" -p lovelace-run
    node "$repo\parity\tools\gate-fixtures.mjs" --filter=complex

## Gate: before

    oracle: 1/1
    rust: 0/1
      FAIL complex
        $.result.display: expected "[6, -1.5 + 0.8660254037844386467637231707525i, -1.5 - 0.8660254037844386467637231707525i]", got "[6, -1.5 + 0.866025404i, -1.5 - 0.866025404i]"
        $.result.structured.elements[0]: keys [exact,im,kind,re] != [exact,kind,value]
        $.result.structured.elements[0].kind: expected "Complex", got "Natural"
    FAIL: rust 0/1

## Gate: after

    oracle: 1/1
    rust: 1/1
    PASS: oracle 1/1, rust 1/1

Whole gate, same build:

    oracle: 20/20
    rust: 20/20
    PASS: oracle 20/20, rust 20/20

The Rust envelope for the fixture now reproduces the golden exactly (after the gate's
volatile-key normalisation):

    display  "[6, -1.5 + 0.8660254037844386467637231707525i, -1.5 - 0.8660254037844386467637231707525i]"
    elements [{kind:"Complex", exact:true,  re:"6",    im:"0"},
              {kind:"Complex", exact:false, re:"-1.5", im:"0.8660254037844386467637231707525"},
              {kind:"Complex", exact:false, re:"-1.5", im:"-0.8660254037844386467637231707525"}]

## What was wrong

Two independent defects, both in the Rust DSP path:

1. `dft` flattened the input to `f64`, computed the transform with binary floating
   point, and then `complex_value` **collapsed** a zero imaginary part to Natural/Integer
   and quantised every other value at nine decimal places. That produced element 0 as
   `Natural 6` and the imaginary parts as `0.866025404`.
2. `Real::is_exact` was a constant `true`: the actual-precision values had no provenance,
   so the structured `exact` field could not report the two computed roots as inexact.

The oracle's `DspPlugin` runs at its 30-place plugin budget (`PluginStandardComputationPrecision`),
builds the gcd-reduced root of unity `e^(-2*pi*i*k/N)` from its special-angle table, and its
`sqrt(3)/2` is `trunc(sqrt(3), 30)/2` — the exact terminating half adds the 31st place:
`0.8660254037844386467637231707525`. Every DFT result stays a `Complex`, including 6+0i.

## The fix

- `rust/lovelace-real/src/lib.rs`: a value now carries an `inexact` provenance bit.
  `is_exact()` reads it; `sqrt` marks a non-square root, `pi` is always inexact, and a
  `from_fraction` that hits its digit cap marks the truncation. Add/sub/mul/div and `pow_int`
  propagate the flag, while equality and rendering ignore it (the reference's own rule).
- `rust/lovelace-lang/src/builtins.rs`: `dft` is now a decimal complex transform over
  `Real`. Roots of unity are gcd-reduced; the 16 angles the reference tabulates as
  rationals/square roots are computed from `sqrt(2)/2`, `sqrt(3)/2`, `1/2`, 0 and ±1, and
  the remaining angles go through argument reduction plus decimal sine/cosine Taylor series
  (ten guard places). Every output element is `Value::Complex`, including a zero imaginary
  part.
- Precision follows the reference plugin rule: `Engine` now records whether the precision
  knob was explicitly set; DSP builtins compute at 30 places by default and follow the knob
  afterwards (`setprecision`). No fixture names or values are special-cased.
- `rust/lovelace-lang/src/lib.rs`: `Complex` display now omits a zero part, matching
  `Complex.ToString` ("6", "4i", "1.5 - 0.5i"). This is also why the existing corpus DSP
  examples keep their documented renderings (`fft([1,0,0,0])` → `[1, 1, 1, 1] (Vector)`).

## Corpus

Before and after the change:

    corpus: 212 passed, 2 failed, 214 total

The two failures are the pre-existing, unrelated `sym/071` (optimize/lower IR text) and
`sym/076` (linsolve display). The corpus sets the engine precision to 1000, so the DSP
examples now exercise the explicit-precision path and still match.

## Tests

    cargo test --workspace
    TOTAL passed: 293 failed: 0
    test exit: 0

No tests were added or removed; the 293-test total is unchanged.
