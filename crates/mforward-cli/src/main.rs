mod forwarders;

use std::path::PathBuf;

use anyhow::Context;
use clap::Parser;
use forwarders::Forwarders;
use mforward_config::{Config, ForwarderFactory};

#[derive(Parser)]
#[command(
    name = "mforward",
    version,
    about = "Run port forwarders from a TOML configuration"
)]
struct Cli {
    #[arg(default_value = "mforward.toml", value_name = "CONFIG")]
    config: PathBuf,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with_writer(std::io::stderr)
        .init();

    let config = Config::read(cli.config)?;
    let mut forwarders = Forwarders::default();
    for forwarder in ForwarderFactory::create(config) {
        forwarders.start(forwarder, ());
    }

    let result = forwarders.wait().await;
    if let Err(ref error) = result {
        tracing::error!(%error, "Forwarding failed");
    }
    result.context("Forwarding failed")
}
