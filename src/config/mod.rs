use crate::config::mode::{ConfigData, Mode};
use crate::provider::handle::ServerHandle;
use crate::provider::CloudConfig;
use inquire::Select;
use std::path::PathBuf;
use std::{collections::HashMap, fmt};

use serde::{Deserialize, Serialize};

use crate::state::State;
use anyhow::anyhow;

pub mod mode;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    Cloud,  // prefer cloud providers
    Manual, // prefer manual servers
    Ask,    // always ask on operations that support it
}

impl fmt::Display for Priority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Priority::Cloud => "Cloud",
            Priority::Manual => "Manual",
            Priority::Ask => "Ask",
        };
        write!(f, "{}", name)
    }
}

pub const PRIORITY_VARIANTS: [Priority; 3] = [Priority::Manual, Priority::Cloud, Priority::Ask];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedConfig {
    pub mode: Mode,
    #[serde(flatten)]
    pub data: ConfigData,
}

impl fmt::Display for SavedConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.mode, self.data)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManualConfig {
    pub name: String,
    pub user: String,
    pub host: String,
    pub port: u16,
    pub ssh_public_key_path: String,
    pub ssh_private_key_path: String,
}

impl CloudConfig for ManualConfig {
    fn name(&self) -> &str {
        &self.name
    }

    fn private_key_path(&self) -> String {
        todo!()
    }
}

impl fmt::Display for ManualConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SSHConfig(){}@{}:{})", self.user, self.host, self.port)
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SavedConfigs {
    pub default: Option<String>,
    #[serde(default)]
    pub priority: Option<Priority>,
    pub items: Vec<SavedConfig>,
}

impl SavedConfig {
    pub fn name(&self) -> &str {
        self.data.name()
    }
    pub fn private_key_path(&self) -> String {
        self.data.private_key_path()
    }
}

impl SavedConfigs {
    pub fn path() -> anyhow::Result<std::path::PathBuf> {
        let x = xdg::BaseDirectories::with_prefix("cargo-remote");
        Ok(x.place_config_file("config.toml")?)
    }
    pub fn load() -> anyhow::Result<Self> {
        let p = Self::path()?;
        if p.is_file() {
            Ok(toml::from_str(&std::fs::read_to_string(&p)?)?)
        } else {
            Ok(Self::default())
        }
    }
    pub fn save(&self) -> anyhow::Result<()> {
        let p = Self::path()?;
        let s = toml::to_string_pretty(self)?;
        std::fs::create_dir_all(p.parent().unwrap())?;
        std::fs::write(p, s)?;
        Ok(())
    }
    pub fn get(&self, name: &str) -> Option<SavedConfig> {
        self.items.iter().find(|c| c.name() == name).cloned()
    }
    pub fn upsert(&mut self, cfg: SavedConfig) {
        let name = cfg.name().to_string();
        if let Some(i) = self.items.iter().position(|c| c.name() == name) {
            self.items[i] = cfg;
        } else {
            self.items.push(cfg);
        }
    }

    pub fn has_any_cloud(&self) -> bool {
        self.items.iter().any(|c| matches!(c.mode, Mode::Hetzner))
    }

    /// Selects a Remote Host and returns Host, User, Port, PrivatKeyPath
    pub fn select_remote_host(&self) -> anyhow::Result<(String, String, u16, PathBuf)> {
        let priority = self.priority.clone().unwrap_or(Priority::Ask);

        let items_by_name: HashMap<String, SavedConfig> = self
            .items
            .iter()
            .cloned()
            .map(|c| (c.name().to_string(), c))
            .collect();

        let state = State::load().unwrap_or_default();
        let mut cloud: Vec<(ServerHandle, SavedConfig)> = state
            .projects
            .into_iter()
            .filter_map(|h| {
                items_by_name
                    .get(&h.config)
                    .cloned()
                    .and_then(|cfg| match (&cfg.mode, &cfg.data) {
                        (Mode::Hetzner, ConfigData::Hetzner(_)) => Some((h, cfg)),
                        _ => None,
                    })
            })
            .collect();

        let mut manual: Vec<SavedConfig> = self
            .items
            .iter()
            .filter(|c| matches!(c.mode, Mode::Manual))
            .cloned()
            .collect();

        enum Selection {
            Manual(SavedConfig),
            Cloud((ServerHandle, SavedConfig)),
        }

        let pick = match priority {
            Priority::Cloud => {
                if cloud.is_empty() {
                    return Err(anyhow!(
                        "no active cloud servers; start one with `cargo remote begin`"
                    ));
                }
                if cloud.len() == 1 {
                    Selection::Cloud(cloud.remove(0))
                } else {
                    let choices: Vec<String> = cloud
                        .iter()
                        .map(|(h, cfg)| {
                            format!(
                                "{} [{} {}:{} id={}]",
                                cfg.name(),
                                h.provider.to_string(),
                                h.host,
                                h.port,
                                h.id
                            )
                        })
                        .collect();
                    let selected = Select::new("Select cloud server", choices).prompt()?;
                    let sel_name = selected
                        .split_once(" [")
                        .map(|(n, _)| n.to_string())
                        .unwrap_or(selected);
                    let idx = cloud
                        .iter()
                        .position(|(_, cfg)| cfg.name() == sel_name.as_str())
                        .ok_or_else(|| anyhow!("selection not found"))?;
                    Selection::Cloud(cloud.remove(idx))
                }
            }
            Priority::Manual => {
                if manual.is_empty() {
                    return Err(anyhow!(
                        "no manual server configured; run `cargo remote configure`"
                    ));
                }
                if manual.len() == 1 {
                    Selection::Manual(manual.remove(0))
                } else {
                    let choices: Vec<String> = manual
                        .iter()
                        .map(|c| match &c.data {
                            ConfigData::Manual(m) => {
                                format!("{} [manual {}:{}]", c.name(), m.host, m.port)
                            }
                            _ => c.name().to_string(),
                        })
                        .collect();
                    let selected = Select::new("Select manual configuration", choices).prompt()?;
                    let sel_name = selected
                        .split_once(" [")
                        .map(|(n, _)| n.to_string())
                        .unwrap_or(selected);
                    let idx = manual
                        .iter()
                        .position(|c| c.name() == sel_name.as_str())
                        .ok_or_else(|| anyhow!("selection not found"))?;
                    Selection::Manual(manual.remove(idx))
                }
            }
            Priority::Ask => {
                if cloud.is_empty() && manual.is_empty() {
                    return Err(anyhow!(
                        "no configurations available; run `cargo remote configure`"
                    ));
                }
                if cloud.len() + manual.len() == 1 {
                    if !cloud.is_empty() {
                        Selection::Cloud(cloud.remove(0))
                    } else {
                        Selection::Manual(manual.remove(0))
                    }
                } else {
                    let mut choices: Vec<String> = Vec::new();
                    for (h, cfg) in cloud.iter() {
                        choices.push(format!(
                            "{} [{} {}:{} id={}]",
                            cfg.name(),
                            h.provider.to_string(),
                            h.host,
                            h.port,
                            h.id
                        ));
                    }
                    for c in manual.iter() {
                        choices.push(match &c.data {
                            ConfigData::Manual(m) => {
                                format!("{} [manual {}:{}]", c.name(), m.host, m.port)
                            }
                            _ => c.name().to_string(),
                        });
                    }

                    let selected = Select::new("Select configuration", choices).prompt()?;
                    let sel_name = selected
                        .split_once(" [")
                        .map(|(n, _)| n.to_string())
                        .unwrap_or(selected);

                    if let Some(i) = cloud
                        .iter()
                        .position(|(_, cfg)| cfg.name() == sel_name.as_str())
                    {
                        Selection::Cloud(cloud.remove(i))
                    } else if let Some(i) =
                        manual.iter().position(|c| c.name() == sel_name.as_str())
                    {
                        Selection::Manual(manual.remove(i))
                    } else {
                        return Err(anyhow!("selection not found"));
                    }
                }
            }
        };

        match pick {
            Selection::Manual(cfg) => match &cfg.data {
                ConfigData::Manual(m) => {
                    let host = m.host.clone();
                    let user = m.user.clone();
                    let port = m.port;
                    let privk = PathBuf::from(&m.ssh_private_key_path);
                    Ok((host, user, port, privk))
                }
                _ => Err(anyhow!("invalid manual config")),
            },
            Selection::Cloud((h, cfg)) => {
                let host = h.host.clone();
                let user = h.username.clone();
                let port = h.port;
                let privk = PathBuf::from(cfg.data.private_key_path());

                Ok((host, user, port, privk))
            }
        }
    }
}
