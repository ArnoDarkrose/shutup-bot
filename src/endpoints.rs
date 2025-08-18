use std::error::Error;
use std::sync::atomic::Ordering;
use std::sync::{Arc, RwLock, atomic::AtomicUsize};

use teloxide::types::MessageId;
use teloxide::{
    dispatching::dialogue::GetChatId,
    prelude::*,
    types::{MessageKind, User},
};

use crate::app::{Command, MsgWrapper, SpamQueue};
use crate::opts::Config;

use super::*;

type EndpointResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

pub async fn handle_spam(
    bot: Bot,
    msg: Message,
    count_messages: Arc<AtomicUsize>,
    config: Arc<RwLock<Config>>,
    spam_queue: Arc<RwLock<SpamQueue>>,
) -> EndpointResult<()> {
    if let Message {
        from: Some(User {
            username: Some(ref username),
            ..
        }),
        ..
    } = msg
        && username
            == SHUTUP_TARGET
                .get()
                .expect("this cell is only written to once at the beginning of main")
        && let MessageKind::Common(ref common_msg) = msg.kind
        && common_msg.forward_origin.is_some()
    {
        let count = count_messages.fetch_add(1, Ordering::AcqRel) + 1;

        let Some(chat_id) = msg.chat_id() else {
            tracing::warn!("failed get chat_id from message");

            return Ok(());
        };

        let meme_limit = { config.read().unwrap().meme_limit };

        if count > meme_limit {
            forward_to_subscribers(bot.clone(), msg.id, chat_id, Arc::clone(&config)).await;

            if let Err(e) = bot.delete_message(chat_id, msg.id).send().await {
                tracing::warn!(?e, "failed to delete message");
            }

            spam_queue.write().unwrap().push_back(MsgWrapper {
                msg_id: msg.id,
                chat_id,
            });

            if let Err(e) = bot
                .send_message(
                    msg.from.as_ref().expect("unreachable: none username").id,
                    "🤡",
                )
                .send()
                .await
            {
                tracing::warn!(?e, "failed to send message");
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
        && username
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

    Ok(())
}

pub async fn subscribe_forwards(msg: Message, config: Arc<RwLock<Config>>) -> EndpointResult<()> {
    let Some(User { id: user_id, .. }) = msg.from else {
        tracing::warn!("could not get user_id");
        return Ok(());
    };

    {
        let mut config = config.write().unwrap();

        if !config.forward_subscribers.iter().any(|&v| v == user_id) {
            config.forward_subscribers.insert(user_id);
        }
    }

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
    spam_queue: Arc<RwLock<SpamQueue>>,
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

        if !config.queue_subscribers.iter().any(|&v| v == chat_id) {
            config.queue_subscribers.insert(chat_id);
        }
    }

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

    Ok(())
}
