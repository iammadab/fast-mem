use fast_mem::paged;
use fast_mem::replay_mem_operations;

fn main() {
    let mut paged_memory = paged::Memory::default();
    // replay_mem_operations("mem_bin/mem-fib-gc.bin", &mut paged_memory);
    replay_mem_operations("mem_bin/mem-exec-block-gc.bin", &mut paged_memory);
}
