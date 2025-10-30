# cargo-remote

Build and run Rust projects on remote machines (manual SSH hosts or on‑demand cloud VMs). Your local laptop stays cool; the server does the heavy lifting.

> Servers: Linux-only. Clients: macOS & Linux (Windows untested).

---

## Why

- Offload heavy `cargo` work to a beefy VM.
- Keep your local machine snappy.
- Reuse the same workflow: `cargo remote build/run/clean` mirrors local commands.

---

## Features

- Manual SSH hosts **or** Hetzner Cloud VMs (created on demand)
- Fast syncing with `rsync`
- One-shot setup via wizards
- Auto-install on servers: Rust toolchain, GCC/LLVM/Make, OpenSSL dev libs, musl
- Readiness checks (`/root/ready`, cloud‑init status)
- Multiple saved configs; pick by name, default, or prompt
- Optional copy‑back of `target/<profile>`

---

## Requirements

On your **client** (macOS/Linux):

- Rust & Cargo
- `ssh`
- `rsync` (on macOS: `brew install rsync`)

For **Hetzner Cloud** mode:

- Hetzner API token
- An SSH public key uploaded to your Hetzner account
- Local private key file present (used to connect)

For **Manual** mode:

- A reachable Linux server with your SSH key installed

---

## Install

```bash
git clone https://github.com/devmaxde/cargo-remote
cargo install --path cargo-remote
```

---

## Quick start (Hetzner Cloud)

```bash
# 1) Create a config
cargo remote configure

# 2) Start a cloud VM (optionally preinstall extra packages)
cargo remote begin --preinstall git,cmake

# 3) Build / run remotely (from your project dir)
cargo remote build --release
cargo remote run

# 4) Inspect readiness and details
cargo remote status

# 5) Tear down the VM
cargo remote end
```

Quick start (Manual SSH host):

```bash
cargo remote configure   # choose “Manual”, enter host/user/port and key paths
cargo remote run         # or build/clean; runs on the selected manual host
```

---

## CLI overview

- `cargo remote configure` — interactive setup (Manual or Hetzner)
- `cargo remote config list|show|edit|delete` — manage saved configs
- `cargo remote begin [--config NAME] [--preinstall a,b,c]` — create cloud VM
- `cargo remote status` — show manual host reachability and cloud readiness
- `cargo remote end` — delete a running cloud VM
- `cargo remote run|build|clean [options] -- [cargo options]` — execute remotely

Common flags for `run|build|clean`:

- `-b, --build-env <KV>` (default `RUST_BACKTRACE=1`): env before `cargo`
- `-d, --rustup-default <toolchain>` (default `stable`)
- `-c, --copy-back <profile>`: copy back `target/<profile>/` (e.g. `debug`, `release`)
- `--no-copy-lock`: don’t pull back `Cargo.lock`
- `--manifest-path <file>` (default `Cargo.toml`)
- `--transfer-hidden`: include dotfiles when syncing

Pick a config explicitly:

```bash
cargo remote begin --config my-cloud
cargo remote build --config my-cloud   # via interactive selection when needed
```

---

## Configuration files

- Configs: `~/.config/cargo-remote/config.toml`
- Active cloud servers: `~/.config/cargo-remote/servers.toml`

Priority modes (set during configure): **Manual**, **Cloud**, or **Ask**.

- **Manual**: prefer manual hosts
- **Cloud**: prefer running cloud servers
- **Ask**: choose interactively each time

---

## How it works

1. (Cloud) VM is created and provisioned via cloud‑init; installs toolchains.
2. Project is synced to `~/remote-builds/<hash>/` via `rsync`.
3. `cargo` runs over SSH in that directory.
4. Artifacts optionally copy back; `Cargo.lock` syncs unless `--no-copy-lock`.

---

## Troubleshooting

- “private key missing”: ensure the local key path in your config exists.
- “no cloud provider configured”: run `cargo remote configure` and choose Hetzner.
- Cloud not ready: `cargo remote status` shows `cloud-init status` output.
- `rsync failed`: verify SSH connectivity and your `--transfer-hidden`/excludes.

---

## Extending

Want another cloud? See **[ADD_PROVIDER.md](ADD_PROVIDER.md)** for the trait and wiring points (`src/provider/*`).

---

## Safety

This tool executes remote commands as root on throwaway VMs. Don’t point it at production systems.

---

## License

MIT
