use {
        walkdir::*,
        frcode::*,
        std::env,
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
    // Get the list of the fixed logical drives
    let ld_bits: DWORD = unsafe { GetLogicalDrives() };
    if ld_bits == 0 {
        match std::io::Error::last_os_error().raw_os_error() {
            Some(e) => eprintln!("GetLogicalDrives: {}", e),
            None    => eprintln!("GetLogicalDrives: DOH!"),
        }
        return;
    }

    let ld_all = "ABCDEFGHIJKLMNOPQRSTUVWXYZ"
                    .chars()
                    .map(|c| { 
                        let mut dr = String::new(); 
                        dr.push(c); dr + ":\\" 
                    })
                    .collect::<Vec<String>>();

    let ld_fix = DwordBits::new(ld_bits)
                    .zip(ld_all)
                    .filter_map(|(b, ld)| {
                        if !b {
                            return None;  // not a logical drive
                        }
                        // Convert an UTF-8 string to an null-delimited UTF-16 string
                        let mut ld_utf16: Vec<u16> = ld.encode_utf16().collect();
                        ld_utf16.push(0); 
                        let ld_type = unsafe { GetDriveTypeW(ld_utf16.as_ptr()) };
                        if ld_type == 3 {
                            Some(ld)
                        }
                        else {
                            None // not a fixed logical drive
                        }
                    })
                    .collect::<Vec<String>>();

    // Generate a dirlist from each logical drives and save it in a temp file 
    let mut dirlist_path = env::temp_dir();
    dirlist_path.push("dirlist");
    dirlist_path.set_extension("tmp");

}
