using System.Numerics;
using Natural = global::Lovelace.Natural.Natural;
using Integer = global::Lovelace.Integer.Integer;

internal static class Program
{
    // Labels used to identify each operand in output (A, B, C, …)
    private static readonly string[] Labels = ["A", "B", "C"];

    public static void Main(string[] args) => RunMainMenu();

    // -----------------------------------------------------------------------
    // Top-level menu  (C++ equivalent: main())
    // -----------------------------------------------------------------------

    /// <summary>
    /// Displays the top-level menu and loops until the user selects Exit.
    /// </summary>
    private static void RunMainMenu()
    {
        while (true)
        {
            Console.WriteLine();
            Console.WriteLine("Select which Lovelace class to test:");
            Console.WriteLine("[1] - Natural");
            Console.WriteLine("[2] - Integer");
            Console.WriteLine("[0] - Exit");
            Console.Write(": ");

            string? input = Console.ReadLine()?.Trim();
            switch (input)
            {
                case "1":
                    RunNaturalMenu();
                    break;
                case "2":
                    RunIntegerMenu();
                    break;
                case "0":
                    return;
                default:
                    Console.WriteLine("Unknown option. Please enter 0, 1, or 2.");
                    break;
            }
        }
    }

    // -----------------------------------------------------------------------
    // Sub-menus (stubs — implemented in subsequent checklist items)
    // -----------------------------------------------------------------------

    private static void RunNaturalMenu()
    {
        Console.WriteLine();
        Console.WriteLine("Enter 3 Natural numbers to use in all tests.");
        Natural[] nums = ReadNumbers<Natural>(3, s => Natural.Parse(s, null));
        string[] labels = Labels[..3];

        for (int i = 0; i < nums.Length; i++)
            PrintParityInfo(labels[i], nums[i]);

        while (true)
        {
            Console.WriteLine();
            Console.WriteLine("Natural operations:");
            Console.WriteLine("[1] - Subtraction (A-B)");
            Console.WriteLine("[2] - Comparisons");
            Console.WriteLine("[3] - Addition (A+B)");
            Console.WriteLine("[4] - Multiplication (A*B)");
            Console.WriteLine("[5] - Exponentiation (A^B)");
            Console.WriteLine("[6] - DivRem (A/B and A%B)");
            Console.WriteLine("[7] - Factorial (A!)");
            Console.WriteLine("[8] - Modulo (A%B)");
            Console.WriteLine("[0] - Back");
            Console.Write(": ");

            string? option = Console.ReadLine()?.Trim();
            Console.WriteLine();
            switch (option)
            {
                case "1":
                    PrintAllPairs(nums, '-', (a, b) => a - b);
                    break;
                case "2":
                    PrintComparisons(nums, labels);
                    break;
                case "3":
                    PrintAllPairs(nums, '+', (a, b) => a + b);
                    break;
                case "4":
                    PrintAllPairs(nums, '*', (a, b) => a * b);
                    break;
                case "5":
                    PrintAllPairs(nums, '^', (a, b) => a.Pow(b));
                    break;
                case "6":
                    PrintDivRem(nums, labels, (a, b) => { var q = Natural.DivRem(a, b, out Natural rem); return (q, rem); });
                    break;
                case "7":
                    PrintFactorials(nums, labels, a => a.Factorial());
                    break;
                case "8":
                    PrintAllPairs(nums, '%', (a, b) => a % b);
                    break;
                case "0":
                    return;
                default:
                    Console.WriteLine("Unknown option.");
                    break;
            }
        }
    }

    private static void RunIntegerMenu()
    {
        Console.WriteLine();
        Console.WriteLine("Enter 3 Integer numbers to use in all tests.");
        Integer[] nums = ReadNumbers<Integer>(3, s => Integer.Parse(s, null));
        string[] labels = Labels[..3];

        for (int i = 0; i < nums.Length; i++)
            PrintIntegerSignInfo(labels[i], nums[i]);

        while (true)
        {
            Console.WriteLine();
            Console.WriteLine("Integer operations:");
            Console.WriteLine("[1] - Subtraction (A-B)");
            Console.WriteLine("[2] - Comparisons");
            Console.WriteLine("[3] - Addition (A+B)");
            Console.WriteLine("[4] - Multiplication (A*B)");
            Console.WriteLine("[5] - Exponentiation (A^B)");
            Console.WriteLine("[6] - DivRem (A/B and A%B)");
            Console.WriteLine("[7] - Factorial (A!)");
            Console.WriteLine("[8] - Modulo (A%B)");
            Console.WriteLine("[0] - Back");
            Console.Write(": ");

            string? option = Console.ReadLine()?.Trim();
            Console.WriteLine();
            switch (option)
            {
                case "1":
                    PrintAllPairs(nums, '-', (a, b) => a - b);
                    break;
                case "2":
                    PrintComparisons(nums, labels);
                    break;
                case "3":
                    PrintAllPairs(nums, '+', (a, b) => a + b);
                    break;
                case "4":
                    PrintAllPairs(nums, '*', (a, b) => a * b);
                    break;
                case "5":
                    PrintAllPairs(nums, '^', (a, b) => a.Pow(b));
                    break;
                case "6":
                    PrintDivRem(nums, labels, (a, b) => { var q = a.DivRem(b, out Integer rem); return (q, rem); });
                    break;
                case "7":
                    PrintFactorials(nums, labels, a => a.Factorial());
                    break;
                case "8":
                    PrintAllPairs(nums, '%', (a, b) => a % b);
                    break;
                case "0":
                    return;
                default:
                    Console.WriteLine("Unknown option.");
                    break;
            }
        }
    }

    // -----------------------------------------------------------------------
    // Infrastructure helpers (stubs — implemented in subsequent checklist items)
    // -----------------------------------------------------------------------

    private static T[] ReadNumbers<T>(int count, Func<string, T> parser)
    {
        Console.WriteLine($"Enter the values of {string.Join(", ", Labels[..count])} below.");
        T[] result = new T[count];
        for (int i = 0; i < count; i++)
        {
            while (true)
            {
                Console.Write($"{Labels[i]}: ");
                string? line = Console.ReadLine()?.Trim();
                if (line is null)
                    continue;
                try
                {
                    result[i] = parser(line);
                    break;
                }
                catch (Exception ex) when (ex is FormatException or OverflowException or ArgumentException)
                {
                    Console.WriteLine($"  Invalid input '{line}': {ex.Message}  Please try again.");
                }
            }
            Console.WriteLine();
        }
        return result;
    }

    private static void PrintParityInfo<T>(string label, T value)
        where T : INumberBase<T>
    {
        Console.WriteLine($"{label} ({value}) {(T.IsEvenInteger(value) ? "" : "not ")}is even");
        Console.WriteLine($"{label} ({value}) {(T.IsOddInteger(value) ? "" : "not ")}is odd");
    }

    private static void PrintIntegerSignInfo(string label, Integer value)
    {
        PrintParityInfo(label, value);
        Console.WriteLine($"{label} ({value}) {(Integer.IsPositive(value) ? "" : "not ")}is positive");
        Console.WriteLine($"{label} ({value}) {(Integer.IsNegative(value) ? "" : "not ")}is negative");
        Console.WriteLine($"{label} ({value}) sign = {value.Sign}");
    }

    private static void PrintAllPairs<T>(T[] nums, char opChar, Func<T, T, T> fn)
    {
        string[] labels = Labels[..nums.Length];
        for (int i = 0; i < nums.Length; i++)
        {
            for (int j = 0; j < nums.Length; j++)
            {
                try
                {
                    T result = fn(nums[i], nums[j]);
                    Console.WriteLine($"{labels[i]}{opChar}{labels[j]} = {result}");
                }
                catch (Exception ex)
                {
                    Console.WriteLine($"{labels[i]}{opChar}{labels[j]}: {ex.Message}");
                }
            }
        }
    }

    private static void PrintComparisons<T>(T[] nums, string[] labels)
        where T : IComparisonOperators<T, T, bool>, IEqualityOperators<T, T, bool>
    {
        // == pass
        for (int i = 0; i < nums.Length; i++)
            for (int j = 0; j < nums.Length; j++)
                Console.WriteLine($"{labels[i]} ({nums[i]}) {(nums[i] == nums[j] ? "" : "not ")}is equal to {labels[j]} ({nums[j]})");
        Console.WriteLine();

        // != pass
        for (int i = 0; i < nums.Length; i++)
            for (int j = 0; j < nums.Length; j++)
                Console.WriteLine($"{labels[i]} ({nums[i]}) {(nums[i] != nums[j] ? "" : "not ")}is different from {labels[j]} ({nums[j]})");
        Console.WriteLine();

        // > pass
        for (int i = 0; i < nums.Length; i++)
            for (int j = 0; j < nums.Length; j++)
                Console.WriteLine($"{labels[i]} ({nums[i]}) {(nums[i] > nums[j] ? "" : "not ")}is greater than {labels[j]} ({nums[j]})");
        Console.WriteLine();

        // < pass
        for (int i = 0; i < nums.Length; i++)
            for (int j = 0; j < nums.Length; j++)
                Console.WriteLine($"{labels[i]} ({nums[i]}) {(nums[i] < nums[j] ? "" : "not ")}is less than {labels[j]} ({nums[j]})");
        Console.WriteLine();

        // >= pass
        for (int i = 0; i < nums.Length; i++)
            for (int j = 0; j < nums.Length; j++)
                Console.WriteLine($"{labels[i]} ({nums[i]}) {(nums[i] >= nums[j] ? "" : "not ")}is greater than or equal to {labels[j]} ({nums[j]})");
        Console.WriteLine();

        // <= pass
        for (int i = 0; i < nums.Length; i++)
            for (int j = 0; j < nums.Length; j++)
                Console.WriteLine($"{labels[i]} ({nums[i]}) {(nums[i] <= nums[j] ? "" : "not ")}is less than or equal to {labels[j]} ({nums[j]})");
    }

    private static void PrintDivRem<T>(T[] nums, string[] labels, Func<T, T, (T quot, T rem)> divRem)
    {
        for (int i = 0; i < nums.Length; i++)
        {
            for (int j = 0; j < nums.Length; j++)
            {
                try
                {
                    var (quot, rem) = divRem(nums[i], nums[j]);
                    Console.WriteLine($"{labels[i]}/{labels[j]} = {quot}  {labels[i]}%{labels[j]} = {rem}");
                }
                catch (Exception ex)
                {
                    Console.WriteLine($"{labels[i]}/{labels[j]}: {ex.Message}");
                }
            }
        }
    }

    private static void PrintFactorials<T>(T[] nums, string[] labels, Func<T, T> factorial)
    {
        for (int i = 0; i < nums.Length; i++)
        {
            try
            {
                T result = factorial(nums[i]);
                Console.WriteLine($"{labels[i]}! = {result}");
            }
            catch (Exception ex)
            {
                Console.WriteLine($"{labels[i]}!: {ex.Message}");
            }
        }
    }
}
