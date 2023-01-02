use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs::{self, File};

const PROJECT_FILE_NAME: &str = "project.luajoin.json";

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub entry_file: String,
    pub src_dir: String,
    pub out_dir: String,
}

pub fn create_config_file(src_dir: &str, out_dir: &str, entry: &str) -> Result<(), Box<dyn Error>> {
    // Create the JSON
    let config_file_content = Config {
        src_dir: src_dir.to_string(),
        out_dir: out_dir.to_string(),

        entry_file: entry.to_string(),
    };

    // Create the file
    let config_file = File::create(PROJECT_FILE_NAME)?;

    // Write the JSON
    serde_json::to_writer_pretty(config_file, &config_file_content)?;

    // Create the source and output directories
    fs::create_dir(&config_file_content.src_dir)?;
    fs::create_dir(&config_file_content.out_dir)?;

    // Write the entry file
    let entry_path = format!(
        "{}/{}.lua",
        &config_file_content.src_dir, &config_file_content.entry_file
    );

    fs::write(entry_path, "print(\"Hello, world!\")")?;
    Ok(())
}

pub fn get_config() -> Option<Config> {
    let file = match File::open(PROJECT_FILE_NAME) {
        Ok(res) => res,
        Err(_) => return None,
    };

    match serde_json::from_reader(file) {
        Ok(res) => res,
        Err(_) => None,
    }
}
