# Adding a Provider

This guide helps maintainers extend the project with new cloud providers beyond Hetzner.

## Concept

The project is an rsync-based utility that compiles code on remote machines. Remote builds only require SSH configuration. Cloud support is an extra convenience: servers can be created and automatically configured for use with rsync. Manual configuration remains the primary mode.

## Provider Architecture

Every cloud integration must implement the `Provider` trait:

```rust
pub trait Provider {
    fn rent(&self, project_key: &str, preinstall: &[String]) -> Result<ServerHandle>;
    fn delete(&self, handle: &ServerHandle) -> Result<()>;
    fn exists(&self, handle: &ServerHandle) -> Result<bool>;
}
```

A `ServerHandle` contains an `id`, which uniquely identifies the cloud resource.

### Provisioning Rules

When a server is created, ensure:

- Rust, Make, and GCC are installed.
- A file `/root/ready` is created to signal completion.

## Integration Points

All provider routing happens in `./src/provider/mod.rs`:

- Extend `ProviderKind` enum with your provider
- Add configuration data to `ConfigData`
- Implement a configuration wizard that integrates with `Mode::run_wizzard`
- Extend `get_provider` to return your provider implementation

Hetzner serves as a complete example:

```
src/provider/hetzner
```

Follow that pattern for any new provider implementation.
