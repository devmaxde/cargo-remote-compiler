use anyhow::{bail, Result};
use std::net::{IpAddr, ToSocketAddrs};
use std::os::unix::process::ExitStatusExt;
use std::process::Command;
use std::time::Duration;

use crate::config::mode::Mode;
use crate::config::SavedConfigs;
use crate::provider::provider_exists;
use crate::state::State;

fn resolve_ip(host: &str) -> Result<IpAddr> {
    if host == "localhost" {
        return Ok(IpAddr::from([127, 0, 0, 1]));
    }
    if let Ok(ip) = host.parse::<IpAddr>() {
        return Ok(ip);
    }
    if let Ok(mut it) = (host, 0).to_socket_addrs() {
        if let Some(addr) = it.next() {
            return Ok(addr.ip());
        }
    }
    if !host.contains('.') && !host.ends_with('.') {
        let h = format!("{host}.local");
        if let Ok(mut it) = (h.as_str(), 0).to_socket_addrs() {
            if let Some(addr) = it.next() {
                return Ok(addr.ip());
            }
        }
    }
    bail!("unresolvable host")
}

pub fn ping_server(target: &str) -> Result<bool> {
    let ip = resolve_ip(target)?;
    let p = ping::new(ip.to_string().parse()?)
        .timeout(Duration::from_secs(2))
        .ttl(128)
        .send();
    Ok(p.is_ok())
}

pub fn status() -> Result<()> {
    let mut st = State::load().unwrap_or_default();
    let cfgs = SavedConfigs::load().unwrap_or_default();
    let mut changed = false;
    let mut rm = vec![];

    println!("Available Servers: ");
    for c in cfgs.items.iter() {
        if !Mode::check_cloud_mode(&c.mode) {
            if let crate::config::mode::ConfigData::Manual(manual_config) = c.data.clone() {
                println!(
                    "[{}] {} up: {}",
                    c.name(),
                    manual_config.host,
                    ping_server(&manual_config.host).unwrap_or_else(|e| {
                        println!("Failes to look up Host with {:?}", e);
                        false
                    })
                );
            }
        }
    }

    if !st.projects.is_empty() {
        println!();
        println!("Cloud-Server: ");
    }
    for h in st.projects.iter() {
        if let Some(c) = cfgs.get(&h.config) {
            if !provider_exists(&c, h)? {
                rm.push(h.clone());
                changed = true;
            } else {
                let privk = c.private_key_path();
                let ssh_base = |args: &[&str]| {
                    let mut cmd = Command::new("ssh");
                    cmd.arg("-i")
                        .arg(&privk)
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

                println!("[{}-{}] {} ready: {}", c.mode, c.name(), h.host, ready);

                if !ready {
                    let out = ssh_base(&["cloud-init", "status"])
                        .output()
                        .unwrap_or_else(|_| std::process::Output {
                            status: std::process::ExitStatus::from_raw(1),
                            stdout: vec![],
                            stderr: vec![],
                        });
                    let msg = String::from_utf8_lossy(&out.stdout);
                    println!("cloud-init:\n{}", msg.trim());
                    println!();
                }
            }
        } else {
            rm.push(h.clone());
            changed = true;
        }
    }
    let mut tmp = vec![];
    for i in st.projects {
        if !rm.contains(&i) {
            tmp.push(i);
        }
    }
    st.projects = tmp;

    if changed {
        st.save()?;
    }
    Ok(())
}
