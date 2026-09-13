import mpmath as mp
mp.mp.dps = 2000
path = r'C:\Users\ricar\dev\.lovelace-audit-P\out\a1.txt'
txt = open(path, encoding='utf-8', errors='replace').read().splitlines()
def row(case, impl):
    cur = None
    for l in txt:
        if l.startswith('CASE '): cur = l.split('|')[0].replace('CASE','').strip()
        if cur == case and ('| ' + impl + ' |') in l: return l.split('|')[-1].strip()
    return None
for case, arg in (('sin(1)','1'), ('sin(10)','10')):
    v = row(case, 'Real.Sin(v)')
    if v is None: print(case, 'ROW NOT FOUND'); continue
    digits = v.replace('-','').replace('.','')
    true = mp.nstr(mp.sin(mp.mpf(arg)), 1900, strip_zeros=False).replace('-','').replace('.','')
    n = min(len(digits), len(true)); m = 0
    for i in range(n):
        if digits[i] == true[i]: m += 1
        else: break
    print(case, 'printed_digits=', len(digits), 'leading_correct=', m)
    print('   printed[:40]=', digits[:40])
    print('   mpmath [:40]=', true[:40])
    print('   around mismatch:', digits[max(0,m-5):m+15], 'vs', true[max(0,m-5):m+15])