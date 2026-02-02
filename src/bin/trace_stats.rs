use std::{env, fs::File};

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

    while pos + 10 <= data.len() {
        let op = data[pos];
        let width = data[pos + 1] as usize;
        if width != 1 && width != 2 && width != 4 && width != 8 {
            eprintln!("error: invalid width {} at offset {}", width, pos);
            std::process::exit(1);
        }

        let _addr = u64::from_le_bytes(data[pos + 2..pos + 10].try_into().expect("addr bytes"));
        pos += 10;

        match op {
            1 => {
                if pos + width > data.len() {
                    eprintln!("error: unexpected end of trace at offset {}", pos);
                    std::process::exit(1);
                }
                pos += width;
            }
            2 => {}
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

    println!("ops: {}", ops);
    println!("bytes: {}/{}", pos, data.len());
}

fn usage_and_exit(message: &str) -> ! {
    eprintln!("error: {}", message);
    eprintln!("usage: trace_stats <trace_path> [--page-shift N] [--top-k N]");
    std::process::exit(1);
}
