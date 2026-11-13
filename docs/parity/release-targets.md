# RUST-009 — supported release targets and reproducible artifacts

This document fixes what "supported target" means for the Rust workspace, gives the exact build
procedure, and records the SHA-256 of the artifacts this round verified. Every number below was
produced on this machine by the commands in §2; none is copied from a previous round.

## 1. Supported targets

A target is **supported** only when the repository builds it *and* exercises the resulting binaries
through their published entry points (the CLI matrix and the golden fixtures). A target that is
merely plausible is listed as unsupported, not as "best effort".

| Target triple | Status | How it is built | How it is exercised | Artifact hash recorded here |
|---|---|---|---|---|
| `x86_64-pc-windows-msvc` | **supported — verified on this machine** | §2 commands, toolchain 1.99.1 | `node parity/tools/ci-binaries.mjs --locked`: entry point, CLI matrix 16/16, fixtures 20/20 per side, corpus 214/214 | yes (§3) |
| `x86_64-unknown-linux-gnu` | **supported — configured in CI, not verified on this machine** | .github/workflows/rust-ci.yml `release-binaries` job, ubuntu-latest, same script | same script in CI | no — this machine has no Linux toolchain (`rustup target list --installed` reports only `x86_64-pc-windows-msvc`), so no Linux bytes were produced or hashed here |
| `aarch64-apple-darwin`, `x86_64-apple-darwin`, `aarch64-unknown-linux-gnu`, `aarch64-pc-windows-msvc` | **not supported** | — | — | none claimed |

Two consequences are recorded rather than implied:

* The desktop product (`rust/lovelace-desktop`) declares its GUI dependencies under
  `[target.'cfg(windows)'.dependencies]` in [rust/lovelace-desktop/Cargo.toml](../../rust/lovelace-desktop/Cargo.toml),
  so it is a Windows-only artifact. The Linux CI job builds and tests every other crate
  (`--exclude lovelace-desktop`); the Windows job runs the full workspace.
* "Supported for Linux" is a CI configuration today, not a verified artifact. The row stays
  unverified until a CI run uploads `rust-binaries-ubuntu-latest` (the workflow does exactly that);
  no hash is invented for it in this document.

## 2. Reproducible build procedure

The workspace is `rust/`; the manifest is `rust/Cargo.toml`. Nothing else is required.

```powershell
$repo = 'C:\Users\ricar\dev\LovelaceSharp'
$env:RUSTUP_HOME = "$repo\.toolchain\rustup"
$env:CARGO_HOME  = "$repo\.toolchain\cargo"
$env:PATH        = "$repo\.toolchain\cargo\bin;" + $env:PATH
$env:TMP         = "$repo\rust\target\tmp"; $env:TEMP = $env:TMP

# x86_64-pc-windows-msvc: this flag is what makes the PE bytes reproducible (see §4).
$env:RUSTFLAGS   = '-C link-arg=/Brepro'

cargo build --release --locked --manifest-path rust/Cargo.toml -p lovelace-run -p lovelace-corpus
node parity/tools/ci-binaries.mjs --locked          # exercises the binaries, writes parity/build/rust-binaries.json
Get-FileHash rust/target/release/lovelace-run.exe    -Algorithm SHA256
Get-FileHash rust/target/release/lovelace-corpus.exe -Algorithm SHA256
```

Inputs that are pinned by this procedure and therefore cannot drift silently:

| Input | Pin | Consequence |
|---|---|---|
| Compiler | `rustc 1.99.1 (58cf8f9c3 2026-10-13)`, `cargo 1.99.1` | a different compiler is a different build, not a failure |
| Dependencies | `rust/Cargo.lock` + `--locked` | resolution drift fails the build instead of changing the artifact |
| Profile | `rust/Cargo.toml`: `opt-level = 3`, `lto = "thin"`, `codegen-units = 1` | recorded in the manifest, not in a shell variable |
| Link | `RUSTFLAGS=-C link-arg=/Brepro` (MSVC only) | removes PE timestamp/PDB variance (§4) |

## 3. Verified artifacts (x86_64-pc-windows-msvc)

Built with the procedure above, then exercised by `node parity/tools/ci-binaries.mjs --locked`
(observed 2026-11-13, transcript in the round report):

| Artifact | Bytes | SHA-256 | Exercised by |
|---|---:|---|---|
| `rust/target/release/lovelace-run.exe` | 3 885 056 | `291d85fe73219a16f51f4ccddf8f7f3c4ca129f0e435007b66160c2d1581308d` | CLI matrix 16/16, fixtures rust 20/20, `--eval 2^10 --json` → `protocolVersion=1`, `display=1024` |
| `rust/target/release/lovelace-corpus.exe` | 3 627 520 | `4ae0aa5631081e919a483993ed7d48b7fc593c9f5c59f70de520af5b30c27435` | `corpus: 214 passed, 0 failed, 214 total` |

Both are real PE images (`50 45 00 00` magic; the script refuses anything else), both are the
release profile, and both hashes are also written to `parity/build/rust-binaries.json` by the gate
itself, so the evidence file and this table come from the same run.

## 4. What "reproducible" claims here, and what it does not

Measured, not assumed. The same sources and flags were built three ways:

| Build | Target directory | `RUSTFLAGS` | lovelace-run SHA-256 | lovelace-corpus SHA-256 |
|---|---|---|---|---|
| A | `rust/target/release` | none | `f7ae55a2…59c4939` | `ceadf6c6…2566bad` |
| B | `rust/target/release`, after `cargo clean --release -p lovelace-run -p lovelace-corpus` and relink | none | `8ab7d547…8491a8c` — **differs from A** | `730cd4d9…e4e3a11a2` — **differs from A** |
| C | `rust/target/release` | `-C link-arg=/Brepro` | `291d85fe…81308d` | `4ae0aa56…c27435` |
| D | `rust/target/repro` (separate target dir) | `-C link-arg=/Brepro` | `291d85fe…81308d` — **identical to C** | `4ae0aa56…c27435` — **identical to C** |
| E | `rust/target/release`, clean relink again | `-C link-arg=/Brepro` | `291d85fe…81308d` — **identical to C** | `4ae0aa56…c27435` — **identical to C** |

So:

* **Claimed and verified:** with the pinned toolchain, `--locked`, the release profile and
  `-C link-arg=/Brepro` on x86_64-pc-windows-msvc, the artifact bytes are reproducible — across a
  clean relink in the same target directory and across a different target directory.
* **Not claimed:** byte identity across host machines, across compiler versions, for the debug
  profile, or for the `x86_64-unknown-linux-gnu` target (not built here). The MSVC linker flag has
  no ELF equivalent to test here.
* **Observed default (no flag):** two builds of identical sources differ (A vs B). The difference is
  link metadata (PE timestamp and PDB identity), not code. A hash therefore identifies a *build*,
  not a source state, unless the /Brepro procedure is used — which is why the workflow sets the
  flag for the Windows release job.

The same caveat applies to the C# oracle artifact; see
[oracle-artifacts.md](oracle-artifacts.md) §5, where three builds of the same pinned C# sources
produced three hashes.

## 5. Reproduce the claim

```powershell
# 1. build twice, forcing a relink in between
node parity/tools/ci-binaries.mjs --locked
$h1 = (Get-FileHash rust/target/release/lovelace-run.exe -Algorithm SHA256).Hash
cargo clean --release -p lovelace-run -p lovelace-corpus --manifest-path rust/Cargo.toml
cargo build --release --locked --manifest-path rust/Cargo.toml -p lovelace-run -p lovelace-corpus
$h2 = (Get-FileHash rust/target/release/lovelace-run.exe -Algorithm SHA256).Hash
$h1 -eq $h2   # True with RUSTFLAGS=-C link-arg=/Brepro, False without it
```

The same two builds are what `parity/tools/reproduce.ps1` step `binaries` runs as part of the
whole REQ-010 sequence.
