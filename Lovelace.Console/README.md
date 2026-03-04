# Lovelace.Console

An interactive menu-driven console application that exercises the `Natural` and `Integer` APIs, mirroring the `testes_Lovelace()` and `testes_InteiroLovelace()` routines from `Legacy/main.cpp`.

## Class: Program

**Namespace**: *(top-level / internal)*

`Program` is a static entry-point class. It presents a top-level menu letting the user choose between the Natural and Integer test suites, prompts for three operands, then offers eight arithmetic operations to run against those operands. All output is written to `Console.Out`; exceptions (e.g. subtraction below zero, division by zero, factorial of a negative) are caught and their messages printed rather than crashing the application.

---

## Public API

### Entry Point

| Member | Signature | Description |
|---|---|---|
| `Main` | `static void Main(string[] args)` | Application entry point. Immediately delegates to `RunMainMenu()`. |

### Private helpers (interaction model)

| Member | Description |
|---|---|
| `RunMainMenu()` | Displays the top-level menu (`[1] Natural`, `[2] Integer`, `[0] Exit`) and loops until Exit. |
| `RunNaturalMenu()` | Reads three `Natural` operands, prints parity info for each, then loops over the Natural operation sub-menu (tests 1–8). |
| `RunIntegerMenu()` | Reads three `Integer` operands, prints parity and sign info for each, then loops over the Integer operation sub-menu (tests 1–8). |
| `ReadNumbers<T>(int count, Func<string,T> parser)` | Prompts for `count` values, parses each with the supplied delegate, re-prompts on parse error. Returns `T[]`. |
| `PrintParityInfo<T>(string label, T value)` | Prints `"<label> (<value>) [not] is even"` and `"[not] is odd"` for any `INumberBase<T>`. |
| `PrintIntegerSignInfo(string label, Integer value)` | Calls `PrintParityInfo`, then prints `"[not] is positive"`, `"[not] is negative"`, and `"sign = <0/-1/1>"`. |
| `PrintAllPairs<T>(T[] nums, char opChar, Func<T,T,T> fn)` | Prints `"Ai op Aj = result"` for every ordered pair; catches and prints exception messages. |
| `PrintComparisons<T>(T[] nums, string[] labels)` | Prints six passes (`==`, `!=`, `>`, `<`, `>=`, `<=`) for every ordered pair. Requires `IComparisonOperators<T,T,bool>` and `IEqualityOperators<T,T,bool>`. |
| `PrintDivRem<T>(T[] nums, string[] labels, Func<T,T,(T quot,T rem)> divRem)` | Prints `"A/B = quot  A%B = rem"` for every ordered pair; catches and prints exception messages. |
| `PrintFactorials<T>(T[] nums, string[] labels, Func<T,T> factorial)` | Prints `"A! = result"` for each operand; catches and prints exception messages. |

### Natural operations (sub-menu items)

| Option | Operation | Notes |
|---|---|---|
| 1 | Subtraction (`A-B`) | Catches `InvalidOperationException` when result would be negative |
| 2 | Comparisons | All six comparison operators via `PrintComparisons` |
| 3 | Addition (`A+B`) | — |
| 4 | Multiplication (`A*B`) | — |
| 5 | Exponentiation (`A^B`) | Uses `Natural.Pow(Natural)` |
| 6 | DivRem | Uses `Natural.DivRem(Natural, Natural, out Natural)`; catches `DivideByZeroException` |
| 7 | Factorial (`A!`) | Uses `Natural.Factorial()`; catches exceptions |
| 8 | Modulo (`A%B`) | Catches `DivideByZeroException` |

### Integer operations (sub-menu items)

| Option | Operation | Notes |
|---|---|---|
| 1 | Subtraction (`A-B`) | Signed; no range restriction |
| 2 | Comparisons | All six comparison operators via `PrintComparisons` |
| 3 | Addition (`A+B`) | — |
| 4 | Multiplication (`A*B`) | — |
| 5 | Exponentiation (`A^B`) | Uses `Integer.Pow(Integer)` |
| 6 | DivRem | Uses `Integer.DivRem(Integer, out Integer)`; catches `DivideByZeroException` |
| 7 | Factorial (`A!`) | Uses `Integer.Factorial()`; catches `InvalidOperationException` for negative inputs |
| 8 | Modulo (`A%B`) | Catches `DivideByZeroException` |

---

## Usage

```bash
dotnet run --project Lovelace.Console
```

Sample session:
```
Select which Lovelace class to test:
[1] - Natural
[2] - Integer
[0] - Exit
: 1

Enter the values of A, B, C below.
A: 10
B: 3
C: 7

A (10) is even
A (10) not is odd
B (3) not is even
B (3) is odd
...

Natural operations:
[3] - Addition (A+B)
...
: 3

A+A = 20
A+B = 13
A+C = 17
...
```

---

## See also

- Requirements: [`.github/requirements/Lovelace.Console.md`](../.github/requirements/Lovelace.Console.md)
- C++ reference: [`Legacy/main.cpp`](../Legacy/main.cpp)
- Library used: [`Lovelace.Natural`](../Lovelace.Natural/README.md), [`Lovelace.Integer`](../Lovelace.Integer/README.md)
