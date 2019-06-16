use {
        frcode::FrDecompress,
        std::env,
        std::fs::File,
        std::io::BufReader,
        serde::Deserialize,
        clap::{App, Arg},
        num_format::{Locale, ToFormattedString},
        glob::Pattern,
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

fn main() {
    /*
        -A, --all              only print entries that match all patterns
        -b, --basename         match only the base name of path names
        -c, --count            only print number of found entries 
        -h, --help             print this help
        -l, --limit, -n LIMIT  limit output (or counting) to LIMIT entries
        -S, --statistics       don't search for entries, print statistics about each used database
    */
    let matches = App::new("locate")
                    .arg(Arg::with_name("stats")
                        .help("pas de recherche, affiche les statistiques de la base de données") 
                        .short("S")                   
                        .long("statistics")
                    )
                    .get_matches();
    

    if matches.is_present("stats") {
        let mut stat = env::temp_dir();
        stat.push("locate");
        stat.set_extension("txt");
        let reader = BufReader::new(unwrap!(File::open(stat)));
        let stats: Statistics = unwrap!(serde_json::from_reader(reader));
        let loc = &Locale::fr_CA;
        println!("Base de données locate.db :");
        println!("      {} répertoires", stats.dirs.to_formatted_string(loc));
        println!("      {} fichiers", stats.files.to_formatted_string(loc));
        println!("      {} octets dans les noms de fichier", stats.files_bytes.to_formatted_string(loc));
        println!("      {} octets utilisés pour stocker la base de données", stats.db_size.to_formatted_string(loc));
        println!("      {} min {} sec pour générer la base de données", stats.elapsed / 60, stats.elapsed % 60);
        return;
    }
}
