//! Resource tracking and allocation guards for search routines.
//!
//! Searches can explode combinatorially. To avoid hard OOM aborts, solvers use:
//! - counter-based budgets ([`crate::scenario::ResourceLimits`])
//! - `try_reserve` wrappers to surface allocation failures as [`crate::scenario::SearchError`]
//!
//! The tracker is intentionally lightweight: budgets are approximate but correlate strongly with
//! memory usage.

use crate::scenario::{ResourceCounts, ResourceLimits, SearchError};

#[derive(Debug, Clone)]
/// Tracks budgets/counters during a search.
pub struct ResourceTracker {
    limits: ResourceLimits,
    counts: ResourceCounts,
}

impl ResourceTracker {
    #[inline]
    pub fn new(limits: ResourceLimits) -> Self {
        Self {
            limits,
            counts: ResourceCounts::default(),
        }
    }

    #[inline]
    pub fn counts(&self) -> ResourceCounts {
        self.counts
    }

    #[inline]
    pub fn bump_states(&mut self, stage: &'static str, delta: usize) -> Result<(), SearchError> {
        self.bump(
            stage,
            "states",
            delta as u64,
            self.limits.max_states as u64,
            |c| &mut c.states,
        )
    }

    #[inline]
    pub fn bump_edges(&mut self, stage: &'static str, delta: usize) -> Result<(), SearchError> {
        self.bump(
            stage,
            "edges",
            delta as u64,
            self.limits.max_edges as u64,
            |c| &mut c.edges,
        )
    }

    #[inline]
    pub fn bump_cache_entries(
        &mut self,
        stage: &'static str,
        delta: usize,
    ) -> Result<(), SearchError> {
        self.bump(
            stage,
            "cache_entries",
            delta as u64,
            self.limits.max_cache_entries as u64,
            |c| &mut c.cache_entries,
        )
    }

    #[inline]
    pub fn dec_cache_entries(&mut self, delta: usize) {
        self.counts.cache_entries = self.counts.cache_entries.saturating_sub(delta as u64);
    }

    #[inline]
    pub fn bump_cached_moves(
        &mut self,
        stage: &'static str,
        delta: usize,
    ) -> Result<(), SearchError> {
        self.bump(
            stage,
            "cached_moves",
            delta as u64,
            self.limits.max_cached_moves as u64,
            |c| &mut c.cached_moves,
        )
    }

    #[inline]
    pub fn dec_cached_moves(&mut self, delta: usize) {
        self.counts.cached_moves = self.counts.cached_moves.saturating_sub(delta as u64);
    }

    #[inline]
    pub fn bump_steps(&mut self, stage: &'static str, delta: u64) -> Result<(), SearchError> {
        self.bump(
            stage,
            "runtime_steps",
            delta,
            self.limits.max_runtime_steps,
            |c| &mut c.runtime_steps,
        )
    }

    fn bump(
        &mut self,
        stage: &'static str,
        metric: &'static str,
        delta: u64,
        limit: u64,
        field: impl FnOnce(&mut ResourceCounts) -> &mut u64,
    ) -> Result<(), SearchError> {
        let observed = {
            let v = field(&mut self.counts);
            *v = v.saturating_add(delta);
            *v
        };

        if observed > limit {
            return Err(SearchError::LimitExceeded {
                stage,
                metric,
                limit,
                observed,
                counts: self.counts,
            });
        }

        Ok(())
    }

    pub fn try_reserve_vec<T>(
        &self,
        stage: &'static str,
        structure: &'static str,
        v: &mut Vec<T>,
        additional: usize,
    ) -> Result<(), SearchError> {
        v.try_reserve(additional)
            .map_err(|_| SearchError::AllocationFailed {
                stage,
                structure,
                counts: self.counts,
            })
    }

    pub fn try_reserve_set<K>(
        &self,
        stage: &'static str,
        structure: &'static str,
        set: &mut rustc_hash::FxHashSet<K>,
        additional: usize,
    ) -> Result<(), SearchError>
    where
        K: std::hash::Hash + Eq,
    {
        set.try_reserve(additional)
            .map_err(|_| SearchError::AllocationFailed {
                stage,
                structure,
                counts: self.counts,
            })
    }

    pub fn try_reserve_map<K, V>(
        &self,
        stage: &'static str,
        structure: &'static str,
        map: &mut rustc_hash::FxHashMap<K, V>,
        additional: usize,
    ) -> Result<(), SearchError>
    where
        K: std::hash::Hash + Eq,
    {
        map.try_reserve(additional)
            .map_err(|_| SearchError::AllocationFailed {
                stage,
                structure,
                counts: self.counts,
            })
    }
}
