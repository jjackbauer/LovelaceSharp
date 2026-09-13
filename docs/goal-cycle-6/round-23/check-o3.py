p = r'C:\Users\ricar\dev\LovelaceSharp\docs\goal-cycle-6\round-23\o3-midbom.ls'
s = open(p, encoding='utf-8').read()
print('len', len(s))
for i, ch in enumerate(s):
    print(i, 'U+%04X' % ord(ch), repr(ch))