# Cycle-6 · round-20 · audit G — cross-surface consistency

**Target**: the published Native AOT binary `C:\Users\ricar\dev\LovelaceSharp\out\aot\Lovelace.Run.exe`
(5 804 544 bytes, 2026-09-12 18:00:55 local — 834 s newer than the newest code file).

**Strategy**: take one computation and ask every surface the same question, then compare the answers:
`--eval` / `--file` / `--stdin` (and the positional-file route), `result.display` / `result.typed` /
`result.structured` / `variables[_].*` / `output[]`, the error envelope, `capabilities()`, the plot
artifact, and the golden fixtures.

**Method**: ~900 process invocations, each launched through `System.Diagnostics`-equivalent process
control (Node `child_process.spawnSync`) with an **argument array** (`--eval` hand-quoted exactly) and
**the script bytes fed to `--stdin` verbatim**, so no shell quoting or console encoding sits between the
probe and the binary. Exit code, stdout bytes and stderr bytes are read from the process. Scripts with
embedded `"` are given to `--eval` through the argument array, never through a PowerShell string (the
round-13 "quote-mangling trap": `& exe --eval 'print("A B")'` from PS 5.1 arrives as `print(A B)` —
observed, shell-level, not a product defect). Where a plain-PowerShell command is quoted below it is one
that was actually run from `pwsh` and it avoids that trap.

**Reproduction rule**: every item below was run at least twice (the fuzz corpus reproduced the one live
divergence 44 times). Run counts are stated.

**Recorded-items check (done first, as required)**: read before probing —
`docs\goal-cycle-6\evidence.md` (EVD-277…EVD-295), `docs\symbolics\a-plus-cycle-6-amendment.md`
(P-1…P-17 and the P.2/P.3 bounds at `:38-59`), `docs\goal-cycle-6\round-18\audit-E-cli.md`
(F1…F8), and `docs\goal-cycle-6\round-19\positions-implementation.md` (§8 "Could not verify / known
residuals", `:532-554`). Every match is named in the item or in §"Matched a recorded item".

---

## What held — the falsification result

No new P0 and no new P1. The cross-surface claim survived, with the exceptions listed as G-2…G-7 below
(every one of them already recorded by an earlier wave) and one **new P2 documentation** item (G-1).

1. **Three surfaces, identical envelopes.** 20 golden-fixture scripts + 150 generated scripts + 20
   hand-built edge inputs (LF, CRLF, CR-only, mixed, trailing LF/CRLF, leading BOM, BOM-only, empty,
   whitespace-only, unicode, interpolated strings, newline inside a string literal, astral characters,
   U+2028) through `--eval`, `--file` and `--stdin`: **exit code, `ok`, `code`, `category`,
   `message`, `diagnostics` (position/line/column), `timings` (position, resultKind, hasOutput),
   `output`, `variables`, `result`** are byte-identical after the documented volatile keys
   (`revision`/`elapsed`/`elapsedTime`/`timings[].elapsed`) are removed — **except** for the
   BOM case of G-2. The positional-file route (`Lovelace.Run <file.ls>`, documented in `--help`) is
   byte-identical to `--file`, BOM included (2 runs).
2. **Value agreement inside one envelope.** For 30+ values (`result.display` vs
   `variables[v].display` vs `variables[_].display` vs the `print` line in `output[]` vs
   `result.structured.value` vs `variables[*].structured`), all carriers are **character-for-character
   equal**, including at 1 100 digits: `setprecision(1100); v = pi(1100)` → every carrier 1 102 chars;
   `evalf(pi, 1000)` → every display carrier 102 chars and every structured carrier 1 002 chars
   (the 100-vs-1000 split is the documented `Real.DisplayDecimalPlaces` bound, §"Documented bounds").
   78 value × precision combinations: the display string is always an exact prefix of the structured
   string, 0 content mismatches.
3. **Ground truth.** 7 high-precision results (`sqrt(2)`, `pi(1000)`, `evalf(sqrt(2),300)`,
   `1/(3*10^1000)`, `e(500)`, `evalf(sinh(34/3),30)`, `evalf(cos(pi(30)),40)`) compared digit by
   digit against mpmath at 1 300 dps (the binary's digits agree; the tiny value's first significant
   digit is at decimal place 1001, as mpmath puts it). No wrong value found.
4. **Error envelopes.** 13 failure classes (parse: unterminated string / junk char / stray `)`;
   runtime: undefined variable, wrong shape, arity, division by zero, unsupported power, bad `evalf`
   digit count, nesting depth, refused allocation, cancellation, `assume(1)`) × 3 surfaces × 2 passes:
   same `code`, `category`, `message`, `diagnostics` position/line/column, `timings`, `output`,
   exit code, 0 bytes of stderr. The only difference in the whole matrix is the wall-clock numbers the
   cancellation diagnostic embeds in its prose (expected).
5. **Positions recomputed independently.** For LF/CRLF/CR × BOM/no-BOM × 3 surfaces, every
   `timings[].position` equals the true statement offset in the text *that surface received*, and every
   `diagnostics[0]` position/line/column equals the recomputed coordinates of the failing statement
   (18 combinations, all exact — this is audit E F1/F2/F3, closed by EVD-293, re-measured).
6. **`capabilities()`.** 19 advertised `unsupported_operations` entries: every `trigger` reproduces the
   advertised `code`, `category` and `message` (record-carried entries carry theirs in the result
   record's `diagnostics`); both `solve_domains_refused` entries refuse. The two `scope` prose claims
   were falsified call by call: all 5 listed positive-base failures and all 5 listed negative-base
   failures refuse, and all 5 listed live counterexamples answer exactly as written
   (`(-4)^(1/2)` = `2*i`, `(-9)^(1/2)` = `3*i`, `(-2)^(1/2)` = `i*sqrt(2)`, `0^(1/2)` = `0`).
7. **Golden fixtures.** All 20 `Lovelace.Run.Tests/fixtures/*.ls` replayed through the published binary
   as `--file <name>.ls --omit-functions` and compared to `<name>.json` under the tests' own rule
   (`revision`/`elapsed`/`elapsedTime`/`timings` → `"<volatile>"`, key order irrelevant): **20/20 MATCH,
   twice** (`TestSupport.cs:20`, `GoldenEnvelopeTests.cs:17-42`). The same 20 scripts are also identical
   through all three surfaces.
8. **Plot artifact.** `plot([1, 2, 3])` with `--plot-dir`: `plot.path` names the file that exists, and
   the file's bytes equal `plot.svg` exactly (14 636 B), for the default name and for `--plot-file "my
   plot.svg"`.
9. **Invariant 1 on both paths.** A failing run carries the captured `output` (`["hello"]`) and the
   cancelled run carries `output` **and** `partialOutput`, equal to each other; `timings[].hasOutput`
   matches which statements printed (audit E F5, closed by EVD-294, re-measured).

---

## G-1 — **P2** — the documentation block that explains `capabilities()` still describes three fixed defects as live bugs

### Exact command (2 runs, byte-identical apart from durations)

~~~powershell
& out\aot\Lovelace.Run.exe --eval 'mean(1)' --json --omit-functions
& out\aot\Lovelace.Run.exe --eval 'max(1,2)' --json --omit-functions
& out\aot\Lovelace.Run.exe --eval 'sum(x, 5)' --json --omit-functions   # x = symbol("x")
& out\aot\Lovelace.Run.exe --eval ('(' * 3000 + '1' + ')' * 3000) --json --omit-functions
~~~

(The two scripts containing `"` were passed through an argument array; the quoted form above is the
PS-safe equivalent for the last two.)

### Observed (verbatim, trimmed to the deciding fields)

~~~json
{"ok":false,"code":"InvalidArgument","category":"TypeMismatch","message":"mean(): argument 1 must be an array or vector; got Natural.","recoverable":true,"diagnostics":[{"message":"…","position":0,"line":1,"column":1}],"timings":[{"position":0,…}],"output":[]}
{"ok":false,"code":"InvalidArgument","category":"TypeMismatch","message":"max(): argument 1 must be an array or vector; got Natural.",…}
{"ok":false,"code":"InvalidArgument","category":"TypeMismatch","message":"sum(): argument 1 must be an array or vector; got Symbolic.",…}
{"ok":false,"code":"DepthExceeded","category":"BudgetExceeded","message":"expression nesting depth 257 exceeds the maximum supported nesting depth of 256. …","diagnostics":[{"message":"…","position":0,"line":1,"column":1}],…}
~~~

All four exit 1 with a well-formed envelope on stdout and **0 bytes on stderr**.

### The stale text

`Lovelace.Symbolics/SymbolicsPlugin.cs:824-829` — the `<list>` that documents exactly what the
`capabilities()` payload does **not** enumerate — still says:

> `/// <item>the caller-side argument errors that are still BUGS rather than capabilities:`
> `/// <c>mean(1)</c>, <c>max(1,2)</c> and <c>sum(x, 5)</c> surface as`
> `/// <c>InternalError</c>/<c>InternalInvariantFailure</c> carrying the raw CLR message "Specified`
> `/// cast is not valid." (audit finding F4), and a 3000-deep script overflows the native stack and`
> `/// emits no envelope at all (F11). Advertising any of them would claim a capability boundary`
> `/// where the truth is a defect; they stay unadvertised until they are fixed or typed.</item>`

### What the correct behaviour is, and how I know

* The four live behaviours above are the correct ones: three typed `InvalidArgument`/`TypeMismatch`
  refusals naming the builtin, and a typed `DepthExceeded`/`BudgetExceeded` refusal — exactly the
  taxonomy `docs/symbolics/dsh-protocol.md:207-209` declares, with an envelope on stdout in all four
  cases.
* The defects the comment describes were closed in the cycle-6 audit-response rounds (EVD-283: the
  argument-shape class swept from 52 internal failures across 26 builtins to 0; EVD-262: recursion and
  nesting refusals are typed). **This item is not a re-discovery of those defects** — they are closed
  and stay closed; it is the comment that was never updated, so the file that owns the published
  capability statement now misdescribes the product in the direction of "these are still defects".
* Severity is P2 because it is prose in a source file, not a field of the published envelope: no
  consumer of the binary reads it. It is reported because this document is the one the next auditor will
  read to decide what is a bound and what is a defect.

### New?

**NEW.** EVD-277…EVD-295, P-1…P-17 and the P.2/P.3 lists contain no item about the comment; the audits
that closed the behaviour (round 13/14) recorded the fix, not the stale text. Not a fix proposal — the
observation is that the record and the product disagree.

---

## G-2 — **P1** — the leading BOM makes the same bytes report different positions on different surfaces (**recorded**, round-19 §8.4 — not new)

### Exact command (PowerShell 5.1; run twice, identical output; the fuzz corpus hit it 44 times)

~~~powershell
$dir = 'C:\Windows\Temp\audit-g-r20'
$nl  = [string][char]10
$bom = [byte[]](0xEF,0xBB,0xBF) + [Text.Encoding]::UTF8.GetBytes('a = 1' + $nl + 'b = 2' + $nl + 'det(1)')
[IO.File]::WriteAllBytes($dir + '\bom3.ls', $bom)
& out\aot\Lovelace.Run.exe --file ($dir + '\bom3.ls') --json --omit-functions
cmd /c ('""C:\Users\ricar\dev\LovelaceSharp\out\aot\Lovelace.Run.exe" --stdin --json --omit-functions < ' + $dir + '\bom3.ls"')
& out\aot\Lovelace.Run.exe --eval ([Text.Encoding]::UTF8.GetString($bom)) --json --omit-functions
~~~

### Observed (verbatim, trimmed to the deciding fields; second run identical)

| surface | `diagnostics[0]` | `timings[].position` |
|---|---|---|
| `--file` | `{"message":"det(): argument 1 must be a square matrix; got Natural.","position":12,"line":3,"column":1}` | `[0, 6, 12]` |
| `--stdin` | same message, `"position":13,"line":3,"column":1` | `[1, 7, 13]` |
| `--eval` | same message, `"position":13,"line":3,"column":1` | `[1, 7, 13]` |

All three exit 1, same code/category, 0 bytes stderr. The same +1 split appears in the BOM+CRLF form
(`--eval` 46 vs `--file` 45), in the parse path (`--eval` 46 vs `--file` 45 for the 13-byte CRLF file
plus a BOM), and in 44 of the 150 fuzz scripts (each a BOM'd script whose envelope carried a
position).

### What the correct behaviour is, and how I know

* The brief's own claim is that one script through the three surfaces produces "the same positions"; a
  byte-identical input does not.
* Each route is *internally* consistent: `--eval`/`--stdin` publish offsets into the text as received
  (BOM included; `ScriptPositions.cs:74-86` skips a leading U+FEFF while indexing the caller's text),
  and `--file` publishes offsets into what `File.ReadAllTextAsync` decoded — .NET strips the BOM as an
  encoding marker (`Runner.cs:143`). So no single field is "wrong" against its own text; the divergence
  is the finding.
* **Recorded**: `docs/goal-cycle-6/round-19/positions-implementation.md:546-549` states it —
  "BOM routes differ by construction … only the `--eval` route is pinned by a test". It is not in
  P-1…P-17 and not in the P.2 list at `a-plus-cycle-6-amendment.md:43`, so it is live and unowned.
* Graded **P1** (a published machine field disagrees with itself across transports for identical bytes);
  a reader may grade it P2 on the grounds that both routes are self-consistent — stated so the parent
  can re-grade.

### New?

**RECORDED, NOT NEW** (round-19 §8.4, measured there on the Release `Lovelace.Run.dll`). Reproduced
here on the published AOT binary.

---

## G-3 — **P2** — a parse error's `message` carries the engine's unmapped offset while its own `position` carries the mapped one (**recorded**, round-19 §8.1 — not new)

### Exact command (2 runs, identical)

~~~powershell
# 13 bytes: 31 2B 31 3B 0D 0A 32 2B 32 3B 0D 0A 29   ("1+1;\r\n2+2;\r\n)")
& out\aot\Lovelace.Run.exe --file C:\Windows\Temp\audit-g-r20\crlfparse.ls --json --omit-functions
~~~

### Observed (verbatim, trimmed)

~~~json
{"ok":false,"code":"ParseError","category":"ParseError",
 "message":"Unexpected token ')' at position 10: expected a number, string, identifier, '[', or '('.",
 "diagnostics":[{"message":"Unexpected token ')' at position 10: …","position":12,"line":3,"column":1}],"timings":[],"output":[]}
~~~

The offending `)` is at offset **12** of the file (line 3, column 1). `position` says 12; the prose in
the same object says 10.

### What the correct behaviour is, and how I know

* `dsh-protocol.md:225-228`: the error envelope's diagnostics are "the parser's source-position form".
  Two numbers in one diagnostic naming two different places is a self-contradiction, and the prose one
  is the *unmapped* engine offset — precisely the class audit E F3 closed for the numeric fields.
* **Recorded**: `positions-implementation.md:534-538` (§8.1) states it and says rewriting the prose
  "would be a new contract on the message text, so it was not done". Graded P2 because the machine
  fields are right and only the human prose carries the old number; the record already owns it.

### New?

**RECORDED, NOT NEW** (round-19 §8.1). Reproduced twice on the published binary.

---

## G-4 — **P2** — `--omit-variables` is ignored on the cancelled path (**recorded**, round-18 F4 / amendment `:43` — not new)

### Exact command (2 runs each; `cancel.ls` = `x = symbol("x"); y = 42; print("hello"); sum(1..50000000)`, outside the repo)

~~~powershell
& out\aot\Lovelace.Run.exe --file C:\Windows\Temp\audit-g-r20\cancel.ls --json --omit-functions --cancel-after 200
& out\aot\Lovelace.Run.exe --file C:\Windows\Temp\audit-g-r20\cancel.ls --json --omit-functions --cancel-after 200 --omit-variables
~~~

### Observed (verbatim, trimmed to the deciding fields)

Both envelopes:

~~~json
"code":"Cancelled","category":"BudgetExceeded",
"output":["hello"], "partialOutput":["hello"],
"partialVariables":[{"name":"x","kind":"Symbolic","display":"x","structured":{…}},
                    {"name":"y","kind":"Natural","display":"42","structured":{…}}],
"cancellation":{"budgetMs":200,"elapsedMs":…,"stopped":true,"exceeded":true,"excessMs":…}
~~~

`--omit-variables` changes nothing (`partialVariables` keeps both entries with their full `structured`
projection). `variables` is absent on this path either way.

### What the correct behaviour is, and how I know

* `Runner.cs:653` documents the flag as "omit the variables array from the envelope (agent loops)", and
  `docs/symbolics/a-plus-cycle-3-alignment.md:374` states its scope as "any run | variable list empty,
  envelope otherwise unchanged"; the cancelled path builds `partialVariables` without consulting
  `omitVariables` (`Runner.cs:302-308` reads `partialVariables` off `CaptureState()`; the flag is
  consulted only on the success path at `Runner.cs:218-224`), while its sibling `--print-budget` *is*
  honoured there.
* **Recorded**: round-18 `audit-E-cli.md:204` (F4) and `a-plus-cycle-6-amendment.md:43` (open P2).

### New?

**RECORDED, NOT NEW.**

---

## G-5 — **P2** — `--text` drops the captured print output on the failing path while `--json` carries it (**recorded**, round-18 F8 — not new)

### Exact command (2 runs; `failprint.ls` = `x = symbol("x"); print("hello"); det(1)`)

~~~powershell
& out\aot\Lovelace.Run.exe --file C:\Windows\Temp\audit-g-r20\failprint.ls --json --omit-functions
& out\aot\Lovelace.Run.exe --file C:\Windows\Temp\audit-g-r20\failprint.ls --text --omit-functions
~~~

### Observed (verbatim, trimmed)

~~~json
JSON: {"ok":false,"code":"InvalidArgument","category":"TypeMismatch",
       "message":"det(): argument 1 must be a square matrix; got Natural.",
       "output":["hello"], "diagnostics":[{"position":33,"line":1,"column":34}], …}
TEXT: stdout 0 bytes; stderr = "Error [InvalidArgument/TypeMismatch]: det(): argument 1 must be a square matrix; got Natural."
~~~

### What the correct behaviour is, and how I know

* `dsh-protocol.md:9-10` (invariant 1) is not scoped to a successful run, and the JSON surface honours
  it — the same binary's two output surfaces therefore disagree about the same run.
* **Recorded**: round-18 `audit-E-cli.md:382` (F8) and `a-plus-cycle-6-amendment.md:43`, which adds
  "a deviation its own `--help` documents". **That parenthetical does not match the binary**: the
  printed help's only `--text` line is `--text               emit a human-readable summary` (observed from
  `out\aot\Lovelace.Run.exe --help`, exit 0). The deviation is real; the claim that `--help` documents
  it is not visible in the help.

### New?

**RECORDED, NOT NEW** (F8). The `--help`-documentation sub-point is an observation about the record,
not a new product finding.

---

## G-6 — **P2** — `pi(n)`/`e(n)` under a smaller ambient precision leaks a raw .NET message (**recorded**, amendment `:43` — not new)

### Exact command (2 runs, identical)

~~~powershell
& out\aot\Lovelace.Run.exe --eval 'setprecision(20); pi(30)' --json --omit-functions
& out\aot\Lovelace.Run.exe --eval 'setprecision(20); e(30)'  --json --omit-functions
~~~

### Observed (verbatim, trimmed)

~~~json
{"ok":false,"code":"InvalidArgument","category":"TypeMismatch",
 "message":"Specified argument was out of the range of valid values. (Parameter 'digits')",
 "diagnostics":[{"message":"Specified argument was out of the range of valid values. (Parameter 'digits')","position":0,"line":1,"column":1}], …}
~~~

Code and category are typed; the message is the CLR's own and names neither the builtin nor the
requested/available counts (contrast `evalf(sin(1), 0)`, which names the argument and the range).

### What the correct behaviour is, and how I know

* `dsh-protocol.md:207-223`: refusal messages name the builtin and both counts ("the counts come from
  the builtin's declared metadata"). Recorded in `a-plus-cycle-6-amendment.md:43` as an open P2
  ("`pi(30)` at `setprecision(20)` leaks a raw .NET message"). Consistent across all three surfaces
  (checked).

### New?

**RECORDED, NOT NEW.**

---

## G-7 — **P2** — the protocol document's own error example is stale: it pins `position 0` for a failure the binary now places at 17 (**recorded**, round-19 §8.2 — not new)

### Evidence

* `docs/symbolics/dsh-protocol.md:189-199` shows `"diagnostics":[{"position":0,"line":1,"column":1}]`
  next to `"timings":[{"position":0,…},{"position":17,…}]` for the `solve(…, integer)` failure.
* The binary's golden fixture for that script now pins `"position": 17, "column": 18`
  (`Lovelace.Run.Tests/fixtures/error_envelope.json:13-15`), and the binary reproduces the fixture:
  `& out\aot\Lovelace.Run.exe --file Lovelace.Run.Tests\fixtures\error_envelope.ls --omit-functions`
  → `"diagnostics":[{"message":"solve(): currently supports domains real and complex; got integer.","position":17,"line":1,"column":18}]`, exit 1, matched by the 20/20 fixture replay.
* **Recorded**: `positions-implementation.md:539-541` (§8.2) — "the protocol document's own error
  example is now stale … left for its owner". Still stale on this tree.

### New?

**RECORDED, NOT NEW.**

---

## Documented bounds confirmed — explicitly **not** counted as findings

* **`display` is 100 decimals while `structured` carries the stored digits** (`evalf(pi,1000)`:
  display 102 chars, structured 1 002; `sqrt(2)`: 102 vs 1 002; `setprecision(1100); pi(1100)`:
  1 102 vs 1 102). The bound is documented — `Lovelace.Suite/docs/Language.md:750-755`: "Raises
  `Real.MaxComputationDecimalPlaces` (… default 1000) and `Real.DisplayDecimalPlaces` (how many
  fractional digits `ToString()` emits for non-periodic values, default 100)" — and is the deliberate
  outcome of P-15 (`a-plus-cycle-6-amendment.md:33`). The digits the two share are identical, so this
  is a documented display bound, not a lost or altered value.
* **CR, CRLF and a BOM are normalised/stripped before the engine sees them** (`ScriptSource.cs:27-32`),
  including inside a string literal: the file whose bytes are `78 20 3D 20 22 41 0D 0A 42 22`
  (`x = "A\r\nB"`) crosses as `{"kind":"Text","value":"A\nB"}` on all three surfaces. Round-18 F3
  recorded this limb and called it "arguably the documented normalisation"; `ScriptSource.cs:15-17`
  states the normalisation as the method's purpose. Consistent across surfaces.
* **`typed` suffix rules**: `Text` has no suffix, `Enum` has no suffix, `Record` uses the record type
  name. All three are pinned by the project's own tests and documents
  (`Lovelace.Suite.Tests/EnumPayloadSeamTests.cs:62`, `ModusTests.cs:86`,
  `docs/symbolics/dsh-protocol.md:113`); `result.kind`/`variables[_].kind` agree with
  `structured.kind`/`structured.type` in every probe.
* **`capabilities()` completeness carve-outs** are documented in the payload's own header
  (`SymbolicsPlugin.cs:798-829`) — the record is "exhaustive over the refusal classes the round-09
  audit exhibited, and explicitly not exhaustive over the runtime's whole error taxonomy". I verified
  the listed set (19/19) and its scope claims, which is what the brief asks; I did not treat the
  documented carve-outs as under-claiming.

---

## Matched a recorded item — not counted as new

| matched | where it is recorded | my re-measurement |
|---|---|---|
| Audit E F1 — runtime failures all reported `position 0/line 1/column 1` | closed by EVD-293 (`63774da`) | **closed**: 4-statement LF/CRLF/CR script × 3 surfaces → `24/4/1`, `27/4/1`, `24/4/1`, `timings` equal to the true offsets |
| Audit E F2 — `line`/`column` computed on the rewritten source (always line 1) | closed by EVD-293 | **closed**: parse failure on line 3 → `10/3/1` (LF), `12/3/1` (13-byte CRLF file) |
| Audit E F3 — `timings[].position` shifted by CRLF | closed by EVD-293 | **closed**: CRLF `[0,7,14,27]`, CR `[0,6,12,24]`, BOM variants shifted only by the BOM (G-2) |
| Audit E F5 — failing run drops `print` output | closed by EVD-294 (`6424e27`) | **closed**: `--json` error envelope carries `"output":["hello"]` |
| Audit E F6 — `ParseError` on the wrong layer | closed by EVD-294 | **closed**: `1+1;` ⏎ `~` → `ParseError/ParseError` (was `InvalidOperation/DomainError`); missing file → `FileReadError` |
| Round-19 §8.1 / §8.2 / §8.4 residuals | `positions-implementation.md:534-549` | **still live**: G-3, G-7, G-2 |
| Round-18 F4 / F8 and the amendment's P2 list | `audit-E-cli.md:204`, `:382`; `amendment.md:43` | **still live**: G-4, G-5; also G-6 |
| Round-18 F7 — `--print-budget` non-monotone | `audit-E-cli.md:347`; `amendment.md:43` | **not reproduced**: one expression family × budgets 1/2/3/5/10/40/1000 gave pretty lengths 3/6/7/10/10/10/10 (monotone). Not counted either way. |
| O-B11 / nesting depth | closed by EVD-262, EVD-254 | **closed**: 500/1000/2000/3000-deep parens and arrays all `DepthExceeded/BudgetExceeded`, exit 1, 0 bytes stderr |

---

## Table

| id | severity | new? | one-line repro |
|---|---|---|---|
| G-1 | P2 | **NEW** | `SymbolicsPlugin.cs:824-829` still calls `mean(1)`/`max(1,2)`/`sum(x,5)` `InternalError` and a 3000-deep script a stack overflow; all four are typed refusals on the binary |
| G-2 | P1 | no — recorded (round-19 §8.4) | BOM'd `a = 1\nb = 2\ndet(1)`: `--file` → `position 12`, `--stdin`/`--eval` → `13`; `timings` `[0,6,12]` vs `[1,7,13]` |
| G-3 | P2 | no — recorded (round-19 §8.1) | CRLF `1+1;\r\n2+2;\r\n)`: message says "at position 10", `diagnostics[].position` says 12 |
| G-4 | P2 | no — recorded (round-18 F4; amendment :43) | `--omit-variables` on a cancelled run leaves `partialVariables` with `x` and `y` and their `structured` forms |
| G-5 | P2 | no — recorded (round-18 F8) | `print("hello"); det(1)`: `--json` carries `output:["hello"]`, `--text` writes 0 bytes to stdout and only the error to stderr |
| G-6 | P2 | no — recorded (amendment :43) | `setprecision(20); pi(30)` → `"Specified argument was out of the range of valid values. (Parameter 'digits')"` |
| G-7 | P2 | no — recorded (round-19 §8.2) | `dsh-protocol.md:194` still shows `position: 0` where the fixture and the binary answer `17` |

---

## Could not check

* **The repository's own test suites / in-process runner.** No `dotnet` build or `dotnet test` was run
  (the brief's target is the published binary); the fixtures were replayed through the AOT binary instead
  of through `TestSupport.RunFixtureAsync`, so "the goldens match the tests' expectation" is asserted
  only by my independent comparison, not by the suite itself.
* **The SymPy differential oracle.** Ground truth was mpmath 1.3.0 at 1 300 dps driven directly
  (`C:\Users\ricar\dev\.lovelace-tools\python`), not the repo's oracle harness; 7 values checked, no
  full-corpus oracle run.
* **Other operating systems.** Every probe is Windows; the Linux CI jobs' behaviour is not measured here.
* **UTF-16 / non-UTF-8 script files across surfaces.** `--file` decodes by BOM/encoding detection while
  `--stdin` decodes bytes through the console input encoding, so the three surfaces are not given the
  same *text* by construction; I did not compare them (audit E covered a non-UTF-8 file as a parse
  refusal on `--file`).
* **`functions[]` / registry metadata** (omitted by `--omit-functions` throughout, per the brief) and
  the `--help`/descriptor prose versus live metadata.
* **Hostile flag values, locked files, huge scripts (`--plot-dir`, `--print-budget` extremes)** — the
  ground of round-18 audit E; not re-run except where an item above needed it.
* **Wall-clock claims** (cancellation promptness, benchmark spread). Cross-surface envelopes differ only
  in the measured durations the cancellation message embeds; that is expected and was not treated as a
  divergence.
