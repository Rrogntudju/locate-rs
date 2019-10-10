fn main() {
    use {
        std::time::Instant,
        std::process::{Command, exit},
        std::env,
    };

    let args: Vec<String> = env::args().collect();
    let mut elapsed = 0;
    let mut exit_code = 0;
    
    if args.len() > 1 {
        let mut cmd = Command::new(&args[1]);
        for arg in args.iter().skip(2) {
            cmd.arg(arg);
        }

        let start = Instant::now();
        let status = cmd.status();
        elapsed = start.elapsed().as_millis();
        
        match status {
            Ok(s) => {
                if let Some(code) = s.code() {
                    exit_code = code;
                }
            }
            Err(e) => {
                println!("{}", e);
                if let Some(code) = e.raw_os_error() {
                    exit_code = code;
                }
            }
        }
    }

    let s = elapsed / 1_000;
    let m = s / 60;
    println!("{}m{}.{}s", m,  s, elapsed % 1_000);
    exit(exit_code);
}
