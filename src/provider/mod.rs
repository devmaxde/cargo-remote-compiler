use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProviderKind {
    Hetzner,
}

impl ProviderKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            ProviderKind::Hetzner => "hetzner",
        }
    }
}

pub const SUPPORTED_PROVIDERS: &[ProviderKind] = &[ProviderKind::Hetzner];

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerHandle {
    pub provider: ProviderKind,
    pub id: String,
    pub host: String,
    pub port: u16,
    pub username: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HetznerConfig {
    pub api_key: String,
    pub location: String,
    pub server_type: String,
    pub image: String,
    pub username: Option<String>,
    pub ssh_key: String,
}

pub trait Provider {
    fn rent(&self, project_key: &str, preinstall: &[String]) -> Result<ServerHandle>;
    fn delete(&self, handle: &ServerHandle) -> Result<()>;
    fn exists(&self, handle: &ServerHandle) -> Result<bool>;
}

pub mod hetzner;

use crate::config::SavedConfig;
use hetzner::HetznerProvider;

pub fn get_provider(c: &SavedConfig) -> Result<Box<dyn Provider>> {
    match c.provider {
        ProviderKind::Hetzner => {
            let h = c
                .hetzner
                .clone()
                .ok_or_else(|| anyhow!("missing hetzner config"))?;
            Ok(Box::new(HetznerProvider { cfg: h }))
        }
    }
}

pub fn provider_exists(c: &SavedConfig, h: &ServerHandle) -> Result<bool> {
    get_provider(c)?.exists(h)
}
