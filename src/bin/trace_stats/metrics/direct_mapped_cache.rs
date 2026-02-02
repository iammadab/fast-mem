use crate::types::{Config, OpContext, Totals};
use crate::utils;

pub struct DirectMappedCacheStats {
    sizes: Vec<usize>,
    slots: Vec<Vec<u64>>,
    hits: Vec<u64>,
    misses: Vec<u64>,
    conflict_misses: Vec<u64>,
}

impl DirectMappedCacheStats {
    pub fn new(sizes: &[usize]) -> Self {
        let mut slots = Vec::with_capacity(sizes.len());
        let mut hits = Vec::with_capacity(sizes.len());
        let mut misses = Vec::with_capacity(sizes.len());
        let mut conflict_misses = Vec::with_capacity(sizes.len());

        for size in sizes {
            let size = *size;
            if size == 0 {
                continue;
            }
            slots.push(vec![u64::MAX; size]);
            hits.push(0);
            misses.push(0);
            conflict_misses.push(0);
        }

        let sizes = sizes.iter().copied().filter(|size| *size > 0).collect();
        Self {
            sizes,
            slots,
            hits,
            misses,
            conflict_misses,
        }
    }

    pub fn update(&mut self, ctx: &OpContext) {
        let page_id = ctx.page_id;
        for (idx, size) in self.sizes.iter().copied().enumerate() {
            let slot_idx = (page_id as usize) % size;
            let slot = &mut self.slots[idx][slot_idx];
            if *slot == page_id {
                self.hits[idx] += 1;
            } else {
                self.misses[idx] += 1;
                if *slot != u64::MAX {
                    self.conflict_misses[idx] += 1;
                }
                *slot = page_id;
            }
        }
    }

    pub fn print(&self, _totals: &Totals, _config: &Config) {
        if self.sizes.is_empty() {
            return;
        }
        println!("direct-mapped page cache sim:");
        for (idx, size) in self.sizes.iter().enumerate() {
            let hits = self.hits[idx];
            let misses = self.misses[idx];
            let total = hits + misses;
            let hit_pct = if total == 0 {
                0.0
            } else {
                (hits as f64) * 100.0 / (total as f64)
            };
            let miss_pct = if total == 0 {
                0.0
            } else {
                (misses as f64) * 100.0 / (total as f64)
            };
            let conflict = self.conflict_misses[idx];
            let conflict_pct = if misses == 0 {
                0.0
            } else {
                (conflict as f64) * 100.0 / (misses as f64)
            };
            println!(
                "  N={}: hits {} ({:.2}%), misses {} ({:.2}%), conflict {} ({:.2}% of misses)",
                size,
                utils::format_count(hits),
                hit_pct,
                utils::format_count(misses),
                miss_pct,
                utils::format_count(conflict),
                conflict_pct
            );
        }
    }
}
