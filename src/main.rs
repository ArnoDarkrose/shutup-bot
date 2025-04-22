use std::sync::{Arc, Mutex, atomic::AtomicUsize};

use chrono::TimeZone;
use chrono::{NaiveDate, NaiveDateTime, NaiveTime, Utc};
use chrono_tz::Europe::Moscow;
use clap::Parser;
use teloxide::{
    dispatching::dialogue::GetChatId,
    prelude::*,
    types::{MessageKind, User},
};

use tracing_journald::Layer;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::{layer::SubscriberExt as _, util::SubscriberInitExt as _};

#[derive(Parser, Debug)]
/// Bot that allows to shutup a meme spammer. Logs are written using journald
struct Cli {
    #[arg(long)]
    /// Telegram username of the target person
    shutup_target: String,

    #[arg(long, default_value = "2")]
    /// Number of memes per day that the person can send without limitations
    meme_limit: usize,

    #[arg(long, short, default_value = "info")]
    /// Level of logs to write, supported values are error, warn, info, debug and tracing
    log_level: String,
}

#[tokio::main]
async fn main() {
    let args = Cli::parse();

    let journald_layer = Layer::new().expect("failed to create journald tracing layer");

    match args.log_level.as_str() {
        "error" | "warn" | "info" | "debug" | "trace" | "ERROR" | "WARN" | "INFO" | "DEBUG"
        | "TRACE" => {}
        _ => {
            panic!("incorrect log_level value")
        }
    }

    let filter = EnvFilter::new(args.log_level);

    tracing_subscriber::registry()
        .with(journald_layer)
        .with(filter)
        .init();

    let bot = Bot::from_env();

    let count_messages = Arc::new(AtomicUsize::new(0));

    let naive = NaiveDateTime::new(
        NaiveDate::from_ymd_opt(2025, 4, 21).unwrap(),
        NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
    );
    let last_count_refresh = Moscow.from_local_datetime(&naive).unwrap();
    let last_count_refresh = Arc::new(Mutex::new(last_count_refresh));

    teloxide::repl(bot, move |bot: Bot, msg: Message| {
        let count_messages = Arc::clone(&count_messages);
        let last_count_refresh = Arc::clone(&last_count_refresh);
        let shutup_target = args.shutup_target.clone();

        async move {
            {
                let mut last_count_refresh_guard = last_count_refresh.lock().unwrap();
                let moscow_time = Utc::now().with_timezone(&Moscow);

                let days_diff = (moscow_time - *last_count_refresh_guard).num_days();

                if days_diff > 0 {
                    let naive = NaiveDateTime::new(
                        moscow_time.date_naive(),
                        NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
                    );
                    let new_count_refresh = Moscow.from_local_datetime(&naive).unwrap();

                    *last_count_refresh_guard = new_count_refresh;

                    count_messages.store(0, std::sync::atomic::Ordering::Relaxed);
                }
            }

            if let Message {
                from:
                    Some(User {
                        username: Some(ref username),
                        ..
                    }),
                ..
            } = msg
            {
                if username == &shutup_target {
                    if let MessageKind::Common(ref common_msg) = msg.kind {
                        if common_msg.forward_origin.is_some() {
                            let count =
                                count_messages.load(std::sync::atomic::Ordering::Acquire) + 1;
                            count_messages.store(count, std::sync::atomic::Ordering::Release);

                            let Some(chat_id) = msg.chat_id() else {
                                tracing::warn!("unable get chat_id from message");

                                return Ok(());
                            };

                            if count > args.meme_limit {
                                if let Err(e) = bot.delete_message(chat_id, msg.id).send().await {
                                    tracing::warn!(?e, "unable to delete message: ");
                                }

                                if let Err(e) =
                                    bot.send_message(msg.from.unwrap().id, "🤡").send().await
                                {
                                    tracing::warn!(?e, "unable to send message: ");
                                }
                            }
                        }
                    }
                }
            }

            Ok(())
        }
    })
    .await;
}
