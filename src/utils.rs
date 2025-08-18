use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, RwLock};

use anyhow::bail;
use chrono::{DateTime, TimeZone};
use chrono::{Duration, Timelike, Utc};
use chrono_tz::Europe::Moscow;
use chrono_tz::Tz;
use teloxide::Bot;
use teloxide::prelude::{Request, Requester};
use tokio::time::{Instant, sleep_until};
use tokio::{
    select,
    signal::unix::{SignalKind, signal},
};

use crate::app::MsgWrapper;
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

pub async fn manage_queue(
    bot: Bot,
    count_messages: Arc<AtomicUsize>,
    spam_queue: Arc<RwLock<VecDeque<MsgWrapper>>>,
    config: Arc<RwLock<Config>>,
) {
    loop {
        sleep_until(Instant::now() + until_next_midnight().to_std().unwrap()).await;

        let mut spam_queue = spam_queue.write().unwrap();

        let (meme_limit, queue_subscribers) = {
            let config = config.read().unwrap();

            (config.meme_limit, config.queue_subscribers.clone())
        };
        let queue_len = spam_queue.len();

        count_messages.store(meme_limit.min(queue_len), Ordering::Relaxed);

        for msg in spam_queue.drain(0..meme_limit.min(queue_len)) {
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
    }
}
