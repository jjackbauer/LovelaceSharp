//! A faithful port of the seeded System.Random sequence used by the seven
//! Lovelace.Symbolics.Tests.PropertyTests census rows.
//!
//! # Contract
//!
//! The C# rows construct new Random(<literal seed>) and drive the generator only through
//! Next(), Next(maxValue) and Next(minValue, maxValue). In .NET 10 a seeded
//! Random is not the Xoshiro256** implementation used by the parameterless constructor:
//! Random(int Seed) selects CompatSeedImpl for the base type, which wraps the
//! CompatPrng subtractive generator (a modified Knuth generator) precisely so that seeded
//! sequences stay compatible across releases. The source is
//! https://github.com/dotnet/runtime/blob/release/10.0/src/libraries/System.Private.CoreLib/src/System/Random.CompatImpl.cs
//! (CompatSeedImpl, CompatPrng.Initialize, CompatPrng.InternalSample,
//! CompatPrng.Sample, CompatPrng.GetSampleForLargeRange). The constructor branch is
//! https://github.com/dotnet/runtime/blob/release/10.0/src/libraries/System.Private.CoreLib/src/System/Random.cs
//! (public Random(int Seed)).
//!
//! Every seed in the seven rows is a literal (20260909..20260915), so the sequence is a
//! pure function of the seed; no clock, Guid, process state or runtime-version-derived seed
//! participates.
//!
//! # Reproduced details
//!
//! * Seeding uses Math.Abs(seed), with int.MinValue mapped to int.MaxValue, the
//!   161803398 "golden ratio" constant, and the four 55-element mixing passes.
//! * Next() is one InternalSample() in [0, int.MaxValue). The sample is
//!   InternalSample() * (1.0 / int.MaxValue); Next(maxValue) truncates
//!   Sample() * maxValue toward zero, and Next(minValue, maxValue) truncates
//!   Sample() * range before adding the minimum. Only small ranges are used by the rows,
//!   but the large-range branch is ported too so this is a complete seeded Random surface.
//!
//! The sequences are pinned by unit tests against values produced by the .NET 10.0.3 runtime
//! directly. A transcript hash covers every draw the seven corpora consume.

/// The seeded System.Random generator .NET keeps for compatibility.
///
/// The 56-entry table mirrors CompatPrng._seedArray; index 0 is unused, exactly as in the
/// reference (Knuth's range is [1..55]).
#[derive(Clone, Debug)]
pub struct DotNetRandom {
    seed_array: [i32; 56],
    inext: usize,
    inextp: usize,
}

impl DotNetRandom {
    /// new Random(seed) for the seeded constructor.
    pub fn new(seed: i32) -> DotNetRandom {
        let mut seed_array = [0i32; 56];

        // C#: int subtraction = (seed == int.MinValue) ? int.MaxValue : Math.Abs(seed);
        let subtraction = if seed == i32::MIN {
            i32::MAX
        } else {
            seed.abs()
        };
        let mut mj = 161_803_398i32.wrapping_sub(subtraction);
        seed_array[55] = mj;
        let mut mk = 1i32;

        let mut ii = 0usize;
        for _ in 1..55 {
            ii += 21;
            if ii >= 55 {
                ii -= 55;
            }
            seed_array[ii] = mk;
            mk = mj.wrapping_sub(mk);
            if mk < 0 {
                mk = mk.wrapping_add(i32::MAX);
            }
            mj = seed_array[ii];
        }

        for _ in 1..5 {
            for i in 1..56 {
                let mut n = i + 30;
                if n >= 55 {
                    n -= 55;
                }
                seed_array[i] = seed_array[i].wrapping_sub(seed_array[1 + n]);
                if seed_array[i] < 0 {
                    seed_array[i] = seed_array[i].wrapping_add(i32::MAX);
                }
            }
        }

        DotNetRandom {
            seed_array,
            inext: 0,
            inextp: 21,
        }
    }

    /// Random.Next(): a non-negative integer in [0, int.MaxValue).
    // The C# method is Next; making this Iterator::next would claim a fused/sized sequence
    // contract the generator does not advertise.
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> i32 {
        self.internal_sample()
    }

    /// Random.Next(maxValue): a non-negative integer in [0, maxValue).
    ///
    /// # Panics
    ///
    /// Panics when max_value < 0, matching the reference's ArgumentOutOfRangeException
    /// contract at the boundary this port can express.
    pub fn next_max(&mut self, max_value: i32) -> i32 {
        assert!(max_value >= 0, "maxValue must be non-negative");
        (self.sample() * f64::from(max_value)) as i32
    }

    /// Random.Next(minValue, maxValue): an integer in [minValue, maxValue).
    ///
    /// # Panics
    ///
    /// Panics when min_value > max_value, matching the reference's
    /// ArgumentOutOfRangeException.
    pub fn next_range(&mut self, min_value: i32, max_value: i32) -> i32 {
        assert!(min_value <= max_value, "minValue must not exceed maxValue");
        let range = i64::from(max_value) - i64::from(min_value);
        if range <= i64::from(i32::MAX) {
            let scaled = (self.sample() * range as f64) as i64;
            (scaled + i64::from(min_value)) as i32
        } else {
            let scaled = (self.get_sample_for_large_range() * range as f64) as i64;
            (scaled + i64::from(min_value)) as i32
        }
    }

    /// Random.NextDouble(): the raw sample in [0.0, 1.0).
    pub fn next_double(&mut self) -> f64 {
        self.sample()
    }

    /// CompatPrng.Sample().
    fn sample(&mut self) -> f64 {
        f64::from(self.internal_sample()) * (1.0 / f64::from(i32::MAX))
    }

    /// CompatPrng.InternalSample().
    fn internal_sample(&mut self) -> i32 {
        let mut loc_inext = self.inext + 1;
        if loc_inext >= 56 {
            loc_inext = 1;
        }
        let mut loc_inextp = self.inextp + 1;
        if loc_inextp >= 56 {
            loc_inextp = 1;
        }

        let mut ret_val = self.seed_array[loc_inext] - self.seed_array[loc_inextp];
        if ret_val == i32::MAX {
            ret_val -= 1;
        }
        if ret_val < 0 {
            ret_val += i32::MAX;
        }

        self.seed_array[loc_inext] = ret_val;
        self.inext = loc_inext;
        self.inextp = loc_inextp;
        ret_val
    }

    /// CompatPrng.GetSampleForLargeRange().
    fn get_sample_for_large_range(&mut self) -> f64 {
        let mut result = self.internal_sample();
        if self.internal_sample() % 2 == 0 {
            result = -result;
        }
        let mut d = f64::from(result);
        d += f64::from(i32::MAX - 1);
        d /= 2.0 * f64::from(i32::MAX) - 1.0;
        d
    }
}

#[cfg(test)]
mod tests {
    use super::DotNetRandom;
    use crate::fnv1a64;

    /// The mixed-call transcript: ten Next(), five Next(100) and five Next(-5, 6)
    /// calls. This is the exact probe the C# runtime produced while pinning the port.
    fn probe(seed: i32) -> (String, Vec<i32>) {
        let mut rng = DotNetRandom::new(seed);
        let mut transcript = String::new();
        let mut values = Vec::new();
        for _ in 0..10 {
            let value = rng.next();
            transcript.push_str(&format!("N:{value};"));
            values.push(value);
        }
        for _ in 0..5 {
            let value = rng.next_max(100);
            transcript.push_str(&format!("M:100:{value};"));
            values.push(value);
        }
        for _ in 0..5 {
            let value = rng.next_range(-5, 6);
            transcript.push_str(&format!("R:-5:6:{value};"));
            values.push(value);
        }
        (transcript, values)
    }

    #[test]
    fn seeded_next_values_match_dotnet_10() {
        // (seed, first ten Next() values, FNV-1a 64 of the 20-draw mixed transcript).
        let golden: [(i32, [i32; 10], u64); 7] = [
            (
                20_260_909,
                [
                    1022901653, 1561700275, 1793539373, 754725002, 1490823099, 1063820590,
                    1895598795, 1018616557, 1688822717, 1517592111,
                ],
                16_060_508_841_591_869_625,
            ),
            (
                20_260_910,
                [
                    2144801472, 44328311, 1147121005, 1213090205, 312897695, 793018192, 710309579,
                    2095975608, 1954502308, 161995416,
                ],
                6_445_143_819_552_792_930,
            ),
            (
                20_260_911,
                [
                    1119217644, 674439994, 500702637, 1671455408, 1282455938, 522215794,
                    1672504010, 1025851012, 72698252, 953882368,
                ],
                13_504_912_312_986_076_737,
            ),
            (
                20_260_912,
                [
                    93633816, 1304551677, 2001767916, 2129820611, 104530534, 251413396, 487214794,
                    2103210063, 338377843, 1745769320,
                ],
                9_446_131_426_748_795_924,
            ),
            (
                20_260_913,
                [
                    1215533635, 1934663360, 1355349548, 440702167, 1074088777, 2128094645,
                    1449409225, 1033085467, 604057434, 390172625,
                ],
                13_626_191_053_975_151_902,
            ),
            (
                20_260_914,
                [
                    189949807, 417291396, 708931180, 899067370, 2043647020, 1857292247, 264120009,
                    2110444518, 869737025, 1182059577,
                ],
                2_740_562_890_973_147_526,
            ),
            (
                20_260_915,
                [
                    1311849626, 1047403079, 62512812, 1357432573, 865721616, 1586489849,
                    1226314440, 1040319922, 1135416616, 1973946529,
                ],
                8_305_199_360_401_033_779,
            ),
        ];

        for (seed, expected, expected_hash) in golden {
            let (transcript, values) = probe(seed);
            assert_eq!(
                &values[..10],
                &expected,
                "seed {seed}: first ten Next() values differ from .NET 10"
            );
            assert_eq!(
                fnv1a64(&transcript),
                expected_hash,
                "seed {seed}: mixed 20-draw transcript differs from .NET 10"
            );
        }
    }

    #[test]
    fn minimum_seed_uses_the_int_min_value_rule() {
        // .NET maps int.MinValue to int.MaxValue instead of calling Math.Abs.
        let (_, values) = probe(i32::MIN);
        assert_eq!(
            &values[..10],
            &[
                1559595546, 1755192844, 1649316172, 1198642031, 442452829, 1200195955, 1945678308,
                949569752, 2099272109, 587775835
            ]
        );
        let (_, negative_one) = probe(-1);
        let (_, positive_one) = probe(1);
        assert_eq!(negative_one, positive_one, "-1 and 1 share |seed|");
    }

    #[test]
    fn next_double_matches_the_documented_sample_contract() {
        let mut rng = DotNetRandom::new(20_260_909);
        let expected = f64::from(1022901653) * (1.0 / f64::from(i32::MAX));
        assert_eq!(rng.next_double().to_bits(), expected.to_bits());
    }
}
