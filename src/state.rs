use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::provider::handle::ServerHandle;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct State {
    pub projects: Vec<ServerHandle>,
}

impl State {
    pub fn path() -> Result<std::path::PathBuf> {
        let x = xdg::BaseDirectories::with_prefix("cargo-remote");
        x.place_config_file("servers.toml").context("place")
    }

    pub fn load() -> Result<Self> {
        let p = Self::path()?;
        if p.is_file() {
            let s = std::fs::read_to_string(&p).context("read state")?;
            Ok(toml::from_str(&s)?)
        } else {
            Ok(State::default())
        }
    }

    pub fn save(&self) -> Result<()> {
        let p = Self::path()?;
        let s = toml::to_string_pretty(self).context("json")?;
        std::fs::write(p, s).context("write state")?;
        Ok(())
    }
}
