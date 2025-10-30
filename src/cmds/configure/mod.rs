use anyhow::Result;
use inquire::{validator::ValueRequiredValidator, Confirm, Select, Text};

use crate::config::{mode::MODE_VARIANTS, SavedConfigs, PRIORITY_VARIANTS};

pub mod manual;

pub fn configure_wizard() -> Result<()> {
    let mut cfgs = SavedConfigs::load().unwrap_or_default();

    println!("General config:");
    if cfgs.priority.is_none() {
        let prio_label = Select::new(
            "Which Server should we use, if muliple options exists? ",
            PRIORITY_VARIANTS.to_vec(),
        )
        .prompt()?;

        cfgs.priority = Some(prio_label);
        println!();
    }
    let mode_label = Select::new(
        "What kind of remote Server do you want to use?:",
        MODE_VARIANTS.to_vec(),
    )
    .prompt()?;
    let name = Text::new("Config name:")
        .with_validator(ValueRequiredValidator::default())
        .prompt()?;

    println!();

    let config = mode_label.run_wizzard(name)?;

    cfgs.upsert(config);
    cfgs.save()?;

    if cfgs.items.len() >= 2 && cfgs.default.is_none() {
        let set_new = Confirm::new(
            "You havn't configured a default configuration. Do you want to do it now? (y/n)",
        )
        .prompt()?;
        if set_new {
            let selected = Select::new("Please choose one: ", cfgs.items.clone()).prompt()?;

            cfgs.default = Some(selected.name().to_string());
            cfgs.save()?;
        }
    }

    println!("Configured");
    Ok(())
}
