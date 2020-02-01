use {
    ctrlc::set_handler,
    std::env,
    std::process::{exit, Command},
    std::sync::Arc,
    std::time::Instant,
};

const STATUS_CONTROL_C_EXIT: i32 = -1073741510; // 0xC000013A_u32

fn time_and_exit(elapsed: u128, exit_code: i32) {
    let m = elapsed / 60_000;
    let rms = elapsed % 60_000;
    let s = rms / 1_000;
    println!("\n{}m {}s {}ms", m, s, rms % 1_000);
    exit(exit_code);
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() > 1 {
        let mut cmd = Command::new(&args[1]);
        for arg in args.iter().skip(2) {
            cmd.arg(arg);
        }

        let start = Arc::new(Instant::now());
        let s = start.clone();
        set_handler(move || {
            time_and_exit(s.elapsed().as_millis(), STATUS_CONTROL_C_EXIT);
        })
        .expect("Error setting Ctrl-C handler");

        let status = cmd.status();
        let elapsed = start.elapsed().as_millis();

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
        time_and_exit(elapsed, exit_code);
    } else {
        time_and_exit(0, 0);
    }
}
