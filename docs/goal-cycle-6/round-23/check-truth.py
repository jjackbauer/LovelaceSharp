import mpmath as mp
mp.mp.dps = 120
def trunc(x, n):
    s = mp.nstr(x, n + 25, strip_zeros=False)
    d = s.split('.')[1]
    return s.split('.')[0] + '.' + d[:n]
print('sin1_30  =', trunc(mp.sin(1), 30))
print('sin1_50  =', trunc(mp.sin(1), 50))
print('sin1_10  =', trunc(mp.sin(1), 10))
print('cos1_30  =', trunc(mp.cos(1), 30))
print('sin1e-5_30 =', trunc(mp.sin(mp.mpf('0.00001')), 30))