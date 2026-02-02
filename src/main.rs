use fast_mem::emulators::noop::NoopMem;
use fast_mem::emulators::paged::{
    PagedMemoryAHash, PagedMemoryDefault, PagedMemoryFxHash, PagedMemoryNoHashU64,
};
use fast_mem::emulators::paged_last_cache::{
    PagedMemoryCacheLast, PagedMemoryCacheLastAHash, PagedMemoryCacheLastDefault,
    PagedMemoryCacheLastFxHash, PagedMemoryCacheLastNoHashU64,
};
use fast_mem::replay_mem_operations;
use fast_mem::MemoryEmulator;

fn main() {
    let perf_mode = std::env::args().any(|arg| arg == "--perf");
    if perf_mode {
        bench_exec_block(PagedMemoryFxHash::default());
        return;
    }

    bench_fib(PagedMemoryFxHash::default());
    bench_exec_block(PagedMemoryFxHash::default());
    bench_fib(PagedMemoryCacheLastFxHash::default());
    bench_exec_block(PagedMemoryCacheLastFxHash::default());
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
    emulator.finish();
}
