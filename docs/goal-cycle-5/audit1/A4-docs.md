# A4 — Documentation attack: docs/symbolics/dsh-protocol.md vs the published AOT binary

**Product under test:** out/aot/Lovelace.Run.exe (Native AOT, published 2026-09-11 13:41 from the current tree; the "revision" field reads 80–89 depending on the script).
**Contract attacked:** docs/symbolics/dsh-protocol.md (210 lines) + the capabilities() record, plus the user-facing READMEs (README.md, Lovelace.Symbolics/README.md) and the executable claims in .github/workflows/ci.yml.
**Independent ground truth:** SymPy 1.14.0 (C:\Users\ricar\dev\.lovelace-tools\python), run inline for every mathematical result quoted.
**Auditor:** independent adversarial pass. No repository file was modified; only this deliverable and temp scripts under %TEMP% were written.

## 0. Method and conventions

Every probe is a concrete input with an expected observable derived **from the contract or from mathematics**, never from binary output. All probes ran as

    out\aot\Lovelace.Run.exe --file %TEMP%\<id>.ls [--print-budget N | --cancel-after N]     (plus --omit-functions unless noted)

and the raw envelope was captured through a byte-exact redirect (Start-Process -RedirectStandardOutput) unless a row says otherwise. An earlier capture path (PowerShell "&") was suspected of fabricating one result; that suspicion was tested at byte level and rejected before anything was reported (see F3).

Abridgement legend used in the raw-output column (nothing that decides a verdict is dropped):

| token | meaning |
|---|---|
| PV1/SFV/MIR2 | "protocolVersion":1, "symbolicFormatVersion":"#!lovelace-sym 1", "mathIrVersion":2 |
| ERR(cat/code) | "ok":false plus the "code"/"category" shown |
| S: | result.structured |
| Sym(p\|c) | {"kind":"Symbolic","pretty":p,"canonical":c, …} |
| E(T:v) / Arr[n] / T(x) | Enum type:value / Array shape / Text |
| … | elided digits or repeated sibling records; never a deciding field |

**Verdict rule (stated once, applied uniformly):** *FINDING* = at least one contract-derived expected observable did not hold; *HELD* = every expected observable held; *INCONCLUSIVE* = the probe did not exercise its intended observable.

## 1. Probe table

### 1.1 dsh-protocol.md invariants and the envelope (P, Q, U groups)

| id | what was probed | command (script / args) | raw output (abridged) | verdict | sev |
|---|---|---|---|---|---|
| P01 | Inv. 1: stdout is one envelope; print lands in output; no result key for a Void last statement | print("hello") | keys = protocolVersion,symbolicFormatVersion,mathIrVersion,ok,revision,output,variables,functions,elapsed,elapsedTime,timings; "output":["hello"]; timings=[{position:0,resultKind:"Void",hasOutput:true}]; no result key | HELD (see F10) | P3 |
| P02 | Inv. 2/3: scalar value form | 1 + 1 | S:{"kind":"Natural","value":"2","exact":true}; variables=[{name:"_",kind:"Natural",display:"2"}] | HELD | — |
| P03 | SolveResult shape and field order (section "A real solve envelope") | x=symbol("x"); solve_full(x^2-4==0,x) | S:SolveResult{status=E(SolveStatus:Solved), variable=Sym(x\|(sym x)), domain=Domain(complex), complete=Bool(true), completeness=E(Completeness:Complete), solutions=Arr[2]/Vector(Solution{value=Sym(-2\|(rat -2 1)), conditions=Arr[0]/Vector(), multiplicity=Int(1,exact=True), exactness=E(SolutionExactness:Exact)}; Solution{value=Sym(2\|(rat 2 1)), …}), families=Arr[0], common_conditions=Arr[0], represented_count=Int(2), unrepresented_count=Int(0), unrepresented_reason=T(""), diagnostics=Arr[0]/Vector()} | HELD | — |
| P04 | Diagnostic form: exactly six fields in declared order; details an Array; Partial status | x=symbol("x"); solve_full(x^4-x^2-1==0,x) | status=E(SolveStatus:Partial), complete=Bool(false), completeness=E(Completeness:Partial), unrepresented_count=Int(2), unrepresented_reason=T("complex algebraic roots not supported (RootOf is real-only in v1)."), diagnostics=Arr[1]/Vector(Diagnostic{code=T(solve.unrepresented-roots), category=E(ErrorCategory:UnsupportedOperation), message=T(…), recoverable=Bool(true), location=NULL, details=Arr[0]/Vector()}) — diagnostics IS the last field | HELD | — |
| P05 | Lines 149–151: nothing representable implies Unevaluated, never a complete-looking subset | x=symbol("x"); solve_full(x^4+1==0,x) | status=E(SolveStatus:Unevaluated), complete=Bool(false), completeness=E(Completeness:Unknown), solutions=Arr[0], unrepresented_count=Int(4), diagnostics=Arr[1](code=T(solve.unevaluated)) | HELD | — |
| P06 | Lines 166–170: provably empty over the requested domain implies NoSolutions, complete:true / Complete | x=symbol("x"); solve_full(x^2+1==0,x,real) | status=E(SolveStatus:NoSolutions), domain=Domain(real), complete=Bool(true), completeness=E(Completeness:Complete), solutions=Arr[0], diagnostics=Arr[1](code=T(solve.no-solutions), category=E(ErrorCategory:NoSolution), message=T("no real solutions")) | HELD | — |
| P07 | Error envelope: domain rejection; exit code 1 | x=symbol("x"); solve_full(x^2-2==0,x,integer) | exit 1; keys = ok,code,category,message,recoverable,diagnostics,elapsed (+3 version keys); "code":"InvalidOperation","category":"DomainError","recoverable":true,"message":"solve(): currently supports domains real and complex; got integer.","diagnostics":[{"message":"…","position":0,"line":1,"column":1}],"elapsed":"428.6 µs" — NO elapsedTime, NO timings | FINDING (F4) | P1 |
| P08 | Section 18b status projection | x=symbol("x"); solve_full(x^2-4==0,x).status | S:{"kind":"Enum","type":"SolveStatus","value":"Solved"} | HELD | — |
| P09 | Lines 45–52: type(r.status) is the declared name, not Enum | …; type(solve_full(x^2-4==0,x).status) | S:{"kind":"Text","value":"SolveStatus"} | HELD | — |
| P10 | mathIrVersion value | x=symbol("x"); compile_full(x^2+1,[x]).mathir_version | S:{"kind":"Integer","value":"2","exact":true} | HELD | — |
| P11 | Lines 187–195: wrong arity gives InvalidArgument/TypeMismatch naming the builtin and both counts | x=symbol("x"); compile_full(x) | exit 1; "code":"InvalidArgument","category":"TypeMismatch","recoverable":true,"message":"compile_full(): expected 2 arguments; got 1." — verbatim match to the doc example | HELD (F4 applies to the envelope) | — |
| P12 | Lines 137–140: timings[].position is the 0-based source offset; one entry per statement | 1 + 1; 2 + 2; 3 + 3 | timings=[{position:0,…},{position:7,…},{position:14,…}] — offsets of the three "1"/"2"/"3" | HELD | — |
| P13 | Inv. 1: printed text captured verbatim | print("a"); 5; print("b") | "output":["a\r","b"] | FINDING (F3) | P1 |
| P14 | Value forms: canonical form of x^2 + 1 | x=symbol("x"); x^2 + 1 | S:Sym(x^2 + 1\|(add (rat 1 1) (pow (sym x) (rat 2 1)))), nodeCount=5 — the doc says (add (pow (sym x) (int 2)) (int 1)) | FINDING (F2) | P1 |
| P15 | error path for an undeclared identifier (harness mistake: symbols not declared) | x/y | "code":"InvalidOperation","category":"DomainError","message":"Undefined variable 'x'." | INCONCLUSIVE (probe error; F4 shape applies) | — |
| P16 | README §11: solution values | x=symbol("x"); solve_full(x^2-2==0,x).solutions | Arr[2]/Vector(Solution{value=Sym(-1/2*sqrt(8)\|(mul (rat -1 2) (pow (rat 8 1) (rat 1 2))), domain=real, exact=false, exactness=E(SolutionExactness:AlgebraicExact)}; …+1/2*sqrt(8)…); SymPy: [-sqrt(2), sqrt(2)] — equal | HELD | — |
| P17 | Section 18b: per-solution conditions, no cross-branch union | x=symbol("x"); solve_full((x^2-1)/(x-1)==0,x) | status=Solved, complete=true, completeness=Complete, solutions=Arr[1](Solution{value=Sym(-1\|(rat -1 1)), conditions=Arr[1](Sym(x - 1 != 0\|(ne (add (rat -1 1) (sym x)) (rat 0 1))))}), common_conditions=Arr[1](same) | HELD | — |
| P18 | Value forms: DSP element kind | dft([1,2,3,4]) | Arr[4]/Vector(Cplx(10,0,exact=True); Cplx(-2,2,exact=True); Cplx(-2,0,exact=True); Cplx(-2,-2,exact=True)) | HELD | — |
| P19 | Lines 197–199: symbol(name [, domain]) accepts the short form | symbol("x") | exit 0; S:Sym(x\|(sym x)) | HELD | — |
| P20 | Lines 197–199 boundary: one past the documented limit | symbol("x", real, 3) | exit 1; "code":"InvalidArgument","category":"TypeMismatch","message":"symbol(): expected 1 to 2 arguments; got 3." | HELD (see F14) | — |
| P21 | unknown builtin | no_such_builtin(1) | exit 1; "code":"InvalidOperation","category":"DomainError","message":"Unknown function 'no_such_builtin'." | HELD | — |
| P22 | parse-error position reporting | 1 + | exit 1; "message":"Unexpected token '' at position 3: expected a number, string, identifier, '[', or '('.","diagnostics":[{"message":"…","position":3,"line":1,"column":4}] | HELD | — |
| P23 | LimitResult: DNE with one-sided limits as structure | x=symbol("x"); limit_full(1/x,x,0) | LimitResult{status=E(LimitStatus:DoesNotExist), exists=Bool(false), value=NULL, left=Sym(-inf\|(mul (rat -1 1) (inf))), left_conditions=Arr[1](Sym(x < 0\|…)), right=Sym(inf\|(inf)), right_conditions=Arr[1](Sym(x > 0\|…)), conditions=Arr[0], exactness=E(SolutionExactness:Exact), diagnostics=Arr[0]}; SymPy limit(1/x,'-')=-oo, '+')=oo | HELD | — |
| P24 | IntegrationResult: unevaluated integral stays a node; nested details Diagnostic | x=symbol("x"); integrate_full(exp(-(x^2)),x) | IntegrationResult{status=E(IntegrationStatus:Unevaluated), expression=Sym(integrate(exp(-x^2), x)\|(integ (x) (fn exp (mul (rat -1 1) (pow (sym x) (rat 2 1)))))), verified=Bool(false), method=T(""), verification_method=T(""), exactness=E(SolutionExactness:Approximate), diagnostics=Arr[1](Diagnostic{code=T(integration.unevaluated), …, details=Arr[1](Diagnostic{code=T(integration.no-closed-form), …})})} | HELD | — |
| P25 | TransformResult: diagnostics last, classification an Enum, rule id | x=symbol("x"); simplify_full(x/x) | TransformResult{status=E(TransformStatus:Satisfied), original=Sym(x/x\|…), expression=Sym(1\|(rat 1 1)), changed=Bool(true), conditions=Arr[1](Sym(x != 0\|(ne (sym x) (rat 0 1)))), steps=Arr[1](RewriteStep{rule_id=T(rat.cancel-x-over-x), classification=E(RuleClassification:Conditional), before=…, after=…, required_conditions=Arr[1](…)}), budget_exceeded=Bool(false), budget_kind=T(""), diagnostics=Arr[0]} | HELD | — |
| P26 | README §13: optimization output | x=symbol("x"); optimize(x^5+2*x^4+3*x^3+x^2,[x]) | Sym(x^2*(1 + x*(3 + x*(x + 2)))\|(mul (pow (sym x) (rat 2 1)) (add (rat 1 1) (mul (sym x) …)))), nodeCount=15 | HELD | — |
| P27 | capabilities() trigger pow.non-integer-exponent | 2^(1/2) | exit 1; "code":"UnsupportedOperation","category":"UnsupportedOperation","message":"Non-integer exponents are not yet supported." — matches the advertised code/category | HELD (F4) | — |
| P28 | capabilities() trigger pow.negative-base-unrepresentable-exponent | (-8)^(1/3) | same code/category/message as P27 | HELD (F4) | — |
| P29 | capabilities() trigger diff.non-symbolic-variable | x=symbol("x"); diff(x^2,1) | "code":"InvalidOperation","category":"DomainError","message":"diff(): argument 2 must be a symbolic variable; got Natural." — matches README §6 verbatim | HELD (F4) | — |
| P30 | capabilities() trigger fft.non-power-of-two-length | fft([1,2,3]) | "code":"InvalidArgument","category":"TypeMismatch","message":"FFT length must be a power of two, but got 3. (Parameter 'x')" — matches the advertised message exactly | HELD (F4) | — |
| Q01 | two statements produce two timings | x=symbol("x"); solve_full(x^2-2==0,x) | ntimings=2; S:SolveResult{… status=Solved, complete=true, completeness=Complete, solutions=Arr[2]} | HELD | — |
| Q02 | printed text, three prints | print("a"); print("b"); print("c") | "output":["a\r","b\r","c"] (PS capture; byte-confirmed by Z03) | FINDING (F3) | P1 |
| Q03 | empty script is valid input | (empty file) | exit 0; same keys minus result; ntimings=0; elapsed=14.4 µs | HELD (F10) | — |
| Q04 | --print-budget 5 on a 5-node value | x=symbol("x"); (x+1)^12 + --print-budget 5 | byte-identical to Q05 (no flag): 10010 bytes both | FINDING (F1) | P1 |
| Q05 | same value with no flag (control) | x=symbol("x"); (x+1)^12 | S:Sym((x + 1)^12\|(pow (add (rat 1 1) (sym x)) (rat 12 1))), nodeCount=5, no truncated/budget | HELD | — |
| Q06 | README §11 cubic roots | x=symbol("x"); solve(x^3-1==0,x) | Arr[3]/Vector(Sym(1/2*(i*sqrt(3) - 1)\|…); Sym(1/2*(-i*sqrt(3) - 1)\|…); Sym(1\|(rat 1 1))); SymPy [1, -1/2 - sqrt(3)*I/2, -1/2 + sqrt(3)*I/2] — same set | HELD | — |
| Q07 | same via solve_full | …; solve_full(x^3-1==0,x) | status=Solved, complete=true, completeness=Complete, represented_count=Int(3), unrepresented_count=Int(0), exactness=E(SolutionExactness:AlgebraicExact) | HELD | — |
| Q08 | Lines 142–147 "how many solutions? (solutions.shape)" for a periodic family | x=symbol("x"); solve_full(sin(x)==0,x) | status=Solved, complete=Bool(true), completeness=Complete, solutions=Arr[0]/Vector(), families=Arr[1](SolutionFamily{template=Sym(k*pi\|(mul (sym k) (pi))), parameter=Sym(k), period=Sym(pi), parameter_domain=Domain(integer), exactness=E(SolutionExactness:ParametricExact)}), represented_count=Int(0), unrepresented_count=Int(0), diagnostics=Arr[0] | FINDING (F5) | P1 |
| Q09 | dft(x [, n]) short form accepted | dft([1,2,3,4], 2) | exit 0; Arr[2]/Vector(Cplx(3,0); Cplx(-1,0)) | HELD | — |
| Q10 | dft one past the documented arity | dft([1,2,3,4], 2, 9) | exit 1; "code":"InvalidArgument","category":"TypeMismatch","message":"dft(): expected 1 to 2 arguments; got 3." | HELD (F14) | — |
| Q11 | RewriteStep / RuleClassification form | x=symbol("x"); simplify_full(x/x).steps | Arr[1]/Vector(RewriteStep{rule_id=T(rat.cancel-x-over-x), classification=E(RuleClassification:Conditional), …}) | HELD | — |
| Q12 | type() of a classification field | …; type(simplify_full(x/x).steps[0].classification) | T(RuleClassification) | HELD | — |
| Q13 | LimitResult unevaluated + Null for absent fields | x=symbol("x"); limit_full(sin(x),x,inf) | LimitResult{status=E(LimitStatus:Unevaluated), exists=NULL, value=NULL, left=NULL, left_conditions=Arr[0], right=NULL, right_conditions=Arr[0], conditions=Arr[0], exactness=E(SolutionExactness:Exact), diagnostics=Arr[1](Diagnostic{code=T(limit.unevaluated), category=E(ErrorCategory:UnsupportedOperation), message=T("coefficient does not evaluate at the point"), recoverable=Bool(true), location=NULL, details=Arr[0]})} | HELD | — |
| Q14 | capabilities() integration trigger | x=symbol("x"); integrate_full(exp(x^2),x) | code integration.unevaluated, category UnsupportedOperation, message = the advertised string verbatim | HELD | — |
| Q15 | is there a language-level transform / transform_full? | x=symbol("x"); transform_full(x^2, x) | exit 1; "message":"Unknown function 'transform_full'." | INCONCLUSIVE, which makes lines 80–87 partly NOT-CHECKABLE | — |
| Q16 | identity-reducible equation implies NoSolutions | x=symbol("x"); solve_full(x - x + 1 == 0, x) | status=E(SolveStatus:NoSolutions), complete=Bool(true), completeness=Complete, unrepresented_count=Int(0), diagnostics=Arr[1](code=T(solve.no-solutions), category=E(ErrorCategory:NoSolution), message=T("the equation reduces to a nonzero constant.")); SymPy solve(x-x+1) = [] | HELD | — |
| Q17 | unsupported structure implies Unevaluated | x=symbol("x"); solve_full(exp(x) + x == 0, x) | status=E(SolveStatus:Unevaluated), complete=Bool(false), completeness=E(Completeness:Unknown), solutions=Arr[0], diagnostics=Arr[1](code=T(solve.unevaluated), message=T("No solver for this structure.")) | HELD | — |
| Q18 | Text value form | "abc" | S:{"kind":"Text","value":"abc"} | HELD | — |
| Q19 | Value forms: rank-2 Array | [[1,2],[3,4]] | S:{"kind":"Array","type":"Array","shape":[2,2],"elements":[Natural,Natural,Natural,Natural]} | HELD | — |
| Q20 | exact periodic fraction | 1/3 | S:{"kind":"Real","value":"0.(3)","exact":true,numerator,denominator} | HELD | — |
| Q21 | arbitrary-precision sqrt | sqrt(2) | Real display 1.4142135623730950488016887242096980785696718753769480731766797379907324784621070388503875… | HELD | — |
| Q22 | OptimizationResult diagnostics last | x=symbol("x"); optimize_full(x^5+2*x^4+3*x^3+x^2,[x]) | OptimizationResult{original=…, optimized=…, estimated_cost_before=Int(17), estimated_cost_after=Int(15), shared_subtrees=Int(5), horner_rewrites=Int(1), transformations=Arr[2]/Vector(T(cse); T(horner)), target=T(mathir), policy=T(cse+horner+precision-aware), diagnostics=Arr[0]/Vector()} | HELD (see observation C5) | — |
| Q23 | SystemSolveResult diagnostics last | x=symbol("x"); y=symbol("y"); solve_system_full([x^2+y^2-1==0,x*y==0],[x,y]) | SystemSolveResult{status=Solved, domain=complex, complete=true, solutions=Arr[4]/Vector(SystemSolution{bindings=Arr[2](Binding{name=Sym(x),value=Sym(0)}, Binding{name=Sym(y),value=Sym(-1)}), conditions=Arr[0], exactness=Exact}; …3 more), diagnostics=Arr[0]}; SymPy [{x:-1,y:0},{x:0,y:-1},{x:0,y:1},{x:1,y:0}] — same set | HELD | — |
| Q24 | README §10b: contradictory assumptions rejected | x=symbol("x"); assume(x > 0); assume(x <= 0) | exit 1; "code":"UnsatisfiableAssumptions","category":"DomainError","recoverable":true,"message":"Assumption x <= 0 contradicts the existing assumptions (its negation x > 0 is provable)." | HELD (F4) | — |
| Q25 | completeness projection | …; solve_full(x^2-4==0,x).completeness | E(Completeness:Complete) | HELD | — |
| Q26 | complete projection | …; solve_full(x^2-4==0,x).complete | Bool(true) | HELD | — |
| Q27 | README §17: unbound symbol is an error | x=symbol("x"); evalf(x, 20) | exit 1; "code":"EvaluationError","category":"DomainError","message":"No value for symbol 'x'." | HELD (F4) | — |
| Q28 | capabilities() plot trigger 1 | x=symbol("x"); plot(sin(x)) | "code":"InvalidOperation","category":"DomainError","message":"plot() argument 1 must be a vector, but got 'Symbolic'." | HELD (F4) | — |
| Q29 | capabilities() plot trigger 2 | x=symbol("x"); plot([x, 1, 2]) | … "Cannot convert value of kind 'Symbolic' to a number for plotting." | HELD (F4) | — |
| Q30 | capabilities() dsp trigger | x=symbol("x"); dft([x, 1, 2, 3]) | … "DSP builtins expect numeric/complex elements, but got 'SymbolExpr'." | HELD (F4) | — |
| Q31 | capabilities() linsolve trigger | A=[[1,2],[2,4]]; b=[[1],[3]]; linsolve(A,b) | … "linsolve() requires a symbolic matrix A." | HELD (F4) | — |
| Q32 | MatrixSolveResult diagnostics last | x=symbol("x"); linsolve_full([[x,1],[0,x]],[0,1]) | MatrixSolveResult{status=Solved, complete=true, completeness=Complete, solutions=Arr[2](Sym(-x^-2\|(mul (rat -1 1) (pow (sym x) (rat -2 1)))); Sym(x^-1\|(pow (sym x) (rat -1 1)))), conditions=Arr[1](Sym(x^2 != 0\|…)), diagnostics=Arr[0]} | HELD | — |
| Q33 | capabilities() limit trigger | x=symbol("x"); limit(sin(x), 1, 0) | … "limit(): argument 2 must be a symbolic variable; got Natural." | HELD (F4) | — |
| Q34 | function declaration syntax (harness mistake) | x=symbol("x"); fs = func f(a) { a + 1 }; f(2) | parse error "Expected ';' or end of input but found 'f' at position 27." | INCONCLUSIVE (re-run as N06) | — |
| Q35 | --print-budget 200 on a 5-node value | x=symbol("x"); (x+1)^12 + --print-budget 200 | 10010 bytes, identical to no-flag | FINDING (F1) | P1 |
| U01 | exit code 2 usage error, no script | Lovelace.Run.exe (no args) | exit 2; stdout EMPTY (0 bytes); help text on stderr only | FINDING (F7) | P2 |
| U02 | exit code 2 usage error, unknown flag | --bogus | exit 2; stdout EMPTY; "Error: Unknown argument '--bogus'." plus help on stderr | FINDING (F7) | P2 |
| U03 | unreadable script file | --file C:\nope\missing.ls | exit 1; "code":"FileReadError","category":"ParseError","recoverable":true,"message":"Cannot read script file 'C:\nope\missing.ls': Could not find a part of the path 'C:\nope\missing.ls'.","diagnostics":[] | FINDING (F4, F9) | P2 |

### 1.2 --print-budget (R, B, E, F, S, T, N, V groups)

All rows use the script x = symbol("x") unless shown. "truncProp" = the structured value contains a truncated key at all.

| id | what was probed | command (script / args) | raw output (abridged) | verdict | sev |
|---|---|---|---|---|---|
| R01 | baseline, no flag | (x+1)^12 | 10010 B; Sym((x + 1)^12\|(pow (add (rat 1 1) (sym x)) (rat 12 1))), nc=5 | HELD | — |
| R02 | budget 1 on a 5-node value | (x+1)^12 + --print-budget 1 | identical to R01 ignoring timings (same length 10010); NO truncated | FINDING (F1) | P1 |
| R03 | baseline, no flag, big value | expand((x+1)^40) | 15169 B; full pretty/canonical | HELD | — |
| R04 | budget 1 on a 198-node value | expand((x+1)^40) + --print-budget 1 | 12609 B; canonical len 48; truncated=true, truncationReason=node-budget, budget=1, nodeCount=198; pretty = "x^40 + 40*x^39 + 780*x^38 + 9880*x^37 + 91390*x^ …" | HELD | — |
| R05 | budget 0 | --print-budget 0 | exit 2; stdout empty | FINDING (F7) | P2 |
| R06 | negative budget | --print-budget -1 | exit 2; stdout empty | FINDING (F7) | P2 |
| R07 | non-numeric budget | --print-budget abc | exit 2; stdout empty | FINDING (F7) | P2 |
| R08 | budget 1 on an Array value | 1..50 + --print-budget 1 | Arr[50] with all 50 elements serialized (2242 chars); keys kind,type,shape,elements only — no truncated and no budget anywhere | FINDING (F1) | P1 |
| R09 | same array, no flag | 1..50 | 12511 B vs 12515 B — same shape, no truncation | HELD | — |
| R10 | family result (F5 control) | solve_full(sin(x) == 0, x) | see Q08 | FINDING (F5) | P1 |
| B0 | budget 0 | (x+1)^12 + --print-budget 0 | exit 2; empty stdout | FINDING (F7) | P2 |
| B1–B8 | budget 1…8 on the same 5-node value | (x+1)^12 + --print-budget {1..8} | every run: nodeCount=5, hasTruncProp=False, canonLen=40; full canonical (pow (add (rat 1 1) (sym x)) (rat 12 1)) | FINDING (F1) x8 | P1 |
| B97 | no flag (control) | x^2 + 1 | nc=5, canonLen 39 | HELD | — |
| B98 | budget 1 with nc=5 | x^2 + 1 + --print-budget 1 | nc=5 > budget 1, yet NO truncated, canonLen 39 | FINDING (F1) | P1 |
| B99 | no flag (control) | (x+1)^12 | nc=5, canonLen 40 | HELD | — |
| E02–E12 | budget 1 on expanded powers k=2…12 | expand((x+1)^k) + --print-budget 1 | all: truncated=true, truncationReason=node-budget, budget=1; canonical cut at 47–48 chars ending " …"; nc = 8,13,18,23,28,33,38,43,48,53,58 | HELD x11 | — |
| F01 | budget far above nodeCount | expand((x+1)^3) + --print-budget 1000 | nc=13, no truncProp, canonLen 103 (full) | HELD | — |
| F02 | budget just below nodeCount | same + --print-budget 8 | truncated=true, reason=node-budget, budget=8, canonLen 47 | HELD | — |
| F03 | budget equal to nodeCount | same + --print-budget 13 | no truncProp, canonLen 103 | HELD | — |
| F04 | budget 1000000 | same | no truncProp, canonLen 103 | HELD | — |
| F05 | no flag | same | canonLen 103 | HELD | — |
| F06 | budget 1, nc=6 | x^2 + x + 1 + --print-budget 1 | nc=6 > 1, NO truncProp, canonLen 47 (full) | FINDING (F1) | P1 |
| F07 | budget 6, nc=6 | same + --print-budget 6 | no truncProp (6 > 6 is false — consistent) | HELD | — |
| F08 | no flag | same | no truncProp | HELD | — |
| F09 | budget 1000 on a 21-node series | series(sin(x)/x,x,0,8) + --print-budget 1000 | no truncProp, canonLen 178 | HELD | — |
| F10 | no flag | same | canonLen 178 | HELD | — |
| S01–S08 | budget 1…8, nc=13 | expand((x+1)^3) + --print-budget {1..8} | all truncated=true with budget=N; canonLen 47 | HELD x8 | — |
| S09–S12 | budget 9…12, nc=13 | same | all truncate; prefix grows: canonLen 56, 61, 66, 71 | HELD x4 | — |
| S13–S15 | budget 13…15, nc=13 | same | no truncProp; full canonLen 103 | HELD x3 | — |
| T01–T06 | budget 1…6, nc=5 | (x+1)^12 + --print-budget {1..6} | every run hasTruncProp=False, hasBudgetProp=False, canonLen=40 | FINDING (F1) x6 | P1 |
| N01 | budget 1, nc=7 | x^2 + 2*x + --print-budget 1 | truncated=true, budget=1, canonLen 47 | HELD | — |
| N02 | budget 1, nc=6, long symbol name | w=symbol("wwwwwwww"); w^2 + w + 1 + --print-budget 1 | truncated=true, budget=1, canonLen 42 | HELD | — |
| N03 | no flag, same | same | no truncProp, canonLen 61 | HELD | — |
| N04 | no flag | x^2 + 2*x | no truncProp, canonLen 53 | HELD | — |
| N05 | per-statement kinds/offsets incl. assignment and print | x=symbol("x"); a = 5; print("p"); x^2 | timings=[0:Symbolic:false 17:Natural:false 24:Void:true 36:Symbolic:false] | HELD | — |
| N06 | func declaration produces Void | func f(a) { a + 1 }; f(2) | timings=[0:Void:false 21:Natural:false]; result.kind=Natural, display 3 | HELD | — |
| N07 | elapsed string vs elapsedTime | x^2 + 1 | elapsed="691.4 µs", elapsedTime={value:691.4,unit:"µs"} | HELD | — |
| N08 | elapsed on a long run | expand((x+1)^120) | elapsed="102.39 ms", elapsedTime={value:102.39,unit:"ms"}; timings[].elapsed.unit also ms | HELD | — |
| V01 | CLEAN-ROOM RE-RUN of F1 | (x+1)^12 + --print-budget 1 | truncProp=False trunc= budget= nc=5 canon=[(pow (add (rat 1 1) (sym x)) (rat 12 1))] | FINDING (F1) | P1 |
| V02 | CLEAN-ROOM RE-RUN of F2 | x^2 + 1 | canon=[(add (rat 1 1) (pow (sym x) (rat 2 1)))] | FINDING (F2) | P1 |
| V03 | CLEAN-ROOM RE-RUN of F3 | print("a"); print("b") | raw "output":["a\r","b"] | FINDING (F3) | P1 |
| V04 | CLEAN-ROOM RE-RUN of F4 | solve(x^2-2==0,x,integer) | exit=1 keys=[protocolVersion,symbolicFormatVersion,mathIrVersion,ok,code,category,message,recoverable,diagnostics,elapsed] hasET=False hasTim=False elapsed=[486.2 µs] | FINDING (F4) | P1 |
| V05 | CLEAN-ROOM RE-RUN of F5 | solve_full(sin(x)==0,x) | kind=Record disp=[SolveResult(status: Solved, …, complete: True, completeness: Complete, solutions: [], families: [SolutionFamily(template: k*pi, …, parameter_domain: integer, …)], common_conditions: [], represented_count: 0, unrepresented_count: 0, unrepresented_reason: , diagnostics: [])] | FINDING (F5) | P1 |
| V06 | CLEAN-ROOM RE-RUN of F6 | assume(x > 5); assumptions() | kind=Text disp=[x > 5]; raw "result":{"kind":"Text","display":"x \u003E 5","typed":"x \u003E 5","structured":{"kind":… | FINDING (F6) | P1 |
| V07 | CLEAN-ROOM control for F1 | expand((x+1)^3) + --print-budget 8 | truncProp=True trunc=True budget=8 nc=13 canon=[(add (rat 1 1) (pow (sym x) (rat 3 1)) (mul ( …] | HELD | — |
| V08 | control | solve_full(sin(x)==0,x).status | kind=Enum disp=[Solved] | HELD | — |

### 1.3 Lovelace.Symbolics/README.md and root README.md (H, M groups)

| id | what was probed (README claim) | command (script) | raw output (abridged) | verdict | sev |
|---|---|---|---|---|---|
| H01 | §11 quadratic solution set | x=symbol("x"); solve(x^2 - 4 == 0, x) | Arr[2]/Vector(Sym(-2\|(rat -2 1)); Sym(2\|(rat 2 1))); SymPy ±2 | HELD | — |
| H02 | §11 cubic closed form (round-03 note) | x=symbol("x"); solve(x^3 - 1 == 0, x) | Arr[3] with i*sqrt(3) written out; matches the SymPy root set | HELD | — |
| H03 | §17 evalf(1/3, 25) | evalf(1/3, 25) | display 0.3333333333333333333333333 — 25 fraction digits, exact match | HELD | — |
| H04 | §12 det([[x,1],[y,x]]) | det([[x, 1], [y, x]]) | Sym(x^2 - y\|(add (pow (sym x) (rat 2 1)) (mul (rat -1 1) (sym y)))); SymPy x**2 - y | HELD | — |
| H05 | §12 inv keeps 0/(x*y) | inv([[x, 1], [0, y]]) | Arr[2x2](Sym(y/(x*y)\|…); Sym(-1/(x*y)\|…); Sym(0/(x*y)\|…); Sym(x/(x*y)\|…)); SymPy [[1/x, -1/(x*y)], [0, 1/y]] — equal off xy=0, as documented | HELD | — |
| H06 | §18b rule_id | x=symbol("x"); simplify_full(x/x).steps[0].rule_id | T(rat.cancel-x-over-x) | HELD | — |
| H07 | Lines 43–52: classification is an Enum | …; simplify_full(x/x).steps[0].classification | E(RuleClassification:Conditional) | HELD | — |
| H08 | type() of that enum | …; type(simplify_full(x/x).steps[0].classification) | T(RuleClassification) | HELD | — |
| H09 | §18b limit_full(1/x,x,0).exists | x=symbol("x"); limit_full(1/x, x, 0).exists | Bool(false) | HELD | — |
| H10 | §18b integrate_full(x^2,x).status | x=symbol("x"); integrate_full(x^2, x).status | E(IntegrationStatus:SolvedExact) | HELD | — |
| H11 | §18b compile_full(...).parameters | x=symbol("x"); compile_full(x^2 + 1, [x]).parameters | Arr[1]/Vector(ParameterInfo{name=T(x), domain=Domain(complex)}) | HELD | — |
| H12 | §3 simplify(sqrt(y^2)) left unevaluated without assumptions | y=symbol("y"); simplify(sqrt(y^2)) | Sym(sqrt(x^2)\|(pow (pow (sym x) (rat 2 1)) (rat 1 2))), exact=false | HELD | — |
| H13 | §3 with assume_positive | x=symbol("x"); assume_positive(x); simplify(sqrt(x^2)) | Sym(x\|(sym x)) | HELD | — |
| H14 | §2 simplify(x/x) under x != 0 | x=symbol("x"); assume(x != 0); simplify(x/x) | Sym(1\|(rat 1 1)) | HELD | — |
| H15 | §3 assumptions() returns x > 5 | x=symbol("x"); assume(x > 5); assumptions() | S:{"kind":"Text","value":"x > 5"} — text only | FINDING (F6) | P1 |
| H16 | §11 solve(exp(x)==5,x) | x=symbol("x"); solve(exp(x) == 5, x) | Arr[1]/Vector(Sym(log(5)\|(fn log (rat 5 1)))); SymPy [log(5)] | HELD | — |
| H17 | §11 inconsistent equation message | x=symbol("x"); solve(1/x == 0, x) | T("the equation reduces to a nonzero constant."); SymPy: no solution | HELD (text-only projection, but solve_full gives structure) | — |
| H18 | double root multiplicity | x=symbol("x"); solve_full(x^2 == 0, x) | status=Solved, complete=true, solutions=Arr[1](Solution{value=Sym(0), multiplicity=Int(2), exactness=Exact}) | HELD | — |
| H19 | complex default domain | x=symbol("x"); solve_full(x^2 + 1 == 0, x) | domain=Domain(complex), status=Solved, solutions=Arr[2](Sym(i\|(i)); Sym(-i\|(mul (rat -1 1) (i)))); SymPy [-I, I] | HELD | — |
| H20 | §11 solve_system projection | x=symbol("x"); y=symbol("y"); solve_system([x^2 + y^2 - 1 == 0, x*y == 0], [x, y]) | T("x = 0, y = -1; x = -1, y = 0; x = 1, y = 0; x = 0, y = 1") — same set as SymPy | HELD | — |
| H21 | §18b unrepresented_count | x=symbol("x"); solve_full(x^4 - x^2 - 1 == 0, x).unrepresented_count | Int(2); SymPy: 4 roots, 2 real | HELD | — |
| H22 | §17 setprecision(20); evalf(sqrt(2),20) | x=symbol("x"); setprecision(20); evalf(sqrt(2), 20) | 1.4142135623730950488 (19 digits) — exact match to the README | HELD | — |
| H23 | §8 series | x=symbol("x"); series(sin(x)/x, x, 0, 8) | Sym(1 - 1/6*x^2 + 1/120*x^4 - 1/5040*x^6 + O(x^8)\|…); SymPy identical | HELD | — |
| H24 | §9 limit DNE prose | x=symbol("x"); limit(1/x, x, 0) | T("does not exist (left: -inf, right: +inf)"); SymPy -oo / oo | HELD | — |
| H25 | root README 2^100 | 2^100 | Nat(1267650600228229401496703205376); SymPy identical | HELD | — |
| H26 | §13 Hornerization | x=symbol("x"); optimize(x^5 + 2*x^4 + 3*x^3 + x^2, [x]) | Sym(x^2*(1 + x*(3 + x*(x + 2)))) | HELD | — |
| H27 | §13 evalir(lower(k,[x]),[2],40) | x=symbol("x"); k = optimize(x^5 + 2*x^4 + 3*x^3 + x^2, [x]); evalir(lower(k, [x]), [2], 40) | Int(92); SymPy: the polynomial at x=2 is 92 | HELD | — |
| H28 | §16 evalir_batch | x=symbol("x"); evalir_batch(compile(x^2+1,[x]), [1, 2, 3], 40) | Arr[3]/Vector(Int(2); Int(5); Int(10)); SymPy 2,5,10 | HELD | — |
| H29 | §18b common_conditions | x=symbol("x"); solve_full((x^2 - 1)/(x - 1) == 0, x).common_conditions | Arr[1]/Vector(Sym(x - 1 != 0\|(ne (add (rat -1 1) (sym x)) (rat 0 1)))) | HELD | — |
| H30 | §18b .domain | x=symbol("x"); solve_full(x^4 - x^2 - 1 == 0, x).domain | Domain(complex) | HELD | — |
| M01 | capabilities() solve.non-symbolic-expression trigger | x=symbol("x"); solve([x == 1], x) | "code":"InvalidOperation","category":"DomainError","message":"solve(): argument 1 must be a symbolic expression; got Vector." | HELD (F4) | — |
| M02 | solve.non-symbolic-variable trigger | x=symbol("x"); solve(x^2 - 2 == 0, 1) | … "solve(): argument 2 must be a symbolic variable; got Natural." | HELD (F4) | — |
| M03 | integrate.non-symbolic-variable trigger | x=symbol("x"); integrate(x^2, 1) | … "integrate(): argument 2 must be a symbolic variable; got Natural." | HELD (F4) | — |
| M04 | unsupported_domains includes rational | x=symbol("x"); solve(x^2 - 2 == 0, x, rational) | … "solve(): currently supports domains real and complex; got rational." | HELD (F4) | — |
| M05 | --cancel-after on a heavy script | x=symbol("x"); expand((x+1)^5000) + --cancel-after 1 | exit 1; "code":"Cancelled","category":"BudgetExceeded","recoverable":true,"message":"the evaluation was cancelled by the caller.","diagnostics":[] | HELD (F4; --cancel-after is outside dsh-protocol.md) | — |
| M10 | root README det(m) | m = [[1, 2], [3, 4]]; det(m) | Int(-2); SymPy -2 | HELD | — |
| M11 | root README inv(m) exact | m = [[1, 2], [3, 4]]; inv(m) | Arr[2x2], display [[-2, 1], [1.5, -0.5]]; SymPy [[-2, 1], [3/2, -1/2]] | HELD | — |
| M12 | root README matmul(m,m) | m = [[1, 2], [3, 4]]; matmul(m, m) | [[7, 10], [15, 22]]; SymPy identical | HELD | — |
| M13 | root README sum(v^2) | v = 1..5; sum(v ^ 2) | Nat(55) | HELD | — |
| M14 | root README v * 10 broadcast | v = 1..5; v * 10 | Arr[5]/Vector(10,20,30,40,50) | HELD | — |
| M15 | root README fib(20) | func fib(n) { if (n < 2) { n } else { fib(n - 1) + fib(n - 2) } }; fib(20) | Nat(6765); SymPy fibonacci(20)=6765 | HELD | — |
| M16 | README §20 partial status | x=symbol("x"); solve_full(x^4 - x^2 - 1 == 0, x).status | E(SolveStatus:Partial) | HELD | — |
| M17 | README §20 completeness | x=symbol("x"); solve_full(x^4 - x^2 - 1 == 0, x).completeness | E(Completeness:Partial) | HELD | — |

### 1.4 Value encodings, registry, determinism (K, Z groups)

| id | what was probed | command | raw output (abridged) | verdict | sev |
|---|---|---|---|---|---|
| K01 | Line 31 canonical atom encoding | x^2 + 1 | "structured":{"kind":"Symbolic","pretty":"x^2 \u002B 1","canonical":"(add (rat 1 1) (pow (sym x) (rat 2 1)))","domain":"complex","exact":true,"nodeCount":5,"freeSymbols":["x"]} | FINDING (F2) | P1 |
| K02 | Line 35: value is a JSON string, exact a JSON boolean | 42 | "result":{"kind":"Natural","display":"42","typed":"42 (Natural)","structured":{"kind":"Natural","value":"42","exact":true}} | HELD | — |
| K03 | Line 37 Boolean literal true (harness assumption: not a literal) | true | exit 1; "message":"Undefined variable 'true'." | INCONCLUSIVE (form verified via .complete / .exists) | — |
| K04 | Array shape/type | [[1,2],[3,4]] | see Q19 | HELD | — |
| K05 | Line 34 Complex literal i (harness assumption) | i | exit 1; "message":"Undefined variable 'i'." | INCONCLUSIVE (form verified via H19, dft) | — |
| K06 | 2 + 3i literal syntax | 2 + 3i | exit 1; "message":"Expected ';' or end of input but found 'i' at position 5." | INCONCLUSIVE | — |
| K07 | Inv. 1 verbatim capture, three prints | print("a"); print("b"); print("c") | "output":["a\r","b\r","c"],"variables":[] (byte-confirmed by Z03) | FINDING (F3) | P1 |
| K08 | §11 periodic inverse projection | x=symbol("x"); solve(sin(x) == 0, x) | "structured":{"kind":"Text","value":"k*pi for integer k"} | HELD | — |
| K09 | periodic family via _full | x=symbol("x"); solve_full(cos(x) == 0, x) | SolveResult{status=Solved, complete=true, solutions=Arr[0], families=Arr[1](template with period pi …)} | HELD | — |
| K10 | assumptions() with no assumptions | x=symbol("x"); assumptions() | "structured":{"kind":"Text","value":""} | FINDING (F6) | P1 |
| K11 | type() of a Record | x=symbol("x"); type(solve_full(x^2-4==0,x)) | S:{"kind":"Text","value":"SolveResult"} | HELD | — |
| K12 | solve_system with an undeclared symbol | x=symbol("x"); solve_system([x^2+y^2-1==0,x*y==0],[x,y]) | exit 1; "message":"Undefined variable 'y'." | INCONCLUSIVE (re-run as H20) | — |
| K13 | no-budget rendering of a 998-node value | x=symbol("x"); expand((x+1)^200) | 71562 B; nodeCount=998; full pretty/canonical; no truncated | HELD | — |
| K14 | --cancel-after 1 on a fast script | x=symbol("x"); expand((x+1)^3) + --cancel-after 1 | exit 0; normal envelope (revision 82) | HELD | — |
| K15 | --cancel-after 100000 | same | exit 0; normal envelope | HELD | — |
| K16 | determinism partner of K01 | x=symbol("x"); x^2 + 1 | first raw byte difference vs K01 at index 9725 — inside "elapsed" only | FINDING (F8) | P2 |
| K17 | --omit-functions honoured | capabilities() + --omit-functions | 21961 B, "functions":[] | HELD | — |
| K18 | builtin registry content | capabilities() | 123 functions; property sets exactly name,parameters,builtin[,plugin]; symbol -> parameters=[name,domain], solve -> [f,x,domain], dft -> [x,n], limit_full -> [f,x,x0]; NO MinArity/Variadic exported; exactness=E(CapabilitiesExactness:BestEffort) | FINDING (F11, F14) | P3 / P2 |
| K19 | --omit-variables honoured | x=symbol("x"); x^2 + 1 + --omit-variables | envelope lacks variables | HELD | — |
| K20 | hunting truncationReason depth-limit | deeply nested 21-node expression + --print-budget 100000 | no truncated at all | INCONCLUSIVE (see C2) | — |
| Z01 | exact CI scenario 4 (ci.yml:298–310), byte-exact capture | print("hello from the script") NEWLINE 1 + 1 | "output":["hello from the script"], "result":{"kind":"Natural","display":"2",…,"value":"2"} | HELD | — |
| Z02 | exact CI scenario 3 (ci.yml:279–296) | x=symbol("x"); y=symbol("y"); jacobian([x*y, x+y], [x, y]) | kind=Array shape=2,2 nElem=4 e0kind=Symbolic e0canonical=[(sym y)] — all four CI assertions hold | HELD | — |
| Z03 | byte-level capture of three prints | print("a"); print("b"); print("c") | bytes at "output": 22 6f 75 74 70 75 74 22 3a 5b 22 61 5c 72 22 2c 22 62 5c 72 22 2c 22 63 22 5d = "output":["a\r","b\r","c"]; element lengths 2,2,1 | FINDING (F3) | P1 |
| Z04 | byte-level capture, print/5/print | print("a"); 5; print("b") | output=[<a> len=2 <b> len=1] | FINDING (F3) | P1 |
| Z05–Z07 | same solve script run three times | x=symbol("x") NEWLINE solve_full(x^2 - 4 == 0, x) | revision=82 all three; elapsed 972.6 µs / 648.5 µs / 595 µs; raw envelopes differ; STRUCTURALLY IDENTICAL once elapsed/elapsedTime/timings values are removed | FINDING (F8) x3 | P2 |
| Z08–Z09 | capabilities() run twice | capabilities() | revision=81 both; raw differs (timings); structurally identical | FINDING (F8) x2 | P2 |

### 1.5 Boundary attack (X group)

| id | what was probed | command | raw output (abridged) | verdict | sev |
|---|---|---|---|---|---|
| X01 | zero arguments (one below the documented minimum) | symbol() | exit 1; "code":"InvalidArgument","category":"TypeMismatch","message":"symbol(): expected 1 to 2 arguments; got 0." | HELD (F14) | — |
| X02 | empty symbol name | symbol("") | exit 0; S:Sym(\|(sym )) — display empty, canonical "(sym )" | FINDING (F12) | P3 |
| X03 | precision 0 | evalf(1/3, 0) | Real, display 0 | HELD (undocumented boundary) | — |
| X04 | precision 1000 | evalf(1/3, 1000) | Real, display length 1002 (1000 digits) | HELD | — |
| X05 | dft with n=1 (smallest legal n) | dft([1,2,3,4], 1) | exit 0; Arr[1], display [1] | HELD | — |
| X06 | empty vector input | dft([]) | exit 0; Arr[0](), display [] | HELD | — |
| X07 | CompilationResult diagnostics last | x=symbol("x"); compile_full(x^2 + 1, [x]) | CompilationResult{ir=T(#!mathir 2 …), parameters=Arr[1](ParameterInfo{name=T(x), domain=Domain(complex)}), result_domain=Domain(complex), result_type=T(Integer), precision_policy=T(caller_supplied), optimization_policy=T(none), target=T(mathir), mathir_version=Int(2), exact=Bool(true), diagnostics=Arr[…]} | FINDING (F13) | P3 |
| X08 | real-domain solve | x=symbol("x"); solve(x^2 - 4 == 0, x, real) | Arr[2]/Vector(Sym(-2); Sym(2)) | HELD | — |
| X09 | member access on a Vector result (harness expectation error) | x=symbol("x"); solve_full(x^2 - 4 == 0, x).solutions.shape | exit 1; "message":"member 'shape' is not available on type 'Vector': member access requires a record result." | INCONCLUSIVE (envelope shape verified in P03) | — |
| X10 | 0^0 | 0^0 | Nat(1) | HELD | — |
| X11 | identity equation, convenience form | x=symbol("x"); solve(x == x, x) | T("unevaluated: 0 = 0: every value is a solution.") | HELD | — |
| X12 | identity equation, structured form | x=symbol("x"); solve_full(x == x, x) | status=E(SolveStatus:Unevaluated), complete=Bool(false), completeness=E(Completeness:Unknown), solutions=Arr[0], families=Arr[0], represented_count=Int(0), unrepresented_count=Int(0), diagnostics=Arr[1](code=T(solve.unevaluated)) | HELD | — |
| X13 | solve_full(0 == 0, x) (harness expectation error: the argument folds to Boolean) | x=symbol("x"); solve_full(0 == 0, x) | exit 1; "message":"solve_full(): argument 1 must be a symbolic expression; got Boolean." | INCONCLUSIVE (covered by X12) | — |
| X14 | solutions element shape under _full | x=symbol("x"); solve_full(x^2 - 4 == 0, x).solutions | Arr[2](Solution{value=…, conditions=Arr[0], multiplicity=Int(1), exactness=E(SolutionExactness:Exact)}; …) | HELD | — |

### 1.6 CI section claims (.github/workflows/ci.yml, read and replicated)

| id | claim probed | evidence | verdict |
|---|---|---|---|
| C1 | dsh-protocol.md:206–210: CI "publishes the runner as a Native AOT binary and then executes it against the scenarios above, asserting the envelope's structure" | ci.yml job aot-smoke (lines 169–327) runs dotnet publish -p:PublishAot=true -o out/aot, uploads out/aot/Lovelace.Run with if-no-files-found: error, then executes the published binary five times asserting ok, protocolVersion, symbolicFormatVersion, mathIrVersion, status.kind/type/value, completeness kind/type/value, complete as the STRING "true"/"false", domain, solutions.shape, per-solution canonical/exactness, array shape, stdout purity, and an error envelope's code/category | HELD |
| C2 | The CI scenarios reproduced on this binary | Z01, Z02 plus the solve/partial/err scenarios: status.kind=="Enum", complete.value=="true", unrepresented_count==2, "got integer" in the message, non-zero exit for the integer domain | HELD |
| C3 | Would the CI gate catch the output-array defect of F3? | CI's print scenario (Z01) has exactly one print, which is also the LAST writer — the only case in which the CR is absent (Z03). The gate cannot see F3 | FINDING-adjacent (supports F3) |

### 1.7 Independent ground truth (W group)

| id | what was probed | raw output | verdict |
|---|---|---|---|
| W01 | SymPy 1.14.0 cross-check of every mathematical result quoted above | sympy 1.14.0; x^4-x^2-1 real roots: [CRootOf(...,0), CRootOf(...,1)] n_real=2, all roots [-I*sqrt(-1/2+sqrt(5)/2), I*…, -sqrt(1/2+sqrt(5)/2), sqrt(1/2+sqrt(5)/2)]; x^4+1 real roots: [] n_complex=4; x^2-2 roots: [-sqrt(2), sqrt(2)]; x^3-1 roots: [1, -1/2 - sqrt(3)*I/2, -1/2 + sqrt(3)*I/2]; det: x**2 - y; inv: [[1/x, -1/(x*y)], [0, 1/y]]; sin(x)=0: [0, pi]; d2/dx2 x^4: 12*x**2; integrate 2x cos(x^2): sin(x**2); limit 1/x L/R: -oo oo; solve x-x+1: []; jacobian: [[y, x], [1, 1]]; 2^100: 1267650600228229401496703205376; sum 1..5 squared: 55; fib(20): 6765; matmul m^2: [[7, 10], [15, 22]]; det: -2 inv: [[-2, 1], [3/2, -1/2]]; series sin(x)/x: 1 - x**2/6 + x**4/120 - x**6/5040 + O(x**8); solve_system: [{x:-1,y:0},{x:0,y:-1},{x:0,y:1},{x:1,y:0}]; integrate 1/(x^2-1): log(x-1)/2 - log(x+1)/2; apart: -1/(2*(x+1)) + 1/(2*(x-1)); poly at x=2: 92; factor: (x-2)*(x-1)*(x+1)*(x+2); solve exp(x)=5: [log(5)]; limit (1-cos x)/x^2: 1/2 | HELD — no mathematical disagreement found in 40+ cross-checks |

## 2. FINDINGS

### A4-F1 — P1 — --print-budget does not bound a rendering whose node count already exceeds the budget
> dsh-protocol.md:17–20 — "--print-budget <nodes> bounds structured renderings: beyond it, pretty/canonical carry a " …"-terminated prefix of the real rendering (never a re-ordered one) and the value reports truncated: true, truncationReason (node-budget/depth-limit) and budget."

**Reproduction 1** (fresh temp file, byte-exact capture):

    %TEMP%\a4clean2\V01.ls : x = symbol("x"); (x+1)^12
    out\aot\Lovelace.Run.exe --file V01.ls --print-budget 1
    -> "structured":{"kind":"Symbolic","pretty":"(x + 1)^12",
        "canonical":"(pow (add (rat 1 1) (sym x)) (rat 12 1))",
        "domain":"complex","exact":true,"nodeCount":5,"freeSymbols":["x"]}

nodeCount = 5 > budget = 1, yet truncated, truncationReason and budget are ABSENT and the full canonical form is returned.
**Reproduction 2** (independent clean room, %TEMP%\a4clean): same script, --print-budget 1 -> truncProp=False trunc= budget= nc=5 canon=[(pow (add (rat 1 1) (sym x)) (rat 12 1))]. Reproduced identically at budgets 2,3,4,5,6 (T01–T06), for x^2 + 1 (B98, V02) and for x^2 + x + 1 (F06, nc=6).

**Control showing the mechanism is not simply off:** expand((x+1)^3) (nc=13) with --print-budget 8 -> truncated:true, truncationReason "node-budget", budget 8, canonical cut to 47 chars (V07, F02, S01–S12); with --print-budget 13 -> NO truncation (F03, S13–S15). So the switch is nodeCount > budget for nc>=7–8, but is additionally gated by rendering length: at nc=5–6 the full rendering (40–47 chars) comes back unmarked, while at nc=6 with a longer rendering (63 chars, N02) truncation IS applied. The measured gate looks like "full rendering longer than about 48 characters AND nodeCount > budget" (?). Either way the documented unconditional "beyond it … reports truncated: true … and budget" does not describe the binary.

**Same rule, second surface:** 1..50 with --print-budget 1 returns all 50 elements with no truncated/budget key anywhere (R08) — Arrays are not budgeted at all.
**Parts that do hold:** the truncated prefix is a true prefix of the untruncated rendering ("never a re-ordered one" verified with StartsWith, R03 vs R04); the reason value used is node-budget; the budget field equals the flag value when truncation occurs.

### A4-F2 — P1 — the documented canonical atom for an integer is stale; the binary writes rationals
> dsh-protocol.md:31 — { "kind": "Symbolic", "pretty": "x^2 + 1", "canonical": "(add (pow (sym x) (int 2)) (int 1))", … }

**Reproduction 1:** x = symbol("x"); x^2 + 1 -> "canonical":"(add (rat 1 1) (pow (sym x) (rat 2 1)))" (K01).
**Reproduction 2** (clean room): V02 -> canon=(add (rat 1 1) (pow (sym x) (rat 2 1))). Third occurrence: P14.

Two independent deltas: the atom spelling ((rat 1 1) vs (int 1)) and the term order (constant first). The document contradicts ITSELF — dsh-protocol.md:119 shows a solution value as "(rat -2 1)", which the binary does produce (P03) — so line 31 is the stale one. An agent that implemented a canonical reader for the documented (int n) atom cannot read any integer in the real format.

### A4-F3 — P1 — output[] entries carry a trailing CR on Windows for every printed line except the last
> dsh-protocol.md:9–10 — "Anything the script prints with print(...) is captured and returned in the top-level output array."
> README.md:858–863 — "The same script, assumptions, and seed produce byte-identical output on every run and platform".

**Reproduction 1** (byte-exact capture, Z03), script print("a"); print("b"); print("c"):

    hex at "output": 22 6f 75 74 70 75 74 22 3a 5b 22 61 5c 72 22 2c 22 62 5c 72 22 2c 22 63 22 5d
    text:            "output":["a\r","b\r","c"]
    element lengths: 2, 2, 1

The \r is a JSON escape emitted BY THE BINARY (bytes 5c 72, backslash + r, inside the string), not an artefact of the capture path: an earlier suspicion that PowerShell injected it was tested with a raw byte redirect and rejected.
**Reproduction 2** (clean room): V03 -> raw "output":["a\r","b"]; and Z04 print("a"); 5; print("b") -> output=[<a> len=2 <b> len=1].
**Scope:** every printed line except the last one. When the script's single print is also the only writer the value is clean (Z01: "output":["hello from the script"]) — which is exactly the CI scenario, so ci.yml cannot see it. The mechanism looks like "trim the trailing newline, then split on \n" applied to Console text that uses Environment.NewLine (?), which would make the value platform-dependent; that platform inference is not directly measurable here (no Linux host), but the Windows value provably is not what print was given.

### A4-F4 — P1 — error envelopes carry only the human elapsed string: no elapsedTime, no timings
> dsh-protocol.md:21–23 — "Durations are structural: elapsedTime is {value, unit} next to the human elapsed string, and timings carries one entry per top-level statement, so an agent never parses a unit suffix."

**Reproduction 1:** x = symbol("x"); solve(x^2 - 2 == 0, x, integer) (V04) ->

    exit 1
    keys = [protocolVersion,symbolicFormatVersion,mathIrVersion,ok,code,category,message,recoverable,diagnostics,elapsed]
    hasET=False  hasTim=False  elapsed=[486.2 µs]

To use that duration an agent must parse "486.2 µs" — the unit suffix the invariant says is never needed.
**Reproduction 2:** P07 (same script, first run): "elapsed":"428.6 µs", identical key set, no structural duration.
**Occurrences (20 probes):** P07, P11, P15, P20, P21, P22, P27, P28, P29, P30, M01, M02, M03, M04, M05, U03, V04, X01, X09, X13 — i.e. EVERY error envelope observed. The document also contradicts itself: its own error example (dsh-protocol.md:174–181) lists only "elapsed": "12 ms", so the binary matches the example and violates the invariant.

### A4-F5 — P1 — solutions.shape / represented_count / unrepresented_count all say 0 for a solved equation with infinitely many solutions
> dsh-protocol.md:144 — "An agent can answer, from structure alone: … how many solutions? (solutions.shape)"
> dsh-protocol.md:149–151 — "When the solver cannot represent the whole set, status is Partial and unrepresented_count/unrepresented_reason say how much is missing and why."

**Reproduction 1** (clean room, V05): x = symbol("x"); solve_full(sin(x) == 0, x) ->

    SolveResult(status: Solved, variable: x, domain: complex, complete: True, completeness: Complete,
      solutions: [], families: [SolutionFamily(template: k*pi, parameter: k, period: pi,
                                              parameter_domain: integer, conditions: [], exactness: ParametricExact)],
      common_conditions: [], represented_count: 0, unrepresented_count: 0, unrepresented_reason: , diagnostics: [])

**Reproduction 2** (Q08): identical; raw "solutions","value":{"kind":"Array","type":"Vector","shape":[0],"elements":[]} next to "families" with "shape":[1].
The truth (SymPy: sin(x)=0 iff x = k*pi; README.md:610 documents "k*pi for integer k") is a countably infinite set. An agent following the documented field list reads solutions.shape == [0], status == Solved, complete == true, unrepresented_count == 0 and concludes "zero solutions, nothing missing". The count is recoverable only from the UNDOCUMENTED families field.
**Severity note:** rated P1, not P0, because the product never asserts a scalar count and families does carry the truth structurally; an auditor who treats the doc's own "how many solutions? (solutions.shape)" as the contract would rate it P0. The contrast case is healthy: for the identity equation x == x the kernel answers Unevaluated with an empty families (X12), so "never a complete-looking subset" holds there.

### A4-F6 — P1 — assumptions() is a bare Text value; the assumption set has no structural form
> dsh-protocol.md:3–5 — "…a versioned JSON envelope on stdout, designed so that an agent never has to parse a display string to recover mathematical meaning."

**Reproduction 1** (clean room, V06): x = symbol("x"); assume(x > 5); assumptions() ->

    "result":{"kind":"Text","display":"x \u003E 5","typed":"x \u003E 5","structured":{"kind":"Text","value":"x > 5"}}

**Reproduction 2** (H15, and K10 for the empty case -> {"kind":"Text","value":""}): identical shape.
The builtin registry (123 entries, K18) contains assumptions and the assume* family but NO assumptions_full, while every other mathematical producer in that registry has one (solve_full, limit_full, integrate_full, simplify_full, optimize_full, linsolve_full, inv_full, cancel_full, compile_full, solve_system_full). So the relation x > 5 — a first-class object elsewhere (Relation/ne/lt nodes appear inside conditions) — is machine-visible here only as the string "x > 5", and an agent must parse ">" to branch on it.

### A4-F7 — P2 — usage errors exit 2 with an empty stdout, so invariant 6 has no structural form for a documented error class
> dsh-protocol.md:26 — "Errors are structural: code, category, message, recoverable, diagnostics." / dsh-protocol.md:185 — "Exit codes: 0 success, 1 script/diagnostic error, 2 usage error."

**Reproduction 1** (U01): Lovelace.Run.exe with no arguments -> exit 2, stdout is 0 BYTES, the message and help text go to stderr only.
**Reproduction 2** (U02): --bogus -> exit 2, stdout 0 bytes, "Error: Unknown argument '--bogus'." on stderr. Also reproduced by --print-budget 0, --print-budget -1 and --print-budget abc (B0, R05, R06, R07) — six probes.
An agent can tell "usage error" from "script error" only from the exit code (structureless), and cannot distinguish "usage error" from "the binary crashed" without reading stderr. *Counter-argument, stated for fairness:* the exit-code line explicitly separates usage errors and the script/diagnostic path is fully structural; a strict reading of invariant 6 makes this P1.

### A4-F8 — P2 — the envelope is not byte-identical across runs, contradicting the determinism claim
> README.md:858–863 — "The same script, assumptions, and seed produce byte-identical output on every run and platform — including under Native AOT (make runner ships the symbolic engine in the published binary)."

**Reproduction 1** (Z05–Z07): the same two-statement solve script run three times -> revision=82 each time, elapsed = 972.6 µs / 648.5 µs / 595 µs; raw stdout differs between every pair.
**Reproduction 2** (Z08–Z09): capabilities() twice -> revision=81 both, raw stdout differs. Also K01 vs K16: first byte difference at index 9725, inside "elapsed".
**Bounded scope (the good news):** after removing elapsed, elapsedTime.value and timings[].elapsed, the three solve runs are STRING-IDENTICAL and so are the two capabilities() runs — the mathematical content is deterministic. Only the timing fields (and, per F3, the CR in output on Windows) break byte-identity, so the README sentence is false as written for the runner's own stdout.

### A4-F9 — P2 — a missing script file is categorised ParseError
> dsh-protocol.md:183–185 — "Categories are spelled from the one taxonomy, ErrorCategory: ParseError, DomainError, UnsupportedOperation, BudgetExceeded, NoSolution, TypeMismatch, InternalInvariantFailure."

**Reproduction 1** (U03): --file C:\nope\missing.ls -> exit 1, "code":"FileReadError","category":"ParseError","message":"Cannot read script file 'C:\nope\missing.ls': Could not find a part of the path 'C:\nope\missing.ls'.","diagnostics":[].
**Reproduction 2:** the same envelope was captured on the first batch and re-read twice in the clean-room batch with identical bytes.
ParseError is a legitimate member of the taxonomy, but no parsing happened; an agent branching on category to choose "fix my syntax" vs "fix my file path" is misled. code: FileReadError is the only field that distinguishes it.

### A4-F10 — P3 — result is absent, not {"kind":"Null"}, when the last statement yields no value
> dsh-protocol.md:15 — "An absent field is {"kind":"Null"}, distinct from an empty string." / the envelope example always shows result.

**Reproduction 1** (P01): print("hello") -> top-level keys are protocolVersion,…,ok,revision,output,variables,functions,elapsed,elapsedTime,timings; there is no result key at all (and no "result":null).
**Reproduction 2** (Q03, empty script): same absence with ntimings=0. Also P13, K07, Z03.
Impact is cosmetic (the last statement produced no value), but it is exactly the absent-vs-Null distinction the invariant names.

### A4-F11 — P3 — capabilities() reports an enum type outside the documented list
> dsh-protocol.md:46–48 — "The declared enum type is part of the value: type names it (SolveStatus, Completeness, SolutionExactness, LimitStatus, IntegrationStatus, TransformStatus, RuleClassification, ErrorCategory)…"

**Reproduction 1** (K18): capabilities() -> {"name":"exactness","value":{"kind":"Enum","type":"CapabilitiesExactness","value":"BestEffort"}}.
**Reproduction 2** (K17, with --omit-functions): same field, same type. A consumer that switches on the documented type list has no case for CapabilitiesExactness.

### A4-F12 — P3 — an empty symbol name is accepted and produces the canonical atom "(sym )"
**Reproduction 1** (X02): symbol("") -> exit 0, structured={"kind":"Symbolic","pretty":"","canonical":"(sym )",…}, display length 0.
**Reproduction 2:** the same script re-run in the boundary batch produced the same "(sym )" rendering.
The canonical form contains a symbol whose name is empty, so its text form is ambiguous with a malformed atom (?). Nothing in dsh-protocol.md constrains symbol names, so this is a rough edge in the machine format rather than a contradicted claim.

### A4-F13 — P3 — CompilationResult.result_type is free text while domains and versions travel structurally
**Reproduction 1** (X07): x = symbol("x"); compile_full(x^2 + 1, [x]) ->

    CompilationResult{ir=Text(#!mathir 2 …), parameters=Arr[1](ParameterInfo{name=Text(x), domain=Domain(complex)}),
      result_domain=Domain(complex), result_type=Text(Integer), precision_policy=Text(caller_supplied),
      optimization_policy=Text(none), target=Text(mathir), mathir_version=Int(2), exact=Bool(true), diagnostics=Arr[…]}

**Reproduction 2:** the record is deterministic and re-rendered identically when the same envelope is re-read.
Not a contradiction of dsh-protocol.md (no enum is documented for result types), but an agent must string-match "Integer" while the sibling field result_domain is a Domain value — an asymmetry the protocol's kind list otherwise avoids. Invariant 2's phrase "nothing degrades to text because the serializer lacked a case" is a causal claim I cannot falsify from outside (see C5).

### A4-F14 — P2 — the builtin arity metadata the protocol names is not exported; accepted arity is recoverable only by parsing message
> dsh-protocol.md:197–199 — "The counts come from the builtin's declared metadata (Parameters, MinArity, Variadic), so a builtin that legitimately accepts a shorter form declares it (symbol(name [, domain]), solve(f, x [, domain]), dft(x [, n])) instead of the validator being widened."
> dsh-protocol.md:3–5 — "…an agent never has to parse a display string…"

**Reproduction 1** (K18): the functions registry exports exactly name, parameters, builtin[, plugin] for all 123 builtins — symbol -> parameters=[name,domain], solve -> [f,x,domain], dft -> [x,n], limit_full -> [f,x,x0]. MinArity/Variadic (or any other optionality marker) appear nowhere, and capabilities() lists no signatures either.
**Reproduction 2** (arity behaviour, four probes): symbol() -> InvalidArgument/TypeMismatch "symbol(): expected 1 to 2 arguments; got 0." (X01); symbol("x", real, 3) -> "…expected 1 to 2 arguments; got 3." (P20); dft([1,2,3,4], 2, 9) -> "dft(): expected 1 to 2 arguments; got 3." (Q10); compile_full(x) -> "compile_full(): expected 2 arguments; got 1." (P11).
The behaviour matches the documented declaration model, but the metadata that decides it is not machine-visible: to learn that dft's n is optional an agent must provoke an error and parse the human message string. No new probes: this finding re-uses K18, X01, P20, Q10, P11.

## 3. COULD NOT DECIDE

| # | claim | why undecidable | what would make it checkable |
|---|---|---|---|
| C1 | dsh-protocol.md:80–87 — the codes transform.budget-exceeded and transform.unsatisfiable-conditions, and "a transform.unsatisfiable-conditions diagnostic is false (retrying cannot give that branch a model)" | No language surface produces them: transform_full -> Unknown function (Q15), and the registry (K18) has no transform/transform_full. simplify_full does return a TransformResult, but its only observed status is Satisfied with budget_exceeded=false (P25) | a builtin (or documented invocation) that runs a transform with unsatisfiable side conditions and with an exhausted budget; then assert code/category/recoverable |
| C2 | dsh-protocol.md:19 — truncationReason may be depth-limit | Never observed. Budget sweeps 1–15 (S01–S15, T01–T06, B1–B8), a 998-node value (K13), a 198-node value (R04) and a 21-node nested expression with --print-budget 100000 (K20) produced either node-budget or no truncation at all | the depth threshold in the serializer, or an input that exceeds it, or a documented depth unit |
| C3 | dsh-protocol.md:164 — table row BudgetExceeded -> complete:false / "the kernel's Completeness for the subset it stopped on" | No probe produced a SolveResult with status BudgetExceeded. --cancel-after 1 aborts the whole envelope instead (M05: code "Cancelled", category "BudgetExceeded", ok:false, no partial SolveResult), and --cancel-after is not documented in dsh-protocol.md at all | a documented solver budget knob, or a documented statement of what --cancel-after returns |
| C4 | dsh-protocol.md:88–90 — a DiagnosticLocation record with start_line/start_column/end_line/end_column | Every kernel diagnostic observed (P04, Q13, Q14, Q16, Q17, V05) carried "location":{"kind":"Null"}. The error envelope's parser diagnostics use the other form (message/position/line/column, P07 etc.) | any input yielding a source-located kernel diagnostic, or a documented example of one |
| C5 | dsh-protocol.md:11–13 — "nothing degrades to text because the serializer lacked a case" | The causal half is unfalsifiable from outside; the kind-list half holds (every field is one of the listed kinds). Text-valued fields that look enum-like exist but are not in the doc's enum list: OptimizationResult.transformations=["cse","horner"], .target="mathir", .policy="cse+horner+precision-aware"; IntegrationResult.method="", .verification_method=""; TransformResult.budget_kind=""; CompilationResult.result_type="Integer" (Q22, P24, P25, X07) | a documented list of which fields are closed sets; then each could be asserted to be an Enum |
| C6 | docs/symbolics/a-plus-convergence-alignment-plan.md:654 — "capabilities()'s unsupported_operations list is now exhaustive rather than self-declared" | Exhaustiveness is a negative claim over an unbounded input space. All 16 advertised triggers reproduced with the advertised code/category/message (P27, P28, P07, P04, Q13, Q14, Q28, Q29, M02, M01, P29, M03, Q33, Q30, Q31, P30), but the space of unsupported operations cannot be bounded from outside | a definition of the supported surface (e.g. a per-builtin capability matrix) that can be enumerated and compared |
| C7 | docs/symbolics/a-plus-convergence-alignment-plan.md:151 — "BuiltinDescriptor.Variadic/MinArity … are never set or read" | Internal-metadata claim; the binary exposes neither flag (see A4-F14), but the observable behaviour (range-form arity errors such as "expected 1 to 2 arguments") is consistent with such metadata existing and being read | source inspection, or a debug surface that exposes the descriptor |
| C8 | Harness mistakes, listed so the reader can discount them | P15 (x/y with no symbols declared), K03 (true is not a literal), K05/K06 (i is not a literal), Q34/K12 (bad func syntax, undeclared y), X09 (member access on a Vector — my expectation, not a contract claim), X13 (0 == 0 folds to Boolean before solve_full). These eight probes failed to test what they were written for | each relevant behaviour was re-probed successfully elsewhere (N06, H19, H20, P03, X12) |

## 4. Summary

**235 probe instances** were run against out/aot/Lovelace.Run.exe: **171 HELD**, **54 FINDING**, **10 INCONCLUSIVE**.

| group | probes | HELD | FINDING | INCONCLUSIVE |
|---|---|---|---|---|
| P — protocol invariants | 30 | 26 | 3 | 1 |
| Q — envelope / capabilities triggers | 35 | 29 | 4 | 2 |
| U — exit codes and usage errors | 3 | 0 | 3 | 0 |
| R — print-budget core | 10 | 4 | 6 | 0 |
| B — print-budget boundary sweep | 12 | 2 | 10 | 0 |
| E — print-budget expanded powers | 11 | 11 | 0 | 0 |
| F — print-budget threshold | 10 | 9 | 1 | 0 |
| S — print-budget at fixed nodeCount | 15 | 15 | 0 | 0 |
| T — print-budget at fixed nc=5 | 6 | 0 | 6 | 0 |
| N — statement kinds, timings, boundary | 8 | 8 | 0 | 0 |
| V — clean-room re-runs | 8 | 2 | 6 | 0 |
| H — Lovelace.Symbolics README behaviour | 30 | 29 | 1 | 0 |
| M — capabilities triggers and root README | 13 | 13 | 0 | 0 |
| K — encodings, registry, determinism | 20 | 10 | 5 | 5 |
| X — boundary attack | 14 | 10 | 2 | 2 |
| Z — CI scenarios, byte capture, repeat runs | 9 | 2 | 7 | 0 |
| W — SymPy cross-check | 1 | 1 | 0 | 0 |
| **total** | **235** | **171** | **54** | **10** | The mathematical core survived an adversarial pass intact — 40+ results cross-checked against SymPy 1.14.0 (roots and root counts, the RootOf real/complex split, limits and one-sided limits, integrals and their derivatives, series, determinants/inverses/matrix solves, Groebner system solutions, evalf digit counts, arbitrary-precision integers) agreed everywhere; no internal invariant failure was raised; no valid input died without an envelope. The protocol's structural promises also largely hold: the real-solve envelope, the SolveStatus/Completeness mapping table, the six-field Diagnostic in declared order ending every rich result record, {"kind":"Null"} for absent fields, position-accurate timings with Void/hasOutput, the InvalidArgument/TypeMismatch arity error verbatim, exit codes 0/1/2, and the CI section's AOT-publish-then-execute gate (verified by reading ci.yml and reproducing all five of its scenarios). Where the documentation breaks is at the edges: **six P1 findings** — --print-budget silently not bounding values whose node count already exceeds it (A4-F1), a stale canonical integer atom (int n) that the binary writes as (rat n 1) (A4-F2), a stray CR on every output entry except the last on Windows (A4-F3), error envelopes that ship only the human elapsed string (A4-F4), an infinite solution family reported with solutions.shape, represented_count and unrepresented_count all zero under complete: true (A4-F5), and assumptions() as the one mathematical producer with no structural form (A4-F6) — plus **four P2s** (structureless usage errors, run-to-run byte differences that falsify README §18, a file-read failure labelled ParseError, and arity metadata that is not exported so accepted arity must be read out of a message string), **four P3s**, and **eight claims that could not be decided** because the surface that would exercise them (transform, a solver budget, source-located diagnostics) is not reachable from the language.
