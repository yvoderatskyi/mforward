mod factory;

pub use factory::{ConfiguredForwarder, ForwarderFactory};

use std::path::{Path, PathBuf};

use mforward_gcp_iap::GcpIapArgs;
use mforward_kubectl::KubectlArgs;
use serde::Deserialize;

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("cannot read config {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("invalid TOML configuration: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("configuration must contain at least one forwarder")]
    Empty,
    #[error("forwarder {index}: {reason}")]
    InvalidArgs { index: usize, reason: &'static str },
}

#[derive(Debug)]
pub struct Config {
    forwarders: Vec<ForwarderConfig>,
}

impl Config {
    fn with_forwarders(
        forwarders: impl IntoIterator<Item = ForwarderConfig>,
    ) -> Result<Self, ConfigError> {
        let forwarders: Vec<_> = forwarders.into_iter().collect();
        if forwarders.is_empty() {
            return Err(ConfigError::Empty);
        }
        Ok(Self { forwarders })
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ConfigFile {
    #[serde(default)]
    defaults: Defaults,
    forwarders: Vec<toml::Table>,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
struct Defaults {
    #[serde(default)]
    kubectl: toml::Table,
    #[serde(default)]
    gcp_iap: toml::Table,
}

impl Defaults {
    fn apply(&self, mut entry: toml::Table) -> toml::Table {
        let defaults = match entry.get("type").and_then(toml::Value::as_str) {
            Some("kubectl") => &self.kubectl,
            Some("gcp-iap") => &self.gcp_iap,
            _ => return entry,
        };
        for (key, value) in defaults {
            entry.entry(key.clone()).or_insert_with(|| value.clone());
        }
        entry
    }
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case", deny_unknown_fields)]
enum ForwarderConfig {
    Kubectl(KubectlArgs),
    GcpIap(GcpIapArgs),
}

impl Config {
    pub fn read(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let path = path.as_ref();
        let contents = std::fs::read_to_string(path).map_err(|source| ConfigError::Read {
            path: path.to_owned(),
            source,
        })?;
        Self::from_toml(&contents)
    }

    pub fn from_toml(contents: &str) -> Result<Self, ConfigError> {
        let file: ConfigFile = toml::from_str(contents)?;
        let forwarders = file
            .forwarders
            .into_iter()
            .map(|entry| file.defaults.apply(entry).try_into())
            .flat_map(|r| r.into_iter());
        let config = Self::with_forwarders(forwarders)?;
        for (index, forwarder) in config.forwarders.iter().enumerate() {
            let result = match forwarder {
                ForwarderConfig::Kubectl(_) => Ok(()),
                ForwarderConfig::GcpIap(_) => Ok(()),
            };
            result.map_err(|reason| ConfigError::InvalidArgs {
                index: index + 1,
                reason,
            })?;
        }
        Ok(config)
    }

    pub fn into_forwarders(self) -> impl IntoIterator<Item = ConfiguredForwarder> {
        ForwarderFactory::create(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const EXAMPLE: &str = include_str!("../../../mforward.example.toml");

    #[test]
    fn constructs_mixed_providers() {
        let forwarders = Config::from_toml(EXAMPLE).unwrap().into_forwarders();
        assert_eq!(forwarders.into_iter().collect::<Vec<_>>().len(), 2);
    }

    #[test]
    fn explicit_fields_override_only_the_matching_provider_defaults() {
        let file: ConfigFile = toml::from_str(EXAMPLE).unwrap();
        let mut entry = file.forwarders[0].clone();
        entry.insert("namespace".into(), "production".into());
        entry.insert("local_port".into(), 0.into());
        let merged = file.defaults.apply(entry);
        assert_eq!(merged["namespace"].as_str(), Some("production"));
        assert_eq!(merged["remote_port"].as_integer(), Some(8080));
        assert_eq!(merged["local_port"].as_integer(), Some(0));
        assert!(!merged.contains_key("project"));
        let _: ForwarderConfig = merged.try_into().unwrap();

        // Invalid explicit values must not fall back to defaults.
        let invalid = EXAMPLE.replace(
            "service_name = \"api\"",
            "service_name = \"api\"\nnamespace = \"\"",
        );
        assert!(Config::from_toml(&invalid).is_err());
    }

    #[test]
    fn defaults_are_optional() {
        let config = Config::from_toml(
            r#"
            [[forwarders]]
            type = "kubectl"
            namespace = "default"
            service_name = "api"
            remote_port = 8080
            local_port = 8080
        "#,
        )
        .unwrap();
        assert_eq!(
            config
                .into_forwarders()
                .into_iter()
                .collect::<Vec<_>>()
                .len(),
            1
        );
    }

    #[test]
    fn rejects_invalid_configuration() {
        for invalid in [
            "forwarders = []".to_owned(),
            EXAMPLE.replace("kubectl", "unknown"),
            EXAMPLE.replace("remote_port = 8080", "remote_port = 0"),
            EXAMPLE.replace("local_port = 8080", "local_port = 65536"),
            EXAMPLE.replace("namespace = \"default\"", "namespace = \" \""),
            EXAMPLE.replace("service_name", "service_nam"),
        ] {
            assert!(Config::from_toml(&invalid).is_err(), "accepted {invalid}");
        }
    }

    #[test]
    fn reads_configuration_file() {
        let config = Config::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../mforward.example.toml"
        ))
        .unwrap();
        assert_eq!(
            config
                .into_forwarders()
                .into_iter()
                .collect::<Vec<_>>()
                .len(),
            2
        );
    }
}
