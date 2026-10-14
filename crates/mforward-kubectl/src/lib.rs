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

fn command(args: &KubectlArgs) -> (&'static str, Vec<String>) {
    (
        "kubectl",
        vec![
            "port-forward".to_owned(),
            "-n".to_owned(),
            args.namespace.value().to_owned(),
            format!("service/{}", args.service_name.value()),
            format!("{}:{}", args.local_port.value(), args.remote_port.value()),
        ],
    )
}

impl Forwarder for Kubectl {
    type Args = KubectlArgs;

    #[tracing::instrument(fields(namespace= args.namespace.value()), skip_all)]
    async fn start(&self, args: &Self::Args) -> Result<(), ForwarderError> {
        tracing::info!("Starting kubectl port-forward");
        let (program, command_args) = command(args);
        let mut child = Command::new(program)
            .args(command_args)
            .kill_on_drop(true)
            .spawn()
            .map_err(KubectlError::from)?;

        tracing::debug!(pid = ?child.id(), "Started kubectl process");

        let status = child.wait().await.map_err(KubectlError::from)?;
        tracing::debug!(%status, "kubectl process exited");
        if !status.success() {
            return Err(KubectlError::ConnectionError(status.code().unwrap_or(-1)).into());
        }
        tracing::info!("kubectl port-forward completed");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_kubectl_port_forward_command() {
        let args = KubectlArgs::new("production", "api", 8080, 0).unwrap();

        assert_eq!(
            command(&args),
            (
                "kubectl",
                vec!["port-forward", "-n", "production", "service/api", "0:8080"]
                    .into_iter()
                    .map(str::to_owned)
                    .collect()
            )
        );
    }
}
