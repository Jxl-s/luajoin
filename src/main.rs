use clap::Parser;
use colorize::AnsiColor;
use std::io;
use std::process;

mod cli;
mod config;
mod console;
mod parser;
mod path;
mod build;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// init, serve, or build
    #[arg(default_value = "build")]
    action: String,

    /// Whether to minify the output or not (only for build)
    #[arg(long, default_value = "false")]
    minify: bool,
}

fn main() {
    let args = Args::parse();

    match args.action.as_str() {
        "init" => {
            console::clear();
            console::log(&"Initializing Project...".blue());

            // Check if the config file already exists
            if let Some(_) = config::get_config() {
                console::log(&"Project file already exists".red());
                return;
            }

            let stdin = io::stdin();

            // Ask for source directory
            let mut src_dir = String::new();
            console::log_inline(&"Enter the source directory (src): ".magenta());

            if let Err(err) = stdin.read_line(&mut src_dir) {
                console::log_error(&format!("Error reading source dir: {}", err));
                return;
            };

            // Ask for output directory
            let mut out_dir = String::new();
            console::log_inline(&"Enter the output directory (out): ".magenta());

            if let Err(err) = stdin.read_line(&mut out_dir) {
                console::log_error(&format!("Error reading output dir: {}", err));
                return;
            }

            // Ask for entry file
            let mut entry = String::new();
            console::log_inline(&"Enter the entry file (main): ".magenta());

            if let Err(err) = stdin.read_line(&mut entry) {
                console::log_error(&format!("Error reading entry file: {}", err));
                return;
            }

            // Trim the inputs
            let src_dir = src_dir.trim();
            let out_dir = out_dir.trim();
            let entry = entry.trim();

            // If the inputs are empty, give them their default value
            let src_dir = if src_dir.is_empty() { "src" } else { src_dir };
            let out_dir = if out_dir.is_empty() { "out" } else { out_dir };
            let entry = if entry.is_empty() { "main" } else { entry };

            // Create the project file
            match config::create_config_file(src_dir, out_dir, entry) {
                Ok(_) => console::log(&"Project successfully created".green()),
                Err(err) => console::log_error(&format!("Problem creating project: {}", err).red()),
            };
        }
        "serve" => {
            console::clear();

            // Initially check for config
            let config = config::get_config().unwrap_or_else(|| {
                console::log_error("Project file not found");
                process::exit(1);
            });

            // Run the CLI and server
            cli::run_server(config.clone());

            // Run the bundler
            cli::run_bundler(config.clone());
        }
        "build" => {
            console::clear();

            let config = config::get_config().unwrap_or_else(|| {
                console::log_error("Project file not found");
                process::exit(1);
            });

            cli::build_project(config);
        }
        _ => console::log_error(&"Invalid action".red()),
    };
}
