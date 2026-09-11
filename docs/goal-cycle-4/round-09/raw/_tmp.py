import json,re,sys
sys.set_int_max_str_digits(200000)
from sympy import Rational,Integer,N
def load(p):
    out,cur={},None
    for line in open(p,encoding="utf-8"):
        line=line.rstrip("\n")
        if line.startswith("### "): cur=line[4:].strip()
        elif line.startswith("STDOUT: ") and cur:
            out[cur]=json.loads(line[8:]); cur=None
    return out
base=r"C:\Users\ricar\dev\LovelaceSharp\docs\goal-cycle-4\round-09\raw\\"
b={}
for n in ("batch1","batch2","batch3","batch4","batch5"): b.update(load(base+n+".txt"))
print("### p28 (0.999...99 38 nines)^2 vs SymPy")
d=b["p28_pow_guard_9999999999"]["result"]["display"]
t=str((Rational(1)-Rational(1,10**38))**2)
print("engine:",d)
print("len:",len(d))
print("sympy :",t[:len(d)])
print("prefix equal:",d==t[:len(d)])
print()
print("### p35 evalf((1/3)^2,20)-evalf((1/3)^2,40) vs SymPy")
d=b["p35_pow_two_precisions_third"]["result"]["display"]
t=-(Rational(1,9)-Rational(int("1"*20),10**20))+(Rational(1,9)-Rational(int("1"*40),10**40))
print("engine:",d)
print("sympy :",t)
print("equal:",Rational(d)==t)
print()
print("### false-exactness published rationals")
for nm in ("d36_identity_one_over_0_95_times_0_95","m01_repro_one_over_0_95_times_0_95","m09_one_over_0_55_times_0_55","m19_one_over_0_94_times_0_94","m45_neg_recip_times","i34_one_over_22_times_22"):
    s=b[nm]["result"]["structured"]
    if "numerator" in s:
        num=int(s["numerator"]); den=int(s["denominator"])
        print(f'{nm}: exact={s["exact"]} num_digits={len(str(abs(num)))} den_digits={len(str(den))} num=den-1? {num==den-1} num_head={str(abs(num))[:12]} den_head={str(den)[:12]} den_tail={str(den)[-6:]}')
        print(f'    1 - value = {N(1-Rational(num,den),6)}   value==1? {Rational(num,den)==1}')
        print(f'    display head/tail: {b[nm]["result"]["display"][:30]} ... {b[nm]["result"]["display"][-14:]}')
    else:
        print(f'{nm}: exact={s["exact"]} (no numerator; display {len(b[nm]["result"]["display"])} chars)')
