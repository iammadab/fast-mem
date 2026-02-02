use std::collections::HashMap;

use crate::types::{Config, OpContext, Totals};
use crate::utils;

pub struct PageLocalityStats {
    page_counts: HashMap<u64, u64>,
    transitions: u64,
    run_len_buckets: Vec<u64>,
    current_page: Option<u64>,
    current_run_len: u64,
}

impl PageLocalityStats {
    pub fn new() -> Self {
        Self {
            page_counts: HashMap::new(),
            transitions: 0,
            run_len_buckets: vec![0u64; 64],
            current_page: None,
            current_run_len: 0,
        }
    }

    pub fn update(&mut self, ctx: &OpContext) {
        *self.page_counts.entry(ctx.page_id).or_insert(0) += 1;
        match self.current_page {
            None => {
                self.current_page = Some(ctx.page_id);
                self.current_run_len = 1;
            }
            Some(prev) => {
                if prev == ctx.page_id {
                    self.current_run_len += 1;
                } else {
                    self.transitions += 1;
                    utils::record_bucket(&mut self.run_len_buckets, self.current_run_len);
                    self.current_page = Some(ctx.page_id);
                    self.current_run_len = 1;
                }
            }
        }
    }

    pub fn finish(&mut self) {
        if self.current_run_len > 0 {
            utils::record_bucket(&mut self.run_len_buckets, self.current_run_len);
        }
    }

    pub fn print(&self, totals: &Totals, config: &Config) {
        let transition_pct = if totals.ops <= 1 {
            0.0
        } else {
            (self.transitions as f64) * 100.0 / ((totals.ops - 1) as f64)
        };
        println!(
            "page transitions: {} ({:.2}%)",
            utils::format_count(self.transitions),
            transition_pct
        );

        let total_runs: u64 = self.run_len_buckets.iter().sum();
        println!("page run lengths:");
        for (idx, count) in self.run_len_buckets.iter().enumerate() {
            if *count == 0 {
                continue;
            }
            let (min, max) = utils::bucket_range(idx);
            let pct = if total_runs == 0 {
                0.0
            } else {
                (*count as f64) * 100.0 / (total_runs as f64)
            };
            println!(
                "  {}-{}: {} ({:.2}%)",
                utils::format_count(min),
                utils::format_count(max),
                utils::format_count(*count),
                pct
            );
        }

        if config.top_k > 0 {
            let mut top_pages: Vec<(u64, u64)> =
                self.page_counts.iter().map(|(k, v)| (*k, *v)).collect();
            top_pages.sort_by(|a, b| b.1.cmp(&a.1));
            top_pages.truncate(config.top_k);
            println!("top pages:");
            for (page_id, count) in top_pages {
                let pct = if totals.ops == 0 {
                    0.0
                } else {
                    (count as f64) * 100.0 / (totals.ops as f64)
                };
                println!(
                    "  0x{:x}: {} ({:.2}%)",
                    page_id,
                    utils::format_count(count),
                    pct
                );
            }
        }
    }
}
