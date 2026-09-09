# symbench — Symbolic-Numeric Pipeline Benchmarks

Phase 5 of the hardening plan. Measures the meaningful workloads, not toy cases:

- **Construction/canonicalization** — parsing a large shared DAG from its canonical text
  (the substrate every algorithm builds on: hashing, interning, canonical forms).
- **Calculus** — 10th-order derivative, 4×4 Jacobian, 4-variable Hessian.
- **Polynomials** — sparse bivariate degree-20 multiplication; cyclic-3 Gröbner basis (GrLex).
- **Compilation** — scalar kernel evaluation, 1000-lane vectorized batch evaluation, and
  direct tree evaluation of the same expression (tree vs MathIR).
- **Arbitrary precision** — exp(sin(x²)) at 64, 256, and 1024 digits.

**Invariant under benchmark:** performance is never gained through silent precision
reduction — every row runs at its declared precision scope.

Run a quick smoke with:

```text
dotnet run -c Release --project symbench -- --filter *Kernel* --job short
```
