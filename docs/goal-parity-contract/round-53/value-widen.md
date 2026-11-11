# Round 53 — the Value widen/widen-pair surface and its nine census rows

**Status: additive production surface and nine C# mirrors delivered; every gate is green.**
The nine census ids are still <code>deferred</code> in <code>parity/</code> because this round's
scope excludes parity; mapping them to the test ids in section 4 is the remaining mechanical step.

Measured 2026-11-11 (America/Sao_Paulo) on commit <code>691bb31</code> (round 51-52), with the
working tree changes listed in section 3.

## 1. The nine ids, read from the C# source

The round-50 resweep groups nine manifest rows under "Value widen/widen-pair API". Eight assert
<code>Value.Widen</code> or <code>Value.WidenPair</code>; the ninth asserts <code>Value.ToString</code>.
The ninth is not a widening assertion — it was folded into the same group by the round-45 sweep
because it is another library-only member of the same C# type. Both members are addressed here.

Sources: <code>Lovelace.Suite/Value.cs</code> and <code>Lovelace.Suite.Tests/ValueTests.cs</code>.

| Census id (project <code>Lovelace.Suite.Tests</code>, class <code>ValueTests</code>) | C# line | Member observed | Concrete expected result |
|---|---:|---|---|
| <code>Value_ToString_GivenNatural_PrefixesKindLabel#0</code> | 46 | <code>Value.ToString()</code> on <code>new Value(Nat.Parse("42"))</code> | starts with <code>Natural:</code> and contains <code>42</code>; the implementation yields exactly <code>Natural: 42</code> (Value.cs:304-307) |
| <code>Widen_GivenNaturalToInteger_ReturnsIntegerValue#0</code> | 60 | <code>Widen(ValueKind.Integer)</code> on Natural 5 | Kind Integer, <code>AsInteger() == Int.Parse("5")</code> |
| <code>Widen_GivenNaturalToReal_ReturnsRealValue#0</code> | 70 | <code>Widen(ValueKind.Real)</code> on Natural 5 | Kind Real, <code>AsReal() == Rl.Parse("5")</code> |
| <code>Widen_GivenIntegerToReal_ReturnsRealValue#0</code> | 80 | <code>Widen(ValueKind.Real)</code> on Integer -3 | Kind Real, <code>AsReal() == Rl.Parse("-3")</code> |
| <code>Widen_GivenSameKind_ReturnsSameValue#0</code> | 90 | <code>Widen(ValueKind.Natural)</code> on Natural 7 | <code>Assert.Same(v, widened)</code>; no conversion |
| <code>Widen_GivenWiderToNarrower_ThrowsInvalidOperationException#0</code> | 99 | <code>Widen(ValueKind.Natural)</code> on Integer 3 | <code>InvalidOperationException</code> |
| <code>WidenPair_GivenNaturalAndInteger_ReturnsBothAsInteger#0</code> | 111 | <code>Value.WidenPair</code> on Natural 5 and Integer 3 | both Kind Integer |
| <code>WidenPair_GivenNaturalAndReal_ReturnsBothAsReal#0</code> | 123 | <code>Value.WidenPair</code> on Natural 5 and Real 3.14 | both Kind Real |
| <code>WidenPair_GivenSameKind_ReturnsUnchanged#0</code> | 135 | <code>Value.WidenPair</code> on Natural 5 and Natural 3 | both Kind Natural |

The full census ids are the rows above prefixed with
<code>Lovelace.Suite.Tests/Lovelace.Suite.Tests.ValueTests/</code>. All nine are present verbatim in
<code>parity/manifest.toml</code> (generated from <code>parity/census.json</code>) as
<code>disposition = "deferred"</code> with the round-45 reason naming this missing API.

### The member contract, from Value.cs

* **ValueKind ordinals** (Value.cs:20-39): Natural 0, Integer 1, Real 2, Boolean 3, Text 4,
  Vector 5, Function 6, Void 7, Array 8, Complex 9, Symbolic 10, Record 11, Domain 12, Enum 13.
  The comment in the C# source says the numeric values are load-bearing and are never renumbered.
* **Kind** (Value.cs:180): the tag set by the constructor; <code>Value(Nat)</code> is Natural,
  <code>Value(Int)</code> Integer, <code>Value(Rl)</code> Real.
* **AsNatural / AsInteger / AsReal** (Value.cs:187-193): the payload accessors behind the two
  successful widen assertions.
* **Widen** (Value.cs:245-281):
  1. <code>target == Kind</code> returns <code>this</code>;
  2. either side non-numeric throws
     <code>Cannot widen from {Kind} to {target}: only numeric kinds (Natural, Integer, Real) support widening.</code>
     plus the reference's Complex/Symbolic hint;
  3. <code>target &lt; Kind</code> throws <code>Cannot narrow from {Kind} to {target}.</code>;
  4. the only conversions are (Natural, Integer), (Natural, Real), (Integer, Real).
* **WidenPair** (Value.cs:288-292): <code>target = (ValueKind)Math.Max((int)a.Kind, (int)b.Kind)</code>,
  then <code>a.Widen(target)</code> and <code>b.Widen(target)</code>.
* **ToString** (Value.cs:304-321): <code>"Kind: value"</code> for the numeric kinds, Complex,
  Symbolic, Boolean, Vector and Array; Text is raw; Function is <code>Function: name</code>;
  Record is <code>Record: TypeName</code>; Domain is <code>Domain: lowercased</code>; Enum is
  <code>Enum: Type.Member</code>; Void is <code>Void</code>.

## 2. Member map (C# to Rust)

All additions live at <code>lovelace_lang::value_widen</code> and on the existing
<code>lovelace_lang::Value</code>. Nothing existing was renamed or changed.

| C# member | Rust addition | Observable preserved |
|---|---|---|
| <code>ValueKind</code> enum | <code>value_widen::ValueKind</code>, <code>#[repr(u8)]</code>, same 14 names and ordinals; <code>rank()</code>, <code>is_numeric()</code>, <code>name()</code> | the tag and the ordinals used by <code>Widen</code> and by <code>WidenPair</code>'s <code>max</code> |
| <code>Value.Kind</code> | <code>Value::kind() -&gt; ValueKind</code> | same tag for every Rust variant; <code>RealClamped</code> reports Real; Vector and Array stay distinct |
| <code>Value.AsNatural()</code> | <code>Value::as_natural() -&gt; Option&lt;&amp;Nat&gt;</code> | payload read for a Natural |
| <code>Value.AsInteger()</code> | <code>Value::as_integer() -&gt; Option&lt;&amp;Int&gt;</code> | payload read for an Integer; used by the Natural-&gt;Integer mirror |
| <code>Value.AsReal()</code> | <code>Value::as_real() -&gt; Option&lt;&amp;Real&gt;</code> | payload read for Real, including a digit-capped <code>RealClamped</code> |
| <code>Value.Widen(ValueKind)</code> | <code>Value::widen(ValueKind) -&gt; Result&lt;Value, LangError&gt;</code> | ladder, same-kind unchanged value, narrowing refusal, non-numeric refusal with the reference messages and hints |
| <code>Value.WidenPair(a, b)</code> | <code>Value::widen_pair(&amp;a, &amp;b) -&gt; Result&lt;(Value, Value), LangError&gt;</code> | both operands widened to the higher ordinal; both kinds equal on success |
| <code>Value.ToString()</code> | <code>Value::kind_prefixed()</code> and <code>impl Display for Value</code> | natural 42 renders <code>Natural: 42</code>; Text raw; Void <code>Void</code>; the other kinds per Value.cs:304-321 |

Two deliberate representation differences, neither observed by these nine rows:

1. C#'s same-kind <code>Widen</code> returns the same reference and the C# test uses
   <code>Assert.Same</code>. A Rust enum value is immutable and has no reference identity, so the
   mirror returns an equal clone and the test asserts structural equality plus the unchanged
   payload. The contract that matters — no conversion and no new kind — is preserved.
2. The <code>As…</code> accessors return <code>Option</code> instead of raising
   <code>ValueShapeException</code>. All nine rows call them only after a successful widen, so the
   throwing shape is not part of this block's observable contract.

## 3. Production change

* **New:** <code>rust/lovelace-lang/src/value_widen.rs</code> — the public
  <code>ValueKind</code>, the inherent <code>Value</code> methods above, the
  <code>Display</code> impl, the reference's widening error/hint wording, and the nine mirrors.
* **Edited:** <code>rust/lovelace-lang/src/lib.rs</code> — one line, <code>pub mod value_widen;</code>.
* **Untouched:** the private widening copies inside <code>builtins.rs</code>
  (<code>widen</code>, <code>kind_rank</code>, <code>numeric_pair</code>) and every existing
  <code>Value</code> method (<code>kind_name</code>, <code>display</code>, <code>typed</code>,
  <code>is_numeric</code>). No <code>.cs</code>, fixture, corpus, <code>parity/</code> or other
  document was modified.

## 4. The nine mirrors

All tests are in <code>lovelace-lang::value_widen::tests</code>. Each uses the same concrete value
as its C# row; where the C# row asserts only the kinds (the three <code>WidenPair</code> rows), the
mirror also pins the payloads, and where the C# row asserts reference identity, the mirror pins
structural equality.

| Census id | Rust test | Assertion |
|---|---|---|
| <code>Value_ToString_GivenNatural_PrefixesKindLabel#0</code> | <code>value_widen::tests::value_to_string_given_natural_prefixes_kind_label</code> | <code>to_string()</code> starts with <code>Natural:</code>, contains <code>42</code>, equals <code>Natural: 42</code> |
| <code>Widen_GivenNaturalToInteger_ReturnsIntegerValue#0</code> | <code>widen_given_natural_to_integer_returns_integer_value</code> | kind Integer and payload 5 |
| <code>Widen_GivenNaturalToReal_ReturnsRealValue#0</code> | <code>widen_given_natural_to_real_returns_real_value</code> | kind Real and payload exactly <code>Real("5")</code> |
| <code>Widen_GivenIntegerToReal_ReturnsRealValue#0</code> | <code>widen_given_integer_to_real_returns_real_value</code> | kind Real and payload exactly <code>Real("-3")</code> |
| <code>Widen_GivenSameKind_ReturnsSameValue#0</code> | <code>widen_given_same_kind_returns_same_value</code> | kind Natural, equal value, payload 7 |
| <code>Widen_GivenWiderToNarrower_ThrowsInvalidOperationException#0</code> | <code>widen_given_wider_to_narrower_throws_invalid_operation_exception</code> | <code>Err</code> message <code>Cannot narrow from Integer to Natural.</code>, code <code>InvalidOperation</code> |
| <code>WidenPair_GivenNaturalAndInteger_ReturnsBothAsInteger#0</code> | <code>widen_pair_given_natural_and_integer_returns_both_as_integer</code> | both Integer, payloads 5 and 3 |
| <code>WidenPair_GivenNaturalAndReal_ReturnsBothAsReal#0</code> | <code>widen_pair_given_natural_and_real_returns_both_as_real</code> | both Real, payloads 5 and 3.14 |
| <code>WidenPair_GivenSameKind_ReturnsUnchanged#0</code> | <code>widen_pair_given_same_kind_returns_unchanged</code> | both Natural, payloads 5 and 3 |

## 5. Gate outputs

Baseline was 4,971 workspace tests; the nine new mirrors make 4,980. Cargo commands ran from
<code>rust/</code>, the Node gates from the repository root, with the repository toolchain
environment (<code>RUSTUP_HOME</code>, <code>CARGO_HOME</code>, <code>PATH</code>, <code>TMP</code>).

| Gate | Command | Observed | Exit |
|---|---|---|---:|
| Workspace suite | <code>cargo test --workspace</code> | <code>TOTAL passed=4980 failed=0</code> | 0 |
| New mirrors | <code>cargo test -p lovelace-lang value_widen</code> | <code>9 passed; 0 failed; 0 ignored; 1475 filtered out</code> | 0 |
| Corpus | <code>cargo run --release -p lovelace-corpus</code> | <code>corpus: 214 passed, 0 failed, 214 total</code> | 0 |
| Protocol fixtures | <code>node parity/tools/gate-fixtures.mjs</code> | <code>oracle: 20/20</code>, <code>rust: 20/20</code>, <code>PASS</code> | 0 |
| CLI matrix | <code>node parity/tools/gate-cli.mjs</code> | <code>CLI matrix: 16/16</code>, <code>PASS: every RUN-001..RUN-014 row behaves like the oracle</code> | 0 |
| Clippy | <code>cargo clippy --workspace --all-targets -- -D warnings</code> | clean, no warnings | 0 |
| Format | <code>cargo fmt --check</code> | clean | 0 |

The release runner was rebuilt (<code>cargo build --release -p lovelace-run</code>) after the
source change before the fixture and CLI gates, so both gates exercised the new tree.

## 6. What remains

* **Manifest mapping.** The nine ids are still <code>deferred</code> in the generated
  <code>parity/manifest.toml</code> and <code>parity/manifest-map.json</code>. Mapping them to the
  nine test ids in section 4, regenerating the manifest and the coverage report, and re-running the
  manifest gate are the next (parity) steps; they were outside this round's scope. The Rust test
  names above are the exact measured ids from <code>cargo test -p lovelace-lang -- --list</code>.
* **The rest of the resweep.** The other 73 missing-API groups (203 of the 212 UNRUNNABLE rows)
  are untouched, as is every behavioural family outside this group.
* **Wrong-kind As calls.** If a future row needs the C# <code>ValueShapeException</code> wording on
  a wrong-kind <code>As…</code> call, that is a separate additive accessor; no row in this block
  observes it.
