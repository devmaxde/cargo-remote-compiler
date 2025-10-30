use crate::provider::{HetznerConfig, ProviderKind};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedConfig {
    pub name: String,
    pub provider: ProviderKind,
    pub ssh_public_key_path: String,
    pub ssh_private_key_path: String,
    pub hetzner: Option<HetznerConfig>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SavedConfigs {
    pub default: Option<String>,
    pub items: Vec<SavedConfig>,
}

impl SavedConfigs {
    pub fn path() -> anyhow::Result<std::path::PathBuf> {
        let x = xdg::BaseDirectories::with_prefix("cargo-remote");
        Ok(x.place_config_file("configs.json")?)
    }
    pub fn load() -> anyhow::Result<Self> {
        let p = Self::path()?;
        if p.is_file() {
            Ok(serde_json::from_str(&std::fs::read_to_string(&p)?)?)
        } else {
            Ok(Self::default())
        }
    }
    pub fn save(&self) -> anyhow::Result<()> {
        let p = Self::path()?;
        let s = serde_json::to_string_pretty(self)?;
        std::fs::create_dir_all(p.parent().unwrap())?;
        std::fs::write(p, s)?;
        Ok(())
    }
    pub fn get(&self, name: &str) -> Option<SavedConfig> {
        self.items.iter().find(|c| c.name == name).cloned()
    }
}
