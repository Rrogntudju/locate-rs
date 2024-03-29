use std::{
    error::Error,
    fs::File,
    io,
    io::{
        prelude::{BufRead, Write},
        BufReader, BufWriter,
    },
    path::Path,
    string::FromUtf8Error,
};

pub struct FrCompress<'a> {
    init: bool,
    prec_prefix_len: i16,
    prec: String,
    lines: Box<dyn Iterator<Item = io::Result<String>> + 'a>,
}

impl<'a> FrCompress<'a> {
    pub fn new(reader: impl BufRead + 'a) -> FrCompress<'a> {
        FrCompress {
            init: false,
            prec_prefix_len: 0,
            prec: String::new(),
            lines: Box::new(reader.lines()),
        }
    }
}

impl<'a> Iterator for FrCompress<'a> {
    type Item = io::Result<Vec<u8>>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.lines.next()? {
            Ok(line) => {
                // https://www.gnu.org/software/findutils/manual/html_node/find_html/LOCATE02-Database-Format.html
                let mut out_bytes: Vec<u8> = vec![];
                if !self.init {
                    let label = b"LOCATEW";
                    out_bytes.push(0); // offset-differential count
                    out_bytes.push(label.len() as u8);
                    out_bytes.extend_from_slice(label);
                    self.init = true;
                }

                // Find the common prefix (case sensitive) between the current and the previous line
                let mut prefix_len: usize = 0;
                for (ch_line, ch_prec) in line.chars().zip(self.prec.chars()) {
                    if ch_line == ch_prec {
                        prefix_len += ch_line.len_utf8();
                    } else {
                        break;
                    }
                }

                // Output the offset-differential count
                let offset: i16 = prefix_len as i16 - self.prec_prefix_len;
                if offset > -128 && offset < 128 {
                    // 1 byte offset
                    out_bytes.extend_from_slice(&(offset as i8).to_be_bytes());
                } else {
                    out_bytes.push(0x80);
                    // 2 bytes offset big-endian
                    out_bytes.extend_from_slice(&offset.to_be_bytes());
                }

                // Output the line without the prefix
                let suffix_len: usize = line.len() - prefix_len;
                if suffix_len < 128 {
                    // 1 byte length
                    out_bytes.extend_from_slice(&(suffix_len as i8).to_be_bytes());
                } else {
                    out_bytes.push(0x80);
                    assert!(suffix_len < 32768);
                    // 2 bytes length big-endian
                    out_bytes.extend_from_slice(&(suffix_len as i16).to_be_bytes());
                }
                out_bytes.extend_from_slice(line[prefix_len..].as_bytes());
                self.prec_prefix_len = prefix_len as i16;
                self.prec = line;

                Some(Ok(out_bytes))
            }

            Err(err) => Some(Err(err)),
        }
    }
}

pub struct FrDecompress<'a> {
    init: bool,
    prec_prefix_len: i16,
    prec: String,
    bytes: Box<dyn Iterator<Item = io::Result<u8>> + 'a>,
}

impl<'a> FrDecompress<'a> {
    pub fn new(reader: impl BufRead + 'a) -> FrDecompress<'a> {
        FrDecompress {
            init: false,
            prec_prefix_len: 0,
            prec: String::with_capacity(1_000),
            bytes: Box::new(reader.bytes()),
        }
    }

    fn count_from_bytes(&mut self) -> Option<i16> {
        let bytes_mut = &mut self.bytes;
        let count_1b = bytes_mut.take(1).filter_map(Result::ok).collect::<Vec<u8>>();
        if count_1b.len() != 1 {
            None
        } else if count_1b[0] != 0x80 {
            Some(i8::from_be_bytes([count_1b[0]]) as i16)
        } else {
            let count_2b = bytes_mut.take(2).filter_map(Result::ok).collect::<Vec<u8>>();
            assert_eq!(count_2b.len(), 2);
            Some(i16::from_be_bytes([count_2b[0], count_2b[1]]))
        }
    }

    fn suffix_from_bytes(&mut self, len: usize) -> Result<String, FromUtf8Error> {
        let bytes_mut = &mut self.bytes;
        let suffix = bytes_mut.take(len).filter_map(Result::ok).collect::<Vec<u8>>();
        assert_eq!(suffix.len(), len);
        String::from_utf8(suffix)
    }
}

impl<'a> Iterator for FrDecompress<'a> {
    type Item = Result<String, Box<dyn Error>>;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.init {
            let _ = self.bytes.next()?; // Skip the offset
            let len = self.count_from_bytes()?;
            let label = match self.suffix_from_bytes(len as usize) {
                Ok(label) => label,
                Err(err) => return Some(Err(err.into())),
            };

            if label == "LOCATEW" {
                self.init = true;
            } else {
                return Some(Err("Fichier updateDB invalide".into()));
            }
        }

        let offset = self.count_from_bytes()?; // end of valid updateDB file happens here
        let suffix_len = self.count_from_bytes()?;
        let suffix = match self.suffix_from_bytes(suffix_len as usize) {
            Ok(suffix) => suffix,
            Err(err) => return Some(Err(err.into())),
        };

        let prefix_len = self.prec_prefix_len + offset;
        let mut line = String::with_capacity((prefix_len + suffix_len) as usize);
        line.push_str(&self.prec[..prefix_len as usize]);
        line.push_str(&suffix);

        self.prec_prefix_len = prefix_len;
        self.prec.clear();
        self.prec.push_str(&line);

        Some(Ok(line))
    }
}

pub fn compress_file(in_file: &Path, out_file: &Path) -> Result<usize, Box<dyn Error>> {
    let reader = BufReader::new(File::open(in_file)?);
    let compressed_lines = FrCompress::new(reader);

    let mut writer = BufWriter::new(File::create(out_file)?);

    let mut ctr_bytes: usize = 0;
    for line in compressed_lines {
        let line = line?;
        writer.write_all(&line)?;
        ctr_bytes += line.len();
    }

    Ok(ctr_bytes)
}

pub fn decompress_file(in_file: &Path, out_file: &Path) -> Result<usize, Box<dyn Error>> {
    let reader = BufReader::new(File::open(in_file)?);
    let decompressed_lines = FrDecompress::new(reader);

    let mut writer = BufWriter::new(File::create(out_file)?);

    let mut ctr_bytes: usize = 0;
    for line in decompressed_lines {
        let line = line?;
        writer.write_all(line.as_bytes())?;
        ctr_bytes += line.len();
    }

    Ok(ctr_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use io::Cursor;

    #[test]
    fn compress_decompress_ok() {
        let dirlist = vec![
            "C:\\Users",
            "C:\\Users\\Fourmilier",
            "C:\\Users\\Fourmilier\\Documents\\Bébé Aardvark.jpg",
            "C:\\Users\\Fourmilier\\Documents\\Bébé Armadillo.jpg",
            "C:\\Windows",
            "D:\\ماريو.txt",
            concat!(
                "E:\\aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "\\bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                "\\cccccccccccccccccccccccccccccccccccccccccccccccccc",
                "\\dddddddddddddddddddddddddddddddddddddddddddddddddd",
            ),
            concat!(
                "E:\\aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "\\bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                "\\cccccccccccccccccccccccccccccccccccccccccccccccccc",
                "\\dddddddddddddddddddddddddddddddddddddddddddddddddd",
                "\\e",
            ),
            "E:\\f",
        ];

        let lines = Cursor::new(dirlist.join("\n"));
        let compressed_lines = FrCompress::new(lines);
        let lines = Cursor::new(compressed_lines.filter_map(Result::ok).flatten().collect::<Vec<u8>>());
        let decompressed_lines = FrDecompress::new(lines);

        for (after, before) in decompressed_lines.filter_map(Result::ok).zip(dirlist) {
            assert_eq!(before, after);
        }
    }
}
