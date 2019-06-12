use {
        walkdir::*,
        frcode::*,
        std::env,
        core::ffi::c_void,
        winapi::um::fileapi::{GetLogicalDrives, GetDriveTypeW},
        winapi::shared::minwindef::DWORD,
};

struct DwordBits {
    dword: DWORD,
    ctr: u8,
}

impl DwordBits {
    fn new (dword: DWORD) -> DwordBits {
        DwordBits {
            dword: dword,
            ctr: 0,
        }
    }
}

impl Iterator for DwordBits {
    type Item = bool;

    fn next(&mut self) -> Option<Self::Item> {
        if self.ctr == 32 {
            return None;
        }

        let bit = (self.dword << self.ctr).rotate_left(1);
        self.ctr = self.ctr + 1;

        return Some(
            match bit {
                0 => false,
                1 | _ => true,
            }
        )
    }
}

fn main() {
    // Get the list of the logical fixed drives
    let drives: DWORD = unsafe { GetLogicalDrives() };
    if drives == 0 {
        match std::io::Error::last_os_error().raw_os_error() {
            Some(e) => eprintln!("GetLogicalDrives: {}", e),
            None    => eprintln!("GetLogicalDrives: DOH!"),
        }
    }
    else {
        for bit in DwordBits::new(drives) {

        }
    }
}
