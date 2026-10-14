use mforward_core::{Forwarder, ForwarderError};
use thiserror::Error;
use tokio::process::Command;
pub mod types;

pub use types::{InstanceName, LocalPort, Project, RemotePort, Zone};

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

fn command(args: &GcpIapArgs) -> (&'static str, Vec<String>) {
    (
        "gcloud",
        vec![
            "compute".to_owned(),
            "start-iap-tunnel".to_owned(),
            args.instance_name.value().to_owned(),
            args.remote_port.value().to_string(),
            format!("--project={}", args.project.value()),
            format!("--zone={}", args.zone.value()),
            format!("--local-host-port=localhost:{}", args.local_port.value()),
        ],
    )
}

impl Forwarder for GcpIap {
    type Args = GcpIapArgs;

    #[tracing::instrument(fields(project= args.project.value()), skip_all)]
    async fn start(&self, args: &Self::Args) -> Result<(), ForwarderError> {
        tracing::info!("Starting GCP IAP tunnel");
        let (program, command_args) = command(args);
        let status = Command::new(program)
            .args(command_args)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_gcloud_iap_tunnel_command() {
        let args = GcpIapArgs::new("project-a", "europe-west1-b", "database", 5432, 0).unwrap();

        assert_eq!(
            command(&args),
            (
                "gcloud",
                vec![
                    "compute",
                    "start-iap-tunnel",
                    "database",
                    "5432",
                    "--project=project-a",
                    "--zone=europe-west1-b",
                    "--local-host-port=localhost:0"
                ]
                .into_iter()
                .map(str::to_owned)
                .collect()
            )
        );
    }
}
