# A first Lovelace script.
# Ctrl+Enter runs it; results land in the logs and workspace panes.

v = 1..5
print("squares:", v ^ 2)
sum(v ^ 2)

m = [[1, 2], [3, 4]]
det(m)

1 / 3

x = symbol("x")
solve(x^2 - 4 == 0, x)
