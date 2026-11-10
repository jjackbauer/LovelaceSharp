use std::env;
use std::fs;
use std::hint::black_box;
use std::time::Instant;

use lovelace_nat::Nat;

fn evaluate(op: &str, a: &Nat, b: &Nat, text: &str) -> (Nat, Option<Nat>) {
    match op {
        "mul" => (a.mul(b), None),
        "div" => {
            let (quotient, remainder) = a.div_rem(b).expect("nonzero divisor");
            (quotient, Some(remainder))
        }
        "parse" => (Nat::from_decimal(text).expect("decimal input"), None),
        _ => panic!("unknown operation {op}"),
    }
}

fn digest(text: &str) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in text.bytes() {
        hash = (hash ^ byte as u64).wrapping_mul(0x100000001b3);
    }
    hash
}

fn main() {
    let args: Vec<String> = env::args().collect();
    assert_eq!(args.len(), 5, "usage: audit-bench OP A_FILE B_FILE REPS");
    let op = &args[1];
    let a_text = fs::read_to_string(&args[2]).expect("read a");
    let b_text = fs::read_to_string(&args[3]).expect("read b");
    let reps: usize = args[4].parse().expect("reps");
    assert!(reps > 0);
    let a = Nat::from_decimal(a_text.trim()).expect("parse a");
    let b = Nat::from_decimal(b_text.trim()).expect("parse b");
    for _ in 0..3 {
        black_box(evaluate(op, &a, &b, a_text.trim()));
    }
    let mut times = Vec::with_capacity(reps);
    for _ in 0..reps {
        let start = Instant::now();
        let result = evaluate(op, &a, &b, a_text.trim());
        times.push(start.elapsed().as_secs_f64() * 1000.0);
        black_box(result);
    }
    times.sort_by(|a, b| a.total_cmp(b));
    let (value, remainder) = evaluate(op, &a, &b, a_text.trim());
    let value_text = value.to_decimal();
    let remainder_text = remainder.map(|v| v.to_decimal()).unwrap_or_default();
    let mean = times.iter().sum::<f64>() / times.len() as f64;
    let median = times[times.len() / 2];
    println!(
        "AUDIT Rust {op} mean={mean:.3}ms median={median:.3}ms len={} fnv={:016x} rem_len={} rem_fnv={:016x}",
        value_text.len(), digest(&value_text), remainder_text.len(), digest(&remainder_text)
    );
}
