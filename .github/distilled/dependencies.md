# Dependencies

> **Scope**: Inter-project and external dependencies across the LovelaceSharp solution
> **Confidence**: High
> **Last updated**: 2026-03-16
> **Source entries**: OBS-003, OBS-006, OBS-010, OBS-018, OBS-020, VAL-001, VAL-002

---

## Purpose

This document tracks inter-project dependency relationships and external package dependencies
within the LovelaceSharp solution. It covers the internal layered dependency chain between
the four core numeric libraries and the console application, as well as any NuGet or framework
dependencies. Content is sourced from OBS entries (project reference observations) and VAL
entries that confirm boundary claims.

---

## Internal Dependency Chain

- ✅ `Lovelace.Natural` → `Lovelace.Representation` (project reference; accesses internal APIs via InternalsVisibleTo) (OBS-003, OBS-006, VAL-001).
- ✅ `Lovelace.Integer` → `Lovelace.Natural` (project reference; uses Natural for magnitude arithmetic) (OBS-010, OBS-018).
- ✅ `Lovelace.Real` → `Lovelace.Integer` (project reference; extends Integer via inheritance) (OBS-011, OBS-018, VAL-002).
- ✅ `Lovelace.Console` → `Lovelace.Natural`, `Lovelace.Integer`, `Lovelace.Real` (direct project references; uses only public APIs) (OBS-016, OBS-018).

## InternalsVisibleTo Grants

- ✅ `Lovelace.Representation` → `Lovelace.Representation.Tests`, `Lovelace.Natural` (OBS-003).
- ✅ `Lovelace.Real` → `Lovelace.Real.Tests` (OBS-020).
- ✅ No `InternalsVisibleTo` in `Lovelace.Natural` or `Lovelace.Integer` (OBS-018).

---

## External Dependencies

- ⚠️ **Framework**: .NET 10.0 across all projects (OBS-018).
- ⚠️ **System namespaces used**: `System.Buffers` (ArrayPool), `System.Numerics` (generic math interfaces), `System.Threading` (Parallel, Interlocked, AsyncLocal), `System.Runtime.CompilerServices`, `System.Runtime.InteropServices` (OBS-001, OBS-005, OBS-012).
- ⚠️ **Test packages**: xUnit 2.9.3, xunit.runner.visualstudio 3.1.4, Microsoft.NET.Test.Sdk 17.14.1, coverlet.collector 6.0.4 (OBS-001).
- ⚠️ No third-party NuGet runtime dependencies — all runtime code uses only BCL types (OBS-018).

---

## Test Project Dependencies

*No content yet — populated by `skill-journal-distill` from OBS and VAL entries.*
