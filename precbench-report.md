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

## Throughput (BenchmarkDotNet, default job)

| Op | float | Lovelace@8 | ratio | double | Lovelace@16 | ratio |
|---|---:|---:|---:|---:|---:|---:|
| Add | 0.26 ns | 43.9 ns | 169× | 0.27 ns | 42.4 ns | 160× |
| Sub | 0.25 ns | 56.0 ns | 222× | 0.25 ns | 59.6 ns | 240× |
| Mul | 0.26 ns | 52.7 ns | 202× | 0.26 ns | 257 ns | 990× |
| Div | 0.74 ns | 923 ns | 1,244× | 0.99 ns | 1,807 ns | 1,816× |
| Sqrt | 1.07 ns | 15.2 µs | 14,204× | 1.91 ns | 36.5 µs | 19,089× |

Allocated per op (Lovelace only): Add 224 B · Sub 240 B · Mul 336 B · Div 3.4 KB (P8) / 8.0 KB (P16)
· Sqrt 87 KB (P8) / 223 KB (P16). Native `float`/`double` allocate zero.

Observations:
- Native ops are single FPU instructions (~0.25–1.9 ns); Lovelace is digit-by-digit decimal
  arithmetic with allocation — hence ~160–1,900× on basic ops and ~14,000–19,000× on `sqrt`.
- Precision scaling is sensible: Add/Sub barely move 8→16 digits (~44→42 ns, ~56→60 ns), `Mul`
  grows ~4.9× (53→257 ns, near-quadratic), `Div` ~2.0×, `Sqrt` ~2.4× (15.2→36.5 µs).

---

## Caveats

- Numbers are BDN's **default job** (pilot-calibrated iteration counts, 99.9% CI, median). BDN
  flagged `LovelaceP8.Mul`/`Div` as multimodal and removed outliers, so those means carry wider
  error bars.
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
dotnet run -c Release --project precbench -- --filter "*"          # full (default job, ~28 min here)
dotnet run -c Release --project precbench -- --filter "*" --job short   # quick smoke
```

Raw BenchmarkDotNet artifacts: `BenchmarkDotNet.Artifacts/results/PrecBench.*-report.{md,csv,html}`.
