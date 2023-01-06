use crate::build::BuildVisitor;
use crate::config::Config;
use crate::parser::RequireVisitor;
use colorize::AnsiColor;
use full_moon::visitors::VisitorMut;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use notify_debouncer_mini::new_debouncer;
use serde::{Deserialize, Serialize};
use simple_websockets::{Event, Message, Responder};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::{
    collections::HashMap,
    env, fs, io,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::console;

#[derive(Serialize, Deserialize)]
struct SourceMaps {
    pub files: Vec<String>,
    pub sources: Vec<usize>,
}

#[derive(Serialize, Deserialize, Debug)]
struct ErrorLog {
    pub message_lines: Vec<usize>,
    pub stack_trace_lines: Vec<usize>,
    pub message_content: String,
}

fn make_bundle(parser: &mut RequireVisitor, config: &Config) {
    // If the output directory does not exist, create it
    if !Path::new(&config.out_dir).exists() {
        fs::create_dir(&config.out_dir).unwrap();
    }

    let start_time = SystemTime::now();

    // Build the file project
    let (bundle_result, source_maps, imports) = match parser.generate_bundle(true) {
        Ok(bundle) => bundle,
        Err(err) => {
            console::log_error(&format!("Problem generating bundle: {}", err));
            return;
        }
    };

    // Write the bundle to the output file
    match fs::write(
        &(config.out_dir.to_owned() + "/bundle.dev.lua"),
        &bundle_result,
    ) {
        Ok(_) => (),
        Err(err) => {
            console::log_error(&format!("Problem writing bundle: {}", err));
            return;
        }
    };

    // Write the source map too
    let src_map = SourceMaps {
        files: imports,
        sources: source_maps,
    };

    let src_map_json = serde_json::to_string(&src_map).unwrap();
    match fs::write(
        &(config.out_dir.to_owned() + "/bundle.dev.lua.map"),
        &src_map_json,
    ) {
        Ok(_) => (),
        Err(err) => {
            console::log_error(&format!("Problem writing source map: {}", err));
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

fn map_to_source(line: usize, config: &Config) -> Option<(String, usize)> {
    let source_map =
        fs::read_to_string(&(config.out_dir.to_owned() + "/bundle.dev.lua.map")).unwrap();
    let source_map: SourceMaps = serde_json::from_str(&source_map).unwrap();

    // Go through the line, find if the current one is larger
    for (i, &cur_line) in source_map.sources.iter().enumerate() {
        if cur_line > line {
            let file = source_map.files.get(i).unwrap();
            let real_line = cur_line - line + 1;

            return Some((file.to_string(), real_line));
        }
    }

    None
}

pub fn run_server(config: Config) {
    let config_2 = config.clone();

    std::thread::spawn(move || {
        std::thread::scope(|f| {
            let clients = Arc::new(Mutex::new(HashMap::<u64, Responder>::new()));
            let clients_clone = clients.clone();

            // Create a new thread for the websocket server
            f.spawn(move || {
                let event_hub =
                    simple_websockets::launch(1338).expect("failed to listen on port 1338");
                console::log("Server started on port 1338!");

                loop {
                    match event_hub.poll_event() {
                        Event::Connect(client_id, responder) => {
                            let mut clients_map = clients_clone.lock().unwrap();
                            clients_map.insert(client_id, responder);
                        }
                        Event::Disconnect(client_id) => {
                            let mut clients_map = clients_clone.lock().unwrap();
                            clients_map.remove(&client_id);
                        }
                        Event::Message(client_id, message) => {
                            if let Message::Text(text) = message {
                                // Get the message
                                let message_json = serde_json::from_str::<Vec<String>>(&text);
                                if let Err(e) = message_json {
                                    console::log_error(
                                        &format!("Error parsing message: {}", e).red(),
                                    );
                                    continue;
                                }

                                // Get the message data
                                let message_vec = message_json.unwrap();
                                let message_type = message_vec.get(0).unwrap();

                                match message_type.as_str() {
                                    "connected" => {
                                        let client_name = message_vec.get(1).unwrap();

                                        // Log the client name
                                        console::log(&format!(
                                            "Client '{}' connected! ({})",
                                            client_name.clone().green(),
                                            client_id
                                        ));
                                    }
                                    "error" => {
                                        let error_data: ErrorLog =
                                            serde_json::from_str(&message_vec.get(1).unwrap())
                                                .unwrap();

                                        // Format the header
                                        let mut header_lines: Vec<String> = Vec::new();

                                        // Format the header, the error
                                        for line in error_data.message_lines {
                                            let (file, rel_line) = map_to_source(line, &config_2)
                                                .unwrap_or_else(|| ("Unknown".to_string(), 0));

                                            header_lines
                                                .push(format!("{}:{}", file, rel_line).cyan());
                                        }

                                        let mut header_lines = header_lines.join(": ") + ": ";
                                        if (header_lines.len() as i32) < 0 {
                                            header_lines = "".to_string();
                                        }

                                        // Get the header format
                                        let error_header =
                                            header_lines + &error_data.message_content.red();

                                        // Get the rest of the message
                                        let mut display_lines: Vec<String> =
                                            vec![error_header, "\tStack Begin".to_string()];

                                        for line in error_data.stack_trace_lines {
                                            let (file, rel_line) = map_to_source(line, &config_2)
                                                .unwrap_or_else(|| ("Unknown".to_string(), 0));

                                            display_lines.push(
                                                format!("\tFile '{}:{}'", file, rel_line).cyan(),
                                            );
                                        }

                                        display_lines.push("\tStack End".to_string());

                                        // Join the display lines into a string
                                        let display_lines = display_lines.join("\n");
                                        console::log(&format!("Runtime Error:\n{}", display_lines));
                                    }
                                    // TODO: Error tracking and source mapping
                                    _ => (),
                                };
                            }
                        }
                    }
                }
            });

            // Create a thread for the CLI
            f.spawn(move || {
                loop {
                    let mut input = String::new();

                    io::stdin()
                        .read_line(&mut input)
                        .expect("Failed to read line");

                    let clients = clients.lock().unwrap();
                    match input.trim().to_lowercase().as_str() {
                        "e" => {
                            console::log(&format!(
                                "Executing bundle for {} clients...",
                                clients.len()
                            ));

                            // Read the bundle
                            let bundle =
                                fs::read_to_string(config.out_dir.to_owned() + "/bundle.dev.lua")
                                    .unwrap();

                            let send_message =
                                serde_json::to_string(&vec![String::from("exec"), bundle]).unwrap();

                            for (_, responder) in clients.iter() {
                                let new_message = Message::Text(send_message.clone());
                                responder.send(new_message);
                            }
                        }
                        "exit" => std::process::exit(0),
                        _ => console::log(&format!("Invalid command '{}'", input.trim())),
                    };
                }
            });
        });
    });
}

pub fn run_bundler(config: Config) {
    // Create the parser
    let mut require_visitor = RequireVisitor::new(&config.src_dir, &config.entry_file);
    make_bundle(&mut require_visitor, &config);

    // Create the bundler
    let (tx, rx) = std::sync::mpsc::channel();

    let mut debouncer = new_debouncer(Duration::from_millis(500), None, tx).unwrap();

    debouncer
        .watcher()
        .watch(Path::new(&config.src_dir), RecursiveMode::Recursive)
        .unwrap();

    for e in rx {
        let e = e.unwrap();

        // Debounced event, go through each file
        let mut marked_file_count = 0;
        for event in &e {
            // Make sure it's Any and not AnyContinuous
            if let notify_debouncer_mini::DebouncedEventKind::Any = event.kind {
                let file_name = event.path.to_str().unwrap().to_string();

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
                console::log(&format!("File '{}' changed!", without_ext))
            }
        }

        if marked_file_count > 0 {
            make_bundle(&mut require_visitor, &config);
        }
    }
}

pub fn build_project(config: Config) {
    let mut require_visitor = RequireVisitor::new(&config.src_dir, &config.entry_file);

    let (bundle_result, source_maps, imports) = match require_visitor.generate_bundle(true) {
        Ok(bundle) => bundle,
        Err(err) => {
            console::log_error(&format!("Problem generating bundle: {}", err));
            return;
        }
    };

    // Create an AST from the bundled result
    let ast = full_moon::parse(&bundle_result).unwrap();
    let built_ast = BuildVisitor {}.visit_ast(ast);
    let built_result = full_moon::print(&built_ast);

    // Write to the file
    fs::write(
        &(config.out_dir.to_owned() + "/bundle.build.lua"),
        &built_result,
    )
    .unwrap();
}
