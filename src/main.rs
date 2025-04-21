use std::sync::{Arc, Mutex, atomic::AtomicUsize};

use chrono::TimeZone;
use chrono::{NaiveDate, NaiveDateTime, NaiveTime, Utc};
use chrono_tz::Europe::Moscow;
use teloxide::{
    dispatching::dialogue::GetChatId,
    prelude::*,
    types::{MessageKind, User},
};

const SHUTUP_TARGET: &str = "ArnoDarkrose";
const MEME_LIMIT: usize = 2;

#[tokio::main]
async fn main() {
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
                if username == SHUTUP_TARGET {
                    if let MessageKind::Common(ref common_msg) = msg.kind {
                        if common_msg.forward_origin.is_some() {
                            let count =
                                count_messages.load(std::sync::atomic::Ordering::Acquire) + 1;
                            count_messages.store(count, std::sync::atomic::Ordering::Release);

                            // bot.send_message(
                            //     msg.chat_id().unwrap(),
                            //     format!(
                            //         "this user's forwards num: {}",
                            //         count_messages.load(std::sync::atomic::Ordering::Relaxed)
                            //     ),
                            // )
                            // .await
                            // .unwrap();

                            if count > MEME_LIMIT {
                                bot.delete_message(msg.chat_id().unwrap(), msg.id)
                                    .send()
                                    .await
                                    .unwrap();
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
