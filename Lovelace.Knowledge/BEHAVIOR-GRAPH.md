# The learned behavior graph — visual companion

Mermaid diagrams for the graph discovered by Lovelace.Knowledge.Run against the real Lovelace.Run
engine (seed 20240617, 314 samples, converged C1-C4). See CONVERGENCE-RESULTS.md for the numeric
evidence.

> **PDF:** a compiled copy lives at [BEHAVIOR-GRAPH.pdf](./BEHAVIOR-GRAPH.pdf). Regenerate it with
> `make graph-pdf`, or directly:
> `node Lovelace.Knowledge/tools/render-graph-pdf.mjs Lovelace.Knowledge/BEHAVIOR-GRAPH.md Lovelace.Knowledge/BEHAVIOR-GRAPH.pdf`
> (the script documents the verified headless-Chrome + mermaid recipe in its header).

## 1. The discovery pipeline

```mermaid
flowchart LR
    A["config<br/>Ω · q · thresholds · seed"] --> B["sample batch<br/>z ~ q(z)"]
    B --> C["execute<br/>Lovelace.Run"]
    C --> D["canonicalize<br/>→ σ"]
    D --> E["reduce<br/>planes · boundaries · frontiers"]
    E --> F["merge<br/>idempotent"]
    F --> G{"measure<br/>C1–C4"}
    G -->|"not converged"| H["bias toward frontiers<br/>bisection · held-out probes"]
    H --> B
    G -->|"converged"| I(["persist graph"])
```

## 2. The behavior planes and their boundaries

Eight planes were found purely from observations, grouped into three families. Edges are the
localized boundaries (each carries a fitted guard and counterexamples on both sides).

```mermaid
flowchart TB
    subgraph NUM["numeric planes (widening chain)"]
        N["Natural<br/>(82 samples)"]
        Z["Integer<br/>(59 samples)"]
        R["Real<br/>(73 samples)"]
    end
    subgraph BOOL["boolean planes"]
        BT["Boolean:True<br/>(37)"]
        BF["Boolean:False<br/>(41)"]
    end
    subgraph ERR["error planes"]
        D0["err: divide by zero<br/>(19)"]
        EN["err: non-integer exponent<br/>(2)"]
        EP["err: negative exponent<br/>(1)"]
    end

    N -->|"− : right > left<br/>(underflow)"| Z
    N -->|"/ : non-exact<br/>(fractional)"| R
    R -->|"/ : exact"| N
    D0 -->|"/ % : right == 0"| N
    BT -->|"> : right ≥ left"| BF
    BF -->|"< : right > left"| BT
    EP -.->|"^ : 0 raised to negative exponent<br/>(random breadth)"| Z
    EN -.->|"^ : fractional exponent<br/>(random breadth)"| R
```

The dashed edges mark planes observed through random sampling that have no localized guard —
honest open-world findings, not forced connections.

## 3. Natural-subtraction underflow (a localized threshold boundary)

Sweeping the right operand b over naturals with a fixed left a, the result stays Natural while
b ≤ a and flips to Integer the moment b > a. The reducer bisects the changed interval down to the
exact integer and fits the guard "right > left".

```mermaid
flowchart LR
    subgraph LOW["b ≤ a  (right ≤ left)"]
        direction TB
        L0["5 − 5 = 0<br/>(Natural)"]
    end
    subgraph HIGH["b > a  (right > left)"]
        direction TB
        H0["5 − 6 = −1<br/>(Integer)"]
    end
    LOW -->|"✂ guard: right > left"| HIGH
```

The same boundary re-appears at every anchor — 0−0/0−1, 1−1/1−2, 2−2/2−3, 5−5/5−6, 10−10/10−11 —
so the guard "right > left" is learned five times and is stable across two step sizes (h=2, h=3),
which is exactly the C2 check.

## 4. Division: one operation, three adjacent planes

Division over naturals exposes a three-way split along the divisor b:

```mermaid
flowchart LR
    E["b = 0<br/>━━ error ━━<br/>5/0 = Cannot divide by zero."]
    X["b = 1<br/>━━ Natural ━━<br/>5/1 = 5  (exact)"]
    F["b ≥ 2<br/>━━ Real ━━<br/>5/2 = 2.5   (terminating)<br/>1/3 = 0.(3)  (periodic)<br/>1/6 = 0.1(6) (periodic)"]
    E -->|"guard: right == 0"| X
    X -->|"divisibility (composite)"| F
```

The Natural → Real edge is not a clean interval (it flips whenever the divisor stops dividing the
dividend), so the reducer marks it composite and reports the bounding samples instead of inventing
a predicate — the open-world rule.

## 5. The widening lattice the numeric planes reveal

The numeric planes are ordered by the engine's widening rule; this lattice is inferred from the
observed result kinds, not read from source:

```mermaid
flowchart TB
    N["Natural  ℕ₀"] --> Z["Integer  ℤ"]
    Z --> R["Real  ℝ<br/>(exact periodic fractions)"]
    N -->|"underflow on −"| Z
    Z -->|"non-exact ÷"| R
```

## 6. Where the 314 samples went

```mermaid
pie showData title "Sample support per behavior plane (314 samples)"
    "Natural" : 82
    "Real" : 73
    "Integer" : 59
    "Boolean:False" : 41
    "Boolean:True" : 37
    "err: divide by zero" : 19
    "err: non-integer exponent" : 2
    "err: negative exponent" : 1
```
