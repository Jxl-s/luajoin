use std::env;

mod config;
mod console;
mod parser;
mod path;

fn main() {
    let args: Vec<String> = env::args().collect();
}
