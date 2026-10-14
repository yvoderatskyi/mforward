use thiserror::Error;
use tokio::process::Command;

use mforward_core::{Forwarder, ForwarderError};
pub mod types;

pub use types::{LocalPort, Namespace, RemotePort, ServiceName};

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KubectlArgs {
    namespace: Namespace,
    service_name: ServiceName,
    remote_port: RemotePort,
    local_port: LocalPort,
}

impl KubectlArgs {
    pub fn new(
        namespace: impl Into<String>,
        service_name: impl Into<String>,
        remote_port: usize,
        local_port: usize,
    ) -> Result<Self, &'static str> {
        Ok(Self {
            namespace: Namespace::new(namespace)?,
            service_name: ServiceName::new(service_name)?,
            remote_port: RemotePort::new(remote_port)?,
            local_port: LocalPort::new(local_port)?,
        })
    }
}

#[derive(Error, Debug)]
pub enum KubectlError {
    #[error("kubectl process error: {0}")]
    UnknownError(#[from] std::io::Error),
    #[error("connection error: {0}")]
    ConnectionError(i32),
}
pub struct Kubectl;

impl Forwarder for Kubectl {
    type A = KubectlArgs;

    #[tracing::instrument(fields(namespace= args.namespace.value()), skip_all)]
    async fn start(&self, args: &Self::A) -> Result<(), ForwarderError> {
        tracing::info!("Starting kubectl port-forward");
        let KubectlArgs {
            namespace,
            service_name,
            remote_port,
            local_port,
        } = args;
        let mut child = Command::new("kubectl")
            .args([
                "port-forward".to_owned(),
                "-n".to_owned(),
                namespace.value().to_owned(),
                format!("service/{}", service_name.value()),
                format!("{}:{}", local_port.value(), remote_port.value()),
            ])
            .kill_on_drop(true)
            .spawn()
            .map_err(KubectlError::from)?;

        tracing::debug!(pid = ?child.id(), "Started kubectl process");

        // Await until the command completes
        let status = child.wait().await.map_err(KubectlError::from)?;
        tracing::debug!(%status, "kubectl process exited");
        if !status.success() {
            return Err(KubectlError::ConnectionError(status.code().unwrap_or(-1)).into());
        }
        tracing::info!("kubectl port-forward completed");
        Ok(())
    }
}
