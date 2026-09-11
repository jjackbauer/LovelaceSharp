```

BenchmarkDotNet v0.15.8, Windows 11 (10.0.26100.4351/24H2/2024Update/HudsonValley)
AMD Ryzen 9 5900X 3.70GHz, 1 CPU, 24 logical and 12 physical cores
.NET SDK 10.0.103
  [Host]    : .NET 10.0.3 (10.0.3, 10.0.326.7603), X64 RyuJIT x86-64-v3
  MediumRun : .NET 10.0.3 (10.0.3, 10.0.326.7603), X64 RyuJIT x86-64-v3

Job=MediumRun  IterationCount=15  LaunchCount=2  
WarmupCount=10  

```
| Method                     | Mean     | Error   | StdDev  | Gen0    | Gen1   | Allocated |
|--------------------------- |---------:|--------:|--------:|--------:|-------:|----------:|
| Factor_SexticRationalRoots | 187.6 μs | 1.65 μs | 2.26 μs | 46.1426 | 1.2207 | 755.16 KB |
