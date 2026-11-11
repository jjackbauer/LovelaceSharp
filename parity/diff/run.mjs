#!/usr/bin/env node
// run.mjs — fail-closed differential gate for the C# oracle vs the Rust runner.
// Dependency-free: Node 24 built-ins only.
//
// Exit codes:
//   0  every executed case matched and both binaries executed
//   1  at least one mismatch (generated case or replayed regression)
//   2  harness/config failure: missing binary, spawn failure, --cases 0,
//      unreadable regression, unknown option, empty corpus, selftest failure

import { spawn } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdir, readFile, readdir, stat, writeFile } from "node:fs/promises";
import path from "node:path";

import { CATEGORIES, corpusHash, generateCases, describeCategories } from "./generator.mjs";
import { VOLATILE_FIELDS, classifyCase, compareEnvelopes, normalize } from "./compare.mjs";
import { minimizeCase } from "./minimize.mjs";

const HERE = import.meta.dirname;
const ROOT = path.resolve(HERE, "..", "..");
const ORACLE_DLL = path.join(ROOT, "Lovelace.Run", "bin", "Release", "net10.0", "Lovelace.Run.dll");
const RUST_EXE = path.join(ROOT, "rust", "target", "release", "lovelace-run.exe");
const REGRESSIONS_DIR = path.join(HERE, "regressions");
const RUNS_DIR = path.join(HERE, "runs");

const SCHEMA_VERSION = 1;
const CASE_TIMEOUT_MS = 30_000;
const BUILD_HINT = "cargo build --release --manifest-path rust/Cargo.toml (use .toolchain\\cargo\\bin\\cargo.exe)";

const EXIT_MATCH = 0;
const EXIT_MISMATCH = 1;
const EXIT_HARNESS = 2;

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------

const USAGE = [
  "parity/diff/run.mjs — differential gate (C# oracle vs Rust runner)",
  "",
  "Usage: node parity/diff/run.mjs [options]",
  "",
  "  --cases N        generated cases (default 200). 0 is a harness failure (exit 2).",
  "  --seed S         integer seed (default 1)",
  "  --concurrency K  parallel case workers (default 6)",
  "  --out DIR        run directory (default parity/diff/runs/<utc-stamp>-seed<S>)",
  "  --filter TEXT    only cases whose id contains TEXT",
  "  --no-minimize    skip minimisation (the full input is still recorded)",
  "  --regressions    run only parity/diff/regressions/*.json",
  "  --list           print the category list and exit 0",
  "  --selftest       prove the harness can fail (exit 0 only if it can)",
  "  --help           this text",
  "",
  "Exit codes: 0 match, 1 mismatch, 2 harness/config failure.",
].join("\n");

function usageError(message) {
  process.stderr.write("HARNESS ERROR: " + message + "\n");
  process.stderr.write(USAGE + "\n");
  process.exit(EXIT_HARNESS);
}

function parseArgs(argv) {
  const options = {
    cases: 200,
    seed: 1,
    concurrency: 6,
    out: null,
    filter: null,
    minimize: true,
    regressionsOnly: false,
    list: false,
    selftest: false,
    help: false,
    selftestChild: null,
  };

  const takesValue = new Set(["--cases", "--seed", "--concurrency", "--out", "--filter"]);

  for (let i = 0; i < argv.length; i += 1) {
    let arg = argv[i];
    let inlineValue = null;
    const eq = arg.indexOf("=");
    if (arg.startsWith("--") && eq !== -1) {
      inlineValue = arg.slice(eq + 1);
      arg = arg.slice(0, eq);
    }
    const nextValue = () => {
      if (inlineValue !== null) return inlineValue;
      i += 1;
      if (i >= argv.length) usageError("option " + arg + " requires a value");
      return argv[i];
    };

    if (arg === "--selftest-child") {
      options.selftestChild = nextValue();
      continue;
    }

    if (takesValue.has(arg)) {
      const value = nextValue();
      if (arg === "--cases") {
        if (!/^[0-9]+$/.test(value)) usageError("--cases requires a non-negative integer, got '" + value + "'");
        options.cases = Number(value);
      } else if (arg === "--seed") {
        if (!/^-?[0-9]+$/.test(value)) usageError("--seed requires an integer, got '" + value + "'");
        options.seed = Number(value);
      } else if (arg === "--concurrency") {
        if (!/^[1-9][0-9]*$/.test(value)) usageError("--concurrency requires a positive integer, got '" + value + "'");
        options.concurrency = Number(value);
      } else if (arg === "--out") {
        options.out = value;
      } else {
        options.filter = value;
      }
      continue;
    }

    if (arg === "--no-minimize") options.minimize = false;
    else if (arg === "--regressions") options.regressionsOnly = true;
    else if (arg === "--list") options.list = true;
    else if (arg === "--selftest") options.selftest = true;
    else if (arg === "--help" || arg === "-h") options.help = true;
    else usageError("unknown option '" + arg + "'");
  }
  return options;
}

// ---------------------------------------------------------------------------
// Binaries and process execution
// ---------------------------------------------------------------------------

async function hashFile(file) {
  const bytes = await readFile(file);
  return { sha256: createHash("sha256").update(bytes).digest("hex"), bytes: bytes.length };
}

async function requireBinary(file, label) {
  try {
    const info = await stat(file);
    if (!info.isFile()) throw new Error("not a file");
  } catch {
    process.stderr.write("HARNESS ERROR: " + label + " binary missing: " + file + "\n");
    if (label === "rust") process.stderr.write("build once with: " + BUILD_HINT + "\n");
    process.exit(EXIT_HARNESS);
  }
}

function runProcess(command, argv) {
  return new Promise((resolve) => {
    let settled = false;
    let stdout = "";
    let stderr = "";
    let timedOut = false;
    const child = spawn(command, argv, { windowsHide: true, stdio: ["ignore", "pipe", "pipe"] });
    const timer = setTimeout(() => {
      timedOut = true;
      child.kill();
    }, CASE_TIMEOUT_MS);

    const finish = (extra) => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      resolve({ stdout, stderr, timedOut, ...extra });
    };

    child.stdout.on("data", (chunk) => { stdout += chunk.toString("utf8"); });
    child.stderr.on("data", (chunk) => { stderr += chunk.toString("utf8"); });
    child.on("error", (error) => finish({ exitCode: -1, spawnError: String(error && error.message ? error.message : error) }));
    child.on("close", (code) => finish({ exitCode: code === null ? -1 : code, spawnError: null }));
  });
}

function runnerConfigs() {
  return {
    oracle: {
      command: "dotnet",
      baseArgs: ["exec", ORACLE_DLL],
      display: 'dotnet exec "' + ORACLE_DLL + '" --eval <script> --json [args]',
      path: ORACLE_DLL,
    },
    rust: {
      command: RUST_EXE,
      baseArgs: [],
      display: '"' + RUST_EXE + '" --eval <script> --json [args]',
      path: RUST_EXE,
    },
  };
}

async function executeCase(entry, configs) {
  const [oracle, rust] = await Promise.all([
    runProcess(configs.oracle.command, [...configs.oracle.baseArgs, "--eval", entry.script, "--json", ...entry.args]),
    runProcess(configs.rust.command, [...configs.rust.baseArgs, "--eval", entry.script, "--json", ...entry.args]),
  ]);

  if (oracle.spawnError || rust.spawnError || oracle.timedOut || rust.timedOut) {
    return {
      id: entry.id,
      category: entry.category,
      script: entry.script,
      args: entry.args,
      status: "error",
      error: oracle.spawnError || rust.spawnError || (oracle.timedOut ? "oracle timed out" : "rust timed out"),
      oracleExit: oracle.exitCode,
      rustExit: rust.exitCode,
    };
  }

  const classification = classifyCase(oracle, rust);
  return {
    id: entry.id,
    category: entry.category,
    script: entry.script,
    args: entry.args,
    status: classification.mismatch ? "mismatch" : "match",
    classification,
    oracleExit: oracle.exitCode,
    rustExit: rust.exitCode,
    oracleStdout: oracle.stdout,
    rustStdout: rust.stdout,
    oracleStderrBytes: classification.oracleStderrBytes,
    rustStderrBytes: classification.rustStderrBytes,
  };
}

async function pool(items, concurrency, worker) {
  const results = new Array(items.length);
  let next = 0;
  const runners = [];
  const width = Math.max(1, Math.min(concurrency, items.length));
  for (let i = 0; i < width; i += 1) {
    runners.push((async () => {
      while (true) {
        const index = next;
        next += 1;
        if (index >= items.length) return;
        results[index] = await worker(items[index], index);
      }
    })());
  }
  await Promise.all(runners);
  return results;
}

// ---------------------------------------------------------------------------
// Regressions
// ---------------------------------------------------------------------------

function regressionKey(entry) {
  return entry.id + "\u0000" + entry.script + "\u0000" + entry.args.join("\u0001");
}

async function loadRegressions() {
  let names;
  try {
    names = await readdir(REGRESSIONS_DIR);
  } catch (error) {
    if (error && error.code === "ENOENT") return [];
    process.stderr.write("HARNESS ERROR: cannot read " + REGRESSIONS_DIR + ": " + error.message + "\n");
    process.exit(EXIT_HARNESS);
  }

  const entries = [];
  for (const name of names.filter((n) => n.endsWith(".json")).sort()) {
    const file = path.join(REGRESSIONS_DIR, name);
    let parsed;
    try {
      parsed = JSON.parse(await readFile(file, "utf8"));
    } catch (error) {
      process.stderr.write("HARNESS ERROR: unreadable regression " + file + ": " + error.message + "\n");
      process.exit(EXIT_HARNESS);
    }
    if (
      !parsed ||
      typeof parsed.id !== "string" ||
      typeof parsed.script !== "string" ||
      !Array.isArray(parsed.args) ||
      parsed.args.some((a) => typeof a !== "string")
    ) {
      process.stderr.write("HARNESS ERROR: malformed regression " + file + " (needs id, script, args[])\n");
      process.exit(EXIT_HARNESS);
    }
    entries.push({
      id: parsed.id,
      category: typeof parsed.category === "string" ? parsed.category : "unknown",
      script: parsed.script,
      args: parsed.args,
      file,
      replayed: true,
    });
  }
  return entries;
}

// ---------------------------------------------------------------------------
// Artifacts
// ---------------------------------------------------------------------------

function utcStamp(date) {
  return date.toISOString().replace(/[-:]/g, "").replace(/\.\d+Z$/, "Z");
}

function firstDifferenceHistogram(results) {
  const tally = new Map();
  for (const result of results) {
    if (result.status !== "mismatch") continue;
    const key = result.classification.firstDifference.path + "|" + result.classification.firstDifference.kind;
    tally.set(key, (tally.get(key) || 0) + 1);
  }
  return [...tally.entries()]
    .sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]))
    .slice(0, 10)
    .map(([key, count]) => {
      const [pathText, kind] = key.split("|");
      return { path: pathText, kind, count };
    });
}

function summarize(results) {
  const categories = {};
  for (const category of CATEGORIES) {
    categories[category.name] = { cases: 0, match: 0, mismatch: 0, error: 0, oracleStderrBytes: 0, rustStderrBytes: 0, firstDifferences: {} };
  }
  const counts = { match: 0, mismatch: 0, error: 0 };
  for (const result of results) {
    const bucket = categories[result.category] || (categories[result.category] = { cases: 0, match: 0, mismatch: 0, error: 0, oracleStderrBytes: 0, rustStderrBytes: 0, firstDifferences: {} });
    bucket.cases += 1;
    bucket[result.status] += 1;
    counts[result.status] += 1;
    bucket.oracleStderrBytes += result.oracleStderrBytes || 0;
    bucket.rustStderrBytes += result.rustStderrBytes || 0;
    if (result.status === "mismatch") {
      const key = result.classification.firstDifference ? result.classification.firstDifference.path : "<none>";
      bucket.firstDifferences[key] = (bucket.firstDifferences[key] || 0) + 1;
    }
  }
  return { counts, categories };
}

async function writeArtifacts({ options, outDir, startedAt, finishedAt, generated, generatedResults, replayResults, results, replayed, regressionSummary, configs, binaryInfo, harnessExit }) {
  // counts/categories describe the GENERATED corpus only; replayed regressions are
  // reported in their own block so `--cases N` can never be confused with the total.
  const { counts, categories } = summarize(generatedResults);
  const replaySummary = summarize(replayResults);

  const manifest = {
    schemaVersion: SCHEMA_VERSION,
    startedAtUtc: startedAt.toISOString(),
    finishedAtUtc: finishedAt.toISOString(),
    seed: options.seed,
    cases: options.cases,
    executed: {
      generated: generated.length,
      regressions: replayed.length,
      total: results.length,
    },
    concurrency: options.concurrency,
    oracle: {
      command: configs.oracle.display,
      path: configs.oracle.path,
      sha256: binaryInfo.oracle.sha256,
      bytes: binaryInfo.oracle.bytes,
    },
    rust: {
      command: configs.rust.display,
      path: configs.rust.path,
      sha256: binaryInfo.rust.sha256,
      bytes: binaryInfo.rust.bytes,
    },
    normalization: [...VOLATILE_FIELDS],
    counts,
    categories,
    regressionCounts: replaySummary.counts,
    harnessExit,
    corpusCount: generated.length,
    corpusSha256: corpusHash(generated),
    filter: options.filter,
    minimized: options.minimize,
    regressions: regressionSummary,
    outDir,
  };

  const categoryRows = (buckets) => Object.entries(buckets).map(([name, bucket]) => ({
    category: name,
    cases: bucket.cases,
    match: bucket.match,
    mismatch: bucket.mismatch,
    error: bucket.error,
    mismatchRate: bucket.cases === 0 ? null : Number((bucket.mismatch / bucket.cases).toFixed(4)),
    oracleStderrBytes: bucket.oracleStderrBytes,
    rustStderrBytes: bucket.rustStderrBytes,
    topFirstDifferences: Object.entries(bucket.firstDifferences)
      .sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]))
      .map(([pathText, count]) => ({ path: pathText, count })),
  }));

  const monitor = {
    schemaVersion: SCHEMA_VERSION,
    startedAtUtc: startedAt.toISOString(),
    seed: options.seed,
    requestedCases: options.cases,
    generated: {
      total: generated.length,
      totals: counts,
      categories: categoryRows(categories),
    },
    regressions: {
      total: replayResults.length,
      totals: replaySummary.counts,
      categories: categoryRows(replaySummary.categories),
    },
    firstDifferenceHistogram: firstDifferenceHistogram(generatedResults.length > 0 ? generatedResults : replayResults),
  };

  await mkdir(outDir, { recursive: true });
  await mkdir(path.join(outDir, "mismatches"), { recursive: true });

  await writeFile(path.join(outDir, "manifest.json"), JSON.stringify(manifest, null, 2) + "\n", "utf8");
  await writeFile(path.join(outDir, "monitor.json"), JSON.stringify(monitor, null, 2) + "\n", "utf8");
  await writeFile(path.join(outDir, "corpus.json"), JSON.stringify(generated, null, 2) + "\n", "utf8");

  const lines = results.map((result) => JSON.stringify({
    id: result.id,
    category: result.category,
    status: result.status,
    oracleExit: result.oracleExit,
    rustExit: result.rustExit,
    oracleStderrBytes: result.oracleStderrBytes || 0,
    rustStderrBytes: result.rustStderrBytes || 0,
    mismatchCategory: result.classification ? result.classification.category : null,
    firstDifference: result.classification && result.classification.firstDifference ? result.classification.firstDifference.path : null,
    differenceCount: result.classification ? result.classification.differenceCount : 0,
    replayed: Boolean(result.replayed),
  }));
  await writeFile(path.join(outDir, "cases.jsonl"), lines.join("\n") + "\n", "utf8");

  for (const result of results) {
    if (result.status === "match") continue;
    const record = {
      schemaVersion: SCHEMA_VERSION,
      case: { id: result.id, category: result.category, script: result.script, args: result.args },
      status: result.status,
      replayed: Boolean(result.replayed),
      regressionFile: result.regressionFile || null,
      error: result.error || null,
      classification: result.classification
        ? {
            category: result.classification.category,
            firstDifference: result.classification.firstDifference,
            differenceCount: result.classification.differenceCount,
            differences: result.classification.differences,
          }
        : null,
      oracle: {
        exitCode: result.oracleExit,
        stdoutBytes: result.classification ? result.classification.oracleStdoutBytes : null,
        stderrBytes: result.oracleStderrBytes || 0,
        stderrPreview: result.classification ? result.classification.oracleStderrPreview : null,
        stdoutPreview: result.classification ? result.classification.oraclePreview : null,
      },
      rust: {
        exitCode: result.rustExit,
        stdoutBytes: result.classification ? result.classification.rustStdoutBytes : null,
        stderrBytes: result.rustStderrBytes || 0,
        stderrPreview: result.classification ? result.classification.rustStderrPreview : null,
        stdoutPreview: result.classification ? result.classification.rustPreview : null,
      },
      minimization: result.minimization || null,
      firstDifference: result.classification ? result.classification.firstDifference : null,
      exitCodes: { oracle: result.oracleExit, rust: result.rustExit },
    };
    const safe = result.id.replace(/[^A-Za-z0-9._-]/g, "_");
    await writeFile(path.join(outDir, "mismatches", safe + ".json"), JSON.stringify(record, null, 2) + "\n", "utf8");
  }

  return { counts, categories };
}

// ---------------------------------------------------------------------------
// Selftest: prove the harness can fail
// ---------------------------------------------------------------------------

const SELFTEST_SCRIPT = "2 + 3 * 4";
const SELFTEST_PROBES = [
  { kind: "control-oracle", base: "oracle", args: ["--omit-functions"], mutate: null, expect: null, label: "control: oracle vs itself" },
  { kind: "control-rust", base: "rust", args: ["--omit-functions"], mutate: null, expect: null, label: "control: rust vs itself" },
  { kind: "flip-ok", base: "oracle", args: ["--omit-functions"], mutate: "flip-ok", expect: { exact: "$.ok" }, label: "corrupt: flip ok" },
  { kind: "drop-key", base: "oracle", args: ["--omit-functions"], mutate: "drop-key", expect: { exact: "$.result.kind" }, label: "corrupt: drop result.kind" },
  { kind: "change-scalar", base: "oracle", args: ["--omit-functions"], mutate: "change-scalar", expect: { exact: "$.result.display" }, label: "corrupt: change result.display" },
  { kind: "reorder-array", base: "oracle", args: [], mutate: "reorder-array", expect: { prefix: "$.functions[0]" }, label: "corrupt: reorder functions[]" },
];

function mutateEnvelope(envelope, kind) {
  const copy = JSON.parse(JSON.stringify(envelope));
  if (kind === "flip-ok") copy.ok = !copy.ok;
  else if (kind === "drop-key") delete copy.result.kind;
  else if (kind === "change-scalar") copy.result.display = String(copy.result.display) + "-corrupt";
  else if (kind === "reorder-array") copy.functions = [...copy.functions].reverse();
  else throw new Error("unknown mutation " + kind);
  return copy;
}

async function selftestChild(kind) {
  const probe = SELFTEST_PROBES.find((p) => p.kind === kind);
  if (!probe) usageError("unknown selftest probe '" + kind + "'");
  const configs = runnerConfigs();
  const config = configs[probe.base];
  await requireBinary(config.path, probe.base === "oracle" ? "oracle" : "rust");

  const run = await runProcess(config.command, [...config.baseArgs, "--eval", SELFTEST_SCRIPT, "--json", ...probe.args]);
  if (run.spawnError || run.timedOut || run.exitCode !== 0) {
    process.stdout.write(JSON.stringify({ probe: kind, harnessError: run.spawnError || (run.timedOut ? "timeout" : "exit " + run.exitCode) }) + "\n");
    process.exit(EXIT_HARNESS);
  }
  let envelope;
  try {
    envelope = JSON.parse(run.stdout);
  } catch (error) {
    process.stdout.write(JSON.stringify({ probe: kind, harnessError: "non-JSON base envelope: " + error.message }) + "\n");
    process.exit(EXIT_HARNESS);
  }

  const expected = normalize(envelope);
  const actual = probe.mutate ? normalize(mutateEnvelope(envelope, probe.mutate)) : expected;
  const compared = compareEnvelopes(expected, actual);
  const detected = !compared.equal;
  process.stdout.write(JSON.stringify({
    probe: kind,
    detected,
    firstDifference: compared.firstDifference ? compared.firstDifference.path : null,
    kind: compared.firstDifference ? compared.firstDifference.kind : null,
  }) + "\n");
  process.exit(detected ? EXIT_MISMATCH : EXIT_MATCH);
}

async function selftest() {
  const configs = runnerConfigs();
  await requireBinary(configs.oracle.path, "oracle");
  await requireBinary(configs.rust.path, "rust");

  process.stdout.write("SELFTEST — falsification of the comparator itself\n");
  let failures = 0;

  for (const probe of SELFTEST_PROBES) {
    const child = spawn(process.execPath, [import.meta.filename, "--selftest-child", probe.kind], {
      windowsHide: true,
      stdio: ["ignore", "pipe", "pipe"],
    });
    let stdout = "";
    let stderr = "";
    child.stdout.on("data", (chunk) => { stdout += chunk.toString("utf8"); });
    child.stderr.on("data", (chunk) => { stderr += chunk.toString("utf8"); });
    const code = await new Promise((resolve) => child.on("close", resolve));

    let payload = null;
    try {
      payload = JSON.parse(stdout.trim().split("\n").filter(Boolean).pop());
    } catch {
      payload = null;
    }

    const wantMismatch = Boolean(probe.mutate);
    let pass = false;
    let detail = "";

    if (!payload || payload.harnessError) {
      detail = "harness error: " + (payload ? payload.harnessError : (stderr || "no output"));
    } else if (wantMismatch) {
      const pathValue = payload.firstDifference || "";
      const pathOk = probe.expect.exact ? pathValue === probe.expect.exact : pathValue.startsWith(probe.expect.prefix);
      pass = payload.detected === true && code === EXIT_MISMATCH && pathOk;
      detail = "mismatch at " + pathValue + " (" + payload.kind + "), harness exit " + code;
    } else {
      pass = payload.detected === false && code === EXIT_MATCH;
      detail = "matched, harness exit " + code;
    }

    if (!pass) failures += 1;
    process.stdout.write("  " + probe.kind.padEnd(16) + (pass ? "PASS  " : "FAIL  ") + detail + "  [" + probe.label + "]\n");
  }

  if (failures > 0) {
    process.stdout.write("SELFTEST FAIL: " + failures + " probe(s) did not behave as required; the gate is not falsifiable.\n");
    process.exit(EXIT_HARNESS);
  }
  process.stdout.write("SELFTEST PASS: 2 controls matched (exit 0), 4 injected corruptions were reported as mismatches (exit 1).\n");
  process.exit(EXIT_MATCH);
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

async function main() {
  const options = parseArgs(process.argv.slice(2));

  if (options.help) {
    process.stdout.write(USAGE + "\n");
    process.exit(EXIT_MATCH);
  }
  if (options.list) {
    process.stdout.write("PARITY DIFF CATEGORIES (weights sum to 100%)\n");
    for (const category of describeCategories()) {
      process.stdout.write("  " + category.name.padEnd(12) + String(category.weight).padStart(3) + "%  " + category.description + "\n");
      process.stdout.write("    templates: " + category.templates.join(", ") + "\n");
    }
    process.exit(EXIT_MATCH);
  }
  if (options.selftestChild) {
    await selftestChild(options.selftestChild);
    return;
  }
  if (options.selftest) {
    await selftest();
    return;
  }

  if (options.cases === 0) {
    process.stderr.write("HARNESS ERROR: --cases 0 is an empty corpus; a gate that runs nothing cannot pass.\n");
    process.exit(EXIT_HARNESS);
  }

  const startedAt = new Date();
  const configs = runnerConfigs();
  await requireBinary(configs.oracle.path, "oracle");
  await requireBinary(configs.rust.path, "rust");

  const binaryInfo = {
    oracle: await hashFile(configs.oracle.path),
    rust: await hashFile(configs.rust.path),
  };

  const allGenerated = options.regressionsOnly ? [] : generateCases({ seed: options.seed, count: options.cases });
  const filter = options.filter;
  const generated = filter ? allGenerated.filter((c) => c.id.includes(filter)) : allGenerated;

  // --cases N is EXACT: N generated cases. No per-category rounding, no silent
  // truncation. Violating this is a harness failure, not a quiet count drift.
  if (!options.regressionsOnly && !filter && generated.length !== options.cases) {
    process.stderr.write("HARNESS ERROR: generator produced " + generated.length + " cases for --cases " + options.cases + "\n");
    process.exit(EXIT_HARNESS);
  }

  const allRegressions = await loadRegressions();
  const regressions = filter ? allRegressions.filter((c) => c.id.includes(filter)) : allRegressions;

  const generatedKeys = new Set(generated.map(regressionKey));
  const duplicatesSkipped = regressions.filter((r) => generatedKeys.has(regressionKey(r)));
  const replayQueue = regressions.filter((r) => !generatedKeys.has(regressionKey(r)));

  if (generated.length === 0 && replayQueue.length === 0) {
    process.stderr.write("HARNESS ERROR: empty corpus (generated=" + generated.length + ", regressions=" + replayQueue.length + ").\n");
    process.exit(EXIT_HARNESS);
  }

  const outDir = options.out
    ? path.resolve(process.cwd(), options.out)
    : path.join(RUNS_DIR, utcStamp(startedAt) + "-seed" + options.seed);

  process.stdout.write("PARITY DIFF  seed=" + options.seed + "  generated=" + generated.length + "  replayed=" + replayQueue.length + "  total=" + (generated.length + replayQueue.length) + "  concurrency=" + options.concurrency + "\n");
  process.stdout.write("  oracle " + configs.oracle.path + "\n         sha256=" + binaryInfo.oracle.sha256 + "\n");
  process.stdout.write("  rust   " + configs.rust.path + "\n         sha256=" + binaryInfo.rust.sha256 + "\n");
  process.stdout.write("  normalize: " + VOLATILE_FIELDS.join(", ") + " -> <volatile>\n");

  const replayResults = await pool(replayQueue, options.concurrency, async (entry) => {
    const result = await executeCase(entry, configs);
    result.replayed = true;
    result.regressionFile = entry.file;
    return result;
  });

  const caseResults = await pool(generated, options.concurrency, async (entry) => {
    const result = await executeCase(entry, configs);
    result.replayed = false;
    return result;
  });

  if (options.minimize) {
    for (const result of caseResults) {
      if (result.status !== "mismatch") continue;
      const classification = result.classification;
      const minimization = await minimizeCase({
        script: result.script,
        category: classification.category,
        firstDifferencePath: classification.firstDifference.path,
        evaluate: async (candidate) => {
          const probe = await executeCase({ ...result, script: candidate }, configs);
          if (probe.status === "error") return { mismatch: false };
          return probe.classification;
        },
      });
      result.minimization = {
        original: minimization.original,
        minimized: minimization.minimized,
        category: classification.category,
        firstDifference: classification.firstDifference,
        exitCodes: { oracle: result.oracleExit, rust: result.rustExit },
        evaluations: minimization.evaluations,
        changed: minimization.changed,
      };
    }
  }

  const results = [...replayResults, ...caseResults];

  const resolved = replayResults.filter((r) => r.status === "match").map((r) => r.id);
  const stillMismatching = replayResults.filter((r) => r.status !== "match").map((r) => r.id);
  const regressionSummary = {
    discovered: allRegressions.length,
    replayed: replayQueue.length,
    duplicatesSkipped: duplicatesSkipped.length,
    stillMismatching,
    resolved,
    newRegressions: [],
  };

  await mkdir(REGRESSIONS_DIR, { recursive: true });
  for (const result of caseResults) {
    if (result.status !== "mismatch") continue;
    const safe = result.id.replace(/[^A-Za-z0-9._-]/g, "_");
    const file = path.join(REGRESSIONS_DIR, safe + ".json");
    let exists = false;
    try {
      await stat(file);
      exists = true;
    } catch {
      exists = false;
    }
    if (!exists) {
      const record = {
        id: result.id,
        category: result.category,
        script: result.script,
        args: result.args,
        observed: {
          oracleExit: result.oracleExit,
          rustExit: result.rustExit,
          firstDifference: result.classification.firstDifference,
        },
        addedAtUtc: new Date().toISOString(),
      };
      await writeFile(file, JSON.stringify(record, null, 2) + "\n", "utf8");
      regressionSummary.newRegressions.push(result.id);
    }
  }

  const hasHarnessError = results.some((r) => r.status === "error");
  const mismatchCount = results.filter((r) => r.status === "mismatch").length;
  const harnessExit = hasHarnessError ? EXIT_HARNESS : mismatchCount > 0 ? EXIT_MISMATCH : EXIT_MATCH;

  const finishedAt = new Date();
  const { counts, categories } = await writeArtifacts({
    options,
    outDir,
    startedAt,
    finishedAt,
    generated,
    generatedResults: caseResults,
    replayResults,
    results,
    replayed: replayQueue,
    regressionSummary,
    configs,
    binaryInfo,
    harnessExit,
  });

  const { counts: replayCounts, categories: replayCountsByCategory } = summarize(replayResults);

  // Two separate ledgers. `--cases N` is the generated ledger and nothing else;
  // replayed regressions are executions in their own right and are counted apart.
  process.stdout.write("\n");
  process.stdout.write("EXECUTION LEDGER\n");
  process.stdout.write("  generated cases" +
    (options.regressionsOnly ? " (--regressions mode: generation disabled)" : " (--cases " + options.cases + ")") +
    ": " + generated.length + "\n");
  process.stdout.write("  replayed regressions: " + replayQueue.length +
    (duplicatesSkipped.length > 0 ? " (" + duplicatesSkipped.length + " duplicate of a generated case skipped)" : "") + "\n");
  process.stdout.write("  total executions: " + results.length + "\n");

  if (generated.length > 0) {
    process.stdout.write("\ngenerated corpus by category   cases  match  mismatch  error\n");
  }
  for (const category of CATEGORIES) {
    const bucket = categories[category.name];
    if (!bucket || bucket.cases === 0) continue;
    process.stdout.write(
      "  " + category.name.padEnd(12) +
      String(bucket.cases).padStart(6) +
      String(bucket.match).padStart(7) +
      String(bucket.mismatch).padStart(10) +
      String(bucket.error).padStart(7) + "\n",
    );
  }
  if (generated.length > 0) {
    process.stdout.write(
      "  " + "GENERATED".padEnd(12) +
      String(generated.length).padStart(6) +
      String(counts.match).padStart(7) +
      String(counts.mismatch).padStart(10) +
      String(counts.error).padStart(7) + "\n",
    );
  }

  if (replayResults.length > 0) {
    process.stdout.write("\nregressions replayed by category   cases  match  mismatch  error\n");
    for (const category of CATEGORIES) {
      const bucket = replayCountsByCategory[category.name];
      if (!bucket || bucket.cases === 0) continue;
      process.stdout.write(
        "  " + category.name.padEnd(12) +
        String(bucket.cases).padStart(6) +
        String(bucket.match).padStart(7) +
        String(bucket.mismatch).padStart(10) +
        String(bucket.error).padStart(7) + "\n",
      );
    }
    process.stdout.write(
      "  " + "REPLAYED".padEnd(12) +
      String(replayResults.length).padStart(6) +
      String(replayCounts.match).padStart(7) +
      String(replayCounts.mismatch).padStart(10) +
      String(replayCounts.error).padStart(7) + "\n",
    );
  }

  const histogramSource = caseResults.length > 0 ? caseResults : replayResults;
  if (histogramSource.some((r) => r.status === "mismatch")) {
    process.stdout.write("\nfirst differences (" + (caseResults.length > 0 ? "generated" : "replayed") + "):\n");
    for (const entry of firstDifferenceHistogram(histogramSource)) {
      process.stdout.write("  " + String(entry.count).padStart(5) + "  " + entry.path + " (" + entry.kind + ")\n");
    }
  }

  process.stdout.write("\nregressions: discovered=" + regressionSummary.discovered +
    " replayed=" + regressionSummary.replayed +
    " stillMismatching=" + regressionSummary.stillMismatching.length +
    " resolved=" + regressionSummary.resolved.length +
    " new=" + regressionSummary.newRegressions.length + "\n");
  if (regressionSummary.resolved.length > 0) {
    process.stdout.write("  resolved (kept, not deleted): " + regressionSummary.resolved.join(", ") + "\n");
  }
  process.stdout.write("run dir: " + outDir + "\n");
  process.stdout.write("harness exit: " + harnessExit + "\n");
  process.exit(harnessExit);
}

main().catch((error) => {
  process.stderr.write("HARNESS ERROR: " + (error && error.stack ? error.stack : error) + "\n");
  process.exit(EXIT_HARNESS);
});
