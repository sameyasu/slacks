/// configure.rs
use super::*;
use std::fs::File;
use std::io::{self, Write, BufReader, BufWriter};

#[derive(Serialize,Deserialize,Debug,Clone)]
pub struct Configs {
    pub webhook_url: Option<String>,
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
       debug_mode: debug
    };

    let _ = configs.load(&get_config_path());

    let current_url = configs.webhook_url;
    let mut webhook_url = None;
    while let Err(e) = validate_webhook_url(&webhook_url) {
        if webhook_url != None {
            println!("{}", e);
        }
        print!("Slack Webhook URL [{}]: ", match &current_url {
            Some(u) => u,
            _ => "None"
        });
        io::stdout().flush().unwrap();
        webhook_url = Some(read_line().unwrap());
    }

    let new_conf = Configs {
        webhook_url: webhook_url,
        debug_mode: false // allways false
    };
    new_conf.save(&get_config_path()).unwrap();
    println!("Saved: {:?}", &new_conf);
}

pub fn get_configs(is_debug_mode: bool) -> Result<Configs, Error> {
    let mut configs = Configs {
       webhook_url: None,
       debug_mode: is_debug_mode
    };

    let _ = configs.load(&get_config_path())
        .map_err(|e| {
            if is_debug_mode {
                println!("Failed to load config file. Cause: {}", e);
            }
        })
        .map(|()| {
            if is_debug_mode {
                println!("{:?}", configs);
            }
        });

    let _ = std::env::var("SLACK_WEBHOOK_URL")
        .map(|url| {
            // override: backward compatibility
            configs.webhook_url = Some(url);
        });

    Ok(configs)

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
