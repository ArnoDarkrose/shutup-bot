// TODO: spawn a task that will wake every 00:00 and referesh counters and send queued messages
// TODO: add a filter for references so that bot also deletes references to tg messages, not only forwards

use std::sync::OnceLock;

use clap::Parser;
use opts::Cli;

use crate::app::App;

mod app;
mod endpoints;
mod helpers;
mod opts;
mod schema;
mod utils;

static SHUTUP_TARGET: OnceLock<String> = OnceLock::new();
static INITIAL_ADMIN: OnceLock<String> = OnceLock::new();

#[tokio::main]
async fn main() {
    let args = Cli::parse();

    args.setup_logs();

    SHUTUP_TARGET.set(args.shutup_target).unwrap();
    INITIAL_ADMIN.set(args.initial_admin.clone()).unwrap();

    App::new(
        args.meme_limit,
        args.connect_timeout,
        args.timeout,
        args.concurrent_connections,
    )
    .start()
    .await;
}
