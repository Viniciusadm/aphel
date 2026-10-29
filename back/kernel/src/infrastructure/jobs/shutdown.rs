use std::sync::Arc;

use tokio::sync::watch;

#[derive(Debug, Clone)]
pub struct Shutdown {
    receiver: watch::Receiver<bool>,
    _detached_sender: Option<Arc<watch::Sender<bool>>>,
}

#[derive(Debug)]
pub struct ShutdownTrigger {
    sender: watch::Sender<bool>,
}

pub fn shutdown_channel() -> (ShutdownTrigger, Shutdown) {
    let (sender, receiver) = watch::channel(false);
    (
        ShutdownTrigger { sender },
        Shutdown {
            receiver,
            _detached_sender: None,
        },
    )
}

impl ShutdownTrigger {
    pub fn trigger(&self) {
        self.sender.send_replace(true);
    }
}

impl Shutdown {
    pub(crate) fn is_triggered(&self) -> bool {
        *self.receiver.borrow()
    }

    pub(crate) async fn wait(&mut self) {
        let _ = self.receiver.wait_for(|triggered| *triggered).await;
    }
}

pub async fn termination_signal() {
    let ctrl_c = async {
        if let Err(error) = tokio::signal::ctrl_c().await {
            tracing::warn!("failed to listen for CTRL+C: {error}");
            std::future::pending::<()>().await;
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(error) => {
                tracing::warn!("failed to listen for SIGTERM: {error}");
                std::future::pending::<()>().await;
            }
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {}
        () = terminate => {}
    }
}
