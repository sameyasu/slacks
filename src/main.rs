#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;
extern crate reqwest;
extern crate docopt;
extern crate regex;

use std::io::{self, Read};
use std::time::Duration;
use docopt::{Docopt, Error};
use regex::Regex;

const DEFAULT_USERNAME: &'static str = "slacks";
const DEFAULT_ICON_EMOJI: &'static str = ":slack:";
const DEFAULT_CHANNEL: &'static str = "#general";
const TIMEOUT_IN_SEC: u64 = 10;

const USAGE: &'static str = "
Usage:
    slacks [-u <username>] [-i <icon_emoji>] [-c <channel>] [-v] -
    slacks [-u <username>] [-i <icon_emoji>] [-c <channel>] [-v] <message>
    slacks -h | --help
    slacks --version

Options:
    -h, --help          Show this message.
    -u <username>       User name (default: slacks)
    -i <icon_emoji>     Icon emoji (default: :robot_face:)
    -c <channel>        Channel name (default: #general)
    -v                  Verbose Mode
    --version           Show version.
    -                   Read from STDIN
";

#[derive(Serialize,Deserialize,Debug)]
struct Payload {
    channel: String,
    username: String,
    icon_emoji: String,
    text: String
}

fn main() {
    let args = Docopt::new(USAGE)
                    .and_then(|d| d.parse())
                    .unwrap_or_else(|e| e.exit());

    if args.get_bool("-h") || args.get_bool("--help") {
        let err = Error::Help;
        err.exit();
    }

    if args.get_bool("--version") {
        let err = Error::Usage(
            format!("{} v{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))
        );
        err.exit();
    }

    // TODO: panicにならないようにエラーメッセージ出力して終了させる
    let webhook_url = std::env::var("SLACK_WEBHOOK_URL").unwrap();

    if is_verbose_mode(&args) {
        println!("Args: {:?}", args);
    }

    let payload = Payload {
        channel: get_channel(&args).unwrap_or_else(|e| e.exit()),
        username: get_username(&args).unwrap_or_else(|e| e.exit()),
        icon_emoji: get_icon_emoji(&args).unwrap_or_else(|e| e.exit()),
        text: get_message(&args).unwrap_or_else(|e| e.exit())
    };
    if is_verbose_mode(&args) {
        println!("Payload: {:?}", payload);
    }

    let json = serde_json::to_string(&payload).unwrap();
    if is_verbose_mode(&args) {
        println!("JSON: {}", &json);
    }

    let resp = post_message(&webhook_url, &json).unwrap_or_else(|e| e.exit());
    if is_verbose_mode(&args) {
        println!("Url: {:?}", resp.url().as_str());
        println!("Status: {:?}", resp.status());
    }
}

fn post_message(url: &str, json: &str) -> Result<reqwest::Response, Error> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(TIMEOUT_IN_SEC))
        .build()
        .unwrap();

    match client.post(url).body(json.to_string()).send() {
        Ok(res) => {
            if res.status() == reqwest::StatusCode::OK {
                Ok(res)
            } else {
                Err(
                    Error::Argv(
                        format!("Failed to post to Slack. StatusCode: {}", res.status())
                    )
                )
            }
        },
        Err(err) => Err(
            Error::Deserialize(format!("Failed to post to Slack. Error: {}", err))
        )
    }
}

fn get_username(args: &docopt::ArgvMap) -> Result<String, Error> {
    match args.get_str("-u").trim() {
        uname if uname.is_empty() => Ok(DEFAULT_USERNAME.to_string()),
        uname if uname.len() > 20 => Err(Error::Argv("username is too long".to_string())),
        uname => Ok(uname.to_string())
    }
}

fn get_icon_emoji(args: &docopt::ArgvMap) -> Result<String, Error> {
    let regexp = Regex::new(r":[a-z0-9\-_]+:").unwrap();
    match args.get_str("-i").trim() {
        icon if icon.is_empty() => Ok(DEFAULT_ICON_EMOJI.to_string()),
        icon if regexp.is_match(icon) => Ok(icon.to_string()),
        _ => Err(Error::Argv("icon_emoji is invalid format. (e.g. :robot_face:)".to_string()))
    }
}

fn get_channel(args: &docopt::ArgvMap) -> Result<String, Error> {
    match args.get_str("-c").trim() {
        channel if channel.is_empty() => Ok(DEFAULT_CHANNEL.to_string()),
        channel if channel.len() > 20 => Err(Error::Argv("channel is too long".to_string())),
        channel => Ok(channel.to_string())
    }
}

fn get_message(args: &docopt::ArgvMap) -> Result<String, Error> {
    if args.get_bool("-") {
        // read from STDIN
        let mut buffer = String::new();
        match io::stdin().read_to_string(&mut buffer) {
            Ok(_) => Ok(buffer.to_string()),
            Err(err) => Err(
                Error::Argv(
                    format!("Failed to read from STDIN. Error:{:?}", err)
                )
            )
        }
    } else {
        match args.get_str("<message>") {
            msg if msg.is_empty() => Err(Error::Usage("Empty message".to_string())),
            msg => Ok(msg.to_string())
        }
    }
}

fn is_verbose_mode(args: &docopt::ArgvMap) -> bool {
    args.get_bool("-v")
}
