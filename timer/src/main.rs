use std::time::Duration;

use {
    ctrlc::set_handler,
    std::env,
    std::process::{exit, Command},
    std::sync::Arc,
    std::time::Instant,
};

const STATUS_CONTROL_C_EXIT: i32 = -1073741510; // 0xC000013A_u32

fn print_time(elapsed: Duration) {
    let secs = elapsed.as_secs();
    let m = secs / 60;
    let s = secs % 60;
    let ms = elapsed.subsec_millis();
    println!("\n{}m {}s {}ms", m, s, ms);
}

fn timer() -> i32 {
    let args: Vec<String> = env::args().collect();
    if args.len() > 1 {
        let mut cmd = Command::new(&args[1]);
        for arg in args.iter().skip(2) {
            cmd.arg(arg);
        }

        let start = Arc::new(Instant::now());
        let s = start.clone();
        set_handler(move || {
            print_time(s.elapsed());
            exit(STATUS_CONTROL_C_EXIT);
        })
        .expect("Error setting Ctrl-C handler");

        let status = cmd.status();
        let elapsed = start.elapsed();

        let mut exit_code = -1;
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
        print_time(elapsed);
        exit_code
    } else {
        0
    }
}

fn main() {
    exit(timer());
}
