use std::{
    collections::HashSet,
    io::Read,
    sync::{Arc, RwLock},
};

use clap::Parser;
use serde::{Deserialize, Serialize};
use teloxide::types::{ChatId, UserId};
use tokio::io::AsyncWriteExt;
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
pub struct State {
    pub admins: HashSet<String>,
    pub meme_limit: usize,
    pub forward_subscribers: HashSet<UserId>,
    pub queue_subscribers: HashSet<ChatId>,
}

impl Default for State {
    fn default() -> Self {
        State {
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

pub async fn save_config(config: Arc<RwLock<State>>) -> tokio::io::Result<()> {
    let contents = serde_json::to_string(&*config.read().unwrap())?;

    let mut path = expanduser::expanduser("~/.config")?;

    tokio::fs::create_dir_all(&path).await?;

    path.push("shutup-bot.json");

    let mut fd = tokio::fs::File::create(path).await?;

    fd.write_all(contents.as_bytes()).await?;

    Ok(())
}

pub fn load_config() -> std::io::Result<State> {
    let path = expanduser::expanduser("~/.config/shutup-bot.json")?;
    let fd = std::fs::File::open(path)?;
    let mut fd = std::io::BufReader::new(fd);

    let mut buf = String::new();
    fd.read_to_string(&mut buf)?;

    Ok(serde_json::from_str::<State>(&buf)?)
}
