```

BenchmarkDotNet v0.15.8, Windows 11 (10.0.26100.4351/24H2/2024Update/HudsonValley)
AMD Ryzen 9 5900X 3.70GHz, 1 CPU, 24 logical and 12 physical cores
.NET SDK 10.0.103
  [Host]   : .NET 10.0.3 (10.0.3, 10.0.326.7603), X64 RyuJIT x86-64-v3
  ShortRun : .NET 10.0.3 (10.0.3, 10.0.326.7603), X64 RyuJIT x86-64-v3

Job=ShortRun  IterationCount=3  LaunchCount=1  
WarmupCount=3  

```
| Method                             | Mean     | Error    | StdDev    | Gen0     | Gen1    | Gen2    | Allocated |
|----------------------------------- |---------:|---------:|----------:|---------:|--------:|--------:|----------:|
| Construct_SharedDAG                | 2.901 ms | 3.074 ms | 0.1685 ms | 789.0625 | 82.0313 | 39.0625 |  12.73 MB |
| ConstructAndCanonicalize_SharedDAG | 3.052 ms | 7.141 ms | 0.3914 ms | 792.9688 | 82.0313 | 39.0625 |  12.78 MB |
