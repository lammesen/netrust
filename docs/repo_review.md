Netrust Code Review (Post-Refactor v0.1.0)

Implementation Update (2025-11-17)
- Transport drivers now honor configurable SSH/HTTP timeouts & retries, NX-OS rollback executes real NX-API payloads, and the Mozilla CA bundle is shipped in-repo for consistent TLS builds.
- Telemetry collectors received regression coverage, the worker queue executes real jobs (daemon + integration tests), and audit logging captures per-device rows while the CLI/TUI/GUI surfaces richer status.
- WASM plugin metadata (vendor/device type/capabilities) load on startup with signature enforcement, readying the host for future runtime driver registration hooks.

Code Quality and Idiomatic Rust Usage
Overall, the codebase is well-structured and idiomatic. The project is organized as a Cargo workspace with multiple crates (e.g. nauto_model, nauto_engine, nauto_drivers, nauto_security, etc.), which enforces separation of concerns
GitHub
. Data structures and enums in nauto_model use Serde for easy YAML/JSON serialization, which is ideal for configuration files and interchange formats. For example, the JobKind enum is annotated with Serde tags so that job files can specify the type (command_batch, config_push, etc.) in a clear way
GitHub
. Similarly, TargetSelector supports all, by_ids, or by_tags in a straightforward manner
GitHub
. These design choices make the tool’s input files intuitive for end-users. The Rust code adheres to modern idioms. Error handling primarily uses the anyhow crate for simplicity in the CLI context, with liberal use of ? and context messages for debugging (e.g. reading files, making HTTP requests)
GitHub
GitHub
. This is appropriate for a user-facing tool where having backtraces and rich error context is more important than fine-grained error types. Where needed, custom error types are defined (see JobEngineError with a variant for missing drivers)
GitHub
, but the code doesn’t overuse complex error enums – a pragmatic balance. Ownership and borrowing are handled correctly, with no obvious memory safety issues. There is extensive use of Arc<dyn DeviceDriver> to share driver instances, which makes sense given drivers are stateless (or internally manage state) and can be used concurrently. Trait objects (dyn) are used here to allow heterogeneous collections of drivers in the registry
GitHub
. The use of the async_trait crate to allow async functions in traits (for device drivers and inventory) is idiomatic for current stable Rust
GitHub
. All such async trait functions return Result<…> so errors are propagated and logged appropriately. The code avoids panics/unwraps in favor of error returns – for example, if a plugin isn’t found in the marketplace, it uses context("plugin not found")? to produce a user-friendly error
GitHub
. Rust best practices like #[derive(Debug)] on structs and implementing Default where it makes sense (e.g. for drivers and simple structs) are followed. Clippy and rustfmt have been run (confirmed by CI), so the code is consistently formatted and free of common lint warnings
GitHub
. The team even included a Cargo.lock and CI for dependency auditing, indicating attention to supply-chain security. One minor nit: the DriverRegistry.find() does a linear search through a vector of drivers
GitHub
. Given the number of drivers is small, this is not a performance issue, but using a HashMap keyed by DeviceType would make lookups O(1) and avoid cloning the Arc (though cloning an Arc is trivial). This could be a small refactor for cleanliness, but functionally it’s correct. Another positive aspect is the attention to secure coding practices in the CLI. For example, the nauto_cli creds command avoids forcing secrets on the command line. It supports --password-stdin and --password-prompt so that users can input credentials without them appearing in shell history or process args
GitHub
GitHub
. In fact, if --password is used, the code explicitly prints a warning reminding the user of the security implications
GitHub
. This shows a mature consideration for real-world usage and aligns with the documentation’s emphasis on security. Overall, code quality is high. The recent refactor has maintained clean abstractions (devices, jobs, drivers) and the coding style is idiomatic. The few areas for improvement in code quality are very minor – for instance, some modules (like the compliance engine) use simple string matching for rules, which might be expanded to more robust parsing later, but the current implementation is straightforward and well-documented.
Async and Concurrency Behavior
The Netrust engine makes heavy use of async/await and is designed to scale to thousands of concurrent device tasks, leveraging Rust’s strengths in asynchronous concurrency. The core job execution loop in JobEngine is a great example of idiomatic async concurrency. When a job is executed, the engine first resolves the target devices (potentially filtering by tags or IDs) via the inventory trait asynchronously
GitHub
. Then it uses a Tokio semaphore to limit parallelism to a configured level (32 by default, or an override in the job)
GitHub
. It spawns an async task for each device using tokio::spawn, but each task must acquire a permit from the semaphore before proceeding, ensuring that at most N devices are active concurrently
GitHub
GitHub
. This pattern – spawning all futures then using a semaphore guard – is idiomatic and efficient. The use of FuturesUnordered to collect tasks and drive them to completion is also a proper choice for concurrently executing a dynamic set of tasks
GitHub
GitHub
. The code awaits each task’s result and logs any join errors (which would indicate a panic in a driver task) without crashing the whole engine
GitHub
. This means one device’s failure won’t take down the entire job – a critical property for long-running automations. Within each device task, the design uses structured concurrency: there’s per-device logging via tracing spans (with the device name and job type attached)
GitHub
, and the task ultimately produces a TaskSummary result for that device. Notably, the code explicitly drops the semaphore permit as soon as the device task is done
GitHub
, which is good practice (though the drop would happen on scope exit anyway, this makes it clear). This design is both idiomatic and robust – it can scale to many tasks and enforces limits to avoid overloading resources. The plan documentation highlights that Tokio’s async runtime can handle tens of thousands of connections, and the implementation here is aligned with that guidance
GitHub
. A quick benchmark is provided via the nauto_cli bench command: for example, running 1000 devices with --parallel 200 completed successfully in tests
GitHub
, demonstrating the engine’s scalability. One area for future enhancement is cancellation and timeout. Currently, if a job is running and a device hangs (say an SSH session that never returns), there’s no built-in timeout except the ones implicit in drivers (e.g. the Meraki HTTP client has a 15s timeout on requests
GitHub
). It would be prudent to allow a global timeout for device tasks or job execution as a whole, or to provide a cancellation mechanism (perhaps via a Ctrl-C handler or an API call to cancel). Tokio provides cancellation via dropping tasks or using select! with a timeout, which could be integrated in the future. For now, the concurrency model is sound. Another possible improvement is batching or staggering in extremely large jobs. The current semaphore ensures at most N concurrent, but if you had, say, 10,000 devices and max_parallel=1000, the engine will spawn 10k tasks immediately (with 9k waiting on permits). Tokio can handle that, but it might be more memory-efficient to spawn tasks incrementally. This is an edge consideration; there’s no evidence of issues at the scales tested so far. The design already mentions batch sizes and canary deployments for staged rollouts
GitHub
, which are handled outside the engine (the transactions subcommand creates a plan file splitting devices into canary + batches). That approach is reasonable: use offline planning to batch, and let the engine execute each batch separately. In summary, the async design is working well: it’s idiomatic (using async_trait, tokio::spawn, futures streams, etc.), and the recent changes have kept it robust. The project leverages Tokio’s multithreading effectively – for example, the TUI is launched via tokio::task::spawn_blocking so that the UI can run in a separate thread without blocking the async runtime
GitHub
. This attention to async details shows senior-level understanding of Rust concurrency. As the project moves toward production, adding timeouts and perhaps dynamic adjustment of concurrency based on success/failure rates would be worth considering, but the core is solid.
Overall Architecture (Job Engine, Drivers, Plugin Model)
The high-level architecture is clearly modeled in the code and matches the design documentation
GitHub
. The Job Engine (in nauto_engine) orchestrates the execution of jobs across devices, treating each device operation almost like an isolated transaction. It relies on an abstraction of an inventory (DeviceInventory trait) and a registry of drivers (DriverRegistry) to remain decoupled from specific device details
GitHub
GitHub
. The engine is straightforward: given a Job (which includes its JobKind and target selector), it resolves the target devices and then executes the action on each device via the appropriate driver. It also handles pre-and post-job bookkeeping like timestamps and auditing. This layering means the engine doesn’t need to know how an Arista switch vs. a Meraki cloud device is managed – it just asks the driver to execute. The Drivers layer is well-designed as a polymorphic interface (DeviceDriver trait) with capability flags to indicate special support (commit, rollback, diff, dry-run)
GitHub
. Each vendor/protocol has its own driver struct implementing that trait. The recent refactor introduced drivers for Cisco IOS CLI, Juniper Junos (NETCONF), Arista EOS, Cisco NX-OS API (HTTP-based), Meraki Cloud (REST API), and a Generic SSH driver
GitHub
. This covers a wide range of network device interaction models. Currently, most of these drivers are stub implementations (they simulate actions with log messages and tokio::time::sleep to mimic delays). For example, the Cisco IOS driver’s execute() function just logs the command and pretends to write config, adding a “write memory” log at the end
GitHub
GitHub
. The Juniper driver simulates loading a snippet, doing a commit check, and commit confirm, all with dummy delays
GitHub
GitHub
. These simulations are useful for testing the engine’s concurrency and logic without needing real hardware. The Meraki driver is the one exception – it actually performs real HTTP calls to the Meraki cloud API using reqwest
GitHub
GitHub
, demonstrating how a real driver might be implemented. The presence of capability flags is already paying off: e.g. JuniperJunosDriver.capabilities() returns supports_commit=true, etc., which the engine uses to decide if it can do a dry-run or not
GitHub
. This is a smart way to allow or skip certain phases (like commit confirmation or diff generation) based on device support. Because drivers are encapsulated, adding new device types or protocols is straightforward. In the future, one could imagine adding a Huawei driver or a gNMI-based driver by implementing the trait and adding it to the registry – no changes needed in the engine. The DriverRegistry simply holds a list of Arc<dyn DeviceDriver> and lets you find one by DeviceType
GitHub
. The use of an enum DeviceType (with variants for CiscoIos, JuniperJunos, etc.) in the Device model ties everything together – each Device knows what type it is, and that keys into the registry. This is working well; the tests show that if a driver isn’t available, the engine will mark that device as “Skipped” with a log message
GitHub
, so the system degrades gracefully if asked to handle an unsupported device. The Plugin model is in an early prototype stage but is thoughtfully designed. The project has a crate nauto_plugin_sdk that defines a lightweight WASM plugin interface (using bitflags for capability masks and a macro to export plugin metadata)
GitHub
. There’s evidence of a WASM host (under spikes/wasm_host) using Wasmtime, and a sample guest plugin (spikes/wasm_plugin) that declares a vendor driver via export_plugin! macro
GitHub
. The CLI has already been wired with a nauto_cli marketplace command group to list, install, and verify plugins from a marketplace index
GitHub
GitHub
. This recent addition is a great step toward extensibility – it means third parties (or internal teams) could develop new drivers or custom job logic in any language that compiles to WASM, and distribute them without rebuilding the core binary. The review of marketplace.rs shows it can read a JSON index of plugins and copy plugin artifacts into an install directory, as well as compute their SHA-256 for verification
GitHub
GitHub
. That’s a good security measure, though currently it just prints the expected signature rather than enforcing it – an area for improvement noted in docs
GitHub
. What needs rework in architecture is mostly completing the integrations between these pieces: the plugin system isn’t yet tied into the running engine. For example, there’s no code (yet) that loads a .wasm plugin at runtime and registers a new DeviceDriver with the DriverRegistry. The design is such that it’s feasible – e.g., a plugin could expose a function to create a driver instance and the host could call it and insert into the registry. Implementing that (and deciding when to load plugins – on startup, or on-demand when a DeviceType isn’t found?) will be a next step. The docs suggest using WIT (WebAssembly Interface Types) to define a host-guest API for registering drivers and logging, which is a solid approach
GitHub
. Another architectural aspect to highlight is the planned “service mode” with an API, scheduler, and worker nodes for distributed operation
GitHub
GitHub
. The code already has placeholders for this: a worker_daemon.rs that continuously polls a queue file and would invoke jobs
GitHub
GitHub
, as well as a CLI worker command to process a batch of queued jobs on demand
GitHub
GitHub
. As of now, these just log the intended actions (no actual job execution is invoked yet), but the structure is in place. This is wise architecturally: it means the same binary can function as a CLI for interactive use or as a long-running agent processing jobs from a message queue. To fully realize this, the team will need to implement the actual queue insertion and retrieval logic (likely replacing the JSONL file with a more robust message broker or database) and link it to the JobEngine.execute calls. Given the groundwork laid and the clear separation of concerns, this should be achievable without major refactoring. In summary, the architecture is strong and extensible. The recent changes (new driver modules, plugin system, queue/worker scaffolding) have been integrated without breaking the overall design. The next steps are mostly filling in the “guts” of these systems (real network operations in drivers, real plugin loading, real API/worker integration). No major architectural overhaul is needed at this point – just incremental improvements to reach production grade.
SSH, NETCONF, RESTCONF, and API-based Network Communication
Currently, real network communication is implemented in a limited fashion – likely by design for the MVP. The only fully implemented network API interaction is the Meraki Cloud driver, which uses reqwest to call Cisco’s Meraki REST API
GitHub
. It retrieves an API token from the secure credential store and makes HTTPS calls to Meraki’s endpoints, handling errors and HTTP response codes properly
GitHub
GitHub
. This is a strong proof of concept that the system can securely manage API keys (it warns if a wrong credential type is used, and fetches the token from the OS keychain via KeyringStore)
GitHub
GitHub
. The Meraki driver sets supports_rollback=true
GitHub
, though the actual rollback() implementation currently just logs a message (real rollback might involve reapplying an old template or using the Meraki dashboard’s capabilities)
GitHub
. Implementing that fully would require storing some form of snapshot ID or prior state, which isn’t done yet. For now, it’s acceptable to treat rollback as a no-op with a log. For SSH and CLI-based network communication (Cisco IOS, Arista EOS, Generic SSH), the drivers are placeholders. They use simple async sleep to simulate round-trip time and push log messages as if commands were run
GitHub
GitHub
. What’s working well here is the consistency of the interface – the drivers all produce DriverExecutionResult objects with logs, and sometimes diffs or pre/post snapshots, in a uniform way. This means once real communication is added, the rest of the system (the engine, audit log, compliance subsystem) can remain unchanged. The simulations also made it easy to test concurrency (the bench command uses 1000 GenericSshDriver devices to gauge throughput without needing 1000 SSH servers
GitHub
GitHub
). However, to meet production-grade requirements, these drivers will need actual SSH and NETCONF implementations:
SSH for CLI (Cisco IOS, Arista, Generic SSH): We recommend integrating an async SSH client library. There are a few options in Rust: russh (pure Rust async SSH) or ssh2-rs (libssh2 bindings, which are synchronous and would require using spawn_blocking). Given the preference for pure async, russh could be tried, or even higher-level crates if available. The driver would need to open an SSH session to the device’s mgmt_address, authenticate (likely using username/password or key from CredentialStore), and then send each command, capturing the output. Capturing output is important for logs and for diff calculation. Right now, the code just echoes the command as the log, but in a real scenario, the log might be the device’s response. For example, running show version on a Cisco router should capture the actual version output. Storing these outputs could be memory-heavy, so perhaps an option to save only to file or stream them to an audit log might be needed for large-scale runs. But at least printing a snippet or a success message per device is expected.
NETCONF for Juniper (and possibly others): The JuniperJunos driver currently simulates commit check/confirm steps
GitHub
. In reality, Junos would use NETCONF (over SSH) or its CLI “configure” mode. The supports_diff=true and other flags on Junos indicate the driver knows how to do candidate configuration management
GitHub
. Implementing this could be done via a NETCONF client library (some Rust libraries exist in varying maturity) or by shelling out to ssh with a NETCONF subsystem. A simpler approach might be to use Junos’s CLI in batch mode (feeding commands and a commit, then capturing the diff via commit confirmed or show | compare). Given the timeframe, using NETCONF is cleaner. There is a crate like netconf-client that could be explored. Regardless, the driver should perform: lock config, load snippet (or set commands), commit check (if dry-run or explicit check requested), commit (with or without confirm), and possibly rollback if needed. The code structure in apply_config() for Junos shows exactly where those steps go
GitHub
GitHub
 – so it’s a matter of replacing the sleep() calls with real NETCONF RPCs and interpreting replies.
NX-OS NX-API and Arista eAPI (HTTP/RESTCONF): The CiscoNxosApi driver is very close to real – it constructs a JSON payload with NX-API’s format for CLI commands and configuration, and it even prepares to POST to an https://<device>/ins endpoint
GitHub
GitHub
. Currently, it calls a simulate_post() which just logs the URL and payload without sending
GitHub
. To implement this for real devices, we’d use the reqwest client (already in the struct) to send the payload, similar to the Meraki driver. NX-OS’s API will return JSON results; the driver can decide how much to parse. For example, it could look at the response to determine success or failure of each CLI command (NX-API returns a status per command). For now, simply checking HTTP status and maybe logging a truncated output or “OK” might suffice, with errors bubbled up if the HTTP call fails. Similarly, Arista EOS offers eAPI (JSON-RPC over HTTP) – the Arista driver currently simulates CLI through SSH, but we could convert it to use eAPI by sending commands to https://<device>/command-api with appropriate JSON. Alternatively, Arista could also be managed via ssh CLI like Cisco IOS; both approaches are used in practice. The decision might depend on user preferences (perhaps support both modes in the future). In any case, having at least one real CLI-based interaction (either by SSH or by eAPI HTTP) in the system soon would elevate the toolkit from simulation to real device control.
One more protocol: RESTCONF/gNMI – these are mentioned (telemetry uses gNMI in a limited way, see below). While not explicitly separate drivers yet, it’s conceivable to have drivers for devices that support RESTCONF (which is just REST API following YANG models) or gNMI (gRPC-based streaming telemetry and config). Those could be future additions. Given the current scope, focusing on SSH and the existing HTTP APIs covers the primary use cases. Telemetry collection for SNMP/gNMI is another form of network communication. The nauto_telemetry crate defines SnmpCollector, GnmiCollector, and HttpCollector, but these are also stubbed (they just return fake metrics after a delay)
GitHub
GitHub
. Improving these to fetch real data would involve using an SNMP library or executing snmpwalk commands for SNMP, and using a gNMI client (which would involve generating gRPC code from OpenConfig .proto files or using an existing gNMI Rust implementation). This is non-trivial, but as a future enhancement, it aligns with making the toolkit useful for monitoring. The groundwork is laid: the telemetry subsystem can output JSON or CSV of collected metrics across devices
GitHub
GitHub
. For now, it’s a nice simulation; making it real would again involve integrating with external crates or services. In summary, network communication in the current code is the biggest area where real implementation is needed. The design is ready for it – methods like execute() and rollback() in each driver have the right signatures and error handling. The recommendation is to introduce well-tested libraries for SSH, NETCONF, and HTTP API calls to replace the simulated logic. This will likely be the most significant chunk of work to go from prototype to production. The recent changes have at least demonstrated how each protocol would be handled (which is a valuable blueprint), and the Meraki integration shows this approach working (e.g., handling timeouts, status codes, JSON payloads). The team should follow that example for the other drivers. Once done, it will greatly expand Netrust’s practical utility in multi-vendor network automation.
CLI, TUI, and GUI User Experience
The user experience provided by Netrust spans a traditional CLI, a terminal UI, and a prototype GUI – an ambitious set of interfaces that cater to different user preferences. Command-Line Interface (CLI): The CLI (nauto_cli) is well-organized into subcommands that mirror real network automation tasks. Recent changes have introduced a rich set of commands (as listed in the README and implemented in code)
GitHub
GitHub
. Key examples include: run (execute a job file against an inventory), compliance (evaluate compliance rules), telemetry (collect and output telemetry data), bench (benchmark concurrency), worker (process job queue items), integrations (import from NetBox, etc.), approvals (manage job approvals workflow), notify (send notifications), and marketplace (manage plugins). This breadth shows foresight in covering not just the “run config on devices” case, but also compliance auditing, telemetry gathering, scheduling (via the schedule subcommand), and even GitOps integration (gitops subcommand for syncing configs with a git repo). Each of these commands is implemented in a separate module, which keeps the codebase modular and easier to maintain
GitHub
. The CLI flags and arguments are defined using clap derive macros, resulting in a consistent UX. Defaults are provided for convenience – e.g., nauto_cli run defaults the audit log path and can toggle --dry-run easily
GitHub
, nauto_cli worker defaults to looking at queue/jobs.jsonl with a limit of 5 jobs at a time
GitHub
, etc. These sensible defaults mean a new user can run example jobs with minimal setup (as shown in the Quick Start docs). The CLI also guides secure usage: as mentioned, the creds command smartly avoids plain-text passwords unless forced, and even then provides warnings
GitHub
. Another nice touch: the ObservabilityCmd uses an enum for output format (text vs JSON) and actually honors it – if --format json is given, the metrics are output as structured JSON, otherwise as Prometheus text
GitHub
. This was a recent fix; previously it always output text ignoring the flag, which could confuse users expecting JSON
GitHub
GitHub
. Now the behavior matches user intent. What’s working well: the CLI covers almost all planned features, and each subcommand does a focused job. For instance, transactions will generate a change plan file splitting devices into canary and batches for staged deployment – a very useful feature for large networks to minimize risk
GitHub
GitHub
. The implementation now validates inputs (no zero batch sizes) to avoid edge-case bugs
GitHub
. The output of transactions is a YAML plan that can be reviewed and then fed into an execution pipeline (possibly via nauto_cli run or a CI/CD system). This design aligns with GitOps principles and shows a high level of UX maturity. Terminal UI (TUI): The TUI is a lightweight curses-style interface implemented with the Ratatui library. It’s launched by nauto_cli tui --inventory inventory.yaml, which loads the devices and then hands off to an interactive UI loop
GitHub
GitHub
. The current TUI is rudimentary but functional: it displays a scrollable list of devices on the left and details of the selected device on the right (ID, address, tags, driver type)
GitHub
GitHub
. Navigation is with the Up/Down arrow keys, and q quits – all indicated in the docs
GitHub
. The TUI cleans up after itself (restoring the terminal mode on exit) and runs in an alternate screen so it doesn’t mess up your shell
GitHub
GitHub
. For an MVP, this is a nice addition – it allows browsing the inventory in a nicer format than YAML and is a stepping stone to interactive operations (in the future, one could imagine selecting a device and triggering a job or viewing its last compliance result, etc.). The user experience of the TUI could be improved by showing dynamic information (right now it’s static info from the inventory). The docs note a next step is to add job progress streaming to the TUI via tracing events
GitHub
. This would allow the TUI to become a live dashboard during a job run – an exciting feature for large deployments so you can watch successes/failures in real time. Achieving this might involve spawning the engine on a separate thread and communicating via channels or using the tracing subscriber to push events to the UI. It’s doable and would significantly enhance the “at-a-glance” value of the TUI. GUI (Tauri-based): The GUI is currently a prototype (spikes/tauri_poc) and not part of the main build, but it represents the future “control center” for Netrust. It uses Tauri (Rust backend, web frontend) to create a cross-platform desktop app
GitHub
. The recent changes expanded it to have multiple panels: an inventory table, a job wizard, a scheduling calendar, compliance snapshot view, etc., as noted in the docs
GitHub
. This shows that the team is aiming for feature parity with the CLI in the GUI – making the tool accessible to users who prefer point-and-click interfaces. The GUI currently operates with dummy data (since the backend API is not fully developed yet). The next steps would be to connect the GUI to the real engine functions. The docs suggest using either direct calls to the CLI’s library functions or eventually a REST/gRPC API once a service mode exists
GitHub
. In the interim, Tauri can invoke Rust commands directly, so one could wire up, for example, an “Execute Job” button to call a Rust function that uses JobEngine internally. From a UX perspective, having all three interfaces (CLI, TUI, GUI) is ambitious but very powerful. The CLI is great for automation scripts and advanced users, the TUI for quick checks and text-mode environments (SSH into a jumpbox and run the TUI), and the GUI for broad adoption and ease of use (especially for demonstrating to stakeholders or training new team members). Maintaining consistency across them will be important. The team seems to be aware of this – for example, ensuring that all actions available in the GUI are backed by the same core logic as the CLI means less duplication. So far, because the GUI is a prototype, there’s no risk of divergence yet. User feedback and polish: A couple of things to improve: the CLI could benefit from more verbose help or examples embedded in --help (though the documentation provides examples). Also, the audit logging currently writes JSON lines to a file
GitHub
GitHub
 – this is great for machine parsing, but a future enhancement might be a human-friendly summary output at the end of a job (e.g., “5 devices succeeded, 2 failed, 1 skipped” which they actually do print to stdout
GitHub
). Indeed, the code prints a brief summary after a run
GitHub
, which is good. Perhaps capturing or printing the failure device names would further help (in case of failures, users typically want to know which devices failed immediately). These are minor UX tweaks. In conclusion, the multi-interface UX is coming together well. The recent commits show a lot of progress in CLI feature completeness and initial GUI capabilities. The CLI and TUI are ready for user testing, and the GUI is on a clear path to integration. The key will be to keep these interfaces consistent (e.g., a job started in the GUI should show up in the CLI’s audit logs and maybe be controllable via CLI, etc.). The groundwork in place suggests this is achievable.
Plugin Architecture and Extensibility
Plugin support is a standout extensibility feature of Netrust, even though it’s currently at prototype level. By using WebAssembly (WASM) as the plugin format, the project benefits from a safe sandboxed execution model, which is crucial for third-party code running in a network automation context (where trust but verify is the motto). The plugin SDK crate defines the interface for plugins. Notably, it provides a CapabilityMask (likely using bitflags) and a PluginMetadata struct, along with a procedural macro export_plugin! that plugin authors use to expose their plugin’s info to the host
GitHub
. This design is similar to how e.g. Firefox extensions or CNI plugins work – define a small ABI and use a macro to reduce boilerplate for plugin writers. On the host side, the code in spikes/wasm_host shows the intention: load a .wasm file via Wasmtime, find the exported metadata, and presumably instantiate the plugin. The architecture document on plugins outlines next steps, such as defining host callbacks for registering drivers and logging events
GitHub
. This implies that eventually a plugin will be able to call back into the host to, say, register a new driver implementation or a new type of job action. For example, a plugin might add support for a new vendor device by implementing the DeviceDriver trait in a WASM-compatible way and then telling the host about it. Since the DriverRegistry today is a simple in-memory list, we’d need to extend it (or have a parallel structure) to accept drivers from plugins at runtime. A possible approach is having a PluginManager that keeps track of loaded plugins and their provided drivers, and on each job execution, the engine checks both the built-in DriverRegistry and any plugin-provided drivers for a matching DeviceType. Implementation-wise, we could have the plugin’s export_plugin! macro declare the device types it supports (the CapabilityMask might already encode some of this), and the host use that to route tasks. The marketplace feature complements the plugin system by providing a way to distribute and manage plugin binaries. The CLI’s marketplace install command will copy a WASM module into a local directory
GitHub
, and list shows what plugins are available with a brief description
GitHub
. This is very user-friendly – akin to a package manager for network automation extensions. The verify function currently computes a SHA-256 hash and prints it along with any expected signature from the index
GitHub
. In the future, it would be good to actually cryptographically verify a signature (for example, if the index provides a signature signed by the plugin author or by Netrust’s “store”). The groundwork is in place for this, just needs to be implemented (likely using an asymmetric key to sign the hash). What’s working well: the concept and structure of the plugin system is forward-looking and aligns with modern infrastructure trends (extending core platforms via safe plugins, like WASM in Envoy/Istio, etc.). By keeping the plugin API minimal (just metadata and a capability mask for now), the team has avoided needing a complex cross-boundary interface at this early stage. They can gradually expand it – e.g., define a fn init(HostContext) -> PluginHandle that each plugin must implement, where HostContext could allow registering a driver or subscribing to events. The use of WASM means a plugin could be written in Rust, Go (with TinyGo to WASM), or even Python (with WASI Python runtimes) – giving flexibility to network engineers who might not be Rustaceans. Needs rework/filling in: runtime integration as mentioned – currently, plugins are not loaded by the main binary at all. A near-term improvement would be to allow the nauto_cli to load all .wasm files in the marketplace/plugins/ directory on startup (or on demand). Perhaps a nauto_cli plugins load command could explicitly load one. Upon loading, the host would use Wasmtime to ensure it’s a valid plugin (matching an expected WASI version or interface), then call its exported functions. Because this is tricky to get perfectly right (especially with async drivers), it might be okay to initially restrict plugins to simpler tasks (e.g., compliance checks or inventory data sources) and not full device drivers. However, the design clearly envisions drivers as plugins (since the example guest plugin likely mimics a vendor driver). Security: Using WASM is a great choice because it isolates memory and can be run with configured permissions. The host can control what syscalls or host functions are available to the WASM (preventing it from doing something crazy like spawning processes or writing files unless explicitly allowed). It will be important to implement signature verification (so you only run trusted plugins) and possibly sandboxing at the OS level too (Wasmtime can be instructed to disable all host I/O, for instance, unless via approved functions). Given that network automation plugins might be downloaded from a marketplace, these precautions are absolutely necessary. The team has noted this in docs (marketplace entries include optional signatures, and the security doc likely covers plugin trust – the question references a security focus). Extensibility beyond plugins: The core is already extensible via configuration – new devices can be added to inventories easily, compliance rules are just data, etc. The plugin architecture extends extensibility to code. Another vector is integrations: the CLI has an integrations netbox-import command
GitHub
, meaning you can pull in data from NetBox (an IPAM/CMDB) to populate your inventory. This is a form of extensibility as well – connecting with external systems. The integrations module likely can be extended to other sources (they mentioned ServiceNow change management in release notes
GitHub
). That part is not WASM-based – it’s native code to parse NetBox JSON. But conceivably, one could use the plugin system to add new integrations too. It might be overkill; writing those directly in Rust is fine. Just worth noting that the architecture leaves room for growth in multiple dimensions. To summarize, the plugin model as recently introduced is promising but incomplete. It’s exactly the right approach for a long-lived automation platform to have a plugin ecosystem. The next implementation steps are clear (finish host loader, enforce signatures, expand the SDK for real driver functions), and the groundwork in the code is good. There are no fundamental design flaws seen in how they approached it – just the challenge of finishing it and making it seamless to use. Once done, Netrust could support a marketplace of device drivers or even automation “apps,” which would be a big differentiator in the Rust network tooling space.
Testing Coverage, Continuous Integration, and Documentation
The project shows a commendable commitment to testing and CI, especially given its early-stage status. After the recent changes, they conducted a regression test pass on 2025-11-17 (as per docs/testing.md) and confirmed all automated tests and checks are green
GitHub
. Let’s break it down: Unit and Integration Tests: Several core crates have unit tests. For example, nauto_engine tests that the driver registry and JobEngine work as expected by simulating a small set of devices and drivers and ensuring the outcomes are correct (success vs skipped, etc.)
GitHub
GitHub
. The drivers crate has tests to ensure each driver’s reported capabilities match expectations (e.g., Juniper’s driver supports commit, Generic SSH does not)
GitHub
GitHub
. The compliance module has a test that feeds in some sample device configs and rules and checks that the pass/fail logic and summary counts are correct
GitHub
GitHub
. These tests cover the logical correctness of important components (concurrency control, data model serialization, rule evaluation). One area to improve is broader integration testing: currently, tests appear to focus on individual crates. There isn’t, for instance, an end-to-end test that runs a full job through the CLI or engine with multiple devices and asserts on the final JobResult. Writing such a test would be valuable – it could use the dummy drivers (so as not to require actual network connectivity) and verify that, say, if one driver returns an error, the overall JobResult has the failure logged, etc. Another integration test could simulate the worker queue: write a temporary jobs.jsonl with a known job and inventory, run the worker command in dry-run and capture the output to ensure it’s picking up the right entries. These kinds of tests give confidence that the pieces work together. Given that the design is modular, setting up these tests is feasible. Manual Testing and Examples: The docs/testing.md file shows a table of manual CLI smoke tests that were performed
GitHub
. This includes running the bench command with 100 devices, using telemetry --format json, generating a transactions plan, running the worker in dry-run, and using the NetBox integration. All of these succeeded without errors, and example output files were generated (like plans/test_plan.yaml). This manual testing is great to see – it’s essentially acting as an integration test. The next step might be to automate some of it, possibly by turning those example commands into a script or using assert_cmd in Rust tests to invoke the CLI with sample files. But even as manual steps, it indicates the core scenarios have been tried end-to-end. Continuous Integration (CI): There is a GitHub Actions workflow that runs on each push/pr, which performs: format check (cargo fmt), lint (cargo clippy with -D warnings to fail on any warning), tests (cargo test), and a dependency audit (cargo audit)
GitHub
. Additionally, it attempts to compile the Tauri GUI on Linux (installing system deps like libwebkit) to ensure the GUI changes don’t break the build
GitHub
GitHub
. This level of CI coverage is excellent for quality assurance. It ensures that even as the project rapidly adds features (as it did in the recent refactor), the basics (style, lint, basic tests, security) remain intact. The use of -D warnings is particularly good for keeping code quality high – it means no ignored clippy hints or deprecation warnings slip through. One thing to note: there were references to a CA certificate bundle issue in the repo review doc
GitHub
GitHub
– it appears .cargo/config.toml was pointing to a user-specific path for a certificate store, and the repo might be missing a certs/cacert.pem. This could cause builds (especially of reqwest with rustls) to fail in some environments or just be confusing. It’s recommended to include a consistent way to handle TLS certs (either include Mozilla’s CA bundle in the repo, or rely on system certs by default). This is a minor configuration detail but affects portability. The documentation and code should agree on how TLS is set up. Since the Meraki driver uses reqwest with rustls-tls, ensuring the certificate trust is correctly configured (especially in offline or corporate environments) is important. A follow-up task is likely to add the certs/cacert.pem to the repo and adjust the config, as suggested in the repo review
GitHub
GitHub
. Documentation: The project’s documentation is comprehensive and up-to-date with the latest changes – a strong indicator of a production-minded team. There are docs covering architecture, design decisions, security considerations, UX, quick start guide, release notes, roadmap, and even a risk register. The README provides a quick synopsis and example CLI usage which is very helpful for new users
GitHub
. The service architecture doc outlines how the system will evolve into a distributed service
GitHub
, and the UX doc explains how to run the CLI/TUI/GUI
GitHub
GitHub
. Maintaining such docs as code changes is often neglected, but here it appears the docs were updated alongside the code (for instance, the release notes mention all the new features delivered in v0.1.0
GitHub
, which matches what we see in the code). This is excellent for onboarding and for user trust. One area to watch: the security documentation. It likely covers how credentials are stored (the use of OS keychain via KeyringStore), and other concerns like not storing plain-text passwords. The code seems to follow through on those promises (credentials are indeed stored in the OS secure store, and never printed)
GitHub
GitHub
. As new features like plugins and remote APIs are added, updating the security guidelines (e.g., how to verify plugins, how to manage API tokens) will be important. Test coverage metrics weren’t mentioned, but given the number of modules, more tests can always be added. Particularly, as real network operations get implemented, writing tests for them might require stubs or simulation (for example, a fake SSH server to test the SSH driver – there are crates to assist with that, or one could abstract the connection in the code to inject a dummy). For now, the simulated drivers make testing easier. In conclusion, the project is in very good shape regarding tests, CI, and docs for this stage. The recent changes were validated by both unit tests and manual scenarios, and any issues discovered were documented (and in many cases, already fixed in code). Continuing this discipline will ensure that as the project grows (more code for real device interactions, etc.), regressions are caught early. Specific suggestions here: add a few end-to-end tests for the CLI (could use the example files in examples/ as inputs and check outputs), and consider using GitHub Actions matrix builds (to test on Windows, for instance, since this is cross-platform) especially once the CLI/GUI is released to others. But overall, the groundwork for a reliable, well-documented tool is clearly laid.
What’s Working Well vs. Needs Rework (Summary)
To recap, many things in Netrust are working excellently after the recent overhaul:
Modular, idiomatic Rust codebase – easy to navigate and extend, with proper error handling and no obvious memory or concurrency bugs.
Async concurrency model – efficient use of Tokio, proven with benchmarks, ready to scale to large device counts
GitHub
.
Feature breadth – support for various device types (via drivers), multiple UX options (CLI/TUI/GUI), and advanced use cases (compliance checks, staged deploys, GitOps integration, etc.) all within one tool.
Security-minded features – secure credential storage and cautious handling of secrets
GitHub
, sandboxed plugin approach, and dependency auditing in CI
GitHub
.
Documentation and Testing – comprehensive docs and a passing test suite give confidence in the code’s reliability and maintainability.
However, some areas need further work to reach production-grade quality, especially given the demands of large-scale network automation:
Improvements and Implementation-Ready Fixes
Implement Real Device Communication – Replace driver stubs with actual network interactions:
SSH CLI Drivers: Integrate an async SSH client (e.g. russh) in CiscoIosDriver, AristaEosDriver, and GenericSshDriver. These drivers should open an SSH connection to Device.mgmt_address using credentials from CredentialStore, send the commanded snippets or show commands, and capture output. Start with basic command execution and error handling (e.g. timeout if no response in N seconds). This will allow real config pushes and command collects on IOS/EOS devices instead of simulated sleeps
GitHub
GitHub
.
NETCONF for Juniper: Use a NETCONF client to send the config snippet in JuniperJunosDriver.apply_config. After loading the snippet, perform a commit check (dry-run) if dry_run=true or as a pre-check, then a commit confirmed (with a timeout) or full commit. Capture any commit errors and return them as Err(...) so the engine marks the device failed
GitHub
GitHub
. Ensure that supports_dry_run remains true and corresponds to commit-check behavior.
NX-OS API and Arista eAPI: In CiscoNxosApiDriver.execute(), use the existing reqwest client to POST the JSON payload to the NX-API endpoint on the device
GitHub
GitHub
. Check the HTTP response; if non-200, return an error with the status/body for debugging
GitHub
GitHub
. Parse minimal JSON if needed to detect command errors. Similarly, consider adding an HTTP-based mode for Arista (sending commands to /command-api using JSON-RPC). Both drivers should use credentials (for NX-OS, likely basic auth or bearer token – possibly an enhancement to Device.credential to support HTTP auth). This will turn those drivers from placeholders into fully functional API clients.
Meraki Driver Enhancements: The Meraki driver is mostly complete, but add handling for rollback. Since supports_rollback=true
GitHub
, implement rollback() to perhaps reapply a stored previous configuration. This might mean capturing a “before” state (Meraki allows retrieving the prior config or using network tags to revert). If not feasible, consider setting supports_rollback=false until a strategy is in place, to avoid false promises
GitHub
.
Telemetry Collectors: Implement real data collection in SnmpCollector, GnmiCollector, and HttpCollector. For SNMP, integrate an SNMP library or call out to snmpget for a few OIDs (e.g., interface counts as simulated). For gNMI, consider using a gRPC client to a demo target or make this a no-op if not critical. The HttpCollector could perform an HTTP GET to a known endpoint (the current placeholder endpoint “https://api.meraki.com” is not actually returning telemetry, so perhaps repurpose this to hit a test endpoint or remove it)
GitHub
GitHub
. Essentially, ensure collect_all() runs collectors in parallel (use join_all to concurrently gather telemetry) and returns realistic metrics. This will make the nauto_cli telemetry command output meaningful data for monitoring.
Timeouts and Retries: For all real network operations, implement sensible timeouts (Tokio’s timeout() or using reqwest timeouts as already set for Meraki
GitHub
) and perhaps simple retry logic for idempotent actions (e.g., if an HTTP call fails due to a transient network issue, retry once). This will improve resiliency in large-scale use.
Extend Testing for Real Operations:
After implementing the above, add integration tests or system tests for critical paths. For example, spin up a local SSH server (or use a network simulator) in a test to verify the SSH driver can login and run a command. This could be as simple as running ssh localhost with a known user on CI (perhaps use docker with an IOS container for a true integration, if available in CI). If that’s not feasible, at least test the parsing logic of drivers (e.g., feed a sample NX-OS JSON output into a function to ensure the code extracts errors or success correctly).
Include a test for the worker functionality: create a temp jobs.jsonl with a known small job, run nauto_cli worker --queue <file> --limit 1 and capture stdout to ensure it prints the expected dispatch message
GitHub
GitHub
. This will verify that the queue parsing works. As the worker will eventually execute jobs, plan to test that path too (possibly by having the worker use a dummy driver to avoid real device calls in test).
Consider using property-based testing for compliance rules (e.g., random config strings to ensure not: rules always invert contains: logic properly). The current tests cover basic cases
GitHub
GitHub
 but more thorough testing can catch edge cases (like overlapping rules, case sensitivity, etc.).
Ensure that any new code (SSH handling, etc.) is covered by at least unit tests for error scenarios (e.g., wrong credentials should return an error, command timeouts propagate correctly).
Improve the Distributed Workflows (Scheduler/Worker):
Complete the implementation of the worker_daemon. Right now it polls the queue file and logs entries
GitHub
GitHub
. It should actually invoke the JobEngine on each item. One approach: have the worker spawn tasks similar to the CLI run does, using the inventory and job path from the QueueItem to load data and then call engine.execute(job). This might require refactoring to expose a library API for running a job outside of main.rs (perhaps move some logic from run_job() into nauto_engine or a new helper that both CLI and worker can call). Implementing this will allow headless, continuous processing of jobs – essential for a production automation system.
Add a mechanism to acknowledge or remove processed queue items. Since using a file, the simplest way is to rewrite the file excluding processed lines or to write results to a separate file. Alternatively, move to a lightweight message queue (even SQLite or Redis) for tracking jobs. In this iteration, maybe simply log that item X is done or mark it in the file (e.g., by prefixing with a timestamp or moving it to a “done” file). This prevents reprocessing the same jobs on the next loop.
Expand the schedule command into a scheduler service. Possibly, implement cron parsing (if not already) and have the worker daemon load schedule definitions to enqueue jobs at the right time. This can be complex, so as an interim, ensuring that the schedule preview (which likely prints upcoming occurrences) is correct is good. The heavy lifting of running on schedule could be a separate process or integrated into the daemon loop.
Integrate the approvals workflow: The CLI has approvals and notify commands, but these likely are stubs. Define what an approval means in this context (maybe a simple file or token that must be present to proceed). Implementation-ready fix: have the approvals command read an “approval queue” (could be a JSON similar to jobs) and allow an admin to approve a pending change. The run command or worker should check if a job requires approval (perhaps mark jobs with a flag in the job YAML or via an Approval struct) and delay execution until an approval is recorded. This ties into policy – might be more design needed. For now, even printing a message “approval required – not implemented” when such a scenario arises is better than silently ignoring it.
Polish CLI & UX:
Error messages and Logging: Ensure all user-facing errors are clear. For example, if an SSH connection fails, catch that and return an error like “Device X (IOS) SSH connection failed: <reason>”. Right now, errors bubble up as anyhow errors, which might include technical details but not context. Using .with_context() as done elsewhere will help attach the device name or action to the error
GitHub
. Also consider using tracing::error to log in drivers when something goes wrong (similar to how engine logs a task failure)
GitHub
.
CLI output improvements: For long-running jobs, a verbose mode could periodically print progress (e.g., “10/100 devices completed”). This could be done by counting results in the loop that collects tasks
GitHub
. Or integrate with the tracing events and use a separate thread to print progress. This is optional, but for large scale, users appreciate feedback during the run.
Audit Logging: The audit log currently records basic job info (ID, name, counts)
GitHub
. Consider extending it to log per-device results (success/fail and maybe a reference to logs or diff). This could be a separate detailed log file (JSON Lines per device). Implementation: loop over JobResult.device_results and write each as JSON to an “audit_devices.log” with job_id, device_id, status, and maybe first error line. This way, post-run, there’s a machine-readable record of each device outcome. At large scale, that’s easier than parsing console output or enabling debug logs.
TUI enhancements: As noted, incorporate live updates. Concretely, use a background thread or spawn within tui::launch to periodically refresh device statuses. For now, even adding a key to refresh (reread inventory file or update statuses from last run) would help. Since the plan is to feed tracing events to the TUI, a near-term step is to buffer those events (maybe in memory or a temp file) and display them when the user presses a key. This requires designing how tracing events map to UI – likely a log view or coloring devices in the list as green/red for success/fail after a job.
GUI integration: Begin connecting the Tauri GUI to the core library. For example, implement a Tauri command (in Rust) to list inventory devices by reading the same YAML as CLI does, and another command to run a job (perhaps reuse run_job but in non-CLI form). This way the GUI buttons can trigger real actions. Also, consider packaging: set up the Tauri config for proper app metadata (name, icon). While this is not a “code fix”, it’s needed for a production-ready GUI deliverable. CI could be extended to build the GUI for different OS eventually.
Secure and Streamline Configuration:
Certificate Bundle: Include the certs/cacert.pem file (a Mozilla CA bundle) in the repository, and update .cargo/config.toml or reqwest settings to use it
GitHub
. Ensure that both Linux and Windows can find the CA certificates (or use native system certs by default, which rustls can do through the webpki-roots crate). This prevents issues when making HTTPS calls (Meraki, NX-OS API) on systems that might not have certificates or where an offline build is done.
Credential Store Abstraction: The nauto_security::KeyringStore presumably uses the OS keychain. That’s great, but document how to use it in headless environments (for example, on a Linux server with no GUI keyring, does it fall back to plaintext file? If so, ensure it’s encrypted or at least protected). As an implementation fix, you might add an option to use an environment variable or a .env file for credentials in CI scenarios, with the understanding of the risk. This was partly addressed by allowing --password-stdin, etc. Make sure to test the keyring on all platforms (Windows, Mac, Linux) as part of release testing, since keychain access can be tricky (e.g., requires user login on Mac).
Signature Verification for Plugins: When running marketplace verify, if the JSON index has a signature field for a plugin, actually verify it using a public key. This means embedding a public key in the tool (or having a keyring of trusted publishers). Using something like ring or ed25519-dalek crate, you can verify that the downloaded WASM’s hash matches the signed hash. This ensures plugins haven’t been tampered with. Implement this before allowing marketplace install to load plugins automatically. It can simply warn or refuse to load an un-signed plugin by default.
Plugin Loading: Implement a basic plugin loading flow in the main CLI: on startup, scan marketplace/plugins/*.wasm, use Wasmtime to load each, retrieve its metadata (name, capabilities) and log it. For now, you might not hook them into the engine fully, but at least confirm they load without error. This paves the way for integrating their functionality and gives feedback if a plugin is incompatible. If a plugin provides a driver for a DeviceType that conflicts with a built-in one, decide the override strategy (maybe prefer built-in unless plugin explicitly configured to replace). These policy decisions can be documented and a flag --enable-plugins can control it. Implementing the loading with graceful failure (one bad plugin shouldn’t crash the whole app) is important – use catch_unwind around plugin init if necessary.
Each of these improvements will move Netrust closer to a production-ready, scalable network automation platform. By focusing on real device communication, robust testing, workflow completion, and security hardening, the next iteration of Netrust will be ready to handle large multi-vendor networks reliably and safely. Many of the building blocks are already in place – it’s now about connecting them and tightening the bolts. The design and recent refactor have set a solid foundation; with the above refinements, Netrust can confidently be piloted in a real network environment. 

Citations
GitHub
repo_review.md
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/docs/repo_review.md#L5-L9
GitHub
lib.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/crates/nauto_model/src/lib.rs#L69-L75
GitHub
lib.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/crates/nauto_model/src/lib.rs#L84-L92
GitHub
meraki_cloud.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/crates/nauto_drivers/src/drivers/meraki_cloud.rs#L140-L149
GitHub
worker.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/apps/nauto_cli/src/worker.rs#L23-L31
GitHub
lib.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/crates/nauto_engine/src/lib.rs#L14-L22
GitHub
lib.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/crates/nauto_drivers/src/lib.rs#L36-L45
GitHub
generic_ssh.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/crates/nauto_drivers/src/drivers/generic_ssh.rs#L11-L19
GitHub
marketplace.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/apps/nauto_cli/src/marketplace.rs#L54-L61
GitHub
ci.yml
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/.github/workflows/ci.yml#L14-L22
GitHub
lib.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/crates/nauto_drivers/src/lib.rs#L44-L52
GitHub
main.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/apps/nauto_cli/src/main.rs#L229-L237
GitHub
main.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/apps/nauto_cli/src/main.rs#L238-L246
GitHub
main.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/apps/nauto_cli/src/main.rs#L216-L225
GitHub
lib.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/crates/nauto_engine/src/lib.rs#L40-L48
GitHub
lib.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/crates/nauto_engine/src/lib.rs#L42-L50
GitHub
lib.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/crates/nauto_engine/src/lib.rs#L43-L51
GitHub
lib.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/crates/nauto_engine/src/lib.rs#L54-L62
GitHub
lib.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/crates/nauto_engine/src/lib.rs#L44-L52
GitHub
lib.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/crates/nauto_engine/src/lib.rs#L61-L69
GitHub
lib.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/crates/nauto_engine/src/lib.rs#L62-L70
GitHub
lib.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/crates/nauto_engine/src/lib.rs#L86-L95
GitHub
lib.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/crates/nauto_engine/src/lib.rs#L128-L132
GitHub
plan.md
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/plan.md#L16-L25
GitHub
testing.md
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/docs/testing.md#L11-L19
GitHub
meraki_cloud.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/crates/nauto_drivers/src/drivers/meraki_cloud.rs#L20-L28
GitHub
testing.md
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/docs/testing.md#L14-L19
GitHub
tui.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/apps/nauto_cli/src/tui.rs#L18-L26
GitHub
plan.md
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/plan.md#L15-L23
GitHub
lib.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/crates/nauto_engine/src/lib.rs#L40-L49
GitHub
lib.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/crates/nauto_drivers/src/lib.rs#L40-L48
GitHub
generic_ssh.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/crates/nauto_drivers/src/drivers/generic_ssh.rs#L20-L28
GitHub
mod.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/crates/nauto_drivers/src/drivers/mod.rs#L1-L9
GitHub
cisco_ios.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/crates/nauto_drivers/src/drivers/cisco_ios.rs#L36-L45
GitHub
cisco_ios.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/crates/nauto_drivers/src/drivers/cisco_ios.rs#L46-L54
GitHub
juniper_junos.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/crates/nauto_drivers/src/drivers/juniper_junos.rs#L76-L84
GitHub
juniper_junos.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/crates/nauto_drivers/src/drivers/juniper_junos.rs#L95-L103
GitHub
meraki_cloud.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/crates/nauto_drivers/src/drivers/meraki_cloud.rs#L134-L143
GitHub
meraki_cloud.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/crates/nauto_drivers/src/drivers/meraki_cloud.rs#L151-L159
GitHub
lib.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/crates/nauto_engine/src/lib.rs#L134-L142
GitHub
lib.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/crates/nauto_engine/src/lib.rs#L120-L128
GitHub
plugins.md
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/docs/plugins.md#L7-L15
GitHub
plugins.md
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/docs/plugins.md#L11-L19
GitHub
marketplace.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/apps/nauto_cli/src/marketplace.rs#L20-L28
GitHub
marketplace.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/apps/nauto_cli/src/marketplace.rs#L46-L55
GitHub
marketplace.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/apps/nauto_cli/src/marketplace.rs#L42-L50
GitHub
marketplace.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/apps/nauto_cli/src/marketplace.rs#L68-L76
GitHub
plugins.md
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/docs/plugins.md#L16-L24
GitHub
plan.md
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/plan.md#L18-L23
GitHub
repo_review.md
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/docs/repo_review.md#L7-L9
GitHub
worker_daemon.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/apps/nauto_cli/src/bin/worker_daemon.rs#L22-L31
GitHub
worker_daemon.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/apps/nauto_cli/src/bin/worker_daemon.rs#L34-L41
GitHub
worker.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/apps/nauto_cli/src/worker.rs#L39-L47
GitHub
meraki_cloud.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/crates/nauto_drivers/src/drivers/meraki_cloud.rs#L198-L207
GitHub
meraki_cloud.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/crates/nauto_drivers/src/drivers/meraki_cloud.rs#L204-L213
GitHub
meraki_cloud.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/crates/nauto_drivers/src/drivers/meraki_cloud.rs#L43-L49
GitHub
meraki_cloud.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/crates/nauto_drivers/src/drivers/meraki_cloud.rs#L109-L117
GitHub
generic_ssh.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/crates/nauto_drivers/src/drivers/generic_ssh.rs#L34-L43
GitHub
arista_eos.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/crates/nauto_drivers/src/drivers/arista_eos.rs#L38-L46
GitHub
bench.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/apps/nauto_cli/src/bench.rs#L19-L27
GitHub
bench.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/apps/nauto_cli/src/bench.rs#L37-L45
GitHub
juniper_junos.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/crates/nauto_drivers/src/drivers/juniper_junos.rs#L94-L102
GitHub
juniper_junos.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/crates/nauto_drivers/src/drivers/juniper_junos.rs#L21-L29
GitHub
juniper_junos.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/crates/nauto_drivers/src/drivers/juniper_junos.rs#L80-L89
GitHub
cisco_nxos_api.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/crates/nauto_drivers/src/drivers/cisco_nxos_api.rs#L48-L57
GitHub
cisco_nxos_api.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/crates/nauto_drivers/src/drivers/cisco_nxos_api.rs#L65-L74
GitHub
cisco_nxos_api.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/crates/nauto_drivers/src/drivers/cisco_nxos_api.rs#L108-L116
GitHub
lib.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/crates/nauto_telemetry/src/lib.rs#L23-L32
GitHub
lib.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/crates/nauto_telemetry/src/lib.rs#L40-L49
GitHub
telemetry.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/apps/nauto_cli/src/telemetry.rs#L26-L30
GitHub
telemetry.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/apps/nauto_cli/src/telemetry.rs#L32-L40
GitHub
README.md
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/README.md#L16-L25
GitHub
main.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/apps/nauto_cli/src/main.rs#L40-L48
GitHub
main.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/apps/nauto_cli/src/main.rs#L1-L9
GitHub
main.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/apps/nauto_cli/src/main.rs#L42-L51
GitHub
worker.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/apps/nauto_cli/src/worker.rs#L7-L15
GitHub
observability.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/apps/nauto_cli/src/observability.rs#L42-L50
GitHub
repo_review.md
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/docs/repo_review.md#L20-L24
GitHub
observability.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/apps/nauto_cli/src/observability.rs#L40-L48
GitHub
transactions.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/apps/nauto_cli/src/transactions.rs#L39-L48
GitHub
transactions.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/apps/nauto_cli/src/transactions.rs#L50-L58
GitHub
transactions.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/apps/nauto_cli/src/transactions.rs#L78-L86
GitHub
main.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/apps/nauto_cli/src/main.rs#L260-L264
GitHub
tui.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/apps/nauto_cli/src/tui.rs#L103-L111
GitHub
tui.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/apps/nauto_cli/src/tui.rs#L119-L128
GitHub
ux.md
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/docs/ux.md#L11-L19
GitHub
tui.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/apps/nauto_cli/src/tui.rs#L22-L31
GitHub
tui.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/apps/nauto_cli/src/tui.rs#L46-L54
GitHub
ux.md
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/docs/ux.md#L22-L27
GitHub
README.md
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/README.md#L35-L44
GitHub
ux.md
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/docs/ux.md#L16-L20
GitHub
audit.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/apps/nauto_cli/src/audit.rs#L23-L31
GitHub
audit.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/apps/nauto_cli/src/audit.rs#L33-L39
GitHub
main.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/apps/nauto_cli/src/main.rs#L190-L198
GitHub
main.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/apps/nauto_cli/src/main.rs#L192-L198
GitHub
marketplace.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/apps/nauto_cli/src/marketplace.rs#L56-L64
GitHub
marketplace.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/apps/nauto_cli/src/marketplace.rs#L46-L54
GitHub
testing.md
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/docs/testing.md#L16-L19
GitHub
release_notes.md
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/docs/release_notes.md#L8-L10
GitHub
testing.md
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/docs/testing.md#L5-L13
GitHub
lib.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/crates/nauto_engine/src/lib.rs#L163-L171
GitHub
lib.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/crates/nauto_engine/src/lib.rs#L72-L76
GitHub
lib.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/crates/nauto_drivers/src/lib.rs#L61-L70
GitHub
lib.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/crates/nauto_drivers/src/lib.rs#L78-L86
GitHub
lib.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/crates/nauto_compliance/src/lib.rs#L121-L130
GitHub
lib.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/crates/nauto_compliance/src/lib.rs#L140-L148
GitHub
ci.yml
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/.github/workflows/ci.yml#L26-L34
GitHub
ci.yml
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/.github/workflows/ci.yml#L35-L39
GitHub
repo_review.md
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/docs/repo_review.md#L18-L26
GitHub
repo_review.md
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/docs/repo_review.md#L39-L41
GitHub
repo_review.md
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/docs/repo_review.md#L20-L28
GitHub
repo_review.md
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/docs/repo_review.md#L40-L41
GitHub
README.md
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/README.md#L16-L24
GitHub
ux.md
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/docs/ux.md#L3-L11
GitHub
release_notes.md
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/docs/release_notes.md#L12-L18
GitHub
meraki_cloud.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/crates/nauto_drivers/src/drivers/meraki_cloud.rs#L26-L29
GitHub
main.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/apps/nauto_cli/src/main.rs#L202-L209
GitHub
ci.yml
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/.github/workflows/ci.yml#L18-L25
GitHub
cisco_ios.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/crates/nauto_drivers/src/drivers/cisco_ios.rs#L36-L44
GitHub
lib.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/crates/nauto_telemetry/src/lib.rs#L60-L68
GitHub
lib.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/crates/nauto_telemetry/src/lib.rs#L70-L78
GitHub
worker_daemon.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/apps/nauto_cli/src/bin/worker_daemon.rs#L28-L37
GitHub
lib.rs
https://github.com/lammesen/netrust/blob/82c5ae72fbd725082a61ea5b859fd47cebabac7e/crates/nauto_engine/src/lib.rs#L104-L113