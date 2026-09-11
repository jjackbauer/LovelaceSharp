```

BenchmarkDotNet v0.15.8, Windows 11 (10.0.26100.4351/24H2/2024Update/HudsonValley)
AMD Ryzen 9 5900X 3.70GHz, 1 CPU, 24 logical and 12 physical cores
.NET SDK 10.0.103
  [Host]   : .NET 10.0.3 (10.0.3, 10.0.326.7603), X64 RyuJIT x86-64-v3
  ShortRun : .NET 10.0.3 (10.0.3, 10.0.326.7603), X64 RyuJIT x86-64-v3

Job=ShortRun  IterationCount=3  LaunchCount=1  
WarmupCount=3  

```
| Method          | Mean      | Error     | StdDev    | Gen0    | Allocated |
|---------------- |----------:|----------:|----------:|--------:|----------:|
| Diff_TenthOrder |  4.556 μs |  7.581 μs | 0.4156 μs |  0.9460 |  15.47 KB |
| Jacobian_4x4    | 64.137 μs |  8.862 μs | 0.4858 μs | 13.3057 | 218.97 KB |
| Hessian_4Var    | 77.079 μs | 20.098 μs | 1.1016 μs | 16.6016 | 272.52 KB |
