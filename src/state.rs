use crate::provider::ServerHandle;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct State {
    pub projects: HashMap<String, ServerHandle>,
}

impl State {
    pub fn path() -> Result<std::path::PathBuf> {
        let x = xdg::BaseDirectories::with_prefix("cargo-remote");
        x.place_data_file("state.json").context("place")
    }

    pub fn load() -> Result<Self> {
        let p = Self::path()?;
        if p.is_file() {
            let s = std::fs::read_to_string(&p).context("read state")?;
            Ok(serde_json::from_str(&s).context("json")?)
        } else {
            Ok(State::default())
        }
    }

    pub fn save(&self) -> Result<()> {
        let p = Self::path()?;
        let s = serde_json::to_string_pretty(self).context("json")?;
        std::fs::write(p, s).context("write state")?;
        Ok(())
    }
}
