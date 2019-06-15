use {
        frcode::FrDecompress,
        std::env,
        std::fs::File,
        std::io::Read,
        serde::Deserialize,
        clap::{App, Arg},
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
        -i, --ignore-case      ignore case distinctions when matching patterns
        -l, --limit, -n LIMIT  limit output (or counting) to LIMIT entries
        -S, --statistics       don't search for entries, print statistics about each used database
        -r, --regexp REGEXP    search for basic regexp REGEXP instead of patterns
            --regex            patterns are extended regexps
        -w, --wholename        match whole path name (default)

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
        let f = unwrap!(File::open(stat));
        let stats = unwrap!(serde_json::from_reader(f));
    }
}
