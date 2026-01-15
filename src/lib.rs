mod paged;

trait MemoryEmulator {
    fn load_u8(&self, addr: u64) -> u8;
    fn load_u16(&self, addr: u64) -> u16;
    fn load_u32(&self, addr: u64) -> u32;
    fn load_u64(&self, addr: u64) -> u64;

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

#[cfg(test)]
mod tests {
    use crate::test_memory_emulator;

    #[test]
    fn test_mem_emulator_correctness() {
        // test_memory_emulator();
    }
}
