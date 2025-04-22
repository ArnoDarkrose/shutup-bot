use std::error::Error;
use std::sync::{Arc, Mutex, atomic::AtomicUsize};

use teloxide::{
    dispatching::dialogue::GetChatId,
    prelude::*,
    types::{MessageKind, User},
};

use super::*;

pub async fn handle_spam(
    bot: Bot,
    msg: Message,
    count_messages: Arc<AtomicUsize>,
    meme_limit: usize,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    if let Message {
        from: Some(User {
            username: Some(ref username),
            ..
        }),
        ..
    } = msg
    {
        if username
            == SHUTUP_TARGET
                .get()
                .expect("this cell is only written to once at the beginning of main")
        {
            if let MessageKind::Common(ref common_msg) = msg.kind {
                if common_msg.forward_origin.is_some() {
                    let count = count_messages.load(std::sync::atomic::Ordering::Acquire) + 1;
                    count_messages.store(count, std::sync::atomic::Ordering::Release);

                    let Some(chat_id) = msg.chat_id() else {
                        tracing::warn!("unable get chat_id from message");

                        return Ok(());
                    };

                    if count > meme_limit {
                        if let Err(e) = bot.delete_message(chat_id, msg.id).send().await {
                            tracing::warn!(?e, "unable to delete message: ");
                        }

                        if let Err(e) = bot
                            .send_message(
                                msg.from
                                    .as_ref()
                                    .expect("if let earlier checks that this is some")
                                    .id,
                                "🤡",
                            )
                            .send()
                            .await
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

pub async fn help_common(bot: Bot, msg: Message) -> Result<(), Box<dyn Error + Send + Sync>> {
    if let Some(User {
        username: Some(ref username),
        ..
    }) = msg.from
    {
        if username
            == SHUTUP_TARGET
                .get()
                .expect("this OnceLock is only written to once at the beginning of main")
        {
            let Some(chat_id) = msg.chat_id() else {
                tracing::warn!("failed to get chat_id");

                return Ok(());
            };

            if let Err(e) = tokio::spawn(bot.send_message(chat_id, "иди нахуй").send())
                .await
                .expect("failed to await on tokio task")
            {
                tracing::warn!(?e, "failed to send message: ");
            }
        }
    }

    Ok(())
}

pub async fn help_admin(bot: Bot, msg: Message) -> Result<(), Box<dyn Error + Send + Sync>> {
    let Some(chat_id) = msg.chat_id() else {
        tracing::warn!("failed to get chat_id");

        return Ok(());
    };

    if let Err(e) = tokio::spawn(bot.send_message(chat_id, "привет").send())
        .await
        .expect("failed to await on tokio task")
    {
        tracing::warn!(?e, "failed to send message: ");
    }

    Ok(())
}

pub async fn add_admin(
    command: Command,
    admins: Arc<Mutex<Vec<String>>>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    match command {
        Command::AddAdmin(admin) => {
            let mut admins = admins.lock().unwrap();
            admins.push(admin);
        }
        _ => {
            unreachable!()
        }
    }

    Ok(())
}

pub async fn set_meme_counter(
    command: Command,
    message_count: Arc<AtomicUsize>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    match command {
        Command::SetMemeCounter(new_counter) => {
            message_count.store(new_counter, std::sync::atomic::Ordering::Relaxed);
        }
        _ => {
            unreachable!()
        }
    }

    Ok(())
}

pub async fn get_meme_counter(
    bot: Bot,
    msg: Message,
    message_count: Arc<AtomicUsize>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let Some(chat_id) = msg.chat_id() else {
        tracing::warn!("failed to get chat_id");

        return Ok(());
    };

    if let Err(e) = tokio::spawn(
        bot.send_message(
            chat_id,
            format!(
                "current meme counter: {}",
                message_count.load(std::sync::atomic::Ordering::Relaxed)
            ),
        )
        .send(),
    )
    .await
    .expect("failed to await on tokio task")
    {
        tracing::warn!(?e, "failed to send message: ");
    }

    Ok(())
}

pub async fn get_admins(
    bot: Bot,
    msg: Message,
    admins: Arc<Mutex<Vec<String>>>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let Some(chat_id) = msg.chat_id() else {
        tracing::warn!("failed to get chat_id");

        return Ok(());
    };

    if let Err(e) = tokio::spawn(
        bot.send_message(chat_id, format!("{:?}", admins.lock().unwrap()))
            .send(),
    )
    .await
    .expect("failed to await on tokio task")
    {
        tracing::warn!(?e, "failed to send message: ");
    }

    Ok(())
}
