use lovelace_nat::Nat;
use std::time::Instant;

fn digits(n: usize, seed: u64) -> String {
    let mut s = seed | 1;
    let mut o = String::with_capacity(n);
    for i in 0..n {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        let d = (s % 10) as u8;
        if i == 0 {
            o.push((b'1' + d % 9) as char)
        } else {
            o.push((b'0' + d) as char)
        }
    }
    o
}

fn main() {
    for (nd, dd) in [(100_000usize, 25_000usize), (300_000, 75_000)] {
        let n = Nat::from_decimal(&digits(nd, 12345)).unwrap();
        let d = Nat::from_decimal(&digits(dd, 99999)).unwrap();
        let t = Instant::now();
        let knuth = lovelace_nat::knuth_probe(&n, &d).unwrap();
        let knuth_ms = t.elapsed().as_secs_f64() * 1000.0;
        let t = Instant::now();
        let bz = lovelace_nat::bz_probe(&n, &d);
        let bz_ms = t.elapsed().as_secs_f64() * 1000.0;
        let t = Instant::now();
        let nt = lovelace_nat::newton_probe(&n, &d);
        let nt_ms = t.elapsed().as_secs_f64() * 1000.0;
        let ok_n = matches!(&nt, Some((q, r)) if *q == knuth.0 && *r == knuth.1);
        let ok_b = matches!(&bz, Some((q, r)) if *q == knuth.0 && *r == knuth.1);
        println!(
            "{nd}/{dd}: knuth {knuth_ms:.1}  bz {bz_ms:.1} ({ok_b})  newton {nt_ms:.1} ({ok_n})"
        );
    }
}
