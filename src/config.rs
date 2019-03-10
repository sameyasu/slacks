/// config.rs
use super::*;
use std::fs::File;
use std::io::{self, Write, BufReader, BufWriter};

#[derive(Serialize,Deserialize,Debug,Clone)]
pub struct Configs {
    pub webhook_url: Option<String>,
    pub channel: Option<String>,
    pub username: Option<String>,
    pub icon_emoji: Option<String>,
    pub debug_mode: bool
}

impl Configs {
    fn load(&mut self, path: &str) -> Result<(), String> {
        File::open(path)
            .map_err(|e| e.to_string())
            .and_then(|file| {
                let c : Configs = serde_json::de::from_reader(BufReader::new(file))
                    .map_err(|e| e.to_string())
                    .unwrap();
                // load to self
                self.clone_from(&c);
                Ok(())
            })
    }

    fn save(&self, path: &str) -> Result<(), String> {
        File::create(path)
            .map_err(|e| e.to_string())
            .and_then(|file|
                serde_json::ser::to_writer_pretty(BufWriter::new(file), self)
                    .map_err(|e| e.to_string())
                    .and_then(|_| Ok(()))
            )
    }
}

pub fn configure(debug: bool) {
    let mut configs = Configs {
        webhook_url: None,
        channel: Some(DEFAULT_CHANNEL.to_string()),
        username: Some(DEFAULT_USERNAME.to_string()),
        icon_emoji: Some(DEFAULT_ICON_EMOJI.to_string()),
        debug_mode: debug
    };

    let config_path = get_config_path();
    let _ = configs.load(&config_path);

    let new_conf = Configs {
        webhook_url: configure_var(&configs.webhook_url, "Slack Webhook URL", validate_webhook_url),
        channel: configure_var(&configs.channel, "Default Channel", validate_channel),
        username: configure_var(&configs.username, "Default Username", validate_username),
        icon_emoji: configure_var(&configs.icon_emoji, "Default Icon Emoji", validate_icon_emoji),
        debug_mode: false // allways false
    };
    new_conf.save(&config_path).unwrap();
    if debug {
        println!("Saved: {:?}", &new_conf);
    }
    println!("Saved your configuration: {}", &config_path);
}

pub fn get_configs(is_debug_mode: bool) -> Configs {
    let mut configs = Configs {
        webhook_url: None,
        channel: Some(DEFAULT_CHANNEL.to_string()),
        username: Some(DEFAULT_USERNAME.to_string()),
        icon_emoji: Some(DEFAULT_ICON_EMOJI.to_string()),
        debug_mode: is_debug_mode
    };

    let _ = configs.load(&get_config_path())
        .map_err(|e| {
            if is_debug_mode {
                println!("Failed to load config file. Cause: {}", e);
            }
            // ignore error
        })
        .map(|()| {
            if is_debug_mode {
                println!("Loaded {:?}", configs);
            }
        });

    let _ = std::env::var("SLACK_WEBHOOK_URL")
        .map(|url| {
            // override: backward compatibility
            configs.webhook_url = Some(url);
        });

    configs
}

fn get_config_path() -> String {
    std::env::var("HOME")
        .map(|home| format!("{}{}", home, DEFAULT_CONFIG_PATH).to_string())
        .unwrap_or(DEFAULT_CONFIG_PATH.to_string())
}

fn read_line() -> Result<String, Error> {
    let mut buffer = String::new();
    let _ = io::stdin().read_line(&mut buffer)
        .unwrap_or_else(|_| panic!("Failed to read stdin".to_string()));
    Ok(buffer.trim().to_string())
}

fn configure_var<F>(var: &Option<String>, description: &str, validator: F) -> Option<String>
    where F: Fn(&Option<String>) -> Result<(), Error>
{
    let var_ref = var.as_ref();
    let mut inputted = None;
    while let Err(e) = validator(&inputted) {
        if inputted.is_some() {
            println!("{}", e);
        }
        print!(
            "{} [{}]: ",
            description,
            match var_ref {
                Some(u) => u,
                None => "None"
            }
        );
        io::stdout().flush().unwrap();
        inputted = match &read_line().unwrap() {
            url if url.is_empty() && var_ref.is_some() => Some(var_ref.unwrap().to_string()),
            url => Some(url.to_string())
        };
    }
    inputted
}
