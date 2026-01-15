### Fast Memory Emulation

A high-performance emulator for a 2⁶⁴-byte addressable memory space, tested against real workloads to measure practical performance.

#### Worklog

Benching against
- Fib (1.2G)
- ExecBlock (44G)

Paged Memory
- splits the address space into 52 and 12 bits
- each page has (2^12) 4096 entries and we have 2^52 (roughly 4 quadrillion) pages
- initial implementation has the following benchmark

```shell
Paged Memory: Fib
1.344989363s
Paged Memory: Exec Block
69.239468067s
```

Noop Memory
- needed to see how much of this time was coming from the paged implementation 
- vs overhead of reading the file 
- Noop only decodes the memory operation, doesn't actually track state
- so this represents a lower bound

```shell
Noop: Fib
477.280097ms
Noop: Exec Block
30.956185228s
```

- too slow

