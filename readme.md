### Fast Memory Emulation

A high-performance emulator for a 2⁶⁴-byte addressable memory space, tested against real workloads to measure practical performance.

### Trace Stats

Docs: `src/bin/trace_stats/README.md`

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
- the next optimization focus is reducing the number of `HashMap` accesses.
- currently every memory operation hits the hashmap
- I did a quick count of the memory operations:
  - fib has 117,000,007 memory operations 
  - exec_block has 4,160,787,522 memory operations
- with the current design, the `HashMap` is hit once for every one of these operations

Last-Page Caching
- one idea is to store a pointer to the last page
- only perform a hashmap access on page transitions
- counting the number of page transitions, I get the following numbers:
  - fib performs 3 page transitions
  - exec_block performs 1,443,055,930 page transitions
- I implemented this and saw practically no improvement:

```shell
PagedMemCacheLast(SipHash): fib
1.32553868s
PagedMemCacheLast(AHash): fib
550.687916ms
PagedMemCacheLast(FxHash): fib
496.418719ms
PagedMemCacheLast(NoHashU64): fib
550.409934ms

PagedMemCacheLast(SipHash): exec_block
49.326271911s
PagedMemCacheLast(AHash): exec_block
36.771238637s
PagedMemCacheLast(FxHash): exec_block
34.023815799s
PagedMemCacheLast(NoHashU64): exec_block
46.605353459s
```


Measuring cache hit and miss

```shell
PagedMemCacheLast(FxHash): exec_block
34.927876887s
cache hit: 613,899,438
cache miss: 3,546,888,084
total: 4,160,787,522
```

- I expected to have only `1,443,055,930` cache misses

Figured out the issue
- when a read is attempted to a page that doesn't exist
- I return 0 as the value but I don't create the page
- so if that address is hit again on the next operation it will still consider it a cache miss
- I changed that and the cache profile is now as follows:

```shell
PagedMemCacheLast(FxHash): exec_block
38.607080239s
cache hit: 2,717,731,592
cache miss: 1,443,055,930
total: 4,160,787,522
```

Fixed-width Fast Paths
- the hot path was dominated by read_into
- most accesses are single-page, so we can avoid the generic read/write loop
- added specialized load/store fast paths for 1/2/4/8-byte ops when the access stays within a page
- only fall back to read_n_bytes/write_n_bytes on cross-page accesses

```shell
PagedMem(FxHash): fib
664.94949ms
PagedMem(FxHash): exec_block
29.649904728s

PagedMemCacheLast(FxHash): fib
704.275774ms
PagedMemCacheLast(FxHash): exec_block
32.209270595s
```

- perf now shows read_into ~0.1%; most time is in replay loop/inlined fast path

More visibility into trace structure. 
- implemented a trace analyzer to get some trace structure metrics
- width distribution: how often reads and writes use 1/2/4/8-byte accesses.
- straddle ops: how many accesses cross a page boundary (so they touch two pages).
- page locality: how frequently we stay on the same page vs jump, plus the length of same-page runs and the hottest pages.
- reuse distance: how many operations occur before we touch the same page again, plus how many pages are seen for the first time.

- Fib output
```shell
trace: mem_bin/mem-fib-gc.bin
page_shift: 12
top_k: 20
ops: 117,000,007
bytes: 1,170,000,078/1,170,000,078
reads: 117,000,006
writes: 1
width distribution (reads):
  u8: 0 (0.00%)
  u16: 117,000,006 (100.00%)
  u32: 0 (0.00%)
  u64: 0 (0.00%)
width distribution (writes):
  u8: 0 (0.00%)
  u16: 0 (0.00%)
  u32: 0 (0.00%)
  u64: 1 (100.00%)
straddle ops: 0 (0.00%)
page transitions: 2 (0.00%)
page run lengths:
  1-1: 1 (33.33%)
  2-3: 1 (33.33%)
  67,108,864-134,217,727: 1 (33.33%)
top pages:
  0x11: 117,000,006 (100.00%)
  0x7fff: 1 (0.00%)
reuse distance (ops):
  1-1: 117,000,004 (100.00%)
  2-3: 1 (0.00%)
cold misses: 2
```

- Exec block output
```shell
trace: mem_bin/mem-exec-block-gc.bin
page_shift: 12
top_k: 20
ops: 4,160,787,522
bytes: 43,933,138,505/43,933,138,505
reads: 3,849,865,371
writes: 310,922,151
width distribution (reads):
  u8: 65,089,222 (1.69%)
  u16: 3,448,063,617 (89.56%)
  u32: 549,584 (0.01%)
  u64: 336,162,948 (8.73%)
width distribution (writes):
  u8: 22,434,883 (7.22%)
  u16: 450,387 (0.14%)
  u32: 591,855 (0.19%)
  u64: 287,445,026 (92.45%)
straddle ops: 0 (0.00%)
page transitions: 1,443,055,929 (34.68%)
page run lengths:
  1-1: 849,404,106 (58.86%)
  2-3: 270,243,888 (18.73%)
  4-7: 240,149,816 (16.64%)
  8-15: 58,637,050 (4.06%)
  16-31: 14,492,186 (1.00%)
  32-63: 8,265,447 (0.57%)
  64-127: 1,383,643 (0.10%)
  128-255: 72,849 (0.01%)
  256-511: 406,945 (0.03%)
top pages:
  0x149: 722,254,357 (17.36%)
  0x14a: 399,262,644 (9.60%)
  0x7ff9: 390,732,648 (9.39%)
  0xd2: 223,477,760 (5.37%)
  0xd3: 223,477,760 (5.37%)
  0xd4: 223,477,760 (5.37%)
  0xd5: 223,477,760 (5.37%)
  0xd1: 220,091,542 (5.29%)
  0xcb: 173,764,795 (4.18%)
  0xd8: 171,842,176 (4.13%)
  0x7ffc: 136,204,425 (3.27%)
  0xd7: 135,402,234 (3.25%)
  0xcd: 125,896,133 (3.03%)
  0xd6: 111,339,910 (2.68%)
  0x6d: 96,470,041 (2.32%)
  0xcc: 75,769,658 (1.82%)
  0x140: 59,805,294 (1.44%)
  0x143: 56,440,047 (1.36%)
  0x7ffb: 25,494,204 (0.61%)
  0x7ff8: 25,089,357 (0.60%)
reuse distance (ops):
  1-1: 2,717,731,592 (65.32%)
  2-3: 971,311,152 (23.34%)
  4-7: 256,260,494 (6.16%)
  8-15: 135,106,344 (3.25%)
  16-31: 41,180,762 (0.99%)
  32-63: 14,106,780 (0.34%)
  64-127: 5,744,887 (0.14%)
  128-255: 4,570,759 (0.11%)
  256-511: 7,485,959 (0.18%)
  512-1,023: 2,477,161 (0.06%)
  1,024-2,047: 846,217 (0.02%)
  2,048-4,095: 644,636 (0.02%)
  4,096-8,191: 685,800 (0.02%)
  8,192-16,383: 1,438,474 (0.03%)
  16,384-32,767: 565,184 (0.01%)
  32,768-65,535: 350,155 (0.01%)
  65,536-131,071: 101,186 (0.00%)
  131,072-262,143: 37,354 (0.00%)
  262,144-524,287: 27,648 (0.00%)
  524,288-1,048,575: 20,948 (0.00%)
  1,048,576-2,097,151: 10,688 (0.00%)
  2,097,152-4,194,303: 20,828 (0.00%)
  4,194,304-8,388,607: 13,859 (0.00%)
  8,388,608-16,777,215: 7,664 (0.00%)
  16,777,216-33,554,431: 4,263 (0.00%)
  33,554,432-67,108,863: 3,559 (0.00%)
  67,108,864-134,217,727: 4,540 (0.00%)
  134,217,728-268,435,455: 6,610 (0.00%)
  268,435,456-536,870,911: 4,484 (0.00%)
  536,870,912-1,073,741,823: 1,785 (0.00%)
  1,073,741,824-2,147,483,647: 130 (0.00%)
  2,147,483,648-4,294,967,295: 1,133 (0.00%)
cold misses: 14,487
```

Why a small page cache should help (from perf + trace stats):
- perf shows hashbrown lookup + hashing dominates the hot path, meaning page table lookup is the main cost per op.
- proposed fix: a tiny cache mapping `page_id -> page pointer` to bypass the HashMap on hits.
- trace stats support this: reuse distance is very short (≈65% after 1 op, ≈88% within 3 ops) and a few pages are extremely hot.
- straddles are 0%, so each op needs only one page lookup—every cache hit directly removes the dominant cost.
- fib is almost a single-page trace, so a last-N cache should nearly eliminate lookups after warmup.

Direct-mapped page cache simulation results.
- Fib cache sim
```shell
direct-mapped page cache sim:
  N=4: hits 117,000,005 (100.00%), misses 2 (0.00%), conflict 0 (0.00% of misses)
  N=8: hits 117,000,005 (100.00%), misses 2 (0.00%), conflict 0 (0.00% of misses)
  N=16: hits 117,000,005 (100.00%), misses 2 (0.00%), conflict 0 (0.00% of misses)
  N=32: hits 117,000,005 (100.00%), misses 2 (0.00%), conflict 0 (0.00% of misses)
```
- Exec block cache sim
```shell
direct-mapped page cache sim:
  N=4: hits 3,749,153,831 (90.11%), misses 411,633,691 (9.89%), conflict 411,633,687 (100.00% of misses)
  N=8: hits 3,897,780,430 (93.68%), misses 263,007,092 (6.32%), conflict 263,007,084 (100.00% of misses)
  N=16: hits 4,086,823,701 (98.22%), misses 73,963,821 (1.78%), conflict 73,963,805 (100.00% of misses)
  N=32: hits 4,140,204,127 (99.51%), misses 20,583,395 (0.49%), conflict 20,583,363 (100.00% of misses)
```

Absent-read reuse metrics (read-only semantics).
- fib: 117,000,006 absent reads across 1 page; reuse is ~100% at distance 1; absent cache sim hits ~100% even at N=4.
- exec_block: 3,464,970,427 absent reads across 236 pages; reuse is very tight (78.43% at distance 1, 20.58% at 2–3).
- absent direct-mapped cache sim: hit rates are ~99.8–99.9% for N=4–32 on the absent-only stream.
- takeaway: a negative cache should eliminate most HashMap lookups for absent reads and is likely to deliver large gains.

Negative cache enabled (read-only semantics).
- baseline vs cache32 (FxHash):
  - PagedMem(FxHash): fib 701.883639ms, exec_block 26.61158114s.
  - PagedMemCache32(FxHash): fib 607.409128ms, exec_block 24.041621447s.
- cache size comparison (FxHash):
  - PagedMemCache4(FxHash): fib 576.994677ms, exec_block 25.13949827s.
  - PagedMemCache8(FxHash): fib 563.77048ms, exec_block 30.588335483s.
  - PagedMemCache16(FxHash): fib 993.117062ms, exec_block 37.714374599s.
  - PagedMemCache32(FxHash): fib 673.450707ms, exec_block 24.587162403s.
