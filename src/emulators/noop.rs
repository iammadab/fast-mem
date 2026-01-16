use crate::MemoryEmulator;

#[derive(Default)]
pub struct NoopMem {}

impl MemoryEmulator for NoopMem {
    fn load_u8(&mut self, _addr: u64) -> u8 {
        1
    }

    fn load_u16(&mut self, _addr: u64) -> u16 {
        1
    }

    fn load_u32(&mut self, _addr: u64) -> u32 {
        1
    }

    fn load_u64(&mut self, _addr: u64) -> u64 {
        1
    }

    fn store_u8(&mut self, _addr: u64, _value: u8) {}

    fn store_u16(&mut self, _addr: u64, _value: u16) {}

    fn store_u32(&mut self, _addr: u64, _value: u32) {}

    fn store_u64(&mut self, _addr: u64, _value: u64) {}

    fn name(&self) -> &'static str {
        "NoopMem"
    }
}
