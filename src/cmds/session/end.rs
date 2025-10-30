use anyhow::Result;
use inquire::Select;

use crate::{config::SavedConfigs, provider::get_provider, state::State};

pub fn end_session() -> Result<()> {
    let mut st = State::load().unwrap_or_default();
    let config = SavedConfigs::load().unwrap();

    let selected = Select::new("Select Session to end: ", st.projects.clone()).prompt()?;

    let cloud_config = config.get(&selected.config).unwrap();

    let provider = get_provider(&cloud_config)?;

    provider.delete(&selected)?;

    st.projects.retain(|e| e.id != selected.id);

    st.save().unwrap();
    Ok(())
}
