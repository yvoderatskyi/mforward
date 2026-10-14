use thiserror::Error;
use tokio::process::Command;

use mforward_core::{Forwarder, ForwarderError};

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
#[serde(try_from = "String")]
pub struct Namespace(String);

impl Namespace {
    pub fn new(value: impl Into<String>) -> Result<Self, &'static str> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err("namespace must not be empty");
        }
        Ok(Self(value))
    }

    pub fn value(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for Namespace {
    type Error = &'static str;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
#[serde(try_from = "String")]
pub struct ServiceName(String);

impl ServiceName {
    pub fn new(value: impl Into<String>) -> Result<Self, &'static str> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err("service_name must not be empty");
        }
        Ok(Self(value))
    }

    pub fn value(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for ServiceName {
    type Error = &'static str;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(try_from = "u16")]
pub struct RemotePort(u16);

impl RemotePort {
    pub fn new(value: usize) -> Result<Self, &'static str> {
        if value == 0 || value > u16::MAX as usize {
            return Err("remote_port must be between 1 and 65535");
        }
        Ok(Self(value as u16))
    }

    pub fn value(&self) -> u16 {
        self.0
    }
}

impl TryFrom<u16> for RemotePort {
    type Error = &'static str;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        Self::new(value as usize)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
pub struct LocalPort(u16);

impl LocalPort {
    pub fn new(value: usize) -> Result<Self, &'static str> {
        if value > u16::MAX as usize {
            return Err("local_port must be between 0 and 65535");
        }
        Ok(Self(value as u16))
    }

    pub fn value(&self) -> u16 {
        self.0
    }
}

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
