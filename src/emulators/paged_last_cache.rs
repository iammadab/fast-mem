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

#[derive(Default)]
pub struct PagedMemoryCacheLast {
    pages: HashMap<u64, Page>,
    last_page_id: Option<u64>,
    last_page_ptr: Option<NonNull<[u8; PAGE_SIZE]>>,
}

impl MemoryEmulator for PagedMemoryCacheLast {
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

    pub(crate) fn read_n_bytes_const<const N: usize>(&mut self, addr: u64) -> [u8; N] {
        let mut out = [0u8; N];
        self.read_into(addr, &mut out);
        out
    }

    fn page_ptr_mut(&mut self, page_id: u64) -> &mut [u8; PAGE_SIZE] {
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

    fn page_ptr(&mut self, page_id: u64) -> Option<&mut [u8; PAGE_SIZE]> {
        if self.last_page_id == Some(page_id) {
            if let Some(mut ptr) = self.last_page_ptr {
                return Some(unsafe { ptr.as_mut() });
            }
        }

        let page = self.pages.get_mut(&page_id)?;
        let ptr = NonNull::from(page.as_mut());

        self.last_page_id = Some(page_id);
        self.last_page_ptr = Some(ptr);
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

#[cfg(test)]
mod tests {
    use crate::emulators::paged_last_cache::{PAGE_SIZE, PagedMemoryCacheLast};

    #[test]
    fn page_reuse_result_in_same_pointer() {
        let mut mem = PagedMemoryCacheLast::default();
        let p1 = mem.page_ptr_mut(5) as *mut [u8; PAGE_SIZE];
        let p2 = mem.page_ptr_mut(5) as *mut [u8; PAGE_SIZE];

        assert_eq!(p1, p2);
    }
}
