use std::{collections::HashMap, ptr::NonNull};

use crate::{MemoryEmulator, named_hasher::NamedHasher};

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

impl<S: NamedHasher, const N: usize> MemoryEmulator for PagedMemoryCacheLine<S, N> {
    fn name(&self) -> String {
        format!("PagedMemoryCacheLine({})", S::NAME)
    }

    fn load_u64(&mut self, addr: u64) -> u64 {
        let bytes = self.read_n_bytes_const::<8>(addr);
        u64::from_le_bytes(bytes)
    }

    fn load_u32(&mut self, addr: u64) -> u32 {
        let bytes = self.read_n_bytes_const::<4>(addr);
        u32::from_le_bytes(bytes)
    }

    fn load_u16(&mut self, addr: u64) -> u16 {
        let bytes = self.read_n_bytes_const::<2>(addr);
        u16::from_le_bytes(bytes)
    }

    fn load_u8(&mut self, addr: u64) -> u8 {
        self.read_n_bytes_const::<1>(addr)[0]
    }

    fn store_u64(&mut self, addr: u64, value: u64) {
        self.write_n_bytes(addr, &value.to_le_bytes());
    }

    fn store_u32(&mut self, addr: u64, value: u32) {
        self.write_n_bytes(addr, &value.to_le_bytes());
    }

    fn store_u16(&mut self, addr: u64, value: u16) {
        self.write_n_bytes(addr, &value.to_le_bytes());
    }

    fn store_u8(&mut self, addr: u64, value: u8) {
        self.write_n_bytes(addr, &value.to_le_bytes());
    }

    fn finish(&self) {
        #[cfg(feature = "cache_stats")]
        println!(
            "cache hit: {}\ncache miss: {}\ntotal: {}",
            self.cache_hit,
            self.cache_miss,
            self.cache_hit + self.cache_miss
        );
    }
}

impl<S: NamedHasher, const N: usize> PagedMemoryCacheLine<S, N> {
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

    pub(crate) fn read_n_bytes_const<const M: usize>(&mut self, addr: u64) -> [u8; M] {
        let mut out = [0u8; M];
        self.read_into(addr, &mut out);
        out
    }

    fn page_ptr_mut(&mut self, page_id: u64) -> &mut [u8; PAGE_SIZE] {
        let c_i = cache_index(page_id, N);
        if let Some(mut line) = self.cache[c_i] {
            if line.page_id == page_id {
                return unsafe { line.ptr.as_mut() };
            }
        }

        let entry = self
            .pages
            .entry(page_id)
            .or_insert_with(|| Box::new([0; PAGE_SIZE]));
        let ptr = NonNull::from(entry.as_mut());

        self.cache[c_i] = Some(CacheLine { page_id, ptr });
        entry
    }

    fn page_ptr(&mut self, page_id: u64) -> Option<&[u8; PAGE_SIZE]> {
        let c_i = cache_index(page_id, N);
        if let Some(line) = self.cache[c_i] {
            if line.page_id == page_id {
                return Some(unsafe { line.ptr.as_ref() });
            }
        }

        let page = self
            .pages
            .entry(page_id)
            .or_insert_with(|| Box::new([0; PAGE_SIZE]));
        let ptr = NonNull::from(page.as_ref());

        self.cache[c_i] = Some(CacheLine { page_id, ptr });
        Some(page)
    }

    fn read_into(&mut self, addr: u64, out: &mut [u8]) {
        let len = out.len();
        if len == 0 {
            return;
        }

        let _ = addr
            .checked_add(len as u64 - 1)
            .unwrap_or_else(|| panic!("read out of range: 0x{:x}", addr));

        let mut curr_addr = addr;
        let mut bytes_left = len;
        let mut dst_off = 0;

        while bytes_left > 0 {
            let idx = Self::page_idx(curr_addr);
            let offset = Self::page_offset(curr_addr);

            let chunk = bytes_left.min(PAGE_SIZE - offset);

            if let Some(page) = self.page_ptr(idx) {
                out[dst_off..dst_off + chunk].copy_from_slice(&page[offset..offset + chunk]);
            } // else leave as zeros

            curr_addr += chunk as u64;
            dst_off += chunk;
            bytes_left -= chunk;
        }
    }

    fn write_n_bytes(&mut self, addr: u64, bytes: &[u8]) {
        if bytes.is_empty() {
            return;
        }

        let _ = addr
            .checked_add(bytes.len() as u64 - 1)
            .unwrap_or_else(|| panic!("write out of range: 0x{:x}", addr));

        let mut curr_addr = addr;
        let mut bytes_left = bytes.len();
        let mut src_off = 0;

        while bytes_left > 0 {
            let idx = Self::page_idx(curr_addr);
            let offset = Self::page_offset(curr_addr);

            let chunk = bytes_left.min(PAGE_SIZE - offset);

            let page = self.page_ptr_mut(idx);
            page[offset..offset + chunk].copy_from_slice(&bytes[src_off..src_off + chunk]);

            curr_addr += chunk as u64;
            src_off += chunk;
            bytes_left -= chunk;
        }
    }
}

/// Map a page_id to a cache index
/// this is a control lever as the distribution of page_id's
/// can affect the cache_hit count
fn cache_index(page_id: u64, capacity: usize) -> usize {
    (page_id as usize) & (capacity - 1)
}
