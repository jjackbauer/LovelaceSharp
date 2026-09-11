```

BenchmarkDotNet v0.15.8, Windows 11 (10.0.26100.4351/24H2/2024Update/HudsonValley)
AMD Ryzen 9 5900X 3.70GHz, 1 CPU, 24 logical and 12 physical cores
.NET SDK 10.0.103
  [Host]   : .NET 10.0.3 (10.0.3, 10.0.326.7603), X64 RyuJIT x86-64-v3
  ShortRun : .NET 10.0.3 (10.0.3, 10.0.326.7603), X64 RyuJIT x86-64-v3

Job=ShortRun  IterationCount=3  LaunchCount=1  
WarmupCount=3  

```
| Method                         | Mean         | Error        | StdDev      | Gen0         | Gen1      | Gen2      | Allocated   |
|------------------------------- |-------------:|-------------:|------------:|-------------:|----------:|----------:|------------:|
| Kernel_EvaluateScalar          |     2.708 ms |     2.965 ms |   0.1625 ms |     863.2813 |   27.3438 |         - |    13.82 MB |
| Kernel_EvaluateBatch_1000Lanes | 3,721.074 ms | 3,486.079 ms | 191.0838 ms | 1094000.0000 | 3000.0000 | 1000.0000 | 17468.09 MB |
| TreeEvaluator_Direct           |     2.605 ms |     3.737 ms |   0.2048 ms |     867.1875 |   19.5313 |         - |    13.83 MB |
