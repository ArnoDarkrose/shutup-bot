use anyhow::bail;
use tokio::{
    select,
    signal::unix::{SignalKind, signal},
};

pub async fn signal_handler() -> anyhow::Result<()> {
    let mut sigint = signal(SignalKind::interrupt())?;
    let mut sigterm = signal(SignalKind::terminate())?;

    select! {
        res = sigint.recv() => {
            match res {
                Some(_) => {Ok(())}
                None => {bail!("signal handler broken")}
            }
        },
        res = sigterm.recv() => {
            match res {
                Some(_) => {Ok(())}
                None => {bail!("signal handler broken")}
            }
        }
    }
}
