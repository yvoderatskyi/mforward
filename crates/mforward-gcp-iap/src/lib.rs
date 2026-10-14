use mforward_core::{Forwarder, ForwarderError};
use thiserror::Error;
use tokio::process::Command;

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
#[serde(try_from = "String")]
pub struct Project(String);

impl Project {
    pub fn new(value: impl Into<String>) -> Result<Self, &'static str> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err("project must not be empty");
        }
        Ok(Self(value))
    }

    pub fn value(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for Project {
    type Error = &'static str;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
#[serde(try_from = "String")]
pub struct Zone(String);

impl Zone {
    pub fn new(value: impl Into<String>) -> Result<Self, &'static str> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err("zone must not be empty");
        }
        Ok(Self(value))
    }

    pub fn value(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for Zone {
    type Error = &'static str;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
#[serde(try_from = "String")]
pub struct InstanceName(String);

impl InstanceName {
    pub fn new(value: impl Into<String>) -> Result<Self, &'static str> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err("instance_name must not be empty");
        }
        Ok(Self(value))
    }

    pub fn value(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for InstanceName {
    type Error = &'static str;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(try_from = "usize")]
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

impl TryFrom<usize> for RemotePort {
    type Error = &'static str;
    fn try_from(value: usize) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(try_from = "usize")]
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

impl TryFrom<usize> for LocalPort {
    type Error = &'static str;
    fn try_from(value: usize) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GcpIapArgs {
    project: Project,
    zone: Zone,
    instance_name: InstanceName,
    remote_port: RemotePort,
    local_port: LocalPort,
}

impl GcpIapArgs {
    pub fn new(
        project: impl Into<String>,
        zone: impl Into<String>,
        instance_name: impl Into<String>,
        remote_port: usize,
        local_port: usize,
    ) -> Result<Self, &'static str> {
        Ok(Self {
            project: Project::new(project)?,
            zone: Zone::new(zone)?,
            instance_name: InstanceName::new(instance_name)?,
            remote_port: RemotePort::new(remote_port)?,
            local_port: LocalPort::new(local_port)?,
        })
    }
}

#[derive(Error, Debug)]
pub enum GcpIapError {
    #[error("gcloud process error: {0}")]
    ConnectionError(#[from] std::io::Error),
    #[error("gcloud tunnel exited unsuccessfully: {0:?}")]
    TunnelError(Option<i32>),
}

#[derive(Debug)]
pub struct GcpIap;

impl Forwarder for GcpIap {
    type A = GcpIapArgs;

    #[tracing::instrument(fields(project= args.project.value()), skip_all)]
    async fn start(&self, args: &Self::A) -> Result<(), ForwarderError> {
        tracing::info!("Starting GCP IAP tunnel");
        let GcpIapArgs {
            project,
            zone,
            instance_name,
            remote_port,
            local_port,
        } = args;

        let status = Command::new("gcloud")
            .args([
                "compute".to_owned(),
                "start-iap-tunnel".to_owned(),
                instance_name.value().to_owned(),
                remote_port.value().to_string(),
                format!("--project={}", project.value()),
                format!("--zone={}", zone.value()),
                format!("--local-host-port=localhost:{}", local_port.value()),
            ])
            .kill_on_drop(true)
            .status()
            .await
            .map_err(GcpIapError::from)?;

        if !status.success() {
            return Err(GcpIapError::TunnelError(status.code()).into());
        }
        tracing::info!(%status, "GCP IAP tunnel completed");
        Ok(())
    }
}
