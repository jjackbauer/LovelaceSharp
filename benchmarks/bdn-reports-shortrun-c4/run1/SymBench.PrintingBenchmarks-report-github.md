```

BenchmarkDotNet v0.15.8, Windows 11 (10.0.26100.4351/24H2/2024Update/HudsonValley)
AMD Ryzen 9 5900X 3.70GHz, 1 CPU, 24 logical and 12 physical cores
.NET SDK 10.0.103
  [Host]   : .NET 10.0.3 (10.0.3, 10.0.326.7603), X64 RyuJIT x86-64-v3
  ShortRun : .NET 10.0.3 (10.0.3, 10.0.326.7603), X64 RyuJIT x86-64-v3

Job=ShortRun  IterationCount=3  LaunchCount=1  
WarmupCount=3  

```
| Method                                   | Mean     | Error    | StdDev   | Gen0   | Allocated |
|----------------------------------------- |---------:|---------:|---------:|-------:|----------:|
| PrettyPrint_TranscendentalComposition    | 924.1 ns | 554.2 ns | 30.38 ns | 0.2441 |   3.99 KB |
| CanonicalPrint_TranscendentalComposition | 636.0 ns | 264.0 ns | 14.47 ns | 0.1898 |   3.11 KB |
