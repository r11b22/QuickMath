//! A small seeded random source for the practice game.

use std::ops::RangeInclusive;

/// xorshift64: plenty for practice exercises, and needs no dependency.
pub struct Random {
    state: u64,
}

impl Random {
    pub fn new(seed: u64) -> Random {
        // xorshift gets stuck at zero.
        Random { state: seed | 1 }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    /// A number in `0..bound`. `bound` must not be zero.
    pub fn below(&mut self, bound: usize) -> usize {
        (self.next_u64() % bound as u64) as usize
    }

    pub fn number(&mut self, range: RangeInclusive<u32>) -> u32 {
        let span = (range.end() - range.start() + 1) as usize;
        range.start() + self.below(span) as u32
    }

    /// One of `items`, which must not be empty.
    pub fn pick<T: Copy>(&mut self, items: &[T]) -> T {
        items[self.below(items.len())]
    }

    /// `N` different elements of `items`, which must hold at least `N`.
    pub fn pick_distinct<T: Copy, const N: usize>(&mut self, items: &[T]) -> [T; N] {
        assert!(items.len() >= N, "cannot pick {N} of {} items", items.len());
        let mut remaining = items.to_vec();
        std::array::from_fn(|_| remaining.swap_remove(self.below(remaining.len())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_gives_the_same_sequence() {
        let mut first = Random::new(42);
        let mut second = Random::new(42);

        for _ in 0..20 {
            assert_eq!(first.below(1000), second.below(1000));
        }
    }

    #[test]
    fn number_stays_in_its_range() {
        let mut random = Random::new(3);

        for _ in 0..200 {
            assert!((2..=4).contains(&random.number(2..=4)));
        }
    }

    #[test]
    fn pick_distinct_never_repeats_an_item() {
        let mut random = Random::new(9);

        for _ in 0..100 {
            let [a, b, c] = random.pick_distinct(&[1, 2, 3]);
            assert!(a != b && b != c && a != c);
        }
    }
}
