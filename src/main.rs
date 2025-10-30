use clap::{Args, Parser, Subcommand};
use log::{error, LevelFilter};
use simple_logger::SimpleLogger;
use std::path::PathBuf;
use std::process::exit;

mod cmds;
mod config;
mod core;
mod provider;
mod state;

use crate::cmds::config::delete::config_delete;
use crate::cmds::config::edit::config_edit;
use crate::cmds::config::list::config_list;
use crate::cmds::config::show::config_show;
use crate::cmds::config::ConfigCmd;
use crate::cmds::configure::configure_wizard;
use crate::cmds::remote::build::cmd_build;
use crate::cmds::remote::clean::cmd_clean;
use crate::cmds::remote::run::cmd_run;
use crate::cmds::session::begin::begin_session;
use crate::cmds::session::end::end_session;

#[derive(Args, Debug)]
pub struct BeginOpts {
    #[arg(long = "config")]
    /// The config name, that should be used to rent a Server. Otherwise the default will be used
    config: Option<String>,

    #[arg(long = "preinstall", value_delimiter = ',')]
    preinstall: Vec<String>,
}

#[derive(Args, Debug)]
pub struct ExecOpts {
    #[arg(short = 'b', long = "build-env", default_value = "RUST_BACKTRACE=1")]
    /// Provide a builld-environment. For example RUST_BACKTRACE=1
    build_env: String,

    #[arg(short = 'd', long = "rustup-default", default_value = "stable")]
    /// Rustip default channel (eg. stable, nightly)
    rustup_default: String,

    #[arg(short = 'c', long = "copy-back")]
    /// Copy back the target folder after running / compiling
    copy_back: Option<String>,

    #[arg(long = "no-copy-lock")]
    /// If set, Cargo.lock wont be copied back
    no_copy_lock: bool,

    #[arg(long = "manifest-path", default_value = "Cargo.toml")]
    /// Manifest path (default Cargo.toml)
    manifest_path: PathBuf,

    #[arg(long = "transfer-hidden")]
    /// Transfer hidden files (eg. .env .gitignore)
    hidden: bool,
}

#[derive(Subcommand, Debug)]
enum RemoteCmd {
    #[command(name = "configure")]
    /// Cli configuration wizard
    Configure,

    #[command(name = "config")]
    /// Subcommand to manage different Configuration
    Config {
        #[command(subcommand)]
        cmd: ConfigCmd,
    },

    #[command(name = "begin")]
    /// Rents and configures a Cloud Server
    Begin {
        #[command(flatten)]
        begin: BeginOpts,
    },

    #[command(name = "end")]
    /// Deletes a rented Cloud Server
    End,

    #[command(name = "status")]
    /// Shows the Status of the Cloud Server
    Status,

    #[command(name = "run")]
    /// Runs the application on a remote Host (Manually configured / Cloud Server)
    Run {
        #[command(flatten)]
        exec: ExecOpts,
        options: Vec<String>,
    },

    #[command(name = "build")]

    /// Builds the application on a remote Host (Manually configured / Cloud Server)
    Build {
        #[command(flatten)]
        exec: ExecOpts,
        options: Vec<String>,
    },

    #[command(name = "clean")]
    /// Cleans remote target folder on the remote Host (Manually configured / Cloud Server)
    Clean {
        #[command(flatten)]
        exec: ExecOpts,
        options: Vec<String>,
    },
}

#[derive(Parser, Debug)]
#[command(name = "cargo-remote", bin_name = "cargo")]
enum CargoCli {
    #[command(name = "remote", subcommand)]
    Remote(RemoteCmd),
}

#[derive(Clone)]
struct SessionRemote {
    project_dir: PathBuf,
    server_ssh: String,
    ssh_key: PathBuf,
    ssh_port: u16,
    build_path: String,
    build_env: String,
    rustup_default: String,
    copy_back: Option<String>,
    no_copy_lock: bool,
    hidden: bool,
    command: String,
    options: Vec<String>,
}

fn main() {
    // Default log level is Info. RUST_ENV will override this
    SimpleLogger::new()
        .with_level(LevelFilter::Info)
        .env()
        .init()
        .unwrap();

    match CargoCli::parse() {
        CargoCli::Remote(cmd) => match cmd {
            RemoteCmd::Configure => {
                if let Err(e) = configure_wizard() {
                    error!("{}", e);
                    exit(2)
                }
            }
            RemoteCmd::Config { cmd } => match cmd {
                ConfigCmd::List => {
                    if let Err(e) = config_list() {
                        error!("{}", e);
                        exit(2)
                    }
                }
                ConfigCmd::Show { name, index } => {
                    if let Err(e) = config_show(name, index) {
                        error!("{}", e);
                        exit(2)
                    }
                }
                ConfigCmd::Delete { name, index } => {
                    if let Err(e) = config_delete(name, index) {
                        error!("{}", e);
                        exit(2)
                    }
                }
                ConfigCmd::Edit => {
                    if let Err(e) = config_edit() {
                        error!("{}", e);
                        exit(2)
                    }
                }
            },
            RemoteCmd::Begin { begin } => {
                if let Err(e) = begin_session(begin) {
                    error!("{}", e);
                    exit(3)
                }
            }
            RemoteCmd::End => {
                if let Err(e) = end_session() {
                    error!("{}", e);
                    exit(3)
                }
            }
            RemoteCmd::Status => {
                if let Err(e) = cmds::status::status() {
                    error!("{}", e);
                    exit(3)
                }
            }
            RemoteCmd::Run { exec, options } => cmd_run(exec, options),
            RemoteCmd::Build { exec, options } => cmd_build(exec, options),
            RemoteCmd::Clean { exec, options } => cmd_clean(exec, options),
        },
    }
}
