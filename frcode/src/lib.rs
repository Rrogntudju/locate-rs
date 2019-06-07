use {
    std::{
        path::Path, 
        error::Error,
        io,
        io::{prelude::*, BufReader, BufWriter},
        fs::File,
        convert::TryFrom,
        },
};

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
    type Item = Result<Vec<u8>, Box<dyn Error>> ;

    fn next(&mut self) -> Option<Self::Item> {
        match self.lines.next()? {
            Ok(line) => 
                {
                    // https://www.gnu.org/software/findutils/manual/html_node/find_html/LOCATE02-Database-Format.html
                    let mut out_bytes: Vec<u8> = vec![];
                    if !self.init {
                        out_bytes.push(0);
                        out_bytes.extend_from_slice(b"LOCATEW\n");
                        self.init = true;
                    }

                    // Find the common prefix between the current and the previous line
                    let mut ctr: u16 = 0;
                    for (ch_line, ch_prec) in line.to_lowercase().chars().zip(self.prec.to_lowercase().chars()) {
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
                    out_bytes.extend_from_slice(line.chars().skip(ctr as usize).collect::<String>().as_bytes());
                    out_bytes.push(0x0a);

                    self.prec_ctr = ctr;
                    self.prec = line;

                    Some(Ok(out_bytes))
                },
            Err(err) => Some(Err(err.into()))
        }
    }
}

pub struct FrDecompress {
    init: bool,
    prec_ctr: u16,
    prec: String,
    lines: Box<dyn Iterator<Item = io::Result<u8>>>,
}

impl FrDecompress {
    pub fn new (reader: impl BufRead + 'static) -> FrDecompress {
            FrDecompress { 
                init: false,
                prec_ctr: 0,
                prec: "".into(),
                lines: Box::new(reader.bytes()),
            }
    }
}

impl Iterator for FrDecompress {
    type Item = Result<String, Box<dyn Error>> ;

    fn next(&mut self) -> Option<Self::Item> {
        Some(Ok("".into()))
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
