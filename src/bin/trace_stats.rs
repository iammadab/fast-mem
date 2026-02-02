use std::{env, fs::File};

use memmap2::Mmap;

#[path = "trace_stats/metrics/mod.rs"]
mod metrics;
#[path = "trace_stats/types.rs"]
mod types;
#[path = "trace_stats/utils.rs"]
mod utils;

use metrics::{page_locality::PageLocalityStats, reuse_distance::ReuseDistanceStats};
use metrics::{straddle::StraddleStats, width::WidthStats};
use types::{Config, OpContext, Totals};

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

    let (totals, mut stats) = parse_trace(&mmap, &config);
    stats.finish();
    print_report(&totals, &stats, &config);
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
    (totals, stats)
}

fn print_report(totals: &Totals, stats: &Stats, config: &Config) {
    println!("ops: {}", utils::format_count(totals.ops));
    println!(
        "bytes: {}/{}",
        utils::format_count(totals.bytes_consumed as u64),
        utils::format_count(totals.bytes_total as u64)
    );
    println!("reads: {}", utils::format_count(totals.read_ops));
    println!("writes: {}", utils::format_count(totals.write_ops));

    stats.width.print(totals, config);
    stats.straddle.print(totals, config);
    stats.page_locality.print(totals, config);
    stats.reuse_distance.print(totals, config);
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

fn usage_and_exit(message: &str) -> ! {
    eprintln!("error: {}", message);
    eprintln!("usage: trace_stats <trace_path> [--page-shift N] [--top-k N]");
    std::process::exit(1);
}
