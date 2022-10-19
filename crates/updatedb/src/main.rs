use {
    frcode::compress_file,
    serde_json::json,
    std::env,
    std::error::Error,
    std::fs::{remove_file, rename, File},
    std::io::{BufWriter, Write},
    std::time::Instant,
    walkdir::WalkDir,
    windows::core::PCWSTR,
    windows::Win32::{Storage::FileSystem::GetDriveTypeW, Storage::FileSystem::GetLogicalDrives},
};

#[derive(Default)]
struct Statistics {
    dirs: usize,
    files: usize,
    files_bytes: usize,
    db_size: usize,
    elapsed: u64,
}

struct DwordBits {
    dword: u32,
    ctr: u8,
}

impl DwordBits {
    fn new(dword: u32) -> DwordBits {
        DwordBits { dword, ctr: 0 }
    }
}

impl Iterator for DwordBits {
    type Item = bool;

    fn next(&mut self) -> Option<Self::Item> {
        if self.ctr == 32 {
            return None;
        }

        let bit = self.dword & (1 << self.ctr) != 0;
        self.ctr += 1;

        Some(bit)
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let start = Instant::now();

    // Get the list of the fixed logical drives
    let ld_bits: u32 = unsafe { GetLogicalDrives() };
    if ld_bits == 0 {
        return Err(match std::io::Error::last_os_error().raw_os_error() {
            Some(e) => format!("GetLogicalDrives: {}", e).into(),
            None => "GetLogicalDrives: DOH!".into(),
        });
    }

    let ld_fix = DwordBits::new(ld_bits)
        .zip("ABCDEFGHIJKLMNOPQRSTUVWXYZ".chars())
        .filter_map(|(b, c)| {
            if b {
                let mut ld = String::with_capacity(3);
                ld.push(c);
                ld.push_str(":\\");
                // Convert an UTF-8 string to a null-delimited UTF-16 string
                let mut ld_utf16: Vec<u16> = ld.encode_utf16().collect();
                ld_utf16.push(0);
                let ld_type = unsafe { GetDriveTypeW(PCWSTR::from_raw(ld_utf16.as_mut_ptr())) };
                if ld_type == 3 {
                    Some(ld)
                } else {
                    None // not a fixed logical drive
                }
            } else {
                None // not a logical drive
            }
        })
        .collect::<Vec<String>>();

    // Generate a dir list from each logical drives and save it to a temp file
    let mut stats = Statistics::default();
    let mut dirlist = env::temp_dir();
    dirlist.set_file_name("dirlist.txt");

    let mut writer = BufWriter::new(File::create(&dirlist)?);
    for ld in ld_fix {
        let walker = WalkDir::new(ld).into_iter().filter_map(Result::ok);
        for entry in walker {
            if let Ok(m) = entry.metadata() {
                let p = entry.path().to_string_lossy(); // path may contain non-unicode sequence
                if m.is_dir() {
                    write!(writer, "{}\\\n", p)?;
                    stats.dirs += 1;
                } else {
                    write!(writer, "{}\n", p)?;
                    stats.files += 1;
                    stats.files_bytes += p.len();
                }
            }
        }
    }
    writer.flush()?;

    // Compress the dir list
    let mut db1 = env::temp_dir();
    db1.set_file_name("locate.db1");
    stats.db_size = compress_file(&dirlist, &db1)?;

    // Cleanup
    remove_file(&dirlist)?;
    let mut db = env::temp_dir();
    db.set_file_name("locate.db");
    if db.is_file() {
        remove_file(&db)?;
    }
    rename(&db1, &db)?;

    // Output the statistics
    stats.elapsed = start.elapsed().as_secs();
    let stats = json!({
        "dirs": stats.dirs,
        "files": stats.files,
        "files_bytes": stats.files_bytes,
        "db_size": stats.db_size,
        "elapsed": stats.elapsed,
    });
    let j = serde_json::to_string(&stats)?;
    let mut path = env::temp_dir();
    path.set_file_name("locate.txt");
    let mut writer = BufWriter::new(File::create(path)?);
    writer.write_all(j.as_bytes())?;
    writer.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dwordbits_ok() {
        let mut bits = DwordBits::new(12 as u32);
        assert_eq!(bits.next(), Some(false));
        assert_eq!(bits.next(), Some(false));
        assert_eq!(bits.next(), Some(true));
        assert_eq!(bits.next(), Some(true));
        bits.for_each(|b| assert_eq!(b, false));
    }
}
