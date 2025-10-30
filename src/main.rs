use anyhow::{anyhow, Result};
use log::error;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::io::Write;
use std::os::unix::process::ExitStatusExt;
use std::path::PathBuf;
use std::process::{exit, Command, Stdio};
use structopt::StructOpt;

mod config;
mod provider;
mod state;
use config::SavedConfigs;
use provider::{get_provider, provider_exists, SUPPORTED_PROVIDERS};
use state::State;

#[derive(StructOpt, Debug)]
pub struct BeginOpts {
    #[structopt(long = "config")]
    config: Option<String>,
    #[structopt(long = "preinstall", use_delimiter = true)]
    preinstall: Vec<String>,
}

#[derive(StructOpt, Debug)]
pub struct ExecOpts {
    #[structopt(short = "b", long = "build-env", default_value = "RUST_BACKTRACE=1")]
    build_env: String,
    #[structopt(short = "d", long = "rustup-default", default_value = "stable")]
    rustup_default: String,
    #[structopt(short = "c", long = "copy-back")]
    copy_back: Option<String>,
    #[structopt(long = "no-copy-lock")]
    no_copy_lock: bool,
    #[structopt(
        long = "manifest-path",
        default_value = "Cargo.toml",
        parse(from_os_str)
    )]
    manifest_path: PathBuf,
    #[structopt(short = "h", long = "transfer-hidden")]
    hidden: bool,
}

#[derive(StructOpt, Debug)]
enum ConfigCmd {
    /// Lists the Cloud-configurations
    #[structopt(name = "list")]
    List,

    /// Shows you the Details of a Configuration
    #[structopt(name = "show")]
    Show {
        #[structopt(long = "name")]
        name: Option<String>,
        #[structopt(long = "index")]
        index: Option<usize>,
    },

    /// Deletes a Configuration
    #[structopt(name = "delete")]
    Delete {
        #[structopt(long = "name")]
        name: Option<String>,
        #[structopt(long = "index")]
        index: Option<usize>,
    },
    /// Edits a Configuration
    #[structopt(name = "edit")]
    Edit,
}

#[derive(StructOpt, Debug)]
enum RemoteCmd {
    /// Configure the System (Cloud-Provider, SSH-Key etc)

    #[structopt(name = "configure")]
    Configure,

    /// Subcommand for the Configuration

    #[structopt(name = "config")]
    Config {
        #[structopt(subcommand)]
        cmd: ConfigCmd,
    },
    /// Stars a compiling Session (this may take a few minutes)
    #[structopt(name = "begin")]
    Begin {
        #[structopt(flatten)]
        begin: BeginOpts,
    },
    /// End a compiling Session
    #[structopt(name = "end")]
    End,
    /// Shows the current running Sessions
    #[structopt(name = "status")]
    Status,
    /// Runs the Programm on the System
    #[structopt(name = "run")]
    Run {
        #[structopt(flatten)]
        exec: ExecOpts,
        options: Vec<String>,
    },
    /// Builds the Project on the Cloud-Server (copy_back recommendet)
    #[structopt(name = "build")]
    Build {
        #[structopt(flatten)]
        exec: ExecOpts,
        options: Vec<String>,
    },
    /// Cleans the Target folder on the Cloud-Server
    #[structopt(name = "clean")]
    Clean {
        #[structopt(flatten)]
        exec: ExecOpts,
        options: Vec<String>,
    },
}

#[derive(StructOpt, Debug)]
#[structopt(name = "cargo-remote", bin_name = "cargo")]
enum CargoCli {
    /// Utility to compile the Project on a Cloud-Server
    #[structopt(name = "remote")]
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

fn metadata_dir(manifest_path: PathBuf) -> Result<PathBuf> {
    let mut m = cargo_metadata::MetadataCommand::new();
    m.manifest_path(manifest_path).no_deps();
    Ok(m.exec()?.workspace_root.into_std_path_buf())
}

fn project_key_from_dir(dir: &PathBuf) -> String {
    let mut hasher = DefaultHasher::new();
    dir.hash(&mut hasher);
    format!("{}", hasher.finish())
}

fn begin_session(begin: BeginOpts) -> Result<()> {
    let project_dir = metadata_dir(PathBuf::from("Cargo.toml"))?;
    let key = project_key_from_dir(&project_dir);
    let cfgs = SavedConfigs::load().unwrap_or_default();
    let name = begin
        .config
        .clone()
        .or(cfgs.default.clone())
        .ok_or_else(|| anyhow!("no default config"))?;
    let c = cfgs.get(&name).ok_or_else(|| anyhow!("config not found"))?;
    if !PathBuf::from(&c.ssh_private_key_path).is_file() {
        return Err(anyhow!("private key missing"));
    }
    if !PathBuf::from(&c.ssh_public_key_path).is_file() {
        return Err(anyhow!("public key missing"));
    }
    let provider = get_provider(&c)?;
    let handle = provider.rent(&key, &begin.preinstall)?;
    let mut st = State::load().unwrap_or_default();
    st.projects.insert(key, handle);
    st.save()?;
    Ok(())
}

fn end_session() -> Result<()> {
    let project_dir = metadata_dir(PathBuf::from("Cargo.toml"))?;
    let key = project_key_from_dir(&project_dir);
    let cfgs = SavedConfigs::load().unwrap_or_default();
    let st = State::load().unwrap_or_default();
    let handle = st.projects.get(&key).ok_or_else(|| anyhow!("no session"))?;
    let c = cfgs
        .get(
            cfgs.default
                .as_ref()
                .ok_or_else(|| anyhow!("no default config"))?,
        )
        .ok_or_else(|| anyhow!("config not found"))?;
    let provider = get_provider(&c)?;
    provider.delete(handle)?;
    let mut st = State::load().unwrap_or_default();
    st.projects.remove(&key);
    st.save()?;
    Ok(())
}

fn status() -> Result<()> {
    let mut st = State::load().unwrap_or_default();
    let cfgs = SavedConfigs::load().unwrap_or_default();
    let mut changed = false;
    let mut rm = vec![];
    println!("Path: {:?}", SavedConfigs::path().unwrap());
    println!("Path: {:?}", State::path().unwrap());

    for (k, h) in st.projects.iter() {
        if let Some(c) = cfgs.get(
            cfgs.default
                .as_ref()
                .ok_or_else(|| anyhow!("no default config"))?,
        ) {
            if !provider_exists(&c, h)? {
                rm.push(k.clone());
                changed = true;
            } else {
                println!("{} {}@{}:{}", k, h.username, h.host, h.port);
                let ssh_base = |args: &[&str]| {
                    let mut cmd = Command::new("ssh");
                    cmd.arg("-i")
                        .arg(&c.ssh_private_key_path)
                        .arg("-p")
                        .arg(h.port.to_string())
                        .arg(format!("{}@{}", h.username, h.host));
                    for a in args {
                        cmd.arg(a);
                    }
                    cmd
                };
                let ready = ssh_base(&["test", "-f", "/root/ready"])
                    .status()
                    .map(|s| s.success())
                    .unwrap_or(false);
                if ready {
                    println!("ready: true");
                } else {
                    let out = ssh_base(&["cloud-init", "status", "--long"])
                        .output()
                        .unwrap_or_else(|_| std::process::Output {
                            status: std::process::ExitStatus::from_raw(1),
                            stdout: vec![],
                            stderr: vec![],
                        });
                    let msg = String::from_utf8_lossy(&out.stdout);
                    println!("ready: false");
                    println!("cloud-init:\n{}", msg.trim());
                }
            }
        }
    }
    for k in rm {
        st.projects.remove(&k);
    }
    if changed {
        st.save()?;
    }
    Ok(())
}

fn configure_wizard() -> Result<()> {
    let mut cfgs = SavedConfigs::load().unwrap_or_default();
    println!("Supported providers:");
    for (i, p) in SUPPORTED_PROVIDERS.iter().enumerate() {
        println!("{}: {}", i + 1, p.as_str());
    }
    println!("Select provider: ");
    std::io::stdout().flush().ok();
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    let idx: usize = input.trim().parse().unwrap_or(0);
    if idx == 0 || idx > SUPPORTED_PROVIDERS.len() {
        return Err(anyhow!("invalid selection"));
    }
    let provider = SUPPORTED_PROVIDERS[idx - 1].clone();
    input.clear();
    println!("Config name: ");
    std::io::stdout().flush().ok();
    std::io::stdin().read_line(&mut input)?;
    let name = input.trim().to_string();
    input.clear();
    println!("SSH public key path: ");
    std::io::stdout().flush().ok();
    std::io::stdin().read_line(&mut input)?;
    let pubk = input.trim().into();
    input.clear();
    println!("SSH private key path: ");
    std::io::stdout().flush().ok();
    std::io::stdin().read_line(&mut input)?;
    let privk = input.trim().into();
    let mut c = config::SavedConfig {
        name: name.clone(),
        provider,
        ssh_public_key_path: pubk,
        ssh_private_key_path: privk,
        hetzner: None,
    };
    input.clear();

    println!("yoo");
    match provider {
        provider::ProviderKind::Hetzner => {
            println!("Hetzner API key: ");
            std::io::stdin().read_line(&mut input)?;
            let api_key = input.trim().into();
            input.clear();
            println!("Location [nbg1]: ");
            std::io::stdin().read_line(&mut input)?;
            let loc = {
                let t = input.trim();
                if t.is_empty() {
                    "nbg1".into()
                } else {
                    t.into()
                }
            };
            input.clear();
            println!("Server type [cpx21]: ");
            std::io::stdin().read_line(&mut input)?;
            let stype = {
                let t = input.trim();
                if t.is_empty() {
                    "cpx21".into()
                } else {
                    t.into()
                }
            };
            input.clear();
            println!("Image [ubuntu-22.04]: ");
            std::io::stdin().read_line(&mut input)?;
            let img = {
                let t = input.trim();
                if t.is_empty() {
                    "ubuntu-22.04".into()
                } else {
                    t.into()
                }
            };
            input.clear();
            println!("SSH-Key name: ");
            std::io::stdin().read_line(&mut input)?;
            let key = {
                let t = input.trim();
                if t.is_empty() {
                    "key-1".into()
                } else {
                    t.into()
                }
            };
            input.clear();

            println!("Username [root]: ");
            std::io::stdin().read_line(&mut input)?;
            let username = {
                let t = input.trim();
                if t.is_empty() {
                    Some("root".into())
                } else {
                    Some(t.into())
                }
            };
            c.hetzner = Some(provider::HetznerConfig {
                api_key,
                location: loc,
                server_type: stype,
                image: img,
                username,
                ssh_key: key,
            });
        }
    }
    println!("Provider Done");
    cfgs.items.push(c.clone());
    if cfgs.default.is_none() {
        cfgs.default = Some(name);
    }
    cfgs.save()?;
    println!("Configured");
    Ok(())
}

fn config_list() -> Result<()> {
    let cfgs = SavedConfigs::load().unwrap_or_default();
    for (i, c) in cfgs.items.iter().enumerate() {
        let d = if cfgs.default.as_deref() == Some(&c.name) {
            "*"
        } else {
            " "
        };
        println!("{} [{}] {} {}", i, d, c.name, c.provider.as_str());
    }
    Ok(())
}

fn config_show(name: Option<String>, index: Option<usize>) -> Result<()> {
    println!("Path: {:?}", SavedConfigs::path().unwrap());
    let cfgs = SavedConfigs::load().unwrap_or_default();
    let c = if let Some(n) = name {
        cfgs.get(&n).ok_or_else(|| anyhow!("not found"))?
    } else if let Some(i) = index {
        cfgs.items.get(i).cloned().ok_or_else(|| anyhow!("index"))?
    } else {
        return Err(anyhow!("provide name or index"));
    };
    println!("{}", serde_json::to_string_pretty(&c)?);
    Ok(())
}

fn config_delete(name: Option<String>, index: Option<usize>) -> Result<()> {
    let mut cfgs = SavedConfigs::load().unwrap_or_default();
    let pos = if let Some(n) = name {
        cfgs.items
            .iter()
            .position(|c| c.name == n)
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
    if cfgs.default.as_deref() == Some(&removed.name) {
        cfgs.default = None;
    }
    cfgs.save()?;
    println!("Deleted");
    Ok(())
}

fn config_edit() -> Result<()> {
    let p = SavedConfigs::path()?;
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".into());
    let status = Command::new(editor).arg(&p).status()?;
    if !status.success() {
        println!("File: {}", p.display());
    }
    Ok(())
}

fn sh_quote(s: &str) -> String {
    let mut out = String::from("'");
    for ch in s.chars() {
        if ch == '\'' {
            out.push_str("'\"'\"'");
        } else {
            out.push(ch);
        }
    }
    out.push('\'');
    out
}

fn remote_home(ssh_key: &PathBuf, ssh_port: u16, server_ssh: &str) -> Result<String> {
    let out = Command::new("ssh")
        .arg("-i")
        .arg(ssh_key)
        .arg("-p")
        .arg(ssh_port.to_string())
        .arg(server_ssh)
        .arg("bash -lc 'printf %s \"$HOME\"'")
        .output()?;
    if !out.status.success() {
        return Err(anyhow!("could not get remote HOME"));
    }
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if s.is_empty() {
        return Err(anyhow!("empty remote HOME"));
    }
    Ok(s)
}

fn remote_exec(exec: ExecOpts, cmd: &str, options: Vec<String>) -> Result<i32> {
    let project_dir = metadata_dir(exec.manifest_path.clone())?;
    let key = project_key_from_dir(&project_dir);
    let st = State::load().unwrap_or_default();
    let h = st
        .projects
        .get(&key)
        .ok_or_else(|| anyhow!("no active session"))?;
    let server_ssh = format!("{}@{}", h.username, h.host);
    let cfgs = SavedConfigs::load().unwrap_or_default();
    let c = cfgs
        .get(
            cfgs.default
                .as_deref()
                .ok_or_else(|| anyhow!("no default config"))?,
        )
        .ok_or_else(|| anyhow!("config not found"))?;
    let home = remote_home(
        &PathBuf::from(c.ssh_private_key_path.clone()),
        h.port,
        &server_ssh,
    )?;
    let build_path = format!("{}/remote-builds/{}/", home, key);
    let s = SessionRemote {
        project_dir,
        server_ssh,
        ssh_key: PathBuf::from(c.ssh_private_key_path),
        ssh_port: h.port,
        build_path,
        build_env: exec.build_env,
        rustup_default: exec.rustup_default,
        copy_back: exec.copy_back,
        no_copy_lock: exec.no_copy_lock,
        hidden: exec.hidden,
        command: cmd.into(),
        options,
    };
    run_session(s)
}

fn run_session(s: SessionRemote) -> Result<i32> {
    let check_cmd = "sh -lc 'test -f /root/ready'".to_string();
    let ready = Command::new("ssh")
        .arg("-i")
        .arg(&s.ssh_key)
        .arg("-p")
        .arg(s.ssh_port.to_string())
        .arg(&s.server_ssh)
        .arg(&check_cmd)
        .status()?
        .success();

    if !ready {
        let ci = Command::new("ssh")
            .arg("-i")
            .arg(&s.ssh_key)
            .arg("-p")
            .arg(s.ssh_port.to_string())
            .arg(&s.server_ssh)
            .arg("cloud-init status --long || true")
            .output()?;
        let msg = String::from_utf8_lossy(&ci.stdout);
        return Err(anyhow!(format!("cloud-init not ready\n{}", msg.trim())));
    }

    let _ = Command::new("ssh")
        .arg("-i")
        .arg(&s.ssh_key)
        .arg("-p")
        .arg(s.ssh_port.to_string())
        .arg(&s.server_ssh)
        .arg(format!(
            "bash -lc \"mkdir -p '{}'\"",
            s.build_path.trim_end_matches('/')
        ))
        .status()?;
    let mut rsync_cmd = Command::new("rsync");
    rsync_cmd
        .arg("-a")
        .arg("--delete")
        .arg("--compress")
        .arg("-e")
        .arg(format!(
            "ssh -i {} -p {}",
            s.ssh_key.to_string_lossy(),
            s.ssh_port
        ))
        .arg("--exclude")
        .arg("target");
    if !s.hidden {
        rsync_cmd
            .arg("--exclude")
            .arg(".*")
            .arg("--exclude")
            .arg("*/.*");
    }
    rsync_cmd
        .arg(format!("{}/", s.project_dir.to_string_lossy()))
        .arg(format!("{}:{}", s.server_ssh, s.build_path));
    let status = rsync_cmd.status()?;
    if !status.success() {
        return Err(anyhow!("rsync failed"));
    }
    let quoted_opts: String = s
        .options
        .iter()
        .map(|x| sh_quote(x))
        .collect::<Vec<_>>()
        .join(" ");
    let cmd = format!(
        "bash -lc \"cd '{}' && {} rustup default {} >/dev/null 2>&1 || true; {} cargo {} {}\"",
        s.build_path, "", s.rustup_default, s.build_env, s.command, quoted_opts
    );
    let out = Command::new("ssh")
        .arg("-i")
        .arg(&s.ssh_key)
        .arg("-p")
        .arg(s.ssh_port.to_string())
        .arg(&s.server_ssh)
        .arg(cmd)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .stdin(Stdio::inherit())
        .output()?;
    if let Some(name) = &s.copy_back {
        let status = Command::new("rsync")
            .arg("-a")
            .arg("--compress")
            .arg("-e")
            .arg(format!(
                "ssh -i {} -p {}",
                s.ssh_key.to_string_lossy(),
                s.ssh_port
            ))
            .arg(format!(
                "{}:'{}/target/{}/'",
                s.server_ssh,
                s.build_path.trim_end_matches('/'),
                name
            ))
            .arg(format!(
                "{}/target/{}/",
                s.project_dir.to_string_lossy(),
                name
            ))
            .status()?;
        if !status.success() {
            return Err(anyhow!("rsync copy-back failed"));
        }
    }
    if !s.no_copy_lock {
        let _ = Command::new("rsync")
            .arg("-a")
            .arg("-e")
            .arg(format!(
                "ssh -i {} -p {}",
                s.ssh_key.to_string_lossy(),
                s.ssh_port
            ))
            .arg(format!("{}:'{}Cargo.lock'", s.server_ssh, s.build_path))
            .arg(format!("{}/Cargo.lock", s.project_dir.to_string_lossy()))
            .status()?;
    }
    Ok(out.status.code().unwrap_or(1))
}

fn main() {
    simple_logger::init().unwrap();
    match CargoCli::from_args() {
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
                if let Err(e) = status() {
                    error!("{}", e);
                    exit(3)
                }
            }
            RemoteCmd::Run { exec, options } => {
                if let Err(e) = remote_exec(exec, "run", options) {
                    error!("{}", e);
                    exit(4)
                }
            }
            RemoteCmd::Build { exec, options } => {
                if let Err(e) = remote_exec(exec, "build", options) {
                    error!("{}", e);
                    exit(4)
                }
            }
            RemoteCmd::Clean { exec, options } => {
                if let Err(e) = remote_exec(exec, "clean", options) {
                    error!("{}", e);
                    exit(4)
                }
            }
        },
    }
}
