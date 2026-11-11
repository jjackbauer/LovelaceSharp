using System.Reflection;
using System.Reflection.Metadata;
using System.Reflection.Metadata.Ecma335;
using System.Text.RegularExpressions;

namespace LovelaceParityCensus;

/// <summary>
/// Resolves a test method to its source file and line.
/// Primary source is the portable PDB emitted next to the test assembly (document + first
/// visible sequence point); the fallback is a deterministic scan of the project .cs files
/// for the declaration, used when the PDB has no sequence points for the method (for example
/// the kickoff method of an async test).
/// </summary>
internal sealed class SourceLocator : IDisposable
{
    private readonly string _repoRoot;
    private readonly string _projectDirectory;
    private readonly FileStream? _pdbStream;
    private readonly MetadataReaderProvider? _provider;
    private readonly MetadataReader? _reader;
    private readonly Dictionary<string, (string File, int Line)?> _scanCache = new(StringComparer.Ordinal);
    private string[]? _sourceFiles;

    public SourceLocator(string projectDirectory, string pdbPath, string repoRoot)
    {
        _repoRoot = repoRoot;
        _projectDirectory = projectDirectory;

        if (!File.Exists(pdbPath)) return;

        try
        {
            _pdbStream = File.OpenRead(pdbPath);
            _provider = MetadataReaderProvider.FromPortablePdbStream(_pdbStream);
            _reader = _provider.GetMetadataReader();
        }
        catch
        {
            _provider?.Dispose();
            _pdbStream?.Dispose();
            _pdbStream = null;
            _provider = null;
            _reader = null;
        }
    }

    public (string? File, int? Line, bool FromPdb) Locate(MethodInfo method)
    {
        (string? file, int? line) = FromPdb(method);
        if (file != null && line != null) return (file, line, true);

        (string? scanFile, int? scanLine) = FromSourceScan(method);
        return (scanFile, scanLine, false);
    }

    private (string? File, int? Line) FromPdb(MethodInfo method)
    {
        if (_reader == null) return (null, null);

        try
        {
            if (method.MetadataToken == 0) return (null, null);
            var handle = (MethodDefinitionHandle)MetadataTokens.Handle(method.MetadataToken);
            if (handle.IsNil) return (null, null);

            MethodDebugInformation debugInfo = _reader.GetMethodDebugInformation(handle);
            if (debugInfo.SequencePointsBlob.IsNil) return (null, null);

            foreach (SequencePoint point in debugInfo.GetSequencePoints())
            {
                if (point.IsHidden) continue;
                if (point.Document.IsNil) continue;
                Document document = _reader.GetDocument(point.Document);
                string name = _reader.GetString(document.Name);
                string? normalized = NormalizeDocument(name);
                if (normalized == null) continue;
                return (normalized, point.StartLine);
            }
        }
        catch
        {
            // fall through to the source scan; a missing line is never fatal
        }

        return (null, null);
    }

    private string? NormalizeDocument(string documentName)
    {
        if (string.IsNullOrEmpty(documentName)) return null;

        string normalized = documentName.Replace((char)92, (char)47);
        if (normalized.StartsWith("/_/", StringComparison.Ordinal))
        {
            return normalized.Substring(3);
        }

        string root = _repoRoot.Replace((char)92, (char)47);
        if (normalized.StartsWith(root + "/", StringComparison.OrdinalIgnoreCase))
        {
            return normalized.Substring(root.Length + 1);
        }

        return normalized;
    }

    private (string? File, int? Line) FromSourceScan(MethodInfo method)
    {
        if (method.DeclaringType == null) return (null, null);
        string className = method.DeclaringType.Name;
        int tick = className.IndexOf((char)96);
        if (tick >= 0) className = className.Substring(0, tick);

        string key = className + "::" + method.Name;
        if (_scanCache.TryGetValue(key, out (string File, int Line)? cached)) return (cached?.File, cached?.Line);

        (string File, int Line)? found = ScanProject(className, method.Name);
        _scanCache[key] = found;
        return (found?.File, found?.Line);
    }

    private (string File, int Line)? ScanProject(string className, string methodName)
    {
        string classPattern = @"\b(?:class|record|struct)\s+" + Regex.Escape(className) + @"\b";
        string methodPattern = @"\b" + Regex.Escape(methodName) + @"\s*\(";

        foreach (string file in SourceFiles())
        {
            string[] lines;
            try
            {
                lines = File.ReadAllLines(file);
            }
            catch
            {
                continue;
            }

            int classLine = -1;
            for (int i = 0; i < lines.Length; i++)
            {
                if (Regex.IsMatch(lines[i], classPattern)) { classLine = i; break; }
            }

            if (classLine < 0) continue;

            for (int i = classLine + 1; i < lines.Length; i++)
            {
                string line = lines[i];
                if (!Regex.IsMatch(line, methodPattern)) continue;
                string trimmed = line.Trim();
                if (trimmed.EndsWith(";", StringComparison.Ordinal)) continue;

                int index = line.IndexOf(methodName, StringComparison.Ordinal);
                if (index <= 0) continue;
                string prefix = line.Substring(0, index);
                if (!Regex.IsMatch(prefix, @"\b(?:public|private|protected|internal|static|async|override|virtual|new)\b")) continue;

                bool hasAttribute = false;
                for (int j = Math.Max(classLine + 1, i - 40); j < i; j++)
                {
                    if (Regex.IsMatch(lines[j], @"^\s*\[[^\]]*(?:Fact|Theory)"))
                    {
                        hasAttribute = true;
                        break;
                    }
                }

                if (!hasAttribute) continue;

                return (Path.GetRelativePath(_repoRoot, file).Replace((char)92, (char)47), i + 1);
            }
        }

        return null;
    }

    private string[] SourceFiles()
    {
        if (_sourceFiles != null) return _sourceFiles;

        var files = new List<string>();
        foreach (string file in Directory.EnumerateFiles(_projectDirectory, "*.cs", SearchOption.AllDirectories))
        {
            string normalized = file.Replace((char)92, (char)47);
            if (normalized.Contains("/bin/", StringComparison.Ordinal)) continue;
            if (normalized.Contains("/obj/", StringComparison.Ordinal)) continue;
            files.Add(file);
        }

        files.Sort(StringComparer.Ordinal);
        _sourceFiles = files.ToArray();
        return _sourceFiles;
    }

    public void Dispose()
    {
        _provider?.Dispose();
        _pdbStream?.Dispose();
    }
}
