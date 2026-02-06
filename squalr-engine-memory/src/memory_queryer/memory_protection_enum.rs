bitflags::bitflags! {
    #[derive(Debug, Copy, Clone)]
    pub struct MemoryProtectionEnum: u32 {
        const NONE = 0x0;
        const READ = 0x1;
        const WRITE = 0x2;
        const EXECUTE = 0x4;
        const COPY_ON_WRITE = 0x8;
        const NO_CACHE = 0x10;
        const WRITE_COMBINE = 0x20;
    }
}
