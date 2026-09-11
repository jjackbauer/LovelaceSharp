```

BenchmarkDotNet v0.15.8, Windows 11 (10.0.26100.4351/24H2/2024Update/HudsonValley)
AMD Ryzen 9 5900X 3.70GHz, 1 CPU, 24 logical and 12 physical cores
.NET SDK 10.0.103
  [Host]   : .NET 10.0.3 (10.0.3, 10.0.326.7603), X64 RyuJIT x86-64-v3
  ShortRun : .NET 10.0.3 (10.0.3, 10.0.326.7603), X64 RyuJIT x86-64-v3

Job=ShortRun  IterationCount=3  LaunchCount=1  
WarmupCount=3  

```
| Method                           | Mean     | Error     | StdDev   | Gen0    | Gen1   | Allocated |
|--------------------------------- |---------:|----------:|---------:|--------:|-------:|----------:|
| Multiply_SparseBivariateDegree20 | 320.3 μs | 182.77 μs | 10.02 μs | 58.1055 | 2.9297 | 949.23 KB |
| Groebner_Cyclic3_GrLex           | 132.4 μs |  55.73 μs |  3.05 μs | 31.0059 | 0.4883 | 508.58 KB |
