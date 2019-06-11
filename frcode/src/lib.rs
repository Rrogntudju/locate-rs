use {
    std::{
        path::Path, 
        error::Error,
        io, io::{prelude::*, BufReader, BufWriter},
        fs::File,
        convert::TryFrom,
        fmt,
        },
    };

#[derive(Debug)]
pub enum FrError {
    InvalidLengthError,
    InvalidLabelError,
}

impl Error for FrError {}

impl fmt::Display for FrError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            FrError::InvalidLengthError => write!(f, "La longueur du suffixe est invalide"),
            FrError::InvalidLabelError => write!(f, "Fichier updateDB invalide"),
        }
    }
}

pub struct FrCompress {
    init: bool,
    prec_ctr: u16,
    prec: String,
    lines: Box<dyn Iterator<Item = io::Result<String>>>,
}

impl FrCompress {
    pub fn new (reader: impl BufRead + 'static) -> FrCompress {
           FrCompress { 
                init: false,
                prec_ctr: 0,
                prec: "".into(),
                lines: Box::new(reader.lines()),
            }
    }
}

impl Iterator for FrCompress {
    type Item = io::Result<Vec<u8>>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.lines.next()? {
            Ok(line) => 
                {
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
                    let mut ctr: u16 = 0;
                    for (ch_line, ch_prec) in line.chars().zip(self.prec.chars()) {
                        if ch_line == ch_prec {
                            ctr = ctr + 1;
                        }
                        else {
                            break;
                        }
                    }

                    // Output the offset-differential count
                    let offset: i16 = ctr as i16 - self.prec_ctr as i16;
                    if let Ok(offset_i8) = i8::try_from(offset) {
                        out_bytes.extend_from_slice(&offset_i8.to_be_bytes()); // 1 byte offset
                    }
                    else {
                        out_bytes.push(0x80);
                        out_bytes.extend_from_slice(&offset.to_be_bytes()); // 2 bytes offset big-endian
                    }

                    // Output the line without the prefix
                    let suffix = line.chars().skip(ctr as usize).collect::<String>();
                    let len: i16 = suffix.len() as i16;  // length in bytes
                    if let Ok(len_i8) = i8::try_from(len) {
                        out_bytes.extend_from_slice(&len_i8.to_be_bytes()); // 1 byte length
                    }
                    else {
                        out_bytes.push(0x80);
                        out_bytes.extend_from_slice(&len.to_be_bytes()); // 2 bytes length big-endian
                    }
                    out_bytes.extend_from_slice(suffix.as_bytes());

                    self.prec_ctr = ctr;
                    self.prec = line;

                    Some(Ok(out_bytes))
                },

            Err(err) => Some(Err(err))
        }
    }
}

pub struct FrDecompress {
    init: bool,
    prec_ctr: u16,
    prec: String,
    bytes: Box<dyn Iterator<Item = io::Result<u8>>>,
}

impl FrDecompress {
    pub fn new (reader: impl BufRead + 'static) -> FrDecompress {
            FrDecompress { 
                init: false,
                prec_ctr: 0,
                prec: "".into(),
                bytes: Box::new(reader.bytes()),
            }
    }
    
    fn count_from_bytes(&mut self) -> Option<i16> {
        let bytes_mut = &mut self.bytes;
        let count_1b = bytes_mut.take(1).map(|b| b.unwrap_or_default()).collect::<Vec<u8>>();
        if count_1b.len() != 1 {
            return None;
        }
        if count_1b[0] != 0x80 {
            Some(i8::from_be_bytes([count_1b[0]]) as i16)
        }
        else {
            let count_2b = bytes_mut.take(2).map(|b| b.unwrap_or_default()).collect::<Vec<u8>>();
            if count_1b.len() != 2 {
                return None;
            }
            let mut buf = [0,0];
            buf.copy_from_slice(&count_2b);
            Some(i16::from_be_bytes(buf))
        }
    }

    fn suffix_from_bytes(&mut self, len: i16) -> Option<Result<String, Box<dyn Error>>>  {
        if len <= 0 {
            return Some(Err(FrError::InvalidLengthError.into()));
        }
        let bytes_mut = &mut self.bytes;
        let suffix = bytes_mut.take(len as usize).map(|b| b.unwrap_or_default()).collect::<Vec<u8>>();
        if suffix.len() != len as usize {
            return None;
        }
        
        Some(
            match String::from_utf8(suffix) {
                Ok(suffix) => Ok(suffix),
                Err(err) => Err(err.into())
            }
        )
    }
}

impl Iterator for FrDecompress {
    type Item = Result<String, Box<dyn Error>> ;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.init {
            let _ = self.bytes.next()?;  // Skip the offset
            let len = self.count_from_bytes()?;
            let label = 
                match self.suffix_from_bytes(len)? {
                    Ok(label) => label,
                    Err(err) => return Some(Err(err.into())),
                };
            
            if label == "LOCATEW" {
                self.init = true;
            }
            else {
                return Some(Err(FrError::InvalidLabelError.into())); 
            }
        }

        let offset = self.count_from_bytes()?;  // end of valid updateDB file happens here
        let len = self.count_from_bytes()?;
        let suffix = 
            match self.suffix_from_bytes(len)? {
                    Ok(suffix) => suffix,
                    Err(err) => return Some(Err(err.into())),
            };

        let len_prefix = self.prec_ctr as i16 + offset; // length in chars
        let prefix = self.prec.chars().take(len_prefix as usize).collect::<String>();
        let line = prefix + &suffix;

        self.prec_ctr = len_prefix as u16;
        self.prec = line.clone();

        Some(Ok(line))
    }
}

pub fn compress_file(in_file: &Path, out_file: &Path) -> Result<(), Box<dyn Error>> {
    let f = File::open(in_file)?;
    let reader = BufReader::new(f);
    let compressed_lines = FrCompress::new(reader);
    
    let f = File::open(out_file)?;
    let mut writer = BufWriter::new(f);
    
    for line in compressed_lines {
        writer.write(&line?)?;
    }
    
    Ok(())
}

pub fn decompress_file(in_file: &Path, out_file: &Path) -> Result<(), Box<dyn Error>> {
    let f = File::open(in_file)?;
    let reader = BufReader::new(f);
    let decompressed_lines = FrDecompress::new(reader);

    let f = File::open(out_file)?;
    let mut writer = BufWriter::new(f);
    
    for line in decompressed_lines {
        writer.write(line?.as_bytes())?;
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use io::Cursor;

    #[test]
    fn compress_decompress_ok() {
        let dirlist = vec!(
            "C:\\Users", 
            "C:\\Users\\Fourmilier",
            "C:\\Users\\Fourmilier\\Documents\\Bébé Aardvark.jpg",
            "C:\\Users\\Fourmilier\\Documents\\Bébé Armadillo.jpg",
            "C:\\Windows",
            "D:\\ماريو.txt",
            concat!("E:\\aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                    "\\bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                    "\\cccccccccccccccccccccccccccccccccccccccccccccccccc"
            ),
            concat!("E:\\aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                    "\\bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                    "\\cccccccccccccccccccccccccccccccccccccccccccccccccc",
                    "\\d"
            ),
            "E:\\e", 
            );

        let lines = Cursor::new(dirlist.join("\n"));
        let compressed_lines = FrCompress::new(lines);
        let lines = Cursor::new(compressed_lines.map(|l| l.unwrap_or_default()).flatten().collect::<Vec<u8>>());
        let decompressed_lines = FrDecompress::new(lines);

        for (after, before) in decompressed_lines.map(|l| l.unwrap_or_default()).zip(dirlist) {
            assert_eq!(before, after);
        }
    }
}
