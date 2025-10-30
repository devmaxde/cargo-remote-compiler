# Cargo Remote

Remote Rust project build and execution using temporary cloud servers.

This tool creates a remote server, transfers your current project, runs any
Cargo command remotely, and optionally copies back the `target/` build artifacts.
It is designed to reduce compile time impact on local machines.

> Linux-based servers only (e.g. Hetzner Cloud). macOS and Windows can be used
> as clients.

---

## Features

- Create and delete on-demand cloud build servers
- Remote `cargo build`, `cargo run`, and `cargo clean`
- Fast file sync using `rsync`
- Automatically installs:
  - Rust toolchain
  - GCC, LLVM, Make
  - musl + musl‐tools
  - OpenSSL development libraries
- Cloud-init readiness checks displayed on status
- SSH key–based access
- Configuration stored locally for multiple servers

---

## Installation

```bash
git clone https://github.com/sgeisler/cargo-remote
cargo install --path cargo-remote/
```

Ensure `ssh` and a modern `rsync` are installed on your client machine.

macOS users may need:

```bash
brew install rsync
```

---

## Quick Start

### 1) Configure a provider (Hetzner)

```bash
cargo remote configure
```

Enter:

- Hetzner API key
- SSH key paths
- Location / server type / image

This creates a default configuration stored at:

```
~/.config/cargo-remote/configs.json
```

---

### 2) Start a remote session

Inside any Rust project:

```bash
cargo remote begin
```

You can inspect readiness anytime:

```bash
cargo remote status
```

If cloud-init is still running, full status is displayed.

---

### 3) Run or build remotely

```bash
cargo remote build --release
cargo remote run
cargo remote clean
```

All commands behave like local Cargo operations, but execute remotely.

---

### 4) End the session

```bash
cargo remote end
```

This terminates the remote server and removes the session record.

---

## Configuration

Example config file:

```toml
[[remote]]
name = "hetzner"
host = "myUser@myIP"
ssh_port = 22
temp_dir = "~/remote-builds"
env = "/etc/profile.d/cargo.sh"
```

Multiple remote configurations can be stored.

Select active config via:

```bash
cargo remote begin --config myServerName
```

---

## Preinstall Packages

You may define additional server packages during setup:

```bash
cargo remote begin --preinstall git,cmake
```

They will be included in cloud-init under the `packages:` list.

---

## How it Works

1. A remote server is created via provider API
2. Cloud-init installs Rust and required system packages
3. Project directory is synced via `rsync`
4. Cargo commands are executed via SSH
5. Target artifacts may optionally sync back to the client

Session state is stored at:

```
~/.local/share/cargo-remote/state.json
```

---

## Safety Notice

Use only on clean, disposable cloud VMs. The tool executes commands remotely as
`root`, so do not use on production systems.

---

## License

MIT

---

## Contributing

Pull requests and provider extensions are welcome.
