use std::io::Write;

use colorize::AnsiColor;

pub fn clear() {
    print!("\x1B[2J\x1B[1;1H");
    std::io::stdout().flush().unwrap();
}

pub fn log(text: &str) {
    // get the current time in hh:mm:ss, with chrono
    let cur_time = chrono::Local::now().format("%H:%M:%S").to_string();
    println!("{} {} | {}", cur_time.black(), "LPack".yellow(), text);
}

pub fn log_inline(text: &str) {
    // get the current time in hh:mm:ss, with chrono
    let cur_time = chrono::Local::now().format("%H:%M:%S").to_string();
    print!("{} {} | {}", cur_time.black(), "LPack".yellow(), text);
    std::io::stdout().flush().unwrap();
}