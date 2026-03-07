using Lovelace.Console.Repl;

internal static class Program
{
    public static void Main(string[] args)
    {
        var version = typeof(Program).Assembly.GetName().Version?.ToString(3) ?? "1.0.0";
        System.Console.WriteLine($"LovelaceSharp REPL v{version}");
        System.Console.WriteLine("Arbitrary-precision arithmetic calculator.");
        System.Console.WriteLine("Type 'help' for a list of operators, functions, and commands.");
        System.Console.WriteLine();

        new ReplSession().Run();
    }
}
