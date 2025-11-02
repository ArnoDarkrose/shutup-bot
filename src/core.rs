use std::sync::{Arc, Mutex, RwLock, atomic::AtomicUsize};

use chrono::{NaiveDate, NaiveDateTime, NaiveTime, TimeZone};
use chrono_tz::Europe::Moscow;
use teloxide::{Bot, macros::BotCommands, prelude::Dispatcher};

use crate::{
    opts::{State, load_config},
    schema::schema,
};

#[derive(BotCommands, Clone)]
#[command(rename_rule = "snake_case")]
#[non_exhaustive]
pub enum Command {
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
    Cbz(String),
}

pub async fn start(meme_limit: usize) {
    let bot = Bot::from_env();

    let count_messages = Arc::new(AtomicUsize::new(0));

    let naive = NaiveDateTime::new(
        NaiveDate::from_ymd_opt(2025, 4, 21).unwrap(),
        NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
    );
    let last_count_refresh = Moscow.from_local_datetime(&naive).unwrap();
    let last_count_refresh = Arc::new(Mutex::new(last_count_refresh));

    let state = Arc::new(RwLock::new(load_config().unwrap_or(State {
        meme_limit,
        ..Default::default()
    })));

    Dispatcher::builder(bot, schema())
        .dependencies(dptree::deps![
            Arc::clone(&last_count_refresh),
            Arc::clone(&count_messages),
            Arc::clone(&state)
        ])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}
