use std::sync::LazyLock;

pub static TOKEN: LazyLock<String> = LazyLock::new(|| {
    std::env::var("TELOXIDE_TOKEN").expect("TELOXIDE_TOKEN env variable must be present")
});

pub const BASE_URL: &str = "https://api.telegram.org";
