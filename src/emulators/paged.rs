use std::{
    collections::HashMap,
    hash::{BuildHasher, RandomState},
};

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

type SipHash = RandomState;
type AHash = ahash::RandomState;
type FxHash = fxhash::FxBuildHasher;
type NoHashU64 = nohash_hasher::BuildNoHashHasher<u64>;

pub type PagedMemoryDefault = PagedMemory<SipHash>;
pub type PagedMemoryAHash = PagedMemory<AHash>;
pub type PagedMemoryFxHash = PagedMemory<FxHash>;
pub type PagedMemoryNoHashU64 = PagedMemory<NoHashU64>;

#[derive(Default)]
pub struct PagedMemory<S: BuildHasher> {
    pages: HashMap<u64, Page, S>,
}

impl<S: BuildHasher> MemoryEmulator for PagedMemory<S> {
    fn load_u64(&self, addr: u64) -> u64 {
        let bytes = self.read_n_bytes_const::<8>(addr);
        u64::from_le_bytes(bytes)
    }

    fn load_u32(&self, addr: u64) -> u32 {
        let bytes = self.read_n_bytes_const::<4>(addr);
        u32::from_le_bytes(bytes)
    }

    fn load_u16(&self, addr: u64) -> u16 {
        let bytes = self.read_n_bytes_const::<2>(addr);
        u16::from_le_bytes(bytes)
    }

    fn load_u8(&self, addr: u64) -> u8 {
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
}

impl<S: BuildHasher> PagedMemory<S> {
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

    pub(crate) fn read_n_bytes_const<const N: usize>(&self, addr: u64) -> [u8; N] {
        let mut out = [0u8; N];
        self.read_into(addr, &mut out);
        out
    }

    /// Read n contiguous bytes from memory
    /// assumes that out is zeroed out
    fn read_into(&self, addr: u64, out: &mut [u8]) {
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

        let mut curr_addr = addr;
        let mut bytes_left = len;
        let mut dst_off = 0;

        while bytes_left > 0 {
            let idx = Self::page_idx(curr_addr);
            let offset = Self::page_offset(curr_addr);

            let chunk = bytes_left.min(PAGE_SIZE - offset);

            if let Some(page) = self.pages.get(&idx) {
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

        let mut curr_addr = addr;
        let mut bytes_left = bytes.len();
        let mut src_off = 0;

        while bytes_left > 0 {
            let idx = Self::page_idx(curr_addr);
            let offset = Self::page_offset(curr_addr);

            let chunk = bytes_left.min(PAGE_SIZE - offset);

            let page = self.ensure_page(idx);
            page[offset..(offset + chunk)].copy_from_slice(&bytes[src_off..(src_off + chunk)]);

            curr_addr += chunk as u64;
            src_off += chunk;
            bytes_left -= chunk;
        }
    }
}
