// compare.mjs — structural comparison of the two runners' wire envelopes.
// Dependency-free: Node 24 built-ins only.
//
// Normalisation is deliberately narrow. Exactly four field names are volatile and
// their VALUE is replaced by the literal string "<volatile>" at any depth:
//   revision, elapsed, elapsedTime, timings
// Nothing else is normalised. In particular, a volatile key that is PRESENT on one
// side and ABSENT on the other is still a structural mismatch (missing-key): key
// presence is part of the wire contract. Widening this is forbidden by the goal.

export const VOLATILE_FIELDS = Object.freeze([
  "revision",
  "elapsed",
  "elapsedTime",
  "timings",
]);

const VOLATILE = new Set(VOLATILE_FIELDS);
const VOLATILE_MARKER = "<volatile>";

/** Maximum number of differing paths retained in a record (the count is exact). */
export const MAX_RECORDED_DIFFERENCES = 100;

/** First 200 characters of a stdout payload, for invalid-JSON evidence. */
export function preview(text, limit = 200) {
  const s = typeof text === "string" ? text : String(text ?? "");
  return s.slice(0, limit);
}

/**
 * Replace the value of every volatile field, at any depth, with "<volatile>".
 * Object keys are emitted in sorted order so serialisation is deterministic.
 */
export function normalize(value) {
  if (Array.isArray(value)) return value.map((item) => normalize(item));
  if (value !== null && typeof value === "object") {
    const out = {};
    for (const key of Object.keys(value).sort()) {
      out[key] = VOLATILE.has(key) ? VOLATILE_MARKER : normalize(value[key]);
    }
    return out;
  }
  return value;
}

function kindOf(value) {
  if (value === null) return "null";
  if (Array.isArray(value)) return "array";
  return typeof value;
}

function brief(value) {
  const text = JSON.stringify(value);
  if (text === undefined) return String(value);
  return text.length > 160 ? text.slice(0, 157) + "..." : text;
}

function scalarsEqual(a, b) {
  // JSON-value equality: 1 and 1.0 are the same number; -0 equals 0.
  return a === b;
}

function walk(expected, actual, path, state) {
  const te = kindOf(expected);
  const ta = kindOf(actual);

  if (te !== ta) {
    state.add({ path, kind: "type", expectedType: te, actualType: ta, expected: brief(expected), actual: brief(actual) });
    return;
  }

  if (te === "array") {
    if (expected.length !== actual.length) {
      state.add({ path, kind: "array-length", expected: expected.length, actual: actual.length });
    }
    const n = Math.min(expected.length, actual.length);
    for (let i = 0; i < n; i += 1) walk(expected[i], actual[i], path + "[" + i + "]", state);
    return;
  }

  if (te === "object") {
    const expectedKeys = Object.keys(expected); // already sorted by normalize()
    const actualKeys = Object.keys(actual);
    const actualSet = new Set(actualKeys);
    const expectedSet = new Set(expectedKeys);
    for (const key of expectedKeys) {
      const childPath = path + "." + key;
      if (!actualSet.has(key)) {
        state.add({ path: childPath, kind: "missing-key", expected: brief(expected[key]), actual: null });
      } else {
        walk(expected[key], actual[key], childPath, state);
      }
    }
    for (const key of actualKeys) {
      if (!expectedSet.has(key)) {
        state.add({ path: path + "." + key, kind: "extra-key", expected: null, actual: brief(actual[key]) });
      }
    }
    return;
  }

  if (!scalarsEqual(expected, actual)) {
    state.add({ path, kind: "value", expected: brief(expected), actual: brief(actual) });
  }
}

/**
 * Compare two already-parsed envelopes structurally.
 * Key order is irrelevant; array order matters; added/removed keys mismatch.
 * @returns {{equal: boolean, differenceCount: number, differences: object[], firstDifference: object|null}}
 */
export function compareEnvelopes(expected, actual) {
  const differences = [];
  let count = 0;
  const state = {
    add(difference) {
      count += 1;
      if (differences.length < MAX_RECORDED_DIFFERENCES) differences.push(difference);
    },
  };
  walk(expected, actual, "$", state);
  return {
    equal: count === 0,
    differenceCount: count,
    differences,
    firstDifference: differences.length > 0 ? differences[0] : null,
  };
}

function parseJson(text) {
  try {
    return { ok: true, value: JSON.parse(text) };
  } catch (error) {
    return { ok: false, error: String(error && error.message ? error.message : error) };
  }
}

/**
 * Classify one executed case from the two runner results.
 * Precedence: process failure is handled by the caller; then exit code; then
 * JSON validity; then the structural envelope comparison.
 *
 * A stderr difference is NOT a mismatch by itself. stderr length is recorded.
 *
 * @param {{exitCode:number, stdout:string, stderr:string, timedOut?:boolean, spawnError?:string}} oracleRun
 * @param {{exitCode:number, stdout:string, stderr:string, timedOut?:boolean, spawnError?:string}} rustRun
 */
export function classifyCase(oracleRun, rustRun) {
  const base = {
    oracleExit: oracleRun.exitCode,
    rustExit: rustRun.exitCode,
    oracleStdoutBytes: Buffer.byteLength(oracleRun.stdout, "utf8"),
    rustStdoutBytes: Buffer.byteLength(rustRun.stdout, "utf8"),
    oracleStderrBytes: Buffer.byteLength(oracleRun.stderr, "utf8"),
    rustStderrBytes: Buffer.byteLength(rustRun.stderr, "utf8"),
    oracleStderrPreview: preview(oracleRun.stderr),
    rustStderrPreview: preview(rustRun.stderr),
  };

  if (oracleRun.exitCode !== rustRun.exitCode) {
    return {
      ...base,
      mismatch: true,
      category: "exit-code",
      firstDifference: {
        path: "$.exitCode",
        kind: "value",
        expected: oracleRun.exitCode,
        actual: rustRun.exitCode,
      },
      differenceCount: 1,
      differences: [],
      oraclePreview: preview(oracleRun.stdout),
      rustPreview: preview(rustRun.stdout),
    };
  }

  const oracleParsed = parseJson(oracleRun.stdout);
  const rustParsed = parseJson(rustRun.stdout);

  if (!oracleParsed.ok || !rustParsed.ok) {
    return {
      ...base,
      mismatch: true,
      category: "invalid-json",
      firstDifference: {
        path: "$",
        kind: "invalid-json",
        expected: oracleParsed.ok ? "<json>" : preview(oracleRun.stdout),
        actual: rustParsed.ok ? "<json>" : preview(rustRun.stdout),
      },
      differenceCount: 1,
      differences: [],
      oraclePreview: preview(oracleRun.stdout),
      rustPreview: preview(rustRun.stdout),
      oracleJsonError: oracleParsed.ok ? null : oracleParsed.error,
      rustJsonError: rustParsed.ok ? null : rustParsed.error,
    };
  }

  const compared = compareEnvelopes(normalize(oracleParsed.value), normalize(rustParsed.value));
  if (!compared.equal) {
    return {
      ...base,
      mismatch: true,
      category: "envelope",
      firstDifference: compared.firstDifference,
      differenceCount: compared.differenceCount,
      differences: compared.differences,
      oraclePreview: null,
      rustPreview: null,
    };
  }

  return {
    ...base,
    mismatch: false,
    category: null,
    firstDifference: null,
    differenceCount: 0,
    differences: [],
    oraclePreview: null,
    rustPreview: null,
  };
}
