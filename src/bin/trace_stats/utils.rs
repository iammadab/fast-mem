pub fn format_count(value: u64) -> String {
    let s = value.to_string();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    let mut count = 0;
    for ch in s.chars().rev() {
        if count == 3 {
            out.push(',');
            count = 0;
        }
        out.push(ch);
        count += 1;
    }
    out.chars().rev().collect()
}

pub fn record_bucket(buckets: &mut [u64], value: u64) {
    if value == 0 {
        return;
    }
    let idx = 63 - value.leading_zeros() as usize;
    if idx >= buckets.len() {
        return;
    }
    buckets[idx] += 1;
}

pub fn bucket_range(idx: usize) -> (u64, u64) {
    let min = 1u64 << idx;
    let max = (1u64 << (idx + 1)) - 1;
    (min, max)
}
