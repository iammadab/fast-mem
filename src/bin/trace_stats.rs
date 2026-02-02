use std::env;

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
}

fn usage_and_exit(message: &str) -> ! {
    eprintln!("error: {}", message);
    eprintln!("usage: trace_stats <trace_path> [--page-shift N] [--top-k N]");
    std::process::exit(1);
}
