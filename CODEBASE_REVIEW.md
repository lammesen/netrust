# Comprehensive Codebase Review: netrust

**Review Date:** 2025-11-17  
**Reviewer:** Senior Rust Engineer & Network Automation Architect  
**Target Scope:** Production-grade multi-vendor network automation (10k+ devices)

---

## High-Level Summary

### Strengths

- **Solid architectural foundation**: Clean separation between model, drivers, engine, security, and UX layers. The trait-based driver abstraction (`DeviceDriver`) is well-designed and extensible.
- **Async-first concurrency**: Tokio-based async runtime with semaphore-based throttling provides a good foundation for parallel device operations.
- **Comprehensive feature set**: Covers CLI/TUI, drivers for major vendors, compliance checking, telemetry collection, GitOps integration, approvals, transactions, and plugin system—ambitious and well-scoped.
- **Security-conscious credential handling**: Uses OS keyring via `keyring` crate with fallback to encrypted file, avoiding plaintext credentials in most paths.
- **Good observability foundation**: Uses `tracing` for structured logging, has audit log support, and Prometheus metrics integration.

### Critical Risks & Issues

- **ALL DRIVERS ARE STUBS**: The SSH/NETCONF/API drivers simulate operations with `tokio::sleep` instead of real network interactions. This is a **showstopper** for production use—no actual device communication occurs.
- **No per-device timeouts**: `JobEngine::execute` spawns tasks with semaphore throttling but has no individual device timeout mechanism. A single hung device will stall indefinitely.
- **Missing task cancellation**: No graceful shutdown or timeout-based cancellation of spawned device tasks. Memory leaks and zombie tasks are likely under failure conditions.
- **Unbounded result accumulation**: `device_results` vector in `JobEngine::execute` accumulates all results in memory—will OOM for 10k+ devices.
- **Blocking operations in async context**: Compliance evaluation (`execute_compliance_job`) runs synchronously, blocking the Tokio runtime for all devices.
- **Plugin system incomplete**: WASM host is a spike with no integration into driver registry; signatures/verification unimplemented; security sandbox missing.
- **Weak compliance engine**: String matching only (`contains:`, `not:`)—production needs regex, JSON path queries, numeric comparisons, and structured data validation.
- **No distributed worker architecture**: Worker mode reads JSONL from disk—no Redis/SQS/Postgres queue, no retry/DLQ, no coordination across nodes.
- **TUI is minimal**: Shows device list but no live job monitoring, no log streaming, no error details.
- **Test coverage is sparse**: Only 6 tests across entire codebase (engine, compliance, telemetry, model). No integration tests, no driver behavior tests, no failure path coverage.

---

## Detailed Findings by Area

### 1. Architecture & Design

**What's working:**

- Clean module boundaries: `nauto_model` (domain types), `nauto_drivers` (vendor abstraction), `nauto_engine` (orchestration), `nauto_security` (credentials), CLI/TUI apps.
- `DeviceDriver` trait is well-designed with capability flags (`supports_commit`, `supports_rollback`, `supports_diff`, `supports_dry_run`).
- `JobEngine<I: DeviceInventory>` is generic over inventory source—allows plugging in DB or external systems.
- Job types (`CommandBatch`, `ConfigPush`, `ComplianceCheck`) cover common use cases.
- Target selectors (`All`, `ByIds`, `ByTags`) provide flexibility for device scoping.

**Problems / Risks:**

- **No service architecture**: Everything is CLI-driven. Documentation mentions distributed workers, HTTP control plane, GitOps, but none are implemented beyond spikes.
- **No job state persistence**: Jobs run in memory only—no database, no resume capability, no audit trail of job history.
- **Approval system is file-based JSON**: `approvals::is_approved` reads from `approvals.json`—not suitable for multi-user or distributed environments.
- **No job queue abstraction**: Worker mode reads JSONL files—hardcoded to filesystem, no pluggable backend (Redis, SQS, Postgres).
- **Transaction plans are CLI-only**: `transactions::generate_plan` creates YAML files but engine doesn't enforce batch execution or rollback policies.
- **No device locking**: Concurrent jobs could target the same device—race conditions, config conflicts, no coordination.

**Recommendations:**

- [ ] **P0**: Define and implement `JobStore` trait in `nauto_engine` for persisting job state, results, and history (sqlite for MVP, Postgres for production).
- [ ] **P0**: Add `JobQueue` trait with implementations for Redis (pub/sub), SQS (long-polling), and Postgres (SKIP LOCKED). Move worker logic to use queue abstraction.
- [ ] **P1**: Implement device locking mechanism (distributed lock with TTL in Redis or DB advisory locks) to prevent concurrent writes to same device.
- [ ] **P1**: Integrate transaction plan execution into `JobEngine::execute`—enforce canary batch, staged rollout, automatic rollback on failure threshold.
- [ ] **P1**: Add `ApprovalStore` trait backed by database; integrate approval checks into job execution path (not just CLI validation).
- [ ] **P2**: Create HTTP/gRPC control plane service (`apps/nauto_service`) exposing job submit, status, cancel endpoints; separate from CLI.

---

### 2. Async & Concurrency

**What's working:**

- Uses Tokio multi-threaded runtime consistently.
- `JobEngine::execute` uses `FuturesUnordered` for concurrent device processing—good choice for heterogeneous completion times.
- `Semaphore` throttles parallel execution (`max_parallel` job setting)—prevents overwhelming network or target devices.
- Drivers use `async_trait` consistently—enables async I/O throughout.

**Problems / Risks:**

- **No per-device timeouts**: `run_device` spawns tasks with no timeout wrapper. A hung SSH session or API call will block indefinitely.
  ```rust
  // In nauto_engine/src/lib.rs:62-65
  tasks.push(tokio::spawn(async move {
      let permit = sem.acquire_owned().await.expect("semaphore closed");
      run_device(device, driver, job_kind, dry_run, permit).await // NO TIMEOUT HERE
  }));
  ```
- **No task cancellation**: Spawned tasks are fire-and-forget. No mechanism to cancel running tasks if job is aborted or times out.
- **Unbounded memory growth**: `device_results` accumulates all results in memory. For 10k devices × 10KB result = 100MB minimum, likely more with logs/diffs.
- **Blocking sync code in async context**: `execute_compliance_job` (line 86-130) runs synchronously—evaluates all devices serially, blocking Tokio worker thread.
- **No backpressure from result collection**: `tasks.next().await` loop (line 69-74) consumes results immediately but doesn't limit active tasks beyond semaphore count.
- **Semaphore panic on close**: `sem.acquire_owned().await.expect("semaphore closed")` panics if semaphore is dropped early—should handle gracefully.

**Recommendations:**

- [ ] **P0**: Wrap `run_device` call in `tokio::time::timeout` with configurable per-device timeout (default 5 minutes). Return `TaskStatus::Failed` on timeout.
  ```rust
  // In nauto_engine/src/lib.rs::execute
  let timeout = device_timeout.unwrap_or(Duration::from_secs(300));
  tasks.push(tokio::spawn(async move {
      let permit = sem.acquire_owned().await.ok()?;
      tokio::time::timeout(timeout, run_device(...)).await
          .unwrap_or_else(|_| TaskSummary { status: TaskStatus::Failed, logs: vec!["timeout".into()], ...})
  }));
  ```
- [ ] **P0**: Add graceful cancellation support: store `JoinHandle`s from `tokio::spawn` in job state; expose `cancel()` method that calls `handle.abort()`.
- [ ] **P0**: Stream results to `JobStore` instead of accumulating in memory. Modify `JobEngine::execute` to write each `TaskSummary` to storage as it completes.
- [ ] **P0**: Make `execute_compliance_job` async and process devices in parallel using `FuturesUnordered` like command/config jobs.
- [ ] **P1**: Add retry logic with exponential backoff for transient failures (connection timeouts, rate limits). Use `tokio_retry` crate or implement custom backoff.
- [ ] **P1**: Replace `expect` on semaphore with `ok()?` and log error—prevents panic if job is cancelled mid-execution.
- [ ] **P2**: Add telemetry for active tasks, queued tasks, and task latency distribution (histogram of device execution times).

---

### 3. Drivers & Protocol Handling

**What's working:**

- Driver registry supports all major vendors: Cisco IOS, Juniper Junos, Arista EOS, NX-OS API, Meraki Cloud, Generic SSH.
- Capability flags (`supports_commit`, `supports_rollback`, etc.) correctly represent vendor differences.
- Drivers distinguish between transport types (SSH CLI, NETCONF, HTTP REST, eAPI).

**Problems / Risks:**

- **ALL DRIVERS ARE SIMULATIONS**: Every driver uses `tokio::sleep` instead of real SSH/NETCONF/API calls. Example from `cisco_ios.rs`:
  ```rust
  // nauto_drivers/src/drivers/cisco_ios.rs:52
  let client = ssh::connect(device, &self.credential_store, self.port).await?;
  ```
  This `ssh::connect` returns immediately—no actual SSH connection is made. It's a mock.

- **Juniper NETCONF is incomplete**: `NetconfSession::connect` opens SSH but doesn't implement NETCONF protocol (hello exchange, RPC framing with `]]>]]>`).
  
- **Arista eAPI transport detection is naive**: Checks device tags for `"eapi"` string (line 377-385)—should use capability negotiation or explicit config.

- **NX-OS API driver has no actual HTTP logic**: Just simulates with sleep.

- **Meraki Cloud driver lacks token refresh**: Uses static API key—no OAuth2 flow, no token expiration handling.

- **No error handling for SSH host key verification**: Drivers will fail on first connection to new device (unknown host key).

- **No retry on connection failure**: Single attempt per device—transient network issues cause job failure.

- **Diff generation is limited**: Uses `similar::TextDiff` for config diffs but only shows first 200 changes (cisco_ios.rs:152)—truncates large diffs silently.

- **Rollback is not tested**: `rollback` method exists but no tests, no integration with job engine's failure handling.

**Recommendations:**

- [ ] **P0**: Implement real SSH execution in `nauto_drivers/src/ssh.rs` using `async_ssh2_tokio::Client::execute`:
  ```rust
  pub async fn connect(device: &Device, cred_store: &impl CredentialStore, port: u16) -> Result<Client> {
      let cred = cred_store.resolve(&device.credential).await?;
      let (username, password) = match cred {
          Credential::UserPassword { username, password } => (username, password),
          _ => bail!("SSH requires username/password"),
      };
      let addr = format!("{}:{}", device.mgmt_address, port);
      let client = Client::connect(&addr, &username).await?;
      client.authenticate_password(&password).await?;
      Ok(client)
  }
  ```
- [ ] **P0**: Remove all `tokio::sleep` simulations from drivers. Replace with real `client.execute(cmd)` calls.

- [ ] **P0**: Implement NETCONF protocol in `juniper_junos.rs` using existing SSH channel:
  - Send NETCONF hello message (`<hello><capabilities>...</capabilities></hello>]]>]]>`)
  - Parse server capabilities from response
  - Wrap all RPC calls with proper XML framing and `]]>]]>` delimiter
  - Handle errors from `<rpc-error>` responses

- [ ] **P0**: Add timeout to every SSH `execute` call using `tokio::time::timeout(30.seconds(), client.execute(...))`.

- [ ] **P0**: Implement HTTP client for Arista eAPI in `arista_eos.rs::run_command_batch_eapi`:
  ```rust
  async fn run_command_batch_eapi(&self, device: &Device, commands: &[String], res: &mut DriverExecutionResult) -> Result<()> {
      let cred = self.credential_store.resolve(&device.credential).await?;
      let url = format!("https://{}/command-api", device.mgmt_address);
      let json = json!({
          "jsonrpc": "2.0",
          "method": "runCmds",
          "params": { "version": 1, "cmds": commands, "format": "text" },
          "id": "1"
      });
      let response = self.http.post(&url)
          .basic_auth(&cred.username(), Some(&cred.password()))
          .json(&json)
          .send().await?
          .error_for_status()?
          .json::<EapiResponse>().await?;
      // Parse result.result[] and append to res.logs
  }
  ```

- [ ] **P0**: Implement NX-OS NX-API in `cisco_nxos_api.rs` using HTTP POST to `/ins`:
  ```rust
  let payload = json!({ "ins_api": { "version": "1.0", "type": "cli_show", "chunk": "0", "sid": "1", "input": command, "output_format": "json" }});
  let response = self.http.post(&url).json(&payload).send().await?;
  ```

- [ ] **P0**: Add host key acceptance policy to SSH connections—use `known_hosts` file or environment variable for first-connection acceptance mode.

- [ ] **P1**: Implement retry with backoff for transient connection failures (timeout, connection refused, DNS errors). Use `tokio_retry::Retry` with exponential backoff (2s, 4s, 8s).

- [ ] **P1**: Add configurable diff line limit or paginate large diffs. Warn user when truncation occurs.

- [ ] **P1**: Integrate rollback into `JobEngine` failure handling: if device task fails and `snapshot` exists, call `driver.rollback(device, snapshot)`.

- [ ] **P1**: Add driver behavior tests: mock SSH server for IOS, NETCONF server for Junos, HTTP server for Arista/NX-OS/Meraki. Verify command execution, config push, diff generation.

- [ ] **P2**: Support SSH key authentication in addition to password (already modeled in `Credential::SshKey` but not used in drivers).

- [ ] **P2**: Add TLS cert validation for HTTPS drivers (eAPI, NX-API, Meraki)—currently uses reqwest defaults but should allow custom CA bundle.

---

### 4. Security

**What's working:**

- Credentials stored in OS keyring via `keyring` crate—avoids plaintext passwords in config files.
- Fallback to encrypted file (`NAUTO_KEYRING_FILE` env var) when keyring unavailable.
- CLI supports `--password-stdin` and `--password-prompt` to avoid shell history exposure.
- Uses `spawn_blocking` for keyring I/O to avoid blocking Tokio runtime.
- SSH and HTTPS use encrypted transport (TLS/SSH).

**Problems / Risks:**

- **Fallback credential file is JSON plaintext**: `write_fallback_secret` (nauto_security/src/lib.rs:80-106) writes credentials as JSON without encryption—only filesystem permissions protect it.
- **No credential rotation**: Once stored, credentials never expire. No mechanism to force refresh from external vault or SSO.
- **No audit of credential access**: No log of who/when credentials were retrieved—compliance issue for SOC2/ISO27001.
- **Plugin WASM signature verification not implemented**: `marketplace::verify_signature` exists but is a stub—any .wasm file can be loaded.
- **No plugin sandboxing**: WASM plugins run with full host access via `wasmtime`—can read files, make network calls, access credentials. Missing WASI restrictions.
- **Audit log is append-only file**: `apps/nauto_cli/src/audit.rs` writes to local file—no tamper-proofing, no remote syslog, no centralized SIEM integration.
- **No TLS cert pinning**: HTTPS drivers accept any valid cert—MITM possible with rogue CA cert.
- **No secret scanning in CI**: No pre-commit hooks or CI checks for accidentally committed secrets.

**Recommendations:**

- [ ] **P0**: Encrypt fallback credential file using `age` or `sodiumoxide` crate. Derive encryption key from system keyring or hardware-backed key (TPM).
  ```rust
  // In nauto_security/src/lib.rs
  async fn write_fallback_secret(...) -> Result<()> {
      let encryption_key = derive_key_from_keyring()?; // Fetch master key from keyring
      let encrypted = encrypt_credentials(&credential, &encryption_key)?;
      std::fs::write(&path, encrypted)?;
  }
  ```

- [ ] **P0**: Implement WASM plugin signature verification using `ed25519-dalek`:
  ```rust
  // In apps/nauto_cli/src/marketplace.rs::verify_signature
  pub fn verify_signature(plugin_path: &Path, signature_path: &Path, pubkey: &ed25519_dalek::PublicKey) -> Result<bool> {
      let plugin_bytes = std::fs::read(plugin_path)?;
      let sig_bytes = std::fs::read(signature_path)?;
      let signature = ed25519_dalek::Signature::from_bytes(&sig_bytes)?;
      Ok(pubkey.verify(&plugin_bytes, &signature).is_ok())
  }
  ```

- [ ] **P0**: Restrict WASM plugin capabilities using WASI:
  - Deny filesystem access by default (no `--dir` mounts)
  - Deny network access (no `--inherit-network`)
  - Allow only explicit host functions (logging, device driver registration)
  - Use `wasmtime::Linker` to expose minimal API surface

- [ ] **P0**: Add audit logging for all credential access events:
  ```rust
  // In nauto_security/src/lib.rs::KeyringStore::resolve
  async fn resolve(&self, reference: &CredentialRef) -> Result<Credential> {
      let credential = /* ... */;
      audit_log(AuditEvent::CredentialAccess { 
          credential_name: reference.name.clone(), 
          timestamp: Utc::now(), 
          user: current_user()? 
      });
      Ok(credential)
  }
  ```

- [ ] **P1**: Support external credential providers (AWS Secrets Manager, HashiCorp Vault, Azure Key Vault):
  - Add `VaultStore` implementation of `CredentialStore` trait
  - Fetch secrets on-demand with caching (TTL 1 hour)
  - Refresh on expiration or explicit rotation event

- [ ] **P1**: Add TLS certificate pinning for critical API endpoints (Meraki Dashboard):
  ```rust
  let http = HttpClient::builder()
      .tls_built_in_root_certs(false)
      .add_root_certificate(load_pinned_cert()?)
      .build()?;
  ```

- [ ] **P1**: Send audit logs to remote syslog or SIEM (Splunk, Datadog):
  ```rust
  // In apps/nauto_cli/src/audit.rs
  pub async fn log_event(event: AuditEvent) -> Result<()> {
      let json = serde_json::to_string(&event)?;
      if let Ok(syslog_addr) = env::var("NAUTO_SYSLOG_ADDR") {
          udp_send(&syslog_addr, &json).await?;
      }
      local_append("logs/audit.log", &json)?;
  }
  ```

- [ ] **P2**: Add pre-commit hook and CI check for secret scanning using `trufflehog` or `gitleaks`.

- [ ] **P2**: Implement RBAC for CLI commands (job execution requires `operator` role, credential management requires `admin` role). Store roles in config file or LDAP.

---

### 5. UX: CLI, TUI, GUI

**What's working:**

**CLI:**
- Comprehensive subcommands: `run`, `creds`, `tui`, `compliance`, `schedule`, `gitops`, `approvals`, `notify`, `integrations`, `marketplace`, `bench`, `transactions`, `worker`, `observability`, `telemetry`.
- Consistent flag naming (--job, --inventory, --audit-log, --dry-run, --plan).
- Good error messages with context (uses `anyhow`).
- Progress indicator for job execution (ProgressBar).
- Supports job plans for staged rollout (canary + batches).

**TUI:**
- Uses `ratatui` for cross-platform terminal UI.
- Shows device list with filtering by tags.
- Detail view for selected device.

**GUI:**
- Tauri spike exists in `spikes/tauri_poc`.
- Separate web UI in `apps/web-ui` (not reviewed in detail).

**Problems / Risks:**

**CLI:**
- **No job status command**: Can't query status of running/completed jobs—must re-run to see results.
- **No log streaming**: Job execution shows spinner until complete—no live progress updates for individual devices.
- **Poor error formatting for large jobs**: On failure, prints all device errors in sequence—unreadable for 100+ device failures.
- **Inconsistent output formats**: Some commands output JSON (`compliance --format json`), others plain text—no `--output json` flag for all commands.
- **No shell completion**: Missing autocomplete for bash/zsh/fish—poor UX for frequent users.

**TUI:**
- **No live job monitoring**: Can't view running jobs, only device inventory.
- **No log/error details**: Shows device list but not recent job results or failure details.
- **No telemetry dashboard**: Can't view metrics, no charts/graphs.
- **Navigation is clunky**: Single keystroke commands (`q`, `j`, `k`) but no help text in UI.

**GUI:**
- **Not integrated with backend**: Tauri spike has mock data—no HTTP client to real job engine.
- **No authentication**: No login page, no session management, no RBAC.
- **No multi-user support**: Can't see other users' jobs or coordinate approvals.

**Recommendations:**

**CLI:**
- [ ] **P0**: Add `nauto_cli job status <job-id>` command to query job state from `JobStore`. Output table of device results with status/logs.
- [ ] **P0**: Stream job progress to stdout as devices complete (don't wait for all to finish):
  ```rust
  // In apps/nauto_cli/src/job_runner.rs
  while let Some(result) = tasks.next().await {
      println!("{} | {} | {}", result.device_id, result.status, result.logs.join(", "));
  }
  ```
- [ ] **P1**: Add `--output json|yaml|table` flag to all commands for structured output. Use `serde_json` and `comfy-table` crates.
- [ ] **P1**: Generate shell completion scripts using `clap_complete`:
  ```rust
  // In apps/nauto_cli/src/main.rs
  Commands::Completions { shell } => {
      clap_complete::generate(shell, &mut Cli::command(), "nauto", &mut io::stdout());
  }
  ```
- [ ] **P2**: Add `--summary-only` flag for large jobs—show success/failure counts instead of per-device details.

**TUI:**
- [ ] **P1**: Add "Jobs" tab showing active and recent jobs (last 100). Display job ID, name, status, device count, success/failure counts.
- [ ] **P1**: Add "Logs" panel at bottom showing tail of selected device's recent job logs.
- [ ] **P1**: Add "Metrics" dashboard showing telemetry snapshots (charts for device health, job latency).
- [ ] **P2**: Add help text overlay (press `?` to show keybindings).

**GUI:**
- [ ] **P1**: Implement HTTP/gRPC client in Tauri backend to call `nauto_service` endpoints (job submit, status, device list).
- [ ] **P1**: Add authentication: login page using OAuth2 or local user database; store JWT in session.
- [ ] **P2**: Add real-time job progress streaming using WebSocket or SSE from backend.
- [ ] **P2**: Add approvals workflow UI: show pending approvals, approve/reject buttons.

---

### 6. Plugin & Extensibility

**What's working:**

- Plugin SDK defines capability mask (`COMMIT`, `ROLLBACK`, `DIFF`, `DRY_RUN`) using `bitflags`.
- `export_plugin!` macro generates FFI exports for plugin metadata (vendor, device type, capabilities).
- WASM host (`spikes/wasm_host`) can load .wasm files using `wasmtime`.
- Marketplace commands (`list`, `install`, `verify`) exist in CLI.

**Problems / Risks:**

- **Plugin SDK has no runtime integration**: Exported plugin metadata (vendor, device_type) isn't read by driver registry—plugins can't register drivers.
- **No plugin lifecycle management**: No init/shutdown hooks, no hot reload, no version compatibility checks.
- **WASM host is isolated spike**: Not integrated into `nauto_drivers` or `nauto_engine`—can't actually extend functionality.
- **No host API for plugins**: Plugins can export metadata but have no way to call back into host (no logging, no credential access, no device interaction).
- **Marketplace is mock**: `marketplace::fetch_index` reads local JSON file—no real registry (crates.io-style), no download from HTTP.
- **Plugin installation is manual**: `marketplace::install` copies files—no dependency resolution, no version pinning.
- **No plugin sandboxing**: WASM plugins run with default WASI permissions—can access filesystem and network.
- **Signature verification is stub**: `verify_signature` returns `Ok(true)` hardcoded—no actual crypto check.

**Recommendations:**

- [ ] **P0**: Integrate WASM host into driver registry:
  ```rust
  // In nauto_drivers/src/lib.rs
  impl DriverRegistry {
      pub fn load_plugin(&mut self, wasm_path: &Path) -> Result<()> {
          let engine = wasmtime::Engine::default();
          let module = wasmtime::Module::from_file(&engine, wasm_path)?;
          let instance = wasmtime::Instance::new(&store, &module, &[])?;
          let vendor = extract_plugin_metadata(&instance)?;
          let driver = WasmDeviceDriver::new(instance, vendor);
          self.drivers.push(Arc::new(driver));
      }
  }
  ```

- [ ] **P0**: Define host API for plugins using `wasmtime::Linker`:
  ```rust
  // In spikes/wasm_host/src/main.rs (move to crate nauto_plugin_host)
  linker.func_wrap("host", "log", |level: u32, msg_ptr: u32, msg_len: u32| {
      let msg = read_string_from_wasm_memory(msg_ptr, msg_len);
      tracing::event!(target: "plugin", Level::from(level), "{}", msg);
  })?;
  linker.func_wrap("host", "execute_command", |device_id_ptr: u32, cmd_ptr: u32| -> u32 {
      // Call into engine to execute command on device
  })?;
  ```

- [ ] **P0**: Implement signature verification (see Security section above).

- [ ] **P1**: Create plugin marketplace registry:
  - Host index at `https://plugins.nauto.dev/index.json` (or GitHub repo)
  - Index contains plugin name, version, SHA256, signature URL
  - `marketplace::fetch_index` downloads via HTTP
  - `marketplace::install` fetches .wasm and .sig, verifies signature before installing

- [ ] **P1**: Add plugin version compatibility checks:
  ```rust
  #[derive(Deserialize)]
  pub struct PluginManifest {
      pub min_nauto_version: semver::Version,
      pub max_nauto_version: Option<semver::Version>,
  }
  pub fn check_compatibility(manifest: &PluginManifest) -> Result<()> {
      let current = semver::Version::parse(env!("CARGO_PKG_VERSION"))?;
      if current < manifest.min_nauto_version {
          bail!("plugin requires nauto >= {}", manifest.min_nauto_version);
      }
  }
  ```

- [ ] **P2**: Add plugin hot reload: watch plugin directory for changes, unload old version, load new version without restarting service.

- [ ] **P2**: Create plugin development guide: sample plugin, build instructions, testing harness.

---

### 7. Testing, CI, and Docs

**What's working:**

**Testing:**
- Unit tests for core logic: `JobEngine::runs_job_across_devices`, `ComplianceEngine::evaluates_contains_and_not`, `collect_all_filters_failures`.
- Uses `MockDriver` for testing engine without real drivers.

**CI:**
- GitHub Actions workflow (`.github/workflows/ci.yml`) runs on push/PR.
- Linting: `cargo fmt --check` and `cargo clippy`.
- Tests: `cargo test` on ubuntu/macos/windows matrix.
- Security: `cargo audit` on ubuntu.
- Tauri smoke build in separate job.

**Docs:**
- Comprehensive markdown docs in `docs/`: architecture, drivers, security, production readiness, etc.
- Release notes, roadmap, testing report.
- Quick start guide with example commands.

**Problems / Risks:**

**Testing:**
- **Only 6 tests total** across 140k lines of Rust code—virtually no coverage.
- **No integration tests**: No end-to-end test of CLI -> Engine -> Driver -> Result.
- **No driver behavior tests**: Drivers are mocked/simulated—no validation of SSH/NETCONF/API logic.
- **No failure path tests**: No tests for timeouts, connection failures, config rollback, job cancellation.
- **No compliance engine tests beyond basic string matching**.
- **No telemetry collector tests** (SNMP, gNMI, HTTP collectors untested).
- **No security tests**: Keyring storage, signature verification, credential encryption not tested.

**CI:**
- **No e2e test job**: CI only runs unit tests—doesn't exercise full job execution flow.
- **Tauri build excluded from main matrix**: Separated into tauri-smoke job—may diverge from main workspace.
- **No benchmark regression tests**: `bench` command exists but not run in CI—performance regressions undetected.
- **No dependency update automation**: No Dependabot or Renovate—security vulnerabilities in dependencies may go unnoticed.
- **Clippy warnings not enforced**: CI runs `clippy` but doesn't fail on warnings (should use `-- -D warnings`).

**Docs:**
- **Docs claim features not implemented**: E.g., `service_architecture.md` describes HTTP control plane and distributed workers—neither exist.
- **Driver docs don't mention simulation**: `drivers.md` lists capabilities but doesn't warn that drivers don't actually connect to devices.
- **No API reference**: No rustdoc published, no trait documentation, no usage examples.
- **Quick start examples may not work**: Commands reference files that may not exist (examples/jobs/show_version.yaml).

**Recommendations:**

**Testing:**
- [ ] **P0**: Add integration tests in `tests/integration/` directory:
  - `test_run_job_e2e.rs`: Load job YAML, load inventory YAML, execute job via `JobEngine`, assert on result counts.
  - `test_cli_commands.rs`: Use `assert_cmd` crate to test CLI commands (run, creds, compliance).
  - `test_driver_simulation.rs`: Validate mock drivers return expected results for each job type.

- [ ] **P0**: Add driver behavior tests with mock servers:
  ```rust
  // In crates/nauto_drivers/tests/cisco_ios_test.rs
  #[tokio::test]
  async fn cisco_ios_executes_show_version() {
      let mock_ssh_server = start_mock_ssh_server("show version", "Cisco IOS Version 15.2").await;
      let driver = CiscoIosDriver::default();
      let device = test_device("10.0.0.1", mock_ssh_server.port());
      let result = driver.execute(&device, DriverAction::Job(&JobKind::CommandBatch { ... })).await?;
      assert!(result.logs[0].contains("Cisco IOS Version 15.2"));
  }
  ```

- [ ] **P0**: Add failure path tests:
  - Test device timeout (mock device that never responds).
  - Test connection refused (no server listening).
  - Test config rollback on failure (driver returns error, verify rollback called).

- [ ] **P1**: Add security tests:
  - Test keyring store/resolve round-trip.
  - Test fallback file encryption.
  - Test plugin signature verification (valid signature passes, invalid fails).

- [ ] **P1**: Add benchmark regression tests in CI:
  ```yaml
  # In .github/workflows/ci.yml
  - name: Benchmark
    run: |
      cargo build --release
      ./target/release/nauto_cli bench --devices 1000 --parallel 200 > bench_output.txt
      python scripts/check_benchmark_threshold.py bench_output.txt  # Fail if slower than baseline
  ```

**CI:**
- [ ] **P0**: Enforce clippy warnings as errors:
  ```yaml
  # In .github/workflows/ci.yml
  - name: Cargo clippy
    run: cargo clippy --all-targets --all-features -- -D warnings
  ```

- [ ] **P0**: Add e2e test job:
  ```yaml
  e2e-test:
    runs-on: ubuntu-latest
    steps:
      - run: cargo build --release
      - run: ./scripts/e2e_test.sh  # Runs CLI against test inventory, validates output
  ```

- [ ] **P1**: Add Dependabot config for automated dependency updates:
  ```yaml
  # .github/dependabot.yml
  version: 2
  updates:
    - package-ecosystem: "cargo"
      directory: "/"
      schedule: { interval: "weekly" }
  ```

- [ ] **P2**: Publish rustdoc to GitHub Pages on each release:
  ```yaml
  # In .github/workflows/docs.yml
  - run: cargo doc --no-deps --all-features
  - uses: peaceiris/actions-gh-pages@v3
    with: { publish_dir: ./target/doc }
  ```

**Docs:**
- [ ] **P0**: Add disclaimers in `README.md` and `docs/drivers.md` that drivers are currently simulated and don't connect to real devices.
- [ ] **P1**: Sync `service_architecture.md` with actual implementation—mark unimplemented features as "planned".
- [ ] **P1**: Add code examples to `docs/quick_start.md` that reference real files in `examples/` directory.
- [ ] **P2**: Generate and publish API reference docs (rustdoc) for public APIs.

---

## Actionable Task List

### P0: Critical Correctness / Security / Scalability Issues

**Drivers & Protocol:**
- [ ] Remove `tokio::sleep` simulations from all drivers (`cisco_ios.rs`, `juniper_junos.rs`, `arista_eos.rs`, `cisco_nxos_api.rs`, `meraki_cloud.rs`, `generic_ssh.rs`).
- [ ] Implement real SSH execution in `nauto_drivers/src/ssh.rs::connect` using `async_ssh2_tokio::Client::execute`.
- [ ] Implement NETCONF protocol in `juniper_junos.rs::NetconfSession` (hello exchange, RPC framing with `]]>]]>`).
- [ ] Implement Arista eAPI HTTP client in `arista_eos.rs::run_command_batch_eapi` using `reqwest`.
- [ ] Implement NX-OS NX-API HTTP client in `cisco_nxos_api.rs` using JSON-RPC over `/ins`.
- [ ] Add timeout wrapper to every driver operation: `tokio::time::timeout(30.seconds(), client.execute(...))`.

**Concurrency & Scalability:**
- [ ] Add per-device timeout to `JobEngine::execute` (wrap `run_device` in `tokio::time::timeout`).
- [ ] Implement graceful task cancellation (store `JoinHandle`s, expose `cancel()` method that calls `abort()`).
- [ ] Stream results to `JobStore` instead of accumulating in memory (refactor `device_results` vector).
- [ ] Make `execute_compliance_job` async and parallel (use `FuturesUnordered` like other job types).
- [ ] Replace `expect` on semaphore with `ok()?` to prevent panic on cancellation.

**Security:**
- [ ] Encrypt fallback credential file using `age` or `sodiumoxide` (derive key from keyring).
- [ ] Implement WASM plugin signature verification using `ed25519-dalek`.
- [ ] Restrict WASM plugin capabilities using WASI (deny filesystem/network access by default).
- [ ] Add audit logging for credential access events (who/when/which credential).

**Architecture:**
- [ ] Define and implement `JobStore` trait for persisting job state and results (sqlite MVP, Postgres production).
- [ ] Add `JobQueue` trait with Redis/SQS/Postgres implementations (remove JSONL file dependency).

**Testing:**
- [ ] Add integration tests: `test_run_job_e2e`, `test_cli_commands`, `test_driver_simulation`.
- [ ] Add driver behavior tests with mock SSH/NETCONF/HTTP servers.
- [ ] Add failure path tests (timeout, connection refused, rollback on error).
- [ ] Add security tests (keyring round-trip, signature verification).

**CI:**
- [ ] Enforce clippy warnings as errors (`-- -D warnings` in CI).
- [ ] Add e2e test job to CI (run CLI against test inventory, validate output).

**Docs:**
- [ ] Add disclaimer in `README.md` that drivers are currently simulated (not production-ready).

---

### P1: Important But Not Blocking

**Drivers & Protocol:**
- [ ] Add retry with exponential backoff for transient connection failures (use `tokio_retry`).
- [ ] Integrate rollback into `JobEngine` failure handling (call `driver.rollback` on task failure).
- [ ] Add configurable diff line limit (warn when truncation occurs).
- [ ] Support SSH key authentication (use `Credential::SshKey` in drivers).

**Concurrency & Scalability:**
- [ ] Add retry logic with exponential backoff for transient failures in `run_device`.
- [ ] Add telemetry for active tasks, queued tasks, task latency distribution.

**Security:**
- [ ] Support external credential providers (AWS Secrets Manager, Vault, Azure Key Vault).
- [ ] Add TLS certificate pinning for critical API endpoints (Meraki Dashboard).
- [ ] Send audit logs to remote syslog or SIEM (Splunk, Datadog).

**Architecture:**
- [ ] Implement device locking mechanism (Redis distributed lock or DB advisory locks).
- [ ] Integrate transaction plan execution into `JobEngine` (canary batch, staged rollout, auto-rollback).
- [ ] Add `ApprovalStore` trait backed by database (replace file-based approvals).

**UX:**
- [ ] Add `nauto_cli job status <job-id>` command to query job state.
- [ ] Stream job progress to stdout as devices complete (don't wait for all).
- [ ] Add `--output json|yaml|table` flag to all CLI commands.
- [ ] Generate shell completion scripts using `clap_complete`.
- [ ] Add "Jobs" tab to TUI showing active and recent jobs.
- [ ] Add "Logs" panel to TUI showing selected device's recent job logs.
- [ ] Add "Metrics" dashboard to TUI.
- [ ] Implement HTTP/gRPC client in Tauri backend (call `nauto_service` endpoints).
- [ ] Add authentication to GUI (login page, OAuth2, JWT session).

**Plugins:**
- [ ] Integrate WASM host into driver registry (load plugins at startup, register drivers).
- [ ] Define host API for plugins using `wasmtime::Linker` (logging, credential access, command execution).
- [ ] Create plugin marketplace registry (HTTP index, download, verify signature).
- [ ] Add plugin version compatibility checks (min/max nauto version).

**Testing:**
- [ ] Add benchmark regression tests in CI (fail if slower than baseline).

**CI:**
- [ ] Add Dependabot config for automated dependency updates.

**Docs:**
- [ ] Sync `service_architecture.md` with actual implementation (mark unimplemented features).
- [ ] Add code examples to `docs/quick_start.md` that reference real files.

---

### P2: Nice-to-Have / Cleanup

**Drivers & Protocol:**
- [ ] Add TLS cert validation for HTTPS drivers (allow custom CA bundle).

**Security:**
- [ ] Add pre-commit hook and CI check for secret scanning (`trufflehog` or `gitleaks`).
- [ ] Implement RBAC for CLI commands (operator vs admin roles).

**Architecture:**
- [ ] Create HTTP/gRPC control plane service (`apps/nauto_service`) exposing job submit/status/cancel endpoints.

**UX:**
- [ ] Add `--summary-only` flag for large jobs (show counts, not per-device details).
- [ ] Add help text overlay to TUI (press `?` to show keybindings).
- [ ] Add real-time job progress streaming to GUI (WebSocket or SSE).
- [ ] Add approvals workflow UI to GUI (show pending, approve/reject buttons).

**Plugins:**
- [ ] Add plugin hot reload (watch directory, unload old, load new).
- [ ] Create plugin development guide (sample plugin, build instructions, testing harness).

**Testing:**
- [ ] Publish rustdoc to GitHub Pages on each release.

**Docs:**
- [ ] Generate and publish API reference docs (rustdoc) for public APIs.

---

## Final Assessment

The netrust codebase demonstrates **strong architectural design** and **ambitious scope**, but is currently a **well-structured prototype** rather than production-ready software. The separation of concerns (model, drivers, engine, security, UX) is exemplary and will support future growth.

However, **all network drivers are simulations**—no actual SSH, NETCONF, or API communication occurs. This is the **single biggest blocker** to production use. Additionally, **lack of per-device timeouts, task cancellation, and result streaming** makes the system unsuitable for large-scale deployments (10k+ devices).

Security is **above average** for an early-stage project (keyring-backed credentials, audit logging, encrypted transport), but **credential file encryption, plugin signature verification, and WASM sandboxing** must be implemented before public release.

Testing coverage is **critically low** (6 tests, no integration tests, no failure path coverage)—will not catch regressions or edge cases.

**Recommendation**: Focus P0 tasks on implementing real drivers and adding timeouts/cancellation/streaming before pursuing P1/P2 features. With 2-3 months of focused effort on P0 items, this codebase can reach MVP quality for pilot deployments (100s of devices). Production scale (10k+ devices) will require P0 + most P1 tasks (distributed workers, job state persistence, full observability).

---

**End of Review**
