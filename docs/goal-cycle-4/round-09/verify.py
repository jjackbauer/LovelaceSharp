# P1 audit: digit-level comparison of specific engine displays against SymPy.
import json, sys, re
sys.set_int_max_str_digits(200000)
from sympy import N, sqrt, Rational, Integer, pi, E

def load(raw_path):
    out = {}
    cur = None
    for line in open(raw_path, encoding='utf-8'):
        line = line.rstrip('\n')
        if line.startswith('### '):
            cur = line[4:].strip()
        elif line.startswith('STDOUT: ') and cur:
            try:
                out[cur] = json.loads(line[8:])
            except Exception:
                out[cur] = None
            cur = None
    return out

def digits_of(s):
    return s.replace('.', '').replace('-', '')

b1 = load(r'C:\Users\ricar\dev\LovelaceSharp\docs\goal-cycle-4\round-09\raw\batch1.txt')
b2 = load(r'C:\Users\ricar\dev\LovelaceSharp\docs\goal-cycle-4\round-09\raw\batch2.txt')
b3 = load(r'C:\Users\ricar\dev\LovelaceSharp\docs\goal-cycle-4\round-09\raw\batch3.txt')
b4 = load(r'C:\Users\ricar\dev\LovelaceSharp\docs\goal-cycle-4\round-09\raw\batch4.txt')
allb = {**b1, **b2, **b3, **b4}

print('##### A. evalf(sqrt(2), 60) display vs SymPy 100-decimal value #####')
d = allb['m51_evalf_sqrt2_60']['result']['display']
truth = str(N(sqrt(2), 110)).replace('.', '')
print('engine display length:', len(d))
print('engine:', d)
print('sympy :', N(sqrt(2), 110))
ed, td = digits_of(d), digits_of(str(N(sqrt(2), 110)))
n = min(len(ed), len(td))
first_bad = next((i for i in range(n) if ed[i] != td[i]), None)
print('compared digits:', n, ' first differing index:', first_bad)
if first_bad is not None:
    print('  engine from there:', ed[first_bad:first_bad + 40])
    print('  sympy  from there:', td[first_bad:first_bad + 40])
print('engine last 20 digits:', ed[-20:])
print('sympy  last 20 digits:', td[:len(ed)][-20:])

print()
print('##### B. evalf(1/3, 100) all threes? #####')
d = allb['m50_evalf_one_third_100']['result']['display']
frac = d.split('.')[1]
print('frac digits:', len(frac), 'all 3?', set(frac) == {'3'})

print()
print('##### C. 2^100000 claimed Natural vs SymPy #####')
d = allb['p13_two_pow_100000']['result']['display']
t = str(2**100000)
print('engine digits:', len(d), 'sympy digits:', len(t), 'equal?', d == t)
print('engine head:', d[:40])
print('engine tail:', d[-40:])
print('sympy  tail:', t[-40:])

print()
print('##### D. false-exactness and wrong-value cases: engine value vs exact truth #####')
def report(name, truth_expr, note=''):
    o = allb[name]
    s = o['result']['structured']
    disp = o['result']['display']
    if 'numerator' in s:
        got = Rational(Integer(int(s['numerator'])), Integer(int(s['denominator'])))
        src = 'numerator/denominator'
    else:
        txt = disp
        m = re.fullmatch(r'(-?)(\d+)(?:\.(\d*)(?:\((\d+)\))?)?', txt)
        sign, ip, frac, per = m.group(1), m.group(2), m.group(3) or '', m.group(4)
        got = Rational(int(ip))
        if frac:
            got += Rational(int(frac), 10**len(frac))
        if per:
            got += Rational(int(per), (10**len(per) - 1) * 10**len(frac))
        if sign == '-':
            got = -got
        src = f'display ({len(txt)} chars)'
    truth = truth_expr
    diff = got - truth
    print(f'{name}:')
    print(f'   observed  = {str(got)[:70]}{"..." if len(str(got))>70 else ""}   [{src}]')
    print(f'   exact     = {truth}')
    print(f'   observed - exact = {str(diff)[:70]}{"..." if len(str(diff))>70 else ""}')
    print(f'   equal? {diff == 0}    envelope exact flag = {s.get("exact")}')

report('m01_repro_one_over_0_95_times_0_95', Integer(1))
report('m09_one_over_0_55_times_0_55', Integer(1))
report('m19_one_over_0_94_times_0_94', Integer(1))
report('m45_neg_recip_times', Integer(-1))
report('m07_one_over_0_75_times_0_75', Integer(1))
report('m11_one_over_0_35_times_0_35', Integer(1))
report('m16_one_over_0_98_times_0_98', Integer(1))
report('m17_one_over_0_97_times_0_97', Integer(1))
report('m18_one_over_0_96_times_0_96', Integer(1))
report('m29_one_over_17_times_17', Integer(1))
report('m30_one_over_23_times_23', Integer(1))
report('m34_one_over_97_times_97', Integer(1))
report('k01_four_thirds_times_three_quarters', Integer(1))
report('k19_bad_product_sqrt', Integer(1))
report('k20_bad_product_squared', Integer(1))
report('k07_seventeen_times_one_over_17_var', Integer(1))
report('n08_two_pow_neg5000', Rational(1, 2**5000))
report('n09_two_pow_neg30103', Rational(1, 2**30103))
report('n12_ten_pow_neg1001', Rational(1, 10**1001))

print()
print('##### E. terminating values that DID hold (spot check) #####')
for nm, truth in [('n04_two_pow_neg100', Rational(1, 2**100)),
                  ('n06_two_pow_neg1000', Rational(1, 2**1000)),
                  ('n07_two_pow_neg1001', Rational(1, 2**1001)),
                  ('n13_ten_pow_neg1000', Rational(1, 10**1000)),
                  ('n14_ten_pow_neg999', Rational(1, 10**999)),
                  ('d06_one_over_near1_38nines', Rational(10**38, 10**38 - 1)),
                  ('d07_one_over_near1_39nines', Rational(10**39, 10**39 - 1)),
                  ('d10_one_over_1_plus_1e38', Rational(10**38, 10**38 + 1)),
                  ('d11_one_over_1_plus_tiny', Rational(10**39, 10**39 + 1)),
                  ('d29_one_over_neg_1_0000001', Rational(-10**7, 10**7 + 1)),
                  ('d31_one_over_0_999999999999999999999999999999999999', Rational(10**36, 10**36 - 1)),
                  ('d32_0_9_over_0_9999', Rational(1000, 1111)),
                  ('g01_one_over_37nines', Rational(10**37, 10**37 - 1)),
                  ('m43_one_over_0_95_plus_1_over_0_95', Rational(40, 19))]:
    o = allb[nm]
    s = o['result']['structured']
    if 'numerator' in s:
        got = Rational(Integer(int(s['numerator'])), Integer(int(s['denominator'])))
        src = 'num/den'
    else:
        txt = o['result']['display']
        m = re.fullmatch(r'(-?)(\d+)(?:\.(\d*)(?:\((\d+)\))?)?', txt)
        sign, ip, frac, per = m.group(1), m.group(2), m.group(3) or '', m.group(4)
        got = Rational(int(ip))
        if frac:
            got += Rational(int(frac), 10**len(frac))
        if per:
            got += Rational(int(per), (10**len(per) - 1) * 10**len(frac))
        if sign == '-':
            got = -got
        src = f'display({len(txt)})'
    print(f'{nm}: equal? {got == truth}   [{src}]   exact flag={s.get("exact")}')

print()
print('##### F. k05/k12/k10 details #####')
for nm in ('k05_two_over_17_times_17', 'k12_bad_17_product_times_17', 'k10_bad_product_scaled', 'k08_bad_product_minus_one_75', 'k18_bad_product_div_self'):
    o = allb[nm]
    s = o['result']['structured']
    print(f'{nm}: display={o["result"]["display"]}')
    print(f'    structured keys={list(s.keys())} exact={s.get("exact")}')
