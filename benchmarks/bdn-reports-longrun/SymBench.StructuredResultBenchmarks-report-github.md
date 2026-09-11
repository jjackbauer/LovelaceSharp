```

BenchmarkDotNet v0.15.8, Windows 11 (10.0.26100.4351/24H2/2024Update/HudsonValley)
AMD Ryzen 9 5900X 3.70GHz, 1 CPU, 24 logical and 12 physical cores
.NET SDK 10.0.103
  [Host]    : .NET 10.0.3 (10.0.3, 10.0.326.7603), X64 RyuJIT x86-64-v3
  MediumRun : .NET 10.0.3 (10.0.3, 10.0.326.7603), X64 RyuJIT x86-64-v3

Job=MediumRun  IterationCount=15  LaunchCount=2  
WarmupCount=10  

```
| Method                                         | Mean          | Error       | StdDev      | Gen0    | Gen1   | Allocated |
|----------------------------------------------- |--------------:|------------:|------------:|--------:|-------:|----------:|
| Record_Construct_SolveResult                   |  1,081.541 ns |  21.2296 ns |  30.4469 ns |  0.2918 | 0.0038 |    4912 B |
| Record_FieldLookup_ByName                      |      4.409 ns |   0.0244 ns |   0.0325 ns |       - |      - |         - |
| Record_MemberAccess_Solutions                  |    744.635 ns |  34.2562 ns |  50.2124 ns |  0.1497 |      - |    2512 B |
| Json_SerializeSolveResult                      |  3,492.425 ns |  19.9873 ns |  28.0195 ns |  0.4730 | 0.0038 |    7969 B |
| Json_SerializeSolveResult_ShippedProjection    |  3,822.354 ns |  64.0355 ns |  91.8378 ns |  0.6180 | 0.0076 |   10353 B |
| Json_SerializeSolveResult_ShippedProjection_64 | 61,003.621 ns | 432.1121 ns | 591.4800 ns | 10.2539 |      - |  172603 B |
