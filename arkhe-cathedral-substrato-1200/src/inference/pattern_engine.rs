//! PatternEngine — Extensão com Sliding Cache Determinístico
//! Ponte parcial para no_std.

#![forbid(unsafe_code)]

extern crate alloc;

use alloc::vec::Vec;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct FixedScore(i16);

impl FixedScore {
    pub const fn from_raw(raw: i16) -> Self {
        Self(raw)
    }

    pub const fn from_f32_approx(val: f32) -> Self {
        Self((val * 256.0) as i16)
    }

    pub const fn to_raw(self) -> i16 {
        self.0
    }
}

pub type BlockId = u16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScoredBlock {
    pub score: FixedScore,
    pub block_id: BlockId,
}

impl core::cmp::Ord for ScoredBlock {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.score.cmp(&other.score)
    }
}

impl core::cmp::PartialOrd for ScoredBlock {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

// ============================================================================
// SlidingCache — cache deslizante de tamanho fixo, determinístico, no_std
// ============================================================================

pub struct SlidingCache<T, const N: usize> {
    buffer: [Option<T>; N],
    head: usize,
    count: usize,
}

impl<T: Copy, const N: usize> SlidingCache<T, N> {
    pub const fn new() -> Self {
        Self {
            buffer: [None; N],
            head: 0,
            count: 0,
        }
    }

    pub fn push(&mut self, item: T) {
        self.buffer[self.head] = Some(item);
        self.head = (self.head + 1) % N;
        if self.count < N {
            self.count += 1;
        }
    }

    pub fn len(&self) -> usize {
        self.count
    }

    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    pub fn iter(&self) -> impl core::iter::Iterator<Item = T> + '_ {
        let count = self.count;
        let head = self.head;

        (0..count).map(move |i| {
            let pos = if count < N {
                i
            } else {
                (head + i) % N
            };
            self.buffer[pos].unwrap()
        })
    }

    pub fn get_all(&self) -> Vec<T> {
        self.iter().collect()
    }
}

pub struct PatternEngine<const CACHE_SIZE: usize> {
    cache: SlidingCache<ScoredBlock, CACHE_SIZE>,
}

impl<const CACHE_SIZE: usize> PatternEngine<CACHE_SIZE> {
    pub const fn new() -> Self {
        Self {
            cache: SlidingCache::new(),
        }
    }

    pub fn ingest(&mut self, block: ScoredBlock) {
        self.cache.push(block);
    }

    pub fn get_recent_patterns(&self) -> Vec<ScoredBlock> {
        self.cache.get_all()
    }
}

// ============================================================================
// Testes
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sliding_cache_push() {
        let mut cache: SlidingCache<i32, 3> = SlidingCache::new();
        assert_eq!(cache.len(), 0);

        cache.push(1);
        assert_eq!(cache.len(), 1);

        cache.push(2);
        cache.push(3);
        assert_eq!(cache.len(), 3);

        // Overflow the cache
        cache.push(4);
        assert_eq!(cache.len(), 3);

        let items: Vec<i32> = cache.get_all();
        assert_eq!(items, alloc::vec![2, 3, 4]); // 1 should be evicted
    }

    #[test]
    fn test_sliding_cache_iter() {
        let mut cache: SlidingCache<i32, 3> = SlidingCache::new();
        cache.push(10);
        cache.push(20);

        let mut iter = cache.iter();
        assert_eq!(iter.next(), Some(10));
        assert_eq!(iter.next(), Some(20));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn test_pattern_engine_ingest() {
        let mut engine: PatternEngine<2> = PatternEngine::new();

        engine.ingest(ScoredBlock { score: FixedScore::from_raw(10), block_id: 1 });
        engine.ingest(ScoredBlock { score: FixedScore::from_raw(20), block_id: 2 });
        engine.ingest(ScoredBlock { score: FixedScore::from_raw(30), block_id: 3 });

        let patterns = engine.get_recent_patterns();
        assert_eq!(patterns.len(), 2);
        assert_eq!(patterns[0].block_id, 2);
        assert_eq!(patterns[1].block_id, 3);
    }
}
