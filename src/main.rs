use core::start;
use std::sync::OnceLock;

use clap::Parser;
use opts::Cli;

mod core;
mod endpoints;
mod helpers;
mod opts;
mod schema;

static SHUTUP_TARGET: OnceLock<String> = OnceLock::new();
static INITIAL_ADMIN: OnceLock<String> = OnceLock::new();

#[tokio::main]
async fn main() {
    let args = Cli::parse();

    args.setup_logs();

    SHUTUP_TARGET.set(args.shutup_target).unwrap();
    INITIAL_ADMIN.set(args.initial_admin.clone()).unwrap();

    start(args.meme_limit).await;
}
