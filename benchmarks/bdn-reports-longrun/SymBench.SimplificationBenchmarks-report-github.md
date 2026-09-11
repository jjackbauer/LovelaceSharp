```

BenchmarkDotNet v0.15.8, Windows 11 (10.0.26100.4351/24H2/2024Update/HudsonValley)
AMD Ryzen 9 5900X 3.70GHz, 1 CPU, 24 logical and 12 physical cores
.NET SDK 10.0.103
  [Host]    : .NET 10.0.3 (10.0.3, 10.0.326.7603), X64 RyuJIT x86-64-v3
  MediumRun : .NET 10.0.3 (10.0.3, 10.0.326.7603), X64 RyuJIT x86-64-v3

Job=MediumRun  IterationCount=15  LaunchCount=2  
WarmupCount=10  

```
| Method                 | Mean     | Error    | StdDev   | Gen0    | Gen1   | Allocated |
|----------------------- |---------:|---------:|---------:|--------:|-------:|----------:|
| Simplify_Safe          | 57.16 μs | 0.620 μs | 0.869 μs | 12.2681 | 0.1831 | 201.09 KB |
| Simplify_Full_TraceOff | 56.06 μs | 0.775 μs | 1.160 μs | 12.1460 | 0.1831 | 199.24 KB |
| Simplify_Full_TraceOn  | 56.75 μs | 0.576 μs | 0.862 μs | 12.1460 | 0.1831 | 199.24 KB |
