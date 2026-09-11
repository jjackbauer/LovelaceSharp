# P1 audit: parse a raw batch log, re-derive every envelope, and check the
# numeric claims against SymPy ground truth.
# usage: python3 analyze.py <raw-batch.txt> [expected-json]
import json, re, sys
sys.set_int_max_str_digits(200000)
from sympy import Rational, N, Integer, sqrt, nsimplify, srepr

def parse_period(display):
    """Parse Lovelace display syntax: [-]int[.frac][(period)] -> exact Rational or None."""
    m = re.fullmatch(r'(-?)(\d+)(?:\.(\d*)(?:\((\d+)\))?)?', display.strip())
    if not m:
        return None
    sign, ip, frac, per = m.group(1), m.group(2), m.group(3) or '', m.group(4)
    val = Rational(int(ip))
    if frac:
        val += Rational(int(frac), 10**len(frac))
    if per:
        k = len(per)
        val += Rational(int(per), (10**k - 1) * 10**len(frac))
    return -val if sign == '-' else val

def rational_of(s):
    return Rational(int(s)) if s not in (None, '') else None

raw = open(sys.argv[1], encoding='utf-8').read().split('\n')
cases = []
cur = None
for line in raw:
    if line.startswith('### '):
        cur = {'name': line[4:].strip()}
        cases.append(cur)
    elif cur is not None and line.startswith('BODY: '):
        cur['body'] = line[6:]
    elif cur is not None and line.startswith('EXIT: '):
        cur['exit'] = line[6:]
    elif cur is not None and line.startswith('STDOUT: '):
        cur['stdout'] = line[8:]
    elif cur is not None and line.startswith('STDERR: '):
        cur['stderr'] = line[8:]

for c in cases:
    print('=' * 100)
    print(f"PROBE {c['name']}")
    print(f"  BODY   : {c.get('body')}")
    print(f"  EXIT   : {c.get('exit')}")
    if 'stderr' in c:
        print(f"  STDERR : {c['stderr']}")
    try:
        o = json.loads(c.get('stdout', ''))
    except Exception as e:
        print(f"  STDOUT : UNPARSEABLE ({e}): {c.get('stdout','')[:200]}")
        continue
    if not o.get('ok'):
        print(f"  RESULT : ERROR envelope code={o.get('code')} category={o.get('category')} msg={o.get('message')}")
        continue
    r = o['result']
    s = r.get('structured', {})
    disp = r.get('display', '')
    print(f"  KIND   : {r.get('kind')}   structured.kind={s.get('kind')}")
    print(f"  DISPLAY: {disp if len(disp) <= 120 else disp[:60] + '...[' + str(len(disp)) + ' chars]...' + disp[-30:]}")
    if 'value' in s:
        print(f"  VALUE  : {s['value'] if len(str(s['value'])) <= 120 else str(s['value'])[:60] + '...[len ' + str(len(str(s['value']))) + ']'}")
    if 'numerator' in s:
        print(f"  NUM/DEN: {s['numerator']} / {s['denominator']}")
    if 'exact' in s:
        print(f"  EXACT  : {s['exact']}")
    if s.get('kind') == 'Boolean':
        print(f"  BOOL   : {s.get('value')}")
