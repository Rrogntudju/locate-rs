use {
        frcode::FrDecompress,
        std::env,
        std::fs::File,
        std::io::{BufReader, BufWriter, Write, stdout},
        std::thread,
        std::sync::mpsc,
        serde_json::Value,
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

fn is_usize(v: String) -> Result<(), String> {
    match v.parse::<usize>() {
        Ok(_) => Ok(()),
        Err(_) => Err(v),
    }
}

fn main() {
    let matches = App::new("locate")
                    .version("0.5.0")
                    .arg(Arg::with_name("stats")
                        .help("don't search for entries, print statistics about database") 
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
                        .validator(is_usize)
                    )
                    .arg(Arg::with_name("pattern")
                        .required_unless("stats")
                        .min_values(1)
                    )
                    .get_matches();
    
    let loc = &Locale::fr_CA;
    if matches.is_present("stats") {
        let mut stat = env::temp_dir();
        stat.push("locate");
        stat.set_extension("txt");
        if !stat.is_file() {
            eprintln!("La base de données n'existe pas. Exécuter updatedb.exe");
            return;
        }
        let reader = BufReader::new(unwrap!(File::open(stat)));
        let stats: Value = unwrap!(serde_json::from_reader(reader));
        let dirs = stats["dirs"].as_u64().unwrap();
        let files = stats["files"].as_u64().unwrap();
        let files_bytes = stats["files_bytes"].as_u64().unwrap();
        let db_size = stats["db_size"].as_u64().unwrap();
        let elapsed = stats["elapsed"].as_u64().unwrap();
        println!("Base de données locate.db :");
        println!("      {} répertoires", dirs.to_formatted_string(loc));
        println!("      {} fichiers", files.to_formatted_string(loc));
        println!("      {} octets dans les noms de fichier", files_bytes.to_formatted_string(loc));
        println!("      {} octets utilisés pour stocker la base de données", db_size.to_formatted_string(loc));
        println!("      {} min {} sec pour générer la base de données", elapsed / 60, elapsed % 60);
        return;
    }
    
    let is_limit =  matches.is_present("limit");
    let limit =
        if is_limit {
            matches.value_of("limit").unwrap().parse::<usize>().unwrap()
        } else {
            0
        };
    let is_count =  matches.is_present("count");
    if is_limit && limit == 0 {
        if is_count {
            println!("0");
        }
        return; // nothing to do
    }
    let is_all =  matches.is_present("all");
    let is_base = matches.is_present("base");
    let patterns = matches.values_of("pattern").unwrap();
    
    let mut glob_pat = vec!();
    for pattern in patterns {
        let pat = 
            if pattern.starts_with("/") {
                pattern.splitn(2, '/').collect::<Vec<&str>>()[1].to_owned()     // pattern «as is» 
            } else if pattern.starts_with("*") || pattern.ends_with("*") {
                pattern.to_owned()      // pattern «as is» 
            } else {
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
    
    // Case-insensitive for ASCII characters only.
    // Working around this issue with to_lowercase() is too costly.
    // Use a pattern like [éÉ] as a workaround.
    let mo = MatchOptions {
        case_sensitive: false,  
        require_literal_separator: false,
        require_literal_leading_dot: false
    };

    let mut db = env::temp_dir();
    db.push("locate");
    db.set_extension("db");
    if !db.is_file() {
        eprintln!("La base de données n'existe pas. Exécuter updatedb.exe");
        return;
    }
    let stdout = stdout();
    let mut out = BufWriter::new(stdout.lock());      
    let mut ctr:usize = 0;

    // run the FrDecompress iterator on his own thread
    let (tx, rx) = mpsc::sync_channel(10_000);
    thread::spawn(move || {
        let decompressed_entries = FrDecompress::new(BufReader::new(unwrap!(File::open(db))));
        for entry in decompressed_entries {
            if let Err(e) = tx.send(unwrap!(entry)) {
                if !is_limit {
                    eprintln!("{}", e);
                }
                return; 
            }
        }
    });

    for entry in rx {
        let is_dir = entry.ends_with('\\');   // dir entries are terminated with a \
        if is_base && is_dir {
            continue;    // no need to match on a dir entry
        }

        let entry_test = 
            if is_base {
                entry.rsplitn(2, '\\').collect::<Vec<&str>>()[0]    // basename
            } else if is_dir {
                entry.rsplitn(2, '\\').collect::<Vec<&str>>()[1]    // dir entry minus the \    
            } else {
                &entry
            };

        if is_all && !glob_pat.iter().all(|p| p.matches_with(entry_test, mo)) {
            continue;
        } else if !glob_pat.iter().any(|p| p.matches_with(entry_test, mo)) {
            continue;
        }

        if !is_count {
            let entry_out =
                if is_dir {
                    entry_test   // dir entry minus the \
                } else {
                    &entry
                };
            unwrap!(out.write_all(entry_out.as_bytes()));
            unwrap!(out.write_all(b"\n"));
        }

        ctr += 1;
        if is_limit && ctr == limit {
            break;
        }
    }
    
    if is_count {
        unwrap!(write!(out, "{}\n", ctr.to_formatted_string(loc)));
    }
}