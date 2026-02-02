// Counts operations that cross a page boundary (addr..addr+width-1 spans two pages).
use crate::types::{Config, OpContext, Totals};
use crate::utils;

// Number of straddling operations.
pub struct StraddleStats {
    count: u64,
}

impl StraddleStats {
    pub fn new() -> Self {
        Self { count: 0 }
    }

    pub fn update(&mut self, ctx: &OpContext) {
        if ctx.page_id != ctx.end_page_id {
            self.count += 1;
        }
    }

    pub fn print(&self, totals: &Totals, _config: &Config) {
        let pct = if totals.ops == 0 {
            0.0
        } else {
            (self.count as f64) * 100.0 / (totals.ops as f64)
        };
        println!(
            "straddle ops: {} ({:.2}%)",
            utils::format_count(self.count),
            pct
        );
    }
}
