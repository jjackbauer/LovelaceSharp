# P1 audit: SymPy verification of the batch5 identity sweep (corrected expectations).
# Every probe body has the form "(a/b)*b"; the expected value is therefore exactly a.
import json, re, sys
sys.set_int_max_str_digits(200000)
from sympy import N, Rational, Integer

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

allb = load(r'C:\Users\ricar\dev\LovelaceSharp\docs\goal-cycle-4\round-09\raw\batch5.txt')

def value_of(name):
    o = allb[name]
    s = o['result']['structured']
    if 'numerator' in s:
        return Rational(Integer(int(s['numerator'])), Integer(int(s['denominator']))), 'num/den', s.get('exact'), o['result']['display'], s.get('kind')
    txt = o['result']['display']
    if s.get('kind') == 'Boolean':
        return txt, 'boolean', None, txt, 'Boolean'
    m = re.fullmatch(r'(-?)(\d+)(?:\.(\d*)(?:\((\d+)\))?)?', txt)
    sign, ip, frac, per = m.group(1), m.group(2), m.group(3) or '', m.group(4)
    got = Rational(int(ip))
    if frac:
        got += Rational(int(frac), 10**len(frac))
    if per:
        got += Rational(int(per), (10**len(per) - 1) * 10**len(frac))
    if sign == '-':
        got = -got
    return got, f'display({len(txt)} chars)', s.get('exact'), txt, s.get('kind')

H = Rational(1, 2)
EXPECT = {
 'i01_one_over_0_7_times_0_7': ('(1/0.7)*0.7', Integer(1)),
 'i02_one_over_0_71_times_0_71': ('(1/0.71)*0.71', Integer(1)),
 'i03_one_over_0_91_times_0_91': ('(1/0.91)*0.91', Integer(1)),
 'i04_one_over_0_87_times_0_87': ('(1/0.87)*0.87', Integer(1)),
 'i05_one_over_0_63_times_0_63': ('(1/0.63)*0.63', Integer(1)),
 'i06_one_over_0_49_times_0_49': ('(1/0.49)*0.49', Integer(1)),
 'i07_one_over_0_21_times_0_21': ('(1/0.21)*0.21', Integer(1)),
 'i08_one_over_0_13_times_0_13': ('(1/0.13)*0.13', Integer(1)),
 'i09_one_over_0_11_times_0_11': ('(1/0.11)*0.11', Integer(1)),
 'i10_one_over_0_09_times_0_09': ('(1/0.09)*0.09', Integer(1)),
 'i11_one_over_0_07_times_0_07': ('(1/0.07)*0.07', Integer(1)),
 'i12_one_over_0_03_times_0_03': ('(1/0.03)*0.03', Integer(1)),
 'i13_one_over_0_01_times_0_01': ('(1/0.01)*0.01', Integer(1)),
 'i14_one_over_0_999_times_0_999': ('(1/0.999)*0.999', Integer(1)),
 'i15_one_over_0_99999_times': ('(1/0.99999)*0.99999', Integer(1)),
 'i16_swapped_0_75': ('0.75*(1/0.75)', Rational(3,4)),
 'i17_seven_over_0_75_times': ('(7/0.75)*0.75', Integer(7)),
 'i18_seven_over_0_95_times': ('(7/0.95)*0.95', Integer(7)),
 'i19_123_over_0_97_times': ('(123/0.97)*0.97', Integer(123)),
 'i20_alt_bracket_0_75': ('(1/(0.75))*0.75', Integer(1)),
 'i26_one_over_0_750_times': ('(1/0.750)*0.750', Integer(1)),
 'i27_one_over_0_75_times_three_quarters': ('(1/0.75)*(3/4)', Integer(1)),
 'i28_four_thirds_times_0_75': ('(4/3)*0.75', Integer(1)),
 'i30_one_over_6_times_3': ('(1/6)*3', H),
 'i31_one_over_12_times_12': ('(1/12)*12', Integer(1)),
 'i32_one_over_14_times_14': ('(1/14)*14', Integer(1)),
 'i33_one_over_18_times_18': ('(1/18)*18', Integer(1)),
 'i34_one_over_22_times_22': ('(1/22)*22', Integer(1)),
 'i35_one_over_26_times_26': ('(1/26)*26', Integer(1)),
 'i37_one_over_0_75_times_1_5': ('(1/0.75)*1.5', Integer(2)),
 'i38_zero_times_bad': ('0*(1/0.75)*0.75', Integer(0)),
}

def mag(x):
    s = N(x, 6)
    return s

print('probe | body | expected (SymPy exact) | engine value | engine exact flag | source | value==expected | |1-value|')
bad = []
for nm, (body, truth) in EXPECT.items():
    got, src, ex, disp, kind = value_of(nm)
    eq = (got == truth)
    if not eq:
        bad.append(nm)
    diff = N(got - truth, 6)
    print(f'{nm}\n   body={body}\n   expected={truth}   engine={str(got)[:60]}{"..." if len(str(got))>60 else ""}\n   equal={eq}  exact_flag={ex}  src={src}  diff={diff}')
print()
print('FAILING identities:', len(bad), 'of', len(EXPECT))
for nm in bad:
    print('   ', nm, '|', EXPECT[nm][0])
holding = [nm for nm in EXPECT if nm not in bad]
print('HOLDING identities:', len(holding))
for nm in holding:
    print('   ', nm, '|', EXPECT[nm][0])

print()
print('=== Boolean probes ===')
for nm in ('i24_four_thirds_product_eq_one', 'i36_one_over_0_7_times_0_7_eq'):
    got, src, ex, disp, kind = value_of(nm)
    print(f'{nm}: engine says {disp}')

print()
print('=== evalf((4/3)*(3/4), N) for N=19,20,40: identical wrong value? ===')
v21, _, _, d21, _ = value_of('i21_evalf_four_thirds_product_20')
for nm in ('i21_evalf_four_thirds_product_20', 'i22_evalf_four_thirds_product_40', 'i23_evalf_four_thirds_product_19'):
    got, src, ex, disp, kind = value_of(nm)
    print(f'{nm}: display_len={len(disp)} tail={disp[-12:]}  equals_1? {got == 1}  identical_to_others? {got == v21}')

print()
print('=== i29 / i39 spot checks ===')
lits = Rational(int('1' + '3'*37), 10**37)
for nm, truth in (('i29_long_literal_times_0_75', lits * Rational(3,4)),
                  ('i39_one_over_0_75_minus_three_quarters', Rational(4,3) - lits)):
    got, src, ex, disp, kind = value_of(nm)
    print(f'{nm}: engine={disp}  exact_flag={ex}')
    print(f'   truth={truth}   equal={got == truth}')
