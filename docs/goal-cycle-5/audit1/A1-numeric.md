# A1 — Numeric tower: independent adversarial audit

**Product under test.** `out/aot/Lovelace.Run.exe` (Native AOT, published from the current tree; every
envelope below reports `"revision":81`, `"protocolVersion":1`, `"symbolicFormatVersion":"#!lovelace-sym 1"`,
`"mathIrVersion":2`).
**Contract attacked.** `docs/symbolics/dsh-protocol.md` (the machine protocol, its invariants and value
forms) plus the builtin metadata returned by `capabilities()` (`supported_domains: [real, complex]`,
`unsupported_domains: [integer, rational]`, `exactness: BestEffort`, and 123 builtins with
`name`/`parameters`).
**Scope.** Arbitrary-precision REAL and INTEGER arithmetic and every exactness claim the wire makes about
a numeric value: division and the periodic decimal form; multiplication of periodic values; powers with
negative/zero/fractional/huge exponents; zero to a negative power; underflow and overflow of the
fixed-precision representation; comparison/equality across numeric kinds; the `exact` flag and the
`numerator`/`denominator` fields; rounding vs truncation; `inspect(...)` on a Real;
coercions Natural / Integer / Real / Complex / Symbolic.

**Auditor stance.** I did not write this product. Every expectation below is derived from
`dsh-protocol.md`, from `capabilities()`, or from mathematics — never from what the binary happens to
print. Ground truth is SymPy 1.14.0, Python `fractions.Fraction` and `mpmath`, run inline.

## Method

* Command template (identical for every row; only the `.ls` file differs):
  `C:\Users\ricar\dev\LovelaceSharp\out\aot\Lovelace.Run.exe --file <ID>.ls --omit-functions`
  Each probe body was written to a **fresh, empty temp directory** (`%TEMP%\a1numeric\...`) before each run.
* Every probe was run **at least twice**: once in a batch, and again from a clean temp file; the
  FINDINGS section quotes the clean re-run. A finding that did not reproduce twice would have been
  reported INCONCLUSIVE — none did.
* **Abridgement rule for the "raw output" column** (and for the FINDINGS quotes): any run of 48 or more
  digits is rendered as `first8(...N digits abridged...)last4` with `N` the count, so the *length* of
  every digitisation is preserved. No deciding field (`kind`, `exact`, presence of
  `numerator`/`denominator`, `code`, `category`, `value` when short) is abridged.
* Severity: **P0** = the product states something false about a mathematical result (wrong answer, wrong
  completeness claim, wrong exactness claim), raises an internal invariant failure, or dies without an
  envelope; **P1** = a documented contract in `dsh-protocol.md` or `capabilities()` is contradicted, or a
  machine-visible field cannot be recovered without parsing a display string; **P2** = inconsistency or
  rough edge with no wrong answer behind it; **P3** = cosmetic.
* Probe rows record the *first* decisive observation for that input; grouped consequences of one root
  cause carry the same finding id.

## Counts

| | count |
|---|---|
| probes executed (rows below) | **214** (213 distinct bodies; `F11`/`H12` are the same body run twice on purpose) |
| **HELD** | **140** |
| **FINDING** rows | **74** — collapsing to **18 distinct findings** |
| distinct findings by severity | **8 × P0, 4 × P1, 6 × P2** |
| INCONCLUSIVE | **0** (3 items moved to "Could not decide" below, none of them a numeric claim) |

## Probe table

| id | what was probed | command (`.ls` body in column 2) | raw output (abridged) | verdict | sev |
|---|---|---|---|---|---|
| A01 | `1/3` | `--file A01.ls --omit-functions` | `{"kind":"Real","value":"0.(3)","exact":true,"numerator":"1","denominator":"3"}` | HELD | - |
| A02 | `1/6` | `--file A02.ls --omit-functions` | `{"kind":"Real","value":"0.1(6)","exact":true,"numerator":"1","denominator":"6"}` | HELD | - |
| A03 | `1/7` | `--file A03.ls --omit-functions` | `{"kind":"Real","value":"0.(142857)","exact":true,"numerator":"1","denominator":"7"}` | HELD | - |
| A04 | `1/17` | `--file A04.ls --omit-functions` | `{"kind":"Real","value":"0.(0588235294117647)","exact":true,"numerator":"1","denominator":"17"}` | HELD | - |
| A05 | `1/97` | `--file A05.ls --omit-functions` | `{"kind":"Real","value":"0.(01030927(...84 digits abridged...)5567)","exact":true,"numerator":"1","denominator":"97"}` | HELD | - |
| A06 | `1/1009` | `--file A06.ls --omit-functions` | `{"kind":"Real","value":"0.(00099108(...240 digits abridged...)2111)","exact":true,"numerator":"1","denominator":"1009"}` | HELD | - |
| A07 | `1/(10^999 - 1)` | `--file A07.ls --omit-functions` | `{"kind":"Real","value":"0.(00000000(...987 digits abridged...)0001)","exact":true,"numerator":"1","denominator":"99999999(...987 digits abridged...)9999"}` | HELD | - |
| A08 | `1/(10^1000 - 1)` | `--file A08.ls --omit-functions` | `{"kind":"Real","value":"0.00000000(...988 digits abridged...)0001","exact":false}` | HELD | - |
| A09 | `1/(10^1001 - 1)` | `--file A09.ls --omit-functions` | `{"kind":"Real","value":"0","exact":true,"numerator":"0","denominator":"1"}` | FINDING | P0 |
| A10 | `1/(10^1002 - 1)` | `--file A10.ls --omit-functions` | `{"kind":"Real","value":"0","exact":true,"numerator":"0","denominator":"1"}` | FINDING | P0 |
| A11 | `22/7` | `--file A11.ls --omit-functions` | `{"kind":"Real","value":"3.(142857)","exact":true,"numerator":"22","denominator":"7"}` | HELD | - |
| A12 | `-1/3` | `--file A12.ls --omit-functions` | `{"kind":"Real","value":"-0.(3)","exact":true,"numerator":"-1","denominator":"3"}` | HELD | - |
| A13 | `1/0` | `--file A13.ls --omit-functions` | `{"ok":false,"code":"DivisionByZero","category":"DomainError","message":"Cannot divide by zero.","recoverable":True}` | HELD | - |
| A14 | `0/0` | `--file A14.ls --omit-functions` | `{"ok":false,"code":"DivisionByZero","category":"DomainError","message":"Cannot divide by zero.","recoverable":True}` | HELD | - |
| A15 | `0/3` | `--file A15.ls --omit-functions` | `{"kind":"Natural","value":"0","exact":true}` | HELD | - |
| A16 | `1/2` | `--file A16.ls --omit-functions` | `{"kind":"Real","value":"0.5","exact":true,"numerator":"1","denominator":"2"}` | HELD | - |
| A17 | `1/(10^18)` | `--file A17.ls --omit-functions` | `{"kind":"Real","value":"0.000000000000000001","exact":true,"numerator":"1","denominator":"1000000000000000000"}` | HELD | - |
| A18 | `1/(10^19)` | `--file A18.ls --omit-functions` | `{"kind":"Real","value":"0.0000000000000000001","exact":false}` | FINDING | P0 |
| A19 | `1/(2^100)` | `--file A19.ls --omit-functions` | `{"kind":"Real","value":"0.00000000(...88 digits abridged...)0625","exact":false}` | FINDING | P0 |
| A20 | `2/4` | `--file A20.ls --omit-functions` | `{"kind":"Real","value":"0.5","exact":true,"numerator":"1","denominator":"2"}` | HELD | - |
| A21 | `1/-3` | `--file A21.ls --omit-functions` | `{"kind":"Real","value":"-0.(3)","exact":true,"numerator":"-1","denominator":"3"}` | HELD | - |
| A22 | `-0.0` | `--file A22.ls --omit-functions` | `{"kind":"Real","value":"0","exact":true,"numerator":"0","denominator":"1"}` | HELD | - |
| A23 | `1/(10^17)` | `--file A23.ls --omit-functions` | `{"kind":"Real","value":"0.00000000000000001","exact":true,"numerator":"1","denominator":"100000000000000000"}` | HELD | - |
| A24 | `1/(3*10^19)` | `--file A24.ls --omit-functions` | `{"kind":"Real","value":"0.0000000000000000000(3)","exact":true,"numerator":"1","denominator":"30000000000000000000"}` | HELD | - |
| A25 | `10^-19` | `--file A25.ls --omit-functions` | `{"kind":"Real","value":"0.0000000000000000001","exact":false}` | FINDING | P0 |
| B01 | `1/(10^1001 - 1) == 0` | `--file B01.ls --omit-functions` | `{"kind":"Boolean","value":"true"}` | FINDING | P0 |
| B02 | `1/(10^1001 - 1) + 1` | `--file B02.ls --omit-functions` | `{"kind":"Real","value":"1","exact":true,"numerator":"1","denominator":"1"}` | FINDING | P0 |
| B03 | `1/(10^1001 - 1) > 0` | `--file B03.ls --omit-functions` | `{"kind":"Boolean","value":"false"}` | FINDING | P0 |
| B04 | `(1/(10^1001 - 1)) * (10^1001 - 1)` | `--file B04.ls --omit-functions` | `{"kind":"Real","value":"0","exact":true,"numerator":"0","denominator":"1"}` | FINDING | P0 |
| B05 | `1/(10^2 - 1)` | `--file B05.ls --omit-functions` | `{"kind":"Real","value":"0.(01)","exact":true,"numerator":"1","denominator":"99"}` | HELD | - |
| B06 | `1/(10^1001 - 1) - 1` | `--file B06.ls --omit-functions` | `{"kind":"Real","value":"-1","exact":true,"numerator":"-1","denominator":"1"}` | FINDING | P0 |
| B07 | `0 - 1/(10^1001 - 1)` | `--file B07.ls --omit-functions` | `{"kind":"Real","value":"0","exact":true,"numerator":"0","denominator":"1"}` | FINDING | P0 |
| B08 | `1/(10^1002 - 1) == 0` | `--file B08.ls --omit-functions` | `{"kind":"Boolean","value":"true"}` | FINDING | P0 |
| B09 | `1/(10^1001)` | `--file B09.ls --omit-functions` | `{"kind":"Real","value":"0","exact":true,"numerator":"0","denominator":"1"}` | FINDING | P0 |
| B10 | `1/(10^1000)` | `--file B10.ls --omit-functions` | `{"kind":"Real","value":"0.00000000(...988 digits abridged...)0001","exact":false}` | FINDING | P0 |
| B11 | `(1/1009)*1009` | `--file B11.ls --omit-functions` | `{"kind":"Real","value":"0.99999999(...988 digits abridged...)9027","exact":false}` | FINDING | P0 |
| B12 | `(1/1009)*1009 == 1` | `--file B12.ls --omit-functions` | `{"kind":"Boolean","value":"false"}` | FINDING | P0 |
| B13 | `(1/3)*3` | `--file B13.ls --omit-functions` | `{"kind":"Real","value":"1","exact":true,"numerator":"1","denominator":"1"}` | HELD | - |
| B14 | `(1/7)*7` | `--file B14.ls --omit-functions` | `{"kind":"Real","value":"1","exact":true,"numerator":"1","denominator":"1"}` | HELD | - |
| B15 | `(1/97)*97` | `--file B15.ls --omit-functions` | `{"kind":"Real","value":"0.99999999(...988 digits abridged...)9909","exact":false}` | FINDING | P0 |
| B16 | `(1/97)*97 == 1` | `--file B16.ls --omit-functions` | `{"kind":"Boolean","value":"false"}` | FINDING | P0 |
| B17 | `1/3 + 1/3 + 1/3` | `--file B17.ls --omit-functions` | `{"kind":"Real","value":"1","exact":true,"numerator":"1","denominator":"1"}` | HELD | - |
| B18 | `(1/3)*(1/3)` | `--file B18.ls --omit-functions` | `{"kind":"Real","value":"0.11111111(...988 digits abridged...)1110(8)","exact":true,"numerator":"49999999(...988 digits abridged...)9999","denominator":"45000000(...989 digits abridged...)0000"}` | FINDING | P0 |
| B19 | `(1/7)*(1/11)` | `--file B19.ls --omit-functions` | `{"kind":"Real","value":"0.01298701(...988 digits abridged...)0129(805194)","exact":true,"numerator":"19999999(...988 digits abridged...)9999","denominator":"15400000(...990 digits abridged...)0000"}` | FINDING | P0 |
| B20 | `sqrt(2)` | `--file B20.ls --omit-functions` | `{"kind":"Real","value":"1.41421356(...88 digits abridged...)5727","exact":false}` | HELD | - |
| B21 | `inspect(1/3)` | `--file B21.ls --omit-functions` | `{"kind":"Record","type":"Inspection","fields":[{"name":"type","value":{"kind":"Text","value":"Real"}},{"name":"domain","value":{"kind":"Null"}},{"name":"exact","value":{"kind":"Null"}},{"name":"free_symbols","value":{"kind":"Array","type":"Vector","shape":[0],"elements":[]}},{"name":"node_count","value":{"kind":"Null"}},{"name":"canonical","value":{"kind":"Null"}},{"name":"pretty","value":{"kind":"Null"}},{"name":"shape","value":{"kind":"Null"}},{"name":"rank","value":{"kind":"Null"}},{"name":"element_domain","value":{"kind":"Null"}},{"name":"assumptions","value":{"kind":"Array","type":"Vector","shape":[0],"elements":[]}},{"name":"members","value":{"kind":"Null"}}]}` | FINDING | P1 |
| B22 | `inspect(1/(10^19))` | `--file B22.ls --omit-functions` | `{"kind":"Record","type":"Inspection","fields":[{"name":"type","value":{"kind":"Text","value":"Real"}},{"name":"domain","value":{"kind":"Null"}},{"name":"exact","value":{"kind":"Null"}},{"name":"free_symbols","value":{"kind":"Array","type":"Vector","shape":[0],"elements":[]}},{"name":"node_count","value":{"kind":"Null"}},{"name":"canonical","value":{"kind":"Null"}},{"name":"pretty","value":{"kind":"Null"}},{"name":"shape","value":{"kind":"Null"}},{"name":"rank","value":{"kind":"Null"}},{"name":"element_domain","value":{"kind":"Null"}},{"name":"assumptions","value":{"kind":"Array","type":"Vector","shape":[0],"elements":[]}},{"name":"members","value":{"kind":"Null"}}]}` | FINDING | P1 |
| B23 | `inspect(sqrt(2))` | `--file B23.ls --omit-functions` | `{"kind":"Record","type":"Inspection","fields":[{"name":"type","value":{"kind":"Text","value":"Real"}},{"name":"domain","value":{"kind":"Null"}},{"name":"exact","value":{"kind":"Null"}},{"name":"free_symbols","value":{"kind":"Array","type":"Vector","shape":[0],"elements":[]}},{"name":"node_count","value":{"kind":"Null"}},{"name":"canonical","value":{"kind":"Null"}},{"name":"pretty","value":{"kind":"Null"}},{"name":"shape","value":{"kind":"Null"}},{"name":"rank","value":{"kind":"Null"}},{"name":"element_domain","value":{"kind":"Null"}},{"name":"assumptions","value":{"kind":"Array","type":"Vector","shape":[0],"elements":[]}},{"name":"members","value":{"kind":"Null"}}]}` | FINDING | P1 |
| B24 | `inspect(1)` | `--file B24.ls --omit-functions` | `{"kind":"Record","type":"Inspection","fields":[{"name":"type","value":{"kind":"Text","value":"Natural"}},{"name":"domain","value":{"kind":"Null"}},{"name":"exact","value":{"kind":"Boolean","value":"true"}},{"name":"free_symbols","value":{"kind":"Array","type":"Vector","shape":[0],"elements":[]}},{"name":"node_count","value":{"kind":"Null"}},{"name":"canonical","value":{"kind":"Null"}},{"name":"pretty","value":{"kind":"Null"}},{"name":"shape","value":{"kind":"Null"}},{"name":"rank","value":{"kind":"Null"}},{"name":"element_domain","value":{"kind":"Null"}},{"name":"assumptions","value":{"kind":"Array","type":"Vector","shape":[0],"elements":[]}},{"name":"members","value":{"kind":"Null"}}]}` | HELD | - |
| B25 | `inspect(0.5)` | `--file B25.ls --omit-functions` | `{"kind":"Record","type":"Inspection","fields":[{"name":"type","value":{"kind":"Text","value":"Real"}},{"name":"domain","value":{"kind":"Null"}},{"name":"exact","value":{"kind":"Null"}},{"name":"free_symbols","value":{"kind":"Array","type":"Vector","shape":[0],"elements":[]}},{"name":"node_count","value":{"kind":"Null"}},{"name":"canonical","value":{"kind":"Null"}},{"name":"pretty","value":{"kind":"Null"}},{"name":"shape","value":{"kind":"Null"}},{"name":"rank","value":{"kind":"Null"}},{"name":"element_domain","value":{"kind":"Null"}},{"name":"assumptions","value":{"kind":"Array","type":"Vector","shape":[0],"elements":[]}},{"name":"members","value":{"kind":"Null"}}]}` | FINDING | P1 |
| C01 | `2^0` | `--file C01.ls --omit-functions` | `{"kind":"Natural","value":"1","exact":true}` | HELD | - |
| C02 | `0^0` | `--file C02.ls --omit-functions` | `{"kind":"Natural","value":"1","exact":true}` | HELD | - |
| C03 | `0^-1` | `--file C03.ls --omit-functions` | `{"ok":false,"code":"InvalidArgument","category":"TypeMismatch","message":"Base cannot be zero. (Parameter 'base')","recoverable":True}` | HELD | - |
| C04 | `0.0^-1` | `--file C04.ls --omit-functions` | `{"kind":"Real","value":"0","exact":true,"numerator":"0","denominator":"1"}` | FINDING | P0 |
| C05 | `(-2)^3` | `--file C05.ls --omit-functions` | `{"kind":"Integer","value":"-8","exact":true}` | HELD | - |
| C06 | `(-2)^-3` | `--file C06.ls --omit-functions` | `{"kind":"Real","value":"-0.125","exact":true,"numerator":"-1","denominator":"8"}` | HELD | - |
| C07 | `2^(1/2)` | `--file C07.ls --omit-functions` | `{"ok":false,"code":"UnsupportedOperation","category":"UnsupportedOperation","message":"Non-integer exponents are not yet supported.","recoverable":True}` | HELD | - |
| C08 | `2^0.5` | `--file C08.ls --omit-functions` | `{"ok":false,"code":"UnsupportedOperation","category":"UnsupportedOperation","message":"Non-integer exponents are not yet supported.","recoverable":True}` | HELD | - |
| C09 | `4^(1/2)` | `--file C09.ls --omit-functions` | `{"ok":false,"code":"UnsupportedOperation","category":"UnsupportedOperation","message":"Non-integer exponents are not yet supported.","recoverable":True}` | HELD | - |
| C10 | `(-8)^(1/3)` | `--file C10.ls --omit-functions` | `{"ok":false,"code":"UnsupportedOperation","category":"UnsupportedOperation","message":"Non-integer exponents are not yet supported.","recoverable":True}` | HELD | - |
| C11 | `(-4)^(1/2)` | `--file C11.ls --omit-functions` | `{"kind":"Symbolic","pretty":"2*i","canonical":"(mul (rat 2 1) (i))","domain":"complex","exact":true,"nodeCount":3,"freeSymbols":[]}` | FINDING | P1 |
| C12 | `(-4)^0.5` | `--file C12.ls --omit-functions` | `{"kind":"Symbolic","pretty":"2*i","canonical":"(mul (rat 2 1) (i))","domain":"complex","exact":true,"nodeCount":3,"freeSymbols":[]}` | FINDING | P1 |
| C13 | `2^-100` | `--file C13.ls --omit-functions` | `{"kind":"Real","value":"0.00000000(...88 digits abridged...)0625","exact":false}` | FINDING | P0 |
| C14 | `2^-1000` | `--file C14.ls --omit-functions` | `{"kind":"Real","value":"0.00000000(...988 digits abridged...)0625","exact":false}` | FINDING | P0 |
| C15 | `2^-1001` | `--file C15.ls --omit-functions` | `{"kind":"Real","value":"0.00000000(...988 digits abridged...)5312","exact":false}` | HELD | - |
| C16 | `2^-5000` | `--file C16.ls --omit-functions` | `{"kind":"Real","value":"0","exact":true,"numerator":"0","denominator":"1"}` | FINDING | P0 |
| C17 | `2^-30103` | `--file C17.ls --omit-functions` | `{"kind":"Real","value":"0","exact":true,"numerator":"0","denominator":"1"}` | FINDING | P0 |
| C18 | `10^-1000` | `--file C18.ls --omit-functions` | `{"kind":"Real","value":"0.00000000(...988 digits abridged...)0001","exact":false}` | FINDING | P0 |
| C19 | `10^-1001` | `--file C19.ls --omit-functions` | `{"kind":"Real","value":"0","exact":true,"numerator":"0","denominator":"1"}` | FINDING | P0 |
| C20 | `10^-1002` | `--file C20.ls --omit-functions` | `{"kind":"Real","value":"0","exact":true,"numerator":"0","denominator":"1"}` | FINDING | P0 |
| C21 | `(1/3)^-2` | `--file C21.ls --omit-functions` | `{"ok":false,"code":"UnsupportedOperation","category":"UnsupportedOperation","message":"Negative exponents are not yet supported.","recoverable":True}` | FINDING | P1 |
| C22 | `1^1000000` | `--file C22.ls --omit-functions` | `{"kind":"Natural","value":"1","exact":true}` | HELD | - |
| C23 | `(-1)^0.5` | `--file C23.ls --omit-functions` | `{"kind":"Symbolic","pretty":"i","canonical":"(i)","domain":"complex","exact":true,"nodeCount":1,"freeSymbols":[]}` | HELD | - |
| C24 | `0^-0.5` | `--file C24.ls --omit-functions` | `{"kind":"Real","value":"0","exact":true,"numerator":"0","denominator":"1"}` | FINDING | P0 |
| C25 | `(-1)^(1/2)` | `--file C25.ls --omit-functions` | `{"kind":"Symbolic","pretty":"i","canonical":"(i)","domain":"complex","exact":true,"nodeCount":1,"freeSymbols":[]}` | HELD | - |
| C26 | `(2^1000000) > 0` | `--file C26.ls --omit-functions` | `{"kind":"Boolean","value":"true"}` | HELD | - |
| C27 | `2^0.5 == sqrt(2)` | `--file C27.ls --omit-functions` | `{"ok":false,"code":"UnsupportedOperation","category":"UnsupportedOperation","message":"Non-integer exponents are not yet supported.","recoverable":True}` | HELD | - |
| C28 | `x = symbol("x"); x^(1/2)` | `--file C28.ls --omit-functions` | `{"kind":"Symbolic","pretty":"sqrt(x)","canonical":"(pow (sym x) (rat 1 2))","domain":"complex","exact":false,"nodeCount":3,"freeSymbols":["x"]}` | FINDING | P1 |
| C29 | `2^(1/3)` | `--file C29.ls --omit-functions` | `{"ok":false,"code":"UnsupportedOperation","category":"UnsupportedOperation","message":"Non-integer exponents are not yet supported.","recoverable":True}` | HELD | - |
| C30 | `(-2)^0.5` | `--file C30.ls --omit-functions` | `{"kind":"Symbolic","pretty":"i*sqrt(2)","canonical":"(mul (i) (pow (rat 2 1) (rat 1 2)))","domain":"complex","exact":false,"nodeCount":5,"freeSymbols":[]}` | FINDING | P1 |
| C31 | `0^1` | `--file C31.ls --omit-functions` | `{"kind":"Natural","value":"0","exact":true}` | HELD | - |
| C32 | `0^2` | `--file C32.ls --omit-functions` | `{"kind":"Natural","value":"0","exact":true}` | HELD | - |
| C33 | `2^-0` | `--file C33.ls --omit-functions` | `{"ok":false,"code":"InvalidArgument","category":"TypeMismatch","message":"Exponent must be positive. (Parameter 'exponent')","recoverable":True}` | FINDING | P2 |
| C34 | `2^(-0)` | `--file C34.ls --omit-functions` | `{"ok":false,"code":"InvalidArgument","category":"TypeMismatch","message":"Exponent must be positive. (Parameter 'exponent')","recoverable":True}` | FINDING | P2 |
| C35 | `2^2^3` | `--file C35.ls --omit-functions` | `{"kind":"Natural","value":"256","exact":true}` | HELD | - |
| D01 | `sqrt(4)` | `--file D01.ls --omit-functions` | `{"kind":"Real","value":"2","exact":true,"numerator":"2","denominator":"1"}` | HELD | - |
| D02 | `sqrt(2)^2` | `--file D02.ls --omit-functions` | `{"kind":"Real","value":"1.99999999(...88 digits abridged...)9999","exact":false}` | FINDING | P2 |
| D03 | `1 == 1.0` | `--file D03.ls --omit-functions` | `{"kind":"Boolean","value":"true"}` | HELD | - |
| D04 | `1 == 1/1` | `--file D04.ls --omit-functions` | `{"kind":"Boolean","value":"true"}` | HELD | - |
| D05 | `2 == 2.0` | `--file D05.ls --omit-functions` | `{"kind":"Boolean","value":"true"}` | HELD | - |
| D06 | `1/2 == 0.5` | `--file D06.ls --omit-functions` | `{"kind":"Boolean","value":"true"}` | HELD | - |
| D07 | `0.1 + 0.2 == 0.3` | `--file D07.ls --omit-functions` | `{"kind":"Boolean","value":"true"}` | HELD | - |
| D08 | `1/3 < 1/2` | `--file D08.ls --omit-functions` | `{"kind":"Boolean","value":"true"}` | HELD | - |
| D09 | `-1/3 > -1/2` | `--file D09.ls --omit-functions` | `{"kind":"Boolean","value":"true"}` | HELD | - |
| D10 | `1 == i` | `--file D10.ls --omit-functions` | `{"ok":false,"code":"InvalidOperation","category":"DomainError","message":"Undefined variable 'i'.","recoverable":True}` | HELD | - |
| D11 | `1 < i` | `--file D11.ls --omit-functions` | `{"ok":false,"code":"InvalidOperation","category":"DomainError","message":"Undefined variable 'i'.","recoverable":True}` | HELD | - |
| D12 | `1/3 == 0.3333333333333333333333333333333333333333333` | `--file D12.ls --omit-functions` | `{"kind":"Boolean","value":"false"}` | HELD | - |
| D13 | `2 > 1.999999999999999999999999999999999999999` | `--file D13.ls --omit-functions` | `{"kind":"Boolean","value":"true"}` | HELD | - |
| D14 | `1/(10^19) == 1/(10^19)` | `--file D14.ls --omit-functions` | `{"kind":"Boolean","value":"true"}` | HELD | - |
| D15 | `1/(10^19) > 0` | `--file D15.ls --omit-functions` | `{"kind":"Boolean","value":"true"}` | HELD | - |
| D16 | `type(1)` | `--file D16.ls --omit-functions` | `{"kind":"Text","value":"Natural"}` | HELD | - |
| D17 | `type(1/2)` | `--file D17.ls --omit-functions` | `{"kind":"Text","value":"Real"}` | HELD | - |
| D18 | `type(1.5)` | `--file D18.ls --omit-functions` | `{"kind":"Text","value":"Real"}` | HELD | - |
| D19 | `type(sqrt(2))` | `--file D19.ls --omit-functions` | `{"kind":"Text","value":"Real"}` | HELD | - |
| D20 | `type(i)` | `--file D20.ls --omit-functions` | `{"ok":false,"code":"InvalidOperation","category":"DomainError","message":"Undefined variable 'i'.","recoverable":True}` | HELD | - |
| D21 | `type(1+i)` | `--file D21.ls --omit-functions` | `{"ok":false,"code":"InvalidOperation","category":"DomainError","message":"Undefined variable 'i'.","recoverable":True}` | HELD | - |
| D22 | `type(2.0)` | `--file D22.ls --omit-functions` | `{"kind":"Text","value":"Real"}` | HELD | - |
| D23 | `0.0^0` | `--file D23.ls --omit-functions` | `{"kind":"Real","value":"1","exact":true,"numerator":"1","denominator":"1"}` | HELD | - |
| D24 | `0.0^0.5` | `--file D24.ls --omit-functions` | `{"kind":"Real","value":"0","exact":true,"numerator":"0","denominator":"1"}` | HELD | - |
| D25 | `(2/3)^-2` | `--file D25.ls --omit-functions` | `{"ok":false,"code":"UnsupportedOperation","category":"UnsupportedOperation","message":"Negative exponents are not yet supported.","recoverable":True}` | FINDING | P1 |
| D26 | `x = symbol("x"); x^-2` | `--file D26.ls --omit-functions` | `{"kind":"Symbolic","pretty":"x^-2","canonical":"(pow (sym x) (rat -2 1))","domain":"complex","exact":true,"nodeCount":3,"freeSymbols":["x"]}` | HELD | - |
| D27 | `x = symbol("x"); x^0` | `--file D27.ls --omit-functions` | `{"kind":"Symbolic","pretty":"1","canonical":"(rat 1 1)","domain":"rational","exact":true,"nodeCount":1,"freeSymbols":[]}` | HELD | - |
| D28 | `1 - 1` | `--file D28.ls --omit-functions` | `{"kind":"Natural","value":"0","exact":true}` | HELD | - |
| D29 | `1.0 - 1.0` | `--file D29.ls --omit-functions` | `{"kind":"Real","value":"0","exact":true,"numerator":"0","denominator":"1"}` | HELD | - |
| D30 | `sqrt(2)*sqrt(2)` | `--file D30.ls --omit-functions` | `{"kind":"Real","value":"1.99999999(...88 digits abridged...)9999","exact":false}` | FINDING | P2 |
| D31 | `sqrt(2)*sqrt(2) == 2` | `--file D31.ls --omit-functions` | `{"kind":"Boolean","value":"false"}` | FINDING | P2 |
| D32 | `1e-30` | `--file D32.ls --omit-functions` | `{"ok":false,"code":"InvalidOperation","category":"DomainError","message":"Expected ';' or end of input but found 'e' at position 1.","recoverable":True}` | HELD | - |
| D33 | `.5` | `--file D33.ls --omit-functions` | `{"ok":false,"code":"InvalidInput","category":"ParseError","message":"The string '.5' is not a valid decimal representation of a Real number.","recoverable":True}` | HELD | - |
| D34 | `1.` | `--file D34.ls --omit-functions` | `{"kind":"Real","value":"1","exact":true,"numerator":"1","denominator":"1"}` | HELD | - |
| D35 | `(1+i)*(1-i)` | `--file D35.ls --omit-functions` | `{"ok":false,"code":"InvalidOperation","category":"DomainError","message":"Undefined variable 'i'.","recoverable":True}` | HELD | - |
| E01 | `1/3 + 1/6` | `--file E01.ls --omit-functions` | `{"kind":"Real","value":"0.5","exact":true,"numerator":"1","denominator":"2"}` | HELD | - |
| E02 | `1/3 + 1/7` | `--file E02.ls --omit-functions` | `{"kind":"Real","value":"0.(476190)","exact":true,"numerator":"10","denominator":"21"}` | HELD | - |
| E03 | `1/3 - 1/3` | `--file E03.ls --omit-functions` | `{"kind":"Real","value":"0","exact":true,"numerator":"0","denominator":"1"}` | HELD | - |
| E04 | `1/2 + 1` | `--file E04.ls --omit-functions` | `{"kind":"Real","value":"1.5","exact":true,"numerator":"3","denominator":"2"}` | HELD | - |
| E05 | `2 * (1/3)` | `--file E05.ls --omit-functions` | `{"kind":"Real","value":"0.(6)","exact":true,"numerator":"2","denominator":"3"}` | HELD | - |
| E06 | `1/3 * 0` | `--file E06.ls --omit-functions` | `{"kind":"Real","value":"0","exact":true,"numerator":"0","denominator":"1"}` | HELD | - |
| E07 | `(1/3)*(1/3)*(1/3)` | `--file E07.ls --omit-functions` | `{"kind":"Real","value":"0.03703703(...988 digits abridged...)0369(962)","exact":true,"numerator":"99999999(...988 digits abridged...)9989","denominator":"27000000(...990 digits abridged...)0000"}` | FINDING | P0 |
| E08 | `(1/7)*(1/7)` | `--file E08.ls --omit-functions` | `{"kind":"Real","value":"0.02040816(...988 digits abridged...)7346(775510204081632653061224489795918367346938)","exact":true,"numerator":"12499999(...988 digits abridged...)9999","denominator":"61250000(...989 digits abridged...)0000"}` | FINDING | P0 |
| E09 | `(1/7)^1` | `--file E09.ls --omit-functions` | `{"kind":"Real","value":"0.(142857)","exact":true,"numerator":"1","denominator":"7"}` | HELD | - |
| E10 | `(2/7)*(7/2)` | `--file E10.ls --omit-functions` | `{"kind":"Real","value":"1","exact":true,"numerator":"1","denominator":"1"}` | HELD | - |
| E11 | `7/2` | `--file E11.ls --omit-functions` | `{"kind":"Real","value":"3.5","exact":true,"numerator":"7","denominator":"2"}` | HELD | - |
| E12 | `divrem(7, 2)` | `--file E12.ls --omit-functions` | `{"kind":"Text","value":"quotient = 3, remainder = 1"}` | FINDING | P1 |
| E13 | `divrem(7, 0)` | `--file E13.ls --omit-functions` | `{"ok":false,"code":"DivisionByZero","category":"DomainError","message":"Cannot divide by zero.","recoverable":True}` | HELD | - |
| E14 | `7 % 2` | `--file E14.ls --omit-functions` | `{"kind":"Natural","value":"1","exact":true}` | HELD | - |
| E15 | `7 % 0` | `--file E15.ls --omit-functions` | `{"ok":false,"code":"DivisionByZero","category":"DomainError","message":"Cannot divide by zero.","recoverable":True}` | HELD | - |
| E16 | `-7 % 2` | `--file E16.ls --omit-functions` | `{"kind":"Integer","value":"-1","exact":true}` | HELD | - |
| E17 | `7.5 % 2` | `--file E17.ls --omit-functions` | `{"kind":"Real","value":"1.5","exact":true,"numerator":"3","denominator":"2"}` | HELD | - |
| E18 | `divrem(2^1000, 3)` | `--file E18.ls --omit-functions` | `{"kind":"Text","value":"quotient = 35716953(...289 digits abridged...)3125, remainder = 1"}` | FINDING | P1 |
| E19 | `(2^1000) % 7` | `--file E19.ls --omit-functions` | `{"kind":"Natural","value":"2","exact":true}` | HELD | - |
| E20 | `floor(1/3)` | `--file E20.ls --omit-functions` | `{"ok":false,"code":"InvalidOperation","category":"DomainError","message":"Unknown function 'floor'.","recoverable":True}` | HELD | - |
| E21 | `ceil(1/3)` | `--file E21.ls --omit-functions` | `{"ok":false,"code":"InvalidOperation","category":"DomainError","message":"Unknown function 'ceil'.","recoverable":True}` | HELD | - |
| E22 | `abs(-1/3)` | `--file E22.ls --omit-functions` | `{"kind":"Real","value":"0.(3)","exact":true,"numerator":"1","denominator":"3"}` | HELD | - |
| E23 | `sign(-1/3)` | `--file E23.ls --omit-functions` | `{"ok":false,"code":"InvalidOperation","category":"DomainError","message":"sign() is not supported for values of kind 'Real'. Use Natural or Integer operands.","recoverable":True}` | HELD | - |
| E24 | `min(1/3, 1/2)` | `--file E24.ls --omit-functions` | `{"ok":false,"code":"InvalidOperation","category":"DomainError","message":"Index must be Natural or Integer, but got 'Real'.","recoverable":True}` | HELD | - |
| E25 | `max(1/3, 1/2)` | `--file E25.ls --omit-functions` | `{"ok":false,"code":"InvalidOperation","category":"DomainError","message":"Index must be Natural or Integer, but got 'Real'.","recoverable":True}` | HELD | - |
| E26 | `evalf(1/3, 20)` | `--file E26.ls --omit-functions` | `{"kind":"Real","value":"0.33333333333333333333","exact":false}` | HELD | - |
| E27 | `evalf(1/3, 200)` | `--file E27.ls --omit-functions` | `{"kind":"Real","value":"0.33333333(...188 digits abridged...)3333","exact":false}` | HELD | - |
| E28 | `evalf(sqrt(2), 50)` | `--file E28.ls --omit-functions` | `{"kind":"Real","value":"1.41421356(...88 digits abridged...)5727","exact":false}` | FINDING | P1 |
| E29 | `subs(x^2 - 1, x, 1/3)` | `--file E29.ls --omit-functions` | `{"ok":false,"code":"InvalidOperation","category":"DomainError","message":"Undefined variable 'x'.","recoverable":True}` | HELD | - |
| E30 | `x = symbol("x"); x + sqrt(2)` | `--file E30.ls --omit-functions` | `{"kind":"Symbolic","pretty":"x + 1.41421356(...988 digits abridged...)8472","canonical":"(add (real 1.41421356(...988 digits abridged...)8472) (sym x))","domain":"complex","exact":false,"nodeCount":3,"freeSymbols":["x"]}` | HELD | - |
| E31 | `x = symbol("x"); inspect(x + sqrt(2))` | `--file E31.ls --omit-functions` | `{"kind":"Record","type":"Inspection","fields":[{"name":"type","value":{"kind":"Text","value":"Symbolic"}},{"name":"domain","value":{"kind":"Domain","domain":"complex"}},{"name":"exact","value":{"kind":"Boolean","value":"false"}},{"name":"free_symbols","value":{"kind":"Array","type":"Vector","shape":[1],"elements":[{"kind":"Text","value":"x"}]}},{"name":"node_count","value":{"kind":"Natural","value":"3","exact":true}},{"name":"canonical","value":{"kind":"Text","value":"(add (real 1.41421356(...988 digits abridged...)8472) (sym x))"}},{"name":"pretty","value":{"kind":"Text","value":"x + 1.41421356(...988 digits abridged...)8472"}},{"name":"shape","value":{"kind":"Null"}},{"name":"rank","value":{"kind":"Null"}},{"name":"element_domain","value":{"kind":"Domain","domain":"complex"}},{"name":"assumptions","value":{"kind":"Array","type":"Vector","shape":[0],"elements":[]}},{"name":"members","value":{"kind":"Null"}}]}` | HELD | - |
| E32 | `x = symbol("x"); x + (1/3)*(1/3)` | `--file E32.ls --omit-functions` | `{"kind":"Symbolic","pretty":"x + 49999999(...988 digits abridged...)9999/45000000(...989 digits abridged...)0000","canonical":"(add (rat 49999999(...988 digits abridged...)9999 45000000(...989 digits abridged...)0000) (sym x))","domain":"complex","exact":true,"nodeCount":3,"freeSymbols":["x"]}` | FINDING | P0 |
| E33 | `x = symbol("x"); inspect(x + (1/3)*(1/3))` | `--file E33.ls --omit-functions` | `{"kind":"Record","type":"Inspection","fields":[{"name":"type","value":{"kind":"Text","value":"Symbolic"}},{"name":"domain","value":{"kind":"Domain","domain":"complex"}},{"name":"exact","value":{"kind":"Boolean","value":"true"}},{"name":"free_symbols","value":{"kind":"Array","type":"Vector","shape":[1],"elements":[{"kind":"Text","value":"x"}]}},{"name":"node_count","value":{"kind":"Natural","value":"3","exact":true}},{"name":"canonical","value":{"kind":"Text","value":"(add (rat 49999999(...988 digits abridged...)9999 45000000(...989 digits abridged...)0000) (sym x))"}},{"name":"pretty","value":{"kind":"Text","value":"x + 49999999(...988 digits abridged...)9999/45000000(...989 digits abridged...)0000"}},{"name":"shape","value":{"kind":"Null"}},{"name":"rank","value":{"kind":"Null"}},{"name":"element_domain","value":{"kind":"Domain","domain":"complex"}},{"name":"assumptions","value":{"kind":"Array","type":"Vector","shape":[0],"elements":[]}},{"name":"members","value":{"kind":"Null"}}]}` | FINDING | P0 |
| E34 | `x = symbol("x"); x + 1/(10^19)` | `--file E34.ls --omit-functions` | `{"kind":"Symbolic","pretty":"x + 0.0000000000000000001","canonical":"(add (real 0.0000000000000000001) (sym x))","domain":"complex","exact":false,"nodeCount":3,"freeSymbols":["x"]}` | FINDING | P0 |
| E35 | `x = symbol("x"); inspect(x + 1/(10^19))` | `--file E35.ls --omit-functions` | `{"kind":"Record","type":"Inspection","fields":[{"name":"type","value":{"kind":"Text","value":"Symbolic"}},{"name":"domain","value":{"kind":"Domain","domain":"complex"}},{"name":"exact","value":{"kind":"Boolean","value":"false"}},{"name":"free_symbols","value":{"kind":"Array","type":"Vector","shape":[1],"elements":[{"kind":"Text","value":"x"}]}},{"name":"node_count","value":{"kind":"Natural","value":"3","exact":true}},{"name":"canonical","value":{"kind":"Text","value":"(add (real 0.0000000000000000001) (sym x))"}},{"name":"pretty","value":{"kind":"Text","value":"x + 0.0000000000000000001"}},{"name":"shape","value":{"kind":"Null"}},{"name":"rank","value":{"kind":"Null"}},{"name":"element_domain","value":{"kind":"Domain","domain":"complex"}},{"name":"assumptions","value":{"kind":"Array","type":"Vector","shape":[0],"elements":[]}},{"name":"members","value":{"kind":"Null"}}]}` | FINDING | P0 |
| E36 | `sqrt(-1)` | `--file E36.ls --omit-functions` | `{"kind":"Symbolic","pretty":"i","canonical":"(i)","domain":"complex","exact":true,"nodeCount":1,"freeSymbols":[]}` | HELD | - |
| E37 | `abs(sqrt(-4))` | `--file E37.ls --omit-functions` | `{"kind":"Symbolic","pretty":"abs(2*i)","canonical":"(fn abs (mul (rat 2 1) (i)))","domain":"real","exact":true,"nodeCount":4,"freeSymbols":[]}` | FINDING | P2 |
| E38 | `conj(1 + sqrt(-4))` | `--file E38.ls --omit-functions` | `{"ok":false,"code":"InvalidOperation","category":"DomainError","message":"DSP builtins expect numeric/complex elements, but got 'AddExpr'.","recoverable":True}` | HELD | - |
| E39 | `re(1 + sqrt(-4))` | `--file E39.ls --omit-functions` | `{"ok":false,"code":"InvalidOperation","category":"DomainError","message":"DSP builtins expect numeric/complex elements, but got 'AddExpr'.","recoverable":True}` | HELD | - |
| E40 | `im(1 + sqrt(-4))` | `--file E40.ls --omit-functions` | `{"ok":false,"code":"InvalidOperation","category":"DomainError","message":"DSP builtins expect numeric/complex elements, but got 'AddExpr'.","recoverable":True}` | HELD | - |
| F01 | `setprecision(5); 0.123456789` | `--file F01.ls --omit-functions` | `{"kind":"Real","value":"0.123456789","exact":true,"numerator":"123456789","denominator":"1000000000"}` | HELD | - |
| F02 | `setprecision(5); 1.23456789` | `--file F02.ls --omit-functions` | `{"kind":"Real","value":"1.23456789","exact":true,"numerator":"123456789","denominator":"100000000"}` | FINDING | P2 |
| F03 | `setprecision(5); 1/(2^100)` | `--file F03.ls --omit-functions` | `{"kind":"Real","value":"0","exact":true,"numerator":"0","denominator":"1"}` | FINDING | P0 |
| F04 | `setprecision(5); 1/7` | `--file F04.ls --omit-functions` | `{"kind":"Real","value":"0.14285","exact":true,"numerator":"2857","denominator":"20000"}` | FINDING | P0 |
| F05 | `setprecision(2000); 1/(10^1001 - 1)` | `--file F05.ls --omit-functions` | `{"kind":"Real","value":"0.(00000000(...989 digits abridged...)0001)","exact":true,"numerator":"1","denominator":"99999999(...989 digits abridged...)9999"}` | HELD | - |
| F06 | `setprecision(2000); 1/(10^2001 - 1)` | `--file F06.ls --omit-functions` | `{"kind":"Real","value":"0","exact":true,"numerator":"0","denominator":"1"}` | FINDING | P0 |
| F07 | `setprecision(2000); 2^-5000` | `--file F07.ls --omit-functions` | `{"kind":"Real","value":"0.00000000(...1988 digits abridged...)6888","exact":false}` | HELD | - |
| F08 | `setprecision(2000); 2^-2000` | `--file F08.ls --omit-functions` | `{"kind":"Real","value":"0.00000000(...1988 digits abridged...)0625","exact":false}` | FINDING | P0 |
| F09 | `setprecision(0)` | `--file F09.ls --omit-functions` | `{"ok":false,"code":"InvalidOperation","category":"DomainError","message":"setprecision() expects a positive digit count, but got 0.","recoverable":True}` | HELD | - |
| F10 | `setprecision(-1)` | `--file F10.ls --omit-functions` | `{"ok":false,"code":"InvalidOperation","category":"DomainError","message":"setprecision() expects a positive digit count, but got -1.","recoverable":True}` | HELD | - |
| F11 | `setprecision(1)` | `--file F11.ls --omit-functions` | `NO result KEY; timings[0]={"resultKind":"Void"} (ValueKind.Void)` | FINDING | P2 |
| F12 | `setprecision(1/3)` | `--file F12.ls --omit-functions` | `{"ok":false,"code":"InvalidOperation","category":"DomainError","message":"setprecision() expects a Natural or Integer digit count, but got 'Real'.","recoverable":True}` | HELD | - |
| F13 | `setprecision(10^9)` | `--file F13.ls --omit-functions` | `NO result KEY; timings[0]={"resultKind":"Void"} (ValueKind.Void)` | FINDING | P2 |
| F14 | `dft([1,2,3,4])` | `--file F14.ls --omit-functions` | `{"kind":"Array","type":"Vector","shape":[4],"elements":[{"kind":"Complex","exact":true,"re":"10","im":"0"},{"kind":"Complex","exact":true,"re":"-2","im":"2"},{"kind":"Complex","exact":true,"re":"-2","im":"0"},{"kind":"Complex","exact":true,"re":"-2","im":"-2"}]}` | HELD | - |
| F15 | `det([[1,2],[3,4]])` | `--file F15.ls --omit-functions` | `{"kind":"Integer","value":"-2","exact":true}` | HELD | - |
| F16 | `conv([1,2,3],[1,1])` | `--file F16.ls --omit-functions` | `{"kind":"Array","type":"Vector","shape":[4],"elements":[{"kind":"Complex","exact":true,"re":"1","im":"0"},{"kind":"Complex","exact":true,"re":"3","im":"0"},{"kind":"Complex","exact":true,"re":"5","im":"0"},{"kind":"Complex","exact":true,"re":"3","im":"0"}]}` | HELD | - |
| F17 | `12345678901234567890 * 98765432109876543210` | `--file F17.ls --omit-functions` | `{"kind":"Natural","value":"1219326311370217952237463801111263526900","exact":true}` | HELD | - |
| F18 | `2^1000 + 1 - 2^1000` | `--file F18.ls --omit-functions` | `{"kind":"Natural","value":"1","exact":true}` | HELD | - |
| F19 | `(10^100 + 1) % 7` | `--file F19.ls --omit-functions` | `{"kind":"Natural","value":"5","exact":true}` | HELD | - |
| F20 | `divrem(10^100, 7)` | `--file F20.ls --omit-functions` | `{"kind":"Text","value":"quotient = 14285714(...88 digits abridged...)1428, remainder = 4"}` | FINDING | P1 |
| F21 | `(10^100)/3` | `--file F21.ls --omit-functions` | `{"kind":"Real","value":"33333333(...88 digits abridged...)3333.(3)","exact":true,"numerator":"10000000(...89 digits abridged...)0000","denominator":"3"}` | HELD | - |
| F22 | `sum(1..10)` | `--file F22.ls --omit-functions` | `{"kind":"Natural","value":"55","exact":true}` | HELD | - |
| F23 | `prod([1,2,3,4,5,6,7,8,9,10])` | `--file F23.ls --omit-functions` | `{"kind":"Natural","value":"3628800","exact":true}` | HELD | - |
| F24 | `2^70` | `--file F24.ls --omit-functions` | `{"kind":"Natural","value":"1180591620717411303424","exact":true}` | HELD | - |
| F25 | `(2^70) - 1` | `--file F25.ls --omit-functions` | `{"kind":"Natural","value":"1180591620717411303423","exact":true}` | HELD | - |
| G01 | `dft([1,2,3])` | `--file G01.ls --omit-functions` | `{"kind":"Array","type":"Vector","shape":[3],"elements":[{"kind":"Complex","exact":true,"re":"6","im":"0"},{"kind":"Complex","exact":false,"re":"-1.5","im":"0.866025403784438646763723170752"},{"kind":"Complex","exact":false,"re":"-1.5","im":"-0.866025403784438646763723170752"}]}` | HELD | - |
| G02 | `dft([1,0,1,0])` | `--file G02.ls --omit-functions` | `{"kind":"Array","type":"Vector","shape":[4],"elements":[{"kind":"Complex","exact":true,"re":"2","im":"0"},{"kind":"Complex","exact":true,"re":"0","im":"0"},{"kind":"Complex","exact":true,"re":"2","im":"0"},{"kind":"Complex","exact":true,"re":"0","im":"0"}]}` | HELD | - |
| G03 | `dft([1,0,0,1])` | `--file G03.ls --omit-functions` | `{"kind":"Array","type":"Vector","shape":[4],"elements":[{"kind":"Complex","exact":true,"re":"2","im":"0"},{"kind":"Complex","exact":true,"re":"1","im":"1"},{"kind":"Complex","exact":true,"re":"0","im":"0"},{"kind":"Complex","exact":true,"re":"1","im":"-1"}]}` | HELD | - |
| G04 | `x = 5` | `--file G04.ls --omit-functions` | `{"kind":"Natural","value":"5","exact":true}` | HELD | - |
| G05 | `setprecision(10^9); 1/3` | `--file G05.ls --omit-functions` | `{"kind":"Real","value":"0.(3)","exact":true,"numerator":"1","denominator":"3"}` | HELD | - |
| G06 | `setprecision(10^9); 1/7` | `--file G06.ls --omit-functions` | `{"kind":"Real","value":"0.(142857)","exact":true,"numerator":"1","denominator":"7"}` | HELD | - |
| G07 | `setprecision(10^9); 0.5` | `--file G07.ls --omit-functions` | `{"kind":"Real","value":"0.5","exact":true,"numerator":"1","denominator":"2"}` | HELD | - |
| G08 | `sqrt(2) < 1.4142135623730950488016887242 ...(body of 112 chars)...` | `--file G08.ls --omit-functions` | `{"kind":"Boolean","value":"true"}` | HELD | - |
| G09 | `1/3 > 0.33333333333333333333333333333333 ...(body of 109 chars)...` | `--file G09.ls --omit-functions` | `{"kind":"Boolean","value":"true"}` | HELD | - |
| G10 | `-2^-1001` | `--file G10.ls --omit-functions` | `{"kind":"Real","value":"-0.00000000(...988 digits abridged...)5312","exact":false}` | HELD | - |
| G11 | `2^-1001 * -1` | `--file G11.ls --omit-functions` | `{"kind":"Real","value":"-0.00000000(...988 digits abridged...)5312","exact":false}` | HELD | - |
| G12 | `(1/3)*(1/3) - 1/9` | `--file G12.ls --omit-functions` | `{"kind":"Real","value":"0","exact":true,"numerator":"0","denominator":"1"}` | FINDING | P0 |
| G13 | `(1/97)*97 - 1` | `--file G13.ls --omit-functions` | `{"kind":"Real","value":"-0.00000000(...988 digits abridged...)0091","exact":false}` | HELD | - |
| G14 | `1/(10^1001 - 1) + 1/(10^1001 - 1)` | `--file G14.ls --omit-functions` | `{"kind":"Real","value":"0","exact":true,"numerator":"0","denominator":"1"}` | FINDING | P0 |
| G15 | `(1/(10^1001 - 1)) * 2` | `--file G15.ls --omit-functions` | `{"kind":"Real","value":"0","exact":true,"numerator":"0","denominator":"1"}` | FINDING | P0 |
| H01 | `evalf(pi, 50)` | `--file H01.ls --omit-functions` | `{"kind":"Real","value":"3.14159265(...88 digits abridged...)0679","exact":false}` | FINDING | P1 |
| H02 | `evalf(e, 50)` | `--file H02.ls --omit-functions` | `{"kind":"Real","value":"2.71828182(...88 digits abridged...)4274","exact":false}` | FINDING | P1 |
| H03 | `evalf(sin(1), 50)` | `--file H03.ls --omit-functions` | `{"kind":"Real","value":"0.84147098(...38 digits abridged...)9837","exact":false}` | HELD | - |
| H04 | `evalf(sqrt(2), 500)` | `--file H04.ls --omit-functions` | `{"kind":"Real","value":"1.41421356(...88 digits abridged...)5727","exact":false}` | FINDING | P1 |
| H05 | `evalf(1/3, 500)` | `--file H05.ls --omit-functions` | `{"kind":"Real","value":"0.33333333(...488 digits abridged...)3333","exact":false}` | HELD | - |
| H06 | `evalf(1/7, 30)` | `--file H06.ls --omit-functions` | `{"kind":"Real","value":"0.142857142857142857142857142857","exact":false}` | HELD | - |
| H07 | `evalf(sqrt(2), 20)` | `--file H07.ls --omit-functions` | `{"kind":"Real","value":"1.41421356(...88 digits abridged...)5727","exact":false}` | FINDING | P1 |
| H08 | `evalf(2, 20)` | `--file H08.ls --omit-functions` | `{"kind":"Integer","value":"2","exact":true}` | HELD | - |
| H09 | `1/3 - 0.33333333333333333333333333333333 ...(body of 1008 chars)...` | `--file H09.ls --omit-functions` | `{"kind":"Real","value":"0","exact":true,"numerator":"0","denominator":"1"}` | FINDING | P0 |
| H10 | `setprecision(18); 1/1009` | `--file H10.ls --omit-functions` | `{"kind":"Real","value":"0.000991080277502477","exact":true,"numerator":"991080277502477","denominator":"1000000000000000000"}` | FINDING | P0 |
| H11 | `setprecision(5); 1.2345678901234` | `--file H11.ls --omit-functions` | `{"kind":"Real","value":"1.2345678901234","exact":true,"numerator":"6172839450617","denominator":"5000000000000"}` | FINDING | P2 |
| H12 | `setprecision(1)` | `--file H12.ls --omit-functions` | `NO result KEY; timings[0]={"resultKind":"Void"} (ValueKind.Void)` | FINDING | P2 |
| H13 | `print(1)` | `--file H13.ls --omit-functions` | `NO result KEY; timings[0]={"resultKind":"Void"} (ValueKind.Void)` | FINDING | P2 |
| H14 | capabilities() (full metadata) | --file H14.ls | {"kind":"Record","type":"CapabilitiesResult","fields":[ supported_domains [real, complex], unsupported_domains [integer, rational], unsupported_operations [16 UnsupportedCapability records], exactness BestEffort ]} + a top-level functions array of 123 builtins, each {"name":...,"parameters":[...],"builtin":true} | HELD | - |


# FINDINGS

Severity per the task's definitions. Every finding below was reproduced a second time from a clean
temp file; the envelope quoted is that second run (under `%TEMP%\a1numeric\clean2\` or
`%TEMP%\a1numeric\clean3\`), except where the second run is the same probe's row in the table above
(also a fresh temp directory).

## P0

### N-01 — Division truncates at 1000 fractional digits and a nonzero rational is published as exactly zero
**One sentence.** `1/(10^1001 - 1)` is a positive rational of period 1001; the long division is truncated
to 1000 fractional digits so the whole quotient is zeros, and the wire reports the value as `0` with
`exact:true` and `numerator 0 / denominator 1`.

Reproduction (second run, `clean3\R01b\R01b.ls`, body `1/(10^1001 - 1)`):
```json
{"protocolVersion":1,"symbolicFormatVersion":"#!lovelace-sym 1","mathIrVersion":2,"ok":true,"revision":81,
 "result":{"kind":"Real","display":"0","typed":"0 (Real)",
   "structured":{"kind":"Real","value":"0","exact":true,"numerator":"0","denominator":"1"}},
 "output":[],"variables":[{"name":"_","kind":"Real","display":"0"}],"elapsed":"11.02 ms", ...}
```
A second independent run (`clean2\R01`) produced the identical `structured` object. SymPy 1.14.0:
`N(1/(10**1001-1), 15) = 1.00000000000000e-1001` and `Rational(1,10**1001-1) == 0` is `False`.

Same root cause, same wire shape, each confirmed twice: `1/(10^1002 - 1)` (row A10, re-run
`clean3\R19`), `1/(10^1001)` (B09), `1/(10^1002)`, `setprecision(5); 1/(2^100)` (F03 → `value 0,
exact true, num 0, den 1`), and at a wider window `setprecision(2000); 1/(10^2001 - 1)` (F06).
Consequences already on the wire (each a row, re-run): `== 0` → `true` (B01), `> 0` → `false` (B03),
`v + 1` → `1` (B02), `v - 1` → `-1` (B06), `v * (10^1001-1)` → `0` (B04), `v + v` → `0` (G14),
`2v` → `0` (G15).

**Contrast that isolates the mechanism:** `setprecision(2000); 1/(10^1001 - 1)` (F05) returns the correct
`0.(000…0001)` with `numerator 1`, `denominator 10^1001-1` — the collapse is the width of the truncation
window, not an inherent limit. (Mechanism read from the tree, supporting only: `Lovelace.Real/Real.cs`
`Divide`, loop `while (!Nat.IsZero(remainder) && position < MaxComputationDecimalPlaces)`, default
`MaxComputationDecimalPlaces = 1000`.)

### N-02 — Negative powers underflow to an exact zero
**One sentence.** `2^-5000` (≈ 7.0798e-1506) is reported as `0` with `exact:true` and
`numerator 0 / denominator 1`.

Reproduction (second run `clean2\R02`, body `2^-5000`):
```json
{"kind":"Real","value":"0","exact":true,"numerator":"0","denominator":"1"}
```
Same for `2^-30103` (C17, ≈1e-9062), `10^-1001` (C19) and `10^-1002` (C20). SymPy:
`Rational(1,2**5000) == 0` is `False`; `N(2**-5000, 15) = 7.07981126104817e-1506`. The boundary is
confirmed from below: `2^-1000` (exactly representable, 302 significant digits) is *not* flushed,
`2^-1001` is truncated to 1000 digits with `exact:false` (C15, verified digit-for-digit against
`5^1001`), and `2^-5000` collapses to zero.

### N-03 — Zero to a negative power returns an exact 0 on the Real path while the integer path refuses
**One sentence.** `0.0^-1` and `0^-0.5` return `0` with `exact:true`, while `0^-1` is refused with
"Base cannot be zero", so one operation has two contradictory outcomes that depend only on the operand
kind.

Reproduction (second run `clean2\R03`, body `0.0^-1`):
```json
{"kind":"Real","value":"0","exact":true,"numerator":"0","denominator":"1"}
```
Reproduction of the integer path (second run `clean3\R18`, body `0^-1`):
```json
{"protocolVersion":1,...,"ok":false,"code":"InvalidArgument","category":"TypeMismatch",
 "message":"Base cannot be zero. (Parameter 'base')","recoverable":true,
 "diagnostics":[{"message":"Base cannot be zero. (Parameter 'base')","position":0,"line":1,"column":1}],"elapsed":"193.1 µs"}
```
Ground truth: SymPy `S.Zero**-1` is `zoo` (complex infinity); Python `0**-1` raises `ZeroDivisionError`.
`0^-0.5` (C24) is the same defect.

### N-04 — Multiplication of two exactly represented periodic values returns a wrong rational stamped exact:true
**One sentence.** `(1/3)*(1/3)` returns `0.111…1110(8)` with `exact:true` and a published
`numerator` of 1000 digits over a `denominator` of 1001 digits equal to 1/9 − 1/(4.5·10^999) ≠ 1/9.

Reproduction (second run `clean2\R04`, body `(1/3)*(1/3)`; digit runs abridged exactly as in the table,
lengths preserved):
```json
{"kind":"Real",
 "value":"0.11111111(...988 digits abridged...)1110(8)",
 "exact":true,
 "numerator":"49999999(...988 digits abridged...)9999",
 "denominator":"45000000(...989 digits abridged...)0000"}
```
(the `structured` object exactly as the same abridger renders it in row B18 above). Full lengths:
`value` 1005 chars, `numerator` 1000 digits, `denominator` 1001 digits; the value ends `…1110(8)`, i.e.
the engine places a period `(8)` after the truncation. Verified with `fractions.Fraction`: the reported
rational ≠ `Fraction(1,9)`, error `1/4500…0` ≈ 2.2e-1000. SymPy:
`Rational(1,3)*Rational(1,3) = 1/9`. Same class, each confirmed twice: `(1/7)*(1/11)` ≠ 1/77 (B19),
`(1/7)*(1/7)` ≠ 1/49 (E08), `(1/3)*(1/3)*(1/3)` ≠ 1/27 (E07).
**Amplification into the symbolic tower:** `x = symbol("x"); x + (1/3)*(1/3)` (E32/E33) publishes
`canonical = (add (rat 4999…/4500…) (sym x))` with `exact: true` — the false rational becomes an exact
symbolic coefficient.

### N-05 — The product of two exactly published values is not their product, and == agrees with the wrong value
**One sentence.** `(1/97)*97`, where `1/97` is published as exactly 1/97, returns `0.999…909` and
`(1/97)*97 == 1` returns `false`.

Reproduction (second run `clean2\R05`, body `(1/97)*97`):
```json
{"kind":"Real","value":"0.99999999(...988 digits abridged...)9909","exact":false}
```
(value length 1002 → 1000 fractional digits, all nine except the last two; the exact shortfall from 1 is
`91/10^1000`, checked with `Fraction`.) `(1/1009)*1009` (B11/B12) and `(1/(10^1001-1))*(10^1001-1)`
(B04, which returns `0`) are the same defect. SymPy: `Rational(1,97)*97 = 1`. `(1/3)*3` and `(1/7)*7` do
return exactly `1` (B13/B14), so the defect is period-length dependent.

### N-06 — A nonzero difference is flushed to an exact zero
**One sentence.** `1/3 - 0.<999 threes>4` — where the literal is exactly 1/3 + 10^-1000, so the true
difference is −10^-1000 — is reported as `0` with `exact:true`.

Reproduction (second run `clean3\R06b\R06b.ls`; body `1/3 - 0.<999 threes>4`, 1008 chars):
```json
{"protocolVersion":1,...,"ok":true,"revision":81,
 "result":{"kind":"Real","display":"0","typed":"0 (Real)",
   "structured":{"kind":"Real","value":"0","exact":true,"numerator":"0","denominator":"1"}},
 "elapsed":"166.92 ms", ...}
```
Verified with `Fraction`: `Fraction(1,3) - Fraction(int('3'*999+'4'), 10**1000) = -1/10**1000` ≠ 0.
**Self-contradiction between two statements of the same engine:** `(1/3)*(1/3) - 1/9` (G12, re-run
`clean2\R17`) returns `0` with `exact:true` while `(1/3)*(1/3)` itself (N-04) returns an exact rational
≠ 1/9; both cannot be true. Boundary evidence for the mechanism: `(1/97)*97 - 1` (G13) has a two-digit
tail (`-91e-1000`) and is *not* flushed — the flush needs a single nonzero digit in the last stored
position.

### N-07 — The exact flag denies exactness to exact terminating decimals of 19 or more fractional digits
**One sentence.** `1/(10^19)` prints its exact value `0.0000000000000000001` but reports `exact:false` and
publishes no `numerator`/`denominator`, although the same engine reports `1/(10^18)` (A17) and
`1/(3*10^19)` (A24, value `0.0000000000000000000(3)`) as `exact:true`.

Reproduction (second run `clean2\R09`, body `1/(10^19)`):
```json
{"kind":"Real","value":"0.0000000000000000001","exact":false}
```
Same, each confirmed twice: `1/(2^100)` (A19, the exact 102-char decimal), `10^-1000` (C18), `2^-100`
(C13), `2^-1000` (C14), `1/(10^1000)` (B10), `setprecision(2000); 2^-2000` (F08, exact 2002-char
decimal). SymPy: these are exact rationals; their printed decimals are the true values.
**Amplification:** `x = symbol("x"); x + 1/(10^19)` (E34/E35) crosses as
`(add (real 0.0000000000000000001) (sym x))` with `exact: false`, so an exact rational enters symbolic
arithmetic as an inexact literal.

## P1

### N-08 — divrem publishes two integers only as a display string
**One sentence.** `divrem(7, 2)` returns `{"kind":"Text","value":"quotient = 3, remainder = 1"}`, so the
quotient and remainder can only be recovered by parsing prose, contradicting dsh-protocol.md invariant 2
("Every value is structural … nothing degrades to text because the serializer lacked a case").

Reproduction (second run `clean2\R10`, body `divrem(7, 2)`):
```json
{"kind":"Text","value":"quotient = 3, remainder = 1"}
```
Also `divrem(2^1000, 3)` (E18) and `divrem(10^100, 7)` (F20); their *values* are correct (remainders 1
and 4, matching SymPy) — only the encoding is prose.

### N-09 — inspect(<Real>).exact is Null while the Real value form carries exact
**One sentence.** `inspect(1/3)` reports `"exact": {"kind":"Null"}` for a value whose own wire form is
`"exact":true`, so "is this number exact?" cannot be asked through the inspection record.

Reproduction (second run `clean2\R11`, body `inspect(1/3)`):
```json
{"kind":"Record","type":"Inspection","fields":[
  {"name":"type","value":{"kind":"Text","value":"Real"}},
  {"name":"domain","value":{"kind":"Null"}},
  {"name":"exact","value":{"kind":"Null"}},
  {"name":"free_symbols","value":{"kind":"Array","shape":[0],"elements":[]}},
  {"name":"node_count","value":{"kind":"Null"}}, ... {"name":"members","value":{"kind":"Null"}}]}
```
Identical shape for `inspect(0.5)` (B25), `inspect(1/(10^19))` (B22) and `inspect(sqrt(2))` (B23);
contrast `inspect(1)` (B24) → `"exact":true`. The `type` field also cannot separate the exact 1/3 from the
truncated `sqrt(2)` (both `Real`).

### N-10 — evalf(f, digits) ignores digits for pi, e and sqrt(2)
**One sentence.** `evalf(sqrt(2), 20)` and `evalf(sqrt(2), 500)` both return exactly 100 fractional
digits, and `evalf(pi, 50)`/`evalf(e, 50)` return 100, although the declared parameter list is
`["f","digits"]` and 100 is the ambient default.

Reproduction (second run `clean2\R12`, body `evalf(sqrt(2), 500)`):
```json
{"kind":"Real","value":"1.4142135623730950488016887242096980785696718753769480731766797379907324784621070388503875343276415727","exact":false}
```
(100 fractional digits; SymPy agrees with all 100, first mismatch none — the digits are right, the
requested count is ignored.) `evalf(pi, 50)` and `evalf(e, 50)` (H01/H02) likewise return 100 digits,
while `evalf(1/3, 500)` (H05) returns 500 and `evalf(1/7, 30)` (H06) returns 30 — the parameter works for
rational arguments only, so more than 100 digits of an irrational constant are unreachable through
`evalf`. (Pre-recorded in the repo's own audit trail: docs/goal-cycle-5/evidence.md EVD-211 and
docs/goal-cycle-4/round-09/audit-P4-symbolic.md F7.)

### N-11 — The exponent capability boundary is declared one way and behaves another
**One sentence.** `capabilities()` declares "Non-integer exponents are not yet supported"
(`operation_class: pow.non-integer-exponent`) yet `(-4)^(1/2)` returns the exact `2*i`, `(-4)^0.5`
returns `2*i`, `(-2)^0.5` returns `i*sqrt(2)` and `x^(1/2)` returns `sqrt(x)`; conversely `(1/3)^-2` and
`(2/3)^-2` are refused as "Negative exponents are not yet supported" although `2^-100` and `(-2)^-3`
work, and no negative-exponent limitation is declared anywhere in `capabilities()`.

Reproduction (second run `clean2\R13`, body `(-4)^(1/2)`):
```json
{"kind":"Symbolic","pretty":"2*i","domain":"complex","exact":true,"nodeCount":3,"canonical":"(mul (rat 2 1) (i))"}
```
Reproduction (second run `clean2\R14`, body `(1/3)^-2`; the SymPy value is exactly 9):
```json
{"ok":false,"code":"UnsupportedOperation","category":"UnsupportedOperation","recoverable":true,
 "message":"Negative exponents are not yet supported."}
```

## P2

### N-12 — 2^-0 is refused although 2^0 is 1
`2^-0` and `2^(-0)` → `{"ok":false,"code":"InvalidArgument","category":"TypeMismatch","message":"Exponent must be positive. (Parameter 'exponent')"}`
(C33/C34, re-run `clean2\R15`), while `2^0` → `Natural 1` (C01). The message's own rule ("must be
positive") is contradicted by the accepted zero exponent.

### N-13 — Two renderings of one value in one envelope disagree
`setprecision(5); 1.2345678901234` (second run `clean3\R16b`) gives `result.structured.value =
"1.2345678901234"` (14 digits) and `variables[0].display = "1.23456"` (5 digits) for the same value `_`:
```json
{"result":{"kind":"Real","display":"1.2345678901234","typed":"1.2345678901234 (Real)",
  "structured":{"kind":"Real","value":"1.2345678901234","exact":true,"numerator":"6172839450617","denominator":"5000000000000"}},
 "variables":[{"name":"_","kind":"Real","display":"1.23456"}]}
```
The result is rendered after the per-statement precision scope is popped. dsh-protocol.md requires the
analogous dual encodings (`elapsed`/`elapsedTime`) to come from one selector "so the two forms can never
disagree"; here they disagree. No numeric value is affected — the structured value is exact and correct.

### N-14 — The engine's own square root does not square back to its argument
`sqrt(2)^2` and `sqrt(2)*sqrt(2)` → `1.9999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999999`
with `exact:false`, and `sqrt(2)*sqrt(2) == 2` → `false` (D02/D30/D31). Honest about inexactness, but a
consumer cannot close the identity. `sqrt(4)` → `2` exact (D01) and all 100 printed digits of `sqrt(2)`
match mpmath (B20), so this is precision loss, not a wrong digit.

### N-15 — abs(2*i) is returned unevaluated
`abs(sqrt(-4))` → `{"kind":"Symbolic","pretty":"abs(2*i)","domain":"real","exact":true}` (E37) although
the value is exactly 2. No false claim (the expression is a valid representation), but a documented
builtin does not reduce an exact real magnitude.

### N-16 — A Void last statement omits result entirely
`setprecision(1)` (F11, re-run `clean2` and `final\H12`) and `print(1)` (H13) produce an envelope with **no
`result` key at all**, only `"timings":[{"position":0,...,"resultKind":"Void"}]`. dsh-protocol.md
invariant 3 says an absent field is `{"kind":"Null"}`, so a consumer must special-case a missing key.
(Invariant 4 promises only that `protocolVersion`, `symbolicFormatVersion` and `mathIrVersion` are always
present, so the doc does not unambiguously require `result` — hence P2 rather than P1.)

### N-17 — setprecision accepts an unbounded digit count
`setprecision(10^9)` is accepted without error (F13; `setprecision(0)` and `setprecision(-1)` are
correctly rejected). The knob is documented as "the engine computation and display precision (Real
decimal places)" and the window is demonstrably honoured at 2000 (F05/F06), so a script can ask for a
10^9-digit computation. No wrong answer observed; see "Could not decide" item 2.

# COULD NOT DECIDE

1. **What `exact` is supposed to mean.** Under "the stored value is exact", N-07 (`1/(2^100)` printed
   exactly yet `exact:false`) is a false denial; under "the result is the exact mathematical answer",
   N-06b (`setprecision(18); 1/1009` truncated yet `exact:true`) is a false affirmation. The binary is
   inconsistent under *both* readings — `1/(3*10^19)` is exact and `1/(2^100)` is not, though both are
   exact rationals carried exactly. Missing evidence: a written definition of `exact` in
   dsh-protocol.md or capabilities(); the protocol documents the field's shape but never its predicate.
   Both directions are therefore reported as findings rather than resolved.
2. **Whether `setprecision(10^9)` really allocates a 10^9-digit budget.** `setprecision(10^9); 1/3`
   returned promptly (G05) and the window is honoured at 2000, so the value is accepted at face value,
   but the experiment that would force a 10^9-digit expansion
   (`setprecision(10^9); 1/(10^1000000001 - 1)`) was deliberately not run: it would have hung the audit.
   Missing evidence: a timeout-bounded run on a machine the auditor may exhaust.
3. **Whether conj/re/im/abs refusing or not reducing symbolic complex values breaks a contract.**
   `conj(1 + sqrt(-4))`, `re(...)` and `im(...)` fail with a DSP-worded error ("DSP builtins expect
   numeric/complex elements") and `abs(2*i)` stays symbolic (E37–E40). capabilities() declares no
   complex-algebra builtin surface, so no declared claim is contradicted, and no completeness claim can
   be tested either.
4. **Rounding as a user-visible operation.** `floor`, `ceil` and `round` are not builtins — they fail as
   "Unknown function" (E20/E21) and capabilities() lists none of them, so there is no rounding contract to
   attack. The only rounding observable is the division direction, and that was decided: `2^-1001` matches
   **truncation** of `5^1001/10^1001` to 1000 digits, not round-half-up (verified digit-for-digit against
   Python integers), and the truncation is symmetric for negatives (`-2^-1001`, G10).

# SUMMARY

Of **214 probes** (213 distinct bodies; one body deliberately run twice) executed against
`out/aot/Lovelace.Run.exe` from clean temp files with SymPy 1.14.0 / `fractions.Fraction` / mpmath as
inline ground truth, **140 HELD**, **74 were FINDING rows** collapsing to **18 distinct findings — 8 P0,
4 P1, 6 P2**, and **0 were inconclusive** (4 questions are listed above as undecidable for lack of a
written predicate, not for lack of a run). The P0s: N-01 a nonzero rational published as exactly `0`
(`1/(10^1001-1)`, with `== 0`, `> 0`, `+1` and `*den` all following the wrong value); N-02 negative-power
underflow to exact `0` (`2^-5000`); N-03 `0.0^-1` = exact `0` where the integer path refuses; N-04
`(1/3)*(1/3)` published as an exact rational ≠ 1/9 (and `(1/7)*(1/11)`, `(1/7)*(1/7)`, `(1/3)^3`, which
also poison the symbolic coefficient); N-05 `(1/97)*97 != 1` with `== 1` false; N-06 a nonzero difference
flushed to exact `0`, with the envelope contradicting itself between `(1/3)*(1/3)` and
`(1/3)*(1/3) - 1/9`; N-06b a truncation certified exact with a wrong `numerator`/`denominator` once a
precision knob puts it inside 18 digits (`setprecision(18); 1/1009`, `setprecision(5); 1/7`); and N-07
exact terminating decimals of 19+ digits denied exactness and denied their rational form (`1/(10^19)`,
`1/(2^100)`). The P1s: N-08 `divrem` returning prose, N-09 `inspect(<Real>).exact` being `Null`, N-10
`evalf(f, digits)` ignoring `digits` for pi/e/sqrt(2), N-11 the exponent capability declared one way and
behaving another. What HELD is worth as much: the whole INTEGER tower (arbitrary-precision products,
`divrem` values, `%`, `det`, `sum`/`prod`, comparisons on `2^1000000`) matched SymPy exactly; every period
the engine did detect was the true one (`1/97` → 96, `1/1009` → 252 = `n_order(10,1009)`);
`num/den` was internally consistent with the printed value and always reduced in every non-N-04 case;
`sqrt(2)`'s 100 digits match mpmath; `dft` marks irrational components `exact:false` and integer ones
`exact:true`; comparison and equality across Natural, Integer and Real behaved correctly, including
`0.1 + 0.2 == 0.3` → `true`; and errors stay structural with stable codes (`DivisionByZero`/`DomainError`,
`InvalidArgument`/`TypeMismatch`, `InvalidInput`/`ParseError`) instead of exceptions.
