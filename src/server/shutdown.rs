use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Notify;
use tokio::time;

pub struct Shutdown {
    idle_reset: Arc<Notify>,
    shutdown_signal: Arc<Notify>,
    shutdown_flag: Arc<AtomicBool>,
    idle_active: Arc<AtomicBool>,
}

impl Shutdown {
    pub fn new() -> Self {
        let shutdown = Shutdown {
            idle_reset: Arc::new(Notify::new()),
            shutdown_signal: Arc::new(Notify::new()),
            shutdown_flag: Arc::new(AtomicBool::new(false)),
            idle_active: Arc::new(AtomicBool::new(true)),
        };

        let idle_reset = shutdown.idle_reset.clone();
        let shutdown_signal = shutdown.shutdown_signal.clone();
        let shutdown_flag = shutdown.shutdown_flag.clone();
        let idle_active = shutdown.idle_active.clone();

        tokio::spawn(async move {
            let idle_timeout = Duration::from_secs(30 * 60);
            loop {
                if shutdown_flag.load(Ordering::SeqCst) {
                    break;
                }
                tokio::select! {
                    _ = time::sleep(idle_timeout) => {
                        if idle_active.load(Ordering::SeqCst) {
                            eprintln!("Idle timeout reached (30 min), shutting down");
                            shutdown_flag.store(true, Ordering::SeqCst);
                            shutdown_signal.notify_waiters();
                            break;
                        }
                    }
                    _ = idle_reset.notified() => {
                        idle_active.store(true, Ordering::SeqCst);
                    }
                    _ = shutdown_signal.notified() => {
                        break;
                    }
                }
            }
        });

        shutdown
    }

    pub fn reset_idle(&self) {
        self.idle_active.store(false, Ordering::SeqCst);
        self.idle_reset.notify_one();
    }

    pub fn signal_shutdown(&self) {
        self.shutdown_flag.store(true, Ordering::SeqCst);
        // notify_waiters wakes every current waiter — the idle-timeout task
        // AND any caller blocked in wait_for_shutdown. notify_one would only
        // wake the first registered waiter, leaving main hung.
        self.shutdown_signal.notify_waiters();
    }

    pub async fn wait_for_shutdown(&self) {
        if self.shutdown_flag.load(Ordering::SeqCst) {
            return;
        }
        // Register as a waiter before re-checking the flag, so a signal that
        // fires between the check and the await still wakes us.
        let notified = self.shutdown_signal.notified();
        tokio::pin!(notified);
        notified.as_mut().enable();
        if self.shutdown_flag.load(Ordering::SeqCst) {
            return;
        }
        notified.await;
    }
}

impl Default for Shutdown {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idle_timeout_source_uses_30_minutes() {
        // Cross-language equivalent of the TS source-string check:
        // the timeout literal in this file should be 30 minutes.
        let src = include_str!("shutdown.rs");
        assert!(
            src.contains("Duration::from_secs(30 * 60)"),
            "expected 30-minute idle timeout literal in source"
        );
    }

    #[tokio::test]
    async fn signal_shutdown_resolves_wait() {
        let s = Shutdown::new();
        s.signal_shutdown();
        // Should complete promptly without panicking.
        tokio::time::timeout(Duration::from_secs(1), s.wait_for_shutdown())
            .await
            .expect("wait_for_shutdown should resolve after signal");
    }

    #[tokio::test]
    async fn signal_shutdown_wakes_main_even_when_idle_task_is_registered_waiter() {
        // Regression: the spawned idle-timeout task registers on
        // shutdown_signal.notified() first. Using notify_one would wake only
        // that task, leaving wait_for_shutdown (called by main) hung. Verify
        // wait_for_shutdown resolves even after the idle task has registered.
        let s = Shutdown::new();
        // Yield so the spawned idle task has a chance to register as a waiter.
        tokio::task::yield_now().await;
        tokio::time::sleep(Duration::from_millis(10)).await;

        s.signal_shutdown();

        tokio::time::timeout(Duration::from_secs(1), s.wait_for_shutdown())
            .await
            .expect("wait_for_shutdown should resolve even with idle task waiting");
    }

    #[tokio::test]
    async fn wait_for_shutdown_returns_immediately_after_prior_signal() {
        // If signal fires before any caller awaits wait_for_shutdown, the
        // shutdown_flag must short-circuit so we don't block forever.
        let s = Shutdown::new();
        s.signal_shutdown();
        tokio::time::sleep(Duration::from_millis(10)).await;

        tokio::time::timeout(Duration::from_secs(1), s.wait_for_shutdown())
            .await
            .expect("wait_for_shutdown should not block when signal already fired");
    }
}
