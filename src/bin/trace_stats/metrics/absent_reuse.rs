use std::collections::{HashMap, HashSet};

use crate::types::{Config, OpContext, Totals};
use crate::utils;

pub struct AbsentReuseStats {
    present_pages: HashSet<u64>,
    absent_reads: u64,
    last_seen: HashMap<u64, u64>,
    reuse_buckets: Vec<u64>,
    cold_misses: u64,
    cache_sizes: Vec<usize>,
    cache_slots: Vec<Vec<u64>>,
    cache_hits: Vec<u64>,
    cache_misses: Vec<u64>,
    cache_conflicts: Vec<u64>,
}

impl AbsentReuseStats {
    pub fn new(cache_sizes: &[usize]) -> Self {
        let mut sizes = Vec::new();
        let mut slots = Vec::new();
        let mut hits = Vec::new();
        let mut misses = Vec::new();
        let mut conflicts = Vec::new();

        for size in cache_sizes {
            let size = *size;
            if size == 0 {
                continue;
            }
            sizes.push(size);
            slots.push(vec![u64::MAX; size]);
            hits.push(0);
            misses.push(0);
            conflicts.push(0);
        }

        Self {
            present_pages: HashSet::new(),
            absent_reads: 0,
            last_seen: HashMap::new(),
            reuse_buckets: vec![0u64; 64],
            cold_misses: 0,
            cache_sizes: sizes,
            cache_slots: slots,
            cache_hits: hits,
            cache_misses: misses,
            cache_conflicts: conflicts,
        }
    }

    pub fn update(&mut self, ctx: &OpContext) {
        match ctx.op {
            1 => {
                self.present_pages.insert(ctx.page_id);
            }
            2 => {
                if self.present_pages.contains(&ctx.page_id) {
                    return;
                }
                self.absent_reads += 1;
                match self.last_seen.insert(ctx.page_id, ctx.op_index) {
                    None => self.cold_misses += 1,
                    Some(prev) => {
                        let distance = ctx.op_index.saturating_sub(prev).max(1);
                        utils::record_bucket(&mut self.reuse_buckets, distance);
                    }
                }
                self.update_cache_sim(ctx.page_id);
            }
            _ => {}
        }
    }

    pub fn print(&self, _totals: &Totals, _config: &Config) {
        println!(
            "absent reads: {} (distinct pages: {})",
            utils::format_count(self.absent_reads),
            utils::format_count(self.last_seen.len() as u64)
        );

        let total_reuse: u64 = self.reuse_buckets.iter().sum();
        println!("absent reuse distance (ops):");
        for (idx, count) in self.reuse_buckets.iter().enumerate() {
            if *count == 0 {
                continue;
            }
            let (min, max) = utils::bucket_range(idx);
            let pct = if total_reuse == 0 {
                0.0
            } else {
                (*count as f64) * 100.0 / (total_reuse as f64)
            };
            println!(
                "  {}-{}: {} ({:.2}%)",
                utils::format_count(min),
                utils::format_count(max),
                utils::format_count(*count),
                pct
            );
        }
        println!(
            "absent cold misses: {}",
            utils::format_count(self.cold_misses)
        );

        if self.cache_sizes.is_empty() {
            return;
        }
        println!("absent direct-mapped cache sim:");
        for (idx, size) in self.cache_sizes.iter().enumerate() {
            let hits = self.cache_hits[idx];
            let misses = self.cache_misses[idx];
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
            let conflict = self.cache_conflicts[idx];
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

    fn update_cache_sim(&mut self, page_id: u64) {
        for (idx, size) in self.cache_sizes.iter().enumerate() {
            let slot_idx = (page_id as usize) % size;
            let slot = &mut self.cache_slots[idx][slot_idx];
            if *slot == page_id {
                self.cache_hits[idx] += 1;
            } else {
                self.cache_misses[idx] += 1;
                if *slot != u64::MAX {
                    self.cache_conflicts[idx] += 1;
                }
                *slot = page_id;
            }
        }
    }
}
