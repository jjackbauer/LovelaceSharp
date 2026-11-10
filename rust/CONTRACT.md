# Workspace contract (shared truth for parallel agents)

Build environment (every cargo command needs this prefix, Windows pwsh):

    $env:RUSTUP_HOME="C:\Users\ricar\dev\LovelaceSharp\.toolchain\rustup"
    $env:CARGO_HOME="C:\Users\ricar\dev\LovelaceSharp\.toolchain\cargo"
    $env:PATH="C:\Users\ricar\dev\LovelaceSharp\.toolchain\cargo\bin;$env:PATH"
    Set-Location "C:\Users\ricar\dev\LovelaceSharp\rust"

**No external crates.** The registry is unreachable; [dependencies] may only reference workspace
crates by path. No unsafe, no floating point in value paths.

## Crates

| Crate | Owner | Depends on |
|---|---|---|
| lovelace-json | done | — |
| lovelace-nat, lovelace-int | agent A | — |
| lovelace-array | agent B | — |
| lovelace-real | agent C | nat, int |
| lovelace-lang | agent D | all of the above |

## Nat / Int API (fixed)

    Nat::zero(), Nat::from_u64(u64), Nat::from_decimal(&str)->Option<Nat>, to_decimal()->String,
    is_zero(), is_one(), add, sub(&self,&Nat)->Option<Nat>, mul,
    div_rem(&self,&Nat)->Option<(Nat,Nat)>, pow(&self,u64)->Nat, factorial(u64)->Nat,
    mul_pow10(&self,usize)->Nat, to_u64()->Option<u64>; Clone+Ord+Hash+Default.

    Int::zero(), Int::from_i64(i64), Int::from_decimal(&str)->Option<Int>, to_decimal()->String,
    is_zero(), is_negative(), magnitude()->&Nat, negate(), add, sub, mul,
    div_rem(&self,&Int)->Option<(Int,Int)>, pow(&self,u64)->Int, factorial(u64)->Int;
    Clone+Ord+Hash+Default.

## Array API (fixed, subset the engine needs)

    NdArray<T> { shape: Vec<usize>, data: Vec<T> }  // row-major
    from_vec(shape, data) -> Result<_, ArrayError>, scalar(T), zeros(shape), ones(shape), eye(n),
    shape(), rank(), numel(), get(&[usize]) -> Option<&T>, iter(), as_slice(),
    reshape(Vec<usize>) -> Result<_, ArrayError>, transpose(), squeeze(), flatten(),
    concat(&[Self], axis), append(&self,&Self,axis), map(f), zip_map(&self,&Self,f),
    zip_map_broadcast(&self,&Self,f), sum(), prod(), min(), max(), sum_axis(axis), prod_axis(axis),
    index_partial(&[usize]) -> Result<NdArray<T>,_>, slice(&[Slice]) -> Result<NdArray<T>,_>.
    trait Field { zero, one, add, sub, mul, div, neg, is_zero, from_i64 }.

## Real API (agent C implements; agent D consumes)

    Real::zero(), Real::from_i64(i64), Real::from_int(&Int), Real::from_str(&str)->Option<Real>,
    Real::from_fraction(&Nat,&Nat,max_digits:usize)->Option<Real>,   // None on /0; detects period
    to_string()->String, is_exact()->bool, is_integer()->bool, to_int_exact()->Option<Int>,
    add, sub, mul, neg, div(&self,&Self,max_digits)->Option<Real>, cmp,
    pow_int(i64)->Real, sqrt(digits:usize)->Real, pi(digits:usize)->Real,
    abs(), trunc(), floor(), ceil(), frac(),
    Clone+PartialEq+PartialOrd.

## Rendering rules (from the corpus — these are the contract)

- typed rendering is `<display> (<Kind>)`; Kind ∈ Natural, Integer, Real, Boolean, Text, Vector,
  Array, Function, Void, Complex, Symbolic, Record, Domain, Enum.
- Natural/Integer: plain decimal, `-` prefix only when negative.
- Real: the **stored digits only**, and a trailing `.0` is never printed:
  `3.14`, `0.25`, `2` (for sqrt(4)), `0.(3)` for a period of one digit, `0.1(6)` for a period
  after a pre-period. Period notation is `preperiod(period)` with the period in parentheses.
  Truncated irrationals print their digits and are NOT marked with an ellipsis in the corpus.
- Boolean: `True` / `False`. Text: raw contents, no quotes.
- Vector/Array: `[a, b, c]` with a comma+space separator, nested exactly by rank.
- Record: `TypeName(field: value, field2: value)`, fields in declaration order, empty list `[]`,
  empty string printed as nothing after the colon+space (see lang/102).
- Errors are plain sentences ending with a period, e.g.
  `Cannot divide by zero.`, `Undefined variable 'nope'.`,
  `Index 3 is out of range for vector of length 3.`,
  `Operands could not be broadcast together with shapes [2] and [3].`,
  `if condition must be Boolean, but got 'Natural'.`,
  `'break' is only valid inside a loop.`,
  `Range step must not be zero.`,
  `Ragged nested list literal: every row must have the same shape.`,
  `Cannot widen from Natural to Text: only numeric kinds (Natural, Integer, Real) support widening.`

## Corpus

`parity/corpus/lang.json` holds 111 `{id, doc, heading, script, kind, text}` records extracted
from `Lovelace.Suite/docs/Language.md`. `kind` is value|error|prints|plot. It is the yardstick:
a test that renders a script and compares to `text` is the parity gate.
## Engine API (agent D implements; every host and the corpus gate consume it)

    pub struct Engine;
    impl Engine {
        pub fn new() -> Engine;                       // core builtins only
        pub fn set_precision(&mut self, decimals: usize);
        pub fn precision(&self) -> usize;
        pub fn eval(&mut self, source: &str) -> Result<Value, LangError>;
        pub fn format_typed(&self, v: &Value) -> String;   // "<display> (<Kind>)"
        pub fn format(&self, v: &Value) -> String;         // display only
        pub fn variables(&self) -> Vec<(String, String)>;  // name, typed display (sorted)
        pub fn functions(&self) -> Vec<(String, Vec<String>)>;
        pub fn take_output(&mut self) -> Vec<String>;      // print() lines since last call
        pub fn last_plot(&self) -> Option<PlotCapture>;    // path, title, svg
        pub fn set_plot_dir(&mut self, dir: &str);
        pub fn revision(&self) -> u64;
    }
    pub struct PlotCapture { pub path: String, pub title: String, pub svg: String }
    pub struct LangError { pub message: String, pub position: usize }
    // Value must be pub with a `kind_name(&self) -> &'static str` method.

The corpus gate (written by the orchestrator, `lovelace-corpus`) calls exactly these methods.
