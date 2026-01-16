use std::ptr::NonNull;

use ahash::HashMap;

use crate::MemoryEmulator;

/// Number of bits to describe entries in a page
const PAGE_SHIFT: u64 = 12;
/// Total number of entries in a page
const PAGE_SIZE: usize = 1 << PAGE_SHIFT;
/// Mask to get the last `PAGE_SHIFT` bits of an address
const PAGE_MASK: u64 = (PAGE_SIZE as u64) - 1;
/// Max memory address
const MAX_ADDR: u64 = u64::MAX;

type Page = Box<[u8; PAGE_SIZE]>;

pub struct PagedMemoryCacheLast {
    paged: HashMap<u64, Page>,
    last_page_id: Option<u64>,
    last_page_ptr: Option<NonNull<Page>>,
}

impl MemoryEmulator for PagedMemoryCacheLast {
    fn load_u8(&self, addr: u64) -> u8 {
        todo!()
    }

    fn load_u16(&self, addr: u64) -> u16 {
        todo!()
    }

    fn load_u32(&self, addr: u64) -> u32 {
        todo!()
    }

    fn load_u64(&self, addr: u64) -> u64 {
        todo!()
    }

    fn store_u8(&mut self, addr: u64, value: u8) {
        todo!()
    }

    fn store_u16(&mut self, addr: u64, value: u16) {
        todo!()
    }

    fn store_u32(&mut self, addr: u64, value: u32) {
        todo!()
    }

    fn store_u64(&mut self, addr: u64, value: u64) {
        todo!()
    }
}
