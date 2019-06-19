use {
        frcode::FrDecompress,
        std::env,
        std::fs::File,
        std::io::{BufReader, BufWriter, Write, stdout},
        serde::Deserialize,
        clap::{App, Arg},
        num_format::{Locale, ToFormattedString},
        glob::{Pattern, MatchOptions},
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

#[derive(Deserialize)]
struct Statistics {
    dirs: usize,
    files: usize,
    files_bytes: usize,
    db_size: usize,
    elapsed: u64,
}

fn is_usize(v: String) -> Result<(), String> {
    match v.parse::<usize>() {
        Ok(_) => Ok(()),
        Err(_) => Err(v),
    }
}

fn main() {
    let limit_max = std::usize::MAX.to_string();
    let matches = App::new("locate")
                    .arg(Arg::with_name("stats")
                        .help("don't search for entries, print statistics about each used database") 
                        .short("s")                   
                        .long("statistics")
                    )
                    .arg(Arg::with_name("all")
                        .help("only print entries that match all patterns") 
                        .short("a")                   
                        .long("all")
                    )
                    .arg(Arg::with_name("base")
                        .help("match only the base name of path names") 
                        .short("b")                   
                        .long("basename")
                    )
                    .arg(Arg::with_name("count")
                        .help("only print number of found entries") 
                        .short("c")                   
                        .long("count")
                    )
                    .arg(Arg::with_name("limit")
                        .help("limit output (or counting) to LIMIT entries") 
                        .short("l")
                        .long("limit")
                        .takes_value(true)
                        .default_value(&limit_max)
                        .validator(is_usize)
                    )
                    .arg(Arg::with_name("pattern")
                        .required_unless("stats")
                        .min_values(1)
                        .value_delimiter(" ")
                    )
                    .get_matches();
    
    let loc = &Locale::fr_CA;
    if matches.is_present("stats") {
        let mut stat = env::temp_dir();
        stat.push("locate");
        stat.set_extension("txt");
        let reader = BufReader::new(unwrap!(File::open(stat)));
        let stats: Statistics = unwrap!(serde_json::from_reader(reader));
        println!("Base de données locate.db :");
        println!("      {} répertoires", stats.dirs.to_formatted_string(loc));
        println!("      {} fichiers", stats.files.to_formatted_string(loc));
        println!("      {} octets dans les noms de fichier", stats.files_bytes.to_formatted_string(loc));
        println!("      {} octets utilisés pour stocker la base de données", stats.db_size.to_formatted_string(loc));
        println!("      {} min {} sec pour générer la base de données", stats.elapsed / 60, stats.elapsed % 60);
        return;
    }
    
    let mut out = BufWriter::new(stdout());    // should be faster than looping over println!()
    let mut ctr:usize = 0;  
    let limit = matches.value_of("limit").unwrap().parse::<usize>().unwrap();
    let is_count =  matches.is_present("count");
    let is_all =  matches.is_present("all");
    let is_base = matches.is_present("base");
    let patterns = matches.values_of("pattern").unwrap();
    
    let mut glob_pat = vec!();
    for pattern in patterns {
        let pat = 
            if pattern.as_bytes().first().unwrap() == &b'/' {
                pattern[1..].to_owned()   // pattern «as is» 
            }
            else {
                format!("*{}*", pattern)  // implicit globbing 
            };

        match Pattern::new(&pat) {
            Ok(p) => glob_pat.push(p),
            Err(e) => {
                eprintln!("«{}» : {}", pat, e);
                return;
            }
        }
    }

    let mo = MatchOptions {
        case_sensitive: false,
        require_literal_separator: false,
        require_literal_leading_dot: false
    };

    let mut db = env::temp_dir();
    db.push("locate");
    db.set_extension("db");
    let reader = BufReader::new(unwrap!(File::open(db)));
    for entry in FrDecompress::new(reader) {
        let entry = unwrap!(entry);
        let is_dir = entry.as_bytes().last().unwrap() == &b'\\';   // dir entries are terminated with a \
        let entry_test =
            if is_base {
                if is_dir {
                    continue;   // no need to match on a dir entry
                }
                else {
                    // match on the basename
                    let idx = entry.as_bytes().iter().rev().position(|b| b == &b'\\').unwrap(); // find the index of the last \
                    &entry[entry.len() - idx..]   // basename
                }
            }
            else {
                if is_dir {
                    &entry[..entry.len() - 1]   // dir entry minus the \
                }
                else {
                    &entry
                }
            };

        if is_all {
            if !glob_pat.iter().all(|p| p.matches_with(&entry_test, mo)) {
                continue;
            } 
        }
        else {
            if !glob_pat.iter().any(|p| p.matches_with(&entry_test, mo)) {
                continue;
            } 
        }

        if !is_count {
            let entry_out =
                if is_dir {
                    &entry[..entry.len() - 1]   // dir entry minus the \
                }
                else {
                    &entry
                };
            unwrap!(write!(out, "{}\n", entry_out));
        }

        ctr = ctr + 1;
        if ctr == limit {
            break;
        }
    }
    
    if is_count {
        unwrap!(write!(out, "{}\n", ctr.to_formatted_string(loc)));
    }
}
 

