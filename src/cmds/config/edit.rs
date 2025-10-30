use anyhow::Result;
use std::process::Command;

use crate::config::SavedConfigs;

pub fn config_edit() -> Result<()> {
    let p = SavedConfigs::path()?;
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".into());
    let status = Command::new(editor).arg(&p).status()?;
    if !status.success() {
        println!("File: {}", p.display());
    }
    Ok(())
}
