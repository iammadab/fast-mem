# Trace Stats

`trace_stats` is a standalone binary for analyzing memory traces. It parses the trace file and reports width distribution, page straddles, page locality (transitions + run-lengths + hot pages), and reuse distance (op-distance between page touches).

## Usage

```shell
cargo run --release --bin trace_stats -- <trace_path> [--page-shift 12] [--top-k 20] [--cache-sizes 4,8,16,32]
```

Flags:

- `--page-shift N`: page size used for locality metrics (`2^N` bytes per page). Default is 12 (4 KiB).
- `--top-k N`: number of hot pages to report. Use `0` to disable. Default is 20.
- `--cache-sizes N,N`: comma-separated sizes for direct-mapped cache simulation. Defaults to `4,8,16,32`.

## Trace format

Each op begins with a 10-byte header:

- `op` (u8): `1` = write, `2` = read
- `width` (u8): `1`, `2`, `4`, or `8`
- `addr` (u64 LE): address

For writes, the header is followed by `width` data bytes.

## Metrics

### Width distribution

Counts reads and writes by access width (u8/u16/u32/u64). This shows which access sizes dominate the trace.

### Straddle ops

Counts operations that cross a page boundary (where `addr..addr+width-1` spans two pages). This is the fraction of ops that cannot be satisfied within a single page.

### Page locality

Tracks locality at the page level:

- page transitions: how often consecutive ops move to a different page.
- run lengths: histogram of consecutive accesses that stay on the same page.
- top pages: the hottest pages by access count (configurable with `--top-k`).

### Reuse distance

Measures op-distance between two accesses to the same page. Buckets the distance in powers of two and counts first-time touches as cold misses. This approximates how effective small page caches might be.

### Direct-mapped cache simulation

Simulates direct-mapped page caches at the configured sizes and reports hit/miss rates plus conflict misses. This helps estimate whether a tiny page cache is likely to pay off.
