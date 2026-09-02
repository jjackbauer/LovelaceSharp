# precbench — Lovelace.Real 8/16-digit precision vs float/double

Consolidated results for the `precbench` test-benchmark project. Accuracy is asserted by the
`precbench.Tests` xUnit suite; throughput is measured by the `precbench` BenchmarkDotNet project.
Machine: Windows 11 (10.0.26100), AMD Ryzen 9 5900X (24 logical / 12 physical cores), .NET SDK 10.0.103,
runtime .NET 10.0.3, BenchmarkDotNet 0.15.8.

---

## Precision mapping

Lovelace `Real` precision knobs count **fractional** digits, so on `[1,10)`-normalized operands
(one integer digit) the benchmark maps 8/16 **significant** digits to 7/15 fractional places:

| Config | Significant digits | `MaxComputationDecimalPlaces` / `DisplayDecimalPlaces` | Native counterpart |
|---|---|---|---|
| P8 | 8 | 7 | `float` (System.Single, ~7.2 digits) |
| P16 | 16 | 15 | `double` (System.Double, ~15.95 digits) |

---

## Accuracy — 13/13 tests pass

| Claim | Result |
|---|---|
| `1/3`, `1/7`, `1/6` in Lovelace | exact periodic: `0.(3)`, `0.(142857)`, `0.1(6)` |
| `0.1 + 0.2` in Lovelace | `== 0.3` exactly (double gives `0.30000000000000004`) |
| `sqrt(2,3,5)` @ P8 | relative error ≤ 1e-6 and ≤ 8× `float`'s error |
| `sqrt(2,3,5)` @ P16 | relative error ≤ 1e-14 and ≤ 8× `double`'s error |
| Precision scoping | statics restored after each scope; P8 → 7 frac places, P16 → 15 |

Structural finding: Lovelace is **exact for rationals** (period detection) and only digit-limited
for irrationals, so the 8/16-vs-float/double comparison is measured on the irrational `sqrt` cases,
where Lovelace is at least as accurate as its native counterpart.

---

## Throughput (BenchmarkDotNet, `ShortRun` job — indicative, not final)

| Op | float | Lovelace@8 | ratio | double | Lovelace@16 | ratio |
|---|---:|---:|---:|---:|---:|---:|
| Add | 0.24 ns | 35.5 ns | 150× | 0.23 ns | 37.7 ns | 163× |
| Sub | 0.24 ns | 45.8 ns | 189× | 0.23 ns | 50.2 ns | 223× |
| Mul | 0.26 ns | 44.6 ns | 172× | 0.25 ns | 199.6 ns | 812× |
| Div | 0.70 ns | 802 ns | 1,153× | 0.90 ns | 1,616 ns | 1,799× |
| Sqrt | 0.99 ns | 13.1 µs | 13,247× | 1.77 ns | 31.2 µs | 17,674× |

Allocated per op (Lovelace only): Add 224 B · Sub 240 B · Mul 336 B · Div 3.4 KB (P8) / 8.0 KB (P16)
· Sqrt 87 KB (P8) / 223 KB (P16). Native `float`/`double` allocate zero.

Observations:
- Native ops are single FPU instructions (~0.2–1.8 ns); Lovelace is digit-by-digit decimal
  arithmetic with allocation — hence ~150–1,800× on basic ops and ~13,000–18,000× on `sqrt`.
- Precision scaling is sensible: Add/Sub barely change 8→16 digits (~35→38 ns), `Mul` grows ~4.5×
  (near-quadratic), `Sqrt` ~2.4×.

---

## Caveats

- `ShortRun` = 3 warmup + 3 iterations per benchmark: these numbers are indicative. For final
  figures run without `--job short` (took ~4 min here).
- Native benchmarks batch with `OperationsPerInvoke = 16` (a single `float` add is below timer
  resolution).
- `sin/cos/exp/log` and `Pow` are not compared: `Real` does not implement transcendentals and its
  `Pow` is integer-exponent only.
- BDN's build timeout is raised to 10 min in `precbench/Program.cs` (the isolated deterministic
  rebuild of the `Lovelace.Real` chain exceeds BDN's 120 s default on this machine).

---

## Re-run

```text
# Accuracy (xUnit)
dotnet test precbench.Tests/precbench.Tests.csproj -c Release

# Throughput (BenchmarkDotNet)
dotnet run -c Release --project precbench -- --filter "*"          # full, rigorous
dotnet run -c Release --project precbench -- --filter "*" --job short   # quick
```

Raw BenchmarkDotNet artifacts: `BenchmarkDotNet.Artifacts/results/PrecBench.*-report.{md,csv,html}`.
