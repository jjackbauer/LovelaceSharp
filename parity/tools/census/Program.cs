using System.Collections;
using System.Globalization;
using System.Reflection;
using System.Security.Cryptography;
using System.Text;
using System.Text.Encodings.Web;
using System.Text.Json;

namespace LovelaceParityCensus;

internal sealed class CensusException : Exception
{
    public CensusException(string message) : base(message) { }
}

internal sealed record CaseRow(
    string Id,
    string Project,
    string Class,
    string Method,
    int CaseIndex,
    string DisplayName,
    string DataKind,
    string? SourceFile,
    int? SourceLine,
    string? DataUnavailable);

internal sealed record ProjectStats(string Name, string Assembly, int Methods, int Cases);

internal sealed class CensusOptions
{
    public string? Repo { get; private set; }
    public string? Out { get; private set; }
    public string? Oracle { get; private set; }
    public string? CapturedAt { get; private set; }
    public string Configuration { get; private set; } = "Debug";
    public string? Probe { get; private set; }
    public bool ShowHelp { get; private set; }

    public static CensusOptions Parse(string[] args)
    {
        var options = new CensusOptions();
        for (int i = 0; i < args.Length; i++)
        {
            string arg = args[i];
            string Next(string name)
            {
                if (i + 1 >= args.Length) throw new CensusException("missing value for " + name);
                return args[++i];
            }

            switch (arg)
            {
                case "--repo": options.Repo = Next(arg); break;
                case "--out": options.Out = Next(arg); break;
                case "--oracle": options.Oracle = Next(arg); break;
                case "--captured-at": options.CapturedAt = Next(arg); break;
                case "--configuration": options.Configuration = Next(arg); break;
                case "--probe": options.Probe = Next(arg); break;
                case "--help":
                case "-h": options.ShowHelp = true; break;
                default: throw new CensusException("unknown argument: " + arg);
            }
        }

        return options;
    }
}

internal static class Program
{
    private const string ToolName = "LovelaceParityCensus 1.0.0";
    private const int ArgumentLimit = 80;
    private const int DisplayLimit = 400;
    private const int ReasonLimit = 300;

    private static int Main(string[] args)
    {
        try
        {
            return Run(args);
        }
        catch (CensusException ex)
        {
            Console.Error.WriteLine("census: ERROR " + ex.Message);
            return 1;
        }
        catch (Exception ex)
        {
            Console.Error.WriteLine("census: FATAL " + ex);
            return 1;
        }
    }

    private static int Run(string[] args)
    {
        CensusOptions options = CensusOptions.Parse(args);
        if (options.ShowHelp)
        {
            Console.WriteLine("usage: census [--repo <path>] [--out <path>] [--oracle <path>]");
            Console.WriteLine("              [--captured-at <iso8601>] [--configuration Debug|Release] [--probe <assembly>]");
            return 0;
        }

        if (options.Probe != null) return Probe(options.Probe);

        string repo = Path.GetFullPath(options.Repo ?? FindRepoRoot());
        string oraclePath = Path.GetFullPath(options.Oracle ?? Path.Combine(repo, "parity", "oracle.json"));
        string outPath = Path.GetFullPath(options.Out ?? Path.Combine(repo, "parity", "census.json"));

        string commit;
        string capturedAtUtc;
        ReadOracle(oraclePath, out commit, out capturedAtUtc);
        if (options.CapturedAt != null) capturedAtUtc = options.CapturedAt;

        Console.WriteLine("census: tool=" + ToolName);
        Console.WriteLine("census: repo=" + repo);
        Console.WriteLine("census: oracle=" + oraclePath + " commit=" + commit + " capturedAtUtc=" + capturedAtUtc);

        var cases = new List<CaseRow>();
        var projects = new List<ProjectStats>();
        var idSet = new HashSet<string>(StringComparer.Ordinal);

        int factMethods = 0, theoryMethods = 0, inlineRows = 0, memberRows = 0, unavailable = 0;
        int sourceFromPdb = 0, sourceFromScan = 0, sourceMissing = 0;

        foreach (DirectoryInfo project in DiscoverTestProjects(repo))
        {
            string assemblyPath = Path.Combine(project.FullName, "bin", options.Configuration, "net10.0", project.Name + ".dll");
            if (!File.Exists(assemblyPath))
            {
                throw new CensusException(
                    "missing built assembly for " + project.Name + ": " + assemblyPath +
                    " (build it with: dotnet build " + project.Name + "/" + project.Name + ".csproj)");
            }

            string assemblyDirectory = Path.GetDirectoryName(assemblyPath) ?? project.FullName;

            // Reproduce the xunit testhost environment for this project: data sources that read
            // AppContext.BaseDirectory or the current directory must see the test output folder,
            // not the census tool's own folder.
            string originalDirectory = Directory.GetCurrentDirectory();
            AppDomain.CurrentDomain.SetData("APP_CONTEXT_BASE_DIRECTORY", assemblyDirectory);
            Directory.SetCurrentDirectory(assemblyDirectory);

            var loadContext = new TestAssemblyLoadContext(assemblyDirectory);
            Assembly assembly = loadContext.LoadFromAssemblyPath(assemblyPath);
            string assemblyRelative = RelativePosix(repo, assemblyPath);

            using var locator = new SourceLocator(project.FullName, Path.ChangeExtension(assemblyPath, ".pdb"), repo);

            int projectMethods = 0;
            int projectCases = 0;
            int projectInline = 0, projectMember = 0, projectUnavailable = 0;
            int projectPdb = 0, projectScan = 0, projectMissing = 0;

            foreach (Type type in GetLoadableTypes(assembly, project.Name))
            {
                if (type.FullName == null) continue;
                string className = type.FullName.Replace('+', '.');

                foreach (MethodInfo method in type.GetMethods(BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Instance | BindingFlags.Static))
                {
                    Attribute[] attributes = GetAttributes(method, className);
                    bool isTheory = attributes.Any(a => Derives(a.GetType(), "Xunit.TheoryAttribute"));
                    bool isFact = !isTheory && attributes.Any(a => Derives(a.GetType(), "Xunit.FactAttribute"));
                    if (!isTheory && !isFact) continue;

                    projectMethods++;
                    var location = locator.Locate(method);
                    if (location.FromPdb) projectPdb++;
                    else if (location.File != null) projectScan++;
                    else projectMissing++;

                    if (isFact)
                    {
                        factMethods++;
                        projectCases++;
                        AddCase(cases, idSet, project.Name, className, method.Name, 0,
                            className + "." + method.Name + "()", "fact",
                            location.File, location.Line, null);
                        continue;
                    }

                    theoryMethods++;
                    var dataAttributes = attributes.Where(a => IsDataAttribute(a.GetType())).ToArray();
                    var rows = new List<(string Kind, object?[] Arguments)>();
                    string? refusal = null;
                    string attemptedKind = dataAttributes.Length == 0 ? "none" : KindOf(dataAttributes[0].GetType());

                    if (dataAttributes.Length == 0)
                    {
                        refusal = "theory has no data attribute (InlineData/MemberData/ClassData)";
                    }
                    else
                    {
                        foreach (Attribute attribute in dataAttributes)
                        {
                            string kind = KindOf(attribute.GetType());
                            try
                            {
                                List<object?[]> enumerated = EnumerateData(attribute, method);
                                if (enumerated.Count == 0)
                                {
                                    refusal = DescribeAttribute(attribute) + " produced zero data rows";
                                    rows.Clear();
                                    break;
                                }

                                foreach (object?[] row in enumerated) rows.Add((kind, row));
                            }
                            catch (Exception ex)
                            {
                                refusal = DescribeAttribute(attribute) + " failed: " + DescribeException(ex);
                                rows.Clear();
                                break;
                            }
                        }

                        if (refusal == null && rows.Count == 0)
                        {
                            refusal = "data attributes produced zero rows";
                        }
                    }

                    if (refusal != null)
                    {
                        unavailable++;
                        projectCases++;
                        projectUnavailable++;
                        AddCase(cases, idSet, project.Name, className, method.Name, 0,
                            className + "." + method.Name + "(<unavailable>)", attemptedKind,
                            location.File, location.Line, refusal);
                        continue;
                    }

                    for (int i = 0; i < rows.Count; i++)
                    {
                        (string kind, object?[] rowArguments) = rows[i];
                        if (kind == "inline") { inlineRows++; projectInline++; }
                        else if (kind == "member") { memberRows++; projectMember++; }
                        else
                        {
                            throw new CensusException(
                                "unsupported data attribute kind '" + kind + "' on " + className + "." + method.Name +
                                "; the census schema only distinguishes inline/member rows");
                        }

                        projectCases++;
                        AddCase(cases, idSet, project.Name, className, method.Name, i,
                            className + "." + method.Name + "(" + FormatArguments(rowArguments) + ")", kind,
                            location.File, location.Line, null);
                    }
                }
            }

            projects.Add(new ProjectStats(project.Name, assemblyRelative, projectMethods, projectCases));
            Console.WriteLine(
                "census: project=" + project.Name +
                " assembly=" + assemblyRelative +
                " methods=" + projectMethods +
                " cases=" + projectCases +
                " inline=" + projectInline +
                " member=" + projectMember +
                " unavailable=" + projectUnavailable +
                " source[pdb=" + projectPdb + " scan=" + projectScan + " missing=" + projectMissing + "]");
            sourceFromPdb += projectPdb;
            sourceFromScan += projectScan;
            sourceMissing += projectMissing;

            Directory.SetCurrentDirectory(originalDirectory);
        }

        projects.Sort((a, b) => string.CompareOrdinal(a.Name, b.Name));
        cases.Sort((a, b) => string.CompareOrdinal(a.Id, b.Id));

        int testCases = cases.Count;
        int testMethods = factMethods + theoryMethods;
        if (testCases != factMethods + inlineRows + memberRows + unavailable)
        {
            throw new CensusException(
                "internal reconciliation failed: cases=" + testCases + " facts=" + factMethods +
                " inline=" + inlineRows + " member=" + memberRows + " unavailable=" + unavailable);
        }

        WriteJson(outPath, commit, capturedAtUtc, projects, cases,
            new Totals(projects.Count, testMethods, testCases, factMethods, theoryMethods, inlineRows, memberRows, unavailable));

        byte[] bytes = File.ReadAllBytes(outPath);
        string sha = Convert.ToHexString(SHA256.HashData(bytes)).ToLowerInvariant();

        Console.WriteLine(
            "census: totals projects=" + projects.Count +
            " testMethods=" + testMethods +
            " testCases=" + testCases +
            " factMethods=" + factMethods +
            " theoryMethods=" + theoryMethods +
            " inlineDataRows=" + inlineRows +
            " memberDataRows=" + memberRows +
            " dataUnavailable=" + unavailable);
        Console.WriteLine("census: source total[pdb=" + sourceFromPdb + " scan=" + sourceFromScan + " missing=" + sourceMissing + "]");
        Console.WriteLine("census: wrote " + outPath + " bytes=" + bytes.Length + " sha256=" + sha);
        return 0;
    }


    /// <summary>Diagnostic mode: dump the xunit attribute types visible to one built assembly.</summary>
    private static int Probe(string assemblyPath)
    {
        string full = Path.GetFullPath(assemblyPath);
        var loadContext = new TestAssemblyLoadContext(Path.GetDirectoryName(full) ?? ".");
        Assembly assembly = loadContext.LoadFromAssemblyPath(full);
        foreach (Type type in assembly.GetTypes())
        {
            if (type.FullName == null || !type.FullName.Contains("Attribute", StringComparison.Ordinal)) continue;
            if (!type.FullName.Contains("Data", StringComparison.Ordinal) && !type.FullName.Contains("Fact", StringComparison.Ordinal) && !type.FullName.Contains("Theory", StringComparison.Ordinal)) continue;
            var chain = new List<string>();
            for (Type? current = type; current != null; current = current.BaseType) chain.Add(current.FullName ?? current.Name);
            Console.WriteLine("type " + type.FullName + " <- " + string.Join(" <- ", chain));
        }

        foreach (Type type in assembly.GetTypes())
        {
            foreach (MethodInfo method in type.GetMethods(BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Instance | BindingFlags.Static))
            {
                object[] attributes = method.GetCustomAttributes(inherit: false);
                if (!attributes.Any(a => a.GetType().FullName == "Xunit.TheoryAttribute")) continue;
                Console.WriteLine("method " + type.FullName + "." + method.Name);
                foreach (object attribute in attributes)
                {
                    var chain = new List<string>();
                    for (Type? current = attribute.GetType(); current != null; current = current.BaseType) chain.Add(current.FullName ?? current.Name);
                    Console.WriteLine("  attr " + attribute.GetType().FullName + " <- " + string.Join(" <- ", chain));
                    MethodInfo? getData = attribute.GetType().GetMethod("GetData", new[] { typeof(MethodInfo) });
                    Console.WriteLine("    GetData=" + (getData == null ? "MISSING" : "present"));
                    if (getData != null)
                    {
                        try
                        {
                            object? data = getData.Invoke(attribute, new object?[] { method });
                            int rows = 0;
                            if (data is IEnumerable sequence) foreach (object? _ in sequence) rows++;
                            Console.WriteLine("    rows=" + rows);
                        }
                        catch (Exception ex)
                        {
                            Console.WriteLine("    THREW " + DescribeException(ex));
                        }
                    }
                }

                return 0;
            }
        }

        return 0;
    }

    private sealed record Totals(int Projects, int TestMethods, int TestCases, int FactMethods, int TheoryMethods, int InlineDataRows, int MemberDataRows, int DataUnavailable);

    private static void AddCase(
        List<CaseRow> cases,
        HashSet<string> idSet,
        string project,
        string className,
        string method,
        int caseIndex,
        string displayName,
        string dataKind,
        string? sourceFile,
        int? sourceLine,
        string? dataUnavailable)
    {
        string id = project + "/" + className + "/" + method + "#" + caseIndex.ToString(CultureInfo.InvariantCulture);
        if (!idSet.Add(id))
        {
            throw new CensusException("duplicate case id '" + id + "' (overloaded test method with the same name?)");
        }

        if (displayName.Length > DisplayLimit) displayName = displayName.Substring(0, DisplayLimit - 3) + "...";
        cases.Add(new CaseRow(id, project, className, method, caseIndex, displayName, dataKind, sourceFile, sourceLine, dataUnavailable));
    }

    private static List<object?[]> EnumerateData(Attribute attribute, MethodInfo method)
    {
        MethodInfo? getData = attribute.GetType().GetMethod(
            "GetData",
            BindingFlags.Public | BindingFlags.Instance,
            binder: null,
            types: new[] { typeof(MethodInfo) },
            modifiers: null);
        if (getData == null)
        {
            throw new InvalidOperationException("no public GetData(MethodInfo) on " + attribute.GetType().FullName);
        }

        object? result;
        try
        {
            result = getData.Invoke(attribute, new object?[] { method });
        }
        catch (TargetInvocationException ex) when (ex.InnerException != null)
        {
            throw ex.InnerException;
        }

        if (result is not IEnumerable sequence)
        {
            throw new InvalidOperationException("GetData returned " + (result == null ? "null" : result.GetType().FullName));
        }

        var rows = new List<object?[]>();
        foreach (object? item in sequence)
        {
            rows.Add(item as object?[] ?? new object?[] { item });
        }

        return rows;
    }

    private static string KindOf(Type attributeType)
    {
        if (attributeType.FullName == "Xunit.InlineDataAttribute") return "inline";
        if (attributeType.FullName == "Xunit.MemberDataAttribute") return "member";
        if (attributeType.FullName == "Xunit.ClassDataAttribute") return "class";
        return "other";
    }

    private static string DescribeAttribute(Attribute attribute)
    {
        Type type = attribute.GetType();
        string label = type.Name;
        try
        {
            PropertyInfo? memberName = type.GetProperty("MemberName", BindingFlags.Public | BindingFlags.Instance);
            if (memberName?.GetValue(attribute) is string name && name.Length > 0)
            {
                label += " '" + name + "'";
            }
        }
        catch
        {
            // best effort only: a diagnostic label must never mask the real failure
        }

        return label;
    }

    private static string DescribeException(Exception exception)
    {
        Exception inner = exception;
        while (inner is TargetInvocationException && inner.InnerException != null) inner = inner.InnerException;
        string message = (inner.Message ?? string.Empty).Replace("\r", " ").Replace("\n", " ").Trim();
        if (message.Length > ReasonLimit) message = message.Substring(0, ReasonLimit - 3) + "...";
        return inner.GetType().FullName + ": " + message;
    }

    private static string FormatArguments(object?[] arguments)
    {
        var parts = new string[arguments.Length];
        for (int i = 0; i < arguments.Length; i++) parts[i] = FormatArgument(arguments[i]);
        return string.Join(", ", parts);
    }

    private static string FormatArgument(object? value)
    {
        string text;
        try
        {
            text = FormatCore(value);
        }
        catch
        {
            text = value == null ? "null" : value.GetType().Name;
        }

        if (text.Length > ArgumentLimit) text = text.Substring(0, ArgumentLimit - 3) + "...";
        return text;
    }

    private static string FormatCore(object? value)
    {
        switch (value)
        {
            case null: return "null";
            case string s: return "\"" + s.Replace("\\", "\\\\").Replace("\"", "\\\"").Replace("\r", "\\r").Replace("\n", "\\n") + "\"";
            case char c: return "'" + c + "'";
            case bool b: return b ? "true" : "false";
            case Type t: return t.FullName ?? t.Name;
            case Array array:
            {
                var items = new List<string>();
                int count = 0;
                foreach (object? item in array)
                {
                    if (count++ == 8) { items.Add("..."); break; }
                    items.Add(FormatArgument(item));
                }

                return "[" + string.Join(", ", items) + "]";
            }
            case IFormattable formattable:
                return formattable.ToString(null, CultureInfo.InvariantCulture) ?? value.GetType().Name;
            default:
                return value.ToString() ?? value.GetType().Name;
        }
    }

    private static Attribute[] GetAttributes(MethodInfo method, string className)
    {
        try
        {
            object[] raw = method.GetCustomAttributes(inherit: false);
            return raw.OfType<Attribute>().ToArray();
        }
        catch (Exception ex)
        {
            throw new CensusException("failed to read attributes of " + className + "." + method.Name + ": " + DescribeException(ex));
        }
    }

    private static bool IsDataAttribute(Type type)
    {
        for (Type? current = type; current != null; current = current.BaseType)
        {
            if (current.FullName == "Xunit.DataAttribute") return true;
            if (current.FullName == "Xunit.Sdk.DataAttribute") return true;
        }

        return false;
    }

    private static bool Derives(Type type, string fullName)
    {
        for (Type? current = type; current != null; current = current.BaseType)
        {
            if (current.FullName == fullName) return true;
        }

        return false;
    }

    private static Type[] GetLoadableTypes(Assembly assembly, string project)
    {
        try
        {
            return assembly.GetTypes();
        }
        catch (ReflectionTypeLoadException ex)
        {
            var details = ex.LoaderExceptions
                .Where(e => e != null)
                .Select(e => e!.Message)
                .Distinct(StringComparer.Ordinal)
                .Take(5);
            throw new CensusException(
                "type load failure in " + project + " (" + ex.LoaderExceptions.Count(e => e != null) + " loader exceptions): " +
                string.Join(" | ", details));
        }
    }

    private static IEnumerable<DirectoryInfo> DiscoverTestProjects(string repo)
    {
        var projects = new List<DirectoryInfo>();
        foreach (string directory in Directory.EnumerateDirectories(repo, "*.Tests", SearchOption.TopDirectoryOnly))
        {
            var info = new DirectoryInfo(directory);
            if (File.Exists(Path.Combine(info.FullName, info.Name + ".csproj"))) projects.Add(info);
            else Console.Error.WriteLine("census: WARNING directory " + info.Name + " has no matching " + info.Name + ".csproj; skipped");
        }

        projects.Sort((a, b) => string.CompareOrdinal(a.Name, b.Name));
        if (projects.Count == 0) throw new CensusException("no *.Tests projects found under " + repo);
        return projects;
    }

    private static void ReadOracle(string oraclePath, out string commit, out string capturedAtUtc)
    {
        if (!File.Exists(oraclePath))
        {
            throw new CensusException("missing " + oraclePath + " (needed for the pinned commit and capturedAtUtc)");
        }

        using JsonDocument document = JsonDocument.Parse(File.ReadAllBytes(oraclePath));
        JsonElement root = document.RootElement;
        commit = root.TryGetProperty("commit", out JsonElement commitElement) ? commitElement.GetString() ?? string.Empty : string.Empty;
        capturedAtUtc = root.TryGetProperty("capturedAtUtc", out JsonElement capturedElement) ? capturedElement.GetString() ?? string.Empty : string.Empty;
        if (commit.Length == 0) throw new CensusException(oraclePath + " has no 'commit'");
        if (capturedAtUtc.Length == 0) throw new CensusException(oraclePath + " has no 'capturedAtUtc'");
    }

    private static string FindRepoRoot()
    {
        var directory = new DirectoryInfo(AppContext.BaseDirectory);
        while (directory != null)
        {
            if (File.Exists(Path.Combine(directory.FullName, "LovelaceSharp.slnx"))) return directory.FullName;
            directory = directory.Parent;
        }

        throw new CensusException("could not locate the repository root from " + AppContext.BaseDirectory + "; pass --repo");
    }

    private static string RelativePosix(string root, string path)
    {
        string relative = Path.GetRelativePath(root, path);
        return relative.Replace('\\', '/');
    }

    private static void WriteJson(
        string outPath,
        string commit,
        string capturedAtUtc,
        List<ProjectStats> projects,
        List<CaseRow> cases,
        Totals totals)
    {
        string? directory = Path.GetDirectoryName(outPath);
        if (!string.IsNullOrEmpty(directory)) Directory.CreateDirectory(directory);

        using var buffer = new MemoryStream();
        var writerOptions = new JsonWriterOptions
        {
            Indented = true,
            NewLine = "\n",
            Encoder = JavaScriptEncoder.UnsafeRelaxedJsonEscaping,
        };

        using (var writer = new Utf8JsonWriter(buffer, writerOptions))
        {
            writer.WriteStartObject();
            writer.WriteNumber("schemaVersion", 1);

            writer.WriteStartObject("oracle");
            writer.WriteString("commit", commit);
            writer.WriteString("capturedAtUtc", capturedAtUtc);
            writer.WriteEndObject();

            writer.WriteStartObject("generator");
            writer.WriteString("tool", ToolName);
            writer.WriteString("method", "reflection");
            writer.WriteEndObject();

            writer.WriteStartObject("totals");
            writer.WriteNumber("projects", totals.Projects);
            writer.WriteNumber("testMethods", totals.TestMethods);
            writer.WriteNumber("testCases", totals.TestCases);
            writer.WriteNumber("factMethods", totals.FactMethods);
            writer.WriteNumber("theoryMethods", totals.TheoryMethods);
            writer.WriteNumber("inlineDataRows", totals.InlineDataRows);
            writer.WriteNumber("memberDataRows", totals.MemberDataRows);
            writer.WriteNumber("dataUnavailable", totals.DataUnavailable);
            writer.WriteEndObject();

            writer.WriteStartArray("projects");
            foreach (ProjectStats project in projects)
            {
                writer.WriteStartObject();
                writer.WriteString("name", project.Name);
                writer.WriteString("assembly", project.Assembly);
                writer.WriteNumber("methods", project.Methods);
                writer.WriteNumber("cases", project.Cases);
                writer.WriteEndObject();
            }

            writer.WriteEndArray();

            writer.WriteStartArray("cases");
            foreach (CaseRow item in cases)
            {
                writer.WriteStartObject();
                writer.WriteString("id", item.Id);
                writer.WriteString("project", item.Project);
                writer.WriteString("class", item.Class);
                writer.WriteString("method", item.Method);
                writer.WriteNumber("caseIndex", item.CaseIndex);
                writer.WriteString("displayName", item.DisplayName);
                writer.WriteString("dataKind", item.DataKind);
                if (item.SourceFile == null) writer.WriteNull("sourceFile");
                else writer.WriteString("sourceFile", item.SourceFile);
                if (item.SourceLine.HasValue) writer.WriteNumber("sourceLine", item.SourceLine.Value);
                else writer.WriteNull("sourceLine");
                if (item.DataUnavailable == null) writer.WriteNull("dataUnavailable");
                else writer.WriteString("dataUnavailable", item.DataUnavailable);
                writer.WriteEndObject();
            }

            writer.WriteEndArray();
            writer.WriteEndObject();
        }

        string json = Encoding.UTF8.GetString(buffer.ToArray()).Replace("\r\n", "\n");
        if (!json.EndsWith("\n", StringComparison.Ordinal)) json += "\n";
        File.WriteAllText(outPath, json, new UTF8Encoding(encoderShouldEmitUTF8Identifier: false));
    }

    private sealed class TestAssemblyLoadContext : System.Runtime.Loader.AssemblyLoadContext
    {
        private readonly string _directory;

        public TestAssemblyLoadContext(string directory) : base("census:" + directory, isCollectible: false)
        {
            _directory = directory;
        }

        protected override Assembly? Load(AssemblyName assemblyName)
        {
            if (assemblyName.Name == null) return null;
            string candidate = Path.Combine(_directory, assemblyName.Name + ".dll");
            return File.Exists(candidate) ? LoadFromAssemblyPath(candidate) : null;
        }
    }
}
