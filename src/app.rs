use std::{
    collections::VecDeque,
    sync::{Arc, Mutex, RwLock, atomic::AtomicUsize},
    time::Duration,
};

use chrono::{NaiveDate, NaiveDateTime, NaiveTime, TimeZone};
use chrono_tz::Europe::Moscow;
use teloxide::{
    Bot,
    macros::BotCommands,
    prelude::Dispatcher,
    types::{ChatId, MessageId},
};
use tower::limit::ConcurrencyLimitLayer;

use crate::{
    opts::{State, load_config},
    schema::schema,
};

/// Possible bot commands
#[derive(BotCommands, Debug, Clone, Hash, Eq, PartialEq, PartialOrd, Ord, Default)]
#[command(rename_rule = "snake_case")]
#[non_exhaustive]
pub enum Command {
    #[default]
    Help,
    AddAdmin(String),
    RemoveAdmin(String),
    SetMemeCounter(usize),
    SetMemeLimit(usize),
    MemeLimit,
    MemeCounter,
    SubscribeForwards,
    ForwardSubscribers,
    Admins,
    Config,
}

/// Stores message id and id of the chat it was sent to
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct MsgWrapper {
    pub msg_id: MessageId,
    pub chat_id: ChatId,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, PartialOrd, Ord, Default)]
pub struct App {
    meme_limit: usize,
    connect_timeout: Duration,
    timeout: Duration,
    concurrent_connections: usize,
}

impl App {
    pub fn new(
        meme_limit: usize,
        connect_timeout: u64,
        timeout: u64,
        concurrent_connections: usize,
    ) -> Self {
        Self {
            meme_limit,
            connect_timeout: Duration::from_secs(connect_timeout),
            timeout: Duration::from_secs(timeout),
            concurrent_connections,
        }
    }

    pub async fn start(self) {
        let client = reqwest::Client::builder()
            .connect_timeout(self.connect_timeout)
            .timeout(self.timeout)
            .tcp_nodelay(true)
            .connector_layer(ConcurrencyLimitLayer::new(self.concurrent_connections))
            .build()
            .expect("client creation failed");

        let bot = Bot::from_env_with_client(client);

        let count_messages = Arc::new(AtomicUsize::new(0));

        let naive = NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2025, 4, 21).unwrap(),
            NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
        );
        let last_count_refresh = Moscow.from_local_datetime(&naive).unwrap();
        let last_count_refresh = Arc::new(Mutex::new(last_count_refresh));

        let state = Arc::new(RwLock::new(load_config().unwrap_or(State {
            meme_limit: self.meme_limit,
            ..Default::default()
        })));

        let spam_queue = Arc::new(Mutex::new(VecDeque::<MsgWrapper>::new()));

        Dispatcher::builder(bot, schema())
            .dependencies(dptree::deps![
                Arc::clone(&last_count_refresh),
                Arc::clone(&count_messages),
                Arc::clone(&state),
                Arc::clone(&spam_queue)
            ])
            .enable_ctrlc_handler()
            .build()
            .dispatch()
            .await;
    }
}
