use {
    std::time::Instant,
    std::process::{Command, exit},
    std::sync::Arc,
    std::env,
    ctrlc::set_handler,
};

fn time_and_exit(elapsed: u128, exit_code: i32) {
    let s = elapsed / 1_000;
    let m = s / 60;
    println!("\n{}m{}.{}s", m,  s, elapsed % 1_000);
    exit(exit_code);
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut elapsed = 0;
    let mut exit_code = 0;
    
    if args.len() > 1 {
        let mut cmd = Command::new(&args[1]);
        for arg in args.iter().skip(2) {
            cmd.arg(arg);
        }

        let start = Arc::new(Instant::now());
        let s = start.clone();
        set_handler(move || {
            time_and_exit(s.elapsed().as_millis(), 0);
        })
        .expect("Error setting Ctrl-C handler");

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

    time_and_exit(elapsed, exit_code);
}
