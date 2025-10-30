use anyhow::{anyhow, Result};
use inquire::Select;
use std::path::PathBuf;

use crate::config::mode::Mode;
use crate::config::SavedConfigs;
use crate::core::{metadata_dir, project_key_from_dir};
use crate::provider::get_provider;
use crate::state::State;
use crate::BeginOpts;

pub fn begin_session(begin: BeginOpts) -> Result<()> {
    let project_dir = metadata_dir(PathBuf::from("Cargo.toml"))?;
    let key = project_key_from_dir(&project_dir);

    let cfgs = SavedConfigs::load().unwrap_or_default();
    if !cfgs.has_any_cloud() {
        return Err(anyhow!(
            "no cloud provider configured; add one via `cargo remote configure`"
        ));
    }

    let name = begin.config.clone().or(cfgs.default.clone());

    let mut c = None;

    // Trying to load based on the provided name / default value
    if let Some(name) = name {
        let tmp = cfgs.get(&name);
        if let Some(config) = tmp {
            if Mode::check_cloud_mode(&config.mode) {
                c = Some(config);
            }
        }
    }
    // If that didn't work --> Manual user selection
    if c.is_none() {
        let mut configs = cfgs.items.clone();
        configs.retain(|f| Mode::check_cloud_mode(&f.mode));

        if configs.is_empty() {
            eprintln!("Could not find a cloud configuration. add one via `cargo remote configure`");
            return Err(anyhow!("Invalid begin config"));
        }

        if configs.len() == 1 {
            c = Some(configs[0].clone());
        } else {
            let selected = Select::new(
            "We could not find a matching config based on your input/defaulty. Please choose one: ",
            configs,
        )
        .prompt()?;

            c = Some(selected)
        }
    };

    let Some(c) = c else {
        panic!("This should never be reached. How did you do that?")
    };

    if !Mode::check_cloud_mode(&c.mode) {
        return Err(anyhow!(
            "`begin` requires a cloud provider config (e.g., Hetzner)"
        ));
    }

    let privk = c.private_key_path();
    if !PathBuf::from(&privk).is_file() {
        return Err(anyhow!("private key missing at {}", privk));
    }

    let provider = get_provider(&c)?;
    let handle = provider.rent(&key, &begin.preinstall)?;
    println!(
        "Server is starting. This may take a few minutes (installing Rust and other Dependencies)"
    );

    let mut st = State::load().unwrap_or_default();
    st.projects.push(handle);
    st.save()?;
    Ok(())
}
