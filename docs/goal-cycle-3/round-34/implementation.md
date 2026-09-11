# Round 34 — N19: limit_full must not assert non-existence it did not prove

STATUS: fix + tests complete; full-suite evidence below.

## 1. Defect

`limit_full(x*sin(1/x), x, 0)` returned `status: Unevaluated` **and** `exists: false`.
The limit is 0 (squeeze theorem), so `exists: false` is a false mathematical claim: the engine
did not determine the limit, yet the payload asserted non-existence.

## 2. Located emission site

`Lovelace.Symbolics/SymbolicsPlugin.cs`, builtin `limit_full` (`Add("limit_full", ...)`, was line 373):

    bool exists = r.Status is LimitStatus.Value or LimitStatus.PlusInfinity or LimitStatus.MinusInfinity;
    new RecordField("exists", exists),

Every other status — `DoesNotExist`, `Unevaluated`, `Failed` — collapsed into `false`, so
"not determined" and "proven not to exist" were conflated into one boolean.

## 3. Decision: three-valued exists

| LimitStatus | exists | meaning |
|---|---|---|
| Value / PlusInfinity / MinusInfinity | true | a limit was determined |
| DoesNotExist | false | non-existence PROVEN |
| Unevaluated / Failed | Null | not determined — no claim |

`status` is unchanged; the distinction lives in `exists`.

**Why Null is the right encoding.** The protocol already defines an absent field as
`{kind:Null}` and it is already how "no one-sided limit" is carried here: `LimitSide` returns a
`null` payload for a side, `PayloadMap.Wrap(null)` maps it to `Value.Void`, and
`StructuredProjection` renders `ValueKind.Void` as `StructuredValueDto("Null")` — distinct from
`false` (Boolean) and from `""` (Text). The registry already uses `Symbolic|Null` for
`value`/`left`/`right` and `Boolean|Null` for `Inspection.exact`, so `Boolean|Null` needs no new
vocabulary. No new value kind and no prose string was invented. A three-valued `bool?` in the
plugin is exactly what the seam already supports.

## 4. Step A — failing output (before fix)

Test added first (`Lovelace.Symbolics.Tests/LimitExistenceTriStateTests.cs`, 6 assertions),
run `dotnet test Lovelace.Symbolics.Tests -c Release --filter FullyQualifiedName~LimitExistenceTriStateTests`:

    Failed Lovelace.Symbolics.Tests.LimitExistenceTriStateTests.UndeterminedLimit_ReportsNullExists_NotFalse [46 ms]
    Error Message:
     Assert.Equal() Failure: Values differ
    Expected: Void
    Actual:   Boolean
     Stack Trace:
       at ...LimitExistenceTriStateTests.UndeterminedLimit_ReportsNullExists_NotFalse() in LimitExistenceTriStateTests.cs:line 45
    Failed!  - Failed:     1, Passed:     5, Skipped:     0, Total:     6, Duration: 408 ms

The 5 passing controls are the ones that must not change (determined limit → true + value 1,
proven non-existence → false, undetermined sides → Null + empty conditions).

Runner on the unfixed binary (`docs/goal-cycle-3/round-34/probe.ps1`):

    n19_undetermined         exit=0 status=Unevaluated exists=[Boolean:false] value=[Null:] left=[Null:] right=[Null:] lcond=0 rcond=0
    n19_control_value        exit=0 status=Value       exists=[Boolean:true]  value=[Symbolic:] left=[Null:] right=[Null:] lcond=0 rcond=0
    n19_dne_abs_over_x       exit=0 status=Unevaluated exists=[Boolean:false] value=[Null:] left=[Null:] right=[Null:] lcond=0 rcond=0
    n19_dne_one_over_x       exit=0 status=DoesNotExist exists=[Boolean:false] value=[Null:] left=[Symbolic:-inf] right=[Symbolic:inf] lcond=1 rcond=1

Two findings from Step A:

* **`abs(x)/x` at 0 is NOT a genuine DoesNotExist here** — the engine returns Unevaluated for it,
  so it is a second member of the "undetermined" class, not a witness of non-existence. The task
  suggested it as the genuine-DNE case; it is not one on this binary. The genuine witness used
  instead is `1/x` at 0 (odd pole: −∞ left, +∞ right, sides disagree).
* The genuine-DNE row shows the pre-existing behaviour that must be preserved: `false` **with**
  both one-sided values and their conditions.

## 5. One-sided findings

The same conflation is **not** present in the one-sided fields, and no change was needed there:

* `left`/`right` go through `LimitSide(r)`, whose default arm already returns a `null` payload —
  i.e. protocol Null — for anything that is not a determined value (Unevaluated, Failed,
  DoesNotExist). An undetermined side therefore already said "nothing", never a fabricated value.
* `left_conditions`/`right_conditions` are an empty `Array` (not Null, not `""`) when the side is
  Null or Unevaluated.
* The only coupling was the gate `SideConditions(..., twoSidedExists)`: it keyed off the same
  derived boolean. It now takes `exists is true` — a *determined* two-sided limit suppresses the
  side constraints; both "proven not to exist" and "not determined" leave them attached, which is
  correct (the side value, when present, genuinely only holds on that side). This is
  behaviour-preserving: `Limits.Limit` never attaches sides to an Unevaluated result
  (`LimitResult.Uneval` carries none), so an undetermined side short-circuits on `side is null`
  regardless of the flag.
* Kernel-level check added: for `x*sin(1/x)` at 0 in all three directions the result is
  `Unevaluated` with `Value`, `FromLeft` and `FromRight` all null — nothing to report as a claim.

## 6. Schema changes

`exists` changed kind, so the registry changed **in the same commit-sized change**:

* `Lovelace.Suite/RecordSchemas.cs`: `LimitResult.exists` `"Boolean"` → `"Boolean|Null"`
  (the same `X|Null` form the registry already uses for `value`/`left`/`right`/`Inspection.exact`).
* `Lovelace.Suite.Tests/RecordSchemaTests.cs`: the matching `AssertSchema("LimitResult", ...)`
  expectation updated in lockstep (the drift test pins kinds as well as names).

No golden files changed: the field's *name* is unchanged and the fixtures exercised are
unaffected, so nothing was regenerated (Run.Tests passed unmodified — see §8).

## 7. Step B — passing output (after fix)

    Passed!  - Failed:     0, Passed:     6, Skipped:     0, Total:     6, Duration: 376 ms - Lovelace.Symbolics.Tests.dll (net10.0)

## 8. Suites (all three required, after the fix)

    dotnet build LovelaceSharp.slnx -c Release --nologo
    Build succeeded.  0 Error(s)

    dotnet test Lovelace.Symbolics.Tests -c Release --nologo
    Passed!  - Failed:     0, Passed:   740, Skipped:     6, Total:   746, Duration: 16 s

    dotnet test Lovelace.Suite.Tests -c Release --nologo
    Passed!  - Failed:     0, Passed:   640, Skipped:     0, Total:   640, Duration: 12 s

    dotnet test Lovelace.Run.Tests -c Release --nologo
    Passed!  - Failed:     0, Passed:    39, Skipped:     0, Total:    39, Duration: 365 ms

No test was deleted, skipped or weakened. No golden file changed, so none was regenerated
(the round touched no fixture or verified file).

## 9. Runner probes (fixed binary) — docs/goal-cycle-3/round-34/probe.ps1

    n19_undetermined         exit=0 status=Unevaluated  exists=[Null:]          value=[Null:]        left=[Null:]         right=[Null:]         lcond=0 rcond=0
    n19_control_value        exit=0 status=Value        exists=[Boolean:true]   value=[Symbolic:1]   left=[Null:]         right=[Null:]         lcond=0 rcond=0
    n19_dne_abs_over_x       exit=0 status=Unevaluated  exists=[Null:]          value=[Null:]        left=[Null:]         right=[Null:]         lcond=0 rcond=0
    n19_dne_one_over_x       exit=0 status=DoesNotExist exists=[Boolean:false]  value=[Null:]        left=[Symbolic:-inf] right=[Symbolic:inf] lcond=1 rcond=1
    n19_left_undetermined    exit=0 kind=Text -> unevaluated: coefficient does not evaluate at the point
    n19_right_undetermined   exit=0 kind=Text -> unevaluated: coefficient does not evaluate at the point

Raw wire form of the N19 case, from `n19_undetermined.out.txt`:

    status           = {"name":"status","value":{"kind":"Enum","type":"LimitStatus","value":"Unevaluated"}}
    exists           = {"name":"exists","value":{"kind":"Null"}}
    value            = {"name":"value","value":{"kind":"Null"}}
    left             = {"name":"left","value":{"kind":"Null"}}
    left_conditions  = {"name":"left_conditions","value":{"kind":"Array","type":"Vector","shape":[0],"elements":[]}}

`exists` is `{kind:Null}` — the protocol's absent value, distinct from `false` and from `""` — while
`status` still says Unevaluated. The three probes the round asked for:

    limit_full(x*sin(1/x),x,0) -> status Unevaluated,  exists NULL,  value NULL
    limit_full(sin(x)/x,x,0)   -> status Value,        exists true,  value 1
    limit_full(1/x,x,0)        -> status DoesNotExist, exists FALSE, left −inf / right +inf

## 10. Files changed (no commit)

* `Lovelace.Symbolics/SymbolicsPlugin.cs` — the emission site: three-valued `bool? exists`, and
  `exists is true` handed to the side-condition gate.
* `Lovelace.Suite/RecordSchemas.cs` — `LimitResult.exists` kind `Boolean` → `Boolean|Null`.
* `Lovelace.Suite.Tests/RecordSchemaTests.cs` — the pinned kind expectation, in lockstep.
* `Lovelace.Symbolics.Tests/LimitExistenceTriStateTests.cs` — new, 6 tests.
* `docs/goal-cycle-3/round-34/` — this report, `probe.ps1`, the `.ls`/`.out.txt` probes.