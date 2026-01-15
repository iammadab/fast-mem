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


- I increased the `BuffReader` capacity from 8KiB to 4MiB
- `read()` syscall count dropped from 5,362,938 -> 10,481
- total kernel time remained roughly 12 - 15 seconds across runs
- this suggests the current bottle neck is not syscall frequency alone
