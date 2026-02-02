use std::collections::HashMap;
use std::ptr::NonNull;

use crate::{
    named_hasher::{AHash, FxHash, NamedHasher, NoHashU64, Sip},
    MemoryEmulator,
};

/// Number of bits to describe entries in a page
const PAGE_SHIFT: u64 = 12;
/// Total number of entries in a page
const PAGE_SIZE: usize = 1 << PAGE_SHIFT;
/// Mask to get the last `PAGE_SHIFT` bits of an address
const PAGE_MASK: u64 = (PAGE_SIZE as u64) - 1;
/// Max memory address
const MAX_ADDR: u64 = u64::MAX;

type Page = Box<[u8; PAGE_SIZE]>;

pub type PagedMemoryCacheDefault<const N: usize> = PagedMemoryCache<N, Sip>;
pub type PagedMemoryCacheAHash<const N: usize> = PagedMemoryCache<N, AHash>;
pub type PagedMemoryCacheFxHash<const N: usize> = PagedMemoryCache<N, FxHash>;
pub type PagedMemoryCacheNoHashU64<const N: usize> = PagedMemoryCache<N, NoHashU64>;

pub type PagedMemoryCache4FxHash = PagedMemoryCache<4, FxHash>;
pub type PagedMemoryCache8FxHash = PagedMemoryCache<8, FxHash>;
pub type PagedMemoryCache16FxHash = PagedMemoryCache<16, FxHash>;
pub type PagedMemoryCache32FxHash = PagedMemoryCache<32, FxHash>;

pub struct PagedMemoryCache<const N: usize, S: NamedHasher> {
    pages: HashMap<u64, Page, S>,
    #[allow(dead_code)]
    cache_ids: [u64; N],
    #[allow(dead_code)]
    cache_ptrs: [Option<NonNull<[u8; PAGE_SIZE]>>; N],
}

impl<const N: usize, S: NamedHasher + Default> Default for PagedMemoryCache<N, S> {
    fn default() -> Self {
        Self {
            pages: HashMap::default(),
            cache_ids: [u64::MAX; N],
            cache_ptrs: [None; N],
        }
    }
}

impl<const N: usize, S: NamedHasher> MemoryEmulator for PagedMemoryCache<N, S> {
    fn name(&self) -> String {
        format!("PagedMemCache{N}({})", S::NAME)
    }

    fn load_u64(&mut self, addr: u64) -> u64 {
        let end = addr
            .checked_add(7)
            .unwrap_or_else(|| panic!("read out of range: 0x{:x}", addr));
        if end > MAX_ADDR {
            panic!("write out of range: 0x{:x}", addr);
        }

        let start_page = Self::page_idx(addr);
        let end_page = Self::page_idx(end);
        if start_page == end_page {
            let offset = Self::page_offset(addr);
            if let Some(page) = self.cache_get(start_page) {
                let mut bytes = [0u8; 8];
                bytes.copy_from_slice(&page[offset..offset + 8]);
                return u64::from_le_bytes(bytes);
            }
            return 0;
        }

        let bytes = self.read_n_bytes_const::<8>(addr);
        u64::from_le_bytes(bytes)
    }

    fn load_u32(&mut self, addr: u64) -> u32 {
        let end = addr
            .checked_add(3)
            .unwrap_or_else(|| panic!("read out of range: 0x{:x}", addr));
        if end > MAX_ADDR {
            panic!("write out of range: 0x{:x}", addr);
        }

        let start_page = Self::page_idx(addr);
        let end_page = Self::page_idx(end);
        if start_page == end_page {
            let offset = Self::page_offset(addr);
            if let Some(page) = self.cache_get(start_page) {
                let mut bytes = [0u8; 4];
                bytes.copy_from_slice(&page[offset..offset + 4]);
                return u32::from_le_bytes(bytes);
            }
            return 0;
        }

        let bytes = self.read_n_bytes_const::<4>(addr);
        u32::from_le_bytes(bytes)
    }

    fn load_u16(&mut self, addr: u64) -> u16 {
        let end = addr
            .checked_add(1)
            .unwrap_or_else(|| panic!("read out of range: 0x{:x}", addr));
        if end > MAX_ADDR {
            panic!("write out of range: 0x{:x}", addr);
        }

        let start_page = Self::page_idx(addr);
        let end_page = Self::page_idx(end);
        if start_page == end_page {
            let offset = Self::page_offset(addr);
            if let Some(page) = self.cache_get(start_page) {
                let mut bytes = [0u8; 2];
                bytes.copy_from_slice(&page[offset..offset + 2]);
                return u16::from_le_bytes(bytes);
            }
            return 0;
        }

        let bytes = self.read_n_bytes_const::<2>(addr);
        u16::from_le_bytes(bytes)
    }

    fn load_u8(&mut self, addr: u64) -> u8 {
        let end = addr;
        if end > MAX_ADDR {
            panic!("write out of range: 0x{:x}", addr);
        }

        let start_page = Self::page_idx(addr);
        if let Some(page) = self.cache_get(start_page) {
            let offset = Self::page_offset(addr);
            return page[offset];
        }

        self.read_n_bytes_const::<1>(addr)[0]
    }

    fn store_u64(&mut self, addr: u64, value: u64) {
        let end = addr
            .checked_add(7)
            .unwrap_or_else(|| panic!("write out of range: 0x{:x}", addr));
        if addr > MAX_ADDR || end > MAX_ADDR {
            panic!("write out of range: 0x{:x}", addr);
        }

        let start_page = Self::page_idx(addr);
        let end_page = Self::page_idx(end);
        if start_page == end_page {
            let offset = Self::page_offset(addr);
            let page = self.cache_get_mut(start_page);
            page[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
            return;
        }

        self.write_n_bytes(addr, &value.to_le_bytes());
    }

    fn store_u32(&mut self, addr: u64, value: u32) {
        let end = addr
            .checked_add(3)
            .unwrap_or_else(|| panic!("write out of range: 0x{:x}", addr));
        if addr > MAX_ADDR || end > MAX_ADDR {
            panic!("write out of range: 0x{:x}", addr);
        }

        let start_page = Self::page_idx(addr);
        let end_page = Self::page_idx(end);
        if start_page == end_page {
            let offset = Self::page_offset(addr);
            let page = self.cache_get_mut(start_page);
            page[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
            return;
        }

        self.write_n_bytes(addr, &value.to_le_bytes());
    }

    fn store_u16(&mut self, addr: u64, value: u16) {
        let end = addr
            .checked_add(1)
            .unwrap_or_else(|| panic!("write out of range: 0x{:x}", addr));
        if addr > MAX_ADDR || end > MAX_ADDR {
            panic!("write out of range: 0x{:x}", addr);
        }

        let start_page = Self::page_idx(addr);
        let end_page = Self::page_idx(end);
        if start_page == end_page {
            let offset = Self::page_offset(addr);
            let page = self.cache_get_mut(start_page);
            page[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
            return;
        }

        self.write_n_bytes(addr, &value.to_le_bytes());
    }

    fn store_u8(&mut self, addr: u64, value: u8) {
        let end = addr;
        if addr > MAX_ADDR || end > MAX_ADDR {
            panic!("write out of range: 0x{:x}", addr);
        }

        let start_page = Self::page_idx(addr);
        let page = self.cache_get_mut(start_page);
        let offset = Self::page_offset(addr);
        page[offset] = value;
    }

    fn finish(&self) {}
}

impl<const N: usize, S: NamedHasher> PagedMemoryCache<N, S> {
    #[inline]
    fn cache_get(&mut self, page_id: u64) -> Option<&[u8; PAGE_SIZE]> {
        if N == 0 {
            return self.pages.get(&page_id).map(|page| page.as_ref());
        }

        debug_assert!(N.is_power_of_two());
        let idx = (page_id as usize) & (N - 1);

        if self.cache_ids[idx] == page_id {
            if let Some(ptr) = self.cache_ptrs[idx] {
                return Some(unsafe { ptr.as_ref() });
            }
        }

        let page = self.pages.get(&page_id)?;
        self.cache_ids[idx] = page_id;
        self.cache_ptrs[idx] = Some(NonNull::from(page.as_ref()));
        Some(page)
    }

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

    /// Returns a mutable reference to a page given an address
    /// lazy allocates the page if needed
    #[inline]
    fn ensure_page(&mut self, idx: u64) -> &mut Page {
        self.pages
            .entry(idx)
            .or_insert_with(|| Box::new([0; PAGE_SIZE]))
    }

    pub(crate) fn read_n_bytes_const<const M: usize>(&mut self, addr: u64) -> [u8; M] {
        let mut out = [0u8; M];
        self.read_into(addr, &mut out);
        out
    }

    /// Read n contiguous bytes from memory
    /// assumes that out is zeroed out
    fn read_into(&mut self, addr: u64, out: &mut [u8]) {
        let len = out.len();
        if len == 0 {
            return;
        }

        let end = addr
            .checked_add(len as u64 - 1)
            .unwrap_or_else(|| panic!("read out of range: 0x{:x}", addr));

        if end > MAX_ADDR {
            panic!("write out of range: 0x{:x}", addr);
        }

        let start_page = Self::page_idx(addr);
        let end_page = Self::page_idx(end);
        if start_page == end_page {
            let offset = Self::page_offset(addr);
            if let Some(page) = self.cache_get(start_page) {
                out.copy_from_slice(&page[offset..offset + len]);
            }
            return;
        }

        let mut curr_addr = addr;
        let mut bytes_left = len;
        let mut dst_off = 0;

        while bytes_left > 0 {
            let idx = Self::page_idx(curr_addr);
            let offset = Self::page_offset(curr_addr);

            let chunk = bytes_left.min(PAGE_SIZE - offset);

            if let Some(page) = self.cache_get(idx) {
                out[dst_off..dst_off + chunk].copy_from_slice(&page[offset..offset + chunk]);
            } // else leave as zeros

            curr_addr += chunk as u64;
            dst_off += chunk;
            bytes_left -= chunk
        }
    }

    /// Write n contiguous bytes into memory
    /// Handles cross page writing
    pub(crate) fn write_n_bytes(&mut self, addr: u64, bytes: &[u8]) {
        if bytes.is_empty() {
            return;
        }

        let end = addr
            .checked_add(bytes.len() as u64 - 1)
            .unwrap_or_else(|| panic!("write out of range: 0x{:x}", addr));

        if addr > MAX_ADDR || end > MAX_ADDR {
            panic!("write out of range: 0x{:x}", addr);
        }

        let start_page = Self::page_idx(addr);
        let end_page = Self::page_idx(end);
        if start_page == end_page {
            let offset = Self::page_offset(addr);
            let page = self.cache_get_mut(start_page);
            page[offset..offset + bytes.len()].copy_from_slice(bytes);
            return;
        }

        let mut curr_addr = addr;
        let mut bytes_left = bytes.len();
        let mut src_off = 0;

        while bytes_left > 0 {
            let idx = Self::page_idx(curr_addr);
            let offset = Self::page_offset(curr_addr);

            let chunk = bytes_left.min(PAGE_SIZE - offset);

            let page = self.cache_get_mut(idx);
            page[offset..(offset + chunk)].copy_from_slice(&bytes[src_off..(src_off + chunk)]);

            curr_addr += chunk as u64;
            src_off += chunk;
            bytes_left -= chunk;
        }
    }

    #[inline]
    fn cache_get_mut(&mut self, page_id: u64) -> &mut [u8; PAGE_SIZE] {
        if N == 0 {
            return self.ensure_page(page_id);
        }

        debug_assert!(N.is_power_of_two());
        let idx = (page_id as usize) & (N - 1);

        if self.cache_ids[idx] == page_id {
            if let Some(mut ptr) = self.cache_ptrs[idx] {
                return unsafe { ptr.as_mut() };
            }
        }

        let entry = self
            .pages
            .entry(page_id)
            .or_insert_with(|| Box::new([0; PAGE_SIZE]));
        let ptr = NonNull::from(entry.as_mut());
        self.cache_ids[idx] = page_id;
        self.cache_ptrs[idx] = Some(ptr);
        entry
    }
}
