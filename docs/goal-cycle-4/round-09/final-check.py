# P1 audit: final compact checks (differences, digit counts, timings, g11 digits).
import json, re, sys
sys.set_int_max_str_digits(200000)
from sympy import N, Rational, Integer, sqrt

def load(raw_path):
    out, cur = {}, None
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

base = r'C:\Users\ricar\dev\LovelaceSharp\docs\goal-cycle-4\round-09\raw'
allb = {}
for b in ('batch1', 'batch2', 'batch3', 'batch4'):
    allb.update(load(base + '\\' + b + '.txt'))

def value_of(entry):
    o = allb[entry]
    s = o['result']['structured']
    if 'numerator' in s:
        return Rational(Integer(int(s['numerator'])), Integer(int(s['denominator']))), 'num/den', s.get('exact'), o['result']['display']
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
    return got, f'display({len(txt)} chars)', s.get('exact'), txt

print('##### 1. headline: false exactness (exact:true, value wrong) #####')
for nm, truth in [('m01_repro_one_over_0_95_times_0_95', Integer(1)),
                  ('m09_one_over_0_55_times_0_55', Integer(1)),
                  ('m19_one_over_0_94_times_0_94', Integer(1)),
                  ('m45_neg_recip_times', Integer(-1)),
                  ('n08_two_pow_neg5000', Rational(1, 2**5000)),
                  ('n09_two_pow_neg30103', Rational(1, 2**30103)),
                  ('n12_ten_pow_neg1001', Rational(1, 10**1001)),
                  ('p06_zero_pow_real_neg1', None)]:
    got, src, ex, disp = value_of(nm)
    print(f'{nm}:  exact flag={ex}  source={src}')
    print(f'   observed digits={len(str(abs(got.numerator)))}/{len(str(got.denominator))}  = {got if len(str(got))<40 else str(got)[:40]+"..."}')
    print(f'   observed as float(25) = {N(got, 25)}')
    if truth is not None:
        print(f'   exact   as float(25) = {N(truth, 25)}')
        print(f'   observed - exact = {N(got - truth, 10)}  (equal? {got == truth})')

print()
print('##### 2. wrong values flagged exact:false (difference magnitude only) #####')
for nm, truth in [('m07_one_over_0_75_times_0_75', Integer(1)),
                  ('m11_one_over_0_35_times_0_35', Integer(1)),
                  ('m16_one_over_0_98_times_0_98', Integer(1)),
                  ('m17_one_over_0_97_times_0_97', Integer(1)),
                  ('m18_one_over_0_96_times_0_96', Integer(1)),
                  ('m29_one_over_17_times_17', Integer(1)),
                  ('m30_one_over_23_times_23', Integer(1)),
                  ('m34_one_over_97_times_97', Integer(1)),
                  ('k01_four_thirds_times_three_quarters', Integer(1)),
                  ('k19_bad_product_sqrt', Integer(1)),
                  ('k07_seventeen_times_one_over_17_var', Integer(1)),
                  ('d37_identity_one_over_7_times_7', Integer(1))]:
    got, src, ex, disp = value_of(nm)
    diff = got - truth
    print(f'{nm}: exact flag={ex} equal? {got == truth}  1-observed = {N(-diff, 6)}  ({src})')

print()
print('##### 3. g11: evalf(1/998001, 40) digits vs SymPy 1000 decimals #####')
d = allb['g11_evalf_1998_40']['result']['display']
truth = str(N(Rational(1, 998001), 1100))
ed = d.replace('.', '')
td = truth.replace('.', '')
k = min(len(ed), len(td))
first_bad = next((i for i in range(k) if ed[i] != td[i]), None)
print('engine decimals:', len(ed), ' first mismatch index:', first_bad)
print('engine tail:', ed[-30:])
print('sympy  same window:', td[len(ed)-30:len(ed)])

print()
print('##### 4. n07 2^-1001 truncation check #####')
d = allb['n07_two_pow_neg1001']['result']['display']
truth = str(N(Rational(1, 2**1001), 1010)).rstrip('0') if False else str(N(Rational(1, 2**1001), 1005))
print('engine len:', len(d), ' engine tail:', d[-25:])
print('sympy  head:', truth[:26], ' sympy 1010-digit tail:', str(N(Rational(1, 2**1001), 1010))[-25:])

print()
print('##### 5. timings: slowest envelopes (hang check) #####')
rows = []
for nm, o in allb.items():
    if o is None:
        continue
    el = o.get('elapsedTime') or {}
    v = el.get('value')
    if v is None:
        continue
    rows.append((v, el.get('unit'), nm, o.get('elapsed')))
rows.sort(reverse=True)
for v, u, nm, raw in rows[:10]:
    print(f'  {nm}: {raw}')
print('total probes measured:', len(rows))
units = set(u for _, u, _, _ in rows)
print('units seen:', units)
