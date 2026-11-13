#!/usr/bin/env node
// RUST-010: test the real release binaries through their published entry points.
//
// This is the script the CI job .github/workflows/rust-ci.yml runs. It does not test the library
// targets: it builds (or accepts) the release executables and drives them exactly as an external
// consumer does — argv + stdin in, the versioned JSON envelope out, the CLI matrix, the golden
// fixtures, and the documentation corpus.
//
// Usage:
//   node parity/tools/ci-binaries.mjs                 # build --release, then exercise the binaries
//   node parity/tools/ci-binaries.mjs --skip-build    # use the already-built binaries
//   node parity/tools/ci-binaries.mjs --locked        # pass --locked to cargo (CI clean checkout)
//
// Exit 0 only when the build, the binary sanity checks, the CLI matrix (16/16), the fixtures
// (20/20 per side) and the corpus (214/214) all pass. A missing oracle DLL fails: the CLI matrix
// is a comparison, and a comparison that cannot run is not a pass.
import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import { spawnSync } from 'node:child_process';

const root = path.resolve(import.meta.dirname, '..', '..');
const args = process.argv.slice(2);
const skipBuild = args.includes('--skip-build');
const locked = args.includes('--locked');
const failures = [];

function run(command, commandArgs, options) {
  const res = spawnSync(command, commandArgs, {
    cwd: options && options.cwd ? options.cwd : root,
    encoding: 'utf8',
    maxBuffer: 64 * 1024 * 1024,
    timeout: 30 * 60 * 1000,
    input: options && options.input,
  });
  return { status: res.status, stdout: res.stdout || '', stderr: res.stderr || '', error: res.error };
}

function lastMeaningful(text) {
  const lines = text.split(/\r?\n/).filter(line => line.trim().length > 0);
  return lines.length ? lines[lines.length - 1] : '(no output)';
}

function sha256(file) {
  return crypto.createHash('sha256').update(fs.readFileSync(file)).digest('hex');
}

function magic(file) {
  const head = fs.readFileSync(file).subarray(0, 4);
  if (head[0] === 0x4d && head[1] === 0x5a) return 'PE';
  if (head[0] === 0x7f && head[1] === 0x45 && head[2] === 0x4c && head[3] === 0x46) return 'ELF';
  if (head[0] === 0xcf && head[1] === 0xfa && head[2] === 0xed && head[3] === 0xfe) return 'Mach-O';
  return 'unknown(' + head.toString('hex') + ')';
}

const binaryNames = ['lovelace-run', 'lovelace-corpus'];
const exe = process.platform === 'win32' ? '.exe' : '';
const releaseDir = path.join(root, 'rust', 'target', 'release');

// --- 1. build ------------------------------------------------------------------------------
if (!skipBuild) {
  const cargoArgs = ['build', '--release', '--manifest-path', path.join(root, 'rust', 'Cargo.toml'), '-p', 'lovelace-run', '-p', 'lovelace-corpus'];
  if (locked) cargoArgs.splice(2, 0, '--locked');
  const build = run('cargo', cargoArgs);
  if (build.status !== 0) {
    console.error(build.stdout);
    console.error(build.stderr);
    failures.push('cargo build --release failed (exit ' + build.status + ')');
  } else {
    console.log('build: PASS (' + lastMeaningful(build.stderr || build.stdout) + ')');
  }
}

// --- 2. the executables exist and are real native binaries ---------------------------------
const binaries = {};
for (const name of binaryNames) {
  const file = path.join(releaseDir, name + exe);
  if (!fs.existsSync(file)) {
    failures.push('missing release binary: ' + path.relative(root, file));
    continue;
  }
  const stat = fs.statSync(file);
  if (stat.size < 256 * 1024) failures.push(name + ': suspiciously small (' + stat.size + ' bytes)');
  const format = magic(file);
  if (format === 'unknown') failures.push(name + ': not a recognised executable (magic ' + format + ')');
  binaries[name] = { file, bytes: stat.size, format, sha256: sha256(file) };
  console.log('binary: ' + name + ' ' + format + ' ' + stat.size + ' bytes sha256=' + binaries[name].sha256);
}

// --- 3. the published entry point answers ---------------------------------------------------
const runner = path.join(releaseDir, 'lovelace-run' + exe);
if (fs.existsSync(runner)) {
  const smoke = run(runner, ['--eval', '2^10', '--json']);
  if (smoke.status !== 0) failures.push('lovelace-run --eval 2^10 exited ' + smoke.status + ': ' + lastMeaningful(smoke.stderr));
  else {
    let envelope = null;
    try { envelope = JSON.parse(smoke.stdout); } catch (e) { failures.push('--eval stdout is not one JSON document: ' + String(e).slice(0, 120)); }
    if (envelope) {
      if (envelope.protocolVersion !== 1) failures.push('envelope protocolVersion is ' + JSON.stringify(envelope.protocolVersion) + ', expected 1');
      if (!envelope.result || envelope.result.display !== '1024') failures.push('--eval 2^10 display is ' + JSON.stringify(envelope.result && envelope.result.display));
      if (lastMeaningful(smoke.stderr) !== '(no output)') failures.push('--eval wrote to stderr: ' + lastMeaningful(smoke.stderr).slice(0, 120));
      console.log('entry point: PASS (protocolVersion=1, display=' + (envelope.result && envelope.result.display) + ')');
    }
  }
}

// --- 4. the CLI matrix against the real binaries --------------------------------------------
const oracleDll = [
  path.join(root, 'Lovelace.Run', 'bin', 'Release', 'net10.0', 'Lovelace.Run.dll'),
  path.join(root, 'Lovelace.Run', 'bin', 'Debug', 'net10.0', 'Lovelace.Run.dll'),
].find(file => fs.existsSync(file));
if (!oracleDll) failures.push('oracle DLL missing: the CLI matrix and the fixtures cannot be compared');
else {
  console.log('oracle: ' + path.relative(root, oracleDll) + ' sha256=' + sha256(oracleDll));
  const cli = run(process.execPath, [path.join(root, 'parity', 'tools', 'gate-cli.mjs')]);
  const cliLine = (cli.stdout.match(/^CLI matrix: .*$/m) || [lastMeaningful(cli.stdout)])[0];
  console.log(cliLine);
  if (cli.status !== 0) {
    console.error(cli.stdout);
    console.error(cli.stderr);
    failures.push('CLI matrix failed (exit ' + cli.status + ')');
  }
  const fixtures = run(process.execPath, [path.join(root, 'parity', 'tools', 'gate-fixtures.mjs')]);
  const fixtureLine = (fixtures.stdout.match(/^(oracle|rust): \d+\/\d+$/gm) || [lastMeaningful(fixtures.stdout)]).join(', ');
  console.log(fixtureLine);
  if (fixtures.status !== 0) {
    console.error(fixtures.stdout);
    console.error(fixtures.stderr);
    failures.push('fixture gate failed (exit ' + fixtures.status + ')');
  }
}

// --- 5. the corpus binary runs its published gate -------------------------------------------
const corpus = path.join(releaseDir, 'lovelace-corpus' + exe);
let corpusSummary = null;
if (fs.existsSync(corpus)) {
  const res = run(corpus, []);
  const line = (res.stdout.match(/^corpus: \d+\/\d+.*$/m) || [lastMeaningful(res.stdout)])[0];
  corpusSummary = line;
  console.log(line);
  const m = line.match(/^corpus: (\d+)\/(\d+)/);
  if (res.status !== 0) failures.push('lovelace-corpus exited ' + res.status + ': ' + lastMeaningful(res.stderr));
  else if (!m) failures.push('lovelace-corpus did not print a corpus: n/m summary');
  else if (m[1] !== m[2]) failures.push('corpus is ' + line + ', expected every example');
}

// --- 6. record the evidence -----------------------------------------------------------------
const record = {
  schemaVersion: 1,
  generatedBy: 'parity/tools/ci-binaries.mjs',
  platform: process.platform,
  arch: process.arch,
  node: process.version,
  cargo: (run('cargo', ['--version']).stdout || '').trim(),
  rustc: (run('rustc', ['--version']).stdout || '').trim(),
  oracleDll: oracleDll ? { path: path.relative(root, oracleDll).split(path.sep).join('/'), sha256: sha256(oracleDll) } : null,
  binaries: Object.fromEntries(Object.entries(binaries).map(([name, b]) => [name, { bytes: b.bytes, format: b.format, sha256: b.sha256, path: path.relative(root, b.file).split(path.sep).join('/') }])),
  corpus: corpusSummary,
  failures,
};
const outDir = path.join(root, 'parity', 'build');
if (!fs.existsSync(outDir)) fs.mkdirSync(outDir, { recursive: true });
const outPath = path.join(outDir, 'rust-binaries.json');
fs.writeFileSync(outPath, JSON.stringify(record, null, 2) + '\n');
console.log('evidence: ' + path.relative(root, outPath));
if (failures.length) {
  console.error('\nFAIL: ' + failures.length + ' binary-CI issue(s)');
  for (const f of failures) console.error('  - ' + f);
  process.exit(1);
}
console.log('PASS: release binaries exercised through their published entry points');
