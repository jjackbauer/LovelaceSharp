# P1 audit report generator: builds the probe table straight from the raw evidence logs
# (no hand transcription) and assembles audit-P1-numeric.md from report-prose.md.
import json, re, sys, io, os
sys.set_int_max_str_digits(200000)
from sympy import Rational, Integer, N, sqrt, oo, nan, S, sympify

ROOT = r'C:\Users\ricar\dev\LovelaceSharp\docs\goal-cycle-4\round-09'
BATCHES = ['batch1', 'batch2', 'batch3', 'batch4', 'batch5']

def load(batch):
    out, order, cur = {}, [], None
    for line in open(os.path.join(ROOT, 'raw', batch + '.txt'), encoding='utf-8'):
        line = line.rstrip('\n')
        if line.startswith('### '):
            cur = line[4:].strip()
            order.append(cur)
        elif line.startswith('BODY: ') and cur:
            out.setdefault(cur, {})['body'] = line[6:]
        elif line.startswith('EXIT: ') and cur:
            out.setdefault(cur, {})['exit'] = line[6:]
        elif line.startswith('STDOUT: ') and cur:
            out.setdefault(cur, {})['stdout'] = line[8:]
        elif line.startswith('STDERR: ') and cur:
            out.setdefault(cur, {})['stderr'] = line[8:]
    return out, order

PROBES, ORDER = {}, []
for b in BATCHES:
    d, o = load(b)
    for name in o:
        PROBES[name] = d[name]
        PROBES[name]['batch'] = b
    ORDER.extend(o)

# ---------------------------------------------------------------- oracles
R = Rational
def r(x):
    return R(x)
ORACLE = {}
LABEL = {}
# --- batch1: division boundary ---
div = {
 'd01_one_over_0_9': (10,9), 'd02_one_over_0_99': (100,99), 'd03_one_over_0_999': (1000,999),
 'd04_one_over_0_9999': (10000,9999), 'd05_one_over_0_95': (20,19), 'd08_one_over_1_1': (10,11),
 'd09_one_over_1_01': (100,101), 'd12_one_over_1': (1,1), 'd13_one_over_0_5': (2,1),
 'd14_one_over_2': (1,2), 'd15_one_over_3': (1,3), 'd16_one_over_11': (1,11),
 'd17_one_over_27': (1,27), 'd18_one_over_7': (1,7), 'd19_one_over_13': (1,13),
 'd20_one_over_21': (1,21), 'd21_one_over_37': (1,37), 'd22_one_over_101': (1,101),
 'd23_ten_over_9': (10,9), 'd24_hundred_over_99': (100,99), 'd25_thousand_over_999': (1000,999),
 'd26_two_over_3': (2,3), 'd27_one_over_neg_0_9': (-10,9), 'd28_neg_one_over_0_9': (-10,9),
 'd30_one_over_1e-30': (10**30,1), 'd32_0_9_over_0_9999': (1000,1111), 'd33_identity_a_over_b_mul_b_period3': (1,1),
 'd34_identity_a_mul_recip_period3': (1,27), 'd35_identity_one_over_0_9_times_0_9': (1,1),
 'd36_identity_one_over_0_95_times_0_95': (1,1), 'd37_identity_one_over_7_times_7': (1,1),
 'd38_identity_one_over_3_times_3': (1,1), 'd39_a_over_b_times_b_mixed': (7,1),
 'd40_div_exactness_rational': (1,4),
}
for k,(a,b) in div.items():
    ORACLE[k] = R(a,b)
ORACLE['d06_one_over_near1_38nines'] = R(10**38, 10**38-1); LABEL['d06_one_over_near1_38nines'] = '10^38/(10^38-1)'
ORACLE['d07_one_over_near1_39nines'] = R(10**39, 10**39-1); LABEL['d07_one_over_near1_39nines'] = '10^39/(10^39-1)'
ORACLE['d10_one_over_1_plus_1e38'] = R(10**38, 10**38+1); LABEL['d10_one_over_1_plus_1e38'] = '10^38/(10^38+1)'
ORACLE['d11_one_over_1_plus_tiny'] = R(10**39, 10**39+1); LABEL['d11_one_over_1_plus_tiny'] = '10^39/(10^39+1)'
ORACLE['d29_one_over_neg_1_0000001'] = R(-10**7, 10**7+1); LABEL['d29_one_over_neg_1_0000001'] = '-10^7/(10^7+1)'
ORACLE['d31_one_over_0_999999999999999999999999999999999999'] = R(10**36, 10**36-1); LABEL['d31_one_over_0_999999999999999999999999999999999999'] = '10^36/(10^36-1)'
ORACLE['d41_div_zero'] = None; LABEL['d41_div_zero'] = 'zoo (undefined)'
ORACLE['d42_div_zero_point_zero'] = None; LABEL['d42_div_zero_point_zero'] = 'zoo (undefined)'
# --- batch2: identity + precision ---
identity2 = ['m05_one_over_19_times_19','m06_one_over_0_85_times_0_85','m07_one_over_0_75_times_0_75',
 'm08_one_over_0_65_times_0_65','m09_one_over_0_55_times_0_55','m10_one_over_0_45_times_0_45',
 'm11_one_over_0_35_times_0_35','m12_one_over_0_25_times_0_25','m13_one_over_0_15_times_0_15',
 'm14_one_over_0_05_times_0_05','m15_one_over_0_99_times_0_99','m16_one_over_0_98_times_0_98',
 'm17_one_over_0_97_times_0_97','m18_one_over_0_96_times_0_96','m19_one_over_0_94_times_0_94',
 'm20_one_over_0_90_times_0_90','m21_one_over_0_8_times_0_8','m22_one_over_0_6_times_0_6',
 'm23_one_over_0_3_times_0_3','m24_one_over_0_2_times_0_2','m25_one_over_0_1_times_0_1',
 'm01_repro_one_over_0_95_times_0_95','m02_swapped_0_95_times_recip','m03_bound_then_multiply',
 'm04_exact_rat_20_19_times_19_20','m28_one_over_13_times_13','m29_one_over_17_times_17',
 'm30_one_over_23_times_23','m31_one_over_29_times_29','m32_one_over_31_times_31',
 'm33_one_over_37_times_37','m34_one_over_97_times_97','m35_one_over_0_9999_times_0_9999',
 'm36_evalf_bad_product_30','m37_evalf_bad_product_50','m39_add_thirds','m40_add_sevenths',
 'm41_one_over_3_times_3_again','m45_neg_recip_times','m44_one_over_0_95_times_0_95_div_2']
for k in identity2:
    ORACLE[k] = Integer(1)
ORACLE['m26_one_over_3_times_2'] = R(2,3)
ORACLE['m27_two_over_3_times_3'] = Integer(2)
ORACLE['m38_bad_product_minus_one'] = Integer(0)
ORACLE['m42_compare_product_to_one'] = True
ORACLE['m43_one_over_0_95_plus_1_over_0_95'] = R(40,19)
ORACLE['m45_neg_recip_times'] = Integer(-1)
ORACLE['m44_one_over_0_95_times_0_95_div_2'] = R(1,2)
for k in ['m46_evalf_one_third_38','m47_evalf_one_third_50','m48_evalf_one_third_51',
          'm49_evalf_one_third_60','m50_evalf_one_third_100']:
    ORACLE[k] = R(1,3); LABEL[k] = '1/3 to the requested digits'
ORACLE['m51_evalf_sqrt2_60'] = sqrt(2); LABEL['m51_evalf_sqrt2_60'] = 'sqrt(2) to 100 decimals (checked)'
ORACLE['m52_evalf_one_seventh_60'] = R(1,7)
ORACLE['m53_evalf_guard_9_tail'] = R(1,10**51)
ORACLE['m54_evalf_2_over_3_60'] = R(2,3)
# --- batch3: pow / edge ---
ORACLE['p01_zero_pow_neg1'] = None; LABEL['p01_zero_pow_neg1'] = 'zoo'
ORACLE['p02_zero_pow_zero'] = Integer(1)
ORACLE['p03_zero_pow_zero_real'] = Integer(1)
ORACLE['p04_zero_pow_neg2'] = None; LABEL['p04_zero_pow_neg2'] = 'zoo'
ORACLE['p05_zero_pow_half'] = Integer(0)
ORACLE['p06_zero_pow_real_neg1'] = None; LABEL['p06_zero_pow_real_neg1'] = 'zoo'
ORACLE['p07_neg8_pow_third'] = None; LABEL['p07_neg8_pow_third'] = '2*(-1)^(1/3) = 1+1.732i'
ORACLE['p08_neg8_pow_real_third'] = None; LABEL['p08_neg8_pow_real_third'] = '~1+1.732i'
ORACLE['p09_neg8_pow_half'] = None; LABEL['p09_neg8_pow_half'] = 'sqrt(-8) = 2.828i'
ORACLE['p10_neg2_pow_3_5'] = None; LABEL['p10_neg2_pow_3_5'] = '(-2)^3.5 = -8*sqrt(2)*i'
ORACLE['p11_neg8_pow_neg_third'] = None; LABEL['p11_neg8_pow_neg_third'] = '(-8)^(-1/3) = -0.5 (real root)'
ORACLE['p12_neg1_pow_half'] = None; LABEL['p12_neg1_pow_half'] = 'i'
ORACLE['p13_two_pow_100000'] = Integer(2**100000); LABEL['p13_two_pow_100000'] = '2^100000 (30103 digits; engine string compared EQUAL)'
ORACLE['p14_two_pow_neg_100000'] = R(1, 2**100000); LABEL['p14_two_pow_neg_100000'] = '1.2418e-30103'
ORACLE['p15_ten_pow_40'] = Integer(10**40)
ORACLE['p16_ten_pow_neg_40'] = R(1,10**40)
ORACLE['p34_cube_root_negative_real'] = None; LABEL['p34_cube_root_negative_real'] = 'real_root(-27,3) = -3'
for k in ['p17_two_pow_half','p18_two_pow_half_30','p19_two_pow_half_50']:
    ORACLE[k] = None; LABEL[k] = 'sqrt(2) = 1.4142...'
ORACLE['p20_inf_only'] = None; LABEL['p20_inf_only'] = 'oo'
ORACLE['p21_inf_minus_inf'] = None; LABEL['p21_inf_minus_inf'] = 'nan'
ORACLE['p22_zero_times_inf'] = None; LABEL['p22_zero_times_inf'] = 'nan'
ORACLE['p23_inf_plus_inf'] = None; LABEL['p23_inf_plus_inf'] = 'oo'
ORACLE['p24_inf_times_neg'] = None; LABEL['p24_inf_times_neg'] = '-oo'
ORACLE['p25_one_over_inf'] = None; LABEL['p25_one_over_inf'] = '0'
ORACLE['p26_ten_pow_38'] = Integer(10**38)
ORACLE['p27_ten_pow_39'] = Integer(10**39)
ORACLE['p28_pow_guard_9999999999'] = (R(1) - R(1,10**38))**2; LABEL['p28_pow_guard_9999999999'] = '(1-10^-38)^2'
ORACLE['p29_pow_neg1_nearzero'] = Integer(10**38)
ORACLE['p30_pow_half_negative_base_frac'] = None; LABEL['p30_pow_half_negative_base_frac'] = '2i'
ORACLE['p31_pow_periodic_base'] = R(1,9)
ORACLE['p32_pow_periodic_base_neg1'] = Integer(3)
ORACLE['p33_pow_periodic_base_neg2'] = Integer(9)
ORACLE['p35_pow_two_precisions_third'] = -(R(1,9) - R(int('1'*20), 10**20)) + (R(1,9) - R(int('1'*40), 10**40))
ORACLE['p36_exp_zero'] = Integer(1)
ORACLE['p37_ln_zero'] = None; LABEL['p37_ln_zero'] = '-oo (ln is not a builtin here)'
ORACLE['p38_ln_neg'] = None; LABEL['p38_ln_neg'] = 'i*pi (ln is not a builtin here)'
ORACLE['p39_sqrt_zero'] = Integer(0)
ORACLE['p40_sqrt_neg_zero'] = Integer(0)
# --- batch4 ---
for k,v,l in [('n01_two_pow_neg1', R(1,2), None), ('n02_two_pow_neg2', R(1,4), None),
              ('n03_two_pow_neg10', R(1,2**10), None), ('n04_two_pow_neg100', R(1,2**100), '2^-100 (100 decimals)'),
              ('n05_two_pow_neg500', R(1,2**500), '2^-500'), ('n06_two_pow_neg1000', R(1,2**1000), '2^-1000 (1000 decimals)'),
              ('n07_two_pow_neg1001', R(1,2**1001), '2^-1001 (needs 1001 decimals)'),
              ('n08_two_pow_neg5000', R(1,2**5000), '7.0798e-1506'),
              ('n09_two_pow_neg30103', R(1,2**30103), '1.2418e-9062'),
              ('n10_one_over_two_pow_1000', R(1,2**1000), '2^-1000'),
              ('n11_one_over_two_pow_1001', R(1,2**1001), '2^-1001'),
              ('n12_ten_pow_neg1001', R(1,10**1001), '1.0e-1001'),
              ('n13_ten_pow_neg1000', R(1,10**1000), '1.0e-1000'),
              ('n14_ten_pow_neg999', R(1,10**999), '1.0e-999'),
              ('n15_evalf_two_pow_neg100', R(1,2**100), '2^-100'),
              ('n16_evalf_two_pow_neg30103', R(1,2**30103), '1.2418e-9062'),
              ('g01_one_over_37nines', R(10**37,10**37-1), '10^37/(10^37-1)'),
              ('g02_parse_one_with_38zeros', Integer(1), '1'),
              ('g03_parse_38nines', R(10**38-1,10**38), '(10^38-1)/10^38'),
              ('g04_parse_38nines_then_zero', R(10**38-1,10**38), '(10^38-1)/10^38'),
              ('g05_parse_39sig_ends9', R(int('123456789012345678901234567890123456789'),10), '1234...678.9'),
              ('g06_parse_39sig_ends0', Integer(int('12345678901234567890123456789012345678')), '1234...678'),
              ('g07_evalf_third_37', R(1,3), '1/3'), ('g08_evalf_third_39', R(1,3), '1/3'),
              ('g09_evalf_sixth_40', R(1,6), '1/6'), ('g10_evalf_ninth_40', R(1,9), '1/9'),
              ('g11_evalf_1998_40', R(1,998001), '1/998001 (first 1001 decimals checked)'),
              ('k01_four_thirds_times_three_quarters', Integer(1), None),
              ('k02_one_over_six_times_six', Integer(1), None), ('k03_one_over_nine_times_nine', Integer(1), None),
              ('k04_five_sixths_times_six', Integer(5), None), ('k05_two_over_17_times_17', Integer(2), None),
              ('k06_three_over_seven_times_seven', Integer(3), None),
              ('k07_seventeen_times_one_over_17_var', Integer(1), None),
              ('k18_bad_product_div_self', Integer(1), None), ('k19_bad_product_sqrt', Integer(1), None),
              ('k20_bad_product_squared', Integer(1), None),
              ('k15_one_over_0_75_only', R(4,3), None), ('k16_one_over_0_75_plus_itself', R(8,3), None),
              ('k17_quarter_recips', Integer(16), None), ('k30_none', None, None)]:
    if k == 'k30_none':
        continue
    ORACLE[k] = v
    if l: LABEL[k] = l
ORACLE['k10_bad_product_scaled'] = Integer(1000000)
ORACLE['k11_bad_17_product_plus_zero'] = Integer(1)
ORACLE['k12_bad_17_product_times_17'] = Integer(17)
ORACLE['k13_evalf_bad_17_product_30'] = Integer(1)
ORACLE['k14_evalf_bad_17_product_40'] = Integer(1)
ORACLE['k05_two_over_17_times_17'] = Integer(2)
ORACLE['k09_bad_product_eq_one_75'] = False
LITS = R(int('1'+'3'*37), 10**37)
ORACLE['i29_long_literal_times_0_75'] = LITS * R(3,4)
ORACLE['i39_one_over_0_75_minus_three_quarters'] = R(4,3) - LITS
# --- batch5 ---
for k in ['i01_one_over_0_7_times_0_7','i02_one_over_0_71_times_0_71','i03_one_over_0_91_times_0_91',
          'i04_one_over_0_87_times_0_87','i05_one_over_0_63_times_0_63','i06_one_over_0_49_times_0_49',
          'i07_one_over_0_21_times_0_21','i08_one_over_0_13_times_0_13','i09_one_over_0_11_times_0_11',
          'i10_one_over_0_09_times_0_09','i11_one_over_0_07_times_0_07','i12_one_over_0_03_times_0_03',
          'i13_one_over_0_01_times_0_01','i14_one_over_0_999_times_0_999','i15_one_over_0_99999_times',
          'i20_alt_bracket_0_75','i26_one_over_0_750_times','i27_one_over_0_75_times_three_quarters',
          'i28_four_thirds_times_0_75','i31_one_over_12_times_12','i32_one_over_14_times_14',
          'i33_one_over_18_times_18','i34_one_over_22_times_22','i35_one_over_26_times_26']:
    ORACLE[k] = Integer(1)
ORACLE['i16_swapped_0_75'] = R(3,4)
ORACLE['i17_seven_over_0_75_times'] = Integer(7)
ORACLE['i18_seven_over_0_95_times'] = Integer(7)
ORACLE['i19_123_over_0_97_times'] = Integer(123)
for k in ['i21_evalf_four_thirds_product_20','i22_evalf_four_thirds_product_40','i23_evalf_four_thirds_product_19',
          'i25_four_thirds_product_minus_one','i38_zero_times_bad']:
    ORACLE[k] = Integer(0) if k == 'i25_four_thirds_product_minus_one' or k == 'i38_zero_times_bad' else Integer(1)
ORACLE['i24_four_thirds_product_eq_one'] = False
ORACLE['i30_one_over_6_times_3'] = R(1,2)
ORACLE['i36_one_over_0_7_times_0_7_eq'] = True
ORACLE['i37_one_over_0_75_times_1_5'] = Integer(2)
ORACLE['k08_bad_product_minus_one_75'] = Integer(0)

# ---------------------------------------------------------------- verdicts
F = {
 'F1': ['d36_identity_one_over_0_95_times_0_95','m01_repro_one_over_0_95_times_0_95','m02_swapped_0_95_times_recip',
        'm03_bound_then_multiply','m04_exact_rat_20_19_times_19_20','m09_one_over_0_55_times_0_55',
        'm19_one_over_0_94_times_0_94','m45_neg_recip_times','i34_one_over_22_times_22'],
 'F2': ['m07_one_over_0_75_times_0_75','m11_one_over_0_35_times_0_35','m16_one_over_0_98_times_0_98',
        'm17_one_over_0_97_times_0_97','m18_one_over_0_96_times_0_96','m29_one_over_17_times_17',
        'm30_one_over_23_times_23','m34_one_over_97_times_97','k01_four_thirds_times_three_quarters',
        'k05_two_over_17_times_17','k07_seventeen_times_one_over_17_var','k10_bad_product_scaled',
        'k11_bad_17_product_plus_zero','k12_bad_17_product_times_17','k13_evalf_bad_17_product_30',
        'k14_evalf_bad_17_product_40','k19_bad_product_sqrt','k20_bad_product_squared',
        'i02_one_over_0_71_times_0_71','i06_one_over_0_49_times_0_49','i16_swapped_0_75',
        'i17_seven_over_0_75_times','i19_123_over_0_97_times','i20_alt_bracket_0_75',
        'i21_evalf_four_thirds_product_20','i22_evalf_four_thirds_product_40','i23_evalf_four_thirds_product_19',
        'i25_four_thirds_product_minus_one','i26_one_over_0_750_times','i27_one_over_0_75_times_three_quarters',
        'i28_four_thirds_times_0_75','i35_one_over_26_times_26'],
 'F3': ['m36_evalf_bad_product_30','m37_evalf_bad_product_50'],
 'F4': ['m38_bad_product_minus_one','m44_one_over_0_95_times_0_95_div_2'],
 'F5': ['p06_zero_pow_real_neg1'],
 'F6': ['p07_neg8_pow_third','p08_neg8_pow_real_third','p10_neg2_pow_3_5','p11_neg8_pow_neg_third',
        'p17_two_pow_half','p18_two_pow_half_30','p19_two_pow_half_50','p29_pow_neg1_nearzero',
        'p32_pow_periodic_base_neg1','p33_pow_periodic_base_neg2','p34_cube_root_negative_real'],
 'F7': ['p14_two_pow_neg_100000','n08_two_pow_neg5000','n09_two_pow_neg30103','n12_ten_pow_neg1001',
        'n16_evalf_two_pow_neg30103'],
}
INCONCLUSIVE = ['p21_inf_minus_inf','p22_zero_times_inf','p23_inf_plus_inf','p25_one_over_inf']
VERDICT = {}
for f, names in F.items():
    for n in names:
        assert n in PROBES, n
        VERDICT[n] = f
for n in INCONCLUSIVE:
    assert n in PROBES, n
    VERDICT[n] = 'INC'
missing = [n for n in ORDER if n not in VERDICT]
for n in missing:
    VERDICT[n] = 'HELD'

# ---------------------------------------------------------------- observed summary
def summarize(entry):
    try:
        o = json.loads(entry.get('stdout', ''))
    except Exception:
        return 'UNPARSEABLE: ' + entry.get('stdout', '')[:60]
    if not o.get('ok'):
        return f"ERROR `{o.get('code')}` — \"{o.get('message')}\" (exit {entry.get('exit','?')})"
    r = o['result']; s = r.get('structured', {})
    kind = r.get('kind')
    disp = str(r.get('display', ''))
    if kind == 'Boolean':
        return f"Boolean `{s.get('value')}`"
    if len(disp) <= 52:
        body = disp
    else:
        body = disp[:22] + '…' + disp[-10:] + f' [{len(disp)}ch]'
    flag = s.get('exact')
    exact = '' if flag is None else f' exact:`{str(flag).lower()}`'
    return f'{kind} `{body}`{exact}'

def oracle_str(name):
    if name in LABEL:
        return LABEL[name]
    v = ORACLE.get(name, 'MISSING')
    if v == 'MISSING':
        return '—'
    if v is None:
        return '—'
    if isinstance(v, bool):
        return str(v).lower()
    s = str(v)
    if len(s) > 26:
        return f'{s[:14]}… = {N(v, 18)}'
    if v == int(v) if hasattr(v, '__int__') and not isinstance(v, bool) else False:
        return s
    try:
        if v.q != 1:
            return f'{s} = {N(v, 18)}'
    except Exception:
        pass
    return s

rows = []
for name in ORDER:
    e = PROBES[name]
    v = VERDICT[name]
    if v == 'HELD':
        verdict = 'HELD'
    elif v == 'INC':
        verdict = 'INCONCLUSIVE'
    else:
        verdict = f'**FINDING {v}**'
    rows.append(f"| {name} | `EXE --file …/probes/{e['batch']}/{name}.ls --omit-functions` | {summarize(e)} | {oracle_str(name)} | {verdict} |")

table = '| probe | command | observed | SymPy value (if applicable) | verdict |\n|---|---|---|---|---|\n' + '\n'.join(rows)

counts = {}
for name in ORDER:
    counts[VERDICT[name]] = counts.get(VERDICT[name], 0) + 1
held = counts.get('HELD', 0); inc = counts.get('INC', 0)
find = sum(v for k, v in counts.items() if k.startswith('F'))
lines = [f"**{len(ORDER)} probes: {held} HELD, {find} FINDING, {inc} INCONCLUSIVE.**", ""]
lines.append('| finding | probes | one line |')
lines.append('|---|---|---|')
lines.append(f"| F1 | {len(F['F1'])} | `(a/b)*b` returns a wrong rational that the envelope marks `\"exact\":true` |")
lines.append(f"| F2 | {len(F['F2'])} | `(a/b)*b` returns a wrong rational (flagged `exact:false`) |")
lines.append(f"| F3 | {len(F['F3'])} | `evalf` of an F1 value returns nines instead of 1 at the requested precision |")
lines.append(f"| F4 | {len(F['F4'])} | the F1 value behaves as exactly 1 in `-1` and `/2` but `== 1` is false |")
lines.append(f"| F5 | {len(F['F5'])} | `0^(-1.0)` returns 0 with `\"exact\":true` while `0^-1` is an error |")
lines.append(f"| F6 | {len(F['F6'])} | `^` refuses negative/fractional exponents it elsewhere supports |")
lines.append(f"| F7 | {len(F['F7'])} | values needing >1000 fractional digits return 0 with `\"exact\":true` |")
counts_block = '\n'.join(lines)

def parse_value(entry):
    o = json.loads(entry.get('stdout', ''))
    s = o['result'].get('structured', {})
    if 'numerator' in s and s['numerator'] not in (None, ''):
        try:
            return R(Integer(int(s['numerator'])), Integer(int(s['denominator'])))
        except Exception:
            return None
    txt = str(o['result'].get('display', ''))
    m = re.fullmatch(r'(-?)(\d+)(?:\.(\d*)(?:\((\d+)\))?)?', txt)
    if not m:
        return None
    sign, ip, frac, per = m.group(1), m.group(2), m.group(3) or '', m.group(4)
    val = R(int(ip))
    if frac:
        val += R(int(frac), 10**len(frac))
    if per:
        val += R(int(per), (10**len(per) - 1) * 10**len(frac))
    return -val if sign == '-' else val

# exact:true audit
VERIFIED_SYMBOLIC = {'p12_neg1_pow_half': 'i', 'p30_pow_half_negative_base_frac': '2*i'}
exact_true, exact_true_wrong, exact_true_ok, unproven = [], [], [], []
for name in ORDER:
    e = PROBES[name]
    try:
        o = json.loads(e.get('stdout', ''))
    except Exception:
        continue
    if not o.get('ok'):
        continue
    s = o['result'].get('structured', {})
    if s.get('exact') is not True:
        continue
    exact_true.append(name)
    if VERDICT[name] in ('F1', 'F5', 'F7'):
        exact_true_wrong.append(name)
    elif name in VERIFIED_SYMBOLIC:
        exact_true_ok.append(name)
    elif name in ORACLE and ORACLE[name] is not None and not isinstance(ORACLE[name], bool):
        got = parse_value(e)
        if got is not None:
            if got == ORACLE[name]:
                exact_true_ok.append(name)
            else:
                unproven.append(name)
        else:
            unproven.append(name)
    else:
        unproven.append(name)
audit = (f"**`exact:true` audit:** {len(exact_true)} probes returned `\"exact\":true`. "
         f"{len(exact_true_wrong)} of them are demonstrated wrong against SymPy "
         f"({', '.join(sorted(set(VERDICT[n] for n in exact_true_wrong)))}). "
         f"{len(exact_true_ok)} more were compared digit-for-digit to SymPy and are exactly right. "
         f"{len(VERIFIED_SYMBOLIC)} are symbolic results (`i`, `2i`) whose `exact:true` matches SymPy's "
         f"`I`/`2*I`. "
         + (f"{len(unproven)} could not be re-derived from the published envelope alone "
            f"({', '.join(unproven)})." if unproven else ''))

def parse_value(entry):
    o = json.loads(entry.get('stdout', ''))
    s = o['result'].get('structured', {})
    if 'numerator' in s and s['numerator'] not in (None, ''):
        try:
            return R(Integer(int(s['numerator'])), Integer(int(s['denominator'])))
        except Exception:
            return None
    txt = str(o['result'].get('display', ''))
    m = re.fullmatch(r'(-?)(\d+)(?:\.(\d*)(?:\((\d+)\))?)?', txt)
    if not m:
        return None
    sign, ip, frac, per = m.group(1), m.group(2), m.group(3) or '', m.group(4)
    val = R(int(ip))
    if frac:
        val += R(int(frac), 10**len(frac))
    if per:
        val += R(int(per), (10**len(per) - 1) * 10**len(frac))
    return -val if sign == '-' else val

prose = open(os.path.join(ROOT, 'report-prose.md'), encoding='utf-8').read()
out = prose.replace('<!--COUNTS-->', counts_block + '\n\n' + audit).replace('<!--TABLE-->', table)
head, tail = out.split('<!--PROSE-TAIL-->')
out = head + tail
open(os.path.join(ROOT, 'audit-P1-numeric.md'), 'w', encoding='utf-8', newline='\n').write(out)

print('probes:', len(ORDER))
print('counts:', {k: v for k, v in sorted(counts.items())})
print('exact:true probes:', len(exact_true), ' wrong:', len(exact_true_wrong), ' verified-right:', len(exact_true_ok), ' unproven:', len(unproven))
print('unproven list:', unproven)
print('missing oracle (—):', [n for n in ORDER if n not in ORACLE and n not in LABEL])
print('wrote audit-P1-numeric.md', len(out), 'chars')
