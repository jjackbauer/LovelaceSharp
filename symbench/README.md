# symbench — Symbolic-Numeric Pipeline Benchmarks

Phase 5 of the hardening plan, extended by the A+ Cycle-2 Phase A deliverable
(alignment plan item 18). Measures the meaningful workloads, not toy cases:

- **Construction/canonicalization** — building a large shared DAG, canonicalizing it back
  to text, and parsing that text (the substrate every algorithm builds on: hashing,
  interning, canonical forms).
- **Simplification** — the safe entry point (`Simplify.SimplifyExpr`) against the full
  transform (`Simplify.Transform`) with the provenance trace off and on.
- **Calculus** — 10th-order derivative, 4×4 Jacobian, 4-variable Hessian, and a
  product/quotient transcendental derivative.
- **Polynomials** — sparse bivariate degree-20 multiplication; cyclic-3 Gröbner basis (GrLex);
  factoring a sextic with six rational roots.
- **Solving** — a complete quartic solve over the complex domain.
- **Printing** — pretty and canonical printing of a nontrivial expression.
- **Structured records** — `RecordValue` construction, field lookup by name, interpreter
  member access, and the structured JSON projection of a SolveResult-shaped record.
- **Compilation** — scalar kernel evaluation, 1000-lane vectorized batch evaluation, and
  direct tree evaluation of the same expression (tree vs MathIR).
- **MathIR** — the IR interpreter itself: one scalar pass and one 1000-lane batch pass.
- **Help catalog** — `Help.Overview()` and `Help.Funcs(null)` over the product plugin set.
- **Arbitrary precision** — exp(sin(x²)) at 64, 256, and 1024 digits.

**Invariant under benchmark:** performance is never gained through silent precision
reduction — every row runs at its declared precision scope.

## Baseline

`benchmarks/symbench-baseline.md` is the recorded Phase A sweep (date, HEAD, machine, SDK,
exact command line). It was produced with BenchmarkDotNet's **ShortRun** job (3 warmup /
3 iterations), which is enough to establish a regression *direction* and is **not** a
publication-grade measurement.

Run a quick smoke with:

```text
dotnet run -c Release --project symbench -- --filter *Kernel* --job short
```

Run the full sweep (many minutes) with:

```text
dotnet run -c Release --project symbench -- --filter * --job short
```
