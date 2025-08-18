use std::sync::{Arc, Mutex, atomic::AtomicUsize};

use chrono::{DateTime, NaiveDateTime, NaiveTime, TimeZone, Utc};
use chrono_tz::{Europe::Moscow, Tz};

pub fn refresh_meme_counter(
    counter: Arc<AtomicUsize>,
    last_count_refresh: Arc<Mutex<DateTime<Tz>>>,
) {
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
