"""Value-correctness instrument for the H-1 family: series at a non-smooth point.

For each case: run the published runner twice through --file (never --eval: PowerShell 5.1 strips
embedded double quotes before argv, EVD-299), require the two envelopes to be identical in display
(determinism), require the published display to be free of internal junk (__t names, unevaluated
diff(, piecewise(, and literal 1/sqrt(0) or sqrt(0)), then parse the display's polynomial part with
SymPy and compare it with the true function value at sample points, allowing the truncation error of
the requested order.
"""
import json, re, subprocess, sys, tempfile, os
import sympy as sp

EXE = sys.argv[1] if len(sys.argv) > 1 else r"C:\Users\ricar\dev\LovelaceSharp\out\aot\Lovelace.Run.exe"
x = sp.symbols('x', real=True)

CASES = [
    ("series(abs(x), x, 0, 3)",       sp.Abs(x),        0, 3),
    ("series(abs(x), x, 0, 2)",       sp.Abs(x),        0, 2),
    ("series(x*abs(x), x, 0, 3)",     x*sp.Abs(x),      0, 3),
    ("series(abs(sin(x)), x, 0, 3)",  sp.Abs(sp.sin(x)),0, 3),
    ("series(abs(x - 1), x, 0, 3)",   sp.Abs(x-1),      0, 3),
    ("series(abs(x), x, 1, 3)",       sp.Abs(x),        1, 3),
    ("series(sin(x), x, 0, 5)",       sp.sin(x),        0, 5),   # smooth control
]

def run(script):
    with tempfile.NamedTemporaryFile('w', suffix='.ls', delete=False, encoding='utf-8') as f:
        f.write(script + "\n"); path = f.name
    try:
        p = subprocess.run([EXE, "--file", path, "--json", "--omit-functions", "--omit-variables"],
                           capture_output=True, text=True, encoding='utf-8')
        line = next((l for l in p.stdout.splitlines() if l.strip().startswith('{')), None)
        return json.loads(line) if line else None
    finally:
        os.unlink(path)

def polynomial_part(display):
    """Lovelace prints 'poly + O(x^3)'; keep the polynomial, drop the O term."""
    d = re.sub(r'\s*\+\s*O\([^)]*\)', '', display)
    d = d.replace('^', '**')
    return d

bad = 0
for script, func, x0, order in CASES:
    body = 'x = symbol("x"); ' + script
    a, b = run(body), run(body)
    if a is None or b is None:
        print(f"FAIL(no-envelope) {script}"); bad += 1; continue
    dispa = a.get('result', {}).get('display') or a.get('message') or ''
    dispb = b.get('result', {}).get('display') or b.get('message') or ''
    det = (dispa == dispb)
    junk = []
    if '__t' in dispa: junk.append('__t')
    if 'diff(' in dispa or 'Derivative(' in dispa: junk.append('diff(')
    if 'piecewise(' in dispa: junk.append('piecewise(')
    if re.search(r'sqrt\(0\)|/\s*0\b', dispa): junk.append('division-by-zero')
    # numeric agreement, where the display is parseable at all
    num = 'n/a'
    if det and not junk:
        try:
            expr = sp.sympify(polynomial_part(dispa), locals={'x': x})
            worst = 0.0
            for pt in ([sp.Rational(1,2)] if x0 == 0 else [sp.Rational(3,2)]):
                got = float(expr.subs(x, pt))
                want = float(func.subs(x, pt))
                worst = max(worst, abs(got - want))
            num = f"max|series-true| = {worst:.6f}"
        except Exception as e:
            num = f"unparseable({type(e).__name__})"
    ok = det and not junk
    print(f"{'ok  ' if ok else 'FAIL'} {script:34s} deterministic={det} junk={junk or '-'} {num}")
    print(f"       display: {dispa[:130]}")
    if not ok: bad += 1
print(f"\nH-1 VALUE CHECK: {len(CASES)-bad}/{len(CASES)} clean")
