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

fn test_memory_emulator<M: MemoryEmulator>(mem: M) {
    todo!()
}
