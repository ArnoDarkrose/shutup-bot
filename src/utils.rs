use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, RwLock};

use anyhow::bail;
use chrono::TimeZone;
use chrono::{Duration, Utc};
use chrono_tz::Europe::Moscow;
use teloxide::Bot;
use teloxide::prelude::{Request, Requester};
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::time::{Instant, sleep_until};
use tokio::{
    select,
    signal::unix::{SignalKind, signal},
};
use tracing::warn;

use crate::app::SpamQueue;
use crate::opts::Config;

pub async fn signal_handler() -> anyhow::Result<()> {
    let mut sigint = signal(SignalKind::interrupt())?;
    let mut sigterm = signal(SignalKind::terminate())?;

    select! {
        res = sigint.recv() => {
            match res {
                Some(_) => {Ok(())}
                None => {bail!("signal handler broken")}
            }
        },
        res = sigterm.recv() => {
            match res {
                Some(_) => {Ok(())}
                None => {bail!("signal handler broken")}
            }
        }
    }
}

fn until_next_midnight() -> Duration {
    let now_utc = Utc::now();
    let now_moscow = now_utc.with_timezone(&Moscow);

    let next_midnight = now_moscow.date_naive().and_hms_opt(0, 0, 0).unwrap() + Duration::days(1);

    let next_midnight_moscow = Moscow.from_local_datetime(&next_midnight).unwrap();
    let next_midnight_utc = next_midnight_moscow.with_timezone(&Utc);

    next_midnight_utc - now_utc
}

pub async fn daily(
    bot: Bot,
    count_messages: Arc<AtomicUsize>,
    spam_queue: Arc<RwLock<SpamQueue>>,
    config: Arc<RwLock<Config>>,
) {
    loop {
        sleep_until(Instant::now() + until_next_midnight().to_std().unwrap()).await;

        let (messages, queue_subscribers) = {
            let mut spam_queue = spam_queue.write().unwrap();

            let config = config.read().unwrap();

            let meme_limit = config.meme_limit;
            let queue_subscribers = config.queue_subscribers.clone();
            let queue_len = spam_queue.len();

            count_messages.store(meme_limit.min(queue_len), Ordering::Relaxed);

            let messages: Vec<_> = spam_queue.drain(0..meme_limit.min(queue_len)).collect();
            (messages, queue_subscribers)
        };

        for msg in messages {
            for subscriber in queue_subscribers.iter() {
                if let Err(e) = bot
                    .forward_message(*subscriber, msg.chat_id, msg.msg_id)
                    .send()
                    .await
                {
                    tracing::warn!(?e, "failed to send message: ");
                };
            }
        }

        save_state(spam_queue.clone(), config.clone()).await;
    }
}

pub async fn save_spam_queue(queue: Arc<RwLock<SpamQueue>>) -> anyhow::Result<()> {
    let queue = queue.read().unwrap().clone();
    let contents = serde_json::to_string(&queue)?;

    let mut path = expanduser::expanduser("~/.config/shutup-bot")?;
    tokio::fs::create_dir_all(&path).await?;
    path.push("spam-queue.json");

    let fd = tokio::fs::File::create(path).await?;
    let mut fd = BufWriter::new(fd);

    fd.write_all(contents.as_bytes()).await?;

    Ok(())
}

pub async fn load_spam_queue() -> anyhow::Result<SpamQueue> {
    let path = expanduser::expanduser("~/.config/shutup-bot/spam-queue.json")?;
    let fd = tokio::fs::File::open(path).await?;
    let mut fd = BufReader::new(fd);

    let mut buf = String::new();
    fd.read_to_string(&mut buf).await?;

    Ok(serde_json::from_str(&buf)?)
}

pub async fn save_state(spam_queue: Arc<RwLock<SpamQueue>>, config: Arc<RwLock<Config>>) {
    let (config_err, queue_err) = tokio::join!(
        save_config(config.clone()),
        save_spam_queue(spam_queue.clone())
    );
    if let Err(e) = config_err {
        warn!(?e, "an error occured while saving config");
    }
    if let Err(e) = queue_err {
        warn!(?e, "an error occured while saving spam queue");
    };
}

pub async fn save_config(config: Arc<RwLock<Config>>) -> tokio::io::Result<()> {
    let contents = serde_json::to_string(&*config.read().unwrap())?;

    let mut path = expanduser::expanduser("~/.config/shutup-bot")?;

    tokio::fs::create_dir_all(&path).await?;

    path.push("config.json");

    let fd = tokio::fs::File::create(path).await?;
    let mut fd = BufWriter::new(fd);

    fd.write_all(contents.as_bytes()).await?;

    Ok(())
}
