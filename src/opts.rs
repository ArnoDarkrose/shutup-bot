use std::collections::HashSet;

use clap::Parser;
use serde::{Deserialize, Serialize};
use teloxide::types::{ChatId, UserId};
use tokio::io::{AsyncReadExt, BufReader};
use tracing_journald::Layer;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

use crate::INITIAL_ADMIN;

#[derive(Parser, Debug)]
/// Bot that allows to shutup a meme spammer. Logs are written using journald
pub struct Cli {
    #[arg(long, env)]
    /// Telegram username of the target person
    pub shutup_target: String,

    #[arg(long, env)]
    /// Username of the initial admin of the bot. Other admins can be added by interacting with the server when it's up
    pub initial_admin: String,

    #[arg(long, default_value = "2", env)]
    /// Number of memes per day that the person can send without limitations
    pub meme_limit: usize,

    #[arg(long, short, default_value = "info", env)]
    /// Level of logs to write, supported values are error, warn, info, debug and tracing
    pub log_level: String,

    #[arg(long, env, default_value = "5")]
    /// Time (in seconds) of request connection timeout
    pub connect_timeout: u64,

    #[arg(long, env, default_value = "17")]
    /// Time (in seconds) of full request timeout
    pub timeout: u64,

    #[arg(long, env, default_value = "100")]
    pub concurrent_connections: usize,
}

impl Cli {
    pub fn setup_logs(&self) {
        let journald_layer = Layer::new().expect("failed to create journald tracing layer");

        match self.log_level.as_str() {
            "error" | "warn" | "info" | "debug" | "trace" | "ERROR" | "WARN" | "INFO" | "DEBUG"
            | "TRACE" => {}
            _ => {
                panic!("incorrect log_level value")
            }
        }

        let filter = EnvFilter::new(&self.log_level);

        tracing_subscriber::registry()
            .with(journald_layer)
            .with(filter)
            .init();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub admins: HashSet<String>,
    pub meme_limit: usize,
    pub forward_subscribers: HashSet<UserId>,
    pub queue_subscribers: HashSet<ChatId>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            admins: {
                let mut res = HashSet::new();
                res.insert(
                    INITIAL_ADMIN
                        .get()
                        .expect(
                            "this is static is only being written to once in the beginning of main",
                        )
                        .to_owned(),
                );
                res
            },
            meme_limit: 3,
            forward_subscribers: HashSet::new(),
            queue_subscribers: HashSet::new(),
        }
    }
}

pub async fn load_config() -> tokio::io::Result<Config> {
    let path = expanduser::expanduser("~/.config/shutup-bot/config.json")?;
    let fd = tokio::fs::File::open(path).await?;
    let mut fd = BufReader::new(fd);

    let mut buf = String::new();
    fd.read_to_string(&mut buf).await?;

    Ok(serde_json::from_str(&buf)?)
}
