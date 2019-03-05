/// configure.rs
use super::*;
use std::fs::File;
use std::io::{self, Write, BufReader, BufWriter};

#[derive(Serialize,Deserialize,Debug)]
pub struct Configs {
    pub webhook_url: String,
    pub debug_mode: bool
}

#[derive(Serialize,Deserialize,Debug)]
struct ConfigFile {
    webhook_url: String
}

pub fn configure(configs: &Configs) {
    let current_url = configs.webhook_url.to_string();
    let mut webhook_url = "".to_string();
    while let Err(_) = validate_webhook_url(&webhook_url) {
        print!("Please input Slack Webhook URL [{}]> ", current_url);
        io::stdout().flush().unwrap();
        webhook_url = read_line().unwrap();
    }

    let new_configs = Configs {
        webhook_url: webhook_url,
        debug_mode: configs.debug_mode
    };
    save_config_file(&get_config_path(), &new_configs).unwrap();
    println!("Configs: {:?}", &new_configs);
}

pub fn get_configs(is_debug_mode: bool) -> Configs {
    match load_config_file(&get_config_path()) {
        Ok(c) => {
            Configs {
                webhook_url: match &c.webhook_url {
                    url if url.is_empty() =>
                        std::env::var("SLACK_WEBHOOK_URL").unwrap_or("".to_string()),
                    url => url.to_string()
                },
                debug_mode: is_debug_mode
            }
        },
        Err(e) => {
            if is_debug_mode {
                println!("Failed to load config file. Causes: {}", e);
            }
            Configs {
                webhook_url: std::env::var("SLACK_WEBHOOK_URL")
                    .unwrap_or_else(|_| {
                        let err = Error::Argv("SLACK_WEBHOOK_URL is not set.".to_string());
                        err.exit();
                    }),
                debug_mode: is_debug_mode
            }
        }
    }
}

fn get_config_path() -> String {
    match std::env::var("HOME") {
        Ok(home) => format!("{}{}", home, DEFAULT_CONFIG_PATH).to_string(),
        Err(_) => DEFAULT_CONFIG_PATH.to_string(),
    }
}

fn load_config_file(path: &str) -> Result<ConfigFile, String> {
    File::open(path)
        .map_err(|e| e.to_string())
        .and_then(|file|
            serde_json::de::from_reader(BufReader::new(file))
                .map_err(|e| e.to_string())
                .and_then(|c| Ok(c))
        )
}

fn save_config_file(path: &str, configs: &Configs) -> Result<(), String> {
    File::create(path)
        .map_err(|e| e.to_string())
        .and_then(|file|
            serde_json::ser::to_writer_pretty(BufWriter::new(file), configs)
                .map_err(|e| e.to_string())
                .and_then(|_| Ok(()))
        )
}

fn read_line() -> Result<String, Error> {
    let mut buffer = String::new();
    let _ = io::stdin().read_line(&mut buffer)
        .unwrap_or_else(|_| panic!("Failed to read stdin".to_string()));
    Ok(buffer.trim().to_string())
}
