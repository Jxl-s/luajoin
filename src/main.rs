use clap::Parser;
use colorize::AnsiColor;
use config::Config;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use parser::RequireVisitor;
use std::{
    collections::HashMap,
    env, fs, io,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

mod config;
mod console;
mod parser;
mod path;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]

/// Joins Lua files into a single file
struct Args {
    /// init, serve, or build
    #[arg(default_value = "build")]
    action: String,

    /// Whether to minify the output or not (only for build)
    #[arg(long, default_value = "false")]
    minify: bool,
}

fn make_bundle(parser: &mut RequireVisitor, config: &Config) {
    // If the output directory does not exist, create it
    if !Path::new(&config.out_dir).exists() {
        fs::create_dir(&config.out_dir).unwrap();
    }

    let start_time = SystemTime::now();

    // Build the file project
    let bundle_result = match parser.generate_bundle(true) {
        Ok(bundle) => bundle,
        Err(err) => {
            console::log_error(&format!("Problem generating bundle: {}", err));
            return;
        }
    };

    // Write the bundle to the output file
    match fs::write(&(config.out_dir.to_owned() + "/bundle.lua"), &bundle_result) {
        Ok(_) => (),
        Err(err) => {
            console::log_error(&format!("Problem writing bundle: {}", err));
            return;
        }
    };

    console::log(
        &format!(
            "Successfully generated bundle in {}ms!",
            start_time.elapsed().unwrap().as_millis()
        )
        .green(),
    );
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
            // Read the config file
            let config = config::get_config();
            if let None = config {
                console::log_error("Project file not found");
                return;
            }

            let config = config.unwrap();
            console::clear();

            // Debouncing for files
            let mut debounce: HashMap<String, SystemTime> = HashMap::new();
            let debounce_time = 50; // in MS

            // Create the parser
            let mut require_visitor = RequireVisitor::new(&config.src_dir, &config.entry_file);
            make_bundle(&mut require_visitor, &config);

            // Create the bundler
            let (tx, rx) = std::sync::mpsc::channel();
            let mut watcher = RecommendedWatcher::new(tx, notify::Config::default()).unwrap();

            watcher
                .watch(Path::new(&config.src_dir), RecursiveMode::Recursive)
                .unwrap();

            for e in rx {
                let e = e.unwrap();

                // Modify(Data(Content)) is the file update event
                // Create(File) is the file creation event
                // Modify(Name(Any)) is the file rename event, and removal
                let valid_event = match e.kind {
                    notify::EventKind::Modify(notify::event::ModifyKind::Data(_)) => true,
                    notify::EventKind::Create(_) => true,
                    notify::EventKind::Modify(notify::event::ModifyKind::Name(_)) => true,
                    _ => false,
                };

                if !valid_event {
                    continue;
                }

                let mut marked_file_count = 0;
                for file in &e.paths {
                    // Find the last event
                    let file_name = file.to_str().unwrap().to_string();

                    // if it doesnt exist in the map, add an instance of unix 0
                    let last_file_event = debounce
                        .entry(file_name.clone())
                        .or_insert(SystemTime::from(UNIX_EPOCH));

                    if last_file_event.elapsed().unwrap().as_millis() < debounce_time as u128 {
                        continue;
                    }

                    debounce.insert(file_name.clone(), SystemTime::now());
                    marked_file_count += 1;

                    // Parse the file's relative path in the project
                    let cur_dir = env::current_dir().unwrap().to_str().unwrap().to_string();

                    // Find the offset, then get the relative file
                    let file_offset = cur_dir.len() + config.src_dir.len() + 2;
                    let relative_file = file_name[file_offset..].to_string().replace("\\", "/");
                    let without_ext: String;

                    // Find the file without the extension
                    if relative_file.ends_with(".json") {
                        // Json file: remove the .json
                        without_ext = relative_file[..relative_file.len() - 5].to_string();
                    } else if relative_file.ends_with("/init.lua") {
                        without_ext = relative_file[..relative_file.len() - 9].to_string();
                    } else if relative_file.ends_with(".lua") {
                        // Lua file: remove the .lua
                        without_ext = relative_file[..relative_file.len() - 4].to_string();
                    } else {
                        // Not a lua file, skip
                        continue;
                    }

                    // Mark the file as changed
                    require_visitor.mark_file_change(&without_ext);
                }

                if marked_file_count > 0 {
                    make_bundle(&mut require_visitor, &config);
                }
            }
        }
        "build" => {}
        _ => console::log_error(&"Invalid action".red()),
    };
}
