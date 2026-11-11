// minimize.mjs — delta debugging over a mismatching script.
// Dependency-free: Node 24 built-ins only.
//
// Strategy: reduce LINES first (contiguous-chunk ddmin), then whitespace-separated
// TOKENS. A reduction is kept only while the re-run reproduces BOTH the same
// mismatch category and the same first differing JSON path. Every candidate costs
// one evaluation = one oracle run + one Rust run. The budget is hard-capped.

export const DEFAULT_EVALUATION_BUDGET = 40;

/**
 * ddmin over an array of units.
 * @param {string[]} units
 * @param {(units: string[]) => string} join
 * @param {(candidate: string, current: string) => Promise<boolean>} reproduces
 * @param {() => boolean} hasBudget
 */
async function ddmin(units, join, reproduces, hasBudget) {
  let n = 2;
  while (units.length >= 2 && hasBudget()) {
    const size = Math.ceil(units.length / n);
    let reduced = false;
    for (let start = 0; start < units.length && hasBudget(); start += size) {
      const complement = units.slice(0, start).concat(units.slice(start + size));
      if (complement.length === 0) continue;
      const candidate = join(complement);
      if (candidate === join(units)) continue;
      if (await reproduces(candidate, join(units))) {
        units = complement;
        n = Math.max(2, n - 1);
        reduced = true;
        break;
      }
    }
    if (!reduced) {
      if (n >= units.length) break;
      n = Math.min(units.length, n * 2);
    }
  }
  return units;
}

/**
 * Minimise a script while preserving the observed mismatch.
 *
 * @param {object} options
 * @param {string} options.script            original script
 * @param {string} options.category          mismatch category to preserve
 * @param {string} options.firstDifferencePath JSON path to preserve
 * @param {(script: string) => Promise<object>} options.evaluate  runs both binaries and classifies
 * @param {number} [options.budget]           max evaluations (default 40)
 * @returns {Promise<{minimized: string, original: string, evaluations: number, changed: boolean}>}
 */
export async function minimizeCase({
  script,
  category,
  firstDifferencePath,
  evaluate,
  budget = DEFAULT_EVALUATION_BUDGET,
}) {
  if (typeof script !== "string") throw new TypeError("script must be a string");
  if (typeof evaluate !== "function") throw new TypeError("evaluate must be a function");

  let evaluations = 0;
  const hasBudget = () => evaluations < budget;

  const reproduces = async (candidate) => {
    if (!hasBudget()) return false;
    evaluations += 1;
    const result = await evaluate(candidate);
    return Boolean(
      result &&
        result.mismatch === true &&
        result.category === category &&
        result.firstDifference &&
        result.firstDifference.path === firstDifferencePath,
    );
  };

  let current = script;

  const lines = current.split(/\r?\n/);
  if (lines.length > 1 && hasBudget()) {
    const reduced = await ddmin(lines, (units) => units.join("\n"), reproduces, hasBudget);
    current = reduced.join("\n");
  }

  const tokens = current.split(/\s+/).filter((token) => token.length > 0);
  if (tokens.length > 1 && hasBudget()) {
    const reduced = await ddmin(tokens, (units) => units.join(" "), reproduces, hasBudget);
    current = reduced.join(" ");
  }

  return {
    original: script,
    minimized: current,
    evaluations,
    changed: current !== script,
  };
}
