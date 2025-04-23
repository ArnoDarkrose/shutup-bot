use std::error::Error;
use std::sync::{Arc, Mutex, atomic::AtomicUsize};

use teloxide::{
    dispatching::dialogue::GetChatId,
    prelude::*,
    types::{MessageKind, User},
};

use super::*;

type EndpointResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

pub async fn handle_spam(
    bot: Bot,
    msg: Message,
    count_messages: Arc<AtomicUsize>,
    config: Arc<Mutex<Config>>,
) -> EndpointResult<()> {
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

                    if count > config.lock().unwrap().meme_limit {
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

pub async fn help_shutup_target(bot: Bot, msg: Message) -> EndpointResult<()> {
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

pub async fn help_admin(bot: Bot, msg: Message) -> EndpointResult<()> {
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

pub async fn add_admin(command: Command, config: Arc<Mutex<Config>>) -> EndpointResult<()> {
    match command {
        Command::AddAdmin(admin) => {
            let mut config = config.lock().unwrap();
            config.admins.insert(admin);
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
) -> EndpointResult<()> {
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
) -> EndpointResult<()> {
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

pub async fn get_admins(bot: Bot, msg: Message, config: Arc<Mutex<Config>>) -> EndpointResult<()> {
    let Some(chat_id) = msg.chat_id() else {
        tracing::warn!("failed to get chat_id");

        return Ok(());
    };

    if let Err(e) = tokio::spawn(
        bot.send_message(chat_id, format!("{:?}", config.lock().unwrap().admins))
            .send(),
    )
    .await
    .expect("failed to await on tokio task")
    {
        tracing::warn!(?e, "failed to send message: ");
    }

    Ok(())
}

pub async fn remove_admin(command: Command, config: Arc<Mutex<Config>>) -> EndpointResult<()> {
    match command {
        Command::RemoveAdmin(admin) => {
            let mut config = config.lock().unwrap();

            if &admin
                != INITIAL_ADMIN
                    .get()
                    .expect("this once_lock is only written to once at the beginning of main")
            {
                config.admins.remove(&admin);
            }
        }
        _ => {
            unreachable!()
        }
    }

    Ok(())
}

pub async fn set_meme_limit(command: Command, config: Arc<Mutex<Config>>) -> EndpointResult<()> {
    match command {
        Command::SetMemeLimit(new_limit) => {
            config.lock().unwrap().meme_limit = new_limit;
        }
        _ => {
            unreachable!()
        }
    }

    Ok(())
}

pub async fn get_meme_limit(
    bot: Bot,
    msg: Message,
    config: Arc<Mutex<Config>>,
) -> EndpointResult<()> {
    let Some(chat_id) = msg.chat_id() else {
        tracing::warn!("could not get chat_id");
        return Ok(());
    };

    if let Err(e) = tokio::spawn(
        bot.send_message(chat_id, format!("{}", config.lock().unwrap().meme_limit))
            .send(),
    )
    .await
    .expect("failed to await on tokio task")
    {
        tracing::warn!(?e, "failed to send message: ");
    }

    Ok(())
}
