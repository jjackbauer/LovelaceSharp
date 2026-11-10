#!/usr/bin/env python3
"""Deterministic shared decimal operands for cross-runtime numerical benchmarks."""

from pathlib import Path
import random

root = Path(__file__).resolve().parents[2]
destination = root / "audit-data"
destination.mkdir(exist_ok=True)

for name, count, seed in [
    ("a.txt", 100_000, 12345),
    ("b_half.txt", 50_000, 67890),
    ("b_quarter.txt", 25_000, 99999),
]:
    randomizer = random.Random(seed)
    digits = str(randomizer.randint(1, 9)) + "".join(
        str(randomizer.randint(0, 9)) for _ in range(count - 1)
    )
    (destination / name).write_text(digits, encoding="ascii")
    print(f"INPUT {name} {count} digits")
