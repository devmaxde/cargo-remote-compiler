use crate::cmds::configure::manual::manual_wizzard;
use crate::config::ManualConfig;
use crate::config::{mode::ConfigData, mode::Mode, SavedConfig};
use crate::provider::handle::ServerHandle;
use crate::provider::hetzner::config::{hetzner_config_wizzard, HetznerConfig};
use anyhow::{anyhow, Result};
use enum_dispatch::enum_dispatch;
use hetzner::HetznerProvider;
use serde::{Deserialize, Serialize};

pub mod handle;
pub mod hetzner;

pub trait Provider {
    fn rent(&self, project_key: &str, preinstall: &[String]) -> Result<ServerHandle>;
    fn delete(&self, handle: &ServerHandle) -> Result<()>;
    fn exists(&self, handle: &ServerHandle) -> Result<bool>;
}

#[enum_dispatch]
pub trait CloudConfig {
    fn name(&self) -> &str;

    fn private_key_path(&self) -> String;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProviderKind {
    Hetzner,
}

#[allow(clippy::to_string_trait_impl)]
impl ToString for ProviderKind {
    fn to_string(&self) -> String {
        match self {
            ProviderKind::Hetzner => "Hetzner",
        }
        .to_string()
    }
}

impl Mode {
    pub fn run_wizzard(&self, name: String) -> Result<SavedConfig> {
        match self {
            Mode::Manual => manual_wizzard(name),
            Mode::Hetzner => hetzner_config_wizzard(name),
        }
    }
}

pub fn get_provider(c: &SavedConfig) -> Result<Box<dyn Provider>> {
    match (&c.mode, &c.data) {
        (Mode::Hetzner, ConfigData::Hetzner(h)) => Ok(Box::new(HetznerProvider { cfg: h.clone() })),
        _ => Err(anyhow!("unsupported provider for this config")),
    }
}

pub fn provider_exists(c: &SavedConfig, h: &ServerHandle) -> Result<bool> {
    get_provider(c)?.exists(h)
}
