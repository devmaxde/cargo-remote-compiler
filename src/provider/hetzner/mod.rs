use std::time::SystemTime;

use crate::provider::{hetzner::config::HetznerConfig, Provider, ProviderKind, ServerHandle};
use anyhow::{anyhow, Result};
use reqwest::blocking::Client;
use serde::Deserialize;
use serde_json::Value;

pub mod config;

pub struct HetznerProvider {
    pub cfg: HetznerConfig,
}

#[derive(Deserialize)]
pub struct LoactionResponse {
    locations: Vec<Location>,
}

#[derive(Deserialize)]
pub struct Location {
    name: String,
    country: String,
    description: String,
}

#[derive(Deserialize)]
pub struct SSHResponse {
    ssh_keys: Vec<SSHKey>,
}

#[derive(Deserialize)]
pub struct SSHKey {
    name: String,
    fingerprint: String,
}

#[derive(Deserialize)]
pub struct ServerTypesResponse {
    server_types: Vec<ServerTypes>,
}

#[derive(Deserialize)]
pub struct ServerTypes {
    name: String,
    cores: i32,
    memory: i32,
    architecture: String,
    category: String,
}

impl std::fmt::Display for Location {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} (Country: {}, Description: {})",
            self.name, self.country, self.description
        )
    }
}

impl std::fmt::Display for ServerTypes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} ({}-Cores: {} Ram: {}, Category: {})",
            self.name, self.architecture, self.cores, self.memory, self.category
        )
    }
}

impl std::fmt::Display for SSHKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.name, self.fingerprint)
    }
}

impl HetznerProvider {
    fn client(&self) -> Client {
        Client::builder().build().unwrap()
    }
    fn base() -> &'static str {
        "https://api.hetzner.cloud/v1"
    }
    fn cloud_init(&self, preinstall: &[String]) -> String {
        let mut s = String::from(
            "#cloud-config\npackage_update: true\npackage_upgrade: true\npackages:\n\
 - build-essential\n\
 - gcc\n\
 - make\n\
 - musl\n\
 - musl-tools\n\
 - libssl-dev\n\
 - pkg-config\n\
 - llvm\n\
 - clang\n\
 - git\n\
 - curl\n\
 - ca-certificates\n",
        );
        for p in preinstall {
            if !p.trim().is_empty() {
                s.push_str(&format!(" - {}\n", p.trim()));
            }
        }
        s.push_str(
            "runcmd:
 - [bash, -lc, \"export DEBIAN_FRONTEND=noninteractive && apt-get update && apt-get -yq upgrade\"]
 - [bash, -lc, \"apt-get install -yqq curl ca-certificates\"]
 - [bash, -lc, \"curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs -o /root/rustup-init.sh\"]
 - [bash, -lc, \"chmod +x /root/rustup-init.sh\"]
 - [bash, -lc, \"/root/rustup-init.sh -y --profile minimal --default-toolchain stable\"]
 - [bash, -lc, \"echo 'export PATH=\\\"$HOME/.cargo/bin:$PATH\\\"' >> /root/.bashrc\"]
 - [bash, -lc, \"printf 'export PATH=\\\"/root/.cargo/bin:$PATH\\\"\\n' > /etc/profile.d/cargo.sh && chmod +x /etc/profile.d/cargo.sh\"]
 - [bash, -lc, \"/root/.cargo/bin/rustc --version && /root/.cargo/bin/cargo --version\"]
 - [bash, -lc, \"touch /root/ready\"]
",
        );
        s
    }

    pub fn get_server_types(api_key: String) -> Result<Vec<ServerTypes>> {
        let client = Client::builder().build().unwrap();

        let resp = client
            .get(format!("{}/server_types", Self::base()))
            .bearer_auth(api_key)
            .send()?;
        if !resp.status().is_success() {
            let msg = resp.text().unwrap_or_default();
            return Err(anyhow!("hetzner create failed: {}", msg));
        }

        let response: ServerTypesResponse = resp.json().unwrap();

        Ok(response.server_types)
    }

    pub fn get_ssh_keys(api_key: String) -> Result<Vec<SSHKey>> {
        let client = Client::builder().build().unwrap();

        let resp = client
            .get(format!("{}/ssh_keys", Self::base()))
            .bearer_auth(api_key)
            .send()?;
        if !resp.status().is_success() {
            let msg = resp.text().unwrap_or_default();
            return Err(anyhow!("hetzner create failed: {}", msg));
        }

        let response: SSHResponse = resp.json().unwrap();

        Ok(response.ssh_keys)
    }

    pub fn get_locations(api_key: String) -> Result<Vec<Location>> {
        let client = Client::builder().build().unwrap();

        let resp = client
            .get(format!("{}/locations", Self::base()))
            .bearer_auth(api_key)
            .send()?;
        if !resp.status().is_success() {
            let msg = resp.text().unwrap_or_default();
            return Err(anyhow!("hetzner create failed: {}", msg));
        }

        let response: LoactionResponse = resp.json().unwrap();

        Ok(response.locations)
    }
}

impl Provider for HetznerProvider {
    fn rent(&self, project_key: &str, preinstall: &[String]) -> Result<ServerHandle> {
        let client = self.client();
        let timestampt = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let name = format!("cargo-remote-{}-{}", project_key, timestampt);

        let body = serde_json::json!({
            "name": name,
            "server_type": self.cfg.server_type,
            "image": self.cfg.image,
            "location": self.cfg.location,
            "ssh_keys": [self.cfg.ssh_key.clone()],
            "user_data": self.cloud_init(preinstall),
        });

        let resp = client
            .post(format!("{}/servers", Self::base()))
            .bearer_auth(&self.cfg.api_key)
            .json(&body)
            .send()?;
        if !resp.status().is_success() {
            let msg = resp.text().unwrap_or_default();
            return Err(anyhow!("hetzner create failed: {}", msg));
        }
        let v: Value = resp.json()?;
        let server = v.get("server").ok_or_else(|| anyhow!("missing server"))?;
        let id = server
            .get("id")
            .and_then(|x| x.as_i64())
            .ok_or_else(|| anyhow!("missing id"))?
            .to_string();
        let ip = server
            .get("public_net")
            .and_then(|p| p.get("ipv4"))
            .and_then(|i| i.get("ip"))
            .and_then(|x| x.as_str())
            .ok_or_else(|| anyhow!("missing IPv4"))?
            .to_string();
        let username = self.cfg.username.clone().unwrap_or("root".into());
        Ok(ServerHandle {
            provider: ProviderKind::Hetzner,
            config: self.cfg.name.clone(),
            id,
            host: ip,
            port: 22,
            username,
        })
    }

    fn delete(&self, handle: &ServerHandle) -> Result<()> {
        let client = self.client();
        let resp = client
            .delete(format!("{}/servers/{}", Self::base(), handle.id))
            .bearer_auth(&self.cfg.api_key)
            .send()?;
        if !resp.status().is_success() {
            let msg = resp.text().unwrap_or_default();
            return Err(anyhow!("hetzner delete failed: {}", msg));
        }
        Ok(())
    }

    fn exists(&self, handle: &ServerHandle) -> Result<bool> {
        let client = self.client();
        let resp = client
            .get(format!("{}/servers/{}", Self::base(), handle.id))
            .bearer_auth(&self.cfg.api_key)
            .send()?;
        Ok(resp.status().is_success())
    }
}
