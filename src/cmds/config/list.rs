use anyhow::Result;

use crate::config::{mode::Mode, SavedConfigs};

pub fn config_list() -> Result<()> {
    let cfgs = SavedConfigs::load().unwrap_or_default();
    if cfgs.items.is_empty() {
        println!("Cargo remote isn't configured. Use `cargo remote configure`");
        return Ok(());
    }

    for (i, c) in cfgs.items.iter().enumerate() {
        let d = if cfgs.default.as_deref() == Some(c.name()) {
            "*"
        } else {
            " "
        };
        let mode = match c.mode {
            Mode::Manual => "manual",
            Mode::Hetzner => "hetzner",
        };
        println!("{} [{}] {} {}", i, d, c.name(), mode);
    }
    Ok(())
}
