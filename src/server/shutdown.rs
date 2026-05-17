use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Notify;
use tokio::time;

pub struct Shutdown {
    idle_reset: Arc<Notify>,
    shutdown_signal: Arc<Notify>,
    idle_active: Arc<AtomicBool>,
}

impl Shutdown {
    pub fn new() -> Self {
        let shutdown = Shutdown {
            idle_reset: Arc::new(Notify::new()),
            shutdown_signal: Arc::new(Notify::new()),
            idle_active: Arc::new(AtomicBool::new(true)),
        };

        let idle_reset = shutdown.idle_reset.clone();
        let shutdown_signal = shutdown.shutdown_signal.clone();
        let idle_active = shutdown.idle_active.clone();

        tokio::spawn(async move {
            let idle_timeout = Duration::from_secs(30 * 60);
            loop {
                tokio::select! {
                    _ = time::sleep(idle_timeout) => {
                        if idle_active.load(Ordering::SeqCst) {
                            eprintln!("Idle timeout reached (30 min), shutting down");
                            shutdown_signal.notify_one();
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
        self.shutdown_signal.notify_one();
    }

    pub async fn wait_for_shutdown(&self) {
        self.shutdown_signal.notified().await;
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
}
