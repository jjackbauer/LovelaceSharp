# O-B11 — the interpreter's evaluation-depth budget (cycle 6, round 7, implementation)

* Work tree: `C:\Users\ricar\dev\LovelaceSharp\.worktrees\c6-ob11` (git worktree at `1b70a32`)
* Changed files: `Lovelace.Abstractions/InputDepth.cs`, `Lovelace.Suite/Interpreter.cs` (2 files, +189/-40)
* Nothing committed, staged or pushed. The two contract tests were **read, never edited**:
  `Lovelace.Suite.Tests/RecursionDepthGuardTests.cs` (SHA256 `39517D4708BBF8808F1FE501D3E9C9FA4C86842D415C45080D1F5084E02F0762`) and
  `Lovelace.Run.Tests/RecursionDepthEnvelopeTests.cs` (SHA256 `8ADE7E6ED9D767D29AFB932A95B429F0078D0437525C4E303845C53886BA6F55`);
  both files' mtime is 2026-09-11 22:03:03, i.e. before this session.
* Every artifact named in §10 lives in the scratch worktree; this report is additionally placed at the
  path the round asked for, `C:\Users\ricar\dev\LovelaceSharp\docs\goal-cycle-6\round-7\ob11-implementation.md`.

---

## 1. The defect, reproduced in this round (before the change)

`rec.ls` = `func f(n) { if (n == 0) { return 0 }; return f(n - 1) }` + newline + `f(N)`, run as

```
dotnet Lovelace.Run/bin/Release/net10.0/Lovelace.Run.dll --file rec.ls --json --omit-functions --omit-variables
```

| N | exit | hex | stdout bytes | stderr bytes | stdout |
|---|---|---|---|---|---|
| 432 | 0 | 0x00000000 | 537 | 0 | `"ok":true … "kind":"Natural","display":"0"` |
| **436** | **-1073741571** | **0xC00000FD** | **0** | **1 997 859** | *(empty — nothing for an agent to read)* |
| **448** | **-1073741571** | **0xC00000FD** | **0** | **2 016 788** | *(empty)* |

stderr begins `Stack overflow.` followed by the interpreter's own frames
(`Lovelace.Suite.Interpreter.ExecuteBlockAsync`, `…EvaluateAsync`, `…EvaluateVariable` at N=436/448).
A stack overflow is not catchable, so no diagnostic, envelope or exit code could be reported — the
process simply died.

## 2. The contract tests and the failing-first run

Both files exist on the pre-fix tree and fail there. Pre-fix build (`dotnet build Lovelace.Run -c Release`:
0 warnings, 0 errors), then

```
dotnet test .\Lovelace.Suite.Tests\Lovelace.Suite.Tests.csproj -c Release --nologo --filter 'FullyQualifiedName~RecursionDepthGuardTests'
```

```
      Failed Lovelace.Suite.Tests.RecursionDepthGuardTests.ADeepExpressionInsideTheRecursiveBody_IsRefused [138 ms]
      Failed Lovelace.Suite.Tests.RecursionDepthGuardTests.RecursionOneLevelPastTheDeepestAcceptedDepth_IsRefused [1 ms]
      Failed Lovelace.Suite.Tests.RecursionDepthGuardTests.RecursionFarPastTheBudget_IsRefused_AndDoesNotReturnAValue [2 ms]
    Failed!  - Failed:     3, Passed:     3, Skipped:     0, Total:     6, Duration: 640 ms - Lovelace.Suite.Tests.dll (net10.0)
```

The three failures are exactly the refusal assertions, each on the value the interpreter returned
instead of a typed refusal:

```
    Assert.NotNull() Failure: Value is null
    Assert.NotNull() Failure: Value is null
    Assert.NotNull() Failure: Value is null
```

```
dotnet test .\Lovelace.Run.Tests\Lovelace.Run.Tests.csproj -c Release --nologo --filter 'FullyQualifiedName~RecursionDepthEnvelopeTests'
```

```
    depth 448: the runner must put an envelope on stdout, but exited -1073741571 with 0 bytes. stderr: Stack overflow.
    depth 432 is past the budget and must be refused. stdout={"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":true,"revision":82,"result":{"kind":"Natural",
    depth 86 is past the budget and must be refused. stdout={"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":true,"revision":82,"result":{"kind":"Natural","
    depth 2000: the runner must put an envelope on stdout, but exited -1073741571 with 0 bytes. stderr: Stack overflow.
    depth 200 is past the budget and must be refused. stdout={"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":true,"revision":82,"result":{"kind":"Natural",
    depth 100000: the runner must put an envelope on stdout, but exited -1073741571 with 0 bytes. stderr: Stack overflow.
    depth 433: the runner must put an envelope on stdout, but exited -1073741571 with 0 bytes. stderr: Stack overflow.
    the runner must put an envelope on stdout; it wrote 0 bytes and exited -1073741571. stderr: Stack overflow.
    the runner must put an envelope on stdout; it wrote 0 bytes and exited -1073741571. stderr: Stack overflow.
    Expected: 1
    Actual:   0
    Failed!  - Failed:    10, Passed:     5, Skipped:     0, Total:    15, Duration: 33 s - Lovelace.Run.Tests.dll (net10.0)
```

The five that passed pre-fix are the sweep depths 0, 1, 8, 64 and 85 — pre-fix *everything* answered,
including 85, and the process died exactly where the budget should have refused.

## 3. What was implemented

One live counter over the interpreter's evaluation walk, charged at the walk's own choke points and
checked on every charge. The unit table (all in `Lovelace.Suite/Interpreter.cs`):

| Walk step | Units | Where | Why |
|---|---|---|---|
| an expression node that **descends** (operator, call, index, member, list, range, interpolation, assignment, postfix) | 1 | `Interpreter.cs:375` (`EvaluateAsync(Expr, Scope)`) | every per-node descent re-enters this one method, so one counter covers a deep chain, a deep **body** and unbounded recursion alike |
| a **leaf** (literal, variable, string) | 0 | same line | a leaf descends nowhere: the budget measures the depth of the descent, not the number of nodes (a wide list of literals is not deep) |
| a **statement** executed (if/while/for/return/expression/function) | 1 | `Interpreter.cs:1064` (`ExecuteAsync(Statement, Scope)`) | a nest of `if`s — with or without braces — is depth the walk pays at **every** call level |
| a `BlockStatement` | 0 | same line | grouping, not a step of its own: the statements inside it are charged individually, and a nest of empty braces is bounded by the parser's own descent budget (256) |
| a **user-function call level** | 4 | `Interpreter.cs:805` (`CallUserFunctionAsync`) | its frame, its scope, and the entry into its body — the four units the test's own arithmetic names |
| a top-level statement's **entry into user code** | −6 credit | `Interpreter.cs:66` (`EntryCredit`), restored at `Interpreter.cs:293` and `:315` | the statement itself (1), the script's own outermost call expression (1) and the frame it pushes (4) are the **base** of the measurement: the budget bounds the depth the script's *function bodies* add below their entry |

The audit's shape therefore costs **six units per call level** (the return statement 1 + the call
expression 1 + the call frame 4), and `f(85)` lands on exactly 512 units.

The refusal crosses the wire as the cycle-5 typed stop, with the message worded for this budget:
`Lovelace.Abstractions/InputDepth.cs:120` gained an optional `measure` word (`"nesting"` by default,
`"evaluation"` here), so the message reads

```
evaluation depth 513 exceeds the maximum supported evaluation depth of 512. Rewrite the input with
less nesting (the engine refuses it before a recursive walk can exhaust the process stack).
```

and the runner's existing arm (`Lovelace.Run/Runner.cs:268`) turns `InputDepthExceededException` into
`code=DepthExceeded, category=BudgetExceeded, recoverable=true`. Every existing call site of the
exception keeps its byte-identical "nesting depth" message.

Bookkeeping invariants: every charge is undone by its own `finally` (an error or a cancellation inside
a node cannot leak depth, and the engine reuses one `Interpreter` for a whole session); when the budget
itself throws, the counter is restored before the throw (`Interpreter.cs:99-105`); the credit is
restored per top-level statement. The credit is a *credit* rather than a per-call exemption so that a
chain of calls nested in **arguments** cannot collect the exemption once per call.

### 3.1 Behaviour change (intended, wider than the audit shape)

The budget is one number over the walk, so the reachable recursion depth depends on what each level
walks: a body that walks one extra expression node per level pays 7 units per level instead of 6 and is
refused earlier (measured in §5.2: `f(n) = n + f(n-1)` accepts 72, refuses 73). Pre-fix those shapes
answered up to ~432. That is the contract the two new tests pin: a deep recursion is a typed refusal,
never a process death.

## 4. The diff (verbatim, `git diff`)

```diff
diff --git a/Lovelace.Abstractions/InputDepth.cs b/Lovelace.Abstractions/InputDepth.cs
index f87b9b2..fcab7e5 100644
--- a/Lovelace.Abstractions/InputDepth.cs
+++ b/Lovelace.Abstractions/InputDepth.cs
@@ -36,6 +36,38 @@ public static class InputDepth
     /// reads it, and only a flat left-associative chain can get there without nesting.</summary>
     public const int MaxParsedTreeDepth = 512;
 
+    /// <summary>The greatest depth the interpreter's own EVALUATION walk may reach. It bounds the
+    /// recursion the budgets above cannot see: a user function that calls itself is deep in neither
+    /// the source (every statement of the body is flat) nor the parsed tree (the body is a handful of
+    /// levels) and returns a scalar — the depth that grows is the INTERPRETER's own native recursion,
+    /// which is uncatchable when it exhausts the process stack.
+    /// <para>
+    /// The unit is one step of that walk (enforced in <c>Interpreter.EnterEvaluation</c>): one unit
+    /// per expression node that descends (a leaf descends nowhere), one per statement executed (a
+    /// BlockStatement is grouping and is covered by the statements inside it), and four per
+    /// user-function call level (its frame, its scope, and the entry into its body). A script's own
+    /// outermost call — its entry into user code — starts each top-level statement with a six-unit
+    /// credit, so the depth that counts is the depth the script's function bodies add below their
+    /// entry.
+    /// </para>
+    /// <para>
+    /// The audit's shape — <c>func f(n) { if (n == 0) { return 0 }; return f(n - 1) }</c> plus
+    /// <c>f(N)</c> — therefore costs six units per call level. Measured on the shipped runner
+    /// (Windows, Release, 1 MB stack): the deepest accepted call is <c>f(85)</c> at exactly 512
+    /// units and <c>f(86)</c> is refused as <c>DepthExceeded/BudgetExceeded</c>; before this budget
+    /// the same script answered through <c>f(432)</c> and terminated the process at <c>f(436)</c>
+    /// with 0xC00000FD, zero bytes on stdout and ~2 MB of "Stack overflow." on stderr.
+    /// </para>
+    /// <para>
+    /// 512 units is a factor-5 margin below that death (f(436) is roughly 2 600 native frames). It is
+    /// one number over the whole walk rather than a call-level cap because a deep expression inside a
+    /// recursive body is walked at EVERY level: 120 nested <c>if</c>s in the body killed the process
+    /// at call depth 5 (measured) and are now refused, while the same 120 nested <c>if</c>s at the top
+    /// level still answer.
+    /// </para>
+    /// </summary>
+    public const int MaxEvaluationDepth = 512;
+
     /// <summary>The greatest number of levels a run-time VALUE may be deep when it is rendered.
     /// The measure counts what the renderers recurse over: a rank-R array costs R levels (the
     /// display walk descends one level per DIMENSION, not per element), a record costs one level,
@@ -79,8 +111,14 @@ public sealed class InputDepthExceededException : Exception
     /// <summary>The budget it exceeded.</summary>
     public int Limit { get; }
 
-    public InputDepthExceededException(string stage, int depth, int limit)
-        : base($"{stage} depth {depth} exceeds the maximum supported nesting depth of {limit}. "
+    /// <param name="stage">What was being walked (e.g. "expression", "evaluation").</param>
+    /// <param name="depth">The measured depth that was refused.</param>
+    /// <param name="limit">The budget it exceeded.</param>
+    /// <param name="measure">What the limit measures: "nesting" for the shape budgets, or
+    /// "evaluation" for <see cref="InputDepth.MaxEvaluationDepth"/>. Only the word changes; the
+    /// message shape is the same for every budget.</param>
+    public InputDepthExceededException(string stage, int depth, int limit, string measure = "nesting")
+        : base($"{stage} depth {depth} exceeds the maximum supported {measure} depth of {limit}. "
                + "Rewrite the input with less nesting (the engine refuses it before a recursive walk "
                + "can exhaust the process stack).")
     {
diff --git a/Lovelace.Suite/Interpreter.cs b/Lovelace.Suite/Interpreter.cs
index dda0944..8f138a8 100644
--- a/Lovelace.Suite/Interpreter.cs
+++ b/Lovelace.Suite/Interpreter.cs
@@ -48,6 +48,74 @@ public sealed class Interpreter
     /// </summary>
     private int _relationPreservingDepth;
 
+    // -----------------------------------------------------------------
+    // Evaluation budget (InputDepth.MaxEvaluationDepth)
+    // -----------------------------------------------------------------
+
+    /// <summary>Budget units one user-function call level costs: its frame, its scope, and the
+    /// entry into its body's statement list.</summary>
+    private const int CallLevelUnits = 4;
+
+    /// <summary>Budget units a top-level statement starts with: the statement itself (one unit), the
+    /// script's own outermost call expression (one unit) and the frame it pushes
+    /// (<see cref="CallLevelUnits"/>). That call is the script's entry into user code — the BASE of
+    /// the measurement — so the budget bounds the depth the script's function bodies add below it,
+    /// and every script has exactly one such entry. It is a CREDIT consumed by the statement's first
+    /// charges rather than a per-call exemption, so a chain of calls nested in ARGUMENTS cannot
+    /// collect the exemption once per call.</summary>
+    private const int EntryCredit = CallLevelUnits + 2;
+
+    /// <summary>The interpreter's LIVE evaluation depth in budget units: one per expression node
+    /// that descends (a leaf costs nothing), one per statement executed (a BlockStatement is
+    /// grouping and costs nothing), and <see cref="CallLevelUnits"/> per user-function call level.
+    /// Every increment is undone by its own <c>finally</c>, so the counter is back where it started
+    /// after a statement, an error, or a cancellation — the engine reuses one
+    /// <see cref="Interpreter"/> for a whole session (leaked depth would poison every later
+    /// evaluation).</summary>
+    private int _evaluationDepth;
+
+    /// <summary>The unspent part of the current statement's <see cref="EntryCredit"/>.</summary>
+    private int _entryCredit = EntryCredit;
+
+    /// <summary>How many user-function frames are executing right now.</summary>
+    private int _userFrameDepth;
+
+    /// <summary>Charges <paramref name="units"/> of the evaluation budget and returns the part that
+    /// actually has to be given back (the <see cref="EntryCredit"/> absorbs the first units of a
+    /// top-level statement). Throws the cycle-5 refusal when the depth passes
+    /// <see cref="InputDepth.MaxEvaluationDepth"/>.</summary>
+    private int EnterEvaluation(int units)
+    {
+        if (_entryCredit > 0)
+        {
+            int free = Math.Min(_entryCredit, units);
+            _entryCredit -= free;
+            units -= free;
+            if (units == 0)
+                return 0;
+        }
+
+        _evaluationDepth += units;
+        if (_evaluationDepth > InputDepth.MaxEvaluationDepth)
+        {
+            int depth = _evaluationDepth;
+            _evaluationDepth -= units;   // leave the counter exactly as it was found
+            throw new InputDepthExceededException(
+                "evaluation", depth, InputDepth.MaxEvaluationDepth, "evaluation");
+        }
+        return units;
+    }
+
+    private void ExitEvaluation(int units) => _evaluationDepth -= units;
+
+    /// <summary>Starts a fresh evaluation budget for one top-level statement: the statement is a new
+    /// entry into user code, so its <see cref="EntryCredit"/> is restored. The depth itself must
+    /// already be back to zero (every charge has a matching <c>finally</c>).</summary>
+    private void ResetEvaluationBudget()
+    {
+        _entryCredit = EntryCredit;
+    }
+
     // -----------------------------------------------------------------
     // Host-configurable settings
     // -----------------------------------------------------------------
@@ -222,6 +290,7 @@ public sealed class Interpreter
     public async Task<Value> EvaluateAsync(Expr expr)
     {
         using var scope = Rl.WithPrecision(ComputationDecimalPlaces, DisplayDecimalPlaces);
+        ResetEvaluationBudget();
         return await EvaluateAsync(expr, _global);
     }
 
@@ -241,6 +310,9 @@ public sealed class Interpreter
                 // statement granularity: a cancelled script stops between statements, so the
                 // result it already produced (variables, captured output) stays readable
                 Lovelace.Abstractions.Cancellation.ThrowIfCancellationRequested();
+                // Every top-level statement is its own entry into user code: it gets a fresh
+                // evaluation budget (and the depth itself is balanced by its own finallys).
+                ResetEvaluationBudget();
                 var statement = program.Statements[i];
                 int position = i < program.StatementPositions.Count ? program.StatementPositions[i] : 0;
 
@@ -291,22 +363,39 @@ public sealed class Interpreter
 
     private async Task<Value> EvaluateAsync(Expr expr, Scope scope)
     {
-        switch (expr)
-        {
-            case LiteralExpr lit: return EvaluateLiteral(lit);
-            case VariableExpr var: return EvaluateVariable(var, scope);
-            case AssignExpr assign: return await EvaluateAssignAsync(assign, scope);
-            case BinaryExpr bin: return await EvaluateBinaryAsync(bin, scope);
-            case UnaryExpr unary: return await EvaluateUnaryAsync(unary, scope);
-            case PostfixExpr postfix: return await EvaluatePostfixAsync(postfix, scope);
-            case CallExpr call: return await EvaluateCallAsync(call, scope);
-            case StringExpr str: return new Value(str.Value);
-            case RangeExpr range: return await EvaluateRangeAsync(range, scope);
-            case IndexExpr idx: return await EvaluateIndexAsync(idx, scope);
-            case MemberExpr member: return await EvaluateMemberAsync(member, scope);
-            case ListExpr list: return await EvaluateListAsync(list, scope);
-            case InterpolatedStringExpr interp: return await EvaluateInterpolatedAsync(interp, scope);
-            default: throw new NotImplementedException($"Unsupported expression type: {expr.GetType().Name}");
+        // The evaluation budget is charged HERE: every per-node descent of the walk re-enters this
+        // one method, so a single counter covers a deep expression chain, a deep statement nest and
+        // unbounded user recursion alike. The charge is undone by the finally, so an error or a
+        // cancellation inside the node cannot leave depth behind.
+        //
+        // A LEAF -- a literal, a variable, a string -- descends nowhere, so it costs no unit: the
+        // budget measures the depth of the DESCENT, not the number of nodes (otherwise a wide list
+        // of literals would be charged for width).
+        bool leaf = expr is LiteralExpr or VariableExpr or StringExpr;
+        int charged = leaf ? 0 : EnterEvaluation(1);
+        try
+        {
+            switch (expr)
+            {
+                case LiteralExpr lit: return EvaluateLiteral(lit);
+                case VariableExpr var: return EvaluateVariable(var, scope);
+                case AssignExpr assign: return await EvaluateAssignAsync(assign, scope);
+                case BinaryExpr bin: return await EvaluateBinaryAsync(bin, scope);
+                case UnaryExpr unary: return await EvaluateUnaryAsync(unary, scope);
+                case PostfixExpr postfix: return await EvaluatePostfixAsync(postfix, scope);
+                case CallExpr call: return await EvaluateCallAsync(call, scope);
+                case StringExpr str: return new Value(str.Value);
+                case RangeExpr range: return await EvaluateRangeAsync(range, scope);
+                case IndexExpr idx: return await EvaluateIndexAsync(idx, scope);
+                case MemberExpr member: return await EvaluateMemberAsync(member, scope);
+                case ListExpr list: return await EvaluateListAsync(list, scope);
+                case InterpolatedStringExpr interp: return await EvaluateInterpolatedAsync(interp, scope);
+                default: throw new NotImplementedException($"Unsupported expression type: {expr.GetType().Name}");
+            }
+        }
+        finally
+        {
+            ExitEvaluation(charged);
         }
     }
 
@@ -711,6 +800,10 @@ public sealed class Interpreter
         for (int i = 0; i < fn.Parameters.Count; i++)
             frame.Define(fn.Parameters[i], args[i]);
 
+        // One call LEVEL of the evaluation budget: the frame, its scope, and the entry into the
+        // body. This is what turns unbounded user recursion into a typed refusal.
+        int charged = EnterEvaluation(CallLevelUnits);
+        _userFrameDepth++;
         try
         {
             return await ExecuteStatementListAsync(fn.Body, frame);
@@ -719,6 +812,11 @@ public sealed class Interpreter
         {
             return rs.Value;
         }
+        finally
+        {
+            _userFrameDepth--;
+            ExitEvaluation(charged);
+        }
     }
 
     // -----------------------------------------------------------------
@@ -958,38 +1056,51 @@ public sealed class Interpreter
 
     private async Task<Value> ExecuteAsync(Statement stmt, Scope scope)
     {
-        switch (stmt)
+        // Every statement the walk executes is one unit of evaluation depth for as long as it runs:
+        // a nest of `if`s, a deep expression and a recursive call chain all pay for their depth here,
+        // whether or not the source spells the blocks with braces. A BlockStatement is GROUPING
+        // rather than a step of its own -- the statements inside it are charged individually -- so it
+        // costs nothing (a nest of empty braces is bounded by the parser's own descent budget).
+        int charged = stmt is BlockStatement ? 0 : EnterEvaluation(1);
+        try
         {
-            case ExpressionStatement es:
-                return await EvaluateAsync(es.Expression, scope);
+            switch (stmt)
+            {
+                case ExpressionStatement es:
+                    return await EvaluateAsync(es.Expression, scope);
 
-            case BlockStatement block:
-                return await ExecuteBlockAsync(block, scope);
+                case BlockStatement block:
+                    return await ExecuteBlockAsync(block, scope);
 
-            case IfStatement ifStmt:
-                return await ExecuteIfAsync(ifStmt, scope);
+                case IfStatement ifStmt:
+                    return await ExecuteIfAsync(ifStmt, scope);
 
-            case WhileStatement whileStmt:
-                return await ExecuteWhileAsync(whileStmt, scope);
+                case WhileStatement whileStmt:
+                    return await ExecuteWhileAsync(whileStmt, scope);
 
-            case ForStatement forStmt:
-                return await ExecuteForAsync(forStmt, scope);
+                case ForStatement forStmt:
+                    return await ExecuteForAsync(forStmt, scope);
 
-            case ReturnStatement returnStmt:
-                return await ExecuteReturnAsync(returnStmt, scope);
+                case ReturnStatement returnStmt:
+                    return await ExecuteReturnAsync(returnStmt, scope);
 
-            case BreakStatement:
-                throw new BreakSignal();
+                case BreakStatement:
+                    throw new BreakSignal();
 
-            case ContinueStatement:
-                throw new ContinueSignal();
+                case ContinueStatement:
+                    throw new ContinueSignal();
 
-            case FunctionStatement funcStmt:
-                DefineFunction(funcStmt.Definition);
-                return Value.Void;
+                case FunctionStatement funcStmt:
+                    DefineFunction(funcStmt.Definition);
+                    return Value.Void;
 
-            default:
-                throw new NotImplementedException($"Unsupported statement type: {stmt.GetType().Name}");
+                default:
+                    throw new NotImplementedException($"Unsupported statement type: {stmt.GetType().Name}");
+            }
+        }
+        finally
+        {
+            ExitEvaluation(charged);
         }
     }
 
```

## 5. Measured boundary (real runner, Release build, Windows x64, 1 MB stack)

### 5.1 The audit shape — deepest accepted / shallowest refused

| shape | deepest accepted | shallowest refused | refusal |
|---|---|---|---|
| `f(n) = f(n-1)` (the audit shape) | `f(85)` — exit 0, `display=0` | `f(86)` — exit 1 | `evaluation depth 513 exceeds the maximum supported evaluation depth of 512` |

Peak arithmetic: 85 call levels × 6 units + the innermost frame's two expression levels = 512 units, so
the deepest accepted call sits **exactly on** the budget; `f(86)` crosses it at the first charge past
512 (the reported depth is the first crossing, not the walk's ultimate depth — the walk aborts there).

### 5.2 Per-shape boundaries (same runner)

```
plain  f(n) returns f(n - 1)    f(85) -> exit=0 ok=True display=0   |   f(86) -> exit=1 ok=False evaluation depth 513 exceeds the maximum suppo...
sum    f(n) returns n + f(n - 1) f(72) -> exit=0 ok=True display=2628   |   f(73) -> exit=1 ok=False evaluation depth 513 exceeds the maximum suppo...
fact   f(n) returns n * f(n - 1) f(72) -> exit=0 ok=True display=61234458376886086861524070385274672740778091784697328983823014963978384987221689274204160000000000000000   |   f(73) -> exit=1 ok=False evaluation depth 513 exceeds the maximum suppo...
fib    f(n-1) + f(n-2)        deepest REACHABLE probe f(28) answers; its depth boundary is not sweepable (exponential call count)
```

### 5.3 The other shapes the contract pins, and the adversarial ones

```
rec-0                      exit=0            ok=True   stdoutBytes=535   stderrBytes=0    code=               category=               display=0
rec-1                      exit=0            ok=True   stdoutBytes=536   stderrBytes=0    code=               category=               display=0
rec-8                      exit=0            ok=True   stdoutBytes=536   stderrBytes=0    code=               category=               display=0
rec-64                     exit=0            ok=True   stdoutBytes=534   stderrBytes=0    code=               category=               display=0
rec-83                     exit=0            ok=True   stdoutBytes=534   stderrBytes=0    code=               category=               display=0
rec-84                     exit=0            ok=True   stdoutBytes=533   stderrBytes=0    code=               category=               display=0
rec-85                     exit=0            ok=True   stdoutBytes=533   stderrBytes=0    code=               category=               display=0
rec-86                     exit=1            ok=False  stdoutBytes=885   stderrBytes=0    code=DepthExceeded  category=BudgetExceeded display=
    message: evaluation depth 513 exceeds the maximum supported evaluation depth of 512. Rewrite the input with less nesting (the engine refuses it before a recurs...
rec-87                     exit=1            ok=False  stdoutBytes=883   stderrBytes=0    code=DepthExceeded  category=BudgetExceeded display=
    message: evaluation depth 513 exceeds the maximum supported evaluation depth of 512. Rewrite the input with less nesting (the engine refuses it before a recurs...
rec-200                    exit=1            ok=False  stdoutBytes=885   stderrBytes=0    code=DepthExceeded  category=BudgetExceeded display=
    message: evaluation depth 513 exceeds the maximum supported evaluation depth of 512. Rewrite the input with less nesting (the engine refuses it before a recurs...
rec-432                    exit=1            ok=False  stdoutBytes=885   stderrBytes=0    code=DepthExceeded  category=BudgetExceeded display=
    message: evaluation depth 513 exceeds the maximum supported evaluation depth of 512. Rewrite the input with less nesting (the engine refuses it before a recurs...
rec-433                    exit=1            ok=False  stdoutBytes=884   stderrBytes=0    code=DepthExceeded  category=BudgetExceeded display=
    message: evaluation depth 513 exceeds the maximum supported evaluation depth of 512. Rewrite the input with less nesting (the engine refuses it before a recurs...
rec-448                    exit=1            ok=False  stdoutBytes=883   stderrBytes=0    code=DepthExceeded  category=BudgetExceeded display=
    message: evaluation depth 513 exceeds the maximum supported evaluation depth of 512. Rewrite the input with less nesting (the engine refuses it before a recurs...
rec-2000                   exit=1            ok=False  stdoutBytes=884   stderrBytes=0    code=DepthExceeded  category=BudgetExceeded display=
    message: evaluation depth 513 exceeds the maximum supported evaluation depth of 512. Rewrite the input with less nesting (the engine refuses it before a recurs...
rec-100000                 exit=1            ok=False  stdoutBytes=885   stderrBytes=0    code=DepthExceeded  category=BudgetExceeded display=
    message: evaluation depth 513 exceeds the maximum supported evaluation depth of 512. Rewrite the input with less nesting (the engine refuses it before a recurs...
deep-10x100                exit=1            ok=False  stdoutBytes=886   stderrBytes=0    code=DepthExceeded  category=BudgetExceeded display=
    message: evaluation depth 513 exceeds the maximum supported evaluation depth of 512. Rewrite the input with less nesting (the engine refuses it before a recurs...
nested-5x120               exit=1            ok=False  stdoutBytes=890   stderrBytes=0    code=DepthExceeded  category=BudgetExceeded display=
    message: evaluation depth 513 exceeds the maximum supported evaluation depth of 512. Rewrite the input with less nesting (the engine refuses it before a recurs...
flat-501                   exit=0            ok=True   stdoutBytes=445   stderrBytes=0    code=               category=               display=501
adv-chain-f85              exit=1            ok=False  stdoutBytes=885   stderrBytes=0    code=DepthExceeded  category=BudgetExceeded display=
    message: evaluation depth 513 exceeds the maximum supported evaluation depth of 512. Rewrite the input with less nesting (the engine refuses it before a recurs...

braceless-85x100         exit=1            ok=False  stdoutBytes=887   stderrBytes=0      code=DepthExceeded  display=
    message: evaluation depth 513 exceeds the maximum supported evaluation depth of 512. Rewrite the in...
braceless-85x250         exit=1            ok=False  stdoutBytes=886   stderrBytes=0      code=DepthExceeded  display=
    message: evaluation depth 513 exceeds the maximum supported evaluation depth of 512. Rewrite the in...
braceless-5x100          exit=1            ok=False  stdoutBytes=887   stderrBytes=0      code=DepthExceeded  display=
    message: evaluation depth 513 exceeds the maximum supported evaluation depth of 512. Rewrite the in...
braceless-5x250          exit=1            ok=False  stdoutBytes=887   stderrBytes=0      code=DepthExceeded  display=
    message: evaluation depth 513 exceeds the maximum supported evaluation depth of 512. Rewrite the in...
blocknest-85x200         exit=1            ok=False  stdoutBytes=492   stderrBytes=0      code=InvalidOperation display=
    message: Expected ';' or '}' but found 'return' at position 838.
blocknest-85x250         exit=1            ok=False  stdoutBytes=496   stderrBytes=0      code=InvalidOperation display=
    message: Expected ';' or '}' but found 'return' at position 1038.
argnest-100              exit=0            ok=True   stdoutBytes=536   stderrBytes=0      code=               display=1
argnest-300              exit=1            ok=False  stdoutBytes=708   stderrBytes=0      code=DepthExceeded  display=
    message: expression nesting depth 257 exceeds the maximum supported nesting depth of 256. Rewrite t...
argnest-400              exit=1            ok=False  stdoutBytes=706   stderrBytes=0      code=DepthExceeded  display=
    message: expression nesting depth 257 exceeds the maximum supported nesting depth of 256. Rewrite t...

blocknest-85x100           exit=0            ok=True   stdoutBytes=535   stderrBytes=0      code=               display=0
blocknest-85x250           exit=0            ok=True   stdoutBytes=537   stderrBytes=0      code=               display=0
blockwrap-85x100           exit=1            ok=False  stdoutBytes=480   stderrBytes=0      code=InvalidOperation display=
    message: Expected ';' or end of input but found 'f' at position 457.
blockwrap-85x250           exit=1            ok=False  stdoutBytes=484   stderrBytes=0      code=InvalidOperation display=
    message: Expected ';' or end of input but found 'f' at position 1057.
ifblock-85x100             exit=1            ok=False  stdoutBytes=890   stderrBytes=0      code=DepthExceeded  display=
    message: evaluation depth 513 exceeds the maximum supported evaluation depth of 512. Rewrite the in...
ifblock-85x250             exit=1            ok=False  stdoutBytes=706   stderrBytes=0      code=DepthExceeded  display=
    message: statement nesting depth 257 exceeds the maximum supported nesting depth of 256. Rewrite th...
```

The brace-less `if` nest is why statements are charged at all: on an intermediate revision of this
change that charged only blocks, 85 call levels × 100 brace-less `if`s still died with 0xC00000FD and
0 bytes on stdout. After the statement charge, every brace-less nest probed (100 and 250 levels, at 85
and at 5 call levels) is refused at 513; the braced 5 × 120 shape is refused too, and a 250-deep
`if`-block nest is caught earlier by the parser's 256 statement budget.

### 5.4 Why 512 is safe on the platform's default stack

* Pre-fix the same runner **died between f(432) and f(436)**: `f(436)` is ~437 nested calls ≈ 2 600
  native frames on the 1 MB stack this engine ships against (cycle 5's frame inventory measures ~6
  native frames per user call level: `CallUserFunctionAsync` → `ExecuteStatementListAsync` →
  `ExecuteAsync` → `ExecuteReturnAsync` → `EvaluateAsync` → `EvaluateCallAsync`).
* The budget now refuses at 85 call levels for the audit shape — a **factor ≈ 5.1 below the measured
  death** — and at 72–73 for the heavier bodies, i.e. never more than ~20 % of the stack the process
  actually has.
* The counter is *absolute*: the walk that reaches the recursion is charged too, so a 500-term chain
  that stays live under the deepest accepted recursion is refused (probe `adv-chain-f85`, refused at
  513), and argument-nested call chains are charged per frame after the one-shot credit.
* The other walks the interpreter reaches are bounded by their own cycle-5 budgets
  (`MaxParsedTreeDepth` 512 for the parsed tree, `MaxValueDepth` 1024 for rendered values), and the
  measured death of the plain *expression* walk is 1280–1536 levels — 2.5–3× this budget.

## 6. The passing runs

```
dotnet test .\Lovelace.Suite.Tests\Lovelace.Suite.Tests.csproj -c Release --nologo --filter 'FullyQualifiedName~RecursionDepthGuardTests'
    Passed!  - Failed: 0, Passed: 6, Skipped: 0, Total: 6, Duration: 731 ms
dotnet test .\Lovelace.Run.Tests\Lovelace.Run.Tests.csproj -c Release --nologo --filter 'FullyQualifiedName~RecursionDepthEnvelopeTests'
    Passed!  - Failed: 0, Passed: 15, Skipped: 0, Total: 15, Duration: 2 s
```

(The wire file took 33 s pre-fix — it was spawning children that died and dumped ~2 MB of stack trace
each; it now takes 2 s.)

Whole solution, scratch tree, oracle forced:

```
$env:PATH = 'C:\Users\ricar\dev\.lovelace-tools\python;' + $env:PATH
$env:LOVELACE_REQUIRE_SYMPY = '1'
dotnet test LovelaceSharp.slnx --configuration Release --nologo
```

| project | failed | passed | skipped | total |
|---|---|---|---|---|
| Lovelace.Representation.Tests.dll (net10.0) | 0 | 91 | 0 | 91 |
| Lovelace.Knowledge.Tests.dll (net10.0) | 0 | 28 | 0 | 28 |
| Lovelace.Abstractions.Tests.dll (net10.0) | 0 | 20 | 0 | 20 |
| Lovelace.Natural.Tests.dll (net10.0) | 0 | 195 | 0 | 195 |
| Lovelace.Array.Tests.dll (net10.0) | 0 | 19 | 0 | 19 |
| Lovelace.Integer.Tests.dll (net10.0) | 0 | 148 | 0 | 148 |
| precbench.Tests.dll (net10.0) | 0 | 13 | 0 | 13 |
| Lovelace.Complex.Tests.dll (net10.0) | 0 | 96 | 0 | 96 |
| Lovelace.Console.Tests.dll (net10.0) | 0 | 15 | 0 | 15 |
| Lovelace.Real.Tests.dll (net10.0) | 0 | 2489 | 0 | 2489 |
| Lovelace.Studio.Tests.dll (net10.0) | 0 | 22 | 0 | 22 |
| Lovelace.Dsp.Tests.dll (net10.0) | 0 | 61 | 0 | 61 |
| Lovelace.Suite.Tests.dll (net10.0) | 0 | 819 | 0 | 819 |
| Lovelace.Run.Tests.dll (net10.0) | 0 | 190 | 0 | 190 |
| Lovelace.Symbolics.Tests.dll (net10.0) | 0 | 1063 | 0 | 1063 |

**15 projects, 5 269 passed, 0 failed, 0 skipped.** The run was repeated after the last (comment-only)
edit; the second run is `post-fix/full-solution-final.log` with the same per-project totals (counts in
the table above are identical; only the durations differ). Oracle on PATH: Python 3.12.14, sympy 1.14.0,
mpmath 1.3.0; `Lovelace.Symbolics.Tests/SympyOracle.cs:27` reads `LOVELACE_REQUIRE_SYMPY` and inverts
its local skip, so with the variable set nothing is skipped and a missing oracle *fails* — Symbolics
reported 1 063 passed / 0 skipped.

## 7. Requirement: no existing test recurses deeper than the cap

No test was weakened, deleted, skipped or loosened. The two contract files are byte-identical to what
the previous round wrote (hashes in the header). The full-solution run is **green with 0 failures**, so
no existing test — in particular none of the 819 Suite tests, 190 Run tests or 1 063 Symbolics tests —
recurses or nests deeper than the budget admits. Nothing had to be reported as over-deep.

## 8. Performance of ordinary recursion below the cap

A/B on the same machine, alternating arms rep by rep, 15 reps per script; the runner's own
`elapsedTime`, normalised to ms (it switches unit between µs/ms/s). pre = a control build of `HEAD`
(`git archive HEAD` into %TEMP%, i.e. the pre-fix interpreter), post = the patched tree.

```
fib20    pre  min= 446.660 median= 496.200 | post min= 480.530 median= 502.720 | median delta=   1.31%
fib24    pre  min=2,420.000 median=2,500.000 | post min=2,390.000 median=2,540.000 | median delta=   1.60%
sum60    pre  min=  18.300 median=  19.290 | post min=  16.640 median=  19.550 | median delta=   1.35%
even40   pre  min=  16.030 median=  18.070 | post min=  16.720 median=  18.740 | median delta=   3.71%
loop20k  pre  min=  57.590 median=  61.400 | post min=  57.430 median=  63.230 | median delta=   2.98%
loop200k pre  min= 318.190 median= 328.840 | post min= 322.170 median= 333.240 | median delta=   1.34%
```

Noise-floor control — the **same** pre-fix binary in both arms, same protocol:

```
fib20    pre  min= 466.730 median= 495.060 | post min= 444.990 median= 492.090 | median delta=  -0.60%
fib24    pre  min=2,380.000 median=2,500.000 | post min=2,350.000 median=2,510.000 | median delta=   0.40%
sum60    pre  min=  16.670 median=  18.880 | post min=  16.300 median=  19.400 | median delta=   2.75%
even40   pre  min=  16.240 median=  18.490 | post min=  15.810 median=  18.280 | median delta=  -1.14%
loop20k  pre  min=  56.910 median=  62.370 | post min=  52.670 median=  60.910 | median delta=  -2.34%
loop200k pre  min= 318.790 median= 327.130 | post min= 311.170 median= 328.390 | median delta=   0.39%
```

The measured post-fix deltas (+1.3 % … +3.7 % on the medians) are the same size as the noise this
protocol produces between two runs of one identical binary (−2.3 % … +2.8 %), and the minimums overlap
in both directions. Conclusion: no slowdown beyond this machine's ±3 % A/B resolution is detectable for
recursion below the cap (fib(20), fib(24), sum(60), the even/odd pair) or for a 20 000/200 000-iteration
loop.

## 9. Could not verify

1. **Native-AOT runner.** Only the CoreCLR Release build was built and run this round. The budget is
   runtime-independent source, but the documented 1 MB stack of the shipped Native-AOT binary was not
   re-measured here; the safety-margin argument cites cycle 5's measurement.
2. **Native frame counts.** The stack margin uses the measured *call-level* death (`f(436)` pre-fix) and
   cycle 5's ~6-frames-per-level figure; no stack-depth instrumentation was run, so "512 units ≈ N
   native frames" is an estimate for the shapes measured.
3. **The `fib` shape's boundary.** `f(n) = f(n-1) + f(n-2)` has the same depth cost per level (~7
   units) but an exponential *call count*: the deepest reachable probe was `f(28)` (317 811 calls, 16 s,
   answers). Its refusal depth was not swept and is not claimed.
4. **The intermediate revision that motivated the statement charge.** The brace-less counter-example
   (85 call levels × 100 brace-less `if`s → 0xC00000FD, 0 bytes on stdout) was observed against an
   intermediate revision of this change in this session, but that revision is not preserved as an
   artifact; the *final* tree refuses every such shape (probe output in §5.3).
5. **Pure `BlockStatement` nesting is uncharged by design.** Bounded by the parser's 256-level descent
   and the 512 parsed-tree budget; measured: 250 nested empty braces at 85 call levels answer
   (`blocknest-85x250`). The exact remaining stack margin for that construct is not measured.
6. **Sub-3 % performance regressions.** The machine is not idle (O-B12: a dozen `dotnet` processes live
   during the probes), so a regression smaller than the ±3 % A/B noise floor cannot be excluded.
7. **Scripts outside this repository that recursed deeper than their shape's boundary now refuse**
   (e.g. the audit shape past 85, `n + f(n-1)` past 72). That is the intended contract, but it is a
   behaviour change for third-party scripts; no shipped test exercises it (§7).
8. **Other hosts' surfaces.** Only the runner's envelope was probed on the wire (`--json`). Studio /
   Console / REPL paths are covered only by their (green) test suites, not by a per-surface probe.

## 10. Artifacts

| file | what |
|---|---|
| `docs/goal-cycle-6/round-7/ob11-implementation.md` | this report |
| `docs/goal-cycle-6/round-7/post-fix/ob11.diff` | the change, verbatim |
| `docs/goal-cycle-6/round-7/pre-fix/suite-tests.log`, `pre-fix/run-tests.log` | the failing-first transcripts |
| `docs/goal-cycle-6/round-7/post-fix/suite-tests.log`, `post-fix/run-tests.log` | the passing transcripts |
| `docs/goal-cycle-6/round-7/post-fix/full-solution.log`, `full-solution-final.log` | the whole-solution runs (oracle forced; the second is the final revision) |
| `docs/goal-cycle-6/round-7/post-fix/depth-boundary.txt` | the audit-shape sweep + the pinned shapes + the live-chain probe |
| `docs/goal-cycle-6/round-7/post-fix/shape-boundaries.txt` | per-shape deepest-accepted / shallowest-refused |
| `docs/goal-cycle-6/round-7/post-fix/adversarial.txt`, `adversarial-blocks.txt` | brace-less / braced / block-nesting / argument-nesting probes |
| `docs/goal-cycle-6/round-7/post-fix/perf-ab.txt`, `perf-noise-control.txt` | the A/B table and its noise floor |
| `docs/goal-cycle-6/round-7/probes/*.ps1` | the probe scripts (provenance for every number above) |

---

*Every claim in this report is either a command with its observed output above, or a file:line in the
scratch tree at the revision in the header.*
