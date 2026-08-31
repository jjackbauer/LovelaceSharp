using Lovelace.Console.Repl;

internal static class Program
{
    public static async Task Main(string[] args)
    {
        var version = typeof(Program).Assembly.GetName().Version?.ToString(3) ?? "1.0.0";
        System.Console.WriteLine($"LovelaceSharp REPL v{version}");
        System.Console.WriteLine("Arbitrary-precision math scripting, vector math, and plotting.");
        System.Console.WriteLine("Type 'help' for a list of statements, operators, functions, and commands.");
        System.Console.WriteLine();

        await new ReplSession().RunAsync();
    }
}
