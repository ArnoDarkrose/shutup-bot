use std::collections::VecDeque;
use std::error::Error;
use std::sync::{Arc, RwLock, atomic::AtomicUsize};

use teloxide::types::MessageId;
use teloxide::{
    dispatching::dialogue::GetChatId,
    prelude::*,
    types::{MessageKind, User},
};

use crate::app::{Command, MsgWrapper};
use crate::opts::{Config, save_config};

use super::*;

type EndpointResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

pub async fn handle_spam(
    bot: Bot,
    msg: Message,
    count_messages: Arc<AtomicUsize>,
    config: Arc<RwLock<Config>>,
    spam_queue: Arc<RwLock<VecDeque<MsgWrapper>>>,
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

                    let meme_limit = { config.read().unwrap().meme_limit };

                    if count > meme_limit {
                        forward_to_subscribers(bot.clone(), msg.id, chat_id, Arc::clone(&config))
                            .await;

                        if let Err(e) = bot.delete_message(chat_id, msg.id).send().await {
                            tracing::warn!(?e, "unable to delete message: ");
                        }

                        spam_queue.write().unwrap().push_back(MsgWrapper {
                            msg_id: msg.id,
                            chat_id,
                        });

                        count_messages.store(count - 1, std::sync::atomic::Ordering::Release);

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

pub async fn forward_to_subscribers(
    bot: Bot,
    msg_id: MessageId,
    src: ChatId,
    config: Arc<RwLock<Config>>,
) {
    let subscribers = config.read().unwrap().forward_subscribers.clone();

    for subscriber in subscribers {
        if let Err(e) = bot.forward_message(subscriber, src, msg_id).send().await {
            tracing::warn!(?e, "failed to forward_message: ");
        }
    }
}

pub async fn get_forward_subscribers(
    bot: Bot,
    msg: Message,
    config: Arc<RwLock<Config>>,
) -> EndpointResult<()> {
    let Some(chat_id) = msg.chat_id() else {
        tracing::warn!("failed to get chat_id");

        return Ok(());
    };

    if let Err(e) = bot
        .send_message(
            chat_id,
            format!("{:?}", config.read().unwrap().forward_subscribers),
        )
        .send()
        .await
    {
        tracing::warn!(?e, "failed to send message: ");
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

pub async fn add_admin(command: Command, config: Arc<RwLock<Config>>) -> EndpointResult<()> {
    match command {
        Command::AddAdmin(admin) => {
            let mut config = config.write().unwrap();
            config.admins.insert(admin);
        }
        _ => {
            unreachable!()
        }
    }

    // We don't really need the result of this, so the handle is dropped
    std::mem::drop(tokio::spawn(save_config(Arc::clone(&config))));

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

pub async fn get_admins(bot: Bot, msg: Message, config: Arc<RwLock<Config>>) -> EndpointResult<()> {
    let Some(chat_id) = msg.chat_id() else {
        tracing::warn!("failed to get chat_id");

        return Ok(());
    };

    if let Err(e) = tokio::spawn(
        bot.send_message(chat_id, format!("{:?}", config.read().unwrap().admins))
            .send(),
    )
    .await
    .expect("failed to await on tokio task")
    {
        tracing::warn!(?e, "failed to send message: ");
    }

    Ok(())
}

pub async fn remove_admin(command: Command, config: Arc<RwLock<Config>>) -> EndpointResult<()> {
    match command {
        Command::RemoveAdmin(admin) => {
            let mut config = config.write().unwrap();

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

    // We don't really need the result of this, so the handle is dropped
    std::mem::drop(tokio::spawn(save_config(Arc::clone(&config))));

    Ok(())
}

pub async fn set_meme_limit(command: Command, config: Arc<RwLock<Config>>) -> EndpointResult<()> {
    match command {
        Command::SetMemeLimit(new_limit) => {
            config.write().unwrap().meme_limit = new_limit;
        }
        _ => {
            unreachable!()
        }
    }

    // We don't really need the result of this, so the handle is dropped
    std::mem::drop(tokio::spawn(save_config(Arc::clone(&config))));

    Ok(())
}

pub async fn get_meme_limit(
    bot: Bot,
    msg: Message,
    config: Arc<RwLock<Config>>,
) -> EndpointResult<()> {
    let Some(chat_id) = msg.chat_id() else {
        tracing::warn!("could not get chat_id");
        return Ok(());
    };

    if let Err(e) = tokio::spawn(
        bot.send_message(chat_id, format!("{}", config.read().unwrap().meme_limit))
            .send(),
    )
    .await
    .expect("failed to await on tokio task")
    {
        tracing::warn!(?e, "failed to send message: ");
    }

    // We don't really need the result of this, so the handle is dropped
    std::mem::drop(tokio::spawn(save_config(Arc::clone(&config))));

    Ok(())
}

pub async fn subscribe_forwards(msg: Message, config: Arc<RwLock<Config>>) -> EndpointResult<()> {
    let Some(User { id: user_id, .. }) = msg.from else {
        tracing::warn!("could not get user_id");
        return Ok(());
    };

    {
        let mut config = config.write().unwrap();

        if config
            .forward_subscribers
            .iter()
            .find(|&&v| v == user_id)
            .is_none()
        {
            config.forward_subscribers.insert(user_id);
        }
    }

    // We don't really need the result of this, so the handle is dropped
    std::mem::drop(tokio::spawn(save_config(Arc::clone(&config))));

    Ok(())
}

pub async fn unsubscribe_forwards(msg: Message, config: Arc<RwLock<Config>>) -> EndpointResult<()> {
    let Some(User { id: user_id, .. }) = msg.from else {
        tracing::warn!("could not get user_id");
        return Ok(());
    };

    {
        let mut config = config.write().unwrap();

        config.forward_subscribers.remove(&user_id);
    }

    // We don't really need the result of this, so the handle is dropped
    std::mem::drop(tokio::spawn(save_config(Arc::clone(&config))));

    Ok(())
}

pub async fn get_config(bot: Bot, msg: Message, config: Arc<RwLock<Config>>) -> EndpointResult<()> {
    let Some(chat_id) = msg.chat_id() else {
        tracing::warn!("could not get chat_id");
        return Ok(());
    };

    if let Err(e) = bot
        .send_message(chat_id, format!("{:?}", config.read().unwrap()))
        .send()
        .await
    {
        tracing::warn!(?e, "failed to send message: ");
    }

    Ok(())
}

pub async fn queue_size(
    bot: Bot,
    msg: Message,
    spam_queue: Arc<RwLock<VecDeque<MsgWrapper>>>,
) -> EndpointResult<()> {
    let Some(chat_id) = msg.chat_id() else {
        tracing::warn!("could not get chat_id");
        return Ok(());
    };

    if let Err(e) = bot
        .send_message(
            chat_id,
            format!("Spam queue size: {}", spam_queue.read().unwrap().len()),
        )
        .send()
        .await
    {
        tracing::warn!(?e, "failed to send message: ");
    }

    Ok(())
}

pub async fn subscribe_queue(msg: Message, config: Arc<RwLock<Config>>) -> EndpointResult<()> {
    let Some(chat_id) = msg.chat_id() else {
        tracing::warn!("could not get chat_id");
        return Ok(());
    };

    {
        let mut config = config.write().unwrap();

        if config
            .queue_subscribers
            .iter()
            .find(|&&v| v == chat_id)
            .is_none()
        {
            config.queue_subscribers.insert(chat_id);
        }
    }

    // We don't really need the result of this, so the handle is dropped
    std::mem::drop(tokio::spawn(save_config(Arc::clone(&config))));

    Ok(())
}

pub async fn unsubscribe_queue(msg: Message, config: Arc<RwLock<Config>>) -> EndpointResult<()> {
    let Some(chat_id) = msg.chat_id() else {
        tracing::warn!("could not get chat_id");
        return Ok(());
    };

    {
        let mut config = config.write().unwrap();

        config.queue_subscribers.remove(&chat_id);
    }

    // We don't really need the result of this, so the handle is dropped
    std::mem::drop(tokio::spawn(save_config(Arc::clone(&config))));

    Ok(())
}
