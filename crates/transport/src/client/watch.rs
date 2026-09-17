use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use anyhow::Result;

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub trait WatchdogTarget: Send + Sync {
    fn ping(&self, timeout: Duration) -> BoxFuture<'_, bool>;
    fn reconnect(&self) -> BoxFuture<'_, Result<()>>;
    fn give_up(&self);
}

#[derive(Debug, Clone, Copy)]
pub struct WatchdogConfig {
    pub interval: Duration,
    pub ping_timeout: Duration,
    pub fails_before_reconnect: u32,
    pub reconnects_before_exit: u32,
}

impl Default for WatchdogConfig {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(60),
            ping_timeout: Duration::from_secs(15),
            fails_before_reconnect: 2,
            reconnects_before_exit: 3,
        }
    }
}

pub struct Watchdog<T: WatchdogTarget> {
    target: T,
    config: WatchdogConfig,
    ping_fails: u32,
    reconnect_fails: u32,
}

impl<T: WatchdogTarget> Watchdog<T> {
    pub fn new(target: T, config: WatchdogConfig) -> Self {
        Self {
            target,
            config,
            ping_fails: 0,
            reconnect_fails: 0,
        }
    }

    pub async fn tick(&mut self) {
        if self.target.ping(self.config.ping_timeout).await {
            if self.ping_fails > 0 || self.reconnect_fails > 0 {
                log::info!(
                    "telegram.watchdog.recovered after_ping_fails={}",
                    self.ping_fails
                );
            }
            self.ping_fails = 0;
            self.reconnect_fails = 0;
            return;
        }

        self.ping_fails += 1;
        log::warn!("telegram.watchdog.ping_failed ping_fails={}", self.ping_fails);
        if self.ping_fails < self.config.fails_before_reconnect {
            return;
        }

        log::warn!("telegram.watchdog.forcing_reconnect");
        let outcome = match self.target.reconnect().await {
            Ok(()) => {
                if self.target.ping(self.config.ping_timeout).await {
                    Ok(())
                } else {
                    Err(anyhow::anyhow!("ping still failing after reconnect"))
                }
            }
            Err(err) => Err(err),
        };

        match outcome {
            Ok(()) => {
                log::info!("telegram.watchdog.reconnected");
                self.ping_fails = 0;
                self.reconnect_fails = 0;
            }
            Err(err) => {
                self.reconnect_fails += 1;
                log::error!(
                    "telegram.watchdog.reconnect_failed reconnect_fails={} err={err}",
                    self.reconnect_fails
                );
                if self.reconnect_fails >= self.config.reconnects_before_exit {
                    log::error!(
                        "telegram.watchdog.giving_up — handing back to supervisor (exit)"
                    );
                    self.target.give_up();
                }
            }
        }
    }

    pub async fn run(mut self) {
        let mut ticker = tokio::time::interval(self.config.interval);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        ticker.tick().await;
        loop {
            ticker.tick().await;
            self.tick().await;
        }
    }
}

#[cfg(test)]
#[path = "../../tests/unit/client/watch_test.rs"]
mod tests;
