# Supported platforms — Lovelace Desktop

CON-009 requires an explicit published target matrix that distinguishes the initial release from
later ports, and forbids a platform claim without a built, run GUI test on it. This file is that
matrix. It is release metadata, not a promise: a row may say `LATER PORT` or `UNSUPPORTED` and that
is a complete, honest answer.

| Platform | Status for this release | Build evidence | Run GUI evidence | Notes |
|---|---|---|---|---|
| Windows 11 x64 (10.0.22631 or later) | **INITIAL RELEASE** | required: `cargo build -p lovelace-desktop` on the release commit | required: native window launched, screenshot captured, GUI suite executed | The only platform this run can certify. DPI: 100/150/200% are exercised by TST-014. |
| Windows 10 x64 | **UNSUPPORTED (untested)** | none | none | No Windows 10 host was available. Not claimed. |
| Linux x86_64 | **LATER PORT** (not in v1) | none | none | Deferred by DEC-006. The ARC-001 Linux half is deferred with it. |
| macOS (arm64 and x86_64) | **LATER PORT** (not in v1) | none | none | Deferred by DEC-006. No macOS host was available. |

## What the application must do on an unsupported platform

PLF-009 requires an explicit, non-silent error. The binary must:

1. fail to build the native shell on a platform without a UI adapter, and
2. when a headless build is asked to start the UI, print a message that names the platform as
   unsupported and exits non-zero — never launch a headless placeholder or pretend to work.

## Rules

- A row changes to a supported status only when a GUI test artifact from that platform is attached to
  the gate evidence for CON-009/PLF-010.
- Cross-compilation alone never changes a row: CON-009 requires a **run** test.
- This matrix is referenced by `gate/register.json` row CON-009 and by the platform claim in the
  release manifest (TST-016).
