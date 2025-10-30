use clap::Subcommand;

pub mod delete;
pub mod edit;
pub mod list;
pub mod show;

#[derive(Subcommand, Debug)]
pub enum ConfigCmd {
    #[command(name = "list")]
    List,
    #[command(name = "show")]
    Show {
        #[arg(long = "name")]
        name: Option<String>,
        #[arg(long = "index")]
        index: Option<usize>,
    },
    #[command(name = "delete")]
    Delete {
        #[arg(long = "name")]
        name: Option<String>,
        #[arg(long = "index")]
        index: Option<usize>,
    },
    #[command(name = "edit")]
    Edit,
}
