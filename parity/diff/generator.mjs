// generator.mjs — deterministic case generator for the differential gate.
// Dependency-free: Node 24 built-ins only.
//
// Reproducibility contract:
//   * no Math.random, no Date, no locale-dependent formatting, no I/O;
//   * every choice comes from a mulberry32 stream seeded by (seed, category name);
//   * category quotas come from the documented weights by largest remainder;
//   * cases are emitted in a fixed round-robin order over the category order.
// Same seed + same count therefore produce byte-identical {id, category, script, args}
// on any machine running this file.

import { createHash } from "node:crypto";

/**
 * Documented category weights (sum = 100). Changing a weight changes every corpus,
 * so weights are part of the reproducibility contract.
 */
export const CATEGORIES = Object.freeze([
  { name: "arithmetic", weight: 18, description: "natural/integer/real arithmetic, huge values, division, powers" },
  { name: "precision", weight: 10, description: "setprecision at 8/16/20/50/1000, pi/e digits, evalf" },
  { name: "arrays", weight: 16, description: "vector, matrix, reshape/transpose, dot/matmul/det/inv/linsolve, empty and singular shapes" },
  { name: "symbolic", weight: 18, description: "symbol, diff, integrate, solve, simplify, series, limit, factor, expand, evaluate" },
  { name: "records", weight: 14, description: "solve_full, integrate_full, limit_full, inv_full, linsolve_full, optimize_full, compile_full" },
  { name: "errors", weight: 12, description: "division by zero, unknown function, wrong arity, parse error, invalid index, domain error" },
  { name: "syntax", weight: 7, description: "comment attempts, CRLF, BOM, unicode identifiers/strings, multiline, tabs" },
  { name: "registry", weight: 5, description: "type, inspect, capabilities, assumptions, and the builtin registry payload" },
]);

/** Flags accepted by the ORACLE (verified with --help); nothing else may be passed. */
export const ALLOWED_FLAGS = Object.freeze(["--omit-functions", "--omit-variables", "--print-budget"]);

const EVAL_ARGS = Object.freeze(["--omit-functions"]);

// --- deterministic PRNG -------------------------------------------------------

function xmur3(text) {
  let h = 1779033703 ^ text.length;
  for (let i = 0; i < text.length; i += 1) {
    h = Math.imul(h ^ text.charCodeAt(i), 3432918353);
    h = (h << 13) | (h >>> 19);
  }
  return function next() {
    h = Math.imul(h ^ (h >>> 16), 2246822507);
    h = Math.imul(h ^ (h >>> 13), 3266489909);
    h ^= h >>> 16;
    return h >>> 0;
  };
}

function mulberry32(seed) {
  let a = seed >>> 0;
  return function next() {
    a = (a + 0x6d2b79f5) | 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

function pick(rng, values) {
  return values[Math.floor(rng() * values.length) % values.length];
}

function int(rng, min, max) {
  return min + Math.floor(rng() * (max - min + 1));
}

function dec(rng, min, max, places) {
  return (min + rng() * (max - min)).toFixed(places);
}

// --- case templates -----------------------------------------------------------
// Each template returns { script, args }. Templates never branch on wall-clock or
// environment; all variation flows through the category PRNG.

const PRELUDE_X = 'x = symbol("x"); ';
const PRELUDE_XY = 'x = symbol("x"); y = symbol("y"); ';

const TEMPLATES = {
  arithmetic: [
    { slug: "natural-add", make: (r) => ({ script: int(r, 0, 99999) + " + " + int(r, 0, 99999) }) },
    { slug: "natural-product", make: (r) => ({ script: int(r, 1000, 99999) + " * " + int(r, 1000, 99999) }) },
    { slug: "huge-power", make: (r) => ({ script: int(r, 2, 7) + "^" + int(r, 20, 120) }) },
    { slug: "negative-integer", make: (r) => ({ script: "-" + int(r, 1, 9999) + " + " + int(r, 1, 9999) + " * " + int(r, 2, 99) }) },
    { slug: "rational-division", make: (r) => ({ script: int(r, 1, 97) + "/" + int(r, 2, 97) }) },
    { slug: "real-arithmetic", make: (r) => ({ script: dec(r, 1, 99, 2) + " + " + dec(r, 1, 99, 2) + " * " + dec(r, 1, 9, 3) }) },
    { slug: "power-edge", make: (r) => ({ script: pick(r, ["0^0", "2^-3", "(-3)^4", "1^100000", "(-2)^31", "0^5"]) }) },
    { slug: "nested-precedence", make: (r) => ({ script: "((" + int(r, 1, 20) + " + " + int(r, 1, 20) + ") * " + int(r, 2, 9) + " - " + int(r, 1, 50) + ") / " + int(r, 2, 9) }) },
    { slug: "huge-sum-powers", make: (r) => ({ script: int(r, 2, 3) + "^200 + " + int(r, 2, 3) + "^100 + 1" }) },
    { slug: "modulo", make: (r) => ({ script: int(r, 1, 9999) + " % " + int(r, 2, 97) }) },
  ],
  precision: [
    { slug: "setprecision-8", make: () => ({ script: "setprecision(8); 1/3" }) },
    { slug: "setprecision-16", make: () => ({ script: "setprecision(16); 22/7" }) },
    { slug: "setprecision-20", make: () => ({ script: "setprecision(20); pi()" }) },
    { slug: "setprecision-50", make: () => ({ script: "setprecision(50); 1/7" }) },
    { slug: "setprecision-1000", make: () => ({ script: "setprecision(1000); 1/3" }) },
    { slug: "evalf-sqrt2", make: (r) => ({ script: "evalf(sqrt(2), " + pick(r, [30, 50, 100]) + ")" }) },
    { slug: "pi-digits", make: (r) => ({ script: "pi(" + pick(r, [8, 16, 50, 100]) + ")" }) },
    { slug: "e-digits", make: (r) => ({ script: "e(" + pick(r, [8, 20, 50]) + ")" }) },
    { slug: "precision-then-arithmetic", make: (r) => ({ script: "setprecision(" + pick(r, [8, 16, 20]) + "); 1/3 + 1/7" }) },
  ],
  arrays: [
    { slug: "vector-sum", make: (r) => ({ script: "sum([" + int(r, 0, 9) + ", " + int(r, 0, 9) + ", " + int(r, 0, 9) + "])" }) },
    { slug: "vector-dot", make: (r) => ({ script: "dot([" + int(r, 0, 9) + ", " + int(r, 0, 9) + ", " + int(r, 0, 9) + "], [" + int(r, 0, 9) + ", " + int(r, 0, 9) + ", " + int(r, 0, 9) + "])" }) },
    { slug: "matrix-det", make: (r) => ({ script: "det([[" + int(r, 1, 9) + ", " + int(r, 1, 9) + "], [" + int(r, 1, 9) + ", " + int(r, 1, 9) + "]])" }) },
    { slug: "matrix-inv", make: (r) => ({ script: "inv([[" + int(r, 1, 5) + ", " + int(r, 1, 5) + "], [" + int(r, 1, 5) + ", " + int(r, 6, 9) + "]])" }) },
    { slug: "matrix-transpose", make: (r) => ({ script: "transpose([[" + int(r, 1, 9) + ", " + int(r, 1, 9) + ", " + int(r, 1, 9) + "], [" + int(r, 1, 9) + ", " + int(r, 1, 9) + ", " + int(r, 1, 9) + "]])" }) },
    { slug: "reshape", make: (r) => ({ script: "reshape([1, 2, 3, 4, 5, 6], " + pick(r, [2, 3]) + ", " + pick(r, [2, 3]) + ")" }) },
    { slug: "matmul", make: () => ({ script: "matmul([[1, 2], [3, 4]], [[5, 6], [7, 8]])" }) },
    { slug: "empty-vector", make: (r) => ({ script: pick(r, ["[]", "zeros(0)", "sum([])", "shape([])", "len([])", "zeros(0, 3)"]) }) },
    { slug: "singular-matrix", make: (r) => ({ script: pick(r, ["det([[1, 2], [2, 4]])", "inv([[1, 2], [2, 4]])", "det([[1, 2, 3], [4, 5, 6], [7, 8, 9]])"]) }) },
    { slug: "identity-zeros", make: (r) => ({ script: pick(r, ["eye(3)", "zeros(2, 2)", "ones(2, 3)", "eye(1)"]) }) },
    { slug: "linsolve-symbolic", make: () => ({ script: PRELUDE_X + "linsolve([[x, 1], [1, x]], [5, 4])" }) },
    { slug: "shape-norm", make: (r) => ({ script: "norm([" + int(r, 1, 9) + ", " + int(r, 1, 9) + ", " + int(r, 1, 9) + "])" }) },
  ],
  symbolic: [
    { slug: "diff-polynomial", make: (r) => ({ script: PRELUDE_X + "diff(x^" + int(r, 2, 5) + " + " + int(r, 2, 9) + "*x^2 + " + int(r, 1, 9) + "*x + " + int(r, 0, 9) + ", x)" }) },
    { slug: "diff-product", make: (r) => ({ script: PRELUDE_X + "diff(" + pick(r, ["sin(x)*exp(x)", "x*cos(x)", "x^2*log(x)"]) + ", x)" }) },
    { slug: "integrate", make: (r) => ({ script: PRELUDE_X + pick(r, ["integrate(2*x*cos(x^2), x)", "integrate(x^2, x)", "integrate(sin(x), x)", "integrate(x*exp(x), x)"]) }) },
    { slug: "solve", make: (r) => ({ script: PRELUDE_X + pick(r, ["solve(x^2 - 4 == 0, x)", "solve(x^2 + 1 == 0, x, real)", "solve(x^3 - x == 0, x)"]) }) },
    { slug: "simplify", make: (r) => ({ script: PRELUDE_X + pick(r, ["simplify(sin(x)^2 + cos(x)^2)", "simplify(x/x)", "simplify(exp(log(x)))"]) }) },
    { slug: "series", make: (r) => ({ script: PRELUDE_X + "series(sin(x)/x, x, 0, " + pick(r, [6, 8, 10]) + ")" }) },
    { slug: "limit", make: (r) => ({ script: PRELUDE_X + pick(r, ["limit(sin(x)/x, x, 0)", "limit(1/x, x, 0)", "limit((1 + 1/x)^x, x, inf)"]) }) },
    { slug: "factor", make: (r) => ({ script: PRELUDE_X + pick(r, ["factor(x^4 - 5*x^2 + 4)", "factor(x^2 - 1)", "factor(x^3 - x)"]) }) },
    { slug: "expand", make: (r) => ({ script: PRELUDE_X + "expand((x + " + int(r, 1, 3) + ")^" + int(r, 2, 4) + ")" }) },
    { slug: "evaluate", make: (r) => ({ script: PRELUDE_X + pick(r, ["subs(x^2 + 1, x, 3)", "evalf(sqrt(2), 30)", "evalf(sin(1), 20)"]) }) },
    { slug: "two-symbols", make: () => ({ script: PRELUDE_XY + "diff(x*y, x)" }) },
  ],
  records: [
    { slug: "solve-full", make: (r) => ({ script: PRELUDE_X + pick(r, ["solve_full(x^2 - 4 == 0, x)", "solve_full(x^2 + 1 == 0, x, real)", "solve_full(x^3 - x == 0, x)"]) }) },
    { slug: "integrate-full", make: (r) => ({ script: PRELUDE_X + pick(r, ["integrate_full(x^2, x)", "integrate_full(sin(x), x)", "integrate_full(exp(-x^2), x)"]) }) },
    { slug: "limit-full", make: (r) => ({ script: PRELUDE_X + pick(r, ["limit_full(1/x, x, 0)", "limit_full(sin(x)/x, x, 0)"]) }) },
    { slug: "inv-full", make: () => ({ script: PRELUDE_X + "inv_full([[x, 1], [1, x]])" }) },
    { slug: "linsolve-full", make: () => ({ script: PRELUDE_X + "linsolve_full([[x, 1], [1, x]], [5, 4])" }) },
    { slug: "optimize-full", make: (r) => ({ script: PRELUDE_X + "optimize_full(x^5 + " + int(r, 1, 3) + "*x^4 + 3*x^3 + x^2 + x + 1, [x])" }) },
    { slug: "compile-full", make: () => ({ script: PRELUDE_X + "compile_full(x^2 + 1, [x])" }) },
    { slug: "simplify-full", make: () => ({ script: PRELUDE_X + "simplify_full(x/x)" }) },
    { slug: "print-budget", make: () => ({ script: PRELUDE_X + "diff(x^3 + 2*x^2 + 5*x + 7, x)", args: ["--omit-functions", "--print-budget", "5"] }) },
  ],
  errors: [
    { slug: "division-by-zero", make: (r) => ({ script: int(r, 1, 99) + "/0" }) },
    { slug: "unknown-function", make: (r) => ({ script: pick(r, ["foo(1)", "nonexistent_fn(2)", "not_a_function(1, 2)"]) }) },
    { slug: "wrong-arity", make: (r) => ({ script: pick(r, ["sin()", "sin(1, 2)", "det()", "diff(x)"]) }) },
    { slug: "parse-error", make: (r) => ({ script: pick(r, ["2+", "(1+2", "1 2", "@", "2 ** 3", "1 +"]) }) },
    { slug: "invalid-index", make: (r) => ({ script: pick(r, ["[1, 2][5]", "[1, 2][-1]", "[[1, 2], [3, 4]][9]"]) }) },
    { slug: "domain-error", make: (r) => ({ script: pick(r, ["log(0)", "0^(-1)", "1/0", "asin(2)"]) }) },
    { slug: "undefined-variable", make: () => ({ script: "x + 1" }) },
    { slug: "symbolic-refusal", make: () => ({ script: "linsolve([[1, 2], [3, 4]], [1, 2])" }) },
  ],
  syntax: [
    { slug: "comment-percent", make: () => ({ script: "% leading comment\n2 + 3" }) },
    { slug: "comment-slashes", make: () => ({ script: "// leading comment\n2 + 3" }) },
    { slug: "comment-hash", make: () => ({ script: "# leading comment\n2 + 3" }) },
    { slug: "crlf", make: () => ({ script: "2 + 3\r\n4 + 5" }) },
    { slug: "bom", make: () => ({ script: "\uFEFF2 + 3" }) },
    { slug: "unicode-identifier", make: (r) => ({ script: pick(r, ["\u03B1 = 5; \u03B1 * 2", "r\u00E9sultat = 2; r\u00E9sultat"]) }) },
    { slug: "unicode-string", make: (r) => ({ script: 'print("' + pick(r, ["h\u00E9llo", "\u65E5\u672C\u8A9E", "\u03B1\u03B2\u03B3"]) + '")' }) },
    { slug: "multiline", make: () => ({ script: "1 +\n2 *\n3" }) },
    { slug: "tabs", make: () => ({ script: "\t2\t+\t3" }) },
    { slug: "omit-variables", make: () => ({ script: "2 + 3", args: ["--omit-functions", "--omit-variables"] }) },
  ],
  registry: [
    { slug: "type-natural", make: () => ({ script: "type(42)", args: [] }) },
    { slug: "type-string", make: (r) => ({ script: 'type("' + pick(r, ["abc", "42"]) + '")', args: [] }) },
    { slug: "type-vector", make: () => ({ script: "type([1, 2, 3])" }) },
    { slug: "inspect-symbolic", make: () => ({ script: PRELUDE_X + "inspect(x^2 + 1)" }) },
    { slug: "capabilities", make: () => ({ script: "capabilities()", args: [] }) },
    { slug: "assumptions", make: (r) => ({ script: PRELUDE_X + "assume(x > " + int(r, 1, 9) + "); assumptions()" }) },
    { slug: "assumptions-clear", make: () => ({ script: "assume_clear()" }) },
  ],
};

// --- generation ---------------------------------------------------------------

/** Largest-remainder apportionment of `count` over the documented weights. */
export function allocateQuotas(count) {
  const totalWeight = CATEGORIES.reduce((sum, c) => sum + c.weight, 0);
  const raw = CATEGORIES.map((c) => (count * c.weight) / totalWeight);
  const base = raw.map((value) => Math.floor(value));
  let remainder = count - base.reduce((a, b) => a + b, 0);
  const order = raw
    .map((value, index) => ({ index, fraction: value - Math.floor(value) }))
    .sort((a, b) => b.fraction - a.fraction || a.index - b.index);
  for (let i = 0; remainder > 0; i += 1) {
    base[order[i % order.length].index] += 1;
    remainder -= 1;
  }
  return base;
}

/**
 * Generate `count` cases deterministically from `seed`.
 * @returns {{id: string, category: string, script: string, args: string[]}[]}
 */
export function generateCases({ seed, count }) {
  if (!Number.isInteger(count) || count < 0) throw new Error("count must be a non-negative integer");
  if (!Number.isInteger(seed)) throw new Error("seed must be an integer");

  const quotas = allocateQuotas(count);
  const buckets = CATEGORIES.map((category, categoryIndex) => {
    const rng = mulberry32(xmur3(seed + ":" + category.name)());
    const templates = TEMPLATES[category.name];
    const cases = [];
    for (let local = 0; local < quotas[categoryIndex]; local += 1) {
      const template = templates[local % templates.length];
      const made = template.make(rng);
      const args = made.args ? [...made.args] : [...EVAL_ARGS];
      for (let a = 0; a < args.length; a += 1) {
        const flag = args[a];
        if (!ALLOWED_FLAGS.includes(flag)) {
          throw new Error("generator produced a flag the oracle does not accept: " + flag);
        }
        // --print-budget takes one numeric value.
        if (flag === "--print-budget") a += 1;
      }
      cases.push({ category: category.name, slug: template.slug, script: made.script, args, occurrence: local + 1 });
    }
    return cases;
  });

  // Round-robin so that a small --cases still covers every category with a quota.
  const ordered = [];
  let remaining = true;
  while (remaining) {
    remaining = false;
    for (const bucket of buckets) {
      if (bucket.length > 0) {
        ordered.push(bucket.shift());
        remaining = true;
      }
    }
  }

  // The id is (seed, category, template, occurrence): it does NOT depend on --cases,
  // so a regression stored from a 20-case run still names the same input in a 5 000-case
  // run of the same seed. It DOES include the seed, because the script depends on it;
  // omitting it would let two seeds collide on one regression filename.
  return ordered.map((entry) => ({
    id: "g-s" + seed + "-" + entry.category + "-" + entry.slug + "-" + String(entry.occurrence).padStart(3, "0"),
    category: entry.category,
    script: entry.script,
    args: entry.args,
  }));
}

/** Canonical serialisation of the generated corpus (byte-stable across machines). */
export function serializeCorpus(cases) {
  return JSON.stringify(
    cases.map((c) => ({ id: c.id, category: c.category, script: c.script, args: c.args })),
  );
}

export function corpusHash(cases) {
  return createHash("sha256").update(Buffer.from(serializeCorpus(cases), "utf8")).digest("hex");
}

export function describeCategories() {
  const total = CATEGORIES.reduce((sum, c) => sum + c.weight, 0);
  return CATEGORIES.map((c) => ({
    name: c.name,
    weight: c.weight,
    share: c.weight / total,
    description: c.description,
    templates: TEMPLATES[c.name].map((t) => t.slug),
  }));
}
