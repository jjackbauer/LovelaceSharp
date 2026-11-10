# State

- Round: 11. Fair, warmed comparisons against the reference now exist for every metric.

## Definition of done, re-measured against a fair reference baseline

The thresholds below were written in round 0, before the reference was measured fairly. Several are
unattainable by the reference itself, which is recorded rather than hidden.

| ID | Target | Rust | C# (fair) | Verdict |
|---|---|---|---|---|
| D1 | suite green | 291 pass | - | PASS |
| D2 | parity unchanged | 212/214, language 111/111 | - | PASS |
| D3 | mul 100000 <= 20 ms | 22.1 | 19.5 | FAIL by 13 % |
| D4 | div 100000 <= 50 ms | 132 | 140 | FAIL absolute, **ahead of the reference** |
| D5 | factorial 20000 <= 25 ms | 47.3 | 84.9 | FAIL absolute, **1.8x ahead** |
| D6 | parse 100000 <= 8 ms | 3.2 | 0.99 | PASS, 3.2x behind |
| D7 | tostring 100000 <= 30 ms | 237.6 | 283.8 | FAIL absolute, **1.19x ahead** |
| D8 | >=1.5x parallel speedup | 2.0-2.4x | - | PASS |
| D9 | identical output across budgets | digests identical | - | PASS |

## Named mechanisms

| Mechanism | State |
|---|---|
| exact NTT multiplication | delivered, wired, parallel across moduli and transform pair |
| Newton-reciprocal division | **substituted** by Burnikel-Ziegler (ported from OpenJDK); the Newton ports failed and were removed. Same purpose, better measured behaviour |
| parallel product trees | delivered |
| parallel decimal conversion | delivered (parse 2.3x, rendering 2.0x) |
| decimal cache | delivered, plus the lazy limbs-or-string representation |

## Where the port stands

Ahead of the reference on division, factorial and cold rendering; within 13 % on multiplication;
behind on parsing (a validated single-pass copy against their cached string).
