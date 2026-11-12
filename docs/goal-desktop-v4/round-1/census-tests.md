# Desktop v4 — Round 1 Census: Tests and Platform Automation

**Scope:** `rust/lovelace-desktop/src/selftest.rs`, `rust/lovelace-desktop/Cargo.toml`, `rust/Cargo.toml`, `.github/workflows/`, `rust/lovelace-desktop/tests/`.

**Method:** every file in scope was read end to end. `cargo` was not run. A directory listing of `.github/workflows/` returned exactly one file (`ci.yml`, 414 lines); `rust/lovelace-desktop/tests/` does not exist.

**Citations:** paths are relative to the repo root. A line range cites the whole construct or the whole file, and is used for absence facts verified by reading every line of that file. Leading indentation is trimmed inside quotes; no quote otherwise alters tokens.

**No quality assessment and no recommendations are made below.**

## Census at a glance

| Item | Count | Evidence |
|---|---|---|
| GitHub Actions workflow files | 1 | `.github/workflows/` listing |
| CI jobs, all `ubuntu-latest` | 3 | ci.yml:24, 184, 257 |
| CI jobs that build or run the Rust desktop | 0 | ci.yml:1-414 |
| `--selftest` checks | 15 | selftest.rs:26 |
| `tests/` integration targets in `lovelace-desktop` | 0 | `tests/` absent; Cargo.toml:1-22 |
| GUI, screenshot, a11y, perf, or installer automation found for the desktop | 0 | selftest.rs:1-514; ci.yml:1-414 |

## 1. OBS table

Every row below is **High** confidence: either a line read directly, or an absence verified by reading every line of the cited file.

### 1a. Selftest harness facts

| ID | Source (file:line) | Fact | Confidence |
|---|---|---|---|
| OBS-H01 | rust/lovelace-desktop/src/selftest.rs:1 | `//! The --selftest acceptance mode: the whole application core, exercised without a window.` | High |
| OBS-H02 | rust/lovelace-desktop/src/selftest.rs:4 | `//! uses, then asserts on the pane models. Nothing here needs a message pump, a desktop session` | High |
| OBS-H03 | rust/lovelace-desktop/src/selftest.rs:16 | `type Check = fn() -> Result<(), String>;` — checks are plain functions returning `Result`. | High |
| OBS-H04 | rust/lovelace-desktop/src/selftest.rs:26 | `let checks: [(&str, Check); 15] = [` — the check list is a fixed-size array typed for 15 entries. | High |
| OBS-H05 | rust/lovelace-desktop/src/selftest.rs:20 | `let engine_name = crate::engine::default_engine_factory()().name();` — the engine name is resolved before checks run. | High |
| OBS-H06 | rust/lovelace-desktop/src/selftest.rs:22 | `"SELFTEST lovelace-desktop {} engine={}",` — header line prints crate version and engine name. | High |
| OBS-H07 | rust/lovelace-desktop/src/selftest.rs:47 | `Ok(()) => println!("SELFTEST ok {name}"),` | High |
| OBS-H08 | rust/lovelace-desktop/src/selftest.rs:50 | `println!("SELFTEST FAIL {name}: {reason}");` | High |
| OBS-H09 | rust/lovelace-desktop/src/selftest.rs:54 | `println!("SELFTEST summary checks={} failed={failed}", checks.len());` | High |
| OBS-H10 | rust/lovelace-desktop/src/selftest.rs:55 | `if failed == 0 { 0 } else { 1 }` — process exit code is 0 only when every check returned `Ok`. | High |
| OBS-H11 | rust/lovelace-desktop/src/selftest.rs:59 | `AppState::new()` — eleven of the fifteen checks obtain a fresh `AppState` via `app()`; the two settings checks, `svg-parse` and `no-network-imports` do not. | High |
| OBS-H12 | rust/lovelace-desktop/src/selftest.rs:63 | `let base = std::env::temp_dir().join(format!(` — file-writing checks use a per-PID scratch directory under the system temp dir, with a `target/selftest` fallback. | High |
| OBS-H13 | rust/lovelace-desktop/src/selftest.rs:235 | `if !app.finish_run(0, stale, 3).is_empty() {` — the cancel check calls the completion path directly with a stale `RunOutput` whose result is `"2"`. | High |
| OBS-H14 | rust/lovelace-desktop/src/selftest.rs:278 | `let effects = app.apply(Command::SaveAs(path.clone()));` — the save check drives `Command::SaveAs` and inspects the returned effects. | High |
| OBS-H15 | rust/lovelace-desktop/src/selftest.rs:496 | `let executable = std::env::current_exe()` — the final check inspects the binary that is running. | High |

### 1b. Per-check assertion anchors (one line quoted per check)

| ID | Source (file:line) | Fact | Confidence |
|---|---|---|---|
| OBS-C01 | rust/lovelace-desktop/src/selftest.rs:100 | `if !effects.contains(&Effect::Repaint) {` — `new-document` requires a repaint effect. | High |
| OBS-C02 | rust/lovelace-desktop/src/selftest.rs:116 | `if document.buffer != "1 + 2\n" {` — `insert-text` compares the buffer byte-for-byte. | High |
| OBS-C03 | rust/lovelace-desktop/src/selftest.rs:157 | `if document.path.as_deref() != Some(path.as_path()) {` — `open-file` requires the opened document to record its path. | High |
| OBS-C04 | rust/lovelace-desktop/src/selftest.rs:182 | `if !logs.contains("= 7") {` — `run` looks for the result text in the log output. | High |
| OBS-C05 | rust/lovelace-desktop/src/selftest.rs:224 | `if !app.logs_text().contains("Run cancelled.") {` — `cancel` checks the cancellation log line. | High |
| OBS-C06 | rust/lovelace-desktop/src/selftest.rs:248 | `if app.documents[1].precision != 250 {` — `precision` checks the document precision after `SetPrecision(250)`. | High |
| OBS-C07 | rust/lovelace-desktop/src/selftest.rs:289 | `if on_disk != "b = 21\nb * 2\n" {` — `save-round-trip` compares exact file bytes. | High |
| OBS-C08 | rust/lovelace-desktop/src/selftest.rs:326 | `if !app.logs_text().contains("Undefined variable 'a'.") {` — `tab-isolation` checks the cross-tab variable error text. | High |
| OBS-C09 | rust/lovelace-desktop/src/selftest.rs:340 | `if app.documents[0].buffer != "ab" {` — `undo-redo` checks the buffer after one undo. | High |
| OBS-C10 | rust/lovelace-desktop/src/selftest.rs:367 | `if again != settings {` — `settings-round-trip` requires structural equality after JSON round trip. | High |
| OBS-C11 | rust/lovelace-desktop/src/selftest.rs:394 | `if newer.active_precision != 1000 {` — `settings-corrupt` requires default precision when the stored schema version is newer. | High |
| OBS-C12 | rust/lovelace-desktop/src/selftest.rs:415 | `if app.logs.len() != LOG_CAP {` — `log-cap` checks the capped log length. | High |
| OBS-C13 | rust/lovelace-desktop/src/selftest.rs:449 | `if plot.width != 640 * svg::MILLI {` — `svg-parse` checks the parsed SVG width. | High |
| OBS-C14 | rust/lovelace-desktop/src/selftest.rs:483 | `if parsed.polylines[0].points.len() != 3 {` — `engine-plot` checks the parsed engine SVG point count. | High |
| OBS-C15 | rust/lovelace-desktop/src/selftest.rs:509 | `let networking = netcheck::network_imports(&imports);` — `no-network-imports` filters the import table for networking DLLs. | High |

### 1c. Build-manifest facts

| ID | Source (file:line) | Fact | Confidence |
|---|---|---|---|
| OBS-B01 | rust/lovelace-desktop/Cargo.toml:7 | `description = "Native Windows desktop IDE for the Lovelace scripting language."` | High |
| OBS-B02 | rust/lovelace-desktop/Cargo.toml:10 | `lovelace-json = { path = "../lovelace-json" }` | High |
| OBS-B03 | rust/lovelace-desktop/Cargo.toml:14 | `lovelace-lang = { path = "../lovelace-lang", optional = true }` — the only other dependency entry. | High |
| OBS-B04 | rust/lovelace-desktop/Cargo.toml:1-22 | The 22-line file has `[package]`, `[dependencies]`, `[features]`, `[lints]` and no `[dev-dependencies]`, `[[test]]`, or `[[bench]]` section. | High |
| OBS-B05 | rust/Cargo.toml:14 | `"lovelace-desktop",` — the crate is a workspace member. | High |
| OBS-B06 | rust/Cargo.toml:19 | `edition = "2024"` | High |
| OBS-B07 | rust/Cargo.toml:20 | `rust-version = "1.85"` | High |
| OBS-B08 | rust/Cargo.toml:28 | `opt-level = 3` — the release profile is the only profile block in the workspace manifest. | High |

### 1d. CI facts

| ID | Source (file:line) | Fact | Confidence |
|---|---|---|---|
| OBS-I01 | .github/workflows/ci.yml:9 | `name: CI` | High |
| OBS-I02 | .github/workflows/ci.yml:12 | `branches: [main]` — with `pull_request` and `workflow_dispatch` (lines 14-15), the only triggers. | High |
| OBS-I03 | .github/workflows/ci.yml:24 | `runs-on: ubuntu-latest` — the `fast-tests` job runner. | High |
| OBS-I04 | .github/workflows/ci.yml:184 | `runs-on: ubuntu-latest` — the `sympy-oracle` job runner. | High |
| OBS-I05 | .github/workflows/ci.yml:257 | `runs-on: ubuntu-latest` — the `aot-smoke` job runner. | High |
| OBS-I06 | .github/workflows/ci.yml:55 | `dotnet build symbench/symbench.csproj --configuration Release --nologo` — the benchmark harness is built, and the adjacent comment (lines 48-50) states CI never runs it. | High |
| OBS-I07 | .github/workflows/ci.yml:98 | `--filter "Category!=Timing&Category!=Costly" \` — the instrumented `dotnet test` loop excludes those categories. | High |
| OBS-I08 | .github/workflows/ci.yml:122 | `dotnet test Lovelace.Real.Tests/Lovelace.Real.Tests.csproj \` — a separate correctness subset with `Category!=Heavy&Category!=Timing`. | High |
| OBS-I09 | .github/workflows/ci.yml:155 | `run_subset Lovelace.Real.Tests/Lovelace.Real.Tests.csproj "Category=Timing" timing` — wall-clock-tagged tests run uninstrumented. | High |
| OBS-I10 | .github/workflows/ci.yml:166 | `uses: codecov/codecov-action@v5` — coverage upload from the `fast-tests` job. | High |
| OBS-I11 | .github/workflows/ci.yml:221 | `dotnet test Lovelace.Symbolics.Tests/Lovelace.Symbolics.Tests.csproj \` — the SymPy differential-oracle step; the job sets `LOVELACE_REQUIRE_SYMPY: '1'` (line 191). | High |
| OBS-I12 | .github/workflows/ci.yml:242 | `dotnet test Lovelace.Real.Tests/Lovelace.Real.Tests.csproj \` — the precision-oracle step (`RealTrigPrecisionOracleTests`). | High |
| OBS-I13 | .github/workflows/ci.yml:280 | `dotnet publish Lovelace.Run/Lovelace.Run.csproj \` — Native AOT publish of the .NET runner with `-p:PublishAot=true`. | High |
| OBS-I14 | .github/workflows/ci.yml:289 | `name: lovelace-run-linux-x64` — uploaded artifact name, `out/aot/Lovelace.Run`. | High |
| OBS-I15 | .github/workflows/ci.yml:293 | `- name: Smoke — the published runner answers protocol questions` — the published binary is executed by five smoke scripts (lines 293-413). | High |
| OBS-I16 | .github/workflows/ci.yml:1-414 | No line in the workflow contains `cargo`, `rust`, `selftest`, `lovelace-desktop`, or `windows-` (case-insensitive search over the whole file). | High |

### 1e. Absence facts in the desktop crate

| ID | Source (file:line) | Fact | Confidence |
|---|---|---|---|
| OBS-A01 | rust/lovelace-desktop/src/selftest.rs:1-514 | No line contains `#[test]`, `#[cfg(test)]`, `Instant`, `Duration`, or `elapsed`. | High |
| OBS-A02 | rust/lovelace-desktop/src/selftest.rs:1-514 | The token `window` occurs only as settings-model fields at lines 361, 362 and 384; no window creation, window handle, message pump, or paint call occurs. | High |
| OBS-A03 | rust/lovelace-desktop/src/selftest.rs:1-514 | No screen-capture, screenshot, image-encode, or image-file-write call occurs; the only `std::fs::write` is line 144, writing the test fixture `opened.ls`. | High |
| OBS-A04 | rust/lovelace-desktop/tests/ (absent) | The directory does not exist, so there is no integration-test target on disk. | High |
| OBS-A05 | rust/lovelace-desktop/Cargo.toml:1-22 | The only dependency keys are `lovelace-json` and `lovelace-lang`; no GUI, test, accessibility, or benchmarking dependency is declared. | High |
| OBS-A06 | rust/lovelace-desktop/src/selftest.rs:1-514 | No line contains the substring `accessib`, and no timing measurement or network I/O call occurs. | High |

## 2. Every `--selftest` check

All 15 entries are registered in the array at `selftest.rs:26-42`; each row below was derived from the body of the named function, not from its name.

| # | Check name | Concrete assertion (input -> expected) | Citation |
|---|---|---|---|
| 1 | `new-document` | Fresh `AppState` -> 1 document. `Command::NewDocument` -> 2 documents, `active == 1`, `Effect::Repaint` present, `documents[0].precision == 1000`. | selftest.rs:84-110 |
| 2 | `insert-text` | `InsertText("1 + 2\n")` -> buffer `"1 + 2\n"`, `dirty == true`, `revision == 1`, `line_col() == (2, 1)`. Then `InsertText("3")` -> buffer `"1 + 2\n3"`. | selftest.rs:112-139 |
| 3 | `open-file` | Pre-writes `"opened = 1\n"` to `<scratch>/opened.ls`; `Command::Open(path)` -> `Effect::Repaint`, 2 documents, `documents[1].buffer == "opened = 1\n"`, path recorded, not dirty, `settings.recent_files` contains the path. | selftest.rs:141-169 |
| 4 | `run` | `InsertText("a = 2 * 3\na + 1")`, then `run_blocking()` -> `Effect::RunRequested`, log contains `"= 7"`, vars contain `a` with a value containing `'6'`, `RunState::Idle`, `revision == 2`. | selftest.rs:171-205 |
| 5 | `cancel` | `InsertText("1 + 1")`, `Command::Run` -> `RunRequested`, `RunState::Running`, pending run present. `Command::Cancel` -> `RunState::Idle`, no pending run, log contains `"Run cancelled."`. Stale `finish_run(0, RunOutput{result: Ok("2"), ...}, 3)` -> returns no effects, document revision unchanged, vars still empty. | selftest.rs:207-242 |
| 6 | `precision` | `NewDocument`, `SetPrecision(250)` -> `documents[1].precision == 250` and `documents[1].engine().precision() == 250`, `documents[0].precision == 1000` (no leak), `settings.active_precision == 250`. `SetPrecision(0)` -> `documents[1].precision == 1`. | selftest.rs:244-271 |
| 7 | `save-round-trip` | `InsertText("b = 21\nb * 2\n")`, `SaveAs(path)` -> `Effect::Saved(path)`, not dirty, file bytes `"b = 21\nb * 2\n"`. `Open(path)` -> reopened buffer equals those bytes, not dirty. | selftest.rs:273-301 |
| 8 | `tab-isolation` | `NewDocument`, `InsertText("a = 1")`, `run_blocking()` -> `documents[1].vars` contain `a`; `documents[0]` buffer empty, `revision == 0`, vars empty. `SelectTab(0)`, `InsertText("a")`, `run_blocking()` -> `Effect::Error` present, log contains `"Undefined variable 'a'."`, `documents[1].vars` still non-empty. | selftest.rs:303-333 |
| 9 | `undo-redo` | `InsertText("ab")`, `InsertText("cd")`; `Undo` -> buffer `"ab"`; `Redo` -> buffer `"abcd"`; two `Undo` -> buffer empty and not dirty. | selftest.rs:335-356 |
| 10 | `settings-round-trip` | Settings with `active_precision = 2500`, window `1440x900`, `editor_width_permille = 640`, recent `"C:/scripts/selftest.ls"`; `to_json()` then `from_json()` -> value equal to the original, `warnings` empty; `SCHEMA_VERSION == 1`. | selftest.rs:358-380 |
| 11 | `settings-corrupt` | `from_json("{ this is not json")` -> `window.width == 1280`, `active_precision == 1000`, `warnings` non-empty. `from_json` with `schema_version = SCHEMA_VERSION + 1` and `active_precision = 7` -> `active_precision == 1000`, a warning containing `"newer than supported"`. | selftest.rs:382-408 |
| 12 | `log-cap` | Push `LOG_CAP + 10` `LogLevel::Info` entries -> `logs.len() == LOG_CAP`, `logs[0].text == "line 10"`. | selftest.rs:410-428 |
| 13 | `svg-parse` | `svg::make_plot_svg("selftest plot", &[1, 4, 2, 8])` then `svg::parse` -> `title == "selftest plot"`, 1 polyline with 4 points, non-empty `texts`, `width == 640 * svg::MILLI`. | selftest.rs:430-453 |
| 14 | `engine-plot` | Engine named `"fake"` -> source `plot("selftest", 1, 4, 2)`, otherwise `plot([1, 4, 2], [4, 5, 6], "selftest")`; insert + `run_blocking()` -> `last_plot` present; parsed engine SVG -> 1 polyline with 3 points, non-empty `texts` or a title. | selftest.rs:455-493 |
| 15 | `no-network-imports` | `current_exe()` -> `netcheck::file_imports` returns a non-empty list; list contains `kernel32.dll` case-insensitively; `netcheck::network_imports(imports)` returns empty. | selftest.rs:495-513 |

## 3. Automation that exists

### 3a. CI jobs in the one workflow

| Job | Runs on | What the job executes | Citation |
|---|---|---|---|
| `fast-tests` | `ubuntu-latest` | `dotnet build` of `symbench` (build only, never executed per the in-file comment); 14 `.csproj` suite paths in a `dotnet test` loop with coverage and `Category!=Timing&Category!=Costly`; a Lovelace.Real.Tests subset; uninstrumented `run_subset` calls for `Category=Timing` on Real, Suite, Run and Symbolics, and `Category=Costly` on Symbolics; Codecov upload. | ci.yml:22-171 |
| `sympy-oracle` | `ubuntu-latest` | Installs Python 3.12 and `sympy`, then two filtered `dotnet test` runs: `FullyQualifiedName~DifferentialOracle` and `FullyQualifiedName~RealTrigPrecisionOracleTests`, with `LOVELACE_REQUIRE_SYMPY: '1'`. | ci.yml:182-245 |
| `aot-smoke` | `ubuntu-latest` | Installs `clang`/`zlib1g-dev`, `dotnet publish` of `Lovelace.Run` as Native AOT to `out/aot`, uploads `lovelace-run-linux-x64`, then runs five smoke scripts against the published binary (solve envelope, partial-solve completeness, array shape, stdout purity, structural error). | ci.yml:255-413 |

### 3b. Automation by kind

| Kind | Exists? | Where | Citation |
|---|---|---|---|
| unit | Unknown | No `#[test]` in `selftest.rs`; no `[dev-dependencies]` or test target in the manifest; unit tests in other desktop `src` files were not in scope. | selftest.rs:1-514; Cargo.toml:1-22 |
| integration | No | `rust/lovelace-desktop/tests/` does not exist; no `[[test]]` target declared. | `tests/` absent; Cargo.toml:1-22 |
| GUI | No | No check creates a window or runs a message pump; no CI job builds or launches the desktop. | selftest.rs:1-5; ci.yml:1-414 |
| screenshot | No | No capture or image-file write in the selftest; no screenshot step in CI. | selftest.rs:1-514; ci.yml:1-414 |
| perf | No (desktop) | No timing measurement in the selftest. CI's two time-related entries are .NET-only: `symbench` is compiled but not run, and `Category=Timing` .NET tests run uninstrumented. | selftest.rs:1-514; ci.yml:48-55, 127-163 |
| a11y | No | No `accessib*` token in the selftest or the manifests; no accessibility step in CI. | selftest.rs:1-514; Cargo.toml:1-22; ci.yml:1-414 |
| offline | Partial | One static check: `no-network-imports` inspects the DLL import table of the running executable for networking DLLs. No runtime traffic or socket-blocking test. | selftest.rs:495-513 |
| packaging | No (desktop) | The only publish/artifact automation packages the .NET `Lovelace.Run` binary for `linux-x64`; no desktop installer or package job. | ci.yml:278-291 |
| custom selftest mode | Yes | 15 in-process checks over `AppState`, driven through `Command` values, with a `SELFTEST ok`/`FAIL` line each and exit code 0/1. | selftest.rs:19-56 |
| CI (GitHub Actions) | Yes | One workflow file, three jobs, all `ubuntu-latest`, triggered on push to main, pull requests, and manual dispatch. | ci.yml:1-15, 21-414 |

## 4. What I could NOT determine within scope, and why

1. **How `--selftest` is wired to `selftest::run()`.** The module doc names the mode and `run()` is public, but the CLI parsing and dispatch live in an entry point (e.g. `main.rs`) that was not in scope.
2. **Whether `lovelace-desktop` has unit tests inside other `src` files.** Only `selftest.rs` was in scope; the other modules it imports (`crate::app`, `crate::engine`, `crate::settings`, `crate::svg`, `crate::netcheck`) were not read, so `#[cfg(test)]` modules there are unknown.
3. **What `netcheck::file_imports` and `netcheck::network_imports` actually classify as networking,** and how the executable's import table is parsed. Only the call sites in `selftest.rs` were read; the module implementation is out of scope.
4. **What the 14 `.csproj` suites, including `Lovelace.Studio.Tests`, actually assert.** They are outside scope and were not read, so GUI, screenshot, or accessibility automation on the .NET side cannot be ruled out by this census.
5. **Whether automation exists outside `.github/workflows/ci.yml`** — other CI providers, `.github/actions`, scripts, Makefiles, or scheduled jobs. Only the `workflows/` directory was in scope; it contains exactly one file.
6. **Workspace-level or other-crate test surfaces** (`rust/tests/`, other members' `tests/` directories) — explicitly out of scope.
7. **Build/runner configuration outside the two manifests** (`Cargo.lock`, `build.rs`, `.cargo/config.toml`, nextest configuration) — not read, so whether a different test runner is configured there is undetermined.
8. **Whether any screenshots, recordings, or performance artifacts already exist elsewhere in the repository** — no repository-wide artifact inventory was performed within scope.
