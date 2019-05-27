use {
    std::{
        path::Path, 
        error::Error,
        io::{prelude::*, BufReader, BufWriter},
        fs::File,
        },
};

struct FrCompress {
    count: i16,
    prefix: String,
    lines: Box<dyn Iterator<Item = std::io::Result<String>>>,
}

impl FrCompress {
    fn new (file: &Path) -> std::io::Result<FrCompress> {
        let f = File::open(file)?;
        let reader = BufReader::new(f);
    
        Ok( FrCompress { 
                count: 0,
                prefix: "".into(),
                lines: Box::new(reader.lines()),
            }
        )
    }
}

impl Iterator for FrCompress {
    type Item = Result<Vec<u8>, Box<dyn Error>> ;

    fn next(&mut self) -> Option<Self::Item> {
        match self.lines.next()? {
            Ok(line) => {Some(Ok(vec!()))},
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

pub fn fr_compress(txt : &str, count: i16) -> (Vec<u8>, i16) {

(vec!(), 0)
}

pub fn fr_decompress(rec: &[u8], prefix: i16) -> (String, i16) {

("".into(), 0)
}


#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
