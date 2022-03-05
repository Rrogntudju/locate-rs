use {
    clap::{Arg, Command},
    frcode::FrDecompress,
    globset::{Candidate, GlobBuilder, GlobSetBuilder},
    num_format::{Locale, ToFormattedString},
    serde_json::Value,
    std::env,
    std::error::Error,
    std::fs::File,
    std::io::{stdout, BufReader, BufWriter, Write},
    std::thread,
};

const PAS_DE_BD: &str = "La base de données est inexistante. Exécuter updatedb.exe";

fn is_usize(v: &str) -> Result<(), String> {
    match v.parse::<usize>() {
        Ok(_) => Ok(()),
        Err(_) => Err(v.to_owned()),
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let matches = Command::new("locate")
        .version("0.6.8")
        .arg(
            Arg::new("stats")
                .help("don't search for entries, print statistics about database")
                .short('S')
                .long("statistics"),
        )
        .arg(Arg::new("all").help("only print entries that match all patterns").short('a').long("all"))
        .arg(
            Arg::new("base")
                .help("match only the base name of path names")
                .short('b')
                .long("basename"),
        )
        .arg(Arg::new("count").help("only print number of found entries").short('c').long("count"))
        .arg(
            Arg::new("case")
                .help("case distinctions when matching patterns")
                .short('C')
                .long("case-sensitive"),
        )
        .arg(
            Arg::new("limit")
                .help("limit output (or counting) to LIMIT entries")
                .short('l')
                .long("limit")
                .takes_value(true)
                .validator(is_usize),
        )
        .arg(Arg::new("pattern").required_unless_present("stats").min_values(1))
        .get_matches();

    let loc = &Locale::fr_CA;
    if matches.is_present("stats") {
        let mut stat = env::temp_dir();
        stat.push("locate");
        stat.set_extension("txt");
        if !stat.is_file() {
            return Err(PAS_DE_BD.into());
        }
        let reader = BufReader::new(File::open(stat)?);
        let stats: Value = serde_json::from_reader(reader)?;
        let dirs = stats["dirs"].as_u64().unwrap_or(0);
        let files = stats["files"].as_u64().unwrap_or(0);
        let files_bytes = stats["files_bytes"].as_u64().unwrap_or(0);
        let db_size = stats["db_size"].as_u64().unwrap_or(0);
        let elapsed = stats["elapsed"].as_u64().unwrap_or(0);
        println!("Base de données locate.db :");
        println!("      {} répertoires", dirs.to_formatted_string(loc));
        println!("      {} fichiers", files.to_formatted_string(loc));
        println!("      {} octets dans les noms de fichier", files_bytes.to_formatted_string(loc));
        println!(
            "      {} octets utilisés pour stocker la base de données",
            db_size.to_formatted_string(loc)
        );
        println!("      {} min {} sec pour générer la base de données", elapsed / 60, elapsed % 60);
        return Ok(());
    }

    let is_limit = matches.is_present("limit");
    let limit = matches.value_of("limit").unwrap_or("0").parse::<usize>().unwrap_or(0);

    let is_count = matches.is_present("count");
    if is_limit && limit == 0 {
        if is_count {
            println!("0");
        }
        return Ok(()); // nothing to do
    }

    let mut db = env::temp_dir();
    db.push("locate");
    db.set_extension("db");
    if !db.is_file() {
        return Err(PAS_DE_BD.into());
    }
    let db_file = File::open(db)?;

    // run the FrDecompress iterator on his own thread
    let (tx, rx) = flume::unbounded();
    thread::spawn(move || {
        let decompressed_entries = FrDecompress::new(BufReader::new(db_file));
        for entry in decompressed_entries {
            if let Err(e) = tx.send(entry.unwrap()) {
                if !is_limit {
                    eprintln!("{}", e);
                }
                break;
            }
        }
    });

    let is_all = matches.is_present("all");
    let is_base = matches.is_present("base");
    let is_case = matches.is_present("case");
    let patterns = matches.values_of("pattern").unwrap();

    let mut gs_builder = GlobSetBuilder::new();
    for pattern in patterns {
        let pattern = if let Some(pattern) = pattern.strip_prefix('/') {
            pattern.to_owned() // pattern «as is»
        } else if pattern.starts_with("*") || pattern.ends_with("*") {
            pattern.to_owned() // pattern «as is»
        } else {
            format!("*{}*", pattern) // implicit globbing
        };

        let g_builder = GlobBuilder::new(&pattern)
            .case_insensitive(!is_case)
            .literal_separator(false)
            .backslash_escape(false)
            .build()?;

        gs_builder.add(g_builder);
    }

    let gs = gs_builder.build()?;
    let glob_count = gs.len();

    let stdout = stdout();
    let mut out = BufWriter::new(stdout.lock());
    let mut ctr: usize = 0;

    for entry in rx {
        // dir entries are terminated with a \
        let (entry_out, is_dir) = match entry.strip_suffix('\\') {
            Some(dir) => (dir, true),
            None => (entry.as_str(), false),
        };

        let candidate = if is_base {
            if is_dir {
                continue; // no need to match on a dir entry
            }
            Candidate::new(entry_out.rsplit_once('\\').unwrap().1) // basename
        } else {
            Candidate::new(entry_out)
        };

        if glob_count == 1 || !is_all {
            if !gs.is_match_candidate(&candidate) {
                continue;
            }
        } else if gs.matches_candidate(&candidate).len() != glob_count {
            continue;
        }

        if !is_count {
            out.write_all(entry_out.as_bytes())?;
            out.write_all(b"\n")?;
        }

        ctr += 1;
        if is_limit && ctr == limit {
            break;
        }
    }

    if is_count {
        writeln!(out, "{}", ctr.to_formatted_string(loc))?;
    }
    out.flush()?;

    Ok(())
}
