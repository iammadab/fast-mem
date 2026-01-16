use fast_mem::MemoryEmulator;
use fast_mem::emulators::noop::NoopMem;
use fast_mem::emulators::paged::{
    PagedMemoryAHash, PagedMemoryDefault, PagedMemoryFxHash, PagedMemoryNoHashU64,
};
use fast_mem::emulators::paged_last_cache::{
    PagedMemoryCacheLast, PagedMemoryCacheLastAHash, PagedMemoryCacheLastDefault,
    PagedMemoryCacheLastFxHash, PagedMemoryCacheLastNoHashU64,
};
use fast_mem::replay_mem_operations;

fn main() {
    bench_fib(PagedMemoryDefault::default());
    bench_fib(PagedMemoryAHash::default());
    bench_fib(PagedMemoryFxHash::default());
    bench_fib(PagedMemoryNoHashU64::default());

    bench_exec_block(PagedMemoryDefault::default());
    bench_exec_block(PagedMemoryAHash::default());
    bench_exec_block(PagedMemoryFxHash::default());
    bench_exec_block(PagedMemoryNoHashU64::default());

    bench_fib(PagedMemoryCacheLastDefault::default());
    bench_fib(PagedMemoryCacheLastAHash::default());
    bench_fib(PagedMemoryCacheLastFxHash::default());
    bench_fib(PagedMemoryCacheLastNoHashU64::default());

    bench_exec_block(PagedMemoryCacheLastDefault::default());
    bench_exec_block(PagedMemoryCacheLastAHash::default());
    bench_exec_block(PagedMemoryCacheLastFxHash::default());
    bench_exec_block(PagedMemoryCacheLastNoHashU64::default());
}

fn bench_exec_block<M: MemoryEmulator>(emulator: M) {
    let label = format!("{}: exec_block", emulator.name());
    bench_memory_replay(label, "mem_bin/mem-exec-block-gc.bin", emulator);
}

fn bench_fib<M: MemoryEmulator>(emulator: M) {
    let label = format!("{}: fib", emulator.name());
    bench_memory_replay(label, "mem_bin/mem-fib-gc.bin", emulator);
}

/// Time a memory emulator against a replay file
fn bench_memory_replay<M: MemoryEmulator>(label: String, path: &'static str, mut emulator: M) {
    let start = std::time::Instant::now();
    println!("{}", label);
    replay_mem_operations(path, &mut emulator);
    let duration = start.elapsed();
    println!("{:?}", duration);
}
