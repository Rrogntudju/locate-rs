use {
        std::time::Instant,
        walkdir::WalkDir,
        frcode::compress_file,
        std::env,
        winapi::um::fileapi::{GetLogicalDrives, GetDriveTypeW},
        winapi::shared::minwindef::DWORD,
        std::fs::{File, remove_file, rename},
        std::io::{BufWriter, Write},
        serde::Serialize,
};

macro_rules! unwrap {
    ($expression:expr) => (
        match $expression {
            Ok(r) => r,
            Err(e) => {
                eprintln!("{}", e); 
                return;
            }
        }
    )
}

#[derive(Default, Serialize)]
struct Statistics {
    nb_dirs: usize,
    nb_files: usize,
    uncompressed: usize,
    compressed: usize,
    elapsed: u64,
}

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

        let bit = self.dword & (1 << self.ctr) != 0;
        self.ctr = self.ctr + 1;

        Some(bit)
    }
}


fn main() {
    let start = Instant::now();

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
                        dr.push(c); 
                        dr + ":\\" 
                    })
                    .collect::<Vec<String>>();

    let ld_fix = DwordBits::new(ld_bits)
                    .zip(ld_all)
                    .filter_map(|(b, ld)| {
                        if !b {
                            return None;  // not a logical drive
                        }
                        // Convert an UTF-8 string to a null-delimited UTF-16 string
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

    // Generate a dir list from each logical drives and save it to a temp file 
    let mut stats = Statistics::default();
    let mut path = env::temp_dir();
    path.push("dirlist");
    path.set_extension("txt");
    {
        let mut writer = BufWriter::new(unwrap!(File::create(&path)));
        for ld in ld_fix {
            let walker = WalkDir::new(ld).into_iter().filter_map(|e| e.ok());
            for entry in walker {
                if let Ok(m) = entry.metadata() {
                    let p =
                        match entry.path().to_str() {
                            Some(p) => p,
                            None => continue, /* path contains non-unicode sequence */
                        };
                    stats.uncompressed = stats.uncompressed + p.len();
                    if m.is_dir() {
                        unwrap!(write!(writer, "{}\\\n", p));
                        stats.nb_dirs = stats.nb_dirs + 1;
                    }
                    else {
                        unwrap!(write!(writer, "{}\n", p));
                        stats.nb_files = stats.nb_files + 1;
                    }
                }
            }
        }
    }

    // Compress the dir list
    let mut out_file = env::temp_dir();
    out_file.push("locate");
    out_file.set_extension("tmp");
    stats.compressed = unwrap!(compress_file(&path, &out_file));

    // Cleanup
    unwrap!(remove_file(&path));
    let mut db = env::temp_dir();
    db.push("locate");
    db.set_extension("db");
    unwrap!(remove_file(&db));
    unwrap!(rename(&out_file, &db));

    // Output the statistics
    stats.elapsed = start.elapsed().as_secs();
    let mut path = env::temp_dir();
    path.push("locate");
    path.set_extension("txt");
    let j = unwrap!(serde_json::to_string(&stats));
    let mut writer = BufWriter::new(unwrap!(File::create(path)));
    unwrap!(writer.write_all(j.as_bytes()));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dwordbits_ok() {
        let mut bits = DwordBits::new(12 as DWORD);
        assert_eq!(bits.next(), Some(false)); 
        assert_eq!(bits.next(), Some(false)); 
        assert_eq!(bits.next(), Some(true));  
        assert_eq!(bits.next(), Some(true)); 
        bits.for_each(|b| assert_eq!(b, false));
    }
}