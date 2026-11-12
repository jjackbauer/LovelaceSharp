# UI candidates: minimal native probes for iced and slint

Comparative evidence for ARC-001. Two standalone probe crates, one per candidate. This file reports
observed facts only; it does not recommend a winner.

- Probe date: 2026-11-12 (America/Sao_Paulo)
- Host: Windows NT 10.0.26100 (Windows 11), AMD Ryzen 9 5900X 12-Core
- Toolchain: cargo 1.99.1 (5f94df478 2026-08-27), rustc 1.99.1 (58cf8f9c3 2026-10-13)
- crates.io latest stable, queried at probe time: iced 0.14.0, slint 1.18.1
- Crate sources inspected at:
  `C:\Users\ricar\dev\LovelaceSharp\.toolchain\cargo\registry\src\index.crates.io-1949cf8c6b5b557f\`
  (all `crate-name-version/...` citations below are relative to that directory)

## Comparison table

| Candidate | Version | C1 build | C2 window | C3 text widget (source citation) | C4 accessibility (source citation) | C5 image API | C6 clean build seconds | C7 debug exe bytes |
|---|---|---|---|---|---|---|---|---|
| iced | iced 0.14.0 (iced_widget 0.14.2, iced_winit 0.14.1, winit 0.30.13, wgpu 27.0.1) | PASS. Exit 0; final line: ``Finished `dev` profile [unoptimized + debuginfo] target(s) in 46.76s`` | PASS. Title `iced ui probe`; process alive 5 s after start | `text_editor::TextEditor` (multi-line). Defined at `iced_widget-0.14.2/src/text_editor.rs:99`; free fn `text_editor` at `iced_widget-0.14.2/src/helpers.rs:1491`; module doc `text_editor.rs:1` says "display a multi-line text input" | **NO.** No `accessibility` feature and no AccessKit dependency anywhere in the iced 0.14.0 published crates; 0 `accesskit` entries in its `Cargo.lock`. Details and search below | `iced::widget::image(...)` with `iced::widget::image::Handle::from_path` / `from_bytes` / `from_rgba`. Gated behind the **non-default** `image` feature: `iced-0.14.0/Cargo.toml:83`, module gate `iced-0.14.0/src/lib.rs:639-643`, widget `iced_widget-0.14.2/src/image.rs:58`, helper `iced_widget-0.14.2/src/helpers.rs:1812`, `Handle` `iced_core-0.14.0/src/image.rs:124` | 46.961 (cargo self-report 46.76) | 26,945,024 |
| slint | slint 1.18.1 (i-slint-core 1.18.1, winit 0.30.13, accesskit 0.24.1, accesskit_winit 0.33.2, image 0.25.10) | PASS. Exit 0; final line: ``Finished `dev` profile [unoptimized + debuginfo] target(s) in 46.22s`` | PASS. Title `slint ui probe`; process alive 5 s after start | `TextEdit`, a built-in `.slint` widget (default fluent style). `i-slint-compiler-1.18.1/widgets/fluent/textedit.slint:8`; shared implementation `i-slint-compiler-1.18.1/widgets/common/textedit-base.slint:6`; multi-line `text` property at `textedit-base.slint:20`; default style "fluent" at `i-slint-compiler-1.18.1/typeloader.rs:957` | **YES, by default.** `accessibility` is in slint's default features (`slint-1.18.1/Cargo.toml:120`); feature chain to `i-slint-backend-winit-1.18.1/Cargo.toml:39-43`; pulls `accesskit` + `accesskit_winit` (`Cargo.toml:268-276`); cfg enabled at `i-slint-backend-winit-1.18.1/build.rs:12`; adapter at `i-slint-backend-winit-1.18.1/accesskit.rs:46-47` (`AccessKitAdapter { inner: accesskit_winit::Adapter }`) | `slint::Image::load_from_path` (`i-slint-core-1.18.1/graphics/image.rs:824`) and `slint::Image::from_rgba8` (`:842`); re-exported via `i-slint-core-1.18.1/api.rs:20-21` + `slint-1.18.1/lib.rs:218`. Decoders enabled by default through the winit backend: `i-slint-backend-winit-1.18.1/Cargo.toml:283-286` enables `image-decoders` + `svg` | 46.411 (cargo self-report 46.22) | 28,834,304 |

## Probe applications

Both are standalone single-crate Cargo workspaces (`[workspace]` in the manifest), one directory each,
with their own `CARGO_TARGET_DIR`. No profile overrides: default `dev` profile, unoptimized + debuginfo.

Probe roots:

- `C:\Users\ricar\dev\LovelaceSharp\.toolchain\ui-probe-iced\`
- `C:\Users\ricar\dev\LovelaceSharp\.toolchain\ui-probe-slint\`

### iced probe source

`ui-probe-iced/Cargo.toml`:

```toml
[package]
name = "ui-probe-iced"
version = "0.1.0"
edition = "2021"

[dependencies]
iced = "0.14.0"

[workspace]
```

`ui-probe-iced/src/main.rs`:

```rust
// Minimal capability probe for iced 0.14: one native window with a multi-line
// text editor (3 lines), a "Run" button, and a static label. Not product code.

use iced::widget::{button, column, text, text_editor};
use iced::{Element, Task};

fn main() -> iced::Result {
    iced::application(Probe::new, Probe::update, Probe::view)
        .title("iced ui probe")
        .window_size([720.0, 480.0])
        .run()
}

struct Probe {
    content: text_editor::Content,
}

#[derive(Debug, Clone)]
enum Message {
    Edit(text_editor::Action),
    Run,
}

impl Probe {
    fn new() -> Self {
        Self {
            content: text_editor::Content::with_text("line one\nline two\nline three"),
        }
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Edit(action) => {
                self.content.perform(action);
            }
            Message::Run => {}
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        column![
            text("iced UI probe: static label"),
            text_editor(&self.content).on_action(Message::Edit),
            button("Run").on_press(Message::Run),
        ]
        .into()
    }
}
```

### slint probe source

`ui-probe-slint/Cargo.toml`:

```toml
[package]
name = "ui-probe-slint"
version = "0.1.0"
edition = "2021"

[dependencies]
slint = "1.18.1"

[workspace]
```

`ui-probe-slint/src/main.rs`:

```rust
// Minimal capability probe for Slint 1.18: one native window with a multi-line
// text editor (3 lines), a "Run" button, and a static label. Not product code.

slint::slint! {
    import { Button, TextEdit } from "std-widgets.slint";

    export component AppWindow inherits Window {
        title: "slint ui probe";
        width: 720px;
        height: 480px;

        in-out property <string> text: "line one\nline two\nline three";

        VerticalLayout {
            padding: 16px;
            spacing: 12px;

            Text { text: "slint UI probe: static label"; }
            TextEdit { text <=> root.text; }
            Button { text: "Run"; }
        }
    }
}

fn main() -> Result<(), slint::PlatformError> {
    let ui = AppWindow::new()?;
    ui.run()
}
```

Note: the first slint attempt omitted `import { Button, TextEdit } from "std-widgets.slint";` and failed
with `error: Unknown element 'TextEdit'` / `error: Unknown element 'Button'`; the import above is required
in the `slint!` macro for std-widgets. The successful measured build contains that import.

## Version resolution (commands and outputs)

```powershell
(Invoke-RestMethod 'https://crates.io/api/v1/crates/iced'  -Headers @{'User-Agent'='ui-probe'}).crate.max_stable_version
# 0.14.0
(Invoke-RestMethod 'https://crates.io/api/v1/crates/slint' -Headers @{'User-Agent'='ui-probe'}).crate.max_stable_version
# 1.18.1
```

Resolved versions (from each probe's `Cargo.lock`):

- iced probe: `iced 0.14.0`, `iced_widget 0.14.2`, `iced_winit 0.14.1`, `iced_renderer 0.14.0`,
  `wgpu 27.0.1`, `winit 0.30.13`, 382 locked packages, **no `accesskit` package**.
- slint probe: `slint 1.18.1`, `i-slint-core 1.18.1`, `winit 0.30.13`, `accesskit 0.24.1`,
  `accesskit_winit 0.33.2`, `image 0.25.10`, 551 locked packages.

## C1 — Does it build?

Environment set before each build (fresh PowerShell process per command):

```powershell
$repo='C:\Users\ricar\dev\LovelaceSharp'
$env:RUSTUP_HOME="$repo\.toolchain\rustup"
$env:CARGO_HOME="$repo\.toolchain\cargo"
$env:TMP="$repo\rust\target-desktop\tmp"; $env:TEMP=$env:TMP
$env:CARGO_TARGET_DIR="$repo\rust\target-probe-iced"   # or target-probe-slint
```

Exact build command (one per probe):

```powershell
& "$repo\.toolchain\cargo\bin\cargo.exe" build --manifest-path "$repo\.toolchain\ui-probe-iced\Cargo.toml"
& "$repo\.toolchain\cargo\bin\cargo.exe" build --manifest-path "$repo\.toolchain\ui-probe-slint\Cargo.toml"
```

Observed outputs (`... \build.log`, tail):

```text
iced:  exit=0
       Finished `dev` profile [unoptimized + debuginfo] target(s) in 46.76s
slint: exit=0
       Finished `dev` profile [unoptimized + debuginfo] target(s) in 46.22s
```

The logs also show the expected compile of the probe crate itself
(`Compiling ui-probe-iced v0.1.0` / `Compiling ui-probe-slint v0.1.0`). Cargo writes progress to stderr,
so the saved logs contain PowerShell's stderr formatting around individual progress lines; the final
`Finished` line above is the verbatim tail.

## C2 — Does a native window open?

Method: start the debug executable with `Start-Process -PassThru`, poll `MainWindowTitle`, wait until at
least 5.0 s have elapsed since start, then check `HasExited`. The window title is read from the Win32 main
window. The process was force-stopped after the observation.

Exact command as run for iced (the slint run is identical with `target-probe-slint` /
`ui-probe-slint` substituted for the two paths; the whole thing is one PowerShell command):

```powershell
$repo='C:\Users\ricar\dev\LovelaceSharp'
$exe="$repo\rust\target-probe-iced\debug\ui-probe-iced.exe"
$out="$repo\.toolchain\ui-probe-iced\window-test.txt"
$sw=[Diagnostics.Stopwatch]::StartNew()
$proc = Start-Process -FilePath $exe -PassThru -WorkingDirectory (Split-Path $exe)
$title=''
for ($i=0; $i -lt 40; $i++) { Start-Sleep -Milliseconds 250; try { $proc.Refresh() } catch { break }
  if ($proc.HasExited) { break }; if ($proc.MainWindowTitle) { $title=$proc.MainWindowTitle; break } }
$titleAt=[Math]::Round($sw.Elapsed.TotalSeconds,2)
$remain = 5.0 - $sw.Elapsed.TotalSeconds
if ($remain -gt 0) { Start-Sleep -Milliseconds ([Math]::Ceiling($remain*1000)) }
try { $proc.Refresh() } catch {}
$alive = -not $proc.HasExited
$exit = if ($proc.HasExited) { $proc.ExitCode } else { 'n/a' }
@("exe=$exe","pid=$($proc.Id)","window_title=$title","title_observed_at_s=$titleAt","alive_at_5s=$alive","has_exited=$($proc.HasExited)","exit_code=$exit") | Set-Content -Encoding utf8 $out
Get-Content $out
if ($alive) { Stop-Process -Id $proc.Id -Force; Start-Sleep -Milliseconds 500; Write-Host "killed_after_test=True" }
```

Observed outputs (saved as `ui-probe-<name>/window-test.txt`):

```text
iced:
exe=C:\Users\ricar\dev\LovelaceSharp\rust\target-probe-iced\debug\ui-probe-iced.exe
pid=29468
window_title=iced ui probe
title_observed_at_s=2.24
alive_at_5s=True
has_exited=False
exit_code=n/a
```

```text
slint:
exe=C:\Users\ricar\dev\LovelaceSharp\rust\target-probe-slint\debug\ui-probe-slint.exe
pid=25832
window_title=slint ui probe
title_observed_at_s=2.14
alive_at_5s=True
has_exited=False
exit_code=n/a
```

Both processes were still running at the 5 s check and were then killed. The title strings match the
titles set in the probe sources.

## C3 — Built-in editable text widget

### iced

Yes. The widget is `iced::widget::text_editor::TextEditor`, a multi-line text input, with an associated
`text_editor::Content` buffer. Source:

- `iced_widget-0.14.2/src/text_editor.rs:1` — `//! Text editors display a multi-line text input for text editing.`
- `iced_widget-0.14.2/src/text_editor.rs:99` — `pub struct TextEditor<`
- `iced_widget-0.14.2/src/text_editor.rs:384` — `pub struct Content<R = crate::Renderer>`
- `iced_widget-0.14.2/src/helpers.rs:1491` — `pub fn text_editor<'a, Message, Theme, Renderer>(`
- `iced_widget-0.14.2/src/text_editor.rs:405` — `pub fn with_text(text: &str) -> Self` (used to pre-fill 3 lines)

Editing is wired manually: the probe handles `text_editor::Action` and calls `Content::perform`.
`helpers.rs` is re-exported through `iced::widget` (`iced-0.14.0/src/lib.rs:637`).

Search command and output:

```powershell
Select-String -Path "$reg\iced_widget-0.14.2\src\text_editor.rs" -Pattern 'pub struct TextEditor|pub fn text_editor'
# 99: pub struct TextEditor<
```

### slint

Yes. The widget is the built-in `.slint` element `TextEdit` (multi-line; `LineEdit` is the single-line
one), implemented in the standard widget library and compiled by `slint-macros`. Source:

- `i-slint-compiler-1.18.1/typeloader.rs:957` — default widget style is `"fluent"` when no style is configured
- `i-slint-compiler-1.18.1/widgets/fluent/textedit.slint:8` — `export component TextEdit {`
- `i-slint-compiler-1.18.1/widgets/common/textedit-base.slint:6` — `export component TextEditBase inherits Rectangle {`
- `i-slint-compiler-1.18.1/widgets/common/textedit-base.slint:20` — `in-out property <string> text <=> text-input.text;`
- `i-slint-compiler-1.18.1/widgets/fluent/std-widgets.slint:23` — `export { TextEdit } from "textedit.slint";`

The probe uses two-way binding (`TextEdit { text <=> root.text; }`), so edits go back into the component
property. The three prefilled lines use the `\n` string escape, which the compiler handles at
`i-slint-compiler-1.18.1/literals.rs:61` (`b'n' => (2, '\n')`). Search command and output:

```powershell
Select-String -Path "$reg\i-slint-compiler-1.18.1\widgets\fluent\textedit.slint" -Pattern 'export component TextEdit'
# 8: export component TextEdit {
```

## C4 — Accessibility tree by default

### iced: NO

Stated plainly: iced 0.14.0 does not ship an accessibility-tree implementation or an accessibility feature.
Evidence:

- `iced-0.14.0/Cargo.toml` feature list (`:48-127`) has `default`, `image`, `image-without-codecs`, `svg`,
  `debug`, etc., but no `accessibility` feature.
- `iced_winit-0.14.1/Cargo.toml` features are `debug`, `default`, `linux-theme-detection`, `sysinfo`,
  `unconditional-rendering`, `wayland`, `x11`; its dependency list has no `accesskit`/`accesskit_winit`.
- A recursive search of all twelve published iced crates for `accesskit` or `accessibility` returned zero matches.
- `ui-probe-iced/Cargo.lock` contains zero `accesskit` entries.

Verification command and output:

```powershell
Get-ChildItem $reg -Directory -Filter 'iced*' |
  ForEach-Object { Get-ChildItem $_.FullName -Recurse -File -Include *.rs,*.toml,*.md |
    Select-String -Pattern 'accesskit|AccessKit|accessibility' -List }
# (no output)
Select-String -Path "$repo\.toolchain\ui-probe-iced\Cargo.lock" -Pattern 'accesskit'
# (no output; count = 0)
```

### slint: YES, with default features

Evidence chain:

- `slint-1.18.1/Cargo.toml:115-123` — `default = [ "std", "backend-default", "renderer-femtovg",
  "renderer-software", "accessibility", "compat-1-2", "system-tray" ]`; `accessibility` is line 120.
- `i-slint-backend-winit-1.18.1/Cargo.toml:39-43` — `accessibility = [ "dep:accesskit", "dep:accesskit_winit",
  "i-slint-core/accessibility-text" ]`.
- `i-slint-backend-winit-1.18.1/Cargo.toml:268-276` — optional `accesskit 0.24` and `accesskit_winit 0.33`
  dependencies (the lock resolved `accesskit 0.24.1`, `accesskit_winit 0.33.2`).
- `i-slint-backend-winit-1.18.1/build.rs:12` — `enable_accesskit: { all(feature = "accessibility",
  not(target_arch = "wasm32")) }`, i.e. the adapter is compiled in on this Windows build.
- `i-slint-backend-winit-1.18.1/accesskit.rs:46-47` — `pub struct AccessKitAdapter { inner: accesskit_winit::Adapter, ... }`.
- `i-slint-backend-winit-1.18.1/winitwindowadapter.rs:231` and `:699` — the adapter is stored on the window
  adapter and constructed during window creation.

Not verified by me: the actual UIA/accessibility node dump of the running slint window (that capture is the
orchestrator's task). The claim above is from the resolved feature graph and crate source.

## C5 — Raster image loading API

### iced

API: `iced::widget::image(handle)` producing the `image::Image` widget, with
`iced::widget::image::Handle::from_path(path)`, `Handle::from_bytes(bytes)` and `Handle::from_rgba(w, h, rgba)`.
This is **feature-gated behind the non-default `image` feature**.

Sources:

- `iced-0.14.0/Cargo.toml:83` — `image = [ "image-without-codecs", "image/default" ]`
- `iced-0.14.0/src/lib.rs:639-643` — `#[cfg(feature = "image")] pub mod image { ... pub use iced_widget::image::*; }`
- `iced_widget-0.14.2/src/image.rs:58` — `pub struct Image<Handle = image::Handle> {`
- `iced_widget-0.14.2/src/image.rs:33` — `pub use image::{FilterMethod, Handle};`
- `iced_widget-0.14.2/src/helpers.rs:1812` — `pub fn image<Handle>(handle: impl Into<Handle>) -> crate::Image<Handle>`
- `iced_core-0.14.0/src/image.rs:124` — `pub fn from_path<T: Into<PathBuf>>(path: T) -> Handle`
- `iced_core-0.14.0/src/image.rs:136` — `pub fn from_bytes(bytes: impl Into<Bytes>) -> Handle`

The default feature set (`iced-0.14.0/Cargo.toml:64-73`) does **not** include `image`, so the default
iced build cannot construct the image widget without adding that feature. It was not enabled in this probe,
so the image path was source-verified only, not compiled or run here.

### slint

API: `slint::Image::load_from_path(&Path)` and `slint::Image::from_rgba8(SharedPixelBuffer<Rgba8Pixel>)`
(also `from_rgb8`, `from_rgba8_premultiplied`), displayed with the `.slint` `Image { source: ... }` element
or an `image` property. Available in this default build.

Sources:

- `i-slint-core-1.18.1/graphics/image.rs:824` — `pub fn load_from_path(path: &std::path::Path) -> Result<Self, LoadImageError>`
- `i-slint-core-1.18.1/graphics/image.rs:833` — `pub fn from_rgb8(buffer: SharedPixelBuffer<Rgb8Pixel>) -> Self`
- `i-slint-core-1.18.1/graphics/image.rs:842` — `pub fn from_rgba8(buffer: SharedPixelBuffer<Rgba8Pixel>) -> Self`
- `i-slint-core-1.18.1/api.rs:20-21` — re-exports `Image`, `Rgb8Pixel`, `Rgba8Pixel`, `SharedPixelBuffer`
- `slint-1.18.1/lib.rs:218` — `pub use i_slint_core::api::*;`
- `i-slint-backend-winit-1.18.1/Cargo.toml:283-286` — the winit backend depends on `i-slint-core` with
  `features = [ "image-decoders", "svg" ]` (so `load_from_path` is compiled in by default)
- `slint-1.18.1/Cargo.toml:125` — `image-default-formats = ["i-slint-core/image-default-formats"]`
  (optional, not in default features; the base decoder handles SVG, PNG and JPEG per the `load_from_path` doc)

Not exercised at runtime in this probe; source-verified only.

## C6 — Clean debug build wall-clock time

Method: delete the probe's `CARGO_TARGET_DIR`, start a `[Diagnostics.Stopwatch]`, run
`cargo build --manifest-path <probe>\Cargo.toml` to completion, stop the stopwatch. One run per candidate,
fresh target dir, builds run sequentially (no other cargo build running), default `dev` profile.

```powershell
if (Test-Path $env:CARGO_TARGET_DIR) { Remove-Item -Recurse -Force $env:CARGO_TARGET_DIR }
$sw=[Diagnostics.Stopwatch]::StartNew()
& "$repo\.toolchain\cargo\bin\cargo.exe" build --manifest-path "$p\Cargo.toml" *> "$p\build.log"
$code=$LASTEXITCODE
$sw.Stop()
```

Observed:

| Candidate | stopwatch seconds | cargo final line | exit |
|---|---|---|---|
| iced | 46.961 | ``Finished `dev` profile [unoptimized + debuginfo] target(s) in 46.76s`` | 0 |
| slint | 46.411 | ``Finished `dev` profile [unoptimized + debuginfo] target(s) in 46.22s`` | 0 |

Raw result files: `ui-probe-iced/build.result.txt`, `ui-probe-slint/build.result.txt`.

## C7 — Debug executable size

Command:

```powershell
(Get-Item "$repo\rust\target-probe-iced\debug\ui-probe-iced.exe").Length
(Get-Item "$repo\rust\target-probe-slint\debug\ui-probe-slint.exe").Length
```

Observed:

| Candidate | path | bytes |
|---|---|---|
| iced | `rust\target-probe-iced\debug\ui-probe-iced.exe` | 26,945,024 |
| slint | `rust\target-probe-slint\debug\ui-probe-slint.exe` | 28,834,304 |

## What I could not verify

- **Visual presence of the three widgets and the three prefilled lines.** I did not take a screenshot or
  read screen pixels; the elements are established by the probe source and its compiled build. The
  orchestrator's window capture is the empirical check.
- **Actual typing/editing interactions.** I did not send keystrokes or mouse input to either window; only
  the built-in widget's existence, API and wiring were checked.
- **Accessibility tree contents.** I did not dump UIA/AccessKit nodes from either running process. For
  slint, "yes" comes from the default feature, the compiled adapter and crate source. For iced, "no" comes
  from the absence of any accessibility feature/dependency in the published 0.14.0 crates and its
  lockfile; that is not the same as an empirical UIA dump.
- **Image rendering at runtime.** Neither image API was exercised: iced's `image` feature was not enabled
  in the probe build (default features do not include it), and slint's `load_from_path` was not called.
  Both are source-verified only.
- **IME, syntax highlighting, docking, SVG rendering.** Not implemented and not tested, per scope.
- **Multi-line rendering details** (wrapping, scrolling, line-height) were not measured.
- **Build-time generality.** One clean run per candidate on one machine (Ryzen 9 5900X, Windows 11). Not
  averaged; no warm-cache comparison. The stopwatch covers the `cargo build` process only, not deletion of
  the target directory.
- **Window title verification** is from the Win32 `MainWindowTitle` property of the process, not from
  reading the rendered title bar.
- **Other GPUs/backends.** iced ran with its default `wgpu` renderer; slint with its default `femtovg`
  (OpenGL) plus software renderer feature. No alternate-backend testing.
- **Startup latency, memory, or idle CPU** were not measured.
