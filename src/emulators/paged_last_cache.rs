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
    pages: HashMap<u64, Page>,
    last_page_id: Option<u64>,
    last_page_ptr: Option<NonNull<[u8; PAGE_SIZE]>>,
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

impl PagedMemoryCacheLast {
    /// Return the page index given the address
    #[inline]
    pub fn page_idx(addr: u64) -> u64 {
        // addr = [PAGE_ID][PAGE_SHIFT]
        addr >> PAGE_SHIFT
    }

    /// Return the entry index within a page
    /// given an address
    #[inline]
    pub fn page_offset(addr: u64) -> usize {
        (addr & PAGE_MASK) as usize
    }

    fn page_ptr_mut(&mut self, addr: u64) -> &mut [u8; PAGE_SIZE] {
        let page_id = Self::page_idx(addr);

        if self.last_page_id == Some(page_id) {
            if let Some(mut ptr) = self.last_page_ptr {
                return unsafe { ptr.as_mut() };
            }
        }

        let entry = self
            .pages
            .entry(page_id)
            .or_insert_with(|| Box::new([0; PAGE_SIZE]));
        let ptr = NonNull::from(entry.as_mut());

        self.last_page_id = Some(page_id);
        self.last_page_ptr = Some(ptr);
        entry
    }
}
