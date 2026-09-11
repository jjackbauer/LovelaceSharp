```

BenchmarkDotNet v0.15.8, Windows 11 (10.0.26100.4351/24H2/2024Update/HudsonValley)
AMD Ryzen 9 5900X 3.70GHz, 1 CPU, 24 logical and 12 physical cores
.NET SDK 10.0.103
  [Host]   : .NET 10.0.3 (10.0.3, 10.0.326.7603), X64 RyuJIT x86-64-v3
  ShortRun : .NET 10.0.3 (10.0.3, 10.0.326.7603), X64 RyuJIT x86-64-v3

Job=ShortRun  IterationCount=3  LaunchCount=1  
WarmupCount=3  

```
| Method                          | Mean         | Error         | StdDev      | Gen0         | Gen1      | Gen2      | Allocated   |
|-------------------------------- |-------------:|--------------:|------------:|-------------:|----------:|----------:|------------:|
| MathIR_Evaluate_Scalar          |     2.553 ms |     0.5547 ms |   0.0304 ms |     863.2813 |   27.3438 |         - |    13.82 MB |
| MathIR_Evaluate_Batch_1000Lanes | 3,543.146 ms | 1,861.1764 ms | 102.0174 ms | 1094000.0000 | 3000.0000 | 1000.0000 | 17468.09 MB |
