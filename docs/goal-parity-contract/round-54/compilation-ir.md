# Round 54 — the Compilation.Compile / IrProgram surface and its eight census rows

**Status: additive production surface and eight C# mirrors delivered; every gate is green.**
The eight census ids are still <code>deferred</code> in <code>parity/</code> because this round's
scope excludes parity; mapping them to the test ids in section 4 is the remaining mechanical step.

Measured 2026-11-11 (America/Sao_Paulo) on commit <code>813755c</code> (round 53), with the
working tree changes listed in section 3. The sibling round-54 document
<code>value-widen-porting.md</code> in the same directory covers the independent Value-widen surface.

## 1. The eight ids, read from the C# source

The round-50 resweep groups eight manifest rows under "Compilation.Compile / IrProgram
multi-parameter compilation API". All eight are in the same class:

<code>Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.CompilationTests/</code>

Sources: <code>Lovelace.Symbolics.Tests/CompilationTests.cs</code> (case bodies),
<code>Lovelace.MathIR/Ir.cs</code> (IrOpKind, IrNode, IrProgram, Lowering),
<code>Lovelace.MathIR/Compilation.cs</code> (CompilationTarget, CompilationResult,
CompiledKernel, Compilation), <code>Lovelace.MathIR/Typing.cs</code> (IrType, IrTyping) and
<code>Lovelace.MathIR/Evaluator.cs</code> (IrEvaluator, NumOps).

| Census id (class CompilationTests) | C# line | Members observed | Concrete expected result |
|---|---:|---|---|
| <code>Kernel_NamedAndPositional_Evaluate#0</code> | 32 | <code>Compilation.CompileKernel(e, [x, y], ctx)</code>; <code>kernel.Evaluate(2, 3)</code>; <code>kernel.Evaluate(dict x=2,y=3)</code>; wrong arity | positional 7; named 7; one positional argument throws <code>ArgumentException</code> |
| <code>Kernel_BatchEvaluation_Columnwise#0</code> | 47 | <code>CompileKernel(x^2+1)</code>; <code>kernel.EvaluateBatch({"x":[1,2,3]})</code>; mismatched column lengths | 3 lanes, 2 / 5 / 10; lengths 2 and 1 throw <code>ArgumentException</code> (column y has 1 lane, expected 2) |
| <code>Kernel_ExactTiers_Preserved#0</code> | 69 | kernel over <code>2*x^3 + 1/2</code>; <code>Evaluate(1/2)</code> | <code>NumRat</code> exactly 3/4 (the exact tier, not a Real) |
| <code>Kernel_Transcendental_ApproximateTier#0</code> | 82 | kernel over <code>exp(x)</code>; <code>Real.WithPrecision(40, 20)</code>; <code>Evaluate(0)</code> | <code>NumReal</code> (<code>IsExact</code> false) comparing equal to 1 |
| <code>Kernel_BatchLazySelect_MixedGuards#0</code> | 95 | piecewise(<code>x &gt; 0: 1/x</code>, otherwise <code>-x</code>); batch x = [2, -3, 1/2] | 1/2, 3, 2 (mixed guards: both branches demanded) |
| <code>Kernel_BatchMatchesScalarEvaluation#0</code> | 112 | kernel over <code>sin(x*y) + x^2</code>; batch vs scalar at three rational pairs, precision 40 | every lane equals its scalar evaluation |
| <code>Kernel_LazySelect_UntakenBranchNeverEvaluates#0</code> | 134 | piecewise(<code>x == 0: 1</code>, otherwise <code>1/x</code>); <code>Evaluate(0)</code> and <code>Evaluate(2)</code> | 1 at x=0 (1/x is never evaluated), 1/2 at x=2 |
| <code>CompilationResult_IrText_RoundTrips#0</code> | 149 | <code>Compilation.Compile(x^5 + 3x, ctx, x)</code>; <code>compilation.IrText</code>; <code>IrProgram.Deserialize(text)</code>; <code>Program.Serialize()</code> | deserialize(text).Serialize() == compilation.Program.Serialize() |

The eighth row is the only one that observes <code>Compilation.Compile</code> (the other seven call
<code>CompileKernel</code>); every one of them observes <code>IrProgram</code> through
<code>Serialize</code> or through the compiled kernel's <code>IrText</code>.

### The member contract, from the C# source

* **IrOpKind** (Ir.cs:7-15): 35 instruction kinds — Constant, Parameter, Add, Sub, Mul, Div,
  Negate, Reciprocal, PowInt, Pow, Sqrt, Exp, Log, Sin, Cos, Tan, Asin, Acos, Atan, Sinh, Cosh,
  Tanh, Abs, Sign, Floor, Ceil, Min, Max, Eq, Ne, Lt, Le, Gt, Ge, Select.
* **IrNode** (Ir.cs:17): <code>record IrNode(IrOpKind Op, int[] Operands, int Aux)</code>.
  Constant/Parameter operands index the constant pool / parameter list; every other operand
  indexes the node list. Aux is 1 for a named constant (Pi/E/I) and 0 otherwise.
* **IrProgram** (Ir.cs:25-90): <code>FormatVersion = 2</code>; <code>Constants</code>,
  <code>Parameters</code>, <code>Nodes</code>; <code>Serialize()</code> writes
  <code>#!mathir 2</code>, then <code>param</code> lines, then <code>const</code> lines, then one
  <code>Op aux operands</code> line per node; <code>Deserialize</code> parses and then calls
  <code>IrTyping.Validate</code>.
* **Lowering** (Ir.cs:93-319): parameters by name; structural CSE; constants canonicalized and
  pooled (aux 0) with named constants pooled as Pi/E/I (aux 1); <code>x^-1</code> to Reciprocal;
  <code>x^(1/2)</code> to Sqrt; an integer exponent to PowInt with a FRESH exponent constant node;
  Add/Mul left-folded into binary nodes (an empty n-ary node is the literal 0); piecewise lowers its
  otherwise branch first, then wraps branches last-to-first in Select; guards map relations to
  Eq/Ne/Lt/Le/Gt/Ge and and/or/not to Select compositions; unknown functions, order terms,
  relations and non-boolean guards throw <code>InvalidOperationException</code>.
* **IrTyping** (Typing.cs:11-137): forward inference (Constant classified from its canonical text,
  Parameter null/unconstrained, ordered arithmetic promotes, transcendental functions promote to
  Real/Complex, Abs Real, Sign Integer-or-Complex, Floor/Ceil Integer-or-null, comparisons Bool,
  Select promotes its two values); <code>Validate</code> runs structural checks first (arity,
  operand range, pool index, parameter index) and then type rules (ordered comparisons/ Min/Max
  reject Complex; Select requires a Bool guard).
* **Compilation / kernels** (Compilation.cs:9-107): <code>CompilationResult(Program, Parameters)</code>
  with <code>IrText = Program.Serialize()</code>; <code>Compile</code> requires at least one
  parameter, lowers, validates; <code>CompileKernel</code> lowers without validating;
  <code>ToKernel</code> wraps a result; <code>CompiledKernel</code> exposes Parameters, IrText,
  positional <code>Evaluate(Num[])</code> (arity-checked), named
  <code>Evaluate(IReadOnlyDictionary)</code>, and column-wise
  <code>EvaluateBatch(IReadOnlyDictionary&lt;string, Num[]&gt;)</code> (equal lengths, every
  parameter column present). Select is lazy in both scalar and batch evaluation; the batch path
  evaluates only the branches its lane set can take.

## 2. Member map (C# to Rust)

All additions live at <code>lovelace_lang::mathir</code>. Nothing existing was renamed or changed;
<code>builtins::ir_text</code>, <code>parse_ir_text</code>, the <code>compile</code>/
<code>compile_full</code>/<code>evalir</code> builtins and every runner envelope are untouched.

| C# member | Rust addition | Observable preserved |
|---|---|---|
| <code>IrOpKind</code> (35 kinds) | <code>mathir::IrOpKind</code>, same names; <code>name()</code>, <code>arity()</code> | typed instruction kind; arity table |
| <code>IrNode(Op, Operands, Aux)</code> | <code>mathir::IrNode</code> with <code>new</code>, <code>op()</code>, <code>operands()</code>, <code>aux()</code>, <code>arity()</code> | raw typed operand access; Constant/Parameter index the pool/list |
| <code>IrProgram.FormatVersion</code> | <code>mathir::FORMAT_VERSION</code> and <code>IrProgram::FORMAT_VERSION</code> | 2 |
| <code>IrProgram.Constants/Parameters/Nodes</code> | <code>constants()</code>, <code>parameters()</code>, <code>nodes()</code>, <code>node(i)</code>, <code>constant(i)</code>, <code>parameter(i)</code>, <code>root()</code> | program metadata; result node index |
| <code>IrProgram.Serialize()</code> | <code>serialize()</code> | byte-identical line format |
| <code>IrProgram.Deserialize(text)</code> | <code>deserialize(text) -&gt; Result&lt;IrProgram, LangError&gt;</code> | version gate, malformed-line refusal, validation |
| <code>Lowering.Lower(e, ctx, params)</code> | <code>IrProgram::lower(expr, params)</code> | same lowering rules and refusals |
| <code>IrType</code> (5) | <code>mathir::IrType</code>; <code>name()</code> | Integer/Rational/Real/Complex/Bool |
| <code>IrTyping.Infer</code> | <code>IrTyping::infer</code> | forward types; None = unconstrained |
| <code>IrTyping.Validate</code> | <code>IrTyping::validate</code> | same first-refusal sentences |
| <code>CompilationResult</code> | <code>CompilationResult</code> with <code>program()</code>, <code>parameters()</code>, <code>ir_text()</code>, <code>result_type()</code> | program + parameter contract; IrText; inferred result type |
| <code>Compilation.Compile</code> | <code>compile(expr, params)</code> | at-least-one-parameter refusal; lower + validate |
| <code>Compilation.CompileKernel</code> | <code>compile_kernel(expr, params)</code> | lower without validating |
| <code>Compilation.ToKernel</code> | <code>to_kernel(&amp;CompilationResult)</code> | same program wrapped as a kernel |
| <code>CompiledKernel.Parameters/IrText</code> | <code>parameters()</code>, <code>ir_text()</code>, <code>program()</code> | |
| <code>CompiledKernel.Evaluate(Num[])</code> | <code>evaluate(&amp;[Num])</code> | arity refusal; ambient precision |
| <code>CompiledKernel.Evaluate(dict)</code> | <code>evaluate_named(&amp;HashMap&lt;String, Num&gt;)</code> | name-keyed bindings |
| <code>CompiledKernel.EvaluateBatch(dict)</code> | <code>evaluate_batch(&amp;HashMap&lt;String, Vec&lt;Num&gt;&gt;)</code> | lane checks, missing-column refusal |
| <code>IrEvaluator.Evaluate/EvaluateBatch</code> | <code>IrEvaluator::evaluate/evaluate_at/evaluate_batch/evaluate_batch_at</code> | demand-driven DAG traversal; lazy Select; vectorised batch |
| <code>Num</code>/<code>NumOps</code> tiers | reuses <code>crate::numeric::Num</code> (Exact/Real/Complex) | exact tiers stay exact; transcendental results are NumReal |

Deliberate representation differences, none of which any of the eight rows observes:

1. A C# <code>Symbol</code> is a name bound to an <code>ExprContext</code>; Rust keys parameters
   and bindings by name (<code>String</code>), because every observed use is a name.
2. C# raises <code>InvalidOperationException</code>/<code>ArgumentException</code>; Rust returns
   <code>LangError</code>. Lowering/validation failures use the crate default
   <code>InvalidOperation</code>/<code>DomainError</code> (the same taxonomy the runner maps a CLR
   invalid operation to); argument refusals use <code>InvalidArgument</code>/<code>TypeMismatch</code>.
3. Precision is the same ambient scope: the C# rows open <code>Real.WithPrecision(40, 20)</code>;
   the mirrors open <code>lovelace_real::Real::with_precision(40, 20)</code>, and the kernel reads
   <code>Real::max_computation_decimal_places()</code> at evaluation time.
4. C# canonical integer constants print as <code>(int n)</code>; the Rust pool keeps the
   publisher's canonical <code>(rat n 1)</code>. <code>classify_constant</code> accepts both, so
   either text validates, and both texts denote Integer.
5. C# has no Div node (Divide is Multiply by a power); Rust's expression type does, so the typed
   lowering emits Div for an explicit quotient. A raw <code>Fun("sqrt")</code> lowers to Sqrt
   (C# converts it to a Power in its constructor).

## 3. Production change

* **New:** <code>rust/lovelace-lang/src/mathir.rs</code> — IrOpKind, IrNode, IrProgram (serialize,
  deserialize, lower, typed accessors), IrType, IrTyping, CompilationResult, compile,
  compile_kernel, to_kernel, CompiledKernel, IrEvaluator, and the lowering/typing/evaluation
  internals with the reference's sentences.
* **Edited:** <code>rust/lovelace-lang/src/lib.rs</code> — one line, <code>pub mod mathir;</code>,
  plus the one-line test-module registration.
* **New tests:** <code>rust/lovelace-lang/src/mathir_round54.rs</code> — the eight mirrors and five
  diagnostic/typing tests (section 4).
* **Untouched:** <code>builtins.rs</code> (including <code>ir_text</code>, <code>emit_ir</code>,
  <code>parse_ir_text</code>), <code>lovelace-run</code>, every <code>.cs</code> file, fixture,
  corpus file, <code>parity/</code> entry and other document.

Why <code>lovelace-lang</code> rather than <code>lovelace-run</code>: the lowering already lives in
<code>lovelace_lang::builtins</code> (<code>ir_text</code>/<code>emit_ir</code>), and the symbolic
expression (<code>lovelace_symbolics::Expr</code>), the numeric tier (<code>numeric::Num</code>) and
the Real precision scope are all reachable from this crate. The C# <code>Lovelace.MathIR</code> is a
library that depends only on <code>Lovelace.Symbolics</code> and is exercised from
<code>Lovelace.Symbolics.Tests</code>; <code>lovelace-run</code> is the CLI/envelope crate, and its
published output must not move. Putting the typed IR in the runner would have made a library surface
test-only and inverted the dependency direction.

## 4. The eight mirrors

All eight are in <code>lovelace_lang::mathir_round54</code> and drive the public surface directly
(they do not go through the language's <code>evalir</code> builtin). Each uses the C# case's own
program and expected values; the instruction listing is the serialized program, so it also pins
the typed node sequence (op, operands, aux) and the constant pool.

| Census id | Rust test | C# assertion mirrored |
|---|---|---|
| <code>Kernel_NamedAndPositional_Evaluate#0</code> | <code>mathir_round54::kernel_named_and_positional_evaluate</code> | positional 7, named 7, one-argument refusal |
| <code>Kernel_BatchEvaluation_Columnwise#0</code> | <code>mathir_round54::kernel_batch_evaluation_columnwise</code> | lanes 2/5/10; mismatched columns refused with the column named |
| <code>Kernel_ExactTiers_Preserved#0</code> | <code>mathir_round54::kernel_exact_tiers_preserved</code> | exact tier, 3/4 |
| <code>Kernel_Transcendental_ApproximateTier#0</code> | <code>mathir_round54::kernel_transcendental_approximate_tier</code> | NumReal, inexact, equal to 1 at precision 40 |
| <code>Kernel_BatchLazySelect_MixedGuards#0</code> | <code>mathir_round54::kernel_batch_lazy_select_mixed_guards</code> | 1/2, 3, 2 for x = [2, -3, 1/2] |
| <code>Kernel_BatchMatchesScalarEvaluation#0</code> | <code>mathir_round54::kernel_batch_matches_scalar_evaluation</code> | every lane equals scalar at three rational pairs |
| <code>Kernel_LazySelect_UntakenBranchNeverEvaluates#0</code> | <code>mathir_round54::kernel_lazy_select_untaken_branch_never_evaluates</code> | 1 at x=0 without dividing, 1/2 at x=2 |
| <code>CompilationResult_IrText_RoundTrips#0</code> | <code>mathir_round54::compilation_result_ir_text_round_trips</code> | Deserialize(IrText).Serialize() == Program.Serialize(); typed nodes and pool pinned |

Programs and expected instruction sequences (also asserted as typed IrNode sequences in the tests):

<code>Compilation.Compile(x^5 + 3x, x)</code>:

    #!mathir 2
    param x
    const (rat 5 1)
    const (rat 3 1)
    Parameter 0 0
    Constant 0 0
    PowInt 0 0,1
    Constant 0 1
    Mul 0 3,0
    Add 0 2,4

<code>CompileKernel(x^2 + 1, x)</code> (column-wise batch):

    #!mathir 2
    param x
    const (rat 2 1)
    const (rat 1 1)
    Parameter 0 0
    Constant 0 0
    PowInt 0 0,1
    Constant 0 1
    Add 0 2,3

<code>CompileKernel(piecewise(x &gt; 0: 1/x, otherwise -x), x)</code> (mixed guards):

    #!mathir 2
    param x
    const (rat -1 1)
    const (rat 0 1)
    Constant 0 0
    Parameter 0 0
    Mul 0 0,1
    Constant 0 1
    Gt 0 1,3
    Reciprocal 0 1
    Select 0 4,5,2

<code>CompileKernel(sin(x*y) + x^2, [x, y])</code> (batch == scalar):

    #!mathir 2
    param x
    param y
    const (rat 2 1)
    Parameter 0 0
    Constant 0 0
    PowInt 0 0,1
    Parameter 0 1
    Mul 0 0,3
    Sin 0 4
    Add 0 2,5

<code>CompileKernel(2*x^3 + 1/2, x)</code> (exact tiers):

    #!mathir 2
    param x
    const (rat 2 1)
    const (rat 3 1)
    const (rat 1 2)
    Constant 0 0
    Parameter 0 0
    Constant 0 1
    PowInt 0 1,2
    Mul 0 0,3
    Constant 0 2
    Add 0 4,5

<code>CompileKernel(piecewise(x == 0: 1, otherwise 1/x), x)</code> (lazy Select):

    #!mathir 2
    param x
    const (rat 0 1)
    const (rat 1 1)
    Parameter 0 0
    Reciprocal 0 0
    Constant 0 0
    Eq 0 0,2
    Constant 0 1
    Select 0 3,4,1

<code>CompileKernel(x*y + 1, [x, y])</code> (named and positional):

    #!mathir 2
    param x
    param y
    const (rat 1 1)
    Parameter 0 0
    Parameter 0 1
    Mul 0 0,1
    Constant 0 0
    Add 0 2,3

<code>CompileKernel(exp(x), x)</code> (approximate tier):

    #!mathir 2
    param x
    Parameter 0 0
    Exp 0 0

Five further tests pin the diagnostics and metadata the same members own:
<code>deserialize_gates_the_format_version</code>,
<code>validation_reports_pool_parameter_and_guard_diagnostics</code>,
<code>lowering_refuses_what_the_reference_refuses</code>,
<code>compile_requires_at_least_one_parameter</code> and
<code>typing_infers_the_documented_tiers</code>. They assert the exact sentences:

* Unsupported version: <code>Unsupported MathIR format version '1' (expected 2).</code>
* Wrong arity: <code>IR validation: node 0 (Add) has 1 operands; expected 2.</code>
* Wrong pool index (reached only after the operand-range pass, as in C#):
  <code>IR validation: Constant node 6 references out-of-range pool index 5.</code>
* Wrong parameter index:
  <code>IR validation: Parameter node 0 references out-of-range parameter index 0.</code>
* Non-Bool Select guard:
  <code>IR validation: Select node 1 requires a Bool guard, got unconstrained.</code>
* Ordered comparison of Complex:
  <code>IR validation: ordered comparison Lt requires real operands, node 2 operand 1 is Complex.</code>
* Lowering refusals: <code>Function 'frobnicate' cannot be lowered to MathIR.</code>,
  <code>Node kind Symbol cannot be lowered to MathIR.</code>,
  <code>Node kind Relation cannot be lowered to MathIR.</code>
* Empty parameter list:
  <code>Compilation requires at least one parameter. (Parameter 'parameters')</code>.
* Kernel arity: <code>Kernel expects 2 argument(s), got 1.</code>
* Column mismatch: <code>Column 'y' has 1 lanes; expected 2.</code>

## 5. Gate outputs

Baseline was 4,980 workspace tests (round 53); the thirteen new tests in
<code>mathir_round54</code> make 4,993. Cargo commands ran from <code>rust/</code>, the Node gates
from the repository root, with the repository toolchain environment (<code>RUSTUP_HOME</code>,
<code>CARGO_HOME</code>, <code>PATH</code>, <code>TMP</code>).

| Gate | Command | Observed | Exit |
|---|---|---|---:|
| Workspace suite | <code>cargo test --workspace</code> | <code>passed=4993 failed=0</code> across 19 test binaries | 0 |
| New mirrors | <code>cargo test -p lovelace-lang mathir_round54</code> | <code>13 passed; 0 failed; 0 ignored; 1484 filtered out</code> | 0 |
| Corpus | <code>cargo run --release -p lovelace-corpus</code> | <code>corpus: 214 passed, 0 failed, 214 total</code> | 0 |
| Protocol fixtures | <code>node parity/tools/gate-fixtures.mjs</code> | <code>oracle: 20/20</code>, <code>rust: 20/20</code>, <code>PASS</code> | 0 |
| CLI matrix | <code>node parity/tools/gate-cli.mjs</code> | <code>CLI matrix: 16/16</code>, <code>PASS: every RUN-001..RUN-014 row behaves like the oracle</code> | 0 |
| Clippy | <code>cargo clippy --workspace --all-targets -- -D warnings</code> | clean, no warnings | 0 |
| Format | <code>cargo fmt --check</code> | clean | 0 |

The release runner was rebuilt (<code>cargo build --release -p lovelace-run</code>) after the source
change and before the fixture and CLI gates, so both gates exercised the new tree.

## 6. What remains

* **Manifest mapping (parity).** The eight ids are still <code>deferred</code> in
  <code>parity/manifest-map.json</code> and the generated <code>parity/manifest.toml</code>.
  Mapping them to the eight test ids above, regenerating the manifest and coverage report, and
  re-running the manifest gate are the next (parity) steps; this round's scope excludes
  <code>parity/</code>.
* **The adjacent IrProgram typing group (7 rows, not claimed here).** The round-50 resweep also
  lists seven <code>MathIrTypingTests</code> rows under "IrProgram.Deserialize/Validate/Infer
  typing API": <code>Deserialize_RejectsMalformedProgram#0</code>,
  <code>Infer_TypesForLoweredExpressions#0</code>, <code>Validate_AcceptsLoweredCorpus#0</code>,
  <code>Validate_RejectsNonBoolSelectGuard#0</code>,
  <code>Validate_RejectsOrderedComparisonOfComplex#0</code>,
  <code>Validate_RejectsOutOfRangeOperand#0</code> and
  <code>Validate_RejectsWrongArity#0</code>. The surface delivered here already implements every
  member those rows call, but their case bodies were outside this round's eight-row block and are
  not mirrored; the five diagnostic tests above cover the same refusals, so a follow-up can add the
  seven direct mirrors cheaply.
* **The rest of the resweep.** The other 73 missing-API groups (203 of the 212 UNRUNNABLE rows)
  are untouched, as is every behavioural family outside this block.
* **Rooting the language builtins on the typed surface.** <code>builtins::ir_text</code> still
  emits the published text with its own emitter (and its n-ary Add/Mul listing). Routing
  <code>compile</code>/<code>lower</code> through <code>mathir</code> would unify the two, but it
  changes published text for three-or-more-term sums and is deliberately out of this additive
  round.
