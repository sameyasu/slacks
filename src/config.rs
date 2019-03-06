/// configure.rs
use super::*;
use std::fs::File;
use std::io::{self, Write, BufReader, BufWriter};

#[derive(Serialize,Deserialize,Debug)]
pub struct Configs {
    pub webhook_url: String,
    pub debug_mode: bool
}

pub fn configure(debug: bool) {
    let configs = get_configs(debug)
        .unwrap_or(
            Configs {
                webhook_url: "".to_string(),
                debug_mode: debug
            }
        );

    let current_url = configs.webhook_url.to_string();
    let mut webhook_url = "".to_string();
    while let Err(_) = validate_webhook_url(&webhook_url) {
        print!("Please input Slack Webhook URL [{}]> ", current_url);
        io::stdout().flush().unwrap();
        webhook_url = read_line().unwrap();
    }

    let new_conf = Configs {
        webhook_url: webhook_url,
        debug_mode: false // allways false
    };
    save_config_file(&get_config_path(), &new_conf).unwrap();
    println!("Config: {:?}", &new_conf);
}

pub fn get_configs(is_debug_mode: bool) -> Result<Configs, Error> {
    match load_config_file(&get_config_path()) {
        Ok(c) => Ok(
            Configs {
                webhook_url: match &c.webhook_url {
                    url if url.is_empty() =>
                        std::env::var("SLACK_WEBHOOK_URL").unwrap_or("".to_string()),
                    url => url.to_string()
                },
                debug_mode: is_debug_mode
            }
        ),
        Err(e) => {
            if is_debug_mode {
                println!("Failed to load config file. Causes: {}", e);
            }
            match std::env::var("SLACK_WEBHOOK_URL") {
                Ok(url) => Ok(
                    Configs {
                        webhook_url: url,
                        debug_mode: is_debug_mode
                    }
                ),
                Err(_) => Err(
                    Error::Argv("SLACK_WEBHOOK_URL is not set.".to_string())
                )
            }
        }
    }
}

fn get_config_path() -> String {
    std::env::var("HOME")
        .map(|home| format!("{}{}", home, DEFAULT_CONFIG_PATH).to_string())
        .unwrap_or(DEFAULT_CONFIG_PATH.to_string())
}

fn load_config_file(path: &str) -> Result<Configs, String> {
    File::open(path)
        .map_err(|e| e.to_string())
        .and_then(|file|
            serde_json::de::from_reader(BufReader::new(file))
                .map_err(|e| e.to_string())
                .and_then(|c| Ok(c))
        )
}

fn save_config_file(path: &str, conf: &Configs) -> Result<(), String> {
    File::create(path)
        .map_err(|e| e.to_string())
        .and_then(|file|
            serde_json::ser::to_writer_pretty(BufWriter::new(file), conf)
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
