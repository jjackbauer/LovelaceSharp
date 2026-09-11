# P1 audit: digit-by-digit comparison of evalf() output against SymPy.
# usage: python3 compare-digits.py <raw-batch.txt> <probe-name> <sympy-expr> <digits>
import json, sys, re
sys.set_int_max_str_digits(200000)
from sympy import N, sqrt, Rational, sympify

raw = open(sys.argv[1], encoding='utf-8').read().split('\n')
want = set(sys.argv[2].split(','))
bodies, outs = {}, {}
cur = None
for line in raw:
    if line.startswith('### '):
        cur = line[4:].strip()
    elif line.startswith('STDOUT: ') and cur:
        outs[cur] = line[8:]
        cur = None

def sig_digits(s):
    s = s.strip().lstrip('-')
    if '.' in s:
        ip, fp = s.split('.')
    else:
        ip, fp = s, ''
    return (ip + fp).lstrip('0'), len(ip.lstrip('0').lstrip()) if ip.lstrip('0') else 0

for name in want:
    o = json.loads(outs[name])
    disp = o['result']['display']
    kind = o['result']['kind']
    exact = o['result']['structured'].get('exact')
    print('=' * 100)
    print(f'{name}: kind={kind} exact={exact} display_len={len(disp)}')
    print(f'  engine: {disp}')
    if kind == 'Real' and 'numerator' in o['result']['structured']:
        num = o['result']['structured']['numerator']; den = o['result']['structured']['denominator']
        print(f'  numerator/denominator: {num} / {den}')
