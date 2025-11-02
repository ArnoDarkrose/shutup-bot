use std::error::Error;
use std::sync::{Arc, RwLock, atomic::AtomicUsize};

use anyhow::anyhow;
use teloxide::types::{
    Document, FileMeta, InputFile, MediaDocument, MediaKind, MessageCommon, MessageId,
};
use teloxide::{
    dispatching::dialogue::GetChatId,
    prelude::*,
    types::{MessageKind, User},
};
use tokio::task;
use tracing::{info, warn};
use zip::CompressionMethod;

use crate::consts::{BASE_URL, TOKEN};
use crate::core::Command;
use crate::manga::{extract_images, write_images_to_archive};
use crate::opts::{State, save_config};

use super::*;

type EndpointResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

pub async fn handle_spam(
    bot: Bot,
    msg: Message,
    count_messages: Arc<AtomicUsize>,
    config: Arc<RwLock<State>>,
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

                    if count > config.read().unwrap().meme_limit {
                        forward_to_subscribers(bot.clone(), msg.id, chat_id, Arc::clone(&config))
                            .await;

                        if let Err(e) = bot.delete_message(chat_id, msg.id).send().await {
                            tracing::warn!(?e, "unable to delete message: ");
                        }

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
    config: Arc<RwLock<State>>,
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
    config: Arc<RwLock<State>>,
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

pub async fn add_admin(command: Command, config: Arc<RwLock<State>>) -> EndpointResult<()> {
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

pub async fn get_admins(bot: Bot, msg: Message, config: Arc<RwLock<State>>) -> EndpointResult<()> {
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

pub async fn remove_admin(command: Command, config: Arc<RwLock<State>>) -> EndpointResult<()> {
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

pub async fn set_meme_limit(command: Command, config: Arc<RwLock<State>>) -> EndpointResult<()> {
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
    config: Arc<RwLock<State>>,
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

pub async fn subscribe_forwards(msg: Message, config: Arc<RwLock<State>>) -> EndpointResult<()> {
    let Some(User { id: user_id, .. }) = msg.from else {
        tracing::warn!("could not get user_id");
        return Ok(());
    };

    {
        let mut config = config.write().unwrap();

        config.forward_subscribers.push(user_id);
    }

    // We don't really need the result of this, so the handle is dropped
    std::mem::drop(tokio::spawn(save_config(Arc::clone(&config))));

    Ok(())
}

pub async fn get_config(bot: Bot, msg: Message, config: Arc<RwLock<State>>) -> EndpointResult<()> {
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

// TODO: refactor this
pub async fn pdf_to_cbz(bot: Bot, command: Command, msg: Message) -> EndpointResult<()> {
    info!("Pdf to cbz request");

    let chat_id = msg
        .chat
        .chat_id()
        .ok_or(anyhow!("Failed to get chat_id"))
        .inspect_err(|err| warn!(?err))?;

    let url = match command {
        Command::Cbz(url) => url,
        _ => {
            unreachable!()
        }
    };

    let (url, file_name) = if let Some((url, filename)) = url.split_once(' ') {
        (url.to_string(), Some(filename.to_owned()))
    } else {
        (url, None)
    };

    let (doc, file_name) = if let MessageKind::Common(MessageCommon {
        media_kind:
            MediaKind::Document(MediaDocument {
                document:
                    Document {
                        file: FileMeta { id, .. },
                        file_name,
                        ..
                    },
                ..
            }),
        ..
    }) = msg.kind
    {
        info!("Getting file...");

        let doc = if url.is_empty() {
            let file_path = match bot.get_file(id).await {
                Ok(file) => file.path,
                Err(err) => {
                    warn!(?err);

                    bot.send_message(chat_id, format!("Failed to download file: {err:?}"))
                        .await?;

                    return Err(err).map_err(|err| err.into());
                }
            };

            let url = format!("{BASE_URL}/file/bot{}/{file_path}", TOKEN.as_str());

            info!("Downloading file...");
            bot.send_message(chat_id, "Downloading file...").await?;
            reqwest::get(&url)
                .await
                .inspect_err(|err| warn!(?err))?
                .bytes()
                .await
                .inspect_err(|err| warn!(?err))?
        } else {
            info!("Downloading file...");
            bot.send_message(chat_id, "Downloading file...").await?;
            reqwest::get(url)
                .await
                .inspect_err(|err| warn!(?err))?
                .bytes()
                .await
                .inspect_err(|err| warn!(?err))?
        };
        bot.send_message(chat_id, "Download successful!").await?;
        info!("Downloading successful");

        (doc, file_name)
    } else {
        if url.is_empty() {
            bot.send_message(chat_id, "No file attached").await?;
            return Err(anyhow!("No file attached")).map_err(|err| err.into());
        } else {
            info!("Downloading file...");
            bot.send_message(chat_id, "Downloading file...").await?;
            let doc = reqwest::get(url)
                .await
                .inspect_err(|err| warn!(?err))?
                .bytes()
                .await
                .inspect_err(|err| warn!(?err))?;
            info!("Downloading successful");
            bot.send_message(chat_id, "Download successful!").await?;
            (doc, file_name)
        }
    };

    let archive = task::block_in_place(move || {
        let doc = lopdf::Document::load_mem(&doc)?;
        let images = extract_images(&doc)?;

        let compression_method = if images[0].1 == "png" {
            CompressionMethod::Deflated
        } else {
            CompressionMethod::Stored
        };
        let archive = write_images_to_archive(images, compression_method);

        crate::manga::error::Result::Ok(archive)
    })?
    .inspect_err(|err| warn!(?err))?;

    let file_name = file_name.map(|v| {
        if v.ends_with(".pdf") {
            let name = v.rsplit_once(".").unwrap().0;
            format!("{name}.cbz")
        } else {
            format!("{v}.cbz")
        }
    });
    let file = InputFile::memory(archive)
        .file_name(file_name.unwrap_or_else(|| "somethingsomething.bin".to_string()));

    bot.send_document(chat_id, file)
        .await
        .inspect_err(|err| warn!(?err))?;

    Ok(())
}
