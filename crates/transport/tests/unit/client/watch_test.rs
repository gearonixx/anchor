use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::*;

#[derive(Default)]
struct FakeTarget {
    pings: Mutex<Vec<bool>>,
    reconnects: Mutex<Vec<bool>>,
    ping_calls: AtomicUsize,
    reconnect_calls: AtomicUsize,
    gave_up: AtomicUsize,
}

impl FakeTarget {
    fn new(pings: &[bool], reconnects: &[bool]) -> Self {
        Self {
            pings: Mutex::new(pings.to_vec()),
            reconnects: Mutex::new(reconnects.to_vec()),
            ..Default::default()
        }
    }

    fn pop(queue: &Mutex<Vec<bool>>, default: bool) -> bool {
        let mut q = queue.lock().unwrap();
        if q.len() > 1 {
            q.remove(0)
        } else {
            q.first().copied().unwrap_or(default)
        }
    }
}

impl WatchdogTarget for FakeTarget {
    fn ping(&self, _timeout: Duration) -> BoxFuture<'_, bool> {
        self.ping_calls.fetch_add(1, Ordering::SeqCst);
        let ok = Self::pop(&self.pings, true);
        Box::pin(async move { ok })
    }

    fn reconnect(&self) -> BoxFuture<'_, Result<()>> {
        self.reconnect_calls.fetch_add(1, Ordering::SeqCst);
        let ok = Self::pop(&self.reconnects, true);
        Box::pin(async move {
            if ok {
                Ok(())
            } else {
                Err(anyhow::anyhow!("reconnect boom"))
            }
        })
    }

    fn give_up(&self) {
        self.gave_up.fetch_add(1, Ordering::SeqCst);
    }
}

fn config() -> WatchdogConfig {
    WatchdogConfig {
        interval: Duration::from_millis(1),
        ping_timeout: Duration::from_millis(1),
        fails_before_reconnect: 2,
        reconnects_before_exit: 3,
    }
}

#[tokio::test]
async fn healthy_pings_never_reconnect() {
    let mut wd = Watchdog::new(FakeTarget::new(&[true], &[true]), config());
    wd.tick().await;
    wd.tick().await;
    assert_eq!(wd.target.reconnect_calls.load(Ordering::SeqCst), 0);
    assert_eq!(wd.target.gave_up.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn one_failure_is_tolerated() {
    let mut wd = Watchdog::new(FakeTarget::new(&[false, true], &[true]), config());
    wd.tick().await;
    assert_eq!(wd.target.reconnect_calls.load(Ordering::SeqCst), 0);
    wd.tick().await;
    assert_eq!(wd.target.reconnect_calls.load(Ordering::SeqCst), 0);
    assert_eq!(wd.ping_fails, 0);
}

#[tokio::test]
async fn two_failures_force_a_reconnect() {
    let mut wd = Watchdog::new(FakeTarget::new(&[false, false, true], &[true]), config());
    wd.tick().await;
    wd.tick().await;
    assert_eq!(wd.target.reconnect_calls.load(Ordering::SeqCst), 1);
    assert_eq!(wd.ping_fails, 0);
    assert_eq!(wd.reconnect_fails, 0);
}

#[tokio::test]
async fn reconnect_that_does_not_restore_the_socket_counts_as_a_failure() {
    let mut wd = Watchdog::new(FakeTarget::new(&[false], &[true]), config());
    wd.tick().await;
    wd.tick().await;
    assert_eq!(wd.target.reconnect_calls.load(Ordering::SeqCst), 1);
    assert_eq!(wd.reconnect_fails, 1);
    assert_eq!(wd.target.gave_up.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn gives_up_after_three_failed_reconnects() {
    let mut wd = Watchdog::new(FakeTarget::new(&[false], &[false]), config());
    for _ in 0..4 {
        wd.tick().await;
    }
    assert_eq!(wd.target.reconnect_calls.load(Ordering::SeqCst), 3);
    assert_eq!(wd.target.gave_up.load(Ordering::SeqCst), 1);
}
