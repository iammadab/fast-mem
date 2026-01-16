use std::{collections::HashMap, ptr::NonNull};

use crate::named_hasher::NamedHasher;

/// Number of bits to describe entries in a page
const PAGE_SHIFT: u64 = 12;
/// Total number of entries in a page
const PAGE_SIZE: usize = 1 << PAGE_SHIFT;
/// Mask to get the last `PAGE_SHIFT` bits of an address
const PAGE_MASK: u64 = (PAGE_SIZE as u64) - 1;
/// Max memory address
const MAX_ADDR: u64 = u64::MAX;

type Page = Box<[u8; PAGE_SIZE]>;

#[derive(Copy, Clone)]
struct CacheLine {
    page_id: u64,
    ptr: NonNull<[u8; PAGE_SIZE]>,
}

pub struct PagedMemoryCacheLine<S: NamedHasher, const N: usize> {
    pages: HashMap<u64, Page, S>,
    cache: [Option<CacheLine>; N],
}

impl<S: NamedHasher, const N: usize> Default for PagedMemoryCacheLine<S, N> {
    fn default() -> Self {
        Self {
            pages: HashMap::default(),
            cache: [None; N],
        }
    }
}
