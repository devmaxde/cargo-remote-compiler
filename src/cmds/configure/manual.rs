use anyhow::{anyhow, Result};
use inquire::Text;

use crate::config::{
    mode::{ConfigData, Mode},
    ManualConfig, SavedConfig,
};

pub fn manual_wizzard(name: String) -> Result<SavedConfig> {
    println!("Manual configuration: ");
    let host = Text::new("Which Url/Ip has the Server? ").prompt()?;
    let port: u16 = Text::new("On which port is ssh running? ")
        .with_initial_value("22")
        .prompt()?
        .parse()
        .map_err(|_| anyhow!("invalid port"))?;

    let user = Text::new("What is the Username? ").prompt()?;
    println!();
    println!("For security reasons we only Support Public Key Authentification!");

    let pubk = Text::new("SSH public key path (absolute): ").prompt()?;

    let parts = pubk.split(".pub").collect::<Vec<&str>>();
    let mut def = String::new();
    if parts.len() == 2 {
        def = parts[0].to_string();
    }

    let privk = Text::new("SSH private key path (absolute): ")
        .with_initial_value(&def)
        .prompt()?;

    Ok(SavedConfig {
        mode: Mode::Manual,
        data: ConfigData::Manual(ManualConfig {
            name,
            user,
            host,
            port,
            ssh_public_key_path: pubk,
            ssh_private_key_path: privk,
        }),
    })
}
