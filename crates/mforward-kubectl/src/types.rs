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
