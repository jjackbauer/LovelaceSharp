```

BenchmarkDotNet v0.15.8, Windows 11 (10.0.26100.4351/24H2/2024Update/HudsonValley)
AMD Ryzen 9 5900X 3.70GHz, 1 CPU, 24 logical and 12 physical cores
.NET SDK 10.0.103
  [Host]   : .NET 10.0.3 (10.0.3, 10.0.326.7603), X64 RyuJIT x86-64-v3
  ShortRun : .NET 10.0.3 (10.0.3, 10.0.326.7603), X64 RyuJIT x86-64-v3

Job=ShortRun  IterationCount=3  LaunchCount=1  
WarmupCount=3  

```
| Method                 | Mean     | Error    | StdDev   | Gen0    | Gen1   | Allocated |
|----------------------- |---------:|---------:|---------:|--------:|-------:|----------:|
| Simplify_Safe          | 54.95 μs | 53.93 μs | 2.956 μs | 12.2681 | 0.1831 | 201.09 KB |
| Simplify_Full_TraceOff | 61.65 μs | 42.98 μs | 2.356 μs | 12.0850 | 0.1221 | 199.21 KB |
| Simplify_Full_TraceOn  | 60.01 μs | 89.16 μs | 4.887 μs | 12.1460 | 0.1831 | 199.24 KB |
