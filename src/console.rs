use std::io::Write;

use colorize::AnsiColor;

pub fn clear() {
    print!("\x1B[2J\x1B[1;1H");
    std::io::stdout().flush().unwrap();
}

pub fn log_error(text: &str) {
    let cur_time = chrono::Local::now().format("%H:%M:%S").to_string();

    // Log to stderr, with red text
    eprintln!(
        "{} {} | {}",
        cur_time.black(),
        "LuaJoin".yellow(),
        text.to_string().red()
    );
}

pub fn log(text: &str) {
    // clear the current line
    print!("\x1B[2K\r");
    std::io::stdout().flush().unwrap();

    // get the current time in hh:mm:ss, with chrono
    let cur_time = chrono::Local::now().format("%H:%M:%S").to_string();
    println!("{} {} | {}", cur_time.black(), "LuaJoin".yellow(), text);
    print!("> ");
    std::io::stdout().flush().unwrap();
}

pub fn log_inline(text: &str) {
    // clear the current line
    print!("\x1B[2K\r");
    std::io::stdout().flush().unwrap();
    
    // get the current time in hh:mm:ss, with chrono
    let cur_time = chrono::Local::now().format("%H:%M:%S").to_string();
    print!("{} {} | {}", cur_time.black(), "LuaJoin".yellow(), text);
    std::io::stdout().flush().unwrap();
}
