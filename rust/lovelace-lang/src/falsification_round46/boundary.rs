//! The deterministic boundary-biased sample generator.

use lovelace_symbolics::Rat;

use super::observation::{ObservationClass, rat_text};

/// A deterministic 64-bit generator (splitmix64), seeded explicitly.
pub(crate) struct DeterministicRng {
    state: u64,
}

impl DeterministicRng {
    /// Seeds the generator with the run's seed.
    pub(crate) fn new(seed: i64) -> DeterministicRng {
        DeterministicRng { state: seed as u64 }
    }

    /// The next 64 bits.
    pub(crate) fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }

    /// A uniform integer in [0, bound).
    pub(crate) fn next_int(&mut self, bound: usize) -> usize {
        assert!(bound > 0, "bound must be positive.");
        let bound = bound as u64;
        let limit = u64::MAX - (u64::MAX % bound);
        let mut draw = self.next_u64();
        while draw >= limit {
            draw = self.next_u64();
        }
        (draw % bound) as usize
    }
}

/// The deterministic step ladder a boundary search walks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct BiasStepLadder {
    /// The step sizes, coarse to fine.
    pub(crate) steps: Vec<i64>,
}

impl BiasStepLadder {
    /// The falsifier's default ladder: coarse-to-fine, coprime, odd steps first.
    pub(crate) fn default_ladder() -> BiasStepLadder {
        BiasStepLadder {
            steps: vec![1, 2, 3, 5, 7],
        }
    }

    /// The coarse step.
    pub(crate) fn coarse(&self) -> i64 {
        self.steps.first().copied().unwrap_or(1)
    }

    /// The fine step: what the refinement walks down to.
    pub(crate) fn fine(&self) -> i64 {
        self.steps.last().copied().unwrap_or(1)
    }

    /// Whether the ladder carries a step of the given size.
    pub(crate) fn contains(&self, step: i64) -> bool {
        self.steps.contains(&step)
    }
}

/// Everything a biased search needs, all of it explicit so a run is reproducible.
#[derive(Clone, Debug)]
pub(crate) struct BoundaryBiasSpec {
    /// The run's seed.
    pub(crate) seed: i64,
    /// The step ladder.
    pub(crate) ladder: BiasStepLadder,
    /// How many points are offered at each ladder step.
    pub(crate) samples_per_level: usize,
    /// The cap on the bisection descent.
    pub(crate) max_narrow_steps: usize,
}

impl BoundaryBiasSpec {
    /// The falsifier's default: 3 points per ladder step, at most 64 descents.
    pub(crate) fn default_spec(seed: i64) -> BoundaryBiasSpec {
        BoundaryBiasSpec {
            seed,
            ladder: BiasStepLadder::default_ladder(),
            samples_per_level: 3,
            max_narrow_steps: 64,
        }
    }
}

/// A deterministic boundary-biased sample generator over an exact-rational line.
pub(crate) struct BoundaryBias {
    spec: BoundaryBiasSpec,
    rng: DeterministicRng,
}

impl BoundaryBias {
    /// Builds a generator for one run.
    pub(crate) fn new(spec: BoundaryBiasSpec) -> BoundaryBias {
        BoundaryBias {
            rng: DeterministicRng::new(spec.seed),
            spec,
        }
    }

    /// The spec this generator was built from.
    pub(crate) fn spec(&self) -> &BoundaryBiasSpec {
        &self.spec
    }

    /// The broad phase: points strictly between the two observations, walked down the ladder.
    pub(crate) fn broad_phase(&mut self, low: &Rat, high: &Rat) -> Result<Vec<Rat>, String> {
        if low >= high {
            return Err(format!(
                "the low end {} must be strictly below the high end {}.",
                rat_text(low),
                rat_text(high)
            ));
        }
        let samples = self.spec.samples_per_level;
        let mut points = Vec::new();
        for step in self.spec.ladder.steps.clone() {
            let step_size = step as usize;
            let divisions = step_size * samples;
            if Self::interior_point(low, high, divisions, 1).is_none() {
                continue;
            }
            let jitter = self.rng.next_int(step_size);
            for index in 0..samples {
                let position = index * step_size + jitter;
                if position == 0 {
                    continue;
                }
                if let Some(point) = Self::interior_point(low, high, divisions, position) {
                    points.push(point);
                }
            }
        }
        Ok(points)
    }

    /// A single point strictly between two rationals, on the grid of divisions equal parts.
    pub(crate) fn interior_point(
        low: &Rat,
        high: &Rat,
        divisions: usize,
        index: usize,
    ) -> Option<Rat> {
        if index == 0 || index >= divisions {
            return None;
        }
        let span = high.add(&low.neg());
        let part = span
            .mul(&Rat::from_i64(index as i64))
            .div(&Rat::from_i64(divisions as i64))?;
        let point = low.add(&part);
        if point > *low && point < *high {
            Some(point)
        } else {
            None
        }
    }

    /// The narrow phase: an exact bisection that drives a bracket down to the ladder's resolution.
    pub(crate) fn narrow_phase<F>(
        &self,
        low: &Rat,
        high: &Rat,
        low_class: ObservationClass,
        classify: &mut F,
    ) -> (Rat, Vec<Rat>)
    where
        F: FnMut(&Rat) -> ObservationClass,
    {
        let mut offered = Vec::new();
        let gap = Rat::from_i64(self.spec.ladder.fine());
        let mut lower = low.clone();
        let mut upper = high.clone();
        let mut width = upper.add(&lower.neg());
        for _ in 0..self.spec.max_narrow_steps {
            if width <= gap {
                break;
            }
            let middle = lower
                .add(&upper)
                .div(&Rat::from_i64(2))
                .expect("divide by two");
            offered.push(middle.clone());
            if classify(&middle) == low_class {
                lower = middle;
            } else {
                upper = middle;
            }
            width = upper.add(&lower.neg());
        }
        (upper, offered)
    }
}
