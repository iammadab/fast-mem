pub struct Config {
    pub trace_path: String,
    pub page_shift: u32,
    pub top_k: usize,
    pub cache_sizes: Vec<usize>,
}

pub struct Totals {
    pub ops: u64,
    pub read_ops: u64,
    pub write_ops: u64,
    pub bytes_consumed: usize,
    pub bytes_total: usize,
}

pub struct OpContext {
    pub op: u8,
    pub width: usize,
    pub width_idx: usize,
    pub addr: u64,
    pub page_id: u64,
    pub end_page_id: u64,
    pub op_index: u64,
}
