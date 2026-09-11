# D-D — Robustness diagnosis: deep-input stack overflow + five wire/hygiene defects

**Round:** goal-cycle-5, diagnosis only.
**Method:** read-only static analysis of the in-scope sources, cross-checked against the round-5 **pre-existing** probe
outputs in `docs/goal-cycle-5/probes/pre/` and `docs/goal-cycle-5/probes/pre2/` (produced by
`run-probes.ps1` and `run-probes2.ps1` before this round). **Nothing was built, tested, or executed in this
round** (the orchestrator holds the build/AOT lock), so every runtime statement below is a citation of an existing
probe artifact, and every mechanism statement is a citation of code.

Files read outside the round's in-bounds list are marked **[corroboration]** and were opened read-only to confirm a
wire shape; no file other than this dossier was created or modified.

---

## 0. Premise correction: the ticket's T0-6 repro does not reproduce as written

The ticket says *"a 3000-deep nested script (e.g. 3000 nested parentheses **or** 3000 nested function calls) kills the
published AOT binary"*. The second example reproduces; **the first does not.** The repo's own baseline probes
contradict it:

| probe (`run-probes2.ps1:31-35`) | shape | 1000 | 2000 | 3000 |
|---|---|---|---|---|
| `d-paren` | `('(' * n) + '1' + (')' * n)` | exit=0 | exit=0 | **exit=0** (482 B envelope, `docs/goal-cycle-5/probes/pre2/_summary.txt`) |
| `d-unary` | `('-' * n) + '1'` | exit=0 | exit=0 | **exit=-1073741571** |
| `d-call` | `('abs(' * n) + '1' + (')' * n)` | exit=0 | exit=0 | **exit=-1073741571** |
| `d-addleft` | `('(' * n) + '1' + ('+1)' * n)` | exit=0 | exit=0 | **exit=-1073741571** |
| `d-array` | `('[' * n) + '1' + (']' * n)` | exit=0 | exit=0 | **exit=-1073741571** |

`run-probes.ps1:14` builds the ticket's own `t06-depth-3000` as `('(' * 3000) + '1' + (')' * 3000)` and
`docs/goal-cycle-5/probes/pre/t06-depth-3000.out.txt` is a **normal success envelope**
(`"ok":true ... "display":"1"`, `t06-depth-3000  exit=0  bytes=480` in `pre/_summary.txt`). The same is true
at 2000 (`t06b-depth-2000.out.txt`).

Two byte-count traps worth recording, because both appear in the raw artifacts:

1. The harness records `bytes=98` for the four crashing runs (`pre2/_summary.txt`), but that is **not**
   stdout: the probe merges stderr with `2>&1` (`run-probes2.ps1:51`), and the 98 bytes are PowerShell's
   `System.Management.Automation.RemoteException` + `Process is terminating due to StackOverflowException.`
   (`pre2/d-unary-3000.out.txt:1-2`). The process's **stdout was zero bytes**, exactly as the ticket reports —
   the envelope is written only after a successful evaluation (`Runner.cs:227-230`) or from a `catch`
   (`Runner.cs:234-251`), and a .NET stack overflow is an uncatchable fail-fast.
2. The `.ls` inputs are single lines longer than the reading tool's 2000-char line cap, so their on-disk length
   must be taken from the generator, not from the reading tool.

**Consequence for the fix:** the discriminator is **AST depth, not parse depth** (see section 1.3), and the acceptance
criteria are the four `*-3000` shapes above **plus** the requirement that the currently-green `d-paren-*` and
`t06*` probes stay green.

---

## DEFECT 1 (T0-6) — native stack overflow on deep input

### Symptom

Input whose **expression tree** is ~3000 levels deep kills the process with `exit=-1073741571` (`0xC00000FD`,
`STATUS_STACK_OVERFLOW`), emitting **zero stdout bytes**, so the agent-facing protocol cannot report it. Depth
2000 succeeds for all five shapes; depth 3000 succeeds only for the shape that produces no deep tree (`d-paren`).
Nested *parentheses* — the ticket's first example — are a non-reproducer on this binary.

### Repro command (do **not** run in this round — the lock is held)

```powershell
Set-Location C:\Users\ricar\dev\LovelaceSharp
$shape = ('abs(' * 3000) + '1' + (')' * 3000)          # d-call; also: ('-'*3000)+'1', ('['*3000)+'1'+(']'*3000)
[System.IO.File]::WriteAllText("$env:TEMP\d-call-3000.ls", $shape)
out\aot\Lovelace.Run.exe --file "$env:TEMP\d-call-3000.ls" --omit-functions
# observed (docs/goal-cycle-5/probes/pre2/d-call-3000.out.txt):
#   exit -1073741571 ; stdout: 0 bytes ; stderr: "Process is terminating due to StackOverflowException."
```

Control that must keep exiting 0 afterwards: `('(' * 3000) + '1' + (')' * 3000` (`pre/t06-depth-3000.ls` via
`run-probes.ps1:14`).

### Root cause

**`Interpreter.EvaluateAsync(Expr expr, Scope scope)` (`Lovelace.Suite/Interpreter.cs:280`) is an unbounded
recursive-async walk of the AST: one `Evaluate*Async` state machine plus one `EvaluateAsync` state machine per
AST level.** No depth guard exists anywhere in the tree (repo-wide grep for `[Dd]epth|StackOverflow|MaxNesting`
finds guards only in unrelated numerical/canonicaliser code, never in `Parser`, `Interpreter`,
`ValueFormatter` or `Printing`'s printer).

The two structural facts that make the evaluator — not the parser — the binding constraint:

**(1) Depth that costs nothing: parentheses add parser frames but no AST nodes.** `Parser.cs:559-565`:

```csharp
559:         if (Current.Kind == TokenKind.LParen)
560:         {
561:             Advance();
562:             var inner = ParseAssignment();
563:             Expect(TokenKind.RParen);
564:             return inner;                       // <- no node: the group collapses
565:         }
```

By contrast `Parser.cs:512-528` wraps list elements in `new ListExpr(elements)` and `Parser.cs:536-552`
wraps call arguments in `new CallExpr(name, args)`. `d-paren`, `d-call` and `d-array` therefore run
the **same** 9-frame-per-level parser descent (section 1.4 row 1), yet only `d-paren` survives 3000 — the only
difference is whether an AST node is built.

**(2) The crashing shapes cannot be the parser, because their parse stack is *shallower* than a surviving one.**
`d-unary` is `Parser.cs:418-421`:

```csharp
418:         if (Current.Kind == TokenKind.Minus)
419:         {
420:             Advance();
421:             return new UnaryExpr(UnaryOp.Negate, ParseUnary());   // 1 frame per '-'
422:         }
```

3000 unary minuses push about 3003 parser frames; `d-paren-3000` pushes about 27 000 parser frames (9 per level)
and **succeeds**. A stack that holds 27 000 parser frames cannot be exhausted by 3 003 of them. The extra depth in
`d-unary` exists only *after* the parse: `-(-(-...1...))` is a 3000-deep `UnaryExpr` chain, and the
evaluator walks it at `Interpreter.cs:565`:

```csharp
563:     private async Task<Value> EvaluateUnaryAsync(UnaryExpr unary, Scope scope)
564:     {
565:         var operand = await EvaluateAsync(unary.Operand, scope);
```

Elimination for the two shapes whose result is a scalar (`d-unary` → Integer, `d-call` → Natural): between the
parse and the envelope there is no other deep recursion — `ValueFormatter.Format` on a scalar is one switch arm
(`ValueFormatter.cs:16-17`), the structured projection of a scalar is one branch, the result has no nested
children, and `Interpreter.ExecuteAsync(Program)` iterates statements (it does not recurse per statement). So the
only per-level recursion left is the evaluator. For `d-addleft` the tree is left-deep
(`ParseAdditive` builds `new BinaryExpr(left, op, right)` in a loop, `Parser.cs:329`, so the *parse* is
iterative) and the evaluator descends the left spine at `Interpreter.cs:356`; for `d-array` it descends the
element at `EvaluateListAsync` (`Interpreter.cs:735`).

**Why ~3000 levels of evaluation exhaust a stack that survives ~27 000 parser frames.** Each AST level costs *two*
`async` state machines (`EvaluateAsync` + the specific `Evaluate*Async`), and an async frame carries a
builder, an awaiter and hoisted locals — an order of magnitude more stack than the tiny synchronous parser leaves.
2000 levels survive, 3000 do not, for every AST-deep shape; that boundary is consistent with a default ~1 MB
main-thread stack and ~150-350 B per async frame. **[Speculation — not measured this round]**: the exact per-frame cost
and the true threshold were not measured (no execution permitted); the *ordering* conclusion above does not depend on
them.

**Also unbounded in the same walk (same class, different trigger):** user-function recursion has no guard either —
`Interpreter.cs:622-640 CallUserFunctionAsync` → `634: return await ExecuteStatementListAsync(fn.Body, frame);`
→ `Interpreter.cs:877/882 ExecuteAsync(Statement, scope)` → `606 EvaluateCallAsync` → `622` again. So
`func f(x) = f(x); f(1)` is a second, input-independent route to the same uncatchable `0xC00000FD`
**[not probed this round — read from code]**.

### 1.4 Every recursive descent that can be entered once per input nesting level

Ordered by where it sits in the pipeline (`SuiteEngine.EvaluateCoreAsync`, `SuiteEngine.cs:269-287`:
`Tokenize` → `ParseProgram` → `ExecuteAsync`, then the runner renders at `Runner.cs:201-225`).

| # | Site (file:line) | Frames / level | Trigger | On the runner path? |
|---|---|---|---|---|
| 1 | `Parser.cs:274` `ParseAssignment` → `:288` `ParseComparison` → `:314` `ParseAdditive` → `:336` `ParseMultiplicative` → `:363` `ParsePower` → `:394` `ParseRange` → `:416` `ParseUnary` → `:432` `ParsePostfix` → `:497` `ParsePrimary` → `:562` back to `ParseAssignment` | 9 | `(`, `[`, call args, list elements, index specs | yes — **measured to survive 3000** (`d-paren-3000`) |
| 2 | `Parser.cs:416-429` `ParseUnary` → `:421`/`:426` self | 1 | `-`/`+` runs (`d-unary`) | yes |
| 3 | `Parser.cs:382-390` `ParsePowerTail` → `:389` `ParsePower` → `:379` `ParseRange`... | 2 | `^` chains | yes |
| 4 | `Parser.cs:114` `ParseStatement` → `:140` `ParseBlock` → `:147`; `:220` `ParseIf`, `:239` `ParseWhile`, `:254` `ParseFor` → `ParseStatement` | 2 | `{{{...}}}`, `if(1)if(1)...` | yes (untested) |
| 5 | `Parser.cs:474` `ParseIndexSpec` → `:476`/`:483`/`:490` `ParseAssignment` | 9 | nested `a[b[c[...]]]` | yes |
| 6 | `Parser.cs:629-633` `ParseSubExpression` → `new Parser().Parse(tokens)` | 9 | interpolation body of an interpolated string | yes — **resets any per-instance counter** |
| 7 | **`Interpreter.cs:280` `EvaluateAsync(Expr, Scope)`** — re-entered from `:336`, `:356-357`, `:565`, `:586`, `:610`, `:648-650`, `:654`, `:679`, `:735`, `:799`, `:996` | **2 (async)** | **every AST-deep shape** | **yes — the measured killer** |
| 8 | `Interpreter.cs:877` `ExecuteAsync(Statement, Scope)` → `:914` `ExecuteBlockAsync` → `:869` `ExecuteStatementListAsync` → `:873` | 3 | statement nesting | yes |
| 9 | `Interpreter.cs:622` `CallUserFunctionAsync` → `:634` `ExecuteStatementListAsync` | ~6 | user-function recursion | yes (no guard) |
| 10 | `Interpreter.cs:1172` `CollectSymbols` (via `:1104` `FreeSymbolsOf`, `:1257` `inspect`) | 1 | symbolic subexpression depth | yes (symbolic only) |
| 11 | `ValueFormatter.cs:14` `Format` → `:27`/`:28` `FormatArray` → `:37-40` `FormatLevel` → `:49` `Format` | 2-3 | nested vectors/arrays | yes (after eval) |
| 12 | `Value.cs:294-311` `ToString` → `:300` `Printing.PrettyPrint`; `:303-304` `ValueFormatter.Format` | 1 + #13 | any `Value.ToString()` on a container | yes |
| 13 | `Printing.cs:23` `PrintExpr`, `:524` `Pretty` (self-calls at :549, :552, :556, :560, :572, :575, :618-692, :1025, :1030), `:435` `DebugPrint`, `:767` `LatexRender`, `:411` `ExprDepth`, `:288` `CollectSymbols` | 1-2 | symbolic subexpression depth | yes |
| 14 | `Ast.cs:52-91` (`Expr` records) and `Ast.cs:110-134` (`Statement` records) — **compiler-generated `ToString`/`Equals`/`GetHashCode` recurse structurally** | 1-2 | any `ToString()` on a node, `Assert.Equal(astA, astB)`, use as a dictionary key, debugger watch | **no** — verified: no `.Equals(`/`ToString()` on an `Expr` in `Runner.cs`, `SuiteEngine.cs`, `Interpreter.cs`; those files only ever `switch` on node types, which does not call `Equals`. Listed because it is equally uncatchable; it is *not* the T0-6 trigger. |
| 15 | Symbolics canonicaliser / evaluation / `Expr` equality (out of scope this round) | ? | deep pre-built symbolic `Expr` | only via a host `SetVariable`/`EvaluateAsync(Expr)` (`Interpreter.cs:210`), not via `--eval` once row 7 is bounded |

Ruled out as triggers: `Tokenizer.Tokenize` (`Tokenizer.cs:22` — single `while`, no self-call);
`ScriptSource.ToSemicolonStatements` (`ScriptSource.cs:35-39` — single `for` with an `int depth`
counter) **[corroboration]**; `StructuredProjection` / JSON serialisation (reachable only after a successful
evaluation).

### 1.5 Which one overflows first, and the two candidates

**Confirmed from the artifacts (not speculation): the shapes that die are exactly the shapes with a deep AST, and the
parser is provably not the binding constraint** — the argument is the frame-count comparison in section 1.3(2), which
needs no measurement. Candidate #1 = **the evaluator**, `Interpreter.EvaluateAsync` (`Interpreter.cs:280`)
with its per-shape partner:

* `d-unary` → `:563` `EvaluateUnaryAsync` (and `:571-577` `UnaryOp.Negate`);
* `d-call` → `:606` `EvaluateCallAsync` → `:610` argument loop;
* `d-addleft` → `:354` `EvaluateBinaryAsync` → `:356` `bin.Left`;
* `d-array` → `:735` `EvaluateListAsync` → element loop.

**Which to instrument first:** `d-unary`. It is the cleanest experiment — about 3003 parser frames plus 3000
evaluation levels and nothing else deep anywhere in the pipeline, so a counter or breakpoint at
`Interpreter.cs:280` either reaches ~3000 and dies there (candidate #1 confirmed outright) or never exceeds a few
hundred (candidate #2).

**Candidate #2 (test only if #1 is refuted): the renderers**, `ValueFormatter.FormatLevel`
(`ValueFormatter.cs:43-60`) and the structured projection, which are also unbounded and are reached after
evaluation for `d-array` (whose value is a 3000-deep nested vector). Note the 2000-deep array already produced a
16 528-byte envelope (`pre2/d-array-2000`), so the renderer survives 2000; it is the fallback explanation only for
`d-array`, and it cannot explain `d-unary`/`d-call`, whose results are scalars.

### Minimal fix proposal — one bounded change

**Where the limit belongs: at the evaluator's single choke point, `Interpreter.EvaluateAsync(Expr, Scope)`
(`Interpreter.cs:280`).** Every per-level expression descent — all 11 re-entry sites in inventory row 7, for every
node kind — passes through that one method, so one counter there bounds *all* of `d-unary`, `d-call`,
`d-addleft`, `d-array` **and** unbounded user-function recursion (row 9 also routes through it), while
leaving parsing, printing and rendering untouched. It is also already the right object: `SuiteEngine` owns exactly
one `Interpreter` (`SuiteEngine.cs:25`), and an `int` field costs nothing per node.

```csharp
// Interpreter.cs — one mechanism, one choke point
private const int MaxExpressionDepth = 1024;   // see the limit note below
private int _evalDepth;

private async Task<Value> EvaluateAsync(Expr expr, Scope scope)
{
    if (_evalDepth >= MaxExpressionDepth)
        throw new EvaluationDepthExceededException(MaxExpressionDepth, expr.GetType().Name);
    _evalDepth++;
    try
    {
        switch (expr) { /* unchanged */ }
    }
    finally { _evalDepth--; }                   // mandatory: the instance is reused across evaluations
}
```

Typed error + one taxonomy arm (the house pattern already exists: `ReentrancyNotSupportedException` lives in
`Lovelace.Suite/EngineExceptions.cs:10` and is classified **before** the `InvalidOperationException` arm):

```csharp
// EngineExceptions.cs
public sealed class EvaluationDepthExceededException(int limit, string nodeKind) : Exception(
    $"Expression nesting exceeds the {limit}-level evaluation limit (at {nodeKind}); " +
    "simplify the expression or split it into statements.");

// Runner.cs:256 Classify — place above the InvalidOperationException arm (line 267) if it derives from it
Lovelace.Suite.EvaluationDepthExceededException => ("DepthExceeded", "BudgetExceeded", true),
```

`SuiteEngine.ToDiagnostic` (`SuiteEngine.cs:344-355`) needs no change; include `at position N` in the
message only if a source position is available at the throw site (it is not — the AST carries no positions — so the
diagnostic will land at position 0, which is honest).

**Second-order hardening, deliberately a separate change:** the parser (rows 1-6) and the renderers (rows 11-13) are
still unbounded. They are *not* the T0-6 cause at n=3000, but they will be at some larger n. If the same constant is
reused there, **mind the baseline**: `d-paren-1000/2000/3000` and `t06-depth-3000` are currently **green**,
so a parser limit below ~3000 silently converts them into failures. Recommended split: **parse limit >= 4096** (keeps
the green baseline green, still bounds the ~27 000-frame descent), **evaluation limit ~1024** (3x margin below the
measured failure boundary at 3000, 4x above anything a handwritten script produces). Value choice is a judgement call —
see Open questions. Do **not** use `RuntimeHelpers.TryEnsureSufficientExecutionStack()` as the only guard: it is
host-stack-dependent and gives a non-reproducible threshold; a counter gives a stable, reportable one.

### Test that would FAIL on the current tree

```csharp
// Lovelace.Suite.Tests/EvaluationDepthTests.cs
[Theory]
[InlineData("-",    3000)]   // d-unary : 3003 parser frames + 3000 AST levels
[InlineData("abs(", 3000)]   // d-call
public void DeepAst_IsRefusedWithATypedError(string shape, int n)
{
    string src = shape == "-" ? new string('-', n) + "1"
                              : string.Concat(Enumerable.Repeat(shape, n)) + "1" + new string(')', n);
    var ex = Assert.Throws<EvaluationDepthExceededException>(() => new SuiteEngine().Evaluate(src));
    Assert.True(ex.Limit > 0);
}

// control: the guard must not be over-eager — this probe is green today (pre2/d-paren-3000, pre/t06-depth-3000)
[Fact] public void DeepParens_StillEvaluate() =>
    Assert.Equal("1", new SuiteEngine().Evaluate(new string('(', 3000) + "1" + new string(')', 3000)).ToString());
```

On the current tree the first test does not fail an assertion — **it terminates the test host** with exit
`-1073741571`, because a .NET stack overflow cannot be caught. The in-process form is therefore not diagnostic on
its own; pair it with the protocol-level test that does produce a readable failure:

```csharp
// Lovelace.Run.Tests/DeepInputProtocolTests.cs — spawn out/aot/Lovelace.Run.exe (or the test's runner host)
var (exit, stdout, _) = RunProcess(exe, "--file", "d-call-3000.ls", "--omit-functions");
Assert.Equal(1, exit);                                     // today: -1073741571
var env = JsonNode.Parse(stdout)!;                         // today: throws, stdout is 0 bytes
Assert.Equal("DepthExceeded", env["code"]!.GetValue<string>());
Assert.Equal("BudgetExceeded", env["category"]!.GetValue<string>());
```

Run the shape matrix (5 shapes x {2000, 3000}) in the child process, and assert the four crash shapes now return
`ok:false` while `d-paren-3000` still returns `ok:true`.

### Blast radius

* **Protocol:** a new stable `code` (`DepthExceeded`) in the error taxonomy. Additive; no existing
  `code` changes.
* **Existing green probes change if the parser is also limited below 3000** (see above). Keep the parse limit high.
* **Hosts that legitimately build deep expressions** now fail: `SuiteEngine.Evaluate` /
  `EvaluateAsync` (`SuiteEngine.cs:211-290`), `Interpreter.EvaluateAsync(Expr)`
  (`Interpreter.cs:210`), Studio (`Lovelace.Studio/IncrementalRunner.cs:193`), the REPL.
  `Solve` / `Integrate` / `Optimize` can emit expressions whose depth grows with input size (the cubic
  solution printed in `pre2/a15-solve-cubic-print.out.txt` is ~10 deep); 1024 leaves a wide margin, but the value
  must be validated against the full sweep, not chosen in the abstract.
* **Statefulness:** the counter must be decremented in `finally`; `SuiteEngine` reuses one `Interpreter`
  for the whole session (`SuiteEngine.cs:25`), so a leaked increment would poison every later evaluation. The
  engine serialises evaluations (`SuiteEngine.cs:197`, `:227`), so one `int` is safe — document that the
  counter assumes the gate.
* **`Task`/`async`:** the counter bounds *logical* depth, which is what matters; it does not depend on whether
  an `await` actually yields.
* **Not addressed by this change:** `{{{...}}}` statement nesting (`Parser.cs:140`) and deep *symbolic* values
  that a host injects via `SetVariable` (`SuiteEngine.cs:297`) — the latter is still unbounded through
  `inspect` (`Interpreter.cs:1172`) and the printers (`Printing.cs:524`).

### Open questions

1. **The exact depth boundary per shape** (only 2000/3000 were probed). The safe evaluation limit depends on the gap
   between the largest real expression the suite produces and the smallest failing n; a bisect at 2200/2400/2600/2800
   for `d-unary` would sharpen it. **[needs execution]**
2. **Is the 1 MB-stack assumption valid for the published AOT image**, and does `Lovelace.Studio` host the engine
   on a smaller-stack thread? If so the evaluation limit must be lower or configurable (a
   `SuiteEngine.MaxExpressionDepth` property, mirroring `ComputationDecimalPlaces`, is the natural home).
3. **Does `d-paren` fail at some larger n** (10 000? 30 000?) in the parser? The parser is unbounded; my
   frame-count argument bounds it only at ~27 000 frames. **[untested]**
4. **`{{{...}}}` at 3000** — statement nesting has no guard and was not probed; ~2 cheap frames/level suggests it
   survives, but "suggests" is not evidence.
5. **User-function recursion depth**: is a separate, smaller limit wanted (a call budget is semantically different from
   an expression budget, and solve-style recursive builtins should not be able to consume it)? Out of scope this round;
   the same counter would bound it as a side effect, which may be too coarse.

---

## DEFECT 2 — smaller wire/hygiene defects

### (a) `print("A"); print("B")` appends a stray CR

**Symptom / evidence.** `docs/goal-cycle-5/probes/pre/t1-print-cr.out.txt`:
`"output":["A\r","B"]` — the first element carries a trailing CR; the last does not.
**Repro.** `run-probes.ps1:25` → `out\aot\Lovelace.Run.exe --file t1-print-cr.ls --omit-functions`.
**Root cause.** The printer emits the platform newline and the splitter only strips the final one:

```csharp
Interpreter.cs:1542   Output.WriteLine(string.Join(" ", args.Select(v => ValueFormatter.Format(v, UnicodeOutput))));
Interpreter.cs:240    var capture = new StringWriter();                       // NewLine = Environment.NewLine = CRLF
Runner.cs:304-305     private static string[] SplitLines(string text) =>
                          text.Length == 0 ? Array.Empty<string>() : text.TrimEnd('\r', '\n').Split('\n');
```

`"A\r\nB\r\n"` → `TrimEnd('\r','\n')` → `"A\r\nB"` → `Split('\n')` → `["A\r","B"]`.
Second candidate producer (does not affect JSON): `PrintText` re-adds newlines with `sb.AppendLine`
(`Runner.cs:312-313`).
**Minimal fix.** Normalise before splitting — `text.Replace("\r\n", "\n").TrimEnd('\n').Split('\n')` — or set the
capture writer's `NewLine = "\n"` at `Interpreter.cs:240`. Prefer the splitter fix: the same defect class
exists in Studio's copies **[corroboration]** `Lovelace.Studio/EngineHost.cs:319` and
`IncrementalRunner.cs:418`.
**Test that fails.** Envelope test asserting `Assert.Equal(new[]{"A","B"}, dto.Output)` — today the array is
`["A\r","B"]`.
**Blast radius.** Any consumer comparing `output` strings; `SplitLines` also feeds the cancelled-run
`partialOutput` (`Runner.cs:242`). Existing goldens may have baked in the CR — check
`Lovelace.Run.Tests` fixtures first.
**Open question.** Is the protocol's `output` specified as LF-only? If not, the spec should say so before the fix
lands.

### (b) Free symbols are reported with two encodings

**Evidence, both visible in one file.** `pre2/a04-inspect-neg100000.out.txt` (an `inspect(...)`) carries the
record field `free_symbols: []`, while `pre2/a02-subs-root.out.txt` and `pre/t01-*.out.txt` carry
`"freeSymbols":[]` in the same envelope's `result.structured`.
**Producer 1 (structured Value).** `Interpreter.cs:1104-1111` `FreeSymbolsOf` → `:1172`
`CollectSymbols` returns a `Value[]` of *Text* values; installed as the `free_symbols` field at
`Interpreter.cs:1076` (`new("free_symbols", new Value(FreeSymbolsOf(v)))`), exposed by `inspect`
(`Interpreter.cs:1257-1261`), and declared an `Array` in `RecordSchemas.cs:122`.
**Producer 2 (flat string array).** `Printing.cs:318-319`
(`FreeSymbolNames(Expr e) => CollectSymbols(e).Select(s => s.Name).ToArray()`) → `StructuredProjection.cs:104`
(`FreeSymbols: Printing.FreeSymbolNames(e).ToArray()`) → the DTO field `string[]? FreeSymbols`
(`StructuredProjection.cs:27`).

So the same fact crosses the wire once as a **structured Array of Text values inside the `Inspection` record** and
once as a **bare `string[]`** on `structured.freeSymbols`, through **two hand-duplicated 13-case traversals**
that have already drifted:

* `Printing.cs:285-287` documents that "relations, piecewise guards, derivatives, integrals, **RootOf**, logic and
  order terms are all traversed", but the switch at `Printing.cs:293-311` has **no `RootOfExpr` case** (it
  handles Symbol/Add/Multiply/Power/Function/Relation/Piecewise/Derivative/Integral/And/Or/Not/Order) —
  `Printing.cs:45`, `:453`, `:668`, `:915` do handle `RootOfExpr` elsewhere in the same file,
  so the omission is local to the traversal. `Interpreter.CollectSymbols` (`:1176-1221`) repeats the same
  13 cases and also omits `RootOfExpr` (confirmed: a repo-wide `RootOf` grep finds no hit in
  `Interpreter.cs`). Both producers therefore under-report `rootof(x^2-2, 1)`.
* Ordering/dedup differs: `Printing.CollectSymbols` uses `SortedSet<Symbol>` (`Printing.cs:290`) →
  deterministic name order; the interpreter uses `List<string>` + `Distinct()` (`Interpreter.cs:1108-1110`)
  → first-occurrence order. The two encodings can disagree on order for the same expression.
* The probe that was supposed to cover this (`run-probes2.ps1:28`, `a21-free-symbols` = `inspect(x*y)`)
  exited 1 with `"Undefined variable 'y'"` (`pre2/a21-free-symbols.out.txt`) — the free-symbol path was
  **not** exercised by the baseline.

**Minimal fix.** Delete `Interpreter.CollectSymbols`; map `Printing.FreeSymbolNames(v.AsSymbolic())` to
`new Value(name)` in `FreeSymbolsOf`. One traversal, one ordering, and the `RootOfExpr` case gets fixed
once.
**Test that fails.** (i) `Assert.Equal(new[]{"x"}, Printing.FreeSymbolNames(rootof(x*x-2, 1)))` — today returns
empty; (ii) an equivalence test that `inspect(e)`'s `free_symbols` names and `structured.freeSymbols`
agree for a symbolic expression that uses a symbol twice in reverse order (`inspect(y + x + y)`): the ordering rule
is currently two rules.
**Blast radius.** `Inspection` schema consumers (`Lovelace.Studio.Tests/StructuredPayloadTests.cs:131` asserts
`{"x"}` — unaffected); `SymbolicsPlugin.cs:1150/1162-1163` already uses `Printing.FreeSymbolNames`, so
the interpreter-side change aligns the Suite with the Symbolics plugin; any golden envelope containing a `rootof`
changes (it becomes *more* correct).
**Open question.** Which encoding is the contract — should `structured.freeSymbols` be projected from the same
`Value` (so both are Array-of-Text), or should the `Inspection` record's field become a flat string array?
Decide once, then make one producer.

### (c) Every error envelope omits `elapsedTime`/`timings`

**Symptom / evidence.** `pre/t2-parse-fail.out.txt` ends
`...,"diagnostics":[...],"elapsed":"156.7 us"}` — no `elapsedTime`, no `timings`. Every `ok:true`
envelope carries both (`pre2/d-paren-3000.out.txt`:
`"elapsedTime":{"value":1.09,"unit":"ms"},"timings":[...]`).
**Root cause — the construction site.** `Runner.cs:277-293`:

```csharp
284:             WriteJson(stdout, new RunErrorDto(ProtocolVersion, Lovelace.Symbolics.Printing.FormatHeader, MathIrVersion,
285:                 false, code, category, message, recoverable, diagnostics, elapsed, output, variables),
```

`elapsed` is the display **string** only; the call site supplies it at `Runner.cs:249-250`
(`..., engine.LastElapsedDisplay, partialOutput, partialVariables)`. The DTO has no such fields —
**[corroboration]** `RunProtocol.cs:43-56` declares `... Diagnostics, string Elapsed, string[]? PartialOutput,
VariableDto[]? PartialVariables`, whereas the success DTO (`RunProtocol.cs:29-42`) declares
`string Elapsed, DurationDto ElapsedTime, TimingDto[] Timings` and is populated at `Runner.cs:217-225`.
The data is **already present on the failure path**: `SuiteEngine.EvaluateAsync` sets `LastElapsed` in a
`finally` (`SuiteEngine.cs:248-253`) that runs before the exception reaches Runner's `catch`, and
`OperationTimings` (`SuiteEngine.cs:139` → `Interpreter.cs:147`) holds the timings of the statements
that completed (`Interpreter.cs:256`); a parse failure legitimately has none because
`ClearOperationTimings()` runs first (`SuiteEngine.cs:231`, `Interpreter.cs:150/222`).
**Minimal fix.** Add the two fields to `RunErrorDto` and pass `Duration(engine.LastElapsed)` plus the same
`OperationTimings` projection used at `Runner.cs:219-225` (~8 lines, additive; `RunErrorDto` is the only
other envelope).
**Test that fails.** `--eval "1 +;"` → assert `error.elapsedTime.value` is a number and `error.timings`
is an array (today both keys are absent — the source-gen options ignore nulls); and a mid-script runtime failure
(`x = 1; 1/0`) → assert `timings` has exactly one entry.
**Blast radius.** Additive to a versioned protocol; the `Cancelled` path shares `WriteError` and gains the same
fields. Adjacent gap worth one line: `RunErrorDto` also has no `revision` and no `result`, so a consumer
cannot tell what state a failed run committed.
**Open question.** Should the failure envelope carry `revision` for symmetry with `RunEnvelopeDto.Revision`
(`RunProtocol.cs:34`)?

### (d) `variables[]` carries only `{name, kind, display}`

**Symptom / evidence.** `pre2/a02-subs-root.out.txt`:
`"variables":[{"name":"_","kind":"Symbolic","display":"4"},{"name":"x","kind":"Symbolic","display":"x"}]` —
three keys, no structure.
**Root cause — record and field construction.** `Runner.cs:184-186`:

```csharp
185:                     .Select(v => new VariableDto(v.Name, v.Kind.ToString(), v.Display))
```

with `VariableDto(string Name, string Kind, string Display)` (`RunProtocol.cs:17` **[corroboration]**), fed
from `SuiteEngine.cs:330-331`:

```csharp
331:             variables[name] = new StateVariable(name, value.Kind, ValueFormatter.Format(value));
```

`StateVariable(string Name, ValueKind Kind, string Display)` (`Diagnostics.cs:36` **[corroboration]**) has
already discarded the `Value`: `Format` returns a string. The contrast inside the same envelope is the point —
the *result* gets a full structured projection (`Runner.cs:201-204` → `StructuredProjection.ToStructured`:
canonical form, exact rationals, shape, free symbols, domain) while *variables* get a display string.
**Minimal fix.** Append `StructuredValueDto? Structured` to `VariableDto` and populate it from the live
`_interpreter.Variables` (or carry the `Value` on `StateVariable`); the projection call already exists.
`--omit-variables` (`Runner.cs:84-88`) keeps the payload opt-out-able.
**Test that fails.** `--eval "x = 1/3"` → assert `variables[0].structured.canonical == "(rat 1 3)"` (today the
`structured` key does not exist).
**Blast radius.** `StateVariable` is shared with Studio; append-with-default keeps it source-compatible;
variable-bearing golden envelopes change.
**Open question.** `Display` is produced under the engine precision scope (`SuiteEngine.cs:328`), so the only
recoverable meaning of a variable is **precision-dependent**: is that a documented contract or an accident? (If
accidental, the structured field is also the correctness fix.)

### (e) The parse refusal for `1 +;` shares a code with the argument guards

**Symptom / evidence.** `pre/t2-parse-fail.out.txt`:
`"code":"InvalidOperation","category":"DomainError","message":"Unexpected token ';' at position 3: expected a number, string, identifier, '[', or '('."`
**Which code/category the parser produces.** The tokenizer accepts `1 +;`; `Parser.ParsePrimary` refuses at
`Parser.cs:567-569`:

```csharp
567:         throw new InvalidOperationException(
568:             $"Unexpected token '{Current.Text}' at position {Current.Position}: " +
569:             "expected a number, string, identifier, '[', or '('.");
```

`Runner.Classify` maps it at `Runner.cs:267`:
`InvalidOperationException => ("InvalidOperation", "DomainError", true)`.

**Yes — it shares a code with the argument guards**, and the guard family is itself inconsistent (all values from the
baseline probes):

| probe | code / category | throw site |
|---|---|---|
| `1 +;` (syntax) | `InvalidOperation` / `DomainError` | `Parser.cs:567` |
| `abs(1,2)`, `len(1,2)`, `det()` | `InvalidOperation` / `DomainError` | `Interpreter.cs:1231` `RequireArity` |
| `sin(1,2)` | `InvalidArgument` / `TypeMismatch` | `BuiltinArityException` (`ModusHost.cs:219`) |
| `mean(1)`, `max(1,2)`, `sum(x,5)` | **`InternalError` / `InternalInvariantFailure`** | cast leak at `Value.cs:212`, see below |
| `"abc` (unterminated string, also syntax) | `InvalidInput` / `ParseError` | `Tokenizer.cs:155` (`FormatException` → `Runner.cs:265`) |

So a **syntax** error is reported as `DomainError`, while a second syntax error gets `ParseError`; and the
reduce family is not guarded at all: `ReduceBuiltin` (`Interpreter.cs:1703-1715`) accepts a scalar and reaches
`ReduceAllOrEmpty` → `input.AsArrayValue()` (`Interpreter.cs:1720`) → `Value.AsArrayValue()` =
`(ArrayValue)_inner` (`Value.cs:212`) → `InvalidCastException`, which falls to Classify's default arm
(`Runner.cs:274`) and is reported as a non-recoverable internal invariant failure
(`"Specified cast is not valid."`).
**Minimal fix.** One typed `ParseException` thrown by the parser's refusal sites and one Classify arm
`=> ("ParseError", "ParseError", true)`; plus a kind check in `ReduceBuiltin`/`ReduceAllOrEmpty`
(`throw new InvalidOperationException($"sum() expects an array, but got '{args[0].Kind}'.")`). Parser
refusal sites to convert: `Parser.cs:37, 43, 72, 102, 154, 169, 205, 249, 567`.
**Test that fails.** `Assert.Equal("ParseError", code("1 +;"))` **and**
`Assert.NotEqual(code("1 +;"), code("len(1,2)"))` — today both are `InvalidOperation`; plus
`Assert.Equal("InvalidOperation", code("mean(1)"))` — today `InternalError`.
**Blast radius.** Changes the `code` of every syntax error (a protocol change: agents keyed on
`InvalidOperation` for syntax break) and of every reduce-family misuse; the message text and
`diagnostics[].position` stay as they are, so the regex-based position extraction (`SuiteEngine.cs:349`) is
unaffected.
**Open question (adjacent, cited):** diagnostic positions are computed against the *rewritten* source —
`Runner.cs:178` passes `ScriptSource.ToSemicolonStatements(source)` and `SuiteEngine.ComputeLineColumn`
(`SuiteEngine.cs:357-375`) counts newlines in that rewritten text, which has had depth-0 newlines replaced by
`;` (`ScriptSource.cs:78-89`). A multi-line script should therefore report line 1 for every error; that needs
its own probe.

---

## Appendix — probe artifacts cited

| artifact | what it establishes |
|---|---|
| `docs/goal-cycle-5/run-probes.ps1:14-15` | the ticket's `t06-depth-3000/2000` are 3000/2000 **nested parentheses** |
| `.../probes/pre/t06-depth-3000.out.txt`, `.../pre/_summary.txt` | 3000 nested parens: **exit=0**, normal envelope |
| `docs/goal-cycle-5/run-probes2.ps1:31-35, 45-55` | the five depth shapes and their 1000/2000/3000 matrix |
| `.../probes/pre2/_summary.txt` | paren survives 3000; unary/call/addleft/array die at 3000 |
| `.../probes/pre2/d-unary-3000.out.txt` | the crash text is the harness's stderr; stdout empty |
| `.../probes/pre/t1-print-cr.out.txt` | defect (a): `"output":["A\r","B"]` |
| `.../probes/pre2/a02-subs-root.out.txt`, `.../pre2/a04-inspect-neg100000.out.txt` | defect (b): `freeSymbols` vs `free_symbols` in one envelope |
| `.../probes/pre2/a21-free-symbols.out.txt` | the baseline free-symbol probe never exercised the path (`Undefined variable 'y'`) |
| `.../probes/pre/t2-parse-fail.out.txt` vs `.../pre2/d-paren-3000.out.txt` | defect (c): error envelope lacks `elapsedTime`/`timings`; success has both |
| `.../probes/pre2/a02-subs-root.out.txt` | defect (d): `variables[]` = `{name,kind,display}` |
| `.../probes/pre/t1-arity*.out.txt` | defect (e): three different codes for argument-guard failures + the `InvalidCastException` leak |
