use crate::types::{Config, OpContext, Totals};
use crate::utils;

pub struct WidthStats {
    reads: [u64; 4],
    writes: [u64; 4],
}

impl WidthStats {
    pub fn new() -> Self {
        Self {
            reads: [0u64; 4],
            writes: [0u64; 4],
        }
    }

    pub fn update(&mut self, ctx: &OpContext) {
        match ctx.op {
            1 => self.writes[ctx.width_idx] += 1,
            2 => self.reads[ctx.width_idx] += 1,
            _ => {}
        }
    }

    pub fn print(&self, totals: &Totals, _config: &Config) {
        let widths = ["u8", "u16", "u32", "u64"];
        println!("width distribution (reads):");
        for (idx, width) in widths.iter().enumerate() {
            let count = self.reads[idx];
            let pct = if totals.read_ops == 0 {
                0.0
            } else {
                (count as f64) * 100.0 / (totals.read_ops as f64)
            };
            println!("  {}: {} ({:.2}%)", width, utils::format_count(count), pct);
        }

        println!("width distribution (writes):");
        for (idx, width) in widths.iter().enumerate() {
            let count = self.writes[idx];
            let pct = if totals.write_ops == 0 {
                0.0
            } else {
                (count as f64) * 100.0 / (totals.write_ops as f64)
            };
            println!("  {}: {} ({:.2}%)", width, utils::format_count(count), pct);
        }
    }
}
