use std::{fmt, process::exit};

use anyhow::Result;
use inquire::{Password, Select, Text};
use serde::{Deserialize, Serialize};

use crate::{
    config::{
        mode::{ConfigData, Mode},
        SavedConfig,
    },
    provider::{hetzner::HetznerProvider, CloudConfig},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HetznerConfig {
    pub name: String,
    pub api_key: String,
    pub location: String,
    pub server_type: String,
    pub image: String,
    pub username: Option<String>,
    pub ssh_key: String,          // Hetzner key name in account
    pub local_privat_key: String, // local private key path (typo preserved)
}

impl CloudConfig for HetznerConfig {
    fn name(&self) -> &str {
        &self.name
    }

    fn private_key_path(&self) -> String {
        self.local_privat_key.clone()
    }
}

impl fmt::Display for HetznerConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "HetznerConfig(location: {}, server_type: {})",
            self.location, self.server_type
        )
    }
}

pub fn hetzner_config_wizzard(name: String) -> Result<SavedConfig> {
    let api_key = Password::new("Hetzner API key")
        .without_confirmation()
        .prompt()?;
    let hetzner_locations = HetznerProvider::get_locations(api_key.clone());

    let location = if let Ok(locations) = hetzner_locations {
        let selected = Select::new("Location: ", locations).prompt()?;
        selected.name
    } else {
        eprintln!("***** We could not query the Locations. This could have many reasons. Your Api-Key could be wrong or your Computer may not be connected to the Internet. This may cause Problems later! *****");
        Text::new("Location: ")
            .with_initial_value("nbg1")
            .prompt()?
    };

    let server_types = HetznerProvider::get_server_types(api_key.clone());
    let server_type = if let Ok(types) = server_types {
        let selected = Select::new("Server type: ", types).prompt()?;
        selected.name
    } else {
        eprintln!("***** We could not query the Server-types. This could have many reasons. Your Api-Key could be wrong or your Computer may not be connected to the Internet. This may cause Problems later! *****");

        Text::new("Server type: ")
            .with_initial_value("cpx21")
            .prompt()?
    };

    let image = Text::new("Image (This tool will use apt to install rust and other dependencies! Ubuntu recommendet):")
        .with_initial_value("ubuntu-22.04")
        .prompt()?;

    let ssh_keys_reponse = HetznerProvider::get_ssh_keys(api_key.clone());
    let ssh_key = if let Ok(keys) = ssh_keys_reponse {
        if keys.is_empty() {
            eprint!("You need to upload your SSH-Public Key to Hetzner in order to connect to the Server later on. Exitiing...");
            exit(1);
        }
        let selected = Select::new("Select your SSH-Key: ", keys).prompt()?;
        selected.name
    } else {
        Text::new("Hetzner SSH-Key name:")
            .with_initial_value("key-1")
            .prompt()?
    };

    let local_privat_key = Text::new("Local SSH Private Key Path: ").prompt()?;

    Ok(SavedConfig {
        mode: Mode::Hetzner,
        data: ConfigData::Hetzner(HetznerConfig {
            name,
            api_key,
            location,
            server_type,
            image,
            username: Some("root".to_string()),
            ssh_key,
            local_privat_key,
        }),
    })
}
