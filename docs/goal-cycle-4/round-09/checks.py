# P1 audit: SymPy ground-truth checks for the values observed from the AOT binary.
# Run: & "C:\Users\ricar\dev\.lovelace-tools\python\python3.exe" checks.py <num> <den>
import sys
sys.set_int_max_str_digits(200000)
from sympy import Rational, N, Integer, sqrt, oo, nan, S, simplify

def show(label, value):
    print(f"{label} = {value}")

print("=== sympy version ===")
import sympy
print(sympy.__version__)

print()
print("=== 1. the d36 claim: (1/0.95)*0.95 as reported by the binary ===")
num = Integer(int(sys.argv[1]))
den = Integer(int(sys.argv[2]))
claimed = Rational(num, den)
print("reported numerator digits =", len(str(num)))
print("reported denominator digits =", len(str(den)))
print("reported numerator == 225*10^k - 1 ?", simplify(num - (den - 1)) == 0)
print("reported denominator == 225*10^k ?  ", den % 225 == 0 and set(str(den)) == {'0'} | set(str(den)[:3]))
print("claimed rational == 1 ?", claimed == 1)
print("1 - claimed =", 1 - claimed)
print("claimed as float (first 30 digits):", N(claimed, 30))
print("exact value of (1/Rational(95,100))*Rational(95,100) =", Rational(1, Rational(95,100)) * Rational(95,100))
print("exact value of Rational(20,19)*Rational(19,20)        =", Rational(20,19) * Rational(19,20))
print("difference from 1 (exact) =", 1 - Rational(20,19)*Rational(19,20))
print("claimed - 1 = (exact)     =", claimed - 1)

print()
print("=== 2. division boundary ground truth (SymPy exact rationals) ===")
cases = {
    "1/0.9": Rational(1, Rational(9,10)),
    "1/0.95": Rational(1, Rational(95,100)),
    "1/0.99": Rational(1, Rational(99,100)),
    "1/0.999": Rational(1, Rational(999,1000)),
    "1/0.9999": Rational(1, Rational(9999,10000)),
    "1/1.1": Rational(1, Rational(11,10)),
    "1/1.01": Rational(1, Rational(101,100)),
    "1/3": Rational(1,3),
    "1/11": Rational(1,11),
    "1/27": Rational(1,27),
    "1/7": Rational(1,7),
    "1/21": Rational(1,21),
    "1/101": Rational(1,101),
    "10/9": Rational(10,9),
    "100/99": Rational(100,99),
    "0.9/0.9999": Rational(9,10)/Rational(9999,10000),
    "1/(-1.0000001)": Rational(1, Rational(-10000001,10000000)),
}
for k, v in cases.items():
    print(f"{k:18s} = {v}   = {N(v, 40)}")

print()
print("=== 3. evalf precision probes (true digits) ===")
for d in (30, 38, 50, 51, 60, 100):
    print(f"1/3 to {d} digits: {N(Rational(1,3), d)}")
print("sqrt(2) to 60:", N(sqrt(2), 60))
print("1/7 to 60:  ", N(Rational(1,7), 60))
print("2^0.5 to 50:", N(sqrt(2), 50))

print()
print("=== 4. pow / edge ground truth ===")
print("0**-1 in SymPy =", S.Zero**-1)
print("0**0  in SymPy =", S.Zero**S.Zero)
print("oo - oo =", oo - oo)
print("0*oo    =", 0*oo)
print("(-8)**Rational(1,3) principal =", (-8)**Rational(1,3), "=", N((-8)**Rational(1,3), 20))
print("real cube root of -8 (real_root) =", sympy.real_root(-8, 3))
print("sqrt(-8) =", N(sqrt(-8), 20))
print("2**100000 digit count =", len(str(2**100000)))
print("2**-100000 =", N(Rational(1, 2**100000), 20))
print("(1/3)**-2 =", Rational(1,3)**-2)
print("(1/3)**2  =", N(Rational(1,3)**2, 30))
