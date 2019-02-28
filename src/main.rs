#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;
extern crate reqwest;
extern crate docopt;
extern crate regex;

use std::fs::File;
use std::io::{self, Read, BufReader};
use std::time::Duration;
use docopt::{Docopt, Error};
use regex::Regex;

const DEFAULT_CONFIG_PATH: &'static str = "/.config/slacks.json";
const DEFAULT_USERNAME: &'static str = "slacks";
const DEFAULT_ICON_EMOJI: &'static str = ":slack:";
const DEFAULT_CHANNEL: &'static str = "#general";
const TIMEOUT_IN_SEC: u64 = 10;

const USAGE: &'static str = "
Usage:
    slacks [-u <username>] [-i <icon_emoji>] [-c <channel>] [--debug] -
    slacks [-u <username>] [-i <icon_emoji>] [-c <channel>] [--debug] <message>
    slacks -h | --help
    slacks --version

Options:
    -                   Read message text from STDIN.
    -u <username>       Set username. (default: slacks)
    -i <icon_emoji>     Set icon emoji. (default: :slack:)
    -c <channel>        Set posting channel. (default: #general)
    --debug             Show debug messages.
    -h, --help          Show this message.
    --version           Show version.

Environment Variables:
    SLACK_WEBHOOK_URL   Incoming Webhook URL. (required)

";

#[derive(Serialize,Deserialize,Debug)]
struct Payload {
    channel: String,
    username: String,
    icon_emoji: String,
    text: String
}

#[derive(Serialize,Deserialize,Debug)]
struct DefaultConfig {
    webhook_url: String,
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

    let default = get_default_config();
    if is_debug_mode(&args) {
        println!("DefaultConfig: {:?}", default);
    }

    let webhook_url = match &default.webhook_url {
        url if url.is_empty() => get_webhook_url().unwrap_or_else(|e| e.exit()),
        url => url.to_string()
    };

    if is_debug_mode(&args) {
        println!("Args: {:?}", args);
    }

    let payload = Payload {
        channel: get_channel(&args).unwrap_or_else(|e| e.exit()),
        username: get_username(&args).unwrap_or_else(|e| e.exit()),
        icon_emoji: get_icon_emoji(&args).unwrap_or_else(|e| e.exit()),
        text: get_message(&args).unwrap_or_else(|e| e.exit())
    };
    if is_debug_mode(&args) {
        println!("Payload: {:?}", payload);
    }

    let json = serde_json::to_string(&payload).unwrap();
    if is_debug_mode(&args) {
        println!("JSON: {}", &json);
    }

    let resp = post_message(&webhook_url, &json).unwrap_or_else(|e| e.exit());
    if is_debug_mode(&args) {
        println!("Url: {:?}", resp.url().as_str());
        println!("Status: {:?}", resp.status());
    }
}

fn get_default_config() -> DefaultConfig {
    match read_config_file(get_config_path()) {
        Ok(c) => c,
        Err(_) => {
            DefaultConfig {
                webhook_url: "".to_string()
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

fn read_config_file(path: String) -> Result<DefaultConfig, io::Error> {
    let file = File::open(path)?;
    let buf_reader = BufReader::new(file);
    let setting = serde_json::de::from_reader(buf_reader)?;
    Ok(setting)
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
    let regexp = Regex::new(r"\A:[a-z0-9\-_\+]+:\z").unwrap();
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

fn is_debug_mode(args: &docopt::ArgvMap) -> bool {
    args.get_bool("--debug")
}

fn get_webhook_url() -> Result<String, Error> {
    let regexp = Regex::new(r"\Ahttps://hooks.slack.com/([a-zA-Z0-9]+/?){1,}\z").unwrap();
    match std::env::var("SLACK_WEBHOOK_URL") {
        Ok(url) => {
            match &url {
                url if regexp.is_match(url) => Ok(url.to_string()),
                _ => Err(Error::Argv("SLACK_WEBHOOK_URL is invalid URL.".to_string()))
            }
        },
        _ => Err(Error::Argv("SLACK_WEBHOOK_URL is not set.".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use docopt::ArgvMap;

    pub fn parse_argv(argv: Vec<&str>) -> Result<ArgvMap, Error> {
        let v = argv.into_iter();
        Docopt::new(USAGE).and_then(|d| d.argv(v).parse())
    }
}

#[cfg(test)]
mod get_username_tests {
    use super::*;

    #[test]
    fn default() {
        let argv = vec!["slacks", "this is a test"];
        let args = tests::parse_argv(argv).unwrap();
        assert_eq!(
            "slacks".to_string(),
            get_username(&args).unwrap()
        );
    }

    #[test]
    fn set_username() {
        let argv = vec!["slacks", "-u", "testuser", "this is a test"];
        let args = tests::parse_argv(argv).unwrap();
        assert_eq!(
            "testuser".to_string(),
            get_username(&args).unwrap()
        );
    }

    #[test]
    fn empty() {
        let argv = vec!["slacks", "-u", "", "this is a test"];
        let args = tests::parse_argv(argv).unwrap();
        assert_eq!(
            "slacks".to_string(),
            get_username(&args).unwrap()
        );
    }

    #[test]
    #[should_panic(expected="username is too long")]
    fn over_20chars() {
        let argv = vec!["slacks", "-u", "012345678901234567890", "this is a test"];
        let args = tests::parse_argv(argv).unwrap();
        get_username(&args).unwrap();
    }
}

#[cfg(test)]
mod get_channel_tests {
    use super::*;

    #[test]
    fn default() {
        let argv = vec!["slacks", "this is a test"];
        let args = tests::parse_argv(argv).unwrap();
        assert_eq!(
            "#general".to_string(),
            get_channel(&args).unwrap()
        );
    }

    #[test]
    fn set_channel() {
        let argv = vec!["slacks", "-c", "#public-channel", "this is a test"];
        let args = tests::parse_argv(argv).unwrap();
        assert_eq!(
            "#public-channel".to_string(),
            get_channel(&args).unwrap()
        );
    }

    #[test]
    fn empty() {
        let argv = vec!["slacks", "-c", "", "this is a test"];
        let args = tests::parse_argv(argv).unwrap();
        assert_eq!(
            "#general".to_string(),
            get_channel(&args).unwrap()
        );
    }

    #[test]
    #[should_panic(expected="channel is too long")]
    fn over_20chars() {
        let argv = vec!["slacks", "-c", "012345678901234567890", "this is a test"];
        let args = tests::parse_argv(argv).unwrap();
        get_channel(&args).unwrap();
    }
}

#[cfg(test)]
mod get_icon_emoji_tests {
    use super::*;

    #[test]
    fn default() {
        let argv = vec!["slacks", "this is a test"];
        let args = tests::parse_argv(argv).unwrap();
        assert_eq!(
            ":slack:".to_string(),
            get_icon_emoji(&args).unwrap()
        );
    }

    #[test]
    fn set_icon_emoji_ok_hand() {
        let argv = vec!["slacks", "-i", ":ok_hand:", "this is a test"];
        let args = tests::parse_argv(argv).unwrap();
        assert_eq!(
            ":ok_hand:".to_string(),
            get_icon_emoji(&args).unwrap()
        );
    }

    #[test]
    fn set_icon_emoji_plus1() {
        let argv = vec!["slacks", "-i", ":+1:", "this is a test"];
        let args = tests::parse_argv(argv).unwrap();
        assert_eq!(
            ":+1:".to_string(),
            get_icon_emoji(&args).unwrap()
        );
    }

    #[test]
    fn empty() {
        let argv = vec!["slacks", "-i", "", "this is a test"];
        let args = tests::parse_argv(argv).unwrap();
        assert_eq!(
            ":slack:".to_string(),
            get_icon_emoji(&args).unwrap()
        );
    }

    #[test]
    #[should_panic(expected="icon_emoji is invalid format.")]
    fn invalid_chars() {
        let argv = vec!["slacks", "-i", "robot_face", "this is a test"];
        let args = tests::parse_argv(argv).unwrap();
        get_icon_emoji(&args).unwrap();
    }
}

#[cfg(test)]
mod get_message_tests {
    use super::*;

    #[test]
    #[should_panic(expected="WithProgramUsage")]
    fn no_args() {
        let argv = vec!["slacks"];
        let _args = tests::parse_argv(argv).unwrap();
    }

    #[test]
    #[should_panic(expected="WithProgramUsage")]
    fn not_specified_message() {
        let argv = vec!["slacks", "-c", "#test-channel"];
        let _args = tests::parse_argv(argv).unwrap();
    }

    #[test]
    #[should_panic(expected="Empty message")]
    fn empty() {
        let argv = vec!["slacks", "-c", "#test-channel", ""];
        let args = tests::parse_argv(argv).unwrap();
        get_message(&args).unwrap();
    }

    #[test]
    fn ok() {
        let argv = vec!["slacks", "this is a test"];
        let args = tests::parse_argv(argv).unwrap();
        assert_eq!(
            "this is a test",
            get_message(&args).unwrap()
        );
    }

    #[test]
    #[ignore]
    // echo -n "this is a test from stdin" | cargo test -- --ignored
    fn read_from_stdin() {
        let argv = vec!["slacks", "-"];
        let args = tests::parse_argv(argv).unwrap();
        assert_eq!(
            "this is a test from stdin",
            get_message(&args).unwrap()
        );
    }
}

#[cfg(test)]
mod get_webhook_url_tests {
    use super::*;

    #[test]
    #[should_panic(expected="SLACK_WEBHOOK_URL is not set.")]
    fn not_set_env() {
        get_webhook_url().unwrap();
    }
}
