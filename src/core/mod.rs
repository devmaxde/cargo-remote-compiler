use crate::config::SavedConfigs;
use anyhow::{anyhow, Result};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

use crate::{ExecOpts, SessionRemote};

pub fn metadata_dir(manifest_path: PathBuf) -> Result<PathBuf> {
    let mut m = cargo_metadata::MetadataCommand::new();
    m.manifest_path(manifest_path).no_deps();
    Ok(m.exec()?.workspace_root.into_std_path_buf())
}

pub fn project_key_from_dir(dir: &PathBuf) -> String {
    let mut hasher = DefaultHasher::new();
    dir.hash(&mut hasher);
    format!("{}", hasher.finish())
}

pub fn sh_quote(s: &str) -> String {
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

pub fn remote_home(ssh_key: &PathBuf, ssh_port: u16, server_ssh: &str) -> Result<String> {
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

pub fn remote_exec(exec: ExecOpts, cmd: &str, options: Vec<String>) -> anyhow::Result<i32> {
    // Identify project + key
    let project_dir = metadata_dir(exec.manifest_path.clone())?;
    let key = project_key_from_dir(&project_dir);

    // Resolve remote host (may prompt if Priority::Ask)
    let cfgs = SavedConfigs::load().unwrap_or_default();
    let (host, user, ssh_port, ssh_key) = cfgs.select_remote_host()?;

    let full_host = format!("{}@{}", user, host);

    // Compute remote build path
    let home = remote_home(&ssh_key, ssh_port, &full_host)?;
    let build_path = format!("{}/remote-builds/{}/", home, key);

    // Hand off to the existing runner
    let s = SessionRemote {
        project_dir,
        server_ssh: full_host,
        ssh_key,
        ssh_port,
        build_path,
        build_env: exec.build_env,
        rustup_default: exec.rustup_default,
        copy_back: exec.copy_back,
        no_copy_lock: exec.no_copy_lock,
        hidden: exec.hidden,
        command: cmd.into(),
        options,
    };

    // Run the acutal Session
    // check_redy()?;
    upsync(&s)?;
    let out = run_cargo(&s)?;
    downsync(&s)?;

    Ok(out.status.code().unwrap_or(1))
}

pub fn upsync(s: &SessionRemote) -> Result<()> {
    // Create the remote build_folder
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

    // Upsync command
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
    Ok(())
    // Upsync done!
}

pub fn downsync(s: &SessionRemote) -> Result<()> {
    // Copy Back
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
    Ok(())
}

pub fn run_cargo(s: &SessionRemote) -> Result<Output> {
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

    Ok(out)
}

#[allow(dead_code)]
pub fn check_ready(s: &SessionRemote) -> Result<()> {
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

    Ok(())
}
