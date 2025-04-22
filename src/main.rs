use std::sync::OnceLock;
use std::sync::{Arc, Mutex, atomic::AtomicUsize};

use chrono::{DateTime, TimeZone};
use chrono_tz::Tz;
use chrono::{NaiveDateTime, NaiveTime, Utc};
use chrono::NaiveDate;
use chrono_tz::Europe::Moscow;
use clap::Parser;
use teloxide::dispatching::UpdateHandler;
use teloxide::prelude::*;
use teloxide::types::User;
use teloxide::utils::command::BotCommands;

use tracing_journald::Layer;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::{layer::SubscriberExt as _, util::SubscriberInitExt as _};

pub mod endpoints;

use endpoints::*;

static SHUTUP_TARGET: OnceLock<String> = OnceLock::new();

#[derive(Parser, Debug)]
/// Bot that allows to shutup a meme spammer. Logs are written using journald
struct Cli {
    #[arg(long)]
    /// Telegram username of the target person
    shutup_target: String,

    #[arg(long)]
    /// Username of the initial admin of the bot. Other admins can be added by interacting with the server when it's up
    initial_admin: String,

    #[arg(long, default_value = "2")]
    /// Number of memes per day that the person can send without limitations
    meme_limit: usize,

    #[arg(long, short, default_value = "info")]
    /// Level of logs to write, supported values are error, warn, info, debug and tracing
    log_level: String,
}

#[derive(BotCommands, Clone)]
#[command(rename_rule = "snake_case")]
pub enum Command {
    Help,
    AddAdmin(String),
    SetMemeCounter(usize),
    MemeCounter,
    Admins,
}

fn refresh_meme_counter(counter: Arc<AtomicUsize>, last_count_refresh: Arc<Mutex<DateTime<Tz>>>) {
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

        counter.store(0, std::sync::atomic::Ordering::Relaxed);
    }
}

fn schema() -> UpdateHandler<Box<dyn std::error::Error + Send + Sync + 'static>> {
    use dptree::case;

    let handle_commands = teloxide::filter_command::<Command, _>()
        .branch(
            dptree::filter(|msg: Message, admins: Arc<Mutex<Vec<String>>>| {
                if let Some(User{username: Some(ref username), ..}) = msg.from {
                    let admins = admins.lock().unwrap();
                    admins.contains(username)
                } else  {
                    false
                }
                
            })
            .branch(case![Command::AddAdmin(admin)].endpoint(add_admin))
            .branch(case![Command::Help].endpoint(help_admin))
            .branch(case![Command::SetMemeCounter(counter)].endpoint(set_meme_counter))
            .branch(case![Command::MemeCounter].endpoint(get_meme_counter))
            .branch(case![Command::Admins].endpoint(get_admins)),
        )
        .branch(case![Command::Help].endpoint(help_common));

    Update::filter_message()
        .inspect_async(|count_messages: Arc<AtomicUsize>, last_count_refresh : Arc<Mutex<DateTime<Tz>>>| async move {
            refresh_meme_counter(count_messages, last_count_refresh);        
         })
        .branch(handle_commands)
        .endpoint(handle_spam)
}

#[tokio::main]
async fn main() {
    let args = Cli::parse();

    SHUTUP_TARGET.set(args.shutup_target).unwrap();

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

    let admins = Arc::new(Mutex::new(vec![args.initial_admin]));

    Dispatcher::builder(bot, schema())
        .dependencies(dptree::deps![
            Arc::clone(&admins),
            Arc::clone(&last_count_refresh),
            Arc::clone(&count_messages),
            args.meme_limit
        ])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}
