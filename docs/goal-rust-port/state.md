# State — parity push complete

- Corpus 212/214 (99.1%). Language document 111/111 (100%). Symbolics 101/103.
- Tests 275, 0 failing. Release build exit 0 (2m11s). Desktop selftest 15/15. 10 crates, zero external dependencies.
- Remaining 2: sym/071 (MathIR byte-identity: literal-provenance constant duplication and inner sum order), sym/076 (Cramer artifact of the reference's pivot order).
- Definition of done: D1 PASS, D2 PASS, D3 FAIL (212/214), D4 PASS, D5 PASS, D6 PASS, D7 PASS.
- The goal stays active; the two remaining examples need provenance-aware literals or output imitation, not more solver capability.
