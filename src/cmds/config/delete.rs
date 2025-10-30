use anyhow::{anyhow, Result};

use crate::config::SavedConfigs;

pub fn config_delete(name: Option<String>, index: Option<usize>) -> Result<()> {
    let mut cfgs = SavedConfigs::load().unwrap_or_default();
    let pos = if let Some(n) = name {
        cfgs.items
            .iter()
            .position(|c| c.name() == n)
            .ok_or_else(|| anyhow!("not found"))?
    } else if let Some(i) = index {
        if i < cfgs.items.len() {
            i
        } else {
            return Err(anyhow!("index"));
        }
    } else {
        return Err(anyhow!("provide name or index"));
    };
    let removed = cfgs.items.remove(pos);
    if cfgs.default.as_deref() == Some(removed.name()) {
        cfgs.default = None;
    }
    cfgs.save()?;
    println!("Deleted");
    Ok(())
}
