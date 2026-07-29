// ═══════════════════════════════════════════════════════════════════════════════
// pattern_engine.rs — Substrato 1300.5: PatternEngine Extension
// Selo: CATHEDRAL-1300.5-PATTERN-ENGINE-v1.0.0-2026-06-13
// Arquiteto: ORCID 0009-0005-2697-4668
// ═══════════════════════════════════════════════════════════════════════════════

use alloc::vec::Vec;
use alloc::collections::VecDeque;

/// Um padrao extraido
#[derive(Debug, Clone, PartialEq)]
pub struct ExtractedPattern {
    pub pattern_id: u64,
    pub confidence: f64,
    pub occurrence_count: usize,
}

pub struct SlidingCache {
    capacity: usize,
    items: VecDeque<u64>,
}

impl SlidingCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            items: VecDeque::with_capacity(capacity),
        }
    }

    pub fn insert(&mut self, item: u64) {
        if self.items.len() == self.capacity {
            self.items.pop_front();
        }
        self.items.push_back(item);
    }

    pub fn contains(&self, item: u64) -> bool {
        self.items.contains(&item)
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }
}

pub struct PatternEngine {
    cache: SlidingCache,
}

impl PatternEngine {
    pub fn new(cache_capacity: usize) -> Self {
        Self {
            cache: SlidingCache::new(cache_capacity),
        }
    }

    pub fn process_stream(&mut self, data: &[u64]) -> Vec<ExtractedPattern> {
        let mut patterns = Vec::new();
        for &item in data {
            if self.cache.contains(item) {
                patterns.push(ExtractedPattern {
                    pattern_id: item,
                    confidence: 0.95,
                    occurrence_count: 2, // simplified
                });
            }
            self.cache.insert(item);
        }
        patterns
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sliding_cache() {
        let mut cache = SlidingCache::new(3);
        cache.insert(1);
        cache.insert(2);
        cache.insert(3);
        assert!(cache.contains(1));
        assert!(cache.contains(2));
        assert!(cache.contains(3));

        cache.insert(4);
        assert!(!cache.contains(1));
        assert!(cache.contains(4));
        assert_eq!(cache.len(), 3);
    }

    #[test]
    fn test_pattern_engine() {
        let mut engine = PatternEngine::new(5);
        let stream = alloc::vec![10, 20, 30, 20, 40, 10]; // 20 and 10 will repeat within cache window
        let patterns = engine.process_stream(&stream);

        assert_eq!(patterns.len(), 2);
        assert_eq!(patterns[0].pattern_id, 20);
        assert_eq!(patterns[1].pattern_id, 10);
    }
}
