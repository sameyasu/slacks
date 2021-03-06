#[macro_use]
extern crate serde_derive;
extern crate docopt;
extern crate regex;
extern crate reqwest;
extern crate serde;
extern crate serde_json;

mod config;

use config::Configs;
use docopt::{ArgvMap, Docopt, Error};
use regex::Regex;
use std::io::{self, Read};
use std::time::Duration;

const DEFAULT_CONFIG_PATH: &str = "/.config/slacks.json";
const DEFAULT_USERNAME: &str = "slacks";
const DEFAULT_ICON_EMOJI: &str = ":slack:";
const DEFAULT_CHANNEL: &str = "#general";
const TIMEOUT_IN_SEC: u64 = 10;

const USAGE: &str = "
Usage:
    slacks [-u <username>] [-i <icon_emoji>] [-c <channel>] [--debug] -
    slacks [-u <username>] [-i <icon_emoji>] [-c <channel>] [--debug] <message>
    slacks --configure [--debug]
    slacks -h | --help
    slacks --version

Options:
    -                   Read message text via STDIN.
    -u <username>       Set username. (default: slacks)
    -i <icon_emoji>     Set icon emoji. (default: :slack:)
    -c <channel>        Set posting channel. (default: #general)
    --configure         Do configuration for your Slack.
    --debug             Show debug messages.
    -h, --help          Show this message.
    --version           Show version.

Environment Variables:
    SLACK_WEBHOOK_URL   Incoming Webhook URL. (deprecated)

";

#[derive(Serialize, Deserialize, Debug)]
struct Payload {
    channel: String,
    username: String,
    icon_emoji: String,
    text: String,
}

trait SlacksArgs {
    fn is_help_command(&self) -> bool;
    fn is_version_command(&self) -> bool;
    fn is_configure_command(&self) -> bool;
    fn is_debug_mode(&self) -> bool;
}

impl SlacksArgs for ArgvMap {
    fn is_help_command(&self) -> bool {
        self.get_bool("-h") || self.get_bool("--help")
    }
    fn is_version_command(&self) -> bool {
        self.get_bool("--version")
    }
    fn is_configure_command(&self) -> bool {
        self.get_bool("--configure")
    }
    fn is_debug_mode(&self) -> bool {
        self.get_bool("--debug")
    }
}

fn main() {
    let args = Docopt::new(USAGE)
        .and_then(|d| d.parse())
        .unwrap_or_else(|e| e.exit());

    let result = match &args {
        args if args.is_help_command() => Err(Error::Help),
        args if args.is_version_command() => Err(Error::Usage(format!(
            "{} v{}",
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION")
        ))),
        args if args.is_configure_command() => config::configure(args.is_debug_mode()),
        args => {
            let conf = config::get_configs(args.is_debug_mode());
            messaging(&args, &conf)
        }
    };

    result.unwrap_or_else(|e| e.exit())
}

fn messaging(args: &ArgvMap, conf: &Configs) -> Result<(), Error> {
    if conf.debug_mode {
        eprintln!("Configs: {:?}", conf);
        eprintln!("Args: {:?}", args);
    }

    validate_webhook_url(&conf.webhook_url)?;

    let payload = Payload {
        channel: get_channel(args, conf)?,
        username: get_username(args, conf)?,
        icon_emoji: get_icon_emoji(args, conf)?,
        text: get_message(args)?,
    };
    if conf.debug_mode {
        eprintln!("Payload: {:?}", payload);
    }

    let json = serde_json::to_string(&payload).unwrap();
    if conf.debug_mode {
        eprintln!("JSON: {}", &json);
    }

    let resp = post_message(conf.webhook_url.as_ref().unwrap(), &json)?;
    if conf.debug_mode {
        eprintln!("Url: {:?}", resp.url().as_str());
        eprintln!("Status: {:?}", resp.status());
    }

    Ok(())
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
                Err(Error::Argv(format!(
                    "Failed to post to Slack. StatusCode: {}",
                    res.status()
                )))
            }
        }
        Err(err) => Err(Error::Deserialize(format!(
            "Failed to post to Slack. Error: {}",
            err
        ))),
    }
}

fn get_username(args: &ArgvMap, configs: &Configs) -> Result<String, Error> {
    let username = Some(match args.get_str("-u").trim() {
        u if u.is_empty() => configs.username.as_ref().unwrap().to_string(),
        u => u.into(),
    });
    validate_username(&username)?;
    Ok(username.unwrap())
}

fn get_icon_emoji(args: &ArgvMap, configs: &Configs) -> Result<String, Error> {
    let icon_emoji = Some(match args.get_str("-i").trim() {
        i if i.is_empty() => configs.icon_emoji.as_ref().unwrap().to_string(),
        i => i.into(),
    });
    validate_icon_emoji(&icon_emoji)?;
    Ok(icon_emoji.unwrap())
}

fn get_channel(args: &ArgvMap, configs: &Configs) -> Result<String, Error> {
    let channel = Some(match args.get_str("-c").trim() {
        c if c.is_empty() => configs.channel.as_ref().unwrap().to_string(),
        c => c.into(),
    });
    validate_channel(&channel)?;
    Ok(channel.unwrap())
}

fn get_message(args: &ArgvMap) -> Result<String, Error> {
    if args.get_bool("-") {
        // read from STDIN
        let mut buffer = String::new();
        match io::stdin().read_to_string(&mut buffer) {
            Ok(_) => Ok(buffer),
            Err(err) => Err(Error::Argv(format!(
                "Failed to read from STDIN. Error:{:?}",
                err
            ))),
        }
    } else {
        match args.get_str("<message>") {
            msg if msg.is_empty() => Err(Error::Usage("Empty message".into())),
            msg => Ok(msg.into()),
        }
    }
}

fn validate_webhook_url(url: &Option<String>) -> Result<(), Error> {
    let regexp = Regex::new(r"\Ahttps://hooks.slack.com/([a-zA-Z0-9]+/?){1,}\z").unwrap();
    match url {
        None => Err(Error::Argv("webhook_url is not set.".into())),
        Some(u) if regexp.is_match(u) => Ok(()),
        _ => Err(Error::Argv("webhook_url is invalid format.".into())),
    }
}

fn validate_channel(channel: &Option<String>) -> Result<(), Error> {
    match channel {
        None => Err(Error::Argv("channel is not set".into())),
        Some(channel) if channel.is_empty() => Err(Error::Argv("channel is empty".into())),
        Some(channel) if channel.len() > 20 => Err(Error::Argv("channel is too long".into())),
        Some(_) => Ok(()),
    }
}

fn validate_username(username: &Option<String>) -> Result<(), Error> {
    match username {
        None => Err(Error::Argv("username is not set".into())),
        Some(uname) if uname.is_empty() => Err(Error::Argv("username is empty".into())),
        Some(uname) if uname.len() > 20 => Err(Error::Argv("username is too long".into())),
        Some(_) => Ok(()),
    }
}

fn validate_icon_emoji(icon_emoji: &Option<String>) -> Result<(), Error> {
    let regexp = Regex::new(r"\A:[a-z0-9\-_\+]+:\z").unwrap();
    match icon_emoji {
        None => Err(Error::Argv("icon_emoji is not set.".into())),
        Some(icon) if icon.is_empty() => Err(Error::Argv("icon_emoji is empty.".into())),
        Some(icon) if regexp.is_match(icon) => Ok(()),
        _ => Err(Error::Argv(
            "icon_emoji is invalid format. (e.g. :robot_face:)".into(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    pub fn parse_argv(argv: Vec<&str>) -> Result<ArgvMap, Error> {
        let v = argv.into_iter();
        Docopt::new(USAGE).and_then(|d| d.argv(v).parse())
    }
}

#[cfg(test)]
mod get_message_tests {
    use super::*;

    #[test]
    #[should_panic(expected = "WithProgramUsage")]
    fn no_args() {
        let argv = vec!["slacks"];
        let _args = tests::parse_argv(argv).unwrap();
    }

    #[test]
    #[should_panic(expected = "WithProgramUsage")]
    fn not_specified_message() {
        let argv = vec!["slacks", "-c", "#test-channel"];
        let _args = tests::parse_argv(argv).unwrap();
    }

    #[test]
    #[should_panic(expected = "Empty message")]
    fn empty() {
        let argv = vec!["slacks", "-c", "#test-channel", ""];
        let args = tests::parse_argv(argv).unwrap();
        get_message(&args).unwrap();
    }

    #[test]
    fn ok() {
        let argv = vec!["slacks", "this is a test"];
        let args = tests::parse_argv(argv).unwrap();
        assert_eq!("this is a test", get_message(&args).unwrap());
    }

    #[test]
    #[ignore]
    // echo -n "this is a test from stdin" | cargo test -- --ignored
    fn read_from_stdin() {
        let argv = vec!["slacks", "-"];
        let args = tests::parse_argv(argv).unwrap();
        assert_eq!("this is a test from stdin", get_message(&args).unwrap());
    }
}

#[cfg(test)]
mod validate_webhook_url_tests {
    use super::*;

    #[test]
    #[should_panic(expected = "webhook_url is not set.")]
    fn none() {
        let _res = validate_webhook_url(&None).unwrap();
    }

    #[test]
    #[should_panic(expected = "webhook_url is invalid format.")]
    fn empty_str() {
        let empty = Some("".into());
        let _res = validate_webhook_url(&empty).unwrap();
    }

    #[test]
    #[should_panic(expected = "webhook_url is invalid format.")]
    fn invalid_url() {
        let invalid_url = Some("https://this.is.an.invalid.url/".into());
        let _res = validate_webhook_url(&invalid_url).unwrap();
    }

    #[test]
    fn valid_url() {
        let valid_url = Some("https://hooks.slack.com/TEST/valid".into());
        let res = validate_webhook_url(&valid_url);
        assert!(res.is_ok());
    }
}

#[cfg(test)]
mod validate_channel_tests {
    use super::*;

    #[test]
    #[should_panic(expected = "channel is not set")]
    fn none() {
        let _res = validate_channel(&None).unwrap();
    }

    #[test]
    #[should_panic(expected = "channel is empty")]
    fn empty() {
        let empty = Some("".into());
        let _res = validate_channel(&empty).unwrap();
    }

    #[test]
    #[should_panic(expected = "channel is too long")]
    fn over_20chars() {
        let long = Some("#12345678901234567890".into());
        let _res = validate_channel(&long).unwrap();
    }

    #[test]
    fn valid_public_channel() {
        let valid = Some("#public-channel".into());
        let res = validate_channel(&valid);
        assert!(res.is_ok());
    }

    #[test]
    fn valid_private_channel() {
        let valid = Some("private-channel".into());
        let res = validate_channel(&valid);
        assert!(res.is_ok());
    }

    #[test]
    fn valid_direct_message() {
        let valid = Some("@you".into());
        let res = validate_channel(&valid);
        assert!(res.is_ok());
    }
}

#[cfg(test)]
mod validate_username_tests {
    use super::*;

    #[test]
    #[should_panic(expected = "username is not set")]
    fn none() {
        let _res = validate_username(&None).unwrap();
    }

    #[test]
    #[should_panic(expected = "username is empty")]
    fn empty_str() {
        let empty = Some("".into());
        let _res = validate_username(&empty).unwrap();
    }

    #[test]
    #[should_panic(expected = "username is too long")]
    fn over_20chars() {
        let long = Some("012345678901234567890".into());
        let _res = validate_username(&long).unwrap();
    }

    #[test]
    fn valid() {
        let valid = Some("test-user".into());
        let res = validate_username(&valid);
        assert!(res.is_ok());
    }
}

#[cfg(test)]
mod validate_icon_emoji_tests {
    use super::*;

    #[test]
    #[should_panic(expected = "icon_emoji is not set.")]
    fn none() {
        let _res = validate_icon_emoji(&None).unwrap();
    }

    #[test]
    #[should_panic(expected = "icon_emoji is invalid format")]
    fn over_20chars() {
        let invalid = Some("robot_face".into());
        let _res = validate_icon_emoji(&invalid).unwrap();
    }

    #[test]
    fn valid_icon_emoji_ok_hand() {
        let valid = Some(":ok_hand:".into());
        let res = validate_icon_emoji(&valid);
        assert!(res.is_ok());
    }

    #[test]
    fn set_icon_emoji_plus1() {
        let valid = Some(":+1:".into());
        let res = validate_icon_emoji(&valid);
        assert!(res.is_ok());
    }
}
