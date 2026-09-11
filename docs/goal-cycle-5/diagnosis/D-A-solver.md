# D-A — Solver diagnosis: false completeness (T0-1) and the solve_system internal-invariant failure (T0-5)

Round: goal-cycle-5 (diagnosis only). No source file was modified; no build, test, or git command was run
(the orchestrator holds the binary locks). Everything below is read from source or from probe envelopes
already captured on disk; each claim carries file:line and a verbatim quote. Line numbers are from the tree
as it stands at the time of this writing.

Confidence summary:

| Defect | Root cause located? | Confidence | Evidence class |
|---|---|---|---|
| T0-1 false completeness | Yes — `Solvers.SolveInverse`, radical (`PowerExpr`) branch | High (deductive chain closed by the observed root value 4 and its canonical form `(rat 4 1)`) | source + pre-captured runner envelope |
| T0-5 InternalInvariantFailure | Yes — unchecked payload cast in the `solve_system` builtin body | High for the throw site; medium for the exact CLR message wording (see §D2.9 item 1) | source + pre-captured runner envelope |

---

## 0. Method, evidence base, scope honesty

### 0.1 In-scope files read
`Lovelace.Symbolics/Solvers/Solve.cs` (all solver sections, lines 1–1216 read in the relevant parts),
`Lovelace.Symbolics/SymbolicsPlugin.cs` (registration, solve/solve_full/solve_system bodies, coercion
helpers), `Lovelace.Suite/Interpreter.cs` (call dispatch, comparison/equality, `sqrt`, reduce/plot
helpers), `Lovelace.Suite/RecordSchemas.cs`, `Lovelace.Suite/Value.cs`,
`Lovelace.Suite/StructuredProjection.cs` (grep only), `Lovelace.Suite/PayloadMap.cs`,
`Lovelace.Run/Runner.cs` (read in full), `Lovelace.Symbolics/Assumptions.cs` (grep only).

### 0.2 Out-of-scope files read (read-only) to trace the two dispatch chains
Defect 2 cannot be answered inside the declared scope alone: `Interpreter.cs:1021` is a *doc comment*
on `MatrixResultRecord`, not a solve_system dispatch. I therefore read, without editing:
`Lovelace.Suite/ModusHost.cs` (the Value↔payload bridge and arity check),
`Lovelace.Suite/NumericOps.cs` (how `x + y == 2` becomes a symbolic relation),
`Lovelace.Abstractions/Modus.cs` (payload contract, descriptor overloads),
`Lovelace.Abstractions/PayloadArray.cs` (one declaration line), and the pre-captured probe
envelopes under `docs/goal-cycle-5/probes/pre*/` and `docs/goal-cycle-4/`.
These citations are tagged **[out-of-scope read]** below so they can be audited separately.

### 0.3 What I did NOT do
No `dotnet`, no test run, no `out/aot/Lovelace.Run.exe`, no git write. No probe was re-executed;
the envelope quotes are the ones already on disk.

---

# Defect 1 (T0-1) — `solve_full(sqrt(x)+2 == 0, x)` claims `Solved/complete=true/Complete` with the non-root 4

## D1.1 Symptom

`solve_full(sqrt(x) + 2 == 0, x)` publishes `status = Solved`, `complete = true`,
`completeness = Complete`, `represented_count = 1`, `unrepresented_count = 0` and one
solution `value: 4`. Substituting the claimed root gives 4, not 0; SymPy reports no solutions.

Verbatim envelope (pre-captured, `docs/goal-cycle-5/probes/pre2/a01-solve-only.out.txt:1`):

`@
"result":{"kind":"Record","display":"SolveResult(status: Solved, variable: x, domain: complex,
complete: True, completeness: Complete, solutions: [Solution(value: 4, conditions: [],
multiplicity: 1, exactness: Exact)], families: [], common_conditions: [], represented_count: 1,
unrepresented_count: 0, unrepresented_reason: , diagnostics: [])"
`@

and the substitution witness (`docs/goal-cycle-5/probes/pre2/a02-subs-root.out.txt:1`, script
`subs(sqrt(x) + 2, x, 4)`) returns `"display":"4"`, `"canonical":"(rat 4 1)"`.

The contract this violates is stated in the source itself:

`@csharp
// Solve.cs:12-15
/// The solver's completeness claim. A <see cref="Solved"/> result is a claim that the represented
/// solution set is the COMPLETE solution set over the requested domain; a solver that can only
/// represent part of the set must say <see cref="Partial"/> (with the unrepresented count and a
/// reason) or <see cref="Unevaluated"/> — never <see cref="Solved"/>.
`@

`@csharp
// Solve.cs:88-89
/// A solver outcome. The public invariant: <see cref="Status"/> == <see cref="SolveStatus.Solved"/>
/// implies the represented set is complete over <see cref="Domain"/>.
`@

and on the wire (`docs/symbolics/dsh-protocol.md:158-164`):

`@
| `status` | `complete` | `completeness` |
| `Solved` | `true` | `Complete` |
`@

## D1.2 Repro command

`docs/goal-cycle-5/probes/pre/t01-solver-completeness.ls:1` (exact file contents):

`@
x = symbol("x"); solve_full(sqrt(x) + 2 == 0, x); subs(sqrt(x) + 2, x, 4)
`@

minimal form: `x = symbol("x"); solve_full(sqrt(x) + 2 == 0, x)` (run through
`out/aot/Lovelace.Run.exe --eval ... --omit-functions --omit-variables`).

## D1.3 Root cause — the radical inversion is accepted without substitution

**Decision point (the exact lines where the extraneous root is accepted and where `Solved` is minted):
`Solve.cs:667` and `Solve.cs:675-678`, inside `Solvers.SolveInverse`
(`Solve.cs:601-687`). The mathematically non-invertible step that creates the bogus candidate is
`Solve.cs:647`.**

Full trace, every hop cited:

**Step 1 — `sqrt(x)` is a `PowerExpr`, not a function call.** The core builtin:

`@csharp
// Interpreter.cs:1451-1456
Register("sqrt", ["x"], async args =>
{
    RequireArity("sqrt", args, 1);
    var arg = args[0];
    if (arg.Kind == ValueKind.Symbolic)
        return new Value(Lovelace.Symbolics.Exprs.Power(arg.AsSymbolic(), Lovelace.Symbolics.Exprs.Rational(1, 2)));
`@

So the residual `sqrt(x) + 2` is `Add(Power(x, 1/2), 2)`.

**Step 2 — SolveCore builds the residual and dispatches.** `f = sqrt(x) + 2` is not a polynomial
(`Solve.cs:202`), not rational (`Solve.cs:206-208`), and the linear-with-arbitrary-coefficients
test cannot produce a non-zero `x` coefficient here (`Solve.cs:251-263`, whose accepted branch requires
`!c1` to be a zero `RationalConstantExpr`). Dispatch reaches step 4:

`@csharp
// Solve.cs:265-268
        // 4. elementary compositions
        var elementary = SolveElementary(f, x, ctx, domain, depth);
        if (elementary is not null)
            return elementary;
`@

**Step 3 — SolveElementary isolates the radical and calls the inverse solver.**

`@csharp
// Solve.cs:578-596
        if (f is AddExpr add)
        {
            foreach (var term in add.Terms)
            {
                Expr rhs;
                if (term is RationalConstantExpr rc0)
                    rhs = Exprs.Rational(Rat.Negate(rc0.Value));   // h(u) + c = 0  ⇒  h(u) = -c
...
                var u = Exprs.Subtract(f, term);   // h(u-part)
                if (u is not (FunctionExpr or PowerExpr))
                    continue;
                var result = SolveInverse(u, rhs, x, ctx, domain, depth);
                if (result is not null)
                    return result;
`@

With `term = 2`, `rhs = -2` and `u = Power(x, 1/2)` (a `PowerExpr`, so line 591 passes).

**Step 4 — the non-invertible rewrite (the root defect), SolveInverse's `PowerExpr` case.**

`@csharp
// Solve.cs:638-660
            case PowerExpr p when p.Exponent is RationalConstantExpr pe && pe.Value != Rat.One:
            {
                u = p.Base;
                var n = pe.Value;
                if (n.IsZero)
                    return null;
                // u^n = c has exactly n solutions over the complex field. Emitting only the
                // principal root would under-report the solution set, so every root of unity is
                // generated whenever the exponent is a small positive integer.
                var principal = Exprs.Power(c, Exprs.Rational(Rat.One / n));
                if (n.IsInteger)
                {
...
                }
                inverses = new[] { principal };
                break;
            }
`@

`n = 1/2`, so `Rat.One / n = 2` and `principal = (-2)^2 = 4` (line 647). Because
`n.IsInteger` is false, line 659 sets `inverses = [4]` — the single candidate produced by raising
both sides to the reciprocal exponent. `u^(1/2) = -2 ⇒ u = 4` is a **one-way** implication under the
principal-branch reading the kernel itself uses elsewhere (cf. the principal-branch comments at
`Solve.cs:619-628`); it silently manufactures a candidate that the original equation does not satisfy.

**Step 5 — the candidate set is accepted unverified and the completeness claim is minted.**

`@csharp
// Solve.cs:666-686
        // u is the part containing x: solve u == inverse for x recursively, once per branch
        var set = new SolutionSet(SolveStatus.Solved, domain);        // <-- line 667: Solved, up front
        foreach (var inverse in inverses)
        {
            var inner = SolveCore(Exprs.Subtract(u, inverse), x, ctx, domain, depth + 1);
            switch (inner.Status)
            {
                case SolveStatus.NoSolutions:
                    continue;
                case SolveStatus.Solved:
                    set.Solutions.AddRange(inner.Solutions);          // <-- lines 675-678: accepted, no check
                    set.Families.AddRange(inner.Families);
                    break;
                default:
                    return inner;
            }
        }
        if (set.Solutions.Count == 0 && set.Families.Count == 0)
            return new SolutionSet(SolveStatus.NoSolutions, domain, "no branch produced a solution.");
        SortSolutions(set);
        return set;                                                   // <-- line 686: Solved returned
`@

The inner solve is `SolveCore(x - 4)` → `SolvePolynomial` (`Solve.cs:346-353`) →
degree-1 factor → `LinearRoot`:

`@csharp
// Solve.cs:449-454
    private static Expr LinearRoot(Polynomial p)
    {
        var c0 = p.ConstantTerm;
        var c1 = p.LeadingCoefficient(MonomialOrder.Lex);
        return Exprs.Divide(Exprs.Rational(Rat.Negate(c0)), Exprs.Rational(c1));
    }
`@

which yields the rational `4` — exactly the `(rat 4 1)` the envelope shows, and `Exact`
(`Solve.cs:427-428`). This closes the chain: the only route in `Solve.cs` that can publish
`value: 4` for this input is `SolveInverse` line 647 → line 670 → `LinearRoot`.

**Step 6 — the false status is projected, never re-derived.** `SolutionSet.Complete` is pure projection
of `Status`:

`@csharp
// Solve.cs:96-101
    public Completeness Complete => Status switch
    {
        SolveStatus.Solved => Completeness.Complete,
        SolveStatus.Partial => Completeness.Partial,
        _ => Completeness.Unknown,
    };
`@

and the plugin maps the status to the published pair (`SymbolicsPlugin.cs:31-40`):

`@csharp
        SolveStatus.Solved => (true, Completeness.Complete),
`@

published at `SymbolicsPlugin.cs:521` (`var (complete, completeness) = SolveCompletenessMapping.Of(status, set.Complete);`)
into fields at `SymbolicsPlugin.cs:527-534`. **The projection is faithful; the lie is upstream in the
kernel's `Status`.** No fix belongs in `SolveCompletenessMapping`.

The only post-solve filtering on this path is condition filtering, which is not verification:

`@csharp
// SymbolicsPlugin.cs:1082-1092
    private static List<Solution> AcceptedSolutions(SolutionSet set, Symbol x)
    {
        var kept = new List<Solution>();
        foreach (var sol in set.Solutions)
        {
            if (sol.Conditions.IsUnsatisfiable || ViolatesConditions(sol, x))
                continue;
            kept.Add(sol);
        }
        return kept;
    }
`@

`ViolatesConditions` (`SymbolicsPlugin.cs:1378-1391`) only inspects
`ExpressionPropertyAssumption { P: SymbolPredicate.NonZero }` atoms; the bogus solution carries
`conditions: []` (envelope), so nothing is dropped. `SymbolicsPlugin.cs:500-502` only turns
`Solved` + zero-accepted into `NoSolutions` — it cannot detect a *wrong* accepted root.

## D1.4 Does a verification step exist already in the file? Yes — two, and this path bypasses both

1. **`ZeroesAt`** — substitution-and-evaluate, used *only* by the rational-function branch:

`@csharp
// Solve.cs:273-281
    /// <summary>True when <paramref name="e"/> evaluates to numeric zero at x = value. Used to
    /// exclude numerator roots that are poles of the original rational function.</summary>
    private static bool ZeroesAt(Expr e, Symbol x, Expr value, ExprContext ctx)
    {
        var at = Evaluation.Substitute(e, ctx, new Dictionary<Symbol, Expr> { [x] = value });
        if (Evaluation.ConstantToNum(at) is not { } n)
            return false;   // not decidable numerically: keep the value, carry the condition
        return NumOps.IsZero(n);
    }
`@

Call site — `Solve.cs:224`, inside the rational-function branch only:
`if (ZeroesAt(denExpr, x, sol.Value, ctx)) { excluded++; continue; }`.

2. **`SatisfiesAll`** — high-precision residual check, used *only* inside `SystemSolvers`:

`@csharp
// Solve.cs:1000-1012 (abridged)
    /// <summary>True when the solution satisfies every remaining equation (symbolically where the
    /// simplifier can prove it, numerically at high precision otherwise).</summary>
    private static bool SatisfiesAll(Expr value, IEnumerable<Expr> equations, Symbol x, ExprContext ctx)
    {
        using var scope = global::Lovelace.Real.Real.WithPrecision(40, 20);
...
            var at = Evaluation.Substitute(eq, ctx, new Dictionary<Symbol, Expr> { [x] = value });
`@

Call site — `Solve.cs:942`: `if (!SatisfiesAll(sol.Value, polys.Skip(1), x, ctx)) continue;`.

**Why the radical path bypasses them:** `SolveInverse` (`Solve.cs:601-687`) contains no call to
either helper, and `SolveCore` returns whatever a sub-solver produced without a post-condition
(`Solve.cs:266-268`). `ZeroesAt` is wired to exactly one call site (224) and `SatisfiesAll` to
exactly one (942); nothing in the elementary/radical family uses them. The verification machinery exists,
is correct in spirit (including the honest "not decidable → keep" convention at `Solve.cs:279`), and is
simply not on this branch.

## D1.5 Minimal fix proposal (bounded)

**File:** `Lovelace.Symbolics/Solvers/Solve.cs` — **method:** `Solvers.SolveInverse`
(`601-687`), radical sub-case of the `PowerExpr` branch (`638-660`).

1. Add one private predicate next to `ZeroesAt`, e.g.
`static bool SatisfiesAt(Expr residual, Symbol x, Expr value, ExprContext ctx)`, that substitutes and
returns false only when the residual is *provably* nonzero — reusing the existing conventions of
`ZeroesAt` (line 277-280: undecidable ⇒ true/keep) and `SatisfiesAll` (line 1004-1005: 40-digit real
scope, but no `return false` on `EvaluationException` — keep instead of drop, so the fix can never
silently delete a legitimate complex root).
2. In the `case SolveStatus.Solved:` arm at `675-678`, admit only candidates whose residual
`Exprs.Subtract(h, c)` (the *original* equation `u^(1/n) = c`, i.e. `sqrt(x) + 2 = 0`) is not
provably nonzero at `sol.Value`. This is a *filter on a superset*: raising to the reciprocal exponent
over-approximates, so dropping provably-false candidates preserves completeness; no `AsPartial` is
needed for dropped candidates.
3. Keep the claim honest when everything is filtered: if candidates were dropped and none survive,
`SolveInverse` must return `new SolutionSet(SolveStatus.NoSolutions, domain, ...)` (the existing
no-branch arm at `683-684` already does this for empty branch results) — never the `Solved` set
constructed at line 667.
4. **Recommended first patch (narrowest):** apply the filter only when `!n.IsInteger` (the radical /
non-integer-exponent case at line 659). The integer case generates all `n` roots of unity (`648-657`)
and its inversion is exact, so it needs no behavioural change; narrowing the patch keeps the blast radius to
radicals.

Follow-up (not required for the first patch, record it): `SolvePeriodic` families
(`Solve.cs:740-808`) are accepted with the same absence of substitution evidence; a family whose
principal representative does not satisfy the argument equation should be dropped or the status degraded.

## D1.6 Test that would FAIL on the current tree

Runner-level (fails today against `out/aot/Lovelace.Run.exe`):

`@
x = symbol("x"); r = solve_full(sqrt(x) + 2 == 0, x); r.status; r.complete; r.completeness; r.represented_count
`@

Current: `Solved`, `true`, `Complete`, `1` — expected after the fix:
`NoSolutions`, `true`, `Complete`, `0` (a *provably empty* set is still a complete answer
per `SymbolicsPlugin.cs:19-23` and `dsh-protocol.md:166-170`, so asserting
`complete == false` would be the wrong expectation).

Symbolics-kernel unit test (fails today), in `Lovelace.Symbolics.Tests`:

`@csharp
var x = Exprs.Current.Symbol("x");
var f = Exprs.Add(Exprs.Power(Exprs.Symbol(x), Exprs.Rational(1, 2)), Exprs.Rational(2));
var set = Solvers.Solve(f, x);                       // f == 0
Assert.Equal(SolveStatus.NoSolutions, set.Status);   // FAILS: currently Solved
Assert.Empty(set.Solutions);                         // FAILS: currently [4]
`@

Property test that generalizes beyond this instance (fails today) — for every solution of every
`h(u) = c` radical instance, the substituted residual must be numerically zero at high precision
(same 40-digit scope as `SatisfiesAll`, `Solve.cs:1004`); run it over
`sqrt(x) = c`, `x^(1/3) = c`, `sqrt(x+1) = c` for c ∈ {−4,−2,−1,0,1,2,4}.

## D1.7 Blast radius and counter-indications

* **Same defect is reachable through the text `solve` builtin.** `SymbolicsPlugin.cs:460-468`:
when `set.Status == SolveStatus.Solved && kept.Count > 0` it returns the values as a Vector, so
`solve(sqrt(x) + 2 == 0, x)` ("code-derived; not re-executed this round") returns `[4]` with no
caveat. Any fix in `SolveInverse` fixes both surfaces at once.
* **Positive blast radius.** Filtering provably-false candidates can only *remove* published roots. It cannot
turn a correct `Solved` into `Partial`, because extraneous candidates are over-approximation, not
missing coverage. Risk of regression is therefore concentrated in (a) numeric decidability and (b) complex
roots.
* **Counter-indication 1 — branch semantics.** Under a *multivalued* reading of the radical,
`sqrt(4) = ±2`, so `x = 4` really would satisfy `sqrt(x) = -2`. The kernel's stated convention is the
principal branch (`Solve.cs:619-628` for `exp`/`log`; `Interpreter.cs:1458-1463` documents the
principal square root for negatives). The fix must therefore *also* state the branch convention explicitly
in the `PowerExpr` case; otherwise "verify by substitution" changes the meaning of radicals instead of
enforcing the documented one. This is the single most important design decision in the fix.
* **Counter-indication 2 — undecidable residuals.** For symbolic `c` or composite `u`, the residual may
not fold to a number. Dropping on undecidable residuals would delete legitimate roots; the required
convention is the existing one, `Solve.cs:279`: *keep* and carry the condition.
* **Counter-indication 3 — complex candidates.** `Solve.cs:644-657` deliberately emits all `n` roots of
unity so the complex solution set is not under-reported. A naive real-only numeric check would drop valid
complex roots; the check must run in the complex domain or refuse to drop.
* **Counter-indication 4 — `Families`.** `SolvePeriodic` output is merged in the same block
(`Solve.cs:677`). If the filter is written as "drop everything when the residual is nonzero", it must not
touch families until family verification is designed (§D1.5 follow-up).
* **Not fixed by this proposal (recorded, not diagnosed):** the sibling bug class where the *status* is
optimistic without an inversion at all — e.g. `SystemSolveResult.Status` (`Solve.cs:859-869`) reports
`Solved` for any non-truncated non-empty list even though `SolveRecursive` can `return` early
(`Solve.cs:938`, `970`, `977`, `982`) without marking `Truncated`.

## D1.8 Open questions

1. **Principal vs multivalued radical semantics** — which reading is normative for v1? The doc set does not
state it for `sqrt` as a *solve* operation (the doc says only what `complete` means). The fix's
expected test outcome (`NoSolutions` vs `Solved` with `x = 4` under a multivalued reading) depends on
this. **I would resolve this first.**
2. **Where exactly to filter.** I propose `SolveInverse` (radical branch). An alternative is a single
post-check in `SolveCore` (before `Solve.cs:267`) that verifies every solution of every sub-solver
against `f` — broader safety net, larger blast radius (polynomial paths would be re-checked too, and
`RootOf`/isolated-root values are numerically hard to verify to 40 digits). **I would test the
`SolveInverse`-local filter first** because it is the smallest change that provably kills this repro.
3. `Exprs.Power(-2, 2)` folding: I did not read `Constructors.cs` / `Exprs.Power` (out of the declared
scope). The trace assumes `Rat.One / (1/2) = 2` and that `Exprs.Power` folds the constant product. The
observed solution value `4` with canonical `(rat 4 1)` is consistent only with that assumption, but the
fold itself was not read. **No claim depends on it beyond the identification of line 647.**
4. Does `Algebra.TryCoefficients` (`Solve.cs:251`) ever return two coefficients for
`sqrt(x) + 2`? Not read (out of scope); if it did, `c1` would have to be a zero
`RationalConstantExpr` for the branch to be skipped, which is what the observed behaviour implies.

---

# Defect 2 (T0-5) — `solve_system(x + y == 2, x - y == 0)` dies as InternalError/InternalInvariantFailure

## D2.1 Symptom

`ok: false`, `code: "InternalError"`, `category: "InternalInvariantFailure"`,
`message: "Specified cast is not valid."`, `recoverable: false` — a CLR
`InvalidCastException` escaping the builtin. Per `dsh-protocol.md:172-181` that taxonomy slot is for
"an internal invariant failure", and prior rounds recorded this as a contract violation
(`docs/goal-cycle-4/evidence.md:54` EVD-195, `docs/goal-cycle-4/round-09/audit-P2P6-wire.md` F4).
Verbatim envelope (`docs/goal-cycle-5/probes/pre/t05-solve-system.out.txt:1`):

`@
{"ok":false,"code":"InternalError","category":"InternalInvariantFailure",
 "message":"Specified cast is not valid.","recoverable":false,
 "diagnostics":[{"message":"Specified cast is not valid.","position":0,"line":1,"column":1}]}
`@

## D2.2 Repro command

`docs/goal-cycle-5/probes/pre/t05-solve-system.ls:1` (exact file contents):

`@
x = symbol("x"); y = symbol("y"); solve_system(x + y == 2, x - y == 0)
`@

The contrasting form (`docs/goal-cycle-5/probes/pre/t05b-solve-system-list.ls:1`) succeeds:

`@
x = symbol("x"); y = symbol("y"); solve_system([x + y == 2, x - y == 0], [x, y])
`@

→ `"ok":true, "display":"x = 1, y = 1"`.

## D2.3 First: `Interpreter.cs:1021` is NOT the dispatch site

`Interpreter.cs:1015-1037` is the doc comment and body of `MatrixResultRecord`; line 1021 is the
middle of that comment:

`@csharp
// Interpreter.cs:1019-1021
    /// mapping <see cref="Lovelace.Symbolics.SolveCompletenessMapping"/> that SolveResult and
    /// SystemSolveResult publish, so a singular system reports NoSolutions/true/Complete exactly as
    /// the scalar solver does. Diagnostics are built from the Round-4b vocabulary: an EMPTY array
`@

There is **no** `solve_system` string anywhere in `Interpreter.cs` (repo-wide grep for
`solve_system` outside `SymbolicsPlugin.cs` returns only probe/evidence/doc files). Plugin builtins
are dispatched generically.

## D2.4 Root cause — the bind/dispatch chain and the exact cast

**The cast that throws: `SymbolicsPlugin.cs:429`, in the lambda body of the `solve_system`
builtin (`427-440`).**

`@csharp
// SymbolicsPlugin.cs:427-435
        Add("solve_system", new[] { "eqs", "vars" }, args =>
        {
            var eqs = ((IReadOnlyList<object?>)args[0]!).Select(AsExpr).ToArray();   // <-- line 429 throws
            var vs = NameList(args[1]).Select(Context.Symbol).ToArray();
            var result = SystemSolvers.Solve(eqs, vs, Context);
            if (result.Solutions.Count == 0)
                return "no solutions" + (result.Note is { } note ? ": " + note : "");
            return string.Join("; ", result.Solutions.Select(sol =>
                string.Join(", ", vs.Select(v => v.Name + " = " + Printing.PrettyPrint(sol.Assignment[v])))));
`@

Full chain, with the payload type at every hop:

1. **Registration.** `SymbolicsPlugin.Register` (`153`) defines the local `Add` helper
(`159-180`) which calls `c.RegisterBuiltin(resolved, args => { ... Run(() => impl(tracked)); })`
(line `166`). `solve_system` is registered with an explicit `BuiltinDescriptor` whose
parameters are `["eqs","vars"]` and whose declared return kind is `"Text"`
(`SymbolicsPlugin.cs:437-440`).
2. **Host adapter.** `ModusHost.RegisterBuiltin(BuiltinDescriptor, Func<IReadOnlyList<object?>, object?>)`
**[out-of-scope read]** (`ModusHost.cs:86-98`):

`@csharp
// ModusHost.cs:91-96
        _interpreter.RegisterBuiltin(descriptor.Name, descriptor.Parameters, args =>
        {
            CheckArity(descriptor.Name, descriptor.Parameters, descriptor.Variadic, descriptor.MinArity, args);
            using var scope = PluginPrecisionScope();
            return WrapResult(implementation(UnwrapArguments(args)));
        }, descriptor);
`@

3. **Interpreter call site.** `Interpreter.cs:606-620`: arguments are evaluated as `Value`s, then
`Interpreter.cs:614-615`:

`@csharp
            if (fn.IsBuiltin)
                return await fn.Builtin!(args);
`@

4. **Arity passes for the two-argument call.** `CheckArity` **[out-of-scope read]**
(`ModusHost.cs:123-132`) treats `Parameters.Count` as the upper bound and `MinArity < 0` as
"exactly the declared count" (`ModusHost.cs:115-122`). Two relation arguments == two declared
parameters, so the body is entered. (This is also why `solve_system([eq1, eq2])` — one argument — leaves
as a *recoverable* `BuiltinArityException`, `ModusHost.cs:219-229`, matching EVD-195's "arity error".)
5. **Payload conversion.** `UnwrapArguments` **[out-of-scope read]** (`ModusHost.cs:200-206`) maps
each `Value` through `PayloadMap.Unwrap` (`PayloadMap.cs:75-93`). For a symbolic relation:

`@csharp
// PayloadMap.cs:81
        ValueKind.Symbolic => value.AsSymbolic(),
`@

and `Value.AsSymbolic` is `(Lovelace.Symbolics.Expr)_inner` (`Value.cs:197`). So
`args[0]` arrives at the plugin body as a **`Lovelace.Symbolics.RelationExpr`** — a class, not a
list.
6. **Why the payload is a relation.** `x + y == 2` is compared symbolically:
`Interpreter.cs:537-540` routes any symbolic operand to `NumericOps.Apply` **[out-of-scope read]**,
which for `BinaryOp.Equal` builds a relation (`NumericOps.cs:169-189`):

`@csharp
// NumericOps.cs:180-188
            BinaryOp.Equal => Exprs.Relation(RelOp.Eq, l, r),
...
        return new Value(result);
`@

`new Value(Expr)` sets `Kind = ValueKind.Symbolic` (`Value.cs:90-94`), so `Unwrap` yields the
`RelationExpr` itself.
7. **The failing cast.** `(IReadOnlyList<object?>)args[0]` on a `RelationExpr` is an invalid
interface cast → `InvalidCastException` at `SymbolicsPlugin.cs:429`. This is the **first**
operation in the body, and the only cast in the entire path that can fail.
8. **Why the runner calls it an internal invariant failure.** `Runner.cs:239` classifies the caught
exception:

`@csharp
// Runner.cs:256-275 (abridged)
    private static (string Code, string Category, bool Recoverable) Classify(Exception ex) => ex switch
    {
...
        InvalidOperationException => ("InvalidOperation", "DomainError", true),
        ArgumentException => ("InvalidArgument", "TypeMismatch", true),
...
        _ => ("InternalError", "InternalInvariantFailure", false),
    };
`@

`InvalidCastException` matches no arm → the `_` default at `Runner.cs:274`; the message text is
the raw `ex.Message` written at `Runner.cs:249`.

The identical unchecked cast exists twice more in the same file
(`SymbolicsPlugin.cs:443` in `solve_system_full` and `SymbolicsPlugin.cs:570`), so any other
builtin that receives a non-list first argument crashes the same way.

## D2.5 Why the one-argument list form takes a different path

Same builtin, same line 429 — only the **runtime type of `args[0]`** differs:

* `solve_system([x + y == 2, x - y == 0], [x, y])`: the vector literal evaluates to a
`ValueKind.Vector` value, `PayloadMap.Unwrap` routes it to `UnwrapArray`
(`PayloadMap.cs:105-114`), which returns `new PayloadArray(payloads, shape)` — and
`PayloadArray` is `public sealed class PayloadArray : IReadOnlyList<object?>`
(**[out-of-scope read]** `Lovelace.Abstractions/PayloadArray.cs:10`). The cast at 429 therefore
**succeeds**; `Select(AsExpr)` (`SymbolicsPlugin.cs:1455-1464`, which accepts `Expr`) unwraps the
`RelationExpr` elements, and `SystemSolvers.Solve` runs the Gröbner elimination.
* `solve_system([eq1, eq2])` (one argument): rejected *before* the body by `CheckArity`
(`ModusHost.cs:129-131`) as a recoverable `InvalidArgument/TypeMismatch` — the "arity error" of
EVD-195.
* The two-relation form is the only one that reaches line 429 with a non-list first argument.

**Documentation caveat (recorded as OPEN, not as a claim):** the repo's own descriptor for this builtin
documents and exemplifies the *list* form — `["solve_system([x^2 + y^2 - 1 == 0, x*y == 0], [x, y])"]`
(`SymbolicsPlugin.cs:439`) — and I could not find the two-relation form documented anywhere in this
repo: a repo-wide grep for `solve_system(x` matches only
`docs/goal-cycle-4/evidence.md:54,59`, `docs/symbolics/a-plus-cycle-4-report.md:205`,
`docs/goal-cycle-5/evidence.md:15`, `docs/goal-cycle-5/run-probes.ps1:12` and the probe script
itself, and `docs/symbolics/dsh-protocol.md` contains **zero** occurrences of `solve_system`. Grep in
the DSH harness checkout (`C:\Users\ricar\deepseek-harness`) also returned zero. If the DSH-facing tool
schema advertises a two-relation form, that schema and the builtin disagree; the fix must make them agree
and it is not decidable from this repo which side is normative.

## D2.6 Minimal fix proposal (bounded)

**File:** `Lovelace.Symbolics/SymbolicsPlugin.cs` — **method:** the `solve_system` lambda
registered at `427-436` (and, identically, `solve_system_full` at `441-459` and the third
occurrence at `570`).

1. **Minimum (contract repair, no new capability):** replace the unchecked cast with a checked coercion that
raises the plugin's existing coercion failure type, so the wrapper at `166-179` converts it into a named,
recoverable error:

`@csharp
if (args[0] is not IReadOnlyList<object?> eqs)                      // instead of (IReadOnlyList<object?>)args[0]!
    throw new BuiltinArgumentError("a vector of equations, e.g. [x + y == 2, x - y == 0]");
`@

`BuiltinArgumentError` is private to this file (`SymbolicsPlugin.cs:1417-1420`) and the wrapper
already renders it as `solve_system(): argument 1 must be a vector of equations, e.g. ...; got Symbolic.`
(`DescribeArgument`, `1422-1427`, and `DescribePayload`, `1430-1444`). The runner then
classifies it `InvalidOperation/DomainError`, `recoverable: true` (`Runner.cs:267`) — an
honest refusal instead of an invariant failure. This is the smallest change that satisfies "never an
internal invariant failure".
2. **If the two-relation form is contractual:** add the accepting branch —
`args.Count == 2 && args[0] is Expr && args[1] is Expr` ⇒ treat the arguments as the equation list and
derive the variables from their free symbols (the kernel already has `CollectNames`,
`Solve.cs:823-833`, and `Interpreter.CollectSymbols`, `Interpreter.cs:1172-1223`) — or declare the
extra form in the descriptor (`Variadic: true`, `MinArity: 2`) and document it in
`SymbolicsPlugin.cs:439`'s example list. **Do not** silently reinterpret `args[1]` as the variable list, which
would misread `x - y == 0`.
3. Do the same checked coercion at `443` and `570` (same defect class, same file).

I would land (1) first: it is provably sufficient to remove the `InternalError`, needs no semantic
decision, and cannot change any working call. (2) is a product decision that needs the doc side resolved.

## D2.7 Test that would FAIL on the current tree

Runner-level (fails today — the probe above is exactly this assertion):

`@
x = symbol("x"); y = symbol("y"); solve_system(x + y == 2, x - y == 0)
`@

Assertion: `code != "InternalError"`; the acceptable minimum is
`code == "InvalidOperation" && recoverable == true` with a message naming `solve_system` and
argument 1. Current result is `InternalError/InternalInvariantFailure/recoverable:false`.

Suite-level test (fails today) asserting the *class* of failure, not prose — a plugin builtin that receives
a relation where it declared a list must produce a `BuiltinArgumentError`-derived
`InvalidOperationException` and never a bare `InvalidCastException`:

`@csharp
var engine = new SuiteEngine();
var symbolics = new Lovelace.Symbolics.SymbolicsPlugin();
engine.LoadPlugin(symbolics);
var ex = await Assert.ThrowsAsync<InvalidOperationException>(() => engine.EvaluateAsync(
    "x = symbol(\"x\"); y = symbol(\"y\"); solve_system(x + y == 2, x - y == 0)"));
Assert.Contains("solve_system", ex.Message);      // today: InvalidCastException, no function name
`@

Regression guard for the working form (must stay green): the t05b script must still return
`x = 1, y = 1`.

## D2.8 Blast radius and counter-indications

* **Blast radius of fix (1) is small and local:** three occurrences of the same cast in one file. The only
behaviour change is that a call which today *crashes* instead *refuses with a named, recoverable error*.
No call that currently succeeds can start failing.
* **Counter-indication — argument kinds other than lists that currently work.** Before the change, any first
argument that happens to implement `IReadOnlyList<object?>` is accepted; the checked pattern
`is not IReadOnlyList<object?>` preserves exactly that set. `PayloadArray` and `object?[]` both
satisfy it.
* **Counter-indication — the private error type.** `BuiltinArgumentError` is a *private nested* type
(`SymbolicsPlugin.cs:1417`) caught only by the same file's wrapper (`175-178`); a fix must throw it from
inside the `Add`-registered lambda, which is exactly where line 429 sits. Throwing a fresh
`InvalidOperationException` directly also works but loses the argument index/kind rendering.
* **Counter-indication — `solve_system_full` shares the defect** but is covered by its own probe; fixing
all three sites is one change, and the `SystemSolveResult` schema (see §3) means the *record* form is the
one a consumer can read structurally.
* **The message wording "Specified cast is not valid." is runtime-dependent.** On the published NativeAOT
binary a failing `castclass` surfaces the parameterless `InvalidCastException` message, while a CoreCLR
test run prints `Unable to cast object of type '...' to type '...'`
(`docs/goal-cycle-3/round-4b/implementation.md:58` shows that form, and the same binary prints
"Specified cast is not valid." for the pure core casts in `mean(1)`/`max(1,2)`,
`docs/goal-cycle-5/probes/pre/t1-arity-mean1.out.txt` / `t1-arity-max.out.txt`). **This does not change
the throw site**, but a test must assert on `code`/`category`, never on the message string.

## D2.9 Open questions

1. **Is the two-relation form contractual, or is the probe simply the wrong call shape?** I could not find
it documented in this repo (§D2.5). If it is *not* contractual, the correct fix is only (1) — a named
refusal — and a documentation correction. **I would check the DSH tool schema / the doc the orchestrator
read before choosing between (1) and (2).**
2. **Correction to the pointer I was given.** `Interpreter.cs:1021` is a comment on
`MatrixResultRecord`, not the SystemSolveResult publish of `solve_system`. The
`solve_system_full` record publish lives in the *plugin* (`SymbolicsPlugin.cs:453-458`), and the
generic dispatch lives in `Interpreter.cs:606-620` + `ModusHost.cs:86-98`. Recorded so the next round
does not re-search line 1021.
3. **Message-level contract.** Should `Runner.Classify` (`Runner.cs:256-275`) have an
`InvalidCastException` arm mapping to `InvalidArgument/TypeMismatch`? That would convert every
remaining raw cast on the user surface into a recoverable error in one place — attractive, but it also
*silences* genuine internal invariant failures, which is exactly what the T0-5 rule wants to keep visible.
I would **not** add that arm; I would fix the three casts. Flagging it because it is the obvious
"one-line" alternative and it is the wrong one.

---

## 3. ALSO RECORD — the two evidence questions

### 3.1 Does `SystemSolveResult` carry a `completeness` field? **No.**

`@csharp
// RecordSchemas.cs:75-77
        Schema("SystemSolveResult",
            ("status", "Enum"), ("domain", "Domain"), ("complete", "Boolean"),
            ("solutions", "Array"), ("diagnostics", "Array")),
`@

Compare `SolveResult`, which does carry it:

`@csharp
// RecordSchemas.cs:65-69
        Schema("SolveResult",
            ("status", "Enum"), ("variable", "Symbolic"), ("domain", "Domain"), ("complete", "Boolean"),
            ("completeness", "Enum"), ("solutions", "Array"), ("families", "Array"),
            ("common_conditions", "Array"), ("represented_count", "Integer"),
            ("unrepresented_count", "Integer"), ("unrepresented_reason", "Text"), ("diagnostics", "Array")),
`@

The builder agrees with the schema — `SymbolicsPlugin.cs:453-458` publishes exactly
`status, domain, complete, solutions, diagnostics` and no `completeness`. The
`MatrixInverseResult`/`MatrixSolveResult` schemas *do* carry `completeness`
(`RecordSchemas.cs:82-87`). This is an asymmetry with `dsh-protocol.md:153-156`, which claims the
mapping keeps `SolveResult` and `SystemSolveResult` from drifting apart: the *pair* is derived from
one expression, but only one of the two records exposes the authoritative enum — a consumer of
`SystemSolveResult` must derive completeness from the convenience boolean. Recorded as evidence only; no
fix proposed this round. (Also note the audit record `docs/goal-cycle-4/round-09/audit-P2P6-wire.md:93`
already raised this as P2-24.)

### 3.2 What does the two-argument path return instead of a record? **Nothing — it throws before any return.**

The builtin reached by `solve_system(...)` is the *text* builtin: its descriptor declares the return kind
as `"Text"` (`SymbolicsPlugin.cs:437-440`) and its two return statements are strings
(`SymbolicsPlugin.cs:432-435`). The `SystemSolveResult` **record** is produced only by
`solve_system_full` (`SymbolicsPlugin.cs:441-459`). So in the failing call the exception at
`SymbolicsPlugin.cs:429` fires before either string return; had the cast been checked, the caller would
have received prose (`"no solutions"` or `"x = 1, y = 1"`), never a record — which is itself the F1
"silent degradation" finding (`docs/goal-cycle-4/round-09/audit-P2P6-wire.md:107-134`).

---

## 4. Appendix — citation index for the two decision points

| Claim | Citation |
|---|---|
| `Solved` ⇒ represented set is complete | `Solve.cs:12-15`, `Solve.cs:88-89`, `docs/symbolics/dsh-protocol.md:158-164` |
| Radical inversion mints the bogus candidate | `Solve.cs:647` (`Exprs.Power(c, Exprs.Rational(Rat.One / n))`) |
| Completeness claim minted before any check | `Solve.cs:667` |
| Candidate accepted with no substitution | `Solve.cs:670`, `Solve.cs:675-678` |
| Verification exists but is not wired here | `Solve.cs:275-281` (used at `224`), `Solve.cs:1002-1030` (used at `942`) |
| Status→(complete, completeness) projection is faithful | `SymbolicsPlugin.cs:31-40`, published at `521`/`527-528` |
| No post-solve equation check in the plugin | `SymbolicsPlugin.cs:1082-1092`, `1378-1391` |
| `sqrt` ⇒ `Power(x, 1/2)` | `Interpreter.cs:1455-1456` |
| Two-argument bind/dispatch chain | `Interpreter.cs:606-620` → `ModusHost.cs:86-98` → `PayloadMap.cs:75-93` |
| The cast that throws | `SymbolicsPlugin.cs:429` (twins at `443`, `570`) |
| Relation payload origin | `NumericOps.cs:169-189`, `Interpreter.cs:537-540` |
| List form succeeds (different runtime type) | `PayloadMap.cs:105-114`, `PayloadArray.cs:10` |
| InternalInvariantFailure classification | `Runner.cs:239`, `Runner.cs:249`, `Runner.cs:256-275` |
| `SystemSolveResult` has no `completeness` | `RecordSchemas.cs:75-77` vs `65-69` |
