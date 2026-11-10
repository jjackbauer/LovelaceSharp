//! Micro-benchmark harness shaped like the C# reference harness, so the two can be compared
//! on one machine with the same operations and the same operand sizes.
//!
//! Usage: lovelace-bench <op> <digits> [reps]
//!   op: add | sub | mul | div | factorial | tostring | parse | pi | sqrt
//! Output: "RESULT <op> <digits> <mean_ms> <median_ms>" plus a DIGEST line so two runs can be
//! compared byte for byte (used to prove that changing the thread budget changes nothing).

use std::time::Instant;

use lovelace_nat::Nat;
use lovelace_real::Real;

fn random_digits(n: usize, seed: u64) -> String {
    let mut state = seed | 1;
    let mut out = String::with_capacity(n);
    for i in 0..n {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        let digit = (state % 10) as u8;
        if i == 0 {
            out.push((b'1' + (digit % 9)) as char);
        } else {
            out.push((b'0' + digit) as char);
        }
    }
    out
}

enum Work {
    Add,
    Sub,
    Mul,
    Div,
    Factorial,
    ToString,
    ToStringCold,
    Parse,
    Pi,
    Sqrt,
}

impl Work {
    fn parse(op: &str) -> Option<Work> {
        Some(match op {
            "add" => Work::Add,
            "sub" => Work::Sub,
            "mul" => Work::Mul,
            "div" => Work::Div,
            "factorial" => Work::Factorial,
            "tostring" => Work::ToString,
            "tostring_cold" => Work::ToStringCold,
            "parse" => Work::Parse,
            "pi" => Work::Pi,
            "sqrt" => Work::Sqrt,
            _ => return None,
        })
    }

    /// Runs the operation once. Returns a digest of the result for the last call, so a caller
    /// can compare runs without timing the conversion.
    fn run(&self, digits: usize, a: &Nat, b: &Nat, want_digest: bool) -> Option<String> {
        match self {
            Work::Add => {
                let _ = std::hint::black_box(a.add(b));
                None
            }
            Work::Sub => {
                let (big, small) = if a.cmp(b) == std::cmp::Ordering::Greater {
                    (a.clone(), b.clone())
                } else {
                    (b.clone(), a.clone())
                };
                let _ = std::hint::black_box(big.sub(&small));
                None
            }
            Work::Mul => {
                let value = std::hint::black_box(a.mul(b));
                if want_digest { Some(value.to_decimal()) } else { None }
            }
            Work::Div => {
                let divisor = Nat::from_decimal(&random_digits((digits / 4).max(1), 99999)).expect("c");
                let pair = std::hint::black_box(a.div_rem(&divisor));
                if want_digest { pair.map(|(q, _)| q.to_decimal()) } else { None }
            }
            Work::Factorial => {
                let value = std::hint::black_box(Nat::factorial(digits as u64));
                if want_digest { Some(value.to_decimal()) } else { None }
            }
            Work::ToString => {
                let text = std::hint::black_box(a.to_decimal());
                if want_digest { Some(text) } else { None }
            }
            Work::ToStringCold => {
                // Force a value that exists only as limbs (arithmetic materialises them), so the
                // rendering below is a real conversion rather than a string already in hand.
                let fresh = a.add(&Nat::from_u64(1));
                let text = std::hint::black_box(fresh.to_decimal());
                if want_digest { Some(text) } else { None }
            }
            Work::Parse => {
                let text = random_digits(digits, 4242);
                let value = std::hint::black_box(Nat::from_decimal(&text).unwrap());
                if want_digest { Some(value.to_decimal()) } else { None }
            }
            Work::Pi => {
                let value = std::hint::black_box(Real::pi(digits)).to_string();
                if want_digest { Some(value) } else { None }
            }
            Work::Sqrt => {
                let value = std::hint::black_box(Real::from_i64(2).sqrt(digits)).to_string();
                if want_digest { Some(value) } else { None }
            }
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() < 2 {
        eprintln!("usage: lovelace-bench <op> <digits> [reps]");
        std::process::exit(2);
    }
    let op = args[0].clone();
    let digits: usize = args[1].parse().expect("digits");
    let reps: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(20);
    let Some(work) = Work::parse(&op) else {
        eprintln!("unknown op {op}");
        std::process::exit(2);
    };

    let a = Nat::from_decimal(&random_digits(digits, 12345)).expect("a");
    let b = Nat::from_decimal(&random_digits((digits / 2).max(2), 67890)).expect("b");

    // Warm up outside the timed region and capture the digest there, so conversion cost never
    // pollutes the measurement.
    let digest = work.run(digits, &a, &b, true);

    let mut times: Vec<f64> = Vec::with_capacity(reps);
    for _ in 0..reps {
        let start = Instant::now();
        let _ = work.run(digits, &a, &b, false);
        times.push(start.elapsed().as_secs_f64() * 1000.0);
    }
    times.sort_by(|x, y| x.partial_cmp(y).unwrap());
    let mean = times.iter().sum::<f64>() / times.len() as f64;
    let median = times[times.len() / 2];
    println!("RESULT {op} {digits} {mean:.3} {median:.3}");
    if let Some(text) = digest {
        let head: String = text.chars().take(24).collect();
        println!("DIGEST {op} {}:{head}", text.len());
    }
}
