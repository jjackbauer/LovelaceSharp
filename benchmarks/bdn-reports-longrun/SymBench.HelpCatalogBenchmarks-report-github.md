```

BenchmarkDotNet v0.15.8, Windows 11 (10.0.26100.4351/24H2/2024Update/HudsonValley)
AMD Ryzen 9 5900X 3.70GHz, 1 CPU, 24 logical and 12 physical cores
.NET SDK 10.0.103
  [Host]    : .NET 10.0.3 (10.0.3, 10.0.326.7603), X64 RyuJIT x86-64-v3
  MediumRun : .NET 10.0.3 (10.0.3, 10.0.326.7603), X64 RyuJIT x86-64-v3

Job=MediumRun  IterationCount=15  LaunchCount=2  
WarmupCount=10  

```
| Method                   | Mean      | Error     | StdDev    | Gen0   | Gen1   | Allocated |
|------------------------- |----------:|----------:|----------:|-------:|-------:|----------:|
| Help_Overview            |  4.532 μs | 0.0742 μs | 0.1110 μs | 0.7324 | 0.0076 |  12.05 KB |
| Help_Funcs_AllCategories | 10.918 μs | 0.2135 μs | 0.3129 μs | 3.1586 | 0.1221 |  51.65 KB |
