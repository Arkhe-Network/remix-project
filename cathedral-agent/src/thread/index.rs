//! Mock implementation of ThreadIndex to avoid heavy missing dependencies like `rusqlite` and `lance-db`

use crate::thread::schema::{Thread, SearchFilters};

pub struct ThreadIndex {}

impl ThreadIndex {
    pub fn new() -> Result<Self, String> {
        Ok(Self {})
    }

    pub async fn index_thread(&self, _thread: &mut Thread) -> Result<(), String> {
        Ok(())
    }

    pub async fn index_batch(&self, _threads: &mut [Thread]) -> Result<Vec<String>, String> {
        Ok(vec![])
    }

    pub async fn search_hybrid(
        &self,
        _query: &str,
        _limit: usize,
        _filters: &SearchFilters,
        _k: usize,
    ) -> Result<Vec<Thread>, String> {
        Ok(vec![])
    }

    pub async fn search_textual(
        &self,
        _query: &str,
        _limit: usize,
        _filters: &SearchFilters,
    ) -> Result<Vec<Thread>, String> {
        Ok(vec![])
    }
}
