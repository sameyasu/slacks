[![Build Status](https://travis-ci.org/sameyasu/slacks.svg?branch=master)](https://travis-ci.org/sameyasu/slacks)

# `slacks`

The easiest way to post to Slack :D

## Usage

```
Usage:
    slacks [-u <username>] [-i <icon_emoji>] [-c <channel>] [--debug] -
    slacks [-u <username>] [-i <icon_emoji>] [-c <channel>] [--debug] <message>
    slacks -h | --help
    slacks --version

Options:
    -                   Read message text from STDIN.
    -u <username>       Set username. (default: slacks)
    -i <icon_emoji>     Set icon emoji. (default: :robot_face:)
    -c <channel>        Set posting channel. (default: #general)
    --debug             Show debug messages.
    -h, --help          Show this message.
    --version           Show version.

Environment Variables:
    SLACK_WEBHOOK_URL   Incoming Webhook URL. (required)
```
