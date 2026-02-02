use std::{collections::HashMap, env, fs::File};

use memmap2::Mmap;

const DEFAULT_PAGE_SHIFT: u32 = 12;
const DEFAULT_TOP_K: usize = 20;

fn main() {
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

    println!("trace_stats");
    println!("trace: {}", trace_path);
    println!("page_shift: {}", page_shift);
    println!("top_k: {}", top_k);

    let file = File::open(&trace_path).unwrap_or_else(|err| {
        eprintln!("error: failed to open {}: {}", trace_path, err);
        std::process::exit(1);
    });
    let mmap = unsafe { Mmap::map(&file).expect("mmap failed") };

    let data = &mmap[..];
    let mut pos: usize = 0;
    let mut ops: u64 = 0;
    let mut read_ops: u64 = 0;
    let mut write_ops: u64 = 0;
    let mut width_reads = [0u64; 4];
    let mut width_writes = [0u64; 4];
    let mut straddle_ops: u64 = 0;
    let mut page_counts: HashMap<u64, u64> = HashMap::new();
    let mut page_transitions: u64 = 0;
    let mut run_len_buckets = vec![0u64; 64];
    let mut current_page: Option<u64> = None;
    let mut current_run_len: u64 = 0;

    while pos + 10 <= data.len() {
        let op = data[pos];
        let width = data[pos + 1] as usize;
        if width != 1 && width != 2 && width != 4 && width != 8 {
            eprintln!("error: invalid width {} at offset {}", width, pos);
            std::process::exit(1);
        }

        let addr = u64::from_le_bytes(data[pos + 2..pos + 10].try_into().expect("addr bytes"));
        let end = addr.checked_add(width as u64 - 1).unwrap_or(u64::MAX);
        let start_page = addr >> page_shift;
        let end_page = end >> page_shift;
        if start_page != end_page {
            straddle_ops += 1;
        }
        *page_counts.entry(start_page).or_insert(0) += 1;
        match current_page {
            None => {
                current_page = Some(start_page);
                current_run_len = 1;
            }
            Some(prev) => {
                if prev == start_page {
                    current_run_len += 1;
                } else {
                    page_transitions += 1;
                    record_run_len(&mut run_len_buckets, current_run_len);
                    current_page = Some(start_page);
                    current_run_len = 1;
                }
            }
        }
        pos += 10;

        let width_idx = match width {
            1 => 0,
            2 => 1,
            4 => 2,
            8 => 3,
            _ => unreachable!(),
        };

        match op {
            1 => {
                if pos + width > data.len() {
                    eprintln!("error: unexpected end of trace at offset {}", pos);
                    std::process::exit(1);
                }
                pos += width;
                write_ops += 1;
                width_writes[width_idx] += 1;
            }
            2 => {
                read_ops += 1;
                width_reads[width_idx] += 1;
            }
            _ => {
                eprintln!("error: unknown op {} at offset {}", op, pos - 10);
                std::process::exit(1);
            }
        }

        ops += 1;
    }

    if pos != data.len() {
        eprintln!(
            "warning: {} trailing bytes at end of trace",
            data.len() - pos
        );
    }

    if current_run_len > 0 {
        record_run_len(&mut run_len_buckets, current_run_len);
    }

    println!("ops: {}", format_count(ops));
    println!(
        "bytes: {}/{}",
        format_count(pos as u64),
        format_count(data.len() as u64)
    );
    println!("reads: {}", format_count(read_ops));
    println!("writes: {}", format_count(write_ops));

    let widths = ["u8", "u16", "u32", "u64"];
    println!("width distribution (reads):");
    for (idx, width) in widths.iter().enumerate() {
        let count = width_reads[idx];
        let pct = if read_ops == 0 {
            0.0
        } else {
            (count as f64) * 100.0 / (read_ops as f64)
        };
        println!("  {}: {} ({:.2}%)", width, format_count(count), pct);
    }

    println!("width distribution (writes):");
    for (idx, width) in widths.iter().enumerate() {
        let count = width_writes[idx];
        let pct = if write_ops == 0 {
            0.0
        } else {
            (count as f64) * 100.0 / (write_ops as f64)
        };
        println!("  {}: {} ({:.2}%)", width, format_count(count), pct);
    }

    let straddle_pct = if ops == 0 {
        0.0
    } else {
        (straddle_ops as f64) * 100.0 / (ops as f64)
    };
    println!(
        "straddle ops: {} ({:.2}%)",
        format_count(straddle_ops),
        straddle_pct
    );

    let transition_pct = if ops <= 1 {
        0.0
    } else {
        (page_transitions as f64) * 100.0 / ((ops - 1) as f64)
    };
    println!(
        "page transitions: {} ({:.2}%)",
        format_count(page_transitions),
        transition_pct
    );

    println!("page run lengths:");
    for (idx, count) in run_len_buckets.iter().enumerate() {
        if *count == 0 {
            continue;
        }
        let (min, max) = bucket_range(idx);
        println!(
            "  {}-{}: {}",
            format_count(min),
            format_count(max),
            format_count(*count)
        );
    }

    if top_k > 0 {
        let mut top_pages: Vec<(u64, u64)> = page_counts.into_iter().collect();
        top_pages.sort_by(|a, b| b.1.cmp(&a.1));
        top_pages.truncate(top_k);
        println!("top pages:");
        for (page_id, count) in top_pages {
            let pct = if ops == 0 {
                0.0
            } else {
                (count as f64) * 100.0 / (ops as f64)
            };
            println!("  0x{:x}: {} ({:.2}%)", page_id, format_count(count), pct);
        }
    }
}

fn record_run_len(buckets: &mut [u64], run_len: u64) {
    if run_len == 0 {
        return;
    }
    let idx = 63 - run_len.leading_zeros() as usize;
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
