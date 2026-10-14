use mforward_core::{Forwarder, ForwarderError};
use mforward_gcp_iap::{GcpIap, GcpIapArgs};
use mforward_kubectl::{Kubectl, KubectlArgs};

use crate::{Config, ForwarderConfig};

pub struct ForwarderFactory;

impl ForwarderFactory {
    pub fn create(config: Config) -> impl IntoIterator<Item = ConfiguredForwarder> {
        config.forwarders.into_iter().map(Self::configure)
    }

    fn configure(config: ForwarderConfig) -> ConfiguredForwarder {
        let provider = match config {
            ForwarderConfig::Kubectl(args) => Provider::Kubectl(Kubectl, args),
            ForwarderConfig::GcpIap(args) => Provider::GcpIap(GcpIap, args),
        };
        ConfiguredForwarder { provider }
    }
}

pub struct ConfiguredForwarder {
    provider: Provider,
}

impl ConfiguredForwarder {
    pub fn name(&self) -> &'static str {
        match &self.provider {
            Provider::Kubectl(_, _) => "kubectl",
            Provider::GcpIap(_, _) => "gcp-iap",
        }
    }
}

enum Provider {
    Kubectl(Kubectl, KubectlArgs),
    GcpIap(GcpIap, GcpIapArgs),
}

impl Forwarder for ConfiguredForwarder {
    type Args = ();

    async fn start(&self, _: &Self::Args) -> Result<(), ForwarderError> {
        match &self.provider {
            Provider::Kubectl(forwarder, args) => forwarder.start(args).await,
            Provider::GcpIap(forwarder, args) => forwarder.start(args).await,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_matching_providers_in_configuration_order() {
        let config = Config::from_toml(include_str!("../../../mforward.example.toml")).unwrap();
        let forwarders = ForwarderFactory::create(config)
            .into_iter()
            .collect::<Vec<_>>();
        assert_eq!(forwarders.len(), 2);
        assert_eq!(forwarders[0].name(), "kubectl");
        assert_eq!(forwarders[1].name(), "gcp-iap");
        assert!(matches!(forwarders[0].provider, Provider::Kubectl(_, _)));
        assert!(matches!(forwarders[1].provider, Provider::GcpIap(_, _)));
    }
}
