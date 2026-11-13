//! The generator of the `random` module (HQL-0041 part 4), and the
//! evaluation seed it draws from.
//!
//! One generator, pinned, so a seed means the same draw on every host and in
//! every release: ChaCha12, its 256-bit key four SplitMix64 outputs of the
//! 64-bit seed, its stream a SplitMix64 mix of the call site and the
//! partition. Every draw from it is written here, not taken from a crate
//! whose sampling may change: an integer below a bound by rejection, a float
//! from the top 53 bits of a word.
//!
//! The generator is not fit for secrets: a seeded draw is predictable by
//! design.

use rand_chacha::ChaCha12Rng;
use rand_core::{RngCore, SeedableRng};
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::{Mutex, PoisonError};

/// How the host fixes the evaluation seed: `--seed`, and whether an
/// evaluation that draws one from entropy reports it.
#[derive(Debug, Clone, Copy, Default)]
pub struct Seeding {
    /// `--seed`: the seed every evaluation draws from, instead of one from
    /// the host's entropy.
    pub seed: Option<i64>,
    /// Whether an evaluation reports a seed it drew from entropy, as an
    /// `info` diagnostic, so `--seed` can repeat it.
    pub report: bool,
}

/// The seeding the host configured.
static SEEDING: Mutex<Seeding> = Mutex::new(Seeding {
    seed: None,
    report: false,
});

/// What one evaluation has drawn: its seed, the generator of each call site
/// that drew from it, and the UUID counter.
struct Drawn {
    seed: i64,
    sites: HashMap<u64, Generator>,
    counter: Option<u64>,
}

thread_local! {
    /// The draws of the evaluation on this thread. An evaluation runs on
    /// one thread, so two evaluating at once each keep their own.
    static DRAWN: RefCell<Option<Drawn>> = const { RefCell::new(None) };
}

/// Fix the evaluation seed: `--seed`, and whether to report an entropy seed.
pub fn set_seeding(seeding: Seeding) {
    *SEEDING.lock().unwrap_or_else(PoisonError::into_inner) = seeding;
    begin_evaluation();
}

/// Start an evaluation: its first unseeded draw fixes its seed afresh.
pub fn begin_evaluation() {
    DRAWN.with(|drawn| *drawn.borrow_mut() = None);
}

/// The site of the UUID counter's own stream, apart from every call site.
const COUNTER_SITE: u64 = u64::MAX;

/// Run `draw` on the generator of a call site in this evaluation, fixing
/// the evaluation seed on the first draw. The second value is the seed when
/// this draw fixed it from entropy and the host asked for it to be reported.
pub(crate) fn with_evaluation_stream<R>(
    site: u64,
    draw: impl FnOnce(&mut Generator) -> R,
) -> (R, Option<i64>) {
    with_drawn(|state| {
        let seed = state.seed;
        let generator = state
            .sites
            .entry(site)
            .or_insert_with(|| Generator::new(seed, site, 0));
        draw(generator)
    })
}

/// The next value of this evaluation's UUID counter: 42 bits, starting at a
/// draw of 41 from the evaluation seed so it has room to count, and rising
/// by one per UUID. The second value is as [`with_evaluation_stream`]'s.
pub(crate) fn next_counter() -> (u64, Option<i64>) {
    with_drawn(|state| {
        let next = match state.counter {
            Some(held) => held.wrapping_add(1) & ((1 << 42) - 1),
            None => Generator::new(state.seed, COUNTER_SITE, 0).next_u64() >> 23,
        };
        state.counter = Some(next);
        next
    })
}

/// Run `use_draws` on this evaluation's draws, fixing its seed first where
/// no draw has.
fn with_drawn<R>(use_draws: impl FnOnce(&mut Drawn) -> R) -> (R, Option<i64>) {
    DRAWN.with(|drawn| {
        let mut drawn = drawn.borrow_mut();
        let mut reported = None;
        let state = drawn.get_or_insert_with(|| {
            let seeding = *SEEDING.lock().unwrap_or_else(PoisonError::into_inner);
            let seed = seeding.seed.unwrap_or_else(|| {
                let seed = entropy();
                if seeding.report {
                    reported = Some(seed);
                }
                seed
            });
            Drawn {
                seed,
                sites: HashMap::new(),
                counter: None,
            }
        });
        (use_draws(state), reported)
    })
}

/// A seed from the host's entropy: the operating system's keys of the
/// standard library's hasher, mixed with the clock.
fn entropy() -> i64 {
    use std::hash::{BuildHasher, Hasher};
    let mut hasher = std::collections::hash_map::RandomState::new().build_hasher();
    if let Ok(elapsed) = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        hasher.write_u128(elapsed.as_nanos());
    }
    hasher.finish().cast_signed()
}

/// One SplitMix64 step: the next output, advancing `state`.
fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// The pinned generator: ChaCha12 keyed by a seed, on the stream of a call
/// site and a partition.
#[derive(Debug, Clone)]
pub struct Generator(ChaCha12Rng);

impl Generator {
    /// The generator of `seed` at the call site `site`, in `partition`.
    #[must_use]
    pub fn new(seed: i64, site: u64, partition: u64) -> Self {
        let mut state = seed.cast_unsigned();
        let mut key = [0_u8; 32];
        for word in key.as_chunks_mut::<8>().0 {
            word.copy_from_slice(&splitmix64(&mut state).to_le_bytes());
        }
        let mut rng = ChaCha12Rng::from_seed(key);
        let mut at_site = site;
        let mut in_partition = partition ^ 0x5851_F42D_4C95_7F2D;
        rng.set_stream(splitmix64(&mut at_site) ^ splitmix64(&mut in_partition));
        Self(rng)
    }

    /// The next 64 bits.
    pub fn next_u64(&mut self) -> u64 {
        self.0.next_u64()
    }

    /// An integer drawn uniformly below `bound`, which is not zero: a word
    /// below the largest multiple of `bound` is kept, and any other drawn
    /// again.
    pub fn below(&mut self, bound: u64) -> u64 {
        debug_assert!(bound > 0, "a bound above zero");
        let rejected = bound.wrapping_neg() % bound;
        loop {
            let word = self.next_u64();
            if word >= rejected {
                return word % bound;
            }
        }
    }

    /// An index drawn uniformly below `bound`, which is not zero.
    pub fn index(&mut self, bound: usize) -> usize {
        // A `usize` fits a `u64` on every supported target, and so does
        // what is drawn below one.
        usize::try_from(self.below(bound as u64)).unwrap_or(0)
    }

    /// A float drawn uniformly from `[0, 1)`: the top 53 bits of a word.
    pub fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / (1_u64 << 53) as f64)
    }

    /// A float drawn uniformly from `(0, 1)`, for a logarithm.
    fn open_unit(&mut self) -> f64 {
        ((self.next_u64() >> 11) as f64 + 0.5) * (1.0 / (1_u64 << 53) as f64)
    }

    /// An `Int` drawn uniformly from `low` to `high`, both included, where
    /// `low <= high`.
    pub fn between(&mut self, low: i64, high: i64) -> i64 {
        let width = high.abs_diff(low);
        if width == u64::MAX {
            return self.next_u64().cast_signed();
        }
        low.wrapping_add(self.below(width + 1).cast_signed())
    }

    /// The positions of `n` elements of `size`, drawn without replacement,
    /// in ascending order: the first `n` of a partial Fisher–Yates shuffle.
    pub fn positions(&mut self, size: usize, n: usize) -> Vec<usize> {
        if n >= size {
            return (0..size).collect();
        }
        let mut order: Vec<usize> = (0..size).collect();
        for at in 0..n {
            let chosen = at + self.index(size - at);
            order.swap(at, chosen);
        }
        order.truncate(n);
        order.sort_unstable();
        order
    }

    /// A uniformly random permutation of `items` (Fisher–Yates).
    pub fn shuffle<T>(&mut self, items: &mut [T]) {
        for at in (1..items.len()).rev() {
            let chosen = self.index(at + 1);
            items.swap(at, chosen);
        }
    }
}

/// A sample of `n` elements of a sequence of unknown length, drawn by
/// reservoir sampling (Algorithm L): it holds `n` elements and their
/// positions, never the sequence.
pub struct Reservoir<T> {
    n: usize,
    held: Vec<(usize, T)>,
    seen: usize,
    weight: f64,
    next: usize,
}

impl<T> Reservoir<T> {
    /// An empty reservoir of `n` places.
    #[must_use]
    pub fn new(n: usize) -> Self {
        Self {
            n,
            held: Vec::with_capacity(n),
            seen: 0,
            weight: 0.0,
            next: 0,
        }
    }

    /// Offer the next element.
    pub fn offer(&mut self, generator: &mut Generator, element: T) {
        let at = self.seen;
        self.seen += 1;
        if self.held.len() < self.n {
            self.held.push((at, element));
            if self.held.len() == self.n {
                self.weight = self.step_weight(generator, 1.0);
                self.next = at + self.skip(generator);
            }
            return;
        }
        if self.n > 0 && at == self.next {
            let place = generator.index(self.n);
            self.held[place] = (at, element);
            self.weight = self.step_weight(generator, self.weight);
            self.next = at + self.skip(generator);
        }
    }

    fn step_weight(&self, generator: &mut Generator, weight: f64) -> f64 {
        weight * (generator.open_unit().ln() / self.n as f64).exp()
    }

    /// How far past the current element the next one taken lies.
    fn skip(&self, generator: &mut Generator) -> usize {
        let gap = (generator.open_unit().ln() / (1.0 - self.weight).ln()).floor();
        // A gap past what a `usize` counts is never reached.
        if gap.is_finite() && gap >= 0.0 && gap < usize::MAX as f64 / 2.0 {
            gap as usize + 1
        } else {
            usize::MAX / 2
        }
    }

    /// The sample, in the order its elements were offered.
    #[must_use]
    pub fn into_sample(mut self) -> Vec<T> {
        self.held.sort_by_key(|(at, _)| *at);
        self.held.into_iter().map(|(_, element)| element).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splitmix64_gives_its_reference_outputs() {
        // The first outputs of SplitMix64 from 1234567, as its reference
        // implementation gives them.
        let mut state = 1_234_567;
        assert_eq!(splitmix64(&mut state), 6_457_827_717_110_365_317);
        assert_eq!(splitmix64(&mut state), 3_203_168_211_198_807_973);
    }

    #[test]
    fn one_seed_and_site_give_one_sequence_and_another_site_another() {
        let mut first = Generator::new(7, 10, 0);
        let mut again = Generator::new(7, 10, 0);
        let mut elsewhere = Generator::new(7, 11, 0);
        let drawn: Vec<u64> = (0..4).map(|_| first.next_u64()).collect();
        assert_eq!(drawn, (0..4).map(|_| again.next_u64()).collect::<Vec<_>>());
        assert_ne!(
            drawn,
            (0..4).map(|_| elsewhere.next_u64()).collect::<Vec<_>>()
        );
    }

    #[test]
    fn a_bounded_draw_stays_in_its_bounds() {
        let mut generator = Generator::new(1, 0, 0);
        for _ in 0..1000 {
            assert!(generator.below(6) < 6);
            assert!((-3..=3).contains(&generator.between(-3, 3)));
            let unit = generator.unit();
            assert!((0.0..1.0).contains(&unit));
        }
        assert_eq!(generator.between(5, 5), 5);
        // The whole range of `Int` is a width a `u64` does not count.
        let _ = generator.between(i64::MIN, i64::MAX);
    }

    #[test]
    fn positions_are_ascending_distinct_and_in_range() {
        let mut generator = Generator::new(3, 0, 0);
        let drawn = generator.positions(100, 10);
        assert_eq!(drawn.len(), 10);
        assert!(drawn.windows(2).all(|pair| pair[0] < pair[1]));
        assert!(drawn.iter().all(|at| *at < 100));
        assert_eq!(generator.positions(3, 5), vec![0, 1, 2]);
    }

    #[test]
    fn a_reservoir_holds_n_in_their_order() {
        let mut generator = Generator::new(9, 0, 0);
        let mut reservoir = Reservoir::new(5);
        for element in 0..10_000 {
            reservoir.offer(&mut generator, element);
        }
        let sample = reservoir.into_sample();
        assert_eq!(sample.len(), 5);
        assert!(sample.windows(2).all(|pair| pair[0] < pair[1]));
        let mut short = Reservoir::new(5);
        for element in 0..3 {
            short.offer(&mut generator, element);
        }
        assert_eq!(short.into_sample(), vec![0, 1, 2]);
    }

    #[test]
    fn a_reservoir_takes_every_position_about_as_often() {
        // Each of 20 positions lands in a sample of 5 a quarter of the time.
        let mut counts = [0_u32; 20];
        for seed in 0..4000 {
            let mut generator = Generator::new(seed, 0, 0);
            let mut reservoir = Reservoir::new(5);
            for element in 0..20 {
                reservoir.offer(&mut generator, element);
            }
            for element in reservoir.into_sample() {
                counts[element] += 1;
            }
        }
        for count in counts {
            assert!((850..1150).contains(&count), "{counts:?}");
        }
    }

    #[test]
    fn uniform_draws_pass_a_chi_squared_check() {
        // `between(1, 6)` over 60 000 draws: chi-squared with 5 degrees of
        // freedom stays below 20.5, its 0.999 quantile.
        let mut generator = Generator::new(42, 0, 0);
        let mut counts = [0_u64; 6];
        for _ in 0..60_000 {
            let drawn = usize::try_from(generator.between(1, 6) - 1).unwrap();
            counts[drawn] += 1;
        }
        let chi: f64 = counts
            .iter()
            .map(|count| (*count as f64 - 10_000.0).powi(2) / 10_000.0)
            .sum();
        assert!(chi < 20.5, "{chi} over {counts:?}");
        // Where a shuffle of three puts its first element: each place a
        // third of the time, chi-squared with 2 degrees of freedom below
        // 13.8.
        let mut places = [0_u64; 3];
        for _ in 0..30_000 {
            let mut items = [0, 1, 2];
            generator.shuffle(&mut items);
            let place = items.iter().position(|item| *item == 0).unwrap();
            places[place] += 1;
        }
        let chi: f64 = places
            .iter()
            .map(|count| (*count as f64 - 10_000.0).powi(2) / 10_000.0)
            .sum();
        assert!(chi < 13.8, "{chi} over {places:?}");
    }
}
