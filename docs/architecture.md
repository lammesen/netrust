# Architecture Interfaces Summary

## Module Layout

| Layer | Crate(s) | Responsibilities | Key Interfaces |
|-------|----------|------------------|----------------|
| Domain Model | `nauto_model` | Devices, credentials, jobs, selectors, job results | Serde structs/enums (`Device`, `Job`, `TargetSelector`, `JobResult`) |
| Drivers | `nauto_drivers` | Vendor-specific transport + capability mapping | `DeviceDriver` trait, `DriverRegistry`, capability flags |
| Orchestration | `nauto_engine` | Target resolution, concurrency, retries, rollback | `JobEngine::execute`, `DeviceInventory`, progress events |
| Security | `nauto_security` | Credential storage + retrieval | `CredentialStore` trait, `KeyringStore` |
| UX | `apps/nauto_cli` (+ future GUI) | CLI/TUI commands, audit logging, job invocation | Clap commands (`run`, `creds`, `tui`), TUI dashboards |
| Extensibility | `spikes/wasm_host` (future crate) | WASM plugin loading | HostContext registration functions |

## Async & Concurrency
- Tokio runtime with multithreaded scheduler.
- `JobEngine` throttles device work using `tokio::sync::Semaphore` (`max_parallel` job setting or default 32).
- Driver operations async/await; command execution returns structured logs/diffs.

## Observability Contract
- `tracing` spans per job (`job_id`), per device (`device_id`).
- CLI emits human-readable logs; future plan to expose JSON logs/metrics.
- Audit log writer (`apps/nauto_cli/src/audit.rs`) records job summary lines for SIEM ingestion.

## Key Interfaces

### DeviceDriver Trait
```rust
#[async_trait]
pub trait DeviceDriver: Send + Sync {
    fn device_type(&self) -> DeviceType;
    fn capabilities(&self) -> CapabilitySet;
    async fn execute(&self, device: &Device, action: DriverAction<'_>) -> Result<DriverExecutionResult>;
    async fn rollback(&self, device: &Device, snapshot: Option<String>) -> Result<()>;
}
```

### JobEngine
```rust
pub struct JobEngine<I: DeviceInventory> { /* ... */ }
impl<I: DeviceInventory> JobEngine<I> {
    pub async fn execute(&self, job: Job) -> Result<JobResult>;
}
```

### CredentialStore
```rust
#[async_trait]
pub trait CredentialStore {
    async fn store(&self, reference: &CredentialRef, credential: &Credential) -> Result<()>;
    async fn resolve(&self, reference: &CredentialRef) -> Result<Credential>;
}
```

## Diagrams
(ASCII overview)

```
CLI/TUI/GUI --> JobEngine --> DriverRegistry --> DeviceDriver (per vendor)
          \-> CredentialStore (Keyring) --> OS vault
JobEngine <-> DeviceInventory (in-memory / future DB)
```

## Next Steps
- Convert this summary into onboarding wiki page.
- Keep document updated when new crates (GUI, plugin SDK) land.

