use std::fs::File;

use memmap2::Mmap;

use crate::emulators::paged::{PagedMemory, PagedMemoryFxHash};

pub mod emulators;

pub trait MemoryEmulator {
    fn load_u8(&mut self, addr: u64) -> u8;
    fn load_u16(&mut self, addr: u64) -> u16;
    fn load_u32(&mut self, addr: u64) -> u32;
    fn load_u64(&mut self, addr: u64) -> u64;

    fn store_u8(&mut self, addr: u64, value: u8);
    fn store_u16(&mut self, addr: u64, value: u16);
    fn store_u32(&mut self, addr: u64, value: u32);
    fn store_u64(&mut self, addr: u64, value: u64);
}

fn test_memory_emulator<M: MemoryEmulator>(mut mem: M) {
    let addrs: &[u64] = &[
        0,
        1,
        2,
        3,
        7,
        8,
        15,
        16,
        0x1000,
        0x1003,
        0x1FFF,
        0xFFFF_FFFF_FFFF_FF00,
    ];

    for a in addrs {
        let addr = *a;
        mem.store_u8(addr, 0xA5);
        assert_eq!(mem.load_u8(addr), 0xA5);

        mem.store_u16(addr, 0xBEEF);
        assert_eq!(mem.load_u16(addr), 0xBEEF);

        mem.store_u32(addr, 0xDEAB_BEED);
        assert_eq!(mem.load_u32(addr), 0xDEAB_BEED);

        mem.store_u64(addr, 0x0123_4567_89AB_CDEF);
        assert_eq!(mem.load_u64(addr), 0x0123_4567_89AB_CDEF);
    }

    let base = 0x3000;
    mem.store_u32(base, 0xAABB_CCDD);
    mem.store_u16(base + 1, 0x1122);
    assert_eq!(mem.load_u32(base), 0xAA11_22DD);

    let base = 0x5000;
    mem.store_u8(base, 0x78);
    mem.store_u8(base + 1, 0x56);
    mem.store_u8(base + 2, 0x34);
    mem.store_u8(base + 3, 0x12);
    assert_eq!(mem.load_u32(base), 0x1234_5678);
}

struct ReplayIter<'a> {
    data: &'a [u8],
}

impl<'a> ReplayIter<'a> {
    #[inline]
    fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        if self.data.len() < n {
            return None;
        }

        let (head, tail) = self.data.split_at(n);
        self.data = tail;
        Some(head)
    }
}

pub fn replay_mem_operations<M: MemoryEmulator>(file_path: &'static str, mem_emulator: &mut M) {
    let file = File::open(file_path).unwrap();
    let mmap = unsafe { Mmap::map(&file).expect("mmap failed") };

    unsafe {
        libc::madvise(
            mmap.as_ptr() as *mut libc::c_void,
            mmap.len(),
            libc::MADV_SEQUENTIAL | libc::MADV_WILLNEED,
        );
    }

    let mut data = ReplayIter { data: &mmap };

    let mut count: u64 = 0;
    let mut last_id: Option<u64> = None;

    loop {
        let header = match data.take(10) {
            Some(h) => h,
            None => break,
        };
        let width = header[1] as usize;
        let addr = u64::from_le_bytes(header[2..10].try_into().unwrap());

        if last_id != Some(PagedMemoryFxHash::page_idx(addr)) {
            count = count.checked_add(1).unwrap();
            last_id = Some(PagedMemoryFxHash::page_idx(addr));
        }

        match header[0] {
            1 => {
                let value = data.take(width).unwrap();

                match width {
                    1 => mem_emulator.store_u8(addr, value[0]),
                    2 => mem_emulator
                        .store_u16(addr, u16::from_le_bytes(value[..width].try_into().unwrap())),
                    4 => mem_emulator
                        .store_u32(addr, u32::from_le_bytes(value[..width].try_into().unwrap())),
                    8 => mem_emulator
                        .store_u64(addr, u64::from_le_bytes(value[..width].try_into().unwrap())),
                    _ => unreachable!(),
                }
            }
            2 => match width {
                1 => {
                    let _ = mem_emulator.load_u8(addr);
                }
                2 => {
                    let _ = mem_emulator.load_u16(addr);
                }
                4 => {
                    let _ = mem_emulator.load_u32(addr);
                }
                8 => {
                    let _ = mem_emulator.load_u64(addr);
                }
                _ => unreachable!(),
            },
            _ => panic!("unknown operation"),
        }
    }

    println!("total ops: {}", count);
}

#[cfg(test)]
mod tests {
    use crate::{
        emulators::{
            paged::{
                PagedMemoryAHash, PagedMemoryDefault, PagedMemoryFxHash, PagedMemoryNoHashU64,
            },
            paged_last_cache::{
                PagedMemoryCacheLast, PagedMemoryCacheLastAHash, PagedMemoryCacheLastDefault,
                PagedMemoryCacheLastFxHash, PagedMemoryCacheLastNoHashU64,
            },
        },
        test_memory_emulator,
    };

    #[test]
    fn test_mem_emulator_correctness() {
        test_memory_emulator(PagedMemoryDefault::default());
        test_memory_emulator(PagedMemoryAHash::default());
        test_memory_emulator(PagedMemoryFxHash::default());
        test_memory_emulator(PagedMemoryNoHashU64::default());
        test_memory_emulator(PagedMemoryCacheLastDefault::default());
        test_memory_emulator(PagedMemoryCacheLastAHash::default());
        test_memory_emulator(PagedMemoryCacheLastFxHash::default());
        test_memory_emulator(PagedMemoryCacheLastNoHashU64::default());
    }
}
