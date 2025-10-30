use serde::{Deserialize, Serialize};

use crate::provider::ProviderKind;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct ServerHandle {
    pub provider: ProviderKind,
    pub config: String, // name of the config used to create this server
    pub id: String,
    pub host: String,
    pub port: u16,
    pub username: String,
}

impl std::fmt::Display for ServerHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.host, self.provider.to_string())
    }
}
