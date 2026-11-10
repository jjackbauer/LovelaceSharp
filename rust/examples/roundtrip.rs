// ad-hoc round-trip probe
fn main() {
    let mut state: u64 = 0x1234_5678_9ABC_DEF0;
    let mut text = String::new();
    for i in 0..100_000usize {
        state ^= state << 13; state ^= state >> 7; state ^= state << 17;
        let d = (state % 10) as u8;
        if i == 0 { text.push((b'1' + d % 9) as char); } else { text.push((b'0' + d) as char); }
    }
    let value = lovelace_nat::Nat::from_decimal(&text).expect("parse");
    let back = value.to_decimal();
    println!("roundtrip_equal={}", back == text);
    println!("len_in={} len_out={}", text.len(), back.len());
    if back != text {
        for (i, (a, b)) in text.chars().zip(back.chars()).enumerate() {
            if a != b { println!("first mismatch at {i}: {a} vs {b}"); break; }
        }
    }
}
