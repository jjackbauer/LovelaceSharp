//! The probe record and the falsification report, including their deterministic text form.

use super::observation::{GENERATOR_VERSION, ObservationClass, SideOutcome, escape, unescape};

/// One observed sample of one identity at one input, in a form that survives leaving the process.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FalsificationProbe {
    /// The stable rewrite-rule id this sample attacks.
    pub(crate) rule_id: String,
    /// The region tag the probe came from.
    pub(crate) region: String,
    /// The probe's own short label.
    pub(crate) label: String,
    /// The exact input, as text.
    pub(crate) input: String,
    /// Whether the input names a complex pair.
    pub(crate) is_complex: bool,
    /// The observation class the falsifier branches on.
    pub(crate) class: ObservationClass,
    /// What the left side did.
    pub(crate) left_outcome: SideOutcome,
    /// What the right side did.
    pub(crate) right_outcome: SideOutcome,
    /// The left-hand rendering.
    pub(crate) left: String,
    /// The right-hand rendering.
    pub(crate) right: String,
    /// The left-hand side's value at this input, when it produced one.
    pub(crate) left_value: String,
    /// The right-hand side's value at this input, when it produced one.
    pub(crate) right_value: String,
    /// The measured difference, rendered deterministically.
    pub(crate) delta_text: String,
    /// The computation precision the sample was evaluated at.
    pub(crate) precision: i64,
    /// The comparison tolerance text the verdict used.
    pub(crate) tolerance: String,
    /// The assumption environment the rule was licensed under.
    pub(crate) environment: String,
    /// The generator that produced the input.
    pub(crate) generator_version: String,
    /// Why this input is near the feature its region names.
    pub(crate) why: String,
}

impl FalsificationProbe {
    /// The generator version stamped on every record.
    pub(crate) fn current_generator_version() -> &'static str {
        GENERATOR_VERSION
    }

    /// True when this sample is a counterexample to the identity.
    pub(crate) fn is_counterexample(&self) -> bool {
        self.class == ObservationClass::Disagree
    }

    /// True when a side threw something the kernel does not expect.
    pub(crate) fn is_defect(&self) -> bool {
        self.left_outcome == SideOutcome::Threw || self.right_outcome == SideOutcome::Threw
    }

    /// The record's deterministic, line-delimited text form.
    pub(crate) fn to_text(&self) -> String {
        let precision = self.precision.to_string();
        let fields = [
            self.rule_id.as_str(),
            self.region.as_str(),
            self.label.as_str(),
            self.input.as_str(),
            if self.is_complex { "complex" } else { "real" },
            self.class.name(),
            self.left_outcome.name(),
            self.right_outcome.name(),
            self.left.as_str(),
            self.right.as_str(),
            self.left_value.as_str(),
            self.right_value.as_str(),
            self.delta_text.as_str(),
            precision.as_str(),
            self.tolerance.as_str(),
            self.environment.as_str(),
            self.generator_version.as_str(),
            self.why.as_str(),
        ];
        fields
            .iter()
            .map(|field| escape(field))
            .collect::<Vec<String>>()
            .join("|")
    }

    /// Parses the form [FalsificationProbe::to_text] writes.
    pub(crate) fn parse(text: &str) -> Result<FalsificationProbe, String> {
        let fields: Vec<&str> = text.split('|').collect();
        if fields.len() != 18 {
            return Err(format!(
                "a falsification probe has 18 fields; got {}.",
                fields.len()
            ));
        }
        let values: Vec<String> = fields.iter().map(|field| unescape(field)).collect();
        Ok(FalsificationProbe {
            rule_id: values[0].clone(),
            region: values[1].clone(),
            label: values[2].clone(),
            input: values[3].clone(),
            is_complex: values[4] == "complex",
            class: ObservationClass::from_name(&values[5])?,
            left_outcome: SideOutcome::from_name(&values[6])?,
            right_outcome: SideOutcome::from_name(&values[7])?,
            left: values[8].clone(),
            right: values[9].clone(),
            left_value: values[10].clone(),
            right_value: values[11].clone(),
            delta_text: values[12].clone(),
            precision: values[13]
                .parse::<i64>()
                .map_err(|_| format!("malformed precision '{}'.", values[13]))?,
            tolerance: values[14].clone(),
            environment: values[15].clone(),
            generator_version: values[16].clone(),
            why: values[17].clone(),
        })
    }

    /// The probe's identity for hashing and de-duplication.
    pub(crate) fn identity_key(&self) -> String {
        format!(
            "{}@{}{}#{}",
            self.rule_id,
            if self.is_complex { "C:" } else { "R:" },
            self.input,
            self.environment
        )
    }
}

/// A deterministic, human- and machine-readable falsification report.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FalsificationReport {
    /// The generator version that produced the report.
    pub(crate) generator_version: String,
    /// How many samples the run took.
    pub(crate) sample_count: usize,
    /// Every probe, in report order.
    pub(crate) probes: Vec<FalsificationProbe>,
    /// Every defect, in report order.
    pub(crate) defects: Vec<String>,
}

impl FalsificationReport {
    /// Every counterexample the run found, in report order.
    pub(crate) fn counterexamples(&self) -> Vec<&FalsificationProbe> {
        self.probes
            .iter()
            .filter(|probe| probe.is_counterexample())
            .collect()
    }

    /// A defect (an unexpected exception type) is a failure of the run, not a class.
    pub(crate) fn failed(&self) -> bool {
        !self.defects.is_empty() || !self.counterexamples().is_empty()
    }

    /// The report's deterministic text form: a header, then one line per probe, then defects.
    pub(crate) fn to_text(&self) -> String {
        let mut text = format!(
            "falsification-report {} samples={} counterexamples={} defects={}\n",
            self.generator_version,
            self.sample_count,
            self.counterexamples().len(),
            self.defects.len()
        );
        for probe in &self.probes {
            text.push_str(&probe.to_text());
            text.push('\n');
        }
        for defect in &self.defects {
            text.push_str("DEFECT|");
            text.push_str(&escape(defect));
            text.push('\n');
        }
        text
    }

    /// Parses the text [FalsificationReport::to_text] writes.
    pub(crate) fn parse(text: &str) -> Result<FalsificationReport, String> {
        let lines: Vec<&str> = text.split('\n').filter(|line| !line.is_empty()).collect();
        let Some(header) = lines.first() else {
            return Err("a falsification report needs a header line.".to_string());
        };
        let header: Vec<&str> = header.split(' ').collect();
        if header.len() != 5 || header[0] != "falsification-report" {
            return Err(format!(
                "malformed falsification report header: {line}",
                line = lines[0]
            ));
        }
        let samples = header[2]
            .split('=')
            .nth(1)
            .ok_or_else(|| format!("malformed sample count '{}'.", header[2]))?
            .parse::<usize>()
            .map_err(|_| format!("malformed sample count '{}'.", header[2]))?;
        let mut probes = Vec::new();
        let mut defects = Vec::new();
        for line in &lines[1..] {
            if let Some(rest) = line.strip_prefix("DEFECT|") {
                defects.push(unescape(rest));
            } else {
                probes.push(FalsificationProbe::parse(line)?);
            }
        }
        Ok(FalsificationReport {
            generator_version: header[1].to_string(),
            sample_count: samples,
            probes,
            defects,
        })
    }

    /// The report's stable hash, over exactly the text [FalsificationReport::to_text] produces.
    pub(crate) fn fingerprint(&self) -> String {
        sha256_hex(&self.to_text())
    }
}

/// SHA-256 as uppercase hex, matching the reference's Convert.ToHexString(SHA256.HashData(...)).
pub(crate) fn sha256_hex(text: &str) -> String {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut state: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    let mut message = text.as_bytes().to_vec();
    let bit_length = (message.len() as u64).wrapping_mul(8);
    message.push(0x80);
    while message.len() % 64 != 56 {
        message.push(0);
    }
    message.extend_from_slice(&bit_length.to_be_bytes());
    for chunk in message.chunks_exact(64) {
        let mut words = [0u32; 64];
        for (index, word) in chunk.chunks_exact(4).enumerate() {
            words[index] = u32::from_be_bytes([word[0], word[1], word[2], word[3]]);
        }
        for index in 16..64 {
            let s0 = words[index - 15].rotate_right(7)
                ^ words[index - 15].rotate_right(18)
                ^ (words[index - 15] >> 3);
            let s1 = words[index - 2].rotate_right(17)
                ^ words[index - 2].rotate_right(19)
                ^ (words[index - 2] >> 10);
            words[index] = words[index - 16]
                .wrapping_add(s0)
                .wrapping_add(words[index - 7])
                .wrapping_add(s1);
        }
        let mut a = state[0];
        let mut b = state[1];
        let mut c = state[2];
        let mut d = state[3];
        let mut e = state[4];
        let mut f = state[5];
        let mut g = state[6];
        let mut h = state[7];
        for index in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let choose = (e & f) ^ ((!e) & g);
            let temp1 = h
                .wrapping_add(s1)
                .wrapping_add(choose)
                .wrapping_add(K[index])
                .wrapping_add(words[index]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let majority = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(majority);
            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }
        state[0] = state[0].wrapping_add(a);
        state[1] = state[1].wrapping_add(b);
        state[2] = state[2].wrapping_add(c);
        state[3] = state[3].wrapping_add(d);
        state[4] = state[4].wrapping_add(e);
        state[5] = state[5].wrapping_add(f);
        state[6] = state[6].wrapping_add(g);
        state[7] = state[7].wrapping_add(h);
    }
    state
        .iter()
        .map(|word| format!("{word:08X}"))
        .collect::<Vec<String>>()
        .join("")
}
