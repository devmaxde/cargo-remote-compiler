use anyhow::{anyhow, Result};

use crate::config::SavedConfigs;

pub fn config_show(name: Option<String>, index: Option<usize>) -> Result<()> {
    println!("Path: {:?}", SavedConfigs::path().unwrap());
    let cfgs = SavedConfigs::load().unwrap_or_default();

    if cfgs.items.is_empty() {
        println!("No config found!");
        return Ok(());
    }

    if cfgs.items.len() == 1 {
        println!("{}", toml::to_string_pretty(&cfgs.items[0])?);
        return Ok(());
    }

    let c = if let Some(n) = name {
        cfgs.get(&n).ok_or_else(|| anyhow!("not found"))?
    } else if let Some(i) = index {
        cfgs.items.get(i).cloned().ok_or_else(|| anyhow!("index"))?
    } else {
        return Err(anyhow!("provide name or index"));
    };
    println!("{}", toml::to_string_pretty(&c)?);
    Ok(())
}
