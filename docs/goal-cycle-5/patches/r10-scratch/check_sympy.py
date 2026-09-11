import sys
sys.set_int_max_str_digits(300000)
import sympy
from sympy import Integer, Rational
print("sympy", sympy.__version__)

d = Integer(2)**100000
r = Rational(1, d)
print("2**-100000 numerator  =", r.p)
print("2**-100000 denominator digits =", len(str(r.q)))
print("den head =", str(r.q)[:30])
print("den tail =", str(r.q)[-30:])

with open(r"C:/Users/ricar/dev/LovelaceSharp/.worktrees/c5-realexact2/c5-scratch/out-den.txt") as f:
    den = f.read().strip()
with open(r"C:/Users/ricar/dev/LovelaceSharp/.worktrees/c5-realexact2/c5-scratch/out-value.txt") as f:
    val = f.read().strip()

print("RUNNER denominator MATCHES sympy 2**100000 :", den == str(r.q))
sig = Integer(5)**100000
exact = "0." + "0" * (100000 - len(str(sig))) + str(sig)
print("RUNNER value MATCHES sympy exact decimal   :", val == exact)
print("sympy N(1/2**100000, 40) =", r.evalf(40))
print("2**-5000 denominator head/tail =", str(Integer(2)**5000)[:20], str(Integer(2)**5000)[-20:], "digits", len(str(Integer(2)**5000)))
print("10**-1001 denominator digits =", len(str(Integer(10)**1001)))
