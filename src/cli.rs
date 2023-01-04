use std::{
    collections::HashMap,
    fs, io,
    sync::{Arc, Mutex},
};

use colorize::AnsiColor;
use simple_websockets::{Event, Message, Responder};

use crate::{config, console};

pub fn run() {
    std::thread::spawn(|| {
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
                                    _ => (),
                                };
                            }
                        }
                    }
                }
            });

            // Create a thread for the CLI
            f.spawn(move || {
                let config = config::get_config().unwrap();

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
