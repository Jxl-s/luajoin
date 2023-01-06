use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs::{self, File};

// Constants
const CONFIG_FILE_NAME: &str = ".luajoin.json";
const PROJ_FILE_NAME: &str = ".project.json";

// Files
const DEV_FILE_CONTENT: &str = include_str!("lua/.dev.lua");
const MAIN_FILE_CONTENT: &str = include_str!("lua/main.lua");

#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    pub entry_file: String,
    pub src_dir: String,
    pub out_dir: String,
}

pub fn create_config_file(src_dir: &str, out_dir: &str, entry: &str) -> Result<(), Box<dyn Error>> {
    // Create the JSON
    let config = Config {
        src_dir: src_dir.to_string(),
        out_dir: out_dir.to_string(),

        entry_file: entry.to_string(),
    };

    // Create the file
    let config_file = File::create(CONFIG_FILE_NAME)?;

    // Write the JSON
    serde_json::to_writer_pretty(config_file, &config)?;

    // Create the source and output directories
    fs::create_dir(&config.src_dir)?;
    fs::create_dir(&config.out_dir)?;

    // Write the entry file
    let entry_path = format!("{}/{}.lua", &config.src_dir, &config.entry_file);

    // Write the dev file
    let dev_path = format!("{}/.dev.lua", &config.src_dir);

    let gitignore_content = format!("/{}\n/{}/.dev.lua", &config.out_dir, &config.src_dir);

    fs::write(entry_path, MAIN_FILE_CONTENT)?;
    fs::write(dev_path, DEV_FILE_CONTENT)?;
    fs::write(".gitignore", &gitignore_content)?;
    fs::write(PROJ_FILE_NAME, format!("{{\"tree\":{{\"$path\":\"src\"}}}}"))?;

    Ok(())
}

pub fn get_config() -> Option<Config> {
    let file = match File::open(CONFIG_FILE_NAME) {
        Ok(res) => res,
        Err(_) => return None,
    };

    match serde_json::from_reader(file) {
        Ok(res) => res,
        Err(_) => None,
    }
}
