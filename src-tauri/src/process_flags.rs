pub const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
pub const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub const fn windows_creation_flags() -> u32 {
    CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW
}
