x = symbol("x", real)
applied = abs(x)
roundtrip = simplify_full(sqrt(x^2)).expression
print(inspect(applied))
print(inspect(roundtrip))
print(subs(abs(x), x, -3))
print(abs(-3))
print(diff(abs(x), x))
