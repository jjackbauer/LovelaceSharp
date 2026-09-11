```

BenchmarkDotNet v0.15.8, Windows 11 (10.0.26100.4351/24H2/2024Update/HudsonValley)
AMD Ryzen 9 5900X 3.70GHz, 1 CPU, 24 logical and 12 physical cores
.NET SDK 10.0.103
  [Host]   : .NET 10.0.3 (10.0.3, 10.0.326.7603), X64 RyuJIT x86-64-v3
  ShortRun : .NET 10.0.3 (10.0.3, 10.0.326.7603), X64 RyuJIT x86-64-v3

Job=ShortRun  IterationCount=3  LaunchCount=1  
WarmupCount=3  

```
| Method   | Mean          | Error       | StdDev     | Gen0          | Gen1          | Gen2          | Allocated     |
|--------- |--------------:|------------:|-----------:|--------------:|--------------:|--------------:|--------------:|
| Eval64   |      10.06 ms |    20.62 ms |   1.130 ms |     2578.1250 |      140.6250 |             - |      41.36 MB |
| Eval256  |     731.86 ms |    88.91 ms |   4.873 ms |   169000.0000 |    10000.0000 |     1000.0000 |    2616.21 MB |
| Eval1024 | 151,240.63 ms | 3,519.90 ms | 192.937 ms | 76995000.0000 | 23876000.0000 | 10810000.0000 | 1106141.56 MB |
