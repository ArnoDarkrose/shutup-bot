use std::sync::atomic::AtomicUsize;
use std::sync::{Arc, Mutex, RwLock};

use chrono::DateTime;
use chrono_tz::Tz;
use teloxide::prelude::*;
use teloxide::dispatching::UpdateHandler;
use teloxide::types::User;

use crate::app::Command;
use crate::helpers::refresh_meme_counter;
use crate::opts::Config;
use crate::{endpoints::*, SHUTUP_TARGET};

pub fn schema() -> UpdateHandler<Box<dyn std::error::Error + Send + Sync + 'static>> {
    use dptree::case;

    let handle_shutup_target_admin = dptree::filter(|msg: Message| {
        if let Some(User{username: Some(ref username), ..}) = msg.from {
            username == SHUTUP_TARGET.get().unwrap()
        } else  {
            false
        }
    
    })
    .branch(case![Command::Help]
    .endpoint(help_shutup_target))
    .endpoint(handle_spam);

    let handle_commands = teloxide::filter_command::<Command, _>()
        .branch(
            dptree::filter(|msg: Message, config: Arc<RwLock<Config>>| {
                if let Some(User{username: Some(ref username), ..}) = msg.from {
                    let admins = &config.read().unwrap().admins;
                    admins.contains(username)
                } else  {
                    false
                }
                
            })
            .branch(case![Command::MemeLimit].endpoint(get_meme_limit))
            .branch(case![Command::AddAdmin(admin)].endpoint(add_admin))
            .branch(case![Command::Admins].endpoint(get_admins))
            .branch(case![Command::MemeCounter].endpoint(get_meme_counter))
            .branch(case![Command::RemoveAdmin(admin)].endpoint(remove_admin))
            .branch(case![Command::SubscribeForwards].endpoint(subscribe_forwards))
            .branch(case![Command::UnsubscribeForwards].endpoint(unsubscribe_forwards))
            .branch(case![Command::ForwardSubscribers].endpoint(get_forward_subscribers))
            .branch(case![Command::Config].endpoint(get_config))
            .branch(case![Command::QueueSize].endpoint(queue_size))
            .branch(case![Command::SubscribeQueue].endpoint(subscribe_queue))
            .branch(case![Command::UnsubscribeQueue].endpoint(unsubscribe_queue))
            .branch(handle_shutup_target_admin)
            .branch(case![Command::Help].endpoint(help_admin))
            .branch(case![Command::SetMemeCounter(counter)].endpoint(set_meme_counter))
            .branch(case![Command::SetMemeLimit(new_limit)].endpoint(set_meme_limit))
        );

    Update::filter_message()
        .inspect_async(|count_messages: Arc<AtomicUsize>, last_count_refresh : Arc<Mutex<DateTime<Tz>>>| async move {
            refresh_meme_counter(count_messages, last_count_refresh);        
         })
        .branch(handle_commands)
        .endpoint(handle_spam)
}

