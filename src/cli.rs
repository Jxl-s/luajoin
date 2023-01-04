use crate::config::Config;
use crate::parser::RequireVisitor;
use colorize::AnsiColor;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Serialize, Deserialize};
use simple_websockets::{Event, Message, Responder};
use std::sync::{Arc, Mutex};
use std::{
    collections::HashMap,
    env, fs, io,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::console;

#[derive(Serialize, Deserialize)]
struct SourceMaps {
    files: Vec<String>,
    sources: Vec<usize>,
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
    match fs::write(&(config.out_dir.to_owned() + "/bundle.lua"), &bundle_result) {
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
    match fs::write(&(config.out_dir.to_owned() + "/bundle.lua.map"), &src_map_json) {
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

pub fn run_server(config: Config) {
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
                                fs::read_to_string(config.out_dir.to_owned() + "/bundle.lua")
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