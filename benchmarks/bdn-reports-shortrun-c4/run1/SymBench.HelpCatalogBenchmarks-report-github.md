```

BenchmarkDotNet v0.15.8, Windows 11 (10.0.26100.4351/24H2/2024Update/HudsonValley)
AMD Ryzen 9 5900X 3.70GHz, 1 CPU, 24 logical and 12 physical cores
.NET SDK 10.0.103
  [Host]   : .NET 10.0.3 (10.0.3, 10.0.326.7603), X64 RyuJIT x86-64-v3
  ShortRun : .NET 10.0.3 (10.0.3, 10.0.326.7603), X64 RyuJIT x86-64-v3

Job=ShortRun  IterationCount=3  LaunchCount=1  
WarmupCount=3  

```
| Method                   | Mean      | Error     | StdDev    | Gen0   | Gen1   | Allocated |
|------------------------- |----------:|----------:|----------:|-------:|-------:|----------:|
| Help_Overview            |  4.893 μs | 8.5003 μs | 0.4659 μs | 0.7324 |      - |  12.07 KB |
| Help_Funcs_AllCategories | 10.303 μs | 0.4883 μs | 0.0268 μs | 3.1738 | 0.1068 |  51.97 KB |
