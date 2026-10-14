use std::any::type_name;

use futures_util::future::{BoxFuture, try_join_all};
use mforward_core::Forwarder;

#[derive(Debug, thiserror::Error)]
pub(crate) enum ForwardersError {
    #[error("{0}")]
    Forwarder(&'static str),
    #[error("failed to listen for Ctrl-C: {0}")]
    Signal(#[from] std::io::Error),
}

#[derive(Default)]
pub(crate) struct Forwarders {
    tasks: Vec<BoxFuture<'static, Result<(), ForwardersError>>>,
}

impl Forwarders {
    pub(crate) fn start<F>(&mut self, forwarder: F, args: F::A)
    where
        F: Forwarder + Send + 'static,
        F::A: Send + 'static,
    {
        self.tasks.push(Box::pin(async move {
            forwarder
                .start(&args)
                .await
                .map_err(|_| ForwardersError::Forwarder(type_name::<F>()))
        }));
    }

    pub(crate) async fn wait(self) -> Result<(), ForwardersError> {
        tokio::select! {
            result = try_join_all(self.tasks) => result.map(|_| {}),
            signal = tokio::signal::ctrl_c() => {
                signal?;
                tracing::info!("Stopping forwarders");
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mforward_core::ForwarderError;
    use std::sync::Arc;
    use tokio::sync::Barrier;
    use tokio::time::{Duration, timeout};

    struct Rendezvous;

    impl Forwarder for Rendezvous {
        type A = Arc<Barrier>;

        async fn start(&self, barrier: &Self::A) -> Result<(), ForwarderError> {
            barrier.wait().await;
            Ok(())
        }
    }

    #[tokio::test]
    async fn forwarders_run_concurrently() {
        let barrier = Arc::new(Barrier::new(2));
        let mut forwarders = Forwarders::default();
        forwarders.start(Rendezvous, barrier.clone());
        forwarders.start(Rendezvous, barrier);
        timeout(Duration::from_secs(2), forwarders.wait())
            .await
            .expect("forwarders must run concurrently")
            .unwrap();
    }

    struct Failing;

    impl Forwarder for Failing {
        type A = ();

        async fn start(&self, _: &()) -> Result<(), ForwarderError> {
            Err(std::io::Error::other("tunnel failed").into())
        }
    }

    #[tokio::test]
    async fn failure_cancels_remaining_forwarders() {
        let barrier = Arc::new(Barrier::new(2));
        let mut forwarders = Forwarders::default();
        forwarders.start(Rendezvous, barrier.clone());
        forwarders.start(Failing, ());
        let error = timeout(Duration::from_secs(2), forwarders.wait())
            .await
            .expect("failure must not wait for other forwarders")
            .unwrap_err();
        let ForwardersError::Forwarder(forwarder) = error else {
            panic!("expected a forwarder error");
        };
        assert_eq!(forwarder, type_name::<Failing>());
        assert_eq!(Arc::strong_count(&barrier), 1);
    }
}
