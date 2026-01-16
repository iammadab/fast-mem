### Fast Memory Emulation

A high-performance emulator for a 2⁶⁴-byte addressable memory space, tested against real workloads to measure practical performance.

#### Worklog

The emulator is benchmarked against two workloads
- Fib - 1.2 GB trace
- ExecBloc - 44 GB trace

Paged Memory
- splits the address space into 52 and 12 bits
- uses KiB pages (2^12 entries per page)
- supports up to 2^52 pages (~ 4 quadrillion possible pages)

Initial benchmark results:

```shell
Paged Memory: Fib
1.344989363s
Paged Memory: Exec Block
69.239468067s
```

Noop Memory
- to understand how much time is attributable to the paged memory implementation
  versus trace replay overhead, I added the Noop memory backend
  - decodes memory operations but does not store or load state
  - provides a lower bound on replay cost

```shell
Noop: Fib
477.280097ms
Noop: Exec Block
30.956185228s
```

Buffering Experiments
- I increased the `BuffReader` capacity from 8KiB to 4MiB
- `read()` syscall count dropped from 5,362,938 -> 10,481
- total kernel time remained roughly 12 - 15 seconds across runs
- this suggests the current bottle neck is not syscall frequency alone

Using mmap
 - I realized I was doing a lot of unnecessary copies
 - disk -> page cache (kernel) -> user buffer -> small parsing buffers
 - 44GB worth of data has to go through that pipeline
 - that is a lot of data movement!!!
 - mmap allows the process to map the trace file directly into its address space and parse in place
 - but this lead to significantly more page faults
 - the new lower bound is roughly 18 - 19s

 ```shell
Noop: Fib
481.295182ms
Noop: Exec Block
18.303799505s
```


Different Hash Functions
- the hot path in perf point to `HashMap::get` as a bottleneck
- I suspected that the hash choice might matter a lot
- trying out different hash functions I get the following benchmarks

```shell

Noop: Fib
485.764666ms
(Fib) Paged Memory: Ahash
523.321286ms
(Fib) Paged Memory: FxHash
465.683205ms
(Fib) Paged Memory: NoHashU64
466.830222ms
(Fib) Paged Memory: Default
1.026691619s

Noop: Exec Block
18.138251986s
Paged Memory: Ahash
37.702665492s
Paged Memory: FxHash
35.639297503s
Paged Memory: NoHashU64
46.41599245s
Paged Memory: Default
54.408217085s
```

- FxHash was consistently faster reducing exec block time from ~54s to ~35s
- in the future it might be worthwhile to design a hash function for my specific use case

Reducing Hashmap Accesses
- going to focus on reducing the number of times the hashmap gets hit.
- currently every memory operation hits the hashmap
- I did a quick count of the memory operations:
  - fib has 117,000,007 memory operations 
  - exec_block has 4,160,787,522 memory operations
- we hit the hashmap for every single one of these operations

- one idea is to store a pointer to the last page
- only doing a hashmap access on page transitions
- counting the number of page transitions I get the following numbers:
  - fib performs 3 page transitions
  - exec_block performs 1,443,055,930 page transitions
