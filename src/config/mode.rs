use std::fmt;

use enum_dispatch::enum_dispatch;
use serde::{Deserialize, Serialize};

use crate::{config::ManualConfig, provider::hetzner::config::HetznerConfig};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Manual,
    Hetzner,
}

pub const MODE_VARIANTS: [Mode; 2] = [Mode::Manual, Mode::Hetzner];

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let out = match self {
            Mode::Manual => "Manual",
            Mode::Hetzner => "Hetzner",
        };

        write!(f, "{}", out)
    }
}

impl Mode {
    // This function exists, so that we can modify this at a central place later
    pub fn check_cloud_mode(mode: &Mode) -> bool {
        !matches!(mode, Mode::Manual)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
#[enum_dispatch(CloudConfig)]
pub enum ConfigData {
    Manual(ManualConfig),
    Hetzner(HetznerConfig),
}
impl fmt::Display for ConfigData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigData::Manual(manual_config) => write!(f, "{}", manual_config),
            ConfigData::Hetzner(hetzner_config) => write!(f, "{}", hetzner_config),
        }
    }
}
