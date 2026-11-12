# Round 34 — DEF-004 closed; J-03/J-09/J-10/J-11/J-12 certified

Status: **five journeys PASS in a writable app-scoped plot directory** (`--journey` summary:
`journeys=5 failed=0`, exit 0). The four window journeys — J-03, J-10, J-11 and J-12 — also pass
in the real product window: 58/58 window checks, 0 failed (`window-proof.json`). The suite
`cargo test -p lovelace-desktop` is **355 passed, 0 failed**.

One environmental result is recorded honestly: with the machine's **default** `%APPDATA%`, this
sandbox does not let the product process write outside the workspace, so J-09's control run fails
at the engine's plot-file write (not at the copy path DEF-004 fixed). The exact command, check and
diagnostic are in §3. With `APPDATA` outside the process cwd (the documented capture requirement)
J-09 passes, and that run uses **no `--out`**, so the per-run output directory it now creates itself
is what the DEF-004 fix is about.

## 1. What changed

| File | Change |
|---|---|
| [`src/journey.rs`](../../../rust/lovelace-desktop/src/journey.rs) | DEF-004: the output directory is created whenever a selected journey writes into it — `J-09` (plot copies) as well as `J-15` (raster/SVG exports). `J-02/J-03/J-10` still run without touching the temp directory. A `needs_output_dir` predicate and its unit test pin the two artifact journeys. No expectation was changed. |

The only product change in this round is that predicate; the journey functions, expected values and
assertions are unchanged. The desktop unit tests (including the `journey::tests` vectors, mismatch
and option tests) remain green.

## 2. `--journey` suite summary

Command (from `round-34\work`, `APPDATA=round-34\appdata-journey`, `TEMP=rust\target-desktop\tmp`):

```powershell
lovelace-desktop.exe --journey J-02 --journey J-03 --journey J-09 --journey J-10 --journey J-15 `
    --out C:\Users\ricar\dev\LovelaceSharp\docs\goal-desktop-v4\round-34\journey-out
# JOURNEY summary journeys=5 failed=0 ; exit 0
```

Script: [`run-journeys-round34.ps1`](run-journeys-round34.ps1) reproduces all three runs;
unedited stdout is [`journey-run.log`](journey-run.log).

| ID | `--journey` verdict | Key observed |
|---|---|---|
| J-02 | **PASS** | result `1024`; variables `a=2`, `b=1024` Natural; rev 1; 3/3 committed |
| J-03 | **PASS** | at precision 20: `1.4142135623730950488` (19 fractional digits); at 1000: 1000 digits, `sha256=92fa8c84…36b63c`; run 2 re-rendered after the change, `reused=0` |
| J-09 | **PASS** | each run output exactly `["alpha", "beta"]` (one each); stdout log lines 2 then 4; two fresh 15 104-byte SVGs, `sha256=6a80a443…82de35`, each equal to the engine file; copies written under the **default** per-run output directory (run-29404) |
| J-10 | **PASS** | before: result `42`, `y=41`, `f(x) origin=user`; after Clear Variables: 0 variables, `f(x) origin=user` retained, 124 functions retained, log `Variables cleared: 2 removed; 124 functions retained.` |
| J-15 | **PASS** | 15 000-byte SVG, census 2/13/11/1, `sha256=0b23d6f7…1406c`; zoom/pan/fit; 800×600 + 1600×1200 rasters; byte-identical export |

Artifacts: [J-09-run1.svg](journey-out/J-09-run1.svg), [J-09-run2.svg](journey-out/J-09-run2.svg),
[J-15-export.svg](journey-out/J-15-export.svg), [J-15-plot-2x.png](journey-out/J-15-plot-2x.png).

## 3. DEF-004 — J-09's result after the fix

### 3.1 The bug and the fix

Before the fix, `--journey J-09` copied each run's SVG to
`%TEMP%\lovelace-journey\run-<pid>\J-09-run<n>.svg` but only `J-15` created that directory. With a
writable app-scoped plot path the run therefore failed with:

```
JOURNEY J-09 FAIL J-09: cannot write ...\lovelace-journey\run-<pid>\J-09-run1.svg:
The system cannot find the path specified. (os error 3)
```

`needs_out` now includes `J-09`, so the directory is created before the first evaluation. The
default-environment control run below shows the *engine plot write* failing for an environmental
reason, not the copy path.

### 3.2 Default environment (no APPDATA redirect)

Command: `lovelace-desktop.exe --journey J-09` from `round-34\work`; `TMP` is
`rust\target-desktop\tmp`; `APPDATA=C:\Users\ricar\AppData\Roaming`; no `--out`.

Observed exit **1**; unedited log [`journey-default-env.log`](journey-default-env.log):

```
JOURNEY session lovelace-desktop 0.1.0 engine=lovelace-lang plot-dir=C:\Users\ricar\AppData\Roaming\Lovelace\plots
JOURNEY J-09 observed input="print(\"alpha\")\nplot(1..10, (1..10)^2, \"side effects\")\nprint(\"beta\")\n"
JOURNEY J-09 FAIL J-09 run 1: terminal state Failed, expected Finished
  (line 2, column 1: Cannot write the plot file
   'C:\Users\ricar\AppData\Roaming\Lovelace\plots\run-1\plot.svg':
   Access to the path '...\run-1\plot.svg' is denied.)
JOURNEY summary journeys=1 failed=1
```

**Exactly which check fails and why:** `J-09 run 1` never reaches its print/plot assertions; the
run's terminal state is `Failed` at statement 2 because the engine cannot write
`<default APPDATA>\Lovelace\plots\run-1\plot.svg`. In this delegated sandbox, the product process
is denied writes outside the workspace (the same denial makes a minimal `rustc` link fail with
`LNK1104` on `%TEMP%\lnk….tmp`; PowerShell itself can write the directory). The product process
writes the same path successfully under the workspace, so the app-scoped plot path itself is
correct; it is the sandbox that makes the *default* location unwritable. This is not the DEF-004
copy failure.

### 3.3 Writable app-scoped plot path (no `--out`)

Command: `APPDATA=round-34\appdata-journey`, cwd `round-34\work`, no `--out`; unedited log
[`journey-j09-default-out.log`](journey-j09-default-out.log). Observed exit **0**, `J-09 PASS`:

```
run 1 output=["alpha", "beta"] stdout-log-lines=2
      plot="...\appdata-journey\Lovelace\plots\run-1\plot.svg"
      copy="...\rust\target-desktop\tmp\lovelace-journey\run-29404\J-09-run1.svg"
      bytes=15104 sha256=6a80a4430f13f8a80d73a9caa06562a47e1a29d53baf2b039109b58da482de35
run 2 output=["alpha", "beta"] stdout-log-lines=4
      plot="...\appdata-journey\Lovelace\plots\run-2\plot.svg"
      copy="...\lovelace-journey\run-29404\J-09-run2.svg"
      bytes=15104 sha256=6a80a4430f13f8a80d73a9caa06562a47e1a29d53baf2b039109b58da482de35
JOURNEY J-09 PASS
JOURNEY summary journeys=1 failed=0
```

All eight J-09 checks are `ok` (two print statements, no duplicate line, plot on disk equal to the
capture, stdout log counts, fresh path per run, identical deterministic rendering).

## 4. Window certification — J-03, J-10, J-11, J-12

Driver: [`capture-round34.ps1`](capture-round34.ps1). The final run was on the **locked** console
(foreground `Windows Default Lock Screen`), so it used only locked-session input paths:
UIA **Invoke** for Run/Clear Variables/Save/Quit, the precision spinner's UIA **RangeValue**,
window messages (`WM_KEYDOWN`/`WM_CHAR`/`WM_KEYUP`) for `sq`, Enter and F8, and
**PrintWindow(PW_RENDERFULLCONTENT)** into 1600×1000 32bpp PNGs. Each journey's app started in
`round-34\work` with `APPDATA` redirected to `round-34\appdata\<journey>` (outside the cwd), so the
app-scoped plot directory is writable and the product's own startup/probe path runs.
`window-proof.json` records 58 checks, **58 passed, 0 failed**, plus the session, executable hash,
capture sizes and SHA-256s.

### Per-journey table

| ID | Input | Expected (journeys.md / REF) | Observed in the window | Artifacts | APPDATA | Result |
|---|---|---|---|---|---|---|
| J-03 | Full: `setprecision(20)`; `sqrt(2)`; `setprecision(1000)`; `sqrt(2)`. Precision control 1000→20→1000; Run, Run. | Visible precision changes; full stored digits at each precision; precision change invalidates the cached result | Toolbar/status **precision 20** after the first change (with `20 digits`), then **precision 1000** after the second; run 1 logs `Run started: 4 statements … precision 20.`, run 2 `precision 1000.`; each run's final `[result]` is the 1000-digit value, 1000 fractional digits, `sha256=92fa8c84…36b63c`; variables table `_ = Real 1.4142135…`; 20-place document logs `[result] = 1.4142135623730950488`, **19 fractional digits**, variables `_ = Real 1.4142135623730950488` | [J-03.png](J-03.png), [J-03-uia.txt](J-03-uia.txt), [J-03-precision-20.png](J-03-precision-20.png), [J-03-precision-20-uia.txt](J-03-precision-20-uia.txt), [J-03-20-digits.png](J-03-20-digits.png), [J-03-20-digits-uia.txt](J-03-20-digits-uia.txt); fixtures [J-03.ls](J-03.ls), [J-03-20.ls](J-03-20.ls) | redirected to `appdata\J-03` (and `appdata\J-03-20`) | **PASS** |
| J-09 | `print("alpha")`; `plot(1..10, (1..10)^2, "side effects")`; `print("beta")`, twice | Each run executes its effects; no duplicate print line within a run | Headless run (§3.3): each run `["alpha","beta"]` once; stdout log lines 2 then 4; two fresh 15 104-byte plots, identical SHA-256; copy directory created by the fix. Default-env control (§3.2) fails at the engine write because this sandbox denies child writes to the default APPDATA. | [journey-j09-default-out.log](journey-j09-default-out.log), [journey-default-env.log](journey-default-env.log), [J-09-run1.svg](journey-out/J-09-run1.svg), [J-09-run2.svg](journey-out/J-09-run2.svg) | default env: **not writable** (control fails); writable run: redirected to `appdata-journey` | **PASS** (writable plot dir); default-env control FAIL is environmental and is named |
| J-10 | `func f(x) = x + 1`; `y = 41`; `f(y)`; Run, Clear Variables | `y` removed, `f` retained | Before: result `42`, `VARIABLES · 2` (`_=42`, `y=41`), `FUNCTIONS · 124`. After: `VARIABLES · 0` with “The last run defined no variables.”, no `y` row; `FUNCTIONS · 124` unchanged and the row `f` with parameter `x` and origin `user` is present ([J-10-uia.txt:257](J-10-uia.txt#L257)); log `[info] Variables cleared: 2 removed; 124 functions retained.`; result pane still `42` | [J-10-before.png](J-10-before.png), [J-10-before-uia.txt](J-10-before-uia.txt), [J-10.png](J-10.png), [J-10-uia.txt](J-10-uia.txt), fixture [J-10.ls](J-10.ls) | redirected to `appdata\J-10` | **PASS** |
| J-11 | Type `sq` in an empty document; observe completion; accept it | `sqrt` with its signature; accepting inserts the completion | Editor caret after typing: line 1 col 3; pane header `Completion: sq -> sqrt(x) [builtin] 1/2` ([J-11-completion-uia.txt:41](J-11-completion-uia.txt#L41)); popup paints `sqrt` with detail `sqrt(x)` (and the second match `squeeze`); Enter accepts: caret line 1 col 5, popup gone; Save writes exactly `sqrt` (4 bytes) | [J-11-completion.png](J-11-completion.png), [J-11-completion-uia.txt](J-11-completion-uia.txt), [J-11-accepted.png](J-11-accepted.png), [J-11-accepted-uia.txt](J-11-accepted-uia.txt), [J-11-inserted.txt](J-11-inserted.txt), fixture [J-11.ls](J-11.ls) | redirected to `appdata\J-11` | **PASS** |
| J-12 | Multiline source: `a = 1`; `s = "😀"; 1 / 0`; `b = 2` (astral character before the failing column) | Exact line/column highlighted, code/category readable, jump to source | Run fails; header `Diagnostic Ln 2, Col 11: Cannot divide by zero.`; log `[diagnostic] Diagnostic: line 2, column 11: Cannot divide by zero.` ([J-12-uia.txt:586](J-12-uia.txt#L586)); underline painted at image box x=134..173, y=172..173 and gutter marker at x=19..22, y=159..172 (same second line; editor rect left 27, row height 17.4); F8 jumps the caret to **line 2, column 11** and the status strip reads `Ln 2, Col 11` | [J-12.png](J-12.png), [J-12-uia.txt](J-12-uia.txt), [J-12-jump.png](J-12-jump.png), [J-12-jump-uia.txt](J-12-jump-uia.txt), fixture [J-12.ls](J-12.ls) | redirected to `appdata\J-12` | **PASS** |

### Capture inventory (1600×1000 PNG, PrintWindow)

| File | Bytes | SHA-256 |
|---|---:|---|
| J-03-precision-20.png | 341 310 | `48802AF0A2E60FC507CF178A9798218462264B6E9CEE74C2BF1A29250AAFBBFE` |
| J-03.png | 342 083 | `CAAD41414F862C8670C83CB479C0CBEF50365C8A710D6E9E45C699262B68CFE4` |
| J-03-20-digits.png | 230 380 | `C90FCE6BF10A50E1C35BA5A719F67428F92A404DF21EEEB4BBAC19B523FF38F5` |
| J-10-before.png | 220 119 | `E75C302D73E358BCDE026D38EE536309BDBBE75BDE63820D3F8E3C3C2B7164B5` |
| J-10.png | 224 285 | `9BD437CABF030300757A9BF314856CC50BF331F268A4639471723AD68C3D5BCD` |
| J-11-completion.png | 185 408 | `246F563721258DDB3E14BCFF4E6F557E61E7E0CEF05EB040F4367C9123F74210` |
| J-11-accepted.png | 179 140 | `2266A79673E1D5131B0AE9984E08BFAFCFA06C07BB180CECA435FBDA53617CA7` |
| J-12.png | 229 598 | `AF6A0E01E324B38E6E588788981823ABCECF670ADAAEA19B4739FC1FE4277C39` |
| J-12-jump.png | 228 247 | `F2F24F6099FB5CFB348FAA6B54CD8666D1638EB48689555D2B3F66B0D15777D0` |

### Honest notes

- **J-03 and the Logs pane.** The window's Logs pane publishes only the run's **final** result
  (`LogKind::Result`), so the intermediate 20-place value inside the four-statement script is not a
  pane row. The 19-digit display is therefore also captured from the first two statements of the
  same journey input as its own document (`J-03-20.ls`, control precision 20); the full script run
  captures the 1000-digit final result and the precision-control change. The headless
  `--journey J-03` certifies both values inside one run via its `committed_results` (19 and 1000
  fractional digits, `reused=0`); no expectation was weakened.
- **J-09 default-APPDATA sandbox limitation.** The exact failure and the proof that it is a
  product-process write denial (not the DEF-004 copy path) are in §3.2. The fix itself is proven by
  the no-`--out` run in §3.3: the journey created `lovelace-journey\run-29404` and wrote both
  copies.
- **J-10 screenshot vs UIA.** The functions table is sorted alphabetically and `f` is below the
  visible fold in the 1600×1000 screenshot; the table header shows 124 functions retained and the
  authoritative `f` / `x` / `user` row is in the UIA dump (line 257).
- **J-11 typed input.** Because the final run was locked, `sq` was delivered with window messages
  (the driver logs `J-11 typing mode=postmessage`). The same driver focuses the window and uses
  SendKeys when the session is unlocked; either path goes to the product's editor and completion
  handler. The popup itself is a painter overlay; its state is exposed in the pane header, which is
  the UIA text quoted above, and in the PNG.
- **J-12 pixel boxes** are evidence that the underline and gutter marker were painted; the exact
  1-based diagnostic position is independently proven by the `Diagnostic Ln 2, Col 11` header, the
  log line, and the F8 caret/status readout. The astral `😀` is two UTF-16 code units, so the
  failing `1 / 0` starts at UTF-16 column 11 (char index 10, byte offset 12).

## 5. Test suite

```
cargo test -p lovelace-desktop --manifest-path rust/Cargo.toml
test result: ok. 355 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

Unedited output: [`cargo-test.log`](cargo-test.log). Command environment:
`RUSTUP_HOME=.toolchain\rustup`, `CARGO_HOME=.toolchain\cargo`,
`CARGO_TARGET_DIR=rust\target-desktop`, `TMP/TEMP=rust\target-desktop\tmp`.

## 6. Reproduce

```powershell
# `--journey` evidence (default-env control + J-09 no --out + five journeys)
powershell -NoProfile -ExecutionPolicy Bypass -File docs\goal-desktop-v4\round-34\run-journeys-round34.ps1

# Window artifacts (UIA Invoke + RangeValue + window-message keys + PrintWindow)
powershell -NoProfile -ExecutionPolicy Bypass -File docs\goal-desktop-v4\round-34\capture-round34.ps1

# Suite
cargo test -p lovelace-desktop --manifest-path rust\Cargo.toml
```
