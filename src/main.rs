use fast_mem::MemoryEmulator;
use fast_mem::emulators::noop::NoopMem;
use fast_mem::emulators::paged::{
    self, PagedMemory, PagedMemoryAHash, PagedMemoryFxHash, PagedMemoryNoHashU64,
};
use fast_mem::replay_mem_operations;

fn main() {
    // bench_fib("Noop: Fib", NoopMem::default());
    // bench_exec_block("Noop: Exec Block", NoopMem::default());

    // bench_fib("Paged Memory: Fib", paged::Memory::default());
    // bench_exec_block("Paged Memory: Exec Block", paged::PagedMemory::default());

    bench_fib("(Fib) Paged Memory: Ahash", PagedMemoryAHash::default());
    bench_fib("(Fib) Paged Memory: FxHash", PagedMemoryFxHash::default());
    bench_fib("(Fib) Paged Memory: Ahash", PagedMemoryNoHashU64::default());

    bench_exec_block("Paged Memory: Ahash", PagedMemoryAHash::default());
    bench_exec_block("Paged Memory: FxHash", PagedMemoryFxHash::default());
    bench_exec_block("Paged Memory: Ahash", PagedMemoryNoHashU64::default());
}

fn bench_exec_block<M: MemoryEmulator>(label: &'static str, emulator: M) {
    bench_memory_replay(label, "mem_bin/mem-exec-block-gc.bin", emulator);
}

fn bench_fib<M: MemoryEmulator>(label: &'static str, emulator: M) {
    bench_memory_replay(label, "mem_bin/mem-fib-gc.bin", emulator);
}

/// Time a memory emulator against a replay file
fn bench_memory_replay<M: MemoryEmulator>(
    label: &'static str,
    path: &'static str,
    mut emulator: M,
) {
    let start = std::time::Instant::now();
    println!("{}", label);
    replay_mem_operations(path, &mut emulator);
    let duration = start.elapsed();
    println!("{:?}", duration);
}
