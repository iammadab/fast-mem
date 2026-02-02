use crate::MemoryEmulator;

#[derive(Default)]
pub struct NoopMem {}

impl MemoryEmulator for NoopMem {
    #[inline]
    fn load_u8(&mut self, _addr: u64) -> u8 {
        1
    }

    #[inline]
    fn load_u16(&mut self, _addr: u64) -> u16 {
        1
    }

    #[inline]
    fn load_u32(&mut self, _addr: u64) -> u32 {
        1
    }

    #[inline]
    fn load_u64(&mut self, _addr: u64) -> u64 {
        1
    }

    #[inline]
    fn store_u8(&mut self, _addr: u64, _value: u8) {}

    #[inline]
    fn store_u16(&mut self, _addr: u64, _value: u16) {}

    #[inline]
    fn store_u32(&mut self, _addr: u64, _value: u32) {}

    #[inline]
    fn store_u64(&mut self, _addr: u64, _value: u64) {}

    fn name(&self) -> String {
        "NoopMem".to_string()
    }

    fn finish(&self) {}
}
