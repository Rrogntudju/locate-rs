use {
    std::{
        path::Path, 
        error::Error,
        io::{prelude::*, BufReader, BufWriter},
        fs::File,
        convert::TryFrom,
        },
};

struct FrCompress {
    init: bool,
    prec_ctr: i16,
    prec: String,
    lines: Box<dyn Iterator<Item = std::io::Result<String>>>,
}

impl FrCompress {
    fn new (file: &Path) -> std::io::Result<FrCompress> {
        let f = File::open(file)?;
        let reader = BufReader::new(f);
    
        Ok( FrCompress { 
                init: false,
                prec_ctr: 0,
                prec: "".into(),
                lines: Box::new(reader.lines()),
            }
        )
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
                        out_bytes.extend_from_slice("LOCATEW".as_bytes());
                        out_bytes.push(0x0a);
                        self.init = true;
                    }

                    // Find the common prefix between the current and the previous line
                    let mut ctr = 0;
                    for (ch_line, ch_prec) in line.to_lowercase().chars().zip(self.prec.to_lowercase().chars()) {
                        if ch_line == ch_prec {
                            ctr = ctr + 1;
                        }
                        else {
                            break;
                        }
                    }

                    // Output the offset-differential count
                    let offset: i16 = ctr - self.prec_ctr;
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


pub fn compress_file(in_file: &Path, out_file: &Path) -> Result<(), Box<dyn Error>> {
    let compressed_lines = FrCompress::new(in_file)?;
    let f = File::open(out_file)?;
    let mut writer = BufWriter::new(f);
    
    for line in compressed_lines {
        writer.write(&line?)?;
    }
    
    Ok(())
}

pub fn decompress_file(in_file: &Path, out_file: &Path) -> Result<(), Box<dyn Error>> {

    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
