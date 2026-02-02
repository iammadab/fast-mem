use std::{collections::HashMap, env, fs::File};

use memmap2::Mmap;

const DEFAULT_PAGE_SHIFT: u32 = 12;
const DEFAULT_TOP_K: usize = 20;

fn main() {
    let config = parse_args();

    println!("trace_stats");
    println!("trace: {}", config.trace_path);
    println!("page_shift: {}", config.page_shift);
    println!("top_k: {}", config.top_k);

    let file = File::open(&config.trace_path).unwrap_or_else(|err| {
        eprintln!("error: failed to open {}: {}", config.trace_path, err);
        std::process::exit(1);
    });
    let mmap = unsafe { Mmap::map(&file).expect("mmap failed") };

    let (totals, stats) = parse_trace(&mmap, &config);
    print_report(&totals, &stats, &config);
}

struct Config {
    trace_path: String,
    page_shift: u32,
    top_k: usize,
}

struct Totals {
    ops: u64,
    read_ops: u64,
    write_ops: u64,
    bytes_consumed: usize,
    bytes_total: usize,
}

struct OpContext {
    op: u8,
    width: usize,
    width_idx: usize,
    addr: u64,
    page_id: u64,
    end_page_id: u64,
    op_index: u64,
}

struct Stats {
    width: WidthStats,
    straddle: StraddleStats,
    page_locality: PageLocalityStats,
    reuse_distance: ReuseDistanceStats,
}

impl Stats {
    fn new() -> Self {
        Self {
            width: WidthStats::new(),
            straddle: StraddleStats::new(),
            page_locality: PageLocalityStats::new(),
            reuse_distance: ReuseDistanceStats::new(),
        }
    }

    fn update(&mut self, ctx: &OpContext) {
        self.width.update(ctx);
        self.straddle.update(ctx);
        self.page_locality.update(ctx);
        self.reuse_distance.update(ctx);
    }

    fn finish(&mut self) {
        self.page_locality.finish();
    }
}

struct WidthStats {
    reads: [u64; 4],
    writes: [u64; 4],
}

impl WidthStats {
    fn new() -> Self {
        Self {
            reads: [0u64; 4],
            writes: [0u64; 4],
        }
    }

    fn update(&mut self, ctx: &OpContext) {
        match ctx.op {
            1 => self.writes[ctx.width_idx] += 1,
            2 => self.reads[ctx.width_idx] += 1,
            _ => {}
        }
    }
}

struct StraddleStats {
    count: u64,
}

impl StraddleStats {
    fn new() -> Self {
        Self { count: 0 }
    }

    fn update(&mut self, ctx: &OpContext) {
        if ctx.page_id != ctx.end_page_id {
            self.count += 1;
        }
    }
}

struct PageLocalityStats {
    page_counts: HashMap<u64, u64>,
    transitions: u64,
    run_len_buckets: Vec<u64>,
    current_page: Option<u64>,
    current_run_len: u64,
}

impl PageLocalityStats {
    fn new() -> Self {
        Self {
            page_counts: HashMap::new(),
            transitions: 0,
            run_len_buckets: vec![0u64; 64],
            current_page: None,
            current_run_len: 0,
        }
    }

    fn update(&mut self, ctx: &OpContext) {
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
                    record_bucket(&mut self.run_len_buckets, self.current_run_len);
                    self.current_page = Some(ctx.page_id);
                    self.current_run_len = 1;
                }
            }
        }
    }

    fn finish(&mut self) {
        if self.current_run_len > 0 {
            record_bucket(&mut self.run_len_buckets, self.current_run_len);
        }
    }
}

struct ReuseDistanceStats {
    reuse_buckets: Vec<u64>,
    last_seen: HashMap<u64, u64>,
    cold_misses: u64,
}

impl ReuseDistanceStats {
    fn new() -> Self {
        Self {
            reuse_buckets: vec![0u64; 64],
            last_seen: HashMap::new(),
            cold_misses: 0,
        }
    }

    fn update(&mut self, ctx: &OpContext) {
        match self.last_seen.insert(ctx.page_id, ctx.op_index) {
            None => self.cold_misses += 1,
            Some(prev) => {
                let distance = ctx.op_index.saturating_sub(prev).max(1);
                record_bucket(&mut self.reuse_buckets, distance);
            }
        }
    }
}

fn parse_args() -> Config {
    let mut args = env::args().skip(1);
    let mut trace_path: Option<String> = None;
    let mut page_shift = DEFAULT_PAGE_SHIFT;
    let mut top_k = DEFAULT_TOP_K;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--page-shift" => {
                let value = args.next().unwrap_or_else(|| {
                    usage_and_exit("missing value for --page-shift");
                });
                page_shift = value.parse::<u32>().unwrap_or_else(|_| {
                    usage_and_exit("invalid value for --page-shift");
                });
            }
            "--top-k" => {
                let value = args.next().unwrap_or_else(|| {
                    usage_and_exit("missing value for --top-k");
                });
                top_k = value.parse::<usize>().unwrap_or_else(|_| {
                    usage_and_exit("invalid value for --top-k");
                });
            }
            _ => {
                if trace_path.is_some() {
                    usage_and_exit("unexpected extra argument");
                }
                trace_path = Some(arg);
            }
        }
    }

    let trace_path = trace_path.unwrap_or_else(|| {
        usage_and_exit("missing trace path");
    });

    Config {
        trace_path,
        page_shift,
        top_k,
    }
}

fn parse_trace(data: &[u8], config: &Config) -> (Totals, Stats) {
    let mut pos: usize = 0;
    let mut op_index: u64 = 0;
    let mut totals = Totals {
        ops: 0,
        read_ops: 0,
        write_ops: 0,
        bytes_consumed: 0,
        bytes_total: data.len(),
    };
    let mut stats = Stats::new();

    while pos + 10 <= data.len() {
        let op = data[pos];
        let width = data[pos + 1] as usize;
        if width != 1 && width != 2 && width != 4 && width != 8 {
            eprintln!("error: invalid width {} at offset {}", width, pos);
            std::process::exit(1);
        }
        let width_idx = width_index(width);

        let addr = u64::from_le_bytes(data[pos + 2..pos + 10].try_into().expect("addr bytes"));
        let end = addr.checked_add(width as u64 - 1).unwrap_or(u64::MAX);
        let page_id = addr >> config.page_shift;
        let end_page_id = end >> config.page_shift;

        let ctx = OpContext {
            op,
            width,
            width_idx,
            addr,
            page_id,
            end_page_id,
            op_index,
        };

        stats.update(&ctx);

        pos += 10;
        match op {
            1 => {
                if pos + width > data.len() {
                    eprintln!("error: unexpected end of trace at offset {}", pos);
                    std::process::exit(1);
                }
                pos += width;
                totals.write_ops += 1;
            }
            2 => {
                totals.read_ops += 1;
            }
            _ => {
                eprintln!("error: unknown op {} at offset {}", op, pos - 10);
                std::process::exit(1);
            }
        }

        totals.ops += 1;
        op_index += 1;
    }

    if pos != data.len() {
        eprintln!(
            "warning: {} trailing bytes at end of trace",
            data.len() - pos
        );
    }

    totals.bytes_consumed = pos;
    stats.finish();

    (totals, stats)
}

fn print_report(totals: &Totals, stats: &Stats, config: &Config) {
    println!("ops: {}", format_count(totals.ops));
    println!(
        "bytes: {}/{}",
        format_count(totals.bytes_consumed as u64),
        format_count(totals.bytes_total as u64)
    );
    println!("reads: {}", format_count(totals.read_ops));
    println!("writes: {}", format_count(totals.write_ops));

    print_width_distribution(&stats.width, totals);

    let straddle_pct = if totals.ops == 0 {
        0.0
    } else {
        (stats.straddle.count as f64) * 100.0 / (totals.ops as f64)
    };
    println!(
        "straddle ops: {} ({:.2}%)",
        format_count(stats.straddle.count),
        straddle_pct
    );

    print_page_locality(&stats.page_locality, totals, config.top_k);
    print_reuse_distance(&stats.reuse_distance);
}

fn print_width_distribution(stats: &WidthStats, totals: &Totals) {
    let widths = ["u8", "u16", "u32", "u64"];
    println!("width distribution (reads):");
    for (idx, width) in widths.iter().enumerate() {
        let count = stats.reads[idx];
        let pct = if totals.read_ops == 0 {
            0.0
        } else {
            (count as f64) * 100.0 / (totals.read_ops as f64)
        };
        println!("  {}: {} ({:.2}%)", width, format_count(count), pct);
    }

    println!("width distribution (writes):");
    for (idx, width) in widths.iter().enumerate() {
        let count = stats.writes[idx];
        let pct = if totals.write_ops == 0 {
            0.0
        } else {
            (count as f64) * 100.0 / (totals.write_ops as f64)
        };
        println!("  {}: {} ({:.2}%)", width, format_count(count), pct);
    }
}

fn print_page_locality(stats: &PageLocalityStats, totals: &Totals, top_k: usize) {
    let transition_pct = if totals.ops <= 1 {
        0.0
    } else {
        (stats.transitions as f64) * 100.0 / ((totals.ops - 1) as f64)
    };
    println!(
        "page transitions: {} ({:.2}%)",
        format_count(stats.transitions),
        transition_pct
    );

    let total_runs: u64 = stats.run_len_buckets.iter().sum();
    println!("page run lengths:");
    for (idx, count) in stats.run_len_buckets.iter().enumerate() {
        if *count == 0 {
            continue;
        }
        let (min, max) = bucket_range(idx);
        let pct = if total_runs == 0 {
            0.0
        } else {
            (*count as f64) * 100.0 / (total_runs as f64)
        };
        println!(
            "  {}-{}: {} ({:.2}%)",
            format_count(min),
            format_count(max),
            format_count(*count),
            pct
        );
    }

    if top_k > 0 {
        let mut top_pages: Vec<(u64, u64)> =
            stats.page_counts.iter().map(|(k, v)| (*k, *v)).collect();
        top_pages.sort_by(|a, b| b.1.cmp(&a.1));
        top_pages.truncate(top_k);
        println!("top pages:");
        for (page_id, count) in top_pages {
            let pct = if totals.ops == 0 {
                0.0
            } else {
                (count as f64) * 100.0 / (totals.ops as f64)
            };
            println!("  0x{:x}: {} ({:.2}%)", page_id, format_count(count), pct);
        }
    }
}

fn print_reuse_distance(stats: &ReuseDistanceStats) {
    let total_reuse: u64 = stats.reuse_buckets.iter().sum();
    println!("reuse distance (ops):");
    for (idx, count) in stats.reuse_buckets.iter().enumerate() {
        if *count == 0 {
            continue;
        }
        let (min, max) = bucket_range(idx);
        let pct = if total_reuse == 0 {
            0.0
        } else {
            (*count as f64) * 100.0 / (total_reuse as f64)
        };
        println!(
            "  {}-{}: {} ({:.2}%)",
            format_count(min),
            format_count(max),
            format_count(*count),
            pct
        );
    }
    println!("cold misses: {}", format_count(stats.cold_misses));
}

fn width_index(width: usize) -> usize {
    match width {
        1 => 0,
        2 => 1,
        4 => 2,
        8 => 3,
        _ => unreachable!(),
    }
}

fn record_bucket(buckets: &mut [u64], value: u64) {
    if value == 0 {
        return;
    }
    let idx = 63 - value.leading_zeros() as usize;
    if idx >= buckets.len() {
        return;
    }
    buckets[idx] += 1;
}

fn bucket_range(idx: usize) -> (u64, u64) {
    let min = 1u64 << idx;
    let max = (1u64 << (idx + 1)) - 1;
    (min, max)
}

fn format_count(value: u64) -> String {
    let s = value.to_string();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    let mut count = 0;
    for ch in s.chars().rev() {
        if count == 3 {
            out.push(',');
            count = 0;
        }
        out.push(ch);
        count += 1;
    }
    out.chars().rev().collect()
}

fn usage_and_exit(message: &str) -> ! {
    eprintln!("error: {}", message);
    eprintln!("usage: trace_stats <trace_path> [--page-shift N] [--top-k N]");
    std::process::exit(1);
}
