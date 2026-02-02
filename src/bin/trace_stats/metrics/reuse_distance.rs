// Measures how many operations occur between two accesses to the same page
// (op-distance reuse), which indicates how effective a small page cache could be.
use std::collections::HashMap;

use crate::types::{Config, OpContext, Totals};
use crate::utils;

// Buckets reuse distances and counts first-time (cold) page touches.
pub struct ReuseDistanceStats {
    reuse_buckets: Vec<u64>,
    last_seen: HashMap<u64, u64>,
    cold_misses: u64,
}

impl ReuseDistanceStats {
    pub fn new() -> Self {
        Self {
            reuse_buckets: vec![0u64; 64],
            last_seen: HashMap::new(),
            cold_misses: 0,
        }
    }

    pub fn update(&mut self, ctx: &OpContext) {
        match self.last_seen.insert(ctx.page_id, ctx.op_index) {
            None => self.cold_misses += 1,
            Some(prev) => {
                let distance = ctx.op_index.saturating_sub(prev).max(1);
                utils::record_bucket(&mut self.reuse_buckets, distance);
            }
        }
    }

    pub fn print(&self, _totals: &Totals, _config: &Config) {
        let total_reuse: u64 = self.reuse_buckets.iter().sum();
        println!("reuse distance (ops):");
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
        println!("cold misses: {}", utils::format_count(self.cold_misses));
    }
}
