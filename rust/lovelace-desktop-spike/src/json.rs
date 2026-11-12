//! Tiny dependency-free JSON writer, so the bench line is produced without serde.

pub fn quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Formats a float with 3 decimals; non-finite values become `null` so the line stays valid JSON.
pub fn num(v: f64) -> String {
    if !v.is_finite() {
        "null".to_string()
    } else {
        format!("{v:.3}")
    }
}

#[derive(Default, Clone)]
pub struct Json {
    s: String,
}

impl Json {
    pub fn new() -> Self {
        Json { s: String::from("{") }
    }

    fn sep(&mut self) {
        if !self.s.ends_with('{') {
            self.s.push(',');
        }
    }

    fn key(&mut self, k: &str) {
        self.s.push_str(&quote(k));
        self.s.push(':');
    }

    /// Insert a pre-serialised JSON value.
    pub fn raw(&mut self, k: &str, v: &str) -> &mut Self {
        self.sep();
        self.key(k);
        self.s.push_str(v);
        self
    }

    pub fn str(&mut self, k: &str, v: &str) -> &mut Self {
        self.raw(k, &quote(v))
    }

    pub fn int(&mut self, k: &str, v: u64) -> &mut Self {
        self.raw(k, &v.to_string())
    }

    pub fn usize(&mut self, k: &str, v: usize) -> &mut Self {
        self.raw(k, &v.to_string())
    }

    pub fn f(&mut self, k: &str, v: f64) -> &mut Self {
        self.raw(k, &num(v))
    }

    pub fn bool(&mut self, k: &str, v: bool) -> &mut Self {
        self.raw(k, if v { "true" } else { "false" })
    }

    pub fn obj(&mut self, k: &str, j: Json) -> &mut Self {
        self.raw(k, &j.finish())
    }

    pub fn arr_f(&mut self, k: &str, vs: &[f64]) -> &mut Self {
        let body = vs.iter().map(|v| num(*v)).collect::<Vec<_>>().join(",");
        self.raw(k, &format!("[{body}]"))
    }

    pub fn arr_str(&mut self, k: &str, vs: &[String]) -> &mut Self {
        let body = vs
            .iter()
            .map(|v| quote(v))
            .collect::<Vec<_>>()
            .join(",");
        self.raw(k, &format!("[{body}]"))
    }

    pub fn finish(mut self) -> String {
        self.s.push('}');
        self.s
    }
}

/// p50 / p95 / max of a sample set, sorted in place.
pub fn percentiles(samples: &mut [f64]) -> (f64, f64, f64) {
    if samples.is_empty() {
        return (f64::NAN, f64::NAN, f64::NAN);
    }
    samples.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let pick = |q: f64| {
        let idx = ((samples.len() - 1) as f64 * q).round() as usize;
        samples[idx.min(samples.len() - 1)]
    };
    (pick(0.50), pick(0.95), samples[samples.len() - 1])
}

pub fn median(samples: &mut [f64]) -> f64 {
    percentiles(samples).0
}
