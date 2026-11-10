// Verifies a large product against an independent oracle: dividing it back must return the
// other operand with zero remainder.
use lovelace_nat::Nat;

fn digits(n: usize, seed: u64) -> String {
    let mut state = seed | 1;
    let mut out = String::with_capacity(n);
    for i in 0..n {
        state ^= state << 13; state ^= state >> 7; state ^= state << 17;
        let d = (state % 10) as u8;
        if i == 0 { out.push((b'1' + d % 9) as char); } else { out.push((b'0' + d) as char); }
    }
    out
}

fn main() {
    for (da, db) in [(100_000usize, 50_000usize), (100_000, 100_000), (30_000, 15_000)] {
        let a = Nat::from_decimal(&digits(da, 12345)).expect("a");
        let b = Nat::from_decimal(&digits(db, 67890)).expect("b");
        let product = a.mul(&b);
        let (q, r) = product.div_rem(&b).expect("divide");
        let ok_q = format!("{}", q) == format!("{}", a);
        let ok_r = r.to_decimal() == "0";
        println!("{da}x{db}: product_digits={} quotient_ok={ok_q} remainder_zero={ok_r}", product.to_decimal().len());
    }
}
