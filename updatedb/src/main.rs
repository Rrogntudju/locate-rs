use {
        walkdir::*,
        frcode::*,
        std::env,
        core::ffi::c_void,
        winapi::um::fileapi::{GetLogicalDrives, GetDriveTypeW},
        winapi::shared::minwindef::DWORD,
}

fn main() {
    // Get the list of the logical fixed drives
    let drives: DWORD = unsafe { GetLogicalDrives() };
}
