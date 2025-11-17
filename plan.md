Cross-Platform Network Automation Tool in Rust – Design & Implementation Plan

1. Problem Framing and Use Cases
	•	Scope of Automation: The tool targets enterprise network engineers who manage large multi-vendor environments. It should streamline tasks like pushing configuration changes in bulk, collecting device data, and ensuring compliance across thousands of devices. By using Rust for a single compiled binary (no heavy runtime or interpreter needed), deployment is simple and resource usage stays lean ￼.
	•	Typical Use Cases:
	•	Config Rollouts: Apply a standard configuration snippet (e.g. update SNMP or ACL settings) to hundreds of switches and routers at once, with the ability to dry-run changes, get diffs, then commit globally.
	•	Bulk Interface Changes: For example, disable a set of interfaces across all branch switches or update interface descriptions on all core routers in one go.
	•	Inventory Collection & Telemetry: Periodically run “show” commands or API calls on all devices to gather interface statuses, routing tables, log snippets, etc., then normalize this data for monitoring or auditing.
	•	Compliance Checks: Compare running configs or settings against a golden baseline (for security policies, VLAN assignments, etc.) and flag deviations. The app might generate a report or even auto-remediate config drift.
	•	Assumptions: We assume network connectivity to all target devices (via SSH, APIs, etc.) and that credentials are provided securely. The tool is not a real-time NMS or SNMP monitoring system (though telemetry features are a future roadmap item), but rather a job-based automation tool. It’s not meant to replace controllers (e.g. Cisco DNA Center) but to give engineers a lightweight alternative for direct device automation. We also assume users have an inventory of devices (IP addresses/hostnames, device types) that can be loaded into the tool.
	•	Non-Goals: The initial version won’t cover every possible network vendor or protocol. Deep streaming telemetry (gNMI, SNMP polling with live dashboards) is out-of-scope for the MVP (to be handled in later versions). Likewise, complex orchestration involving multi-step workflows (like transactional changes spanning multiple devices with inter-dependencies) will be kept simple in the beginning – jobs largely execute per device in parallel without cross-device logic in MVP.

2. High-Level Architecture

System Overview: The application is structured into modular layers, separating concerns of device communication, job orchestration, data storage, and user interface. All components are implemented in Rust for efficiency and safety. The entire app compiles into a single, cross-platform binary for Windows, macOS, and Linux (facilitated by Rust’s zero-runtime output ￼). Below is a description of the main architecture components:
	•	Device Drivers Layer: At the bottom, a device abstraction layer defines a common interface to communicate with devices. Each vendor or device OS (Cisco IOS, NX-OS, Juniper Junos, Aruba AOS-CX, Meraki cloud, etc.) has a Driver implementation that knows how to connect and run commands on that platform. This layer handles protocol details – whether it’s executing CLI commands over SSH, sending NETCONF RPCs, or calling REST APIs. By abstracting this, the upper layers can issue high-level actions (like “get config” or “apply config snippet”) without worrying about the exact command syntax for each vendor.
	•	Job Orchestration Engine: The core engine receives job definitions (what to run, on which devices) and orchestrates their execution. It handles concurrency (running tasks in parallel across many devices), sequencing (performs pre-checks, then config changes, then post-checks), and error handling. The engine treats a batch job somewhat like a transaction: for example, if a config push job is requested, it can perform a dry-run (or a NETCONF ), then apply changes, then do post-change verification. If any step fails, it can trigger a rollback or stop further changes. The engine’s design uses Rust’s async runtime so that thousands of device tasks can be managed concurrently with minimal threads ￼. It will include batching and rate-limiting logic to avoid overwhelming networks or devices (for instance, running at most N concurrent SSH sessions at a time, or pausing between batches if needed).
	•	State and Inventory Management: A persistence layer is responsible for storing device inventory, credentials (securely), job definitions, and historical results. This could be a lightweight embedded database or structured files. The inventory contains device records (hostname/IP, device type/vendor, credentials or credential profile, tags/location info). The state includes past job runs, logs, and any cached data (like last known config for diffing). For MVP, a simple approach might use local JSON/YAML files or SQLite for structured data, to keep external dependencies minimal. This layer provides query capabilities (e.g. “select all devices in site=Oslo of type=switch”) for targeting jobs.
	•	API Layer: The core exposes a local API (could be an HTTP REST API or a gRPC interface) that allows external access to its functionality. This API layer makes the tool scriptable and integrable: e.g. a user could invoke jobs via CLI commands that internally call the same API, or a GUI frontend could use the API to control the engine. By having a well-defined API, even third-party tools or scripts could drive the automation engine (for instance, an external script could trigger a job via HTTP). In implementation, this might use an embedded web server (only listening locally by default for security) or Rust’s tonic gRPC for high performance calls.
	•	User Interfaces (CLI/TUI/GUI): On top of the API, we provide two interfaces:
	•	A CLI/TUI for power users and quick automation usage. This might be a single binary that can operate in two modes – a command-line mode for one-shot commands and a full-screen TUI (text-based UI) for interactive usage. The CLI will support commands like run-job, add-device, etc., with flags for headless use in scripts or pipelines. The TUI mode provides an interactive text dashboard in the terminal, with menus to browse devices, create or select jobs, and monitor progress in real time.
	•	An Optional GUI for users who prefer a graphical interface. The GUI would be a desktop application that communicates with the same backend engine (possibly by embedding the engine via frameworks like Tauri). This GUI would present forms and tables for inventory and job results, a dashboard for recent runs, and possibly visualization of telemetry data in the future. The GUI is not required to use the tool (the CLI/TUI and API are sufficient for advanced users), but it lowers the entry barrier for daily use by network engineers less comfortable with terminals.

Concurrency & Async: Rust’s asynchronous ecosystem (in particular, Tokio) will be heavily utilized across the architecture. The device drivers will use async I/O for network communications so that dozens or hundreds of SSH or HTTP sessions can be open in parallel without blocking threads. The job engine will spawn tasks for each device or each sub-task and use async/await to efficiently manage thousands of simultaneous operations. This design means even scaling to tens of thousands of devices is feasible with appropriate batching – Tokio can handle massive numbers of concurrent tasks on a fixed thread pool, achieving minimal overhead per connection ￼. For example, 10,000 SSH sessions might be handled by a pool of, say, 16 OS threads that rapidly switch between tasks, rather than 10,000 OS threads.

Observability and Safety: Throughout the architecture, we will include robust logging, metrics, and error propagation. Each layer emits structured logs (using Rust’s tracing framework) so that events (device outputs, errors, job start/stop) can be recorded with context (device name, job ID, etc.) ￼. The engine will catch and handle errors at each step, marking individual device tasks as failed rather than crashing the whole application. Metrics (counters, timings) are collected for performance monitoring – e.g. how many devices succeeded vs failed, how long each took, etc. This architecture will treat changes somewhat transactionally: by doing pre-checks and post-checks, and by having a plan for rollback (e.g. storing backup configs or using vendor-specific rollback commands), the system can ensure that failures result in minimal persistent harm. Security is woven through all layers (detailed in section 8), including encrypted storage of sensitive data and strict controls on external API access.

3. Technology Choices and Rationale

We choose Rust for every component of this product, leveraging its performance, safety, and cross-platform capabilities. This means no Python/Go/Node dependencies at runtime – just a single Rust binary. As one network engineer’s experience noted, Rust avoids the need for managing Python environments or container images for distribution, making deployment much easier ￼. Below are specific technology decisions with alternatives considered:
	•	Async Runtime: We will use Tokio as the asynchronous runtime. Alternatives: async-std or smol were considered, but Tokio is the de facto standard in Rust with a large ecosystem and library support. Tokio offers a proven, high-performance scheduler that excels at handling many connections with low overhead. For example, Rust+Tokio have been shown to handle thousands or even millions of connections concurrently with low memory usage and high throughput ￼. Tokio’s integration with popular libraries (e.g. reqwest for HTTP, database drivers, etc.) and utilities like tokio::sync for synchronization makes it an obvious choice. Recommendation: Tokio – for its maturity, ecosystem, and performance.
	•	Network Protocol Libraries:
SSH (CLI access): For device CLI automation, SSH is critical. We have two main approaches:
	•	Use the ssh2 crate (Rust bindings to libssh2, a C library) possibly combined with an async adapter (async-ssh2-tokio). This is stable and widely used but involves FFI to C and is not fully async without a wrapper.
	•	Use a pure-Rust implementation like russh/thrussh or a high-level wrapper like async_ssh (built on thrussh) ￼ ￼. Pure Rust avoids external dependencies and can integrate with Tokio natively. However, we must evaluate feature completeness (key exchange algorithms, authentication methods, etc.).
Recommendation: Start with async-ssh2-tokio or similar – this provides an asynchronous API over the battle-tested libssh2 ￼. This gives us immediate support for common SSH functions (exec commands, shell channel, SFTP if needed). In parallel, we will prototype with russh to see if a pure Rust solution meets our needs; if it does, we can migrate to reduce external C deps. The chosen library must support modern ciphers (many enterprises disable old algorithms), and allow us to manage the SSH session (disable paging on CLI, handle prompts, etc.).
NETCONF/RESTCONF (API access): For devices supporting NETCONF (e.g. Juniper, IOS-XE, Nokia), we have the netconf-rs crate as a starting point. It supports NETCONF over SSH and provides vendor utilities ￼, but it’s somewhat dated and limited (only explicitly mentions H3C in docs). Alternatives are limited in Rust – we might integrate directly at the XML protocol level using an XML library (e.g. quick-xml) on top of an SSH session. Another approach is RESTCONF/REST APIs (which use HTTP/HTTPS+JSON) – these can be handled with standard HTTP clients.
Recommendation: Use NETCONF where available (via netconf-rs or custom implementation) for structured config operations, especially on Juniper (to leverage candidate config, commit/rollback features). For Cisco IOS-XE, which supports RESTCONF, use the HTTP approach described below. We acknowledge the risk that Rust NETCONF libraries are young (one community note observed the existing crates are not heavily used or updated ￼), so we allocate time to test and potentially improve these libraries.
HTTP/REST (e.g. REST APIs, Meraki Cloud): Many modern network devices and controllers expose RESTful APIs (Cisco Meraki cloud, Aruba Cloud Central, some SD-WAN controllers, etc.). We will use reqwest for HTTP(S) calls. Alternatives: hyper (lower-level but more control), surf (async-std based). We prefer reqwest because it is built on Tokio, supports async out-of-the-box, and includes conveniences like automatic JSON serialization/deserialization via Serde. It’s well-maintained and ergonomic ￼. For Meraki specifically, we’ll implement a Meraki driver that uses reqwest to call the Meraki Dashboard API (with API keys), handling pagination and rate-limiting according to their specs. Recommendation: reqwest for any HTTP/REST interactions, for its ease of use and robust feature set (TLS, proxies, etc.).
SNMP (future telemetry): While not needed for initial config management, future versions will incorporate SNMP for monitoring/telemetry. We note the existence of the snmp/snmp2 crates which provide async support and SNMPv3 capabilities ￼. In a later phase, we can integrate these to poll device statistics at scale. For now, SNMP is part of the roadmap rather than MVP.
	•	Data Formats & Templating: We will use Serde for configuration parsing (supporting JSON, YAML as needed). For example, inventory files or job definitions can be in YAML (human-friendly for engineers). We’ll use serde_yaml/serde_json to load these. For output data (like JSON results or reports), Serde will help serialize Rust structs to JSON for the API or CLI to output. For templating configurations or commands, we plan to include a template engine – e.g. tera or handlebars – so that user-defined job templates can include placeholders (like variables for interface names, IP addresses, etc.). Recommendation: Serde for config and data I/O, and Tera (TinyTemplate) for lightweight text templating in job definitions, since these are pure Rust and easy to embed.
	•	Logging & Metrics: We adopt Rust’s modern logging and tracing tools. Choice: tracing crate for structured logging across async calls. Unlike traditional unstructured logs, tracing lets us attach context (span tags like device IP, job ID) to logs ￼, which is invaluable in a concurrent environment (so logs from 1000 devices don’t intermix incoherently). We’ll use tracing_subscriber to output logs in JSON or plain text as needed, and possibly integrate with an UI log viewer. For metrics, we have options like metrics crate (with exporters for Prometheus) or using OpenTelemetry. Initially, a simple approach is to expose counters (jobs run, failures, average durations) via logging or a /metrics endpoint. Recommendation: tracing for logs (with feature to forward to OpenTelemetry if needed), and a lightweight metrics library (like metrics + metrics-exporter-prometheus for future integration with Grafana).
	•	Secrets and Credential Management: Security is paramount. We will not store plaintext passwords in config files. Instead, we’ll leverage OS-specific secure storage when available. Choice: use the keyring crate, which provides a cross-platform API to the system credential store (Windows Credential Vault, macOS Keychain, or Linux Secret Service/Keyring) ￼. This way, when the user adds a device with a username/password, we can store the secret in the OS vault and only keep a reference (e.g. key name) in our config. The keyring crate is async-aware (especially on Linux, it can use DBus asynchronously) and avoids us implementing crypto ourselves. Alternative: We considered storing secrets encrypted in a local file (using a master password or OS-specific encryption APIs), but using the OS keychain is preferable for user convenience and security. Recommendation: keyring crate with the appropriate feature flags for each OS’s native secure store ￼. We will also support loading SSH keys (paths to key files) and integrate with SSH agent if available, so that passwords aren’t needed for SSH in those cases.
	•	Frontend – TUI (Terminal UI): For the text-based interface, we choose a Rust TUI library that gives a rich UI in terminals. Options:
	•	ratatui (formerly tui-rs) with crossterm for handling input. This is a well-known combo for building terminal dashboards, and supports windows, scrolling, mouse, etc.
	•	Alternative: Termimad or cursive crates (higher-level abstractions for TUI forms). ratatui (a fork of tui-rs that is actively maintained) provides low-level control to draw our own widgets and is quite performant. It integrates with the async model by allowing the UI to run in its own thread or as part of the async tasks (via input event handling).
We prefer ratatui for flexibility – we can create custom views (lists of devices, progress bars per task, etc.). For input handling, crossterm is proven cross-platform. Recommendation: ratatui + crossterm for building the TUI. This allows a network engineer to run the tool in a terminal and get a fullscreen text UI with menus and real-time updates (useful over SSH sessions or on servers with no GUI).
	•	Frontend – GUI: For the graphical interface, we need a solution that is also cross-platform and can interface with our Rust backend. We consider two main approaches:
	1.	Web technology GUI with Tauri: Tauri is a framework that lets us create a desktop app using a Rust backend and a WebView for the UI. We can build the UI in HTML/CSS/JS (or even with Rust-generated WASM via frameworks like Dioxus/Leptos). Tauri’s advantage is an extremely small and efficient runtime: apps can be as small as ~2-5 MB and use far less RAM than Electron, since Tauri uses the OS’s native WebView instead of bundling Chromium ￼. Also, Tauri’s backend is Rust, which aligns with our “Rust-only” mandate, and it gives us a secure bridge to call Rust commands from the UI.
	2.	Pure Rust GUI libraries: e.g. Iced, egui (eframe), or Slint. These avoid web technologies and create native GUIs directly in Rust. Iced is a popular Elm-inspired GUI library that can target multiple platforms; egui is immediate-mode and quite easy to integrate, but the look-and-feel might be less native. Slint provides a UI DSL with a Rust back-end, good for fluid design but a bit newer.
Recommendation: Tauri with a Rust-based frontend. We lean toward Tauri because it allows using modern web UI paradigms (which enable building a polished interface quickly) while keeping the core in Rust. We could implement the UI using a Rust->Wasm framework like Dioxus or Leptos, meaning our GUI code is also in Rust (compiled to WebAssembly and rendered in the Tauri WebView). This satisfies the 100% Rust constraint for core product code and takes advantage of rich web UI libraries for things like tables, forms, and charts. Tauri also focuses on security (isolating the web UI from the system by default) and performance (small bundle, low memory) ￼. An alternative could be Iced, which would give a single binary with no web dependency, but it might result in a heavier binary and potentially more limited UI components. Given the need for charts and real-time dashboards in the future, a web-based approach (with libraries like D3.js or Chart.js via Tauri) is appealing. Thus, we’ll prototype with Tauri. If Tauri’s webview is problematic in some contexts (older systems, etc.), we have the CLI/TUI as fallback.
	•	Diff and Config Comparison: For comparing configurations or outputs, we will incorporate a diff library. The Rust crate similar provides text diffing (with support for line-based unified diff) and is dependency-free ￼. Alternatives like difftastic (more structural diff) or calling external GNU diff exist, but using similar directly in Rust allows us to produce a unified diff of configs for the user inside the tool. We will use this crate when showing differences between pre- and post-check configs or when doing compliance checks against a baseline. For instance, to diff two config texts, TextDiff::from_lines can be used and iterate over changes ￼. Recommendation: similar crate for computing diffs to present to the user, because it’s straightforward and efficient.

In summary, the stack is Rust from top to bottom: Tokio for async, specialized crates for networking (ssh2/async, reqwest, netconf), Serde for data, tracing for logs, keyring for secrets, ratatui/crossterm for TUI, and Tauri (with possibly Dioxus/Leptos) for GUI. Each choice balances performance, safety, and the ability to run lean – for example, using Tauri instead of Electron gives a huge win in memory and size ￼, and using Rust async for concurrency avoids the overhead of threads per device (no Java GC pauses or Python GIL issues, as one engineer discovered by rewriting a socket server in Rust ￼). These technology decisions set a solid foundation for a high-performance, cross-platform network automation tool.

4. Data and Domain Model

We will design Rust structs and traits to represent the core domain entities: Device, Credential, Job, Task/Command, JobResult, etc. The data model emphasizes a clear separation of device-specific details and abstract job logic.
	•	Device Model: Each network device in the inventory is represented by a Device struct. This includes identity and capability info:

struct Device {
    id: String,              // unique identifier (could be hostname or an UUID)
    name: String,            // human-friendly name
    device_type: DeviceType, // enum: e.g. CiscoIOS, CiscoNXOS, JuniperJunos, ArubaAOS, MerakiCloud, GenericSSH, etc.
    mgmt_address: String,    // IP or hostname for management
    driver: Box<dyn DeviceDriver>, // Associated driver implementing vendor-specific trait (see below)
    tags: Vec<String>,       // e.g. ["site:Oslo", "role:core", "vendor:Cisco"]
    credentials: CredentialRef // reference to credentials (key to secure store or SSH key path)
}

The DeviceType enum or similar field is used for high-level categorization. The driver field is a polymorphic handle that knows how to communicate with this device. We fill this after determining the type (e.g. a CiscoIOS device gets a CiscoIOSDriver implementation). Alternatively, we might not store the driver in each device but rather have a lookup (e.g. a factory that given a DeviceType returns the appropriate DeviceDriver). Multi-vendor capabilities are handled via traits: for example, we might have a trait ConfigDiffable implemented by drivers that can fetch full config to compare, or a trait SupportsCommitCheck for devices that can validate changes before commit (Juniper, NX-OS, etc.). Capabilities could also be a bitmask or set of feature flags in the Device struct (for quick checks like device.supports["rollback"] == true), populated based on type.

	•	Credential Model: To abstract different auth methods, we define:

enum Credential {
    UserPassword { username: String, password_key: String },
    SshKey { username: String, key_path: String, passphrase: Option<String> },
    Token { token_key: String }  // e.g. API token (for Meraki, DNAC, etc.)
}

Here password_key or token_key might reference a key in the OS keychain (since actual secrets are stored securely). The Credential enum covers common cases (interactive password, SSH key, or token). Credentials can be stored separately from Device and referenced by name, so users can update a password in one place and all devices using it get updated.

	•	Job Model: A Job represents a batch operation to be executed. We can model it as:

struct Job {
    job_id: Uuid,
    name: String,
    job_type: JobType,
    targets: TargetFilter,   // could be a list of Device IDs or a query like tags filter
    parameters: JobParameters, // e.g. for a config push, this might hold a config snippet or template variables
    schedule: Option<CronExpr>, // for future (if scheduled recurring jobs)
}
enum JobType { 
    CommandBatch, ConfigPush, ConfigPull, ComplianceCheck, CustomPlugin(String) 
}

JobType might be an enum for built-in types, plus possibly a marker for custom/plugin jobs. Each job has a target selection – which devices to run on – defined either explicitly (list of device IDs) or via a query (all devices with certain tags or of certain type). The JobParameters would be specific to the job type (for a CommandBatch, it might be a list of CLI commands to run; for a ConfigPush, it could be a blob of config or a reference to a template file; for ComplianceCheck, maybe a policy definition to check against). This structure allows us to define jobs in a declarative way, possibly loadable from a YAML definition for reuse.

	•	Execution Units – Task/Command: Within a job, each device execution can be represented as a Task or Command entity. For example:

struct Task {
    device: Device,
    job_id: Uuid,
    sequence: Vec<Command>, // sequence of commands/steps to run on the device
    state: TaskState,       // Pending, Running, Success, Failed, RolledBack, etc.
    result: Option<DeviceResult>
}
struct Command {
    action: ActionType,    // e.g. "CliCommand", "PushConfig", "GetConfig", etc.
    payload: String,       // the command string or config snippet
    expect_response: bool, // whether to capture output
}

In a simple CommandBatch job, the Task.sequence might be one Command (like “show version”). In a ConfigPush job, sequence could be: [ maybe “copy run start” or commit check, then actual config commands, then “write memory” or commit]. Representing commands explicitly can help if we implement rollback (e.g. we could store inverse commands for rollback if available). The TaskState tracks progress per device.

	•	Job Result and Reporting: After execution, we gather results. A JobResult could aggregate per-device outcomes:

struct JobResult {
    job_id: Uuid,
    started_at: DateTime,
    finished_at: DateTime,
    device_results: Vec<DeviceResult>,
    overall_status: JobStatus  // e.g. Success, PartialSuccess, Failed
}
struct DeviceResult {
    device_id: String,
    status: DeviceStatus,  // Success, Failed, Skipped, etc.
    output: HashMap<String, String>, // output or data collected, e.g. command -> text output
    error: Option<String>,
    config_diff: Option<String>     // if relevant, a diff of before/after config
}

Each DeviceResult may contain the raw output of commands run, or parsed data if we implement parsers. For config changes, if we retrieved the config before and after, we can include a config_diff (in unified diff format) to show exactly what changed on that device. The overall_status might be PartialSuccess if some devices failed while others succeeded. These results can be saved (for audit logs) or presented in the UI (for example, the GUI could show a table of devices with green/red status and allow clicking to see logs/diffs).

	•	Templates and Scripts: We also anticipate having template definitions as part of the model. For example, a user might define a template for an interface configuration change (with variables for interface name, description, VLAN, etc.). This could be represented as:

struct Template {
    name: String,
    content: String,    // the template text (could use a templating syntax)
    template_type: TemplateType, // e.g. "cli-snippet", "junos-set-style", etc.
    parameters: Vec<TemplateParam> // definitions of expected variables
}

The system can load such templates and allow the user to instantiate them in a Job by supplying parameter values. This promotes reuse of common changes.

	•	Trait-Based Capabilities: To handle multi-vendor differences, we use Rust traits to define behaviors. For example:

#[async_trait::async_trait]
trait DeviceDriver {
    async fn connect(&self, device: &Device) -> Result<Session, ConnectError>;
    async fn run_command(&self, session: &mut Session, cmd: &str) -> Result<String, RunError>;
    async fn push_config(&self, session: &mut Session, config: &str) -> Result<(), PushError>;
    async fn get_config(&self, session: &mut Session) -> Result<String, RunError>;
    // ... other common actions
    fn capabilities(&self) -> CapabilitySet;
}

Each vendor’s driver (e.g. CiscoIOSDriver, JuniperJunosDriver) implements this trait. The capabilities() method might return a set of features (like "candidate_config", "commit_confirm", "diff_supported", etc.) which higher-level logic can use. For example, JunosDriver’s capabilities might include "commit_confirm", enabling the job engine to use commit-confirm rollback logic on Juniper devices, whereas CiscoIOSDriver might not. This trait-based design allows us to add new drivers without changing the core engine – just implement the trait for the new device type and register it.
Additionally, we can extend traits for more specialized functions. Perhaps trait NetconfDriver extends DeviceDriver with a method fn rpc(&self, session, xml: &str) -> Result<String> for devices that support NETCONF. Or a trait SupportsRollback with a fn rollback_last(&self) for those with native rollback (Junos, NX-OS checkpoint, etc.). Using traits and default method implementations, we can provide sane fallbacks (e.g. a generic method to simulate diff by doing get_config and comparing, for devices that don’t have native diff capability). This approach mirrors the philosophy of the Rust rustmiko crate, which provides an abstraction for network devices inspired by Netmiko ￼ – type-safe wrappers so that users of the API call high-level methods (like device.interface_up("Gig0/1")) instead of writing expect scripts each time.

In summary, the data model organizes everything as Rust types, enabling compile-time checks for many errors. New device types can be slotted in by implementing traits. The job definitions are flexible enough to cover simple one-off commands up to complex config deployments. The use of secure references for credentials and standardized result structures will make the tool robust and maintainable.

5. Job Engine Design

The Job Engine is the heart of the application, executing the work in a safe, controlled, and high-performance manner. It takes a Job (defined as above) and carries it through all stages: target resolution, execution pipeline, error handling, and idempotency/rollback logic.
	•	Job Definition Input: Jobs can be defined through YAML/JSON files (for reusable playbook-like jobs) or via CLI/GUI input (ad-hoc jobs). For example, a YAML job definition might specify a name, type, device filter, and commands to run. The engine will have a parser to create a Job struct from such definitions. We plan to support Jinja2-like or Tera templates within these definitions for dynamic content. This gives users the flexibility of defining jobs outside of the binary (like Ansible playbooks, but in a Rust-executed context).
	•	Target Resolution: When a job is triggered, the first step is resolving which devices to operate on. The targets in the Job can be:
	•	Explicit list: e.g. device IDs or hostnames.
	•	Filter query: e.g. site = "Oslo" AND role = "access".
	•	All devices of type: e.g. all Juniper devices.
The engine will query the inventory (which might be in memory or a DB) to get the matching devices. It then instantiates a Task for each device.
	•	Execution Pipeline: Each job goes through phases, which the engine coordinates:
	1.	Pre-check / Dry-run Phase: If the job type supports it, perform validations before making changes. For config pushes, this could mean syntax checking or using a device API to validate. For example, on Juniper, use commit check via NETCONF; on Arista or NX-OS, use a dry-run commit or config session diff if available; for IOS which lacks dry-run, we might just push to a test device or skip this phase. If any device fails the pre-check, the job can either abort for that device or abort entirely depending on settings (likely skip that device and continue others, marking it failed early).
	2.	Execution Phase: The engine dispatches the actual commands/config to devices. This is done in parallel with a concurrency limit. For example, we might allow up to 100 concurrent tasks by default (user configurable). If there are 1000 devices, the engine will start tasks for 100; as each finishes, it starts the next, to avoid overload. Within each Task, the sequence of Commands is run (as defined in the Job or default sequence for that job type). We use Rust async for running these: e.g. spawn an async task per device using tokio::spawn. We might use a Tokio Semaphore or tokio::sync::Semaphore to cap concurrency: acquire permit before spawning a new connection to enforce the limit. Each Task collects output and errors as it goes.
	3.	Post-check / Validation Phase: After applying changes, the engine can perform post-checks. For instance, if the job was “enable interfaces”, it can run a verification command like show interface status to confirm those interfaces are now up. Or if it was a config push, it might re-fetch the running config to compute a diff against the pre-change config. These post-check steps can be built into job definitions or automatically applied for certain job types. The results are gathered into the DeviceResult (including any diffs or verification outputs).
	4.	Rollback (if needed): If a device’s execution fails midway and we consider the changes partial, the engine will attempt a rollback on that device. Rollback strategy depends on capabilities: e.g. on Juniper, if commit fails, device automatically doesn’t apply changes (transactional), so no rollback needed; if a failure occurs after some changes were applied on a Cisco IOS device, the engine could either attempt to reapply the saved pre-change config (if we archived it) or mark that device as needing manual attention. We will implement a basic rollback for cases where it’s safe: one approach is to automatically save each device’s config before changes (e.g. copy run start to a file or use an API to fetch config), and if something goes wrong, push back the saved config. This is heavy for large scale, so we’ll make it optional or limited. Another strategy is “configure replace” on IOS devices using their built-in feature (if available) to load a known-good config file. The engine’s rollback design will be cautious by default – maybe just stop on first serious error and require user confirmation to proceed or rollback. In the future, we can enhance this to be more automated.
	•	Progress Tracking: The engine will update job and task status in real-time. Each Task (device) transitions states, and we emit events (for UI and logs) such as “Device X config push started”, “Device X success”, “Device Y failed pre-check: reason…”. A real-time progress view (in TUI/GUI) can subscribe to these updates (possibly by the engine exposing an event stream or simply through shared memory updated under a mutex that the UI thread reads).
	•	Concurrency & Retry Policies: Concurrency is tuned to avoid bottlenecks. For example, establishing 1000 SSH sessions at once might spike CPU or exhaust sockets. We will default to a safe number (like 50 or 100 concurrent) but allow configuration. The engine should also detect rate-limits: e.g. Meraki API might allow only X calls per second; the Meraki driver can internally queue or delay requests to respect that. For reliability, we implement retry logic for transient failures. We can use an exponential backoff for certain errors (like if an SSH connection fails due to a network glitch, try again after a short wait, perhaps up to 3 attempts). There’s a Rust crate retry or we can implement our own simple retry with tokio timers. For idempotence, we ensure that retrying a command won’t duplicate effects (most show commands are safe to retry; config commands might not be, so we carefully design where to retry).
	•	Idempotency Strategy: We aim for jobs to be re-run safely. That means if you run the same job twice, the second run should ideally recognize no changes needed or at least not cause harm. For configuration deployment, this is achieved by using diff/patch logic: e.g. compare current config vs desired config, and only apply what’s different. Our engine can incorporate this by, for example, computing diffs and skipping devices that already have the desired state. For command batches (like “show” commands), idempotency isn’t an issue (running again just gets new data). For things like “toggle interfaces down then up”, we’ll rely on users not writing non-idempotent sequences unless intended. In the future, we might incorporate a configuration state concept, where a job defines the end state and the engine figures out the commands to reach that state (like desired state config management). Initially, we implement a simpler approach: provide dry-run diffs and allow the user to confirm, ensuring they know what will change each time.
	•	Handling Partial Failures: In a large batch, it’s likely some devices fail (offline, credentials wrong, etc.). The engine must handle this gracefully:
	•	It will not abort the entire job for a few device failures (unless configured to be strict). It will mark those devices as failed and continue with others.
	•	If a systematic failure occurs (e.g. credential wrong for all devices, or an invalid config command causing every device to error), a high failure rate might trigger the job to stop early. We can set a threshold (say >30% failures aborts the job) – this could be configurable.
	•	At job end, we produce a report highlighting which devices succeeded and which failed (and why). For failures, we do minimal rollback if possible and log the device state.
	•	The user can then address issues (fix credentials or correct a config) and re-run the job targeting just the failed devices (we might even automate that: “retry failed devices” option).
	•	Result Reporting and Storage: Once execution is complete, the engine compiles the JobResult. This is stored (in memory and optionally persisted to a file or DB for history). The result includes logs and outputs. We’ll implement formatting for results:
	•	CLI: possibly output a summary table, and have options to save full logs or diffs to files.
	•	GUI: show a dashboard with success/fail counts, and allow drilling down per device.
	•	We’ll also consider notifications (maybe a simple email or Slack webhook) as an extension, but not core MVP.
	•	Example Flow: To illustrate, suppose a user wants to push an NTP server configuration to all Cisco IOS devices:
	•	They define a job (perhaps via a template) of type ConfigPush with a CLI snippet:

ip name-server 1.2.3.4
ntp server 5.6.7.8 prefer

targets: device_type == CiscoIOS. They choose “dry-run” option.

	•	The engine finds 500 Cisco IOS devices. It creates tasks and uses, say, 50 concurrent SSH sessions at a time. For each device:
	•	Pre-check: it might run show run | include ntp server to see current NTP config.
	•	Execution: send the config lines over the SSH CLI (enter config mode, apply lines). Because IOS has no commit, changes take effect immediately.
	•	Post-check: it could again run show run | include ntp server to verify the new server is present.
	•	It captures the diff (which might just be the lines added, if we stored pre-check output).
	•	If a device had the config already (pre-check shows the exact NTP line exists), the engine might skip applying to that device and mark it “unchanged” (idempotent behavior).
	•	If any device fails (say one was unreachable), it logs the error, and that task is marked failed but others continue.
	•	At completion, a summary is printed: e.g. “Out of 500 devices: 480 succeeded, 15 already compliant (no change), 5 failed (unreachable).” The user can inspect logs for the 5 and decide to retry.

The job engine is designed to be robust and autonomous enough to run large batches unattended, while also giving the user control and insight (dry runs, diffs, safe rollback) to trust it. We will thoroughly test it with simulated devices or lab devices to ensure that, for example, 10k devices with small changes can be processed in reasonable time and memory.

6. Vendor Abstraction Layer

Supporting multiple vendors and device types is a core requirement. We achieve this with a trait-based abstraction in Rust that encapsulates device-specific logic in Driver implementations. This layer allows adding new vendors or updating behavior without touching the core engine.
	•	DeviceDriver Trait: As introduced earlier, we define a DeviceDriver trait that declares functions for key operations (connect, run command, get config, etc.). Each vendor-specific driver implements these. This uses Rust’s dynamic dispatch or generics to call the appropriate implementation at runtime. For example:
	•	CiscoIosDriver implements DeviceDriver: connection might involve opening an SSH session and issuing terminal length 0 (to disable paging), then it’s ready. Commands are run by writing to the SSH channel and reading output until a prompt is seen.
	•	JuniperDriver implements DeviceDriver: connection might choose NETCONF (if configured) by starting a NETCONF-over-SSH session. push_config might use Junos commit APIs instead of CLI.
	•	MerakiDriver might not use SSH at all; its connect() could be a no-op or a test of API reachability, and run_command might actually call a REST API (the concept of “command” is different for Meraki, it could map specific actions like “get clients” or “set SSID”).
We keep the trait surface minimal but extensible. For actions that don’t apply to a certain device, the implementation can return a consistent error (e.g. calling get_config on MerakiDriver could return an Unsupported error, since configs are managed in the cloud differently).
	•	CLI vs API differences: Within a single vendor, we might have multiple drivers or modes. For instance, Cisco Catalyst switches can be automated via CLI/SSH or via a controller (DNAC) API – but our scope is direct device access, so CLI. Cisco Nexus could be CLI or NX-API (HTTP). We likely implement one primary way per vendor for MVP (mostly CLI for on-prem devices, REST for Meraki). Over time, the driver layer could incorporate multiple transports – possibly via feature flags or separate driver structs (CiscoNXOSDriverCli vs CiscoNXOSDriverApi). To the job engine, they both satisfy DeviceDriver. This design means the engine doesn’t care if it’s sending CLI text or JSON payload – that’s handled inside the driver.
	•	Adding New Vendors: The plugin system (discussed next) will allow inclusion of new drivers without modifying the core. But even without plugins, our code can be organized by modules for each vendor:

mod drivers {
    pub mod cisco;
    pub mod juniper;
    pub mod aruba;
    pub mod meraki;
    // ...
}

Each defines one or more structs implementing DeviceDriver. To add support for, say, Arista EOS, we could create arista.rs, implement the trait (likely very similar to Cisco IOS driver as EOS CLI is similar), and register it with the inventory (so that DeviceType::Arista gets associated with AristaDriver). Because Rust is compiled, adding a new built-in vendor means releasing a new version of the tool – which is fine for official support. For third parties wanting to extend without rebuilding, see plugins below.

	•	Handling Different Capabilities: Not all devices support the same operations. Our abstraction will use Rust’s trait and enum flexibility to handle this cleanly:
	•	Different Methods: Some devices use NETCONF’s XML, others use text CLI. Our driver implementations can call into common libraries (like using netconf-rs in the Juniper driver, or using our SSH library in the Cisco driver). If a new protocol arises (e.g. gNMI for some devices), we could add that in a driver, or as a separate trait.
	•	Output Parsing: One challenge is that “show interfaces” on Cisco vs Juniper are completely different outputs. Normalizing them is complex. Initially, we won’t fully parse outputs to a common schema (that’s like what tools as Batfish or Nornir plugins do). We will instead let each driver optionally parse key outputs it knows (e.g. a driver might implement fn parse_interfaces(output: &str) -> Vec<InterfaceStats> if we choose to structure the data). These could be separate traits or simply functions in the driver module. For now, we mostly capture raw output (maybe with minor cleaning) and tag it with device type so that users know how to interpret it.
	•	Feature Flags per Vendor: We maintain a central list or database of what each vendor supports (like a YAML or JSON that says: Juniper: supports commit-confirm,  rollback, Netconf; Cisco IOS: supports no rollback, no native diff; NX-OS: supports checkpoint/rollback). This could even be coded as a struct:

struct CapabilitySet { supports_commit: bool, supports_rollback: bool, supports_netconf: bool, ... }

and each driver returns one. The engine will use this to adjust behavior (e.g. use get_config+diff to simulate commit-check on devices with no native check).

	•	Example Implementation Snippet:

#[async_trait::async_trait]
impl DeviceDriver for CiscoIosDriver {
    async fn connect(&self, device: &Device) -> Result<Session, Error> {
        let mut session = ssh::connect(device.mgmt_address, &device.credentials).await?;
        session.execute("terminal length 0").await?; // disable paging
        Ok(session)
    }
    async fn run_command(&self, session: &mut Session, cmd: &str) -> Result<String, Error> {
        session.execute(cmd).await // execute and capture output
    }
    async fn push_config(&self, session: &mut Session, config: &str) -> Result<(), Error> {
        session.execute("configure terminal").await?;
        for line in config.lines() {
            session.execute(line).await?; 
        }
        session.execute("end").await?;
        // (Could optionally do "write memory")
        Ok(())
    }
    async fn get_config(&self, session: &mut Session) -> Result<String, Error> {
        session.execute("show running-config").await
    }
    fn capabilities(&self) -> CapabilitySet {
        CapabilitySet { 
           supports_commit: false, supports_rollback: false, supports_netconf: false, 
           // ... maybe supports_diff: false (since IOS has no native diff) 
        }
    }
}

And a Juniper driver might override push_config to use NETCONF RPC instead of CLI, and set supports_commit=true, etc.

	•	Testing and Validation per Vendor: Each driver will be tested against real or simulated devices to ensure the commands and prompts are handled correctly. Because Rust ensures memory safety, the main bugs likely will be logic or parsing issues. We intend to maintain a library of unit tests using sample outputs from each vendor. For example, test that JuniperDriver.parse_commit_error(xml) correctly identifies an error, or that CiscoDriver can properly detect the "More”` paging prompt (if any). By isolating vendor logic in this layer, those tests are easier to write and the core engine can be tested with stub drivers.

This abstraction layer is key to multi-vendor support. It follows the principle of polymorphism through traits – similar to how the Netmiko Python library allows different device classes, but in Rust we get compile-time enforcement. If we need to integrate with an external library (like rustmiko crate), we could wrap it in our trait implementation. In fact, if suitable, we might even leverage rustmiko for some drivers rather than writing from scratch – that crate provides device-specific command implementations for Cisco and Juniper ￼. However, to keep control and meet our exact needs, we are likely to implement our own minimal drivers initially, possibly borrowing logic from existing projects (acknowledging their licenses). Over time, adding a new vendor (Arista, Fortinet, etc.) would be as simple as adding a new file, implementing the trait, and distributing either via plugin or new binary release.

7. Plugin and Scripting Model

To allow extensibility without modifying the core, we design a plugin system. Plugins enable third-party developers (or power users) to add support for new vendors, define new job types or workflows, and extend reports, all without needing to fork or recompile the main application. Key considerations are safety, ease of installation, and compatibility across platforms.
	•	Plugin Scope: We envision plugins primarily for:
	•	New Device Drivers: e.g. a plugin that adds support for a vendor we don’t include by default (perhaps F5 load balancers or a specialized industrial switch). The plugin would implement the DeviceDriver trait (or a related interface) for that device and register it so the app can use it.
	•	New Job Types / Workflows: e.g. a plugin could introduce a complex workflow like “Upgrade OS image” which might involve multiple steps and decision logic, beyond the built-in job types. Or a plugin could interface with an external system (say, pull data from NetBox and use it in compliance checks).
	•	Custom Reports or Post-processing: e.g. a plugin that takes device outputs and generates an HTML report or pushes data to a monitoring system.
	•	Dynamic Loading vs Static Extension: In Rust, true dynamic loading (like loading a .dll/.so at runtime with Rust types) is tricky due to ABI compatibility. However, we have a few strategies:
	1.	Dynamic Libraries with C ABI: We could define a C-compatible interface for plugins. For example, the app looks for .dll/.so files in a plugins/ directory, uses libloading to load them, and expects them to expose certain symbols (like a register_plugin function). The plugin, written in Rust, would use #[no_mangle] extern "C" functions to interact. It could use the C ABI to call into trait objects or a simplified function table. This requires careful version management (plugins compiled against our app’s plugin API version).
	2.	WASM Plugins: Another modern approach is to use WebAssembly for plugins. The idea is to compile plugin code to WASM and have the main app include a WASM runtime (like Wasmtime). The plugin and host communicate through a well-defined interface (e.g. using WASI or the Component Model with an IDL). WebAssembly provides sandboxing (so a malicious or buggy plugin cannot crash the host easily, and has restricted access) and compatibility (a single .wasm plugin works on all OSes). It addresses the major issues of native plugins: security, interface stability, binary compatibility ￼. The host can tightly control what functions are exposed to the plugin (for instance, provide an API to get device inventory or execute a command, but nothing else).
	3.	Scripting via Interpreter: Alternatively, embed a scripting engine (like a Lua or JavaScript engine) and allow plugins in that language. But this goes against the “Rust only” grain and introduces a runtime, so we likely avoid this.
	•	Recommendation – WASM Components: We recommend using WASM-based plugins for the long run. The emerging WebAssembly Component Model allows defining an interface in a language-neutral way (for instance, using WIT, WebAssembly Interface Types) and generating bindings for host and guest. The host (our app) would implement certain functions (like “execute_command_on_device(device_id, command) -> output”) that the plugin can call, and the plugin can implement functions we call (like “run(plugin_context)”). This addresses:
	•	Security: Plugins are sandboxed in WASM – they can’t crash the host or access arbitrary memory ￼. We can limit their functionality to a whitelist of host calls.
	•	Cross-platform ease: The same .wasm plugin works on Windows, Linux, Mac as long as the host can run it.
	•	Ease of installation: The user could download a .wasm file and drop it in a plugins folder; the app can scan and load it (no compilation needed by the end user, fulfilling the “should be easy and not require technical skills” goal).
The downsides are a slight performance overhead (WASM calls ~1.2-1.5x slower than native in many cases ￼, but that’s usually acceptable given most tasks are network-bound) and some complexity in implementation. Given that by 2025 the WASM component ecosystem is maturing, we believe this is a forward-looking choice.
	•	Alternative – Dynamic Native Plugins: If we choose to implement simpler dynamic loading first, we can define a minimal C ABI interface for drivers. For example, a plugin could expose fn get_driver() -> Box<dyn DeviceDriver> using trait objects with something like the abi_stable crate to ensure a stable ABI across Rust versions. This requires the plugin to be compiled specifically against the same version of the host’s interface (so we must distribute a plugin SDK crate). It’s doable but can lead to issues with Rust updates. Also, safety is a concern: a misbehaving native plugin can segfault or steal secrets with the host’s privileges ￼. We could mitigate by running plugins in a separate process, but that adds IPC complexity.
	•	Plugin API Design: We will design a Plugin SDK that plugin authors use (likely a Rust crate that we publish). This SDK will include trait definitions and data structures that a plugin can use to interact with the host. For example:

// In the SDK crate:
pub trait Plugin {
    fn metadata(&self) -> PluginInfo;  // e.g. name, version, what it extends
    fn init(&mut self, host: &mut dyn HostContext) -> Result<()>; 
}
pub trait HostContext {
    fn register_device_driver(&mut self, device_type: &str, driver: Box<dyn DeviceDriver>);
    fn register_job_type(&mut self, job_type: &str, executor: fn(Job) -> JobResult);
    // ... possibly functions to log or access inventory
}

The plugin, when loaded, will be given a HostContext to register things. For instance, a plugin adding “AcmeRouter” support would call register_device_driver("AcmeOS", Box::new(AcmeDriver{})). The host then knows whenever a device of type “AcmeOS” is encountered, use that driver. Similarly, a plugin could register a new job type, say “Generate Network Diagram”, with a function that knows how to perform that (maybe by querying all devices via the host context and outputting a diagram file).
For WASM, this interface might be translated to WIT (WebAssembly Interface Types). The host would implement methods for registration and the plugin would call them via imports. Because WASM is sandboxed, complex types need serialization – the Component Model helps with this by allowing high-level types directly.

	•	Ease of Installation: We will make installing plugins as simple as placing a file or using a command:
	•	The app might have a command plugin install <url> which downloads a plugin (WASM or native) and puts it in the plugins directory. It could verify a signature or checksum for safety if provided.
	•	On startup or on demand, the app loads all plugins in the directory. It will handle errors gracefully (a bad plugin shouldn’t crash the app – e.g. if a native plugin fails to load, log it and skip).
	•	In the GUI, we could present a list of available certified plugins and allow one-click install. For example, a future integration with a “plugin marketplace” or simply a JSON index on our website.
	•	Safety and Permissions: Following least privilege, plugins (especially WASM ones) will by default have no access to system resources. The host will expose only specific APIs. For example, a plugin should not open arbitrary files or make network connections unless explicitly allowed. In our context, a plugin likely doesn’t need to do its own networking except via host-provided channels (the host already has network access to devices). If we use WASM, we won’t include WASI file system or networking by default, only what’s needed. This prevents a malicious plugin from doing something like exfiltrating credentials – it wouldn’t have direct access to them unless we mistakenly provide it. Native plugins are harder to sandbox, which is another reason to prefer WASM. As noted in a discussion, loading native code means it runs with host’s privileges and can do anything ￼, so we must trust native plugins fully or not allow them from untrusted sources.
	•	Plugin Example Use-Case: Suppose a community user wants to add support for VendorX OS. They write a small Rust crate using our Plugin SDK:

struct VendorXDriver;
#[async_trait]
impl DeviceDriver for VendorXDriver { /* ... implement connect, run, etc. ... */ }

struct VendorXPlugin;
impl Plugin for VendorXPlugin {
    fn metadata(&self) -> PluginInfo {
        PluginInfo { name: "VendorX Support", version: "0.1", author: "Alice" }
    }
    fn init(&mut self, host: &mut dyn HostContext) -> Result<()> {
        host.register_device_driver("VendorX", Box::new(VendorXDriver));
        Ok(())
    }
}

// the plugin crate would expose a C ABI function or be compiled to WASM that the host calls to get an instance of VendorXPlugin

They compile this to vendorx_plugin.wasm. The network engineer using our tool downloads this file to the plugins folder. On next run, our app loads it, executes the plugin’s init, and now “VendorX” appears as a known device type. The engineer can then add VendorX devices to the inventory and run jobs on them like any other device.

	•	Comparing Approaches:
	•	Dynamic native plugins: Pro: can use full power of Rust, potentially slightly faster execution for heavy tasks. Con: ABI and safety issues, platform-specific builds needed (user would have to pick the correct DLL for Windows vs Linux, etc., or we bundle multiple).
	•	WASM plugins: Pro: one build runs everywhere, very safe sandbox, clear interface definition (no worrying about Rust’s ABI changes) ￼. Con: learning curve for developers (but our SDK can hide complexity), slight overhead on calls (but likely negligible for network-bound tasks).
	•	Feature-gated crates (static): This means we ship optional drivers in the codebase turned on by cargo features. Pro: no runtime overhead, just compile in if needed. Con: requires end-users to compile their own binary to include the feature, which is exactly what we want to avoid for “non-technical” plugin installation. This is more appropriate for our internal use to maintain optional components (e.g. maybe we make the Meraki support a feature that can be disabled to reduce binary size if someone doesn’t need it).

Given the above, we recommend a WASM-based plugin system as the long-term solution for extensibility. We might implement a simpler dynamic loading as an interim (since WASM component model tooling is still evolving), but the design will aim to converge on WASM. We will keep the plugin interface as simple and versioned as possible ￼ – a lean interface means less chance of breakage. If we update the interface, we can support a version number so the host knows which plugin versions are compatible.

Finally, documentation and clear SDK examples will be provided so that adding a plugin doesn’t require deep Rust expertise. Ideally, a network engineer with some coding skill can copy an example driver plugin, modify the command strings for their device, and compile it easily (or we provide pre-built binaries if possible). Installation would then be, as desired, not technically demanding (download and place file, or use an in-app installer).

8. Security and Compliance Considerations

Security is critical because this tool will handle device credentials and perform configuration changes on critical infrastructure. We incorporate security at multiple levels:
	•	Credential Storage: As discussed, we use OS-provided secure storage via the keyring crate. This means when a user adds a credential (password or token), it’s saved in (for example) the Windows Credential Manager or macOS Keychain, not in plaintext config ￼. The application will fetch these secrets at runtime when needed (prompting the OS if necessary – on some OSes, accessing the keychain might require user to allow it). We will also support encrypted storage for cases where OS keychain is not available (e.g. a portable mode): possibly an encrypted file where the master password is prompted on tool startup. By default though, OS keychain is preferred for a seamless experience.
	•	SSH Keys and API Tokens: The tool will encourage use of SSH key authentication for devices. It will allow either passphrase-protected keys (with passphrase stored in keychain if user opts) or use of ssh-agent. For API tokens (like Meraki API key), those are stored in keychain as well under a distinct entry. We will document how to supply these securely (for example, instruct users to run nauto set-token meraki which will prompt and save to keychain).
	•	Run Permissions: The application, running locally, will operate with the user’s privileges. We assume the user running it is authorized to make changes on the network devices (since they have the credentials). We will implement least privilege in the sense that the app itself doesn’t require admin rights on the local machine to run (except if needed to access certain OS stores – but usually reading your own keychain entries doesn’t require admin).
	•	Audit Logging: Every action the tool performs can be sensitive, so we maintain audit logs. This log will record events like: which user (local OS user) ran the tool, what job was run, which devices were targeted, and summary of changes. If the organization has syslog or SIEM, they might want these logs forwarded. We can provide an option to output an audit log to a file or syslog. The audit log will avoid sensitive data (no plain-text passwords, etc.) but will include device identifiers and change details. For example: “2025-11-17 14:50:32 UTC – user jdoe – Job ‘NTP Update’ on 50 devices – 48 success, 2 fail – changed lines: …”. This helps with compliance, giving a trace of who did what and when, akin to how Cisco’s local AAA logging or tools like Oxidized log changes.
	•	Transaction Safety: While network devices often don’t have full transaction support, our engine design uses a pseudo-transaction approach to maintain consistency. For instance, if applying a config to a router fails halfway through, the tool will detect that and not proceed to mark success. We either roll back (if possible) or mark it for human intervention. We also plan to implement commit confirmation (on devices like Juniper) – e.g. use commit with confirm timer so if our tool lost connectivity after making a change, the device auto-rolls back. Using these device-native safety nets where available drastically reduces the risk of a misconfiguration locking us out (which is a classic network automation concern). The tool will provide guidance (documentation and perhaps warnings in UI) on safe use, e.g. “Don’t disable the management interface on all devices in one go – you could lock yourself out.” If the user tries something obviously dangerous (like push a config that would shut down an interface being used for management), we might have a pre-check to catch it (maybe via a linting plugin or by recognizing certain patterns).
	•	Encryption in Transit: All network communication should be encrypted. SSH is used for CLI, which is encrypted by nature. For HTTP/API, we will use HTTPS (TLS) whenever supported (which is typically always for modern device APIs). We’ll support verifying TLS certificates or allow self-signed with an option (some on-prem controllers might use self-signed certs, we’ll allow user to provide a CA or skip verification if they knowingly accept that risk). SNMPv3 (when implemented) will be used over SNMPv2 whenever possible to avoid cleartext community strings; if SNMPv2 must be used, we limit its use to read-only actions in low-risk environments.
	•	Code Safety: By writing in Rust, we eliminate entire classes of vulnerabilities (buffer overflows, use-after-free, etc.). This is a huge advantage for a tool that might run with access to sensitive networks. Rust’s safety guarantees mean the core application is resilient against memory corruption attacks. We will still be vigilant about logic vulnerabilities (like injecting unintended commands via crafted input). For example, if a user or plugin supplies a device hostname that includes command separators, etc., we ensure such data is not directly injected into shell without sanitization (in our case, we’re not running shell commands on the local OS with user input, but we are sending commands to devices – those are intentional).
	•	Plugins Security: As elaborated, we will sandbox plugins. If we allow community plugins, we might add a warning or require signing for plugins from external sources. The user should be made aware: only install plugins from trusted sources. We can implement a digital signature check (if we have a plugin index, sign plugins with our key).
	•	Compliance Mode / Read-Only Mode: Some users might want to use this tool in a read-only capacity for auditing. We could provide a mode (perhaps a flag or a different executable) that does not allow any config changes, only retrieval commands. This mode could be used by auditors or monitoring systems safely, without fear of accidental changes. In read-only mode, any attempt to execute a config change command would be blocked. This is an additional safety measure to consider for version 1.0.
	•	Secure Defaults: Out-of-the-box, the tool will be conservative:
	•	It will not save device passwords in configs – only secure storage or prompt.
	•	It will require explicit --yes or confirmation in the UI for potentially disruptive operations (like pushing configs) unless in a scripted non-interactive mode.
	•	It will default to a concurrency level that is high but safe (to avoid DDoSing your own network). The user can increase it if needed.
	•	Logging of sensitive info will be avoided. We will ensure that debug logs do not accidentally print passwords or SNMP communities. If logging device outputs that might contain sensitive config lines (like enable secret ...), we might mask those in logs.
	•	The local API (if enabled) will by default listen on localhost only. If a user wants to enable remote API access (say to integrate with an external system calling it), we will advise them to put it behind proper authentication or only run via SSH tunnel, etc. We could implement a simple auth (like an API token required for API calls) if needed.
	•	Compliance Auditing: The tool itself can help with compliance. We can include a policy engine (likely in a future milestone) where users define rules (like “no telnet enabled on any device”, “SNMP community must be xyz on all devices”). The tool can run these checks and produce compliance reports. This ties into security as well – ensuring network configs meet security standards. In MVP, this might be rudimentary (maybe just use regex on configs to find violations), but eventually could integrate with projects like Batfish or OpenConfig checks.
	•	Date/Time and Source logging: In logs and possibly on the device (for devices that support it), we’ll log who made a change. For example, some devices let you include a comment in the config or a commit message – e.g. Junos allows a commit comment, NX-OS has a commit label. The tool can insert a note like “Changed by auto-tool (user jdoe) at 2025-11-17T14:50Z”. This way, if someone later looks at device config or logs, there’s an audit trace back to our tool.

In summary, our security design is about protecting credentials, preventing unauthorized use, and tracking changes. By using secure storage, sandboxing extension code, and leveraging Rust’s safety, we significantly mitigate risks. Compliance is enhanced by robust logging and potential policy-check features. We will document recommended security practices (like rotating credentials, not embedding creds in scripts, etc.) alongside the tool.

9. UX and Workflow Examples

The user experience is split between a command-line interface (plus TUI) for quick automation and an optional GUI for a richer, visual approach. Both interfaces aim to be intuitive for network engineers, who may not be software experts. We describe typical workflows:

CLI Usage Examples

For advanced users or scripting, the CLI provides direct commands:
	•	Inventory Management: The user can add devices via CLI or import from a file. E.g.:

$ nauto inventory add --name R1 --type CiscoIOS --address 10.0.0.1 --user admin --ask-pass

This would prompt for the password (securely, without echo) and store it in the keychain under service “nauto” user “R1”. Alternatively, import:

$ nauto inventory import --file devices.csv --format csv --columns name,ip,type

After import, user can list inventory:

$ nauto inventory list --filter 'site=Oslo'

which might output a table of devices in Oslo site with their status (maybe we store last reachable or so).

	•	Running a Job via CLI:
	•	Ad-hoc command:

$ nauto run --targets "role=core and vendor=Cisco" --cmd "show version"

This would connect to all devices matching that query and run show version, then print results. We might output each device’s hostname followed by the output or summary if many.

	•	Using a predefined job template:

$ nauto job execute --name "Enable NTP" --param server=5.5.5.5 --dry-run

This assumes “Enable NTP” was defined as a template with a variable server. The tool would fill in the param and show a preview (because --dry-run was specified) of what changes it would make. The user sees diffs or planned commands. They confirm, then it executes.

	•	Batch config push example:

$ nauto run --file ntp_update_job.yaml 

If ntp_update_job.yaml contains a job definition (with targets, config snippet, etc.), the tool executes it. CLI feedback could be a live progress: perhaps printing a line for each device as it completes (or an interactive spinner if attached to a TTY).

	•	Filtering and selecting by tags: Suppose inventory has tags like site or device role, the user can do:

$ nauto run --tag site=Oslo --type CommandBatch --commands "show ip int br; show version"

This would run two commands on all Oslo devices.

	•	Output and logging on CLI: By default, the CLI will summarize. For full detail, user can add flags like --verbose or --json (to output raw JSON results for piping to other tools). For instance:

$ nauto run --targets all --cmd "show interface status" --json > results.json

would produce JSON containing each device’s output.

	•	Help and Guidance: Running nauto --help will show top-level usage. Subcommands like nauto job --help will detail how to define jobs. We plan to have sensible aliases and short commands (maybe even allow nauto do <command> as a shortcut for ad-hoc commands). The CLI design will be done with a crate like Clap which supports clear help messages and tab-completion (for common flags, etc.).

TUI Workflow

When launched in interactive mode (perhaps nauto tui or simply running nauto without arguments might drop into TUI):
	•	The terminal UI might open with a dashboard overview: e.g. number of devices, recent jobs run, etc. Navigation would be with keyboard (arrow keys, enter, etc., with hints at bottom).
	•	Navigating Inventory: The engineer can switch to an “Inventory” screen. This might list devices (maybe grouped by site or type). They could filter by typing (the TUI could have a filter prompt). Selecting a device could show device details (interfaces, maybe a recent config snapshot if available, or last contact time).
	•	Running a Job via TUI: The user might press a key (say J) to open the “Job” menu. Options could be:
	•	“Ad-hoc Command”
	•	“Push Config Template”
	•	“Predefined Job”
	•	“Compliance Scan”
etc.
Suppose they pick “Ad-hoc Command”: a dialog asks for the command(s) and which devices. The UI would allow picking devices by a multi-select list or by entering a tag filter. After confirmation, it kicks off the job.
	•	Monitoring Job Progress: Once a job starts, the TUI shows a live progress view. Possibly a table with each target device and a status: pending/running/done. As outputs come in, the user could select a device in the list to view its log output in real-time (like tailing the commands). If the job is large, maybe aggregate progress like “50/200 devices completed”. We’d use curses-style progress bars or spinning indicators for effect.
	•	Results in TUI: After completion, the TUI might highlight failures in red. The user can scroll through device logs or diffs. There might be a hotkey to “save results” or “export diff” if they want to archive it.
	•	Configuration Diff view: For jobs that alter config, the TUI could have a side-by-side or unified diff viewer. For example, select a device -> press D to view the diff of running config before vs after. The diff (as produced by our diff engine) can be color-coded (green for additions, red for removals).
	•	Compliance Mode in TUI: The user might run a compliance check which shows a report of each device and pass/fail for each rule. The TUI can display a summary (e.g. “80% devices compliant, 5 devices failed rule X, click for details”).
	•	Editing Templates/Jobs: Ideally, even the TUI could allow the user to write or edit a job template (perhaps launching an editor like vim or in-UI text edit). However, that might be advanced; initially we might assume they prepare templates externally or via CLI.
	•	General Usability: We will ensure that the TUI is not overwhelming: use clear sections, maybe windows/panes layout. For instance, top half for device list, bottom half for detailed output or logs. Navigation keys will be indicated (e.g. “[F10] to exit, [Tab] to switch pane, [Enter] to select, [?] for help”). Given many network engineers are familiar with terminal UIs (network devices themselves often use text UIs or menu systems), this should be comfortable.

GUI Workflow

If the user opts for the GUI (maybe launched by nauto gui or a separate app icon if installed):
	•	Inventory Management: The GUI would have an Inventory view (table of devices). There could be a toolbar or menu: “Add Device” opens a form (fields: name, IP, type, credentials, maybe test connection button). We can allow importing via a CSV/Excel by drag-and-drop or file selector.
	•	Launching Jobs: The GUI could have a “Jobs” panel. Perhaps a list of saved job templates on the left and a “New Job” button. Creating a new job might present a wizard:
	1.	Choose job type (from a dropdown: Command, Config Push, etc. – and possibly plugin-provided ones).
	2.	If Command, it shows a text area to input one or multiple commands.
	3.	If Config Push, it shows either a text box to paste config or a dropdown to select a template and fields to fill parameters.
	4.	Select targets: possibly a multi-select tree (sites -> devices), or a rule builder (like choose tag = X).
	5.	Dry-run checkbox and schedule option if we allow scheduling.
	6.	A summary screen and “Run” button.
	•	Real-time Dashboard: As mentioned in future roadmap, we want a dashboard for telemetry. In the near-term GUI, we might have at least a “Device status” page where it pings or SSH-checks devices periodically and shows green/red (reachability). But full SNMP graphing might be later. The design would keep a placeholder for it.
	•	Viewing Results: When a job is running, the GUI shows progress bars and per-device status possibly in a grid or list. It might update via events (the backend can push WebSocket events or use Tauri’s event system to notify the front-end). The user can click a device to see log output. After completion, the GUI could present statistics (“90 succeeded, 10 failed”) and allow exporting the result (CSV or PDF report).
	•	Compliance Reports: In a GUI, we can present compliance results as a table of rules vs devices (with red/yellow/green cells), which is easier to visualize than in text.
	•	Polish: We’ll incorporate small UX niceties: confirmation dialogs for destructive actions (“Are you sure you want to push config to 200 devices?”), ability to stop a running job (“Abort” which will attempt to cancel tasks that haven’t started or maybe send a break to those in progress if possible), and context-sensitive help (maybe tooltips or a help sidebar explaining fields).

Overall, the UX is designed to guide the engineer through tasks while still offering flexibility. Beginners might prefer the GUI to avoid syntax and see everything clearly; as they get comfortable or need to automate further, the CLI/TUI is there for power use (scripting integration, faster operation over SSH, etc.).

10. Sample Rust Code Snippets

Below are illustrative snippets demonstrating key parts of the system. These are simplified examples to show structuring and usage of some crates:
	•	DeviceDriver Trait and Implementation (Cisco example):

use async_trait::async_trait;
// Define a trait for device drivers
#[async_trait]
pub trait DeviceDriver {
    async fn connect(&self, device: &Device) -> Result<Session, ConnectError>;
    async fn exec_command(&self, sess: &mut Session, command: &str) -> Result<String, ExecError>;
    async fn get_config(&self, sess: &mut Session) -> Result<String, ExecError>;
    async fn apply_config(&self, sess: &mut Session, config: &str) -> Result<(), ExecError>;
    fn capabilities(&self) -> CapabilitySet;
}

// Example driver for Cisco IOS devices using SSH
pub struct CiscoIosDriver;
#[async_trait]
impl DeviceDriver for CiscoIosDriver {
    async fn connect(&self, device: &Device) -> Result<Session, ConnectError> {
        // Use an SSH library to connect
        let addr = format!("{}:22", device.mgmt_address);
        let user = &device.credentials.username;
        let pass = retrieve_password(&device.credentials.password_key)?;
        let mut session = async_ssh2_tokio::connect(&addr, user, &pass).await?;  [oai_citation:33‡docs.rs](https://docs.rs/async-ssh2-tokio#:~:text=async_ssh2_tokio%20is%20an%20asynchronous%2C%20easy,connection%2C%20authentication%2C%20and%20command%20execution)
        // After connect, send command to disable paging
        session.write("terminal length 0\n").await?;
        session.read_until_prompt().await?;
        Ok(session)
    }
    async fn exec_command(&self, sess: &mut Session, command: &str) -> Result<String, ExecError> {
        sess.write(format!("{}\n", command).as_bytes()).await?;
        let output = sess.read_until_prompt().await?;
        Ok(String::from_utf8_lossy(&output).to_string())
    }
    async fn get_config(&self, sess: &mut Session) -> Result<String, ExecError> {
        self.exec_command(sess, "show running-config").await
    }
    async fn apply_config(&self, sess: &mut Session, config: &str) -> Result<(), ExecError> {
        sess.write(b"configure terminal\n").await?;
        sess.read_until_prompt().await?;
        for line in config.lines() {
            sess.write(format!("{}\n", line).as_bytes()).await?;
            sess.read_until_prompt().await?; // read after each line or batch
        }
        sess.write(b"end\n").await?;
        sess.read_until_prompt().await?;
        // optionally: sess.write(b"write memory\n").await?;
        Ok(())
    }
    fn capabilities(&self) -> CapabilitySet {
        CapabilitySet { 
            transactional: false, diff: false, rollback: false, 
            batch_size: None // no specific capability 
        }
    }
}

Explanation: This uses a hypothetical async_ssh2_tokio for brevity ￼. In reality, error handling and prompt detection need to be robust (we might have a Session that knows the device prompt pattern). The capabilities indicate Cisco IOS doesn’t support transaction or native diff.

	•	Job Execution Loop (pseudo-code with async/await):

async fn execute_job(job: Job) -> JobResult {
    let devices = inventory.query(&job.targets);
    let max_concurrency = job.max_parallel.unwrap_or(50);
    let semaphore = Arc::new(tokio::sync::Semaphore::new(max_concurrency));
    let mut handles = Vec::new();
    let result_map = Arc::new(DashMap::new()); // thread-safe map to collect results

    for device in devices {
        let permit = semaphore.clone().acquire_owned().await.unwrap();
        let job_clone = job.clone();
        let res_map = result_map.clone();
        handles.push(tokio::spawn(async move {
            let driver = device.driver(); // get appropriate driver
            let mut sess = match driver.connect(&device).await {
                Ok(s) => s,
                Err(e) => {
                    res_map.insert(device.id.clone(), DeviceResult::connect_fail(e));
                    drop(permit);
                    return;
                }
            };
            // Pre-check (if needed)
            if job_clone.job_type == JobType::ConfigPush {
                if let Some(check_cmd) = job_clone.precheck_command() {
                    let pre = driver.exec_command(&mut sess, &check_cmd).await.unwrap_or_default();
                    res_map.insert(device.id.clone(), DeviceResult::precheck(pre));
                }
            }
            // Execute main payload
            let device_res = match job_clone.job_type {
                JobType::CommandBatch{commands} => {
                    let mut outputs = Vec::new();
                    for cmd in commands {
                        match driver.exec_command(&mut sess, &cmd).await {
                            Ok(out) => outputs.push(out),
                            Err(e) => { 
                                // stop on first error per device
                                log::error!("{} failed on {}: {:?}", cmd, device.name, e);
                                break;
                            }
                        }
                    }
                    DeviceResult::commands_result(outputs)
                }
                JobType::ConfigPush{ config } => {
                    let before = driver.get_config(&mut sess).await.unwrap_or_default();
                    let apply_res = driver.apply_config(&mut sess, &config).await;
                    let after = driver.get_config(&mut sess).await.unwrap_or_default();
                    let diff = diff_configs(&before, &after);
                    DeviceResult::config_result(apply_res.is_ok(), diff, apply_res.err())
                }
                // other job types...
            };
            res_map.insert(device.id.clone(), device_res);
            drop(permit); // release semaphore slot
        }));
    }
    // Await all tasks
    for h in handles {
        let _ = h.await;
    }
    // Compile results
    let mut device_results = Vec::new();
    for result in result_map.iter() {
        device_results.push(result.value().clone());
    }
    JobResult::from_devices(job, device_results)
}

Explanation: We limit concurrency with a semaphore. We spawn a task per device to do the work (connect, run pre-check if defined, then execute commands or config). We collect results in a thread-safe map (could also use channels to send results back). In the ConfigPush branch, we fetch config before and after and use a diff_configs function – presumably using our similar crate – to get a diff string ￼. The final JobResult aggregates all device results.

	•	Simple TUI Layout (using ratatui):

use ratatui::{Terminal, backend::CrosstermBackend, widgets::{Block, Borders, List, ListItem}, layout::{Layout, Constraint, Direction}};
// Suppose we have an AppState with inventory and recent logs
struct AppState { devices: Vec<Device>, log_lines: Vec<String>, selected: usize }

fn draw_ui<B: ratatui::backend::Backend>(f: &mut ratatui::Frame<B>, app: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)].as_ref())
        .split(f.size());
    // Left pane: device list
    let items: Vec<ListItem> = app.devices.iter().map(|dev| {
        let status = if dev.last_status.success { "[OK]" } else { "[!!]" };
        ListItem::new(format!("{} {}", status, dev.name))
    }).collect();
    let devices_list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Devices"))
        .highlight_symbol(">> ");
    f.render_stateful_widget(devices_list, chunks[0], &mut /* state for selection */);
    // Right pane: log/output
    let log_block = Block::default().borders(Borders::ALL).title("Output");
    // Display either a selected device output or general log
    let text = if let Some(dev) = app.devices.get(app.selected) {
        format!("Logs for {}:\n{}", dev.name, dev.session_log) 
    } else {
        app.log_lines.join("\n")
    };
    let paragraph = ratatui::widgets::Paragraph::new(text).block(log_block);
    f.render_widget(paragraph, chunks[1]);
}

Explanation: We create a two-column layout: left is 30% width for a device list, right is 70% for output. The device list shows each device with an OK or failure status and highlights the selected one. The right pane shows either the selected device’s logs or general logs. In a real app, we’d maintain some ListState for the selection.

	•	Config Diff using similar crate:

use similar::{TextDiff, ChangeTag};

fn diff_configs(old_cfg: &str, new_cfg: &str) -> String {
    let diff = TextDiff::from_lines(old_cfg, new_cfg);
    let mut diff_text = String::new();
    for change in diff.iter_all_changes() {
        let sign = match change.tag() {
            ChangeTag::Delete => "-",
            ChangeTag::Insert => "+",
            ChangeTag::Equal  => " ",
        };
        diff_text.push_str(&format!("{}{}", sign, change));  [oai_citation:36‡docs.rs](https://docs.rs/similar/latest/similar/#:~:text=for%20change%20in%20diff.iter_all_changes%28%29%20,sign%2C%20change%29%3B)
    }
    diff_text
}

Explanation: This produces a unified diff-like output where each line is prefixed with +, -, or space. In practice, we might use TextDiff::unified_diff() for a more standard format, but this shows manual iteration ￼. The change when printed yields the line content.

These snippets demonstrate the general shape of code. In actual implementation, we’d structure it across modules (e.g., module for drivers, module for job execution, etc.) and handle errors properly. But these give an idea of how Rust async and crates like ratatui and similar are used.

11. MVP vs v1.0 Roadmap

We plan the development in stages: a minimal viable product to deliver core functionality, followed by iterative enhancements toward a full 1.0 release and beyond.

MVP (Minimum Viable Product): The MVP should be achievable by a small team in a reasonable timeframe and will include:
	•	Core Engine: Async job execution with concurrency control. Ability to run commands and push simple config changes on at least 2 major vendor types (e.g. Cisco IOS and Juniper Junos) to prove multi-vendor concept. Basic error handling and logging.
	•	Device Support: Drivers for: Cisco IOS/IOS-XE (SSH/CLI), Juniper (NETCONF or CLI commit), and one more if feasible (maybe Cisco NX-OS or Aruba AOS-Switch CLI, which is similar to IOS). Also a “Generic SSH” driver that can send user-provided commands to any SSH device (with no parsing or special handling) – this covers unlisted vendors in a basic way.
	•	CLI & TUI: A functional CLI to add/list devices and execute jobs. TUI interface with inventory view and ability to trigger a command on multiple devices, showing output. It might not be extremely polished but should work on major terminals. The CLI should support reading a job definition from a file (so users can create repeatable job files).
	•	Config Push Safeguards: Dry-run option and diff output for config changes. Maybe only supported on Juniper in MVP (since commit check is easy there) and for others, we at least show the diff between pre/post config if we fetch it.
	•	Secure Storage: Integration with keychain (maybe pick one OS to fully support in MVP, e.g. test on Windows and MacOS keychain; Linux might use a simple file encryption if needed for initial release).
	•	Logging: Basic structured logging to console/file. Possibly just info/error logs with device context.
	•	No GUI yet: MVP might skip the GUI or include a very minimal one (depending on team skill). Since TUI can cover interactive use, GUI could be deferred to post-MVP.
	•	No Plugins yet: MVP will have no plugin system exposed. It might be coded in a way that adding new drivers is easy (modular), but without dynamic loading. All supported vendors are compiled in.

The goal of MVP is to prove the concept: a Rust tool can talk to multiple network devices concurrently and perform batch actions faster and safer than manual or Python scripts. It should be tested on a reasonably large inventory (perhaps simulate 500 or 1000 devices with a mock SSH server to see performance).

Post-MVP Milestones:
	•	Milestone 1: Version 1.0 Release Features
	•	Extended Device Coverage: Add a few more vendor drivers (Aruba/ProCurve CLI, Cisco NX-OS CLI, perhaps Arista EOS CLI, Cisco Meraki API). Each new driver extends market coverage. Ensure at least 5 distinct platforms are supported robustly.
	•	GUI Application: Develop the desktop GUI with Tauri. Focus on inventory management and job execution flows. By v1.0, the GUI should allow most common tasks (maybe not every single advanced feature, but core ones). This includes building some React/Dioxus components for device table, job config form, and real-time log updates.
	•	Plugin System (beta): Introduce a preliminary plugin mechanism. Perhaps initially support dynamic loading of drivers as DLLs for immediate needs, but behind feature flags. Or release a “developer preview” of the WASM plugin interface with one example plugin. The aim is to get feedback and ensure safety before broad use.
	•	Compliance Engine: Implement a basic compliance checker. Possibly allow writing simple rules in a config file (like “config must contain X” or “command output must match regex Y”). Provide a CLI command and UI page to run these checks across inventory and report. This addresses the use case of security audits.
	•	Reporting Enhancements: Provide options to output results in various formats: JSON, CSV (for command outputs), or even an HTML/PDF summary (maybe using a template or an embedded reporting library). The GUI could have an “Export Report” button for job results.
	•	Scheduling Jobs: Add a scheduling feature for recurring tasks. Maybe integrate with Cron on the host system or have the app maintain its own scheduler thread. E.g. user can schedule a nightly compliance check at 2 AM, or a weekly backup of configs. This requires the app to run continuously (we might convert it into a service/daemon mode for this, or just advise running it on a server).
	•	Integration with Git (Config-as-Code): For v1.0, a simple integration: after a config change job, optionally commit the new configs to a git repository (if the user has one set up with device configs). Or allow pulling configs from a git repo to apply to devices. This could be a step toward full GitOps for network. For example, user supplies a Git repo URL where each device’s intended config is stored; the tool can pull that and deploy to devices, or conversely, fetch running configs and push commits to a repo for backup/version control. Even if not fully automated, providing the hooks or a command to do so is valuable.
	•	Milestone 2: Advanced Features and Hardening
	•	Real-time Dashboard & Telemetry: Build on the compliance/monitoring features to provide a live dashboard of key metrics. For example, integrate SNMP or streaming telemetry (gNMI if devices support it) to show interface usage or device up/down status in real-time. This might involve running a small telemetry server or subscribing to event streams. The GUI could then show graphs (CPU, memory usage of devices, etc.). Essentially, move a bit into the territory of network monitoring systems, but focusing on quick wins (like easily pulling environmental data or error counters from many devices).
	•	Performance Tuning: Optimize for tens of thousands of devices. This may involve using more efficient data structures, ensuring the async tasks scale (Tokio should handle it, but e.g. we might need to increase some OS limits or use persistent connections where possible). We might add connection pooling for API calls, or reusing SSH connections for successive jobs if the user runs them frequently (maybe keep them open in a pool).
	•	High-Availability / Distributed Option: If needed, consider how the tool might scale beyond one machine. Perhaps allow a mode where multiple instances coordinate (though this might be beyond scope – but could be a thought if one laptop can’t handle 50k devices at once, maybe distribute load across several processes).
	•	Refining Plugin System: Based on feedback, finalize the WASM plugin approach. Possibly provide an official Plugin SDK and a gallery of community plugins by this point. Emphasize easy installation and safety.
	•	Robustness and UX Polish: By Milestone 2, fix all rough edges: e.g. ensure graceful handling of network timeouts (no hanging tasks), better progress estimation for long jobs, make the UI responsive even during huge jobs (maybe moving heavy tasks to background threads to not block UI loop), etc. Also incorporate user feedback to improve workflows (maybe adding a wizard for common tasks, etc.).
	•	Documentation & Examples: Provide comprehensive docs, not just for users but also for plugin authors. Possibly create example scenarios like “How to do a bulk interface shutdown safely” as a guide.
	•	Milestone 3: Enterprise Integrations & Scale
	•	Integration with External Systems: Add integrations with things like ITSM (ServiceNow), logging systems (Splunk), or network inventory sources (NetBox). For example, allow syncing device inventory from NetBox or writing back device state to CMDB.
	•	User Management & Auth: If multiple people use the tool or if it runs as a service, consider multi-user support with roles (maybe an API server with basic auth, to allow a team to share it). This is more relevant if running as a centralized server.
	•	Advanced Change Management: Features like generating configuration change scripts for review (instead of applying immediately, output a script that could be applied later), or integration with a CI/CD pipeline – e.g. push config changes via a Git PR and have the tool apply them when merged.
	•	Extend Real-time Capabilities: Possibly incorporate event-driven automation (like react to syslog or traps to trigger jobs) – this enters network orchestration territory.

The above milestones ensure we deliver incremental value. By v1.0 we aim to have a solid, user-friendly tool that covers the most common needs (inventory, multi-vendor changes, basic compliance, scheduling, GUI, plugin extensibility). Further versions focus on expanding scope (monitoring, integrations) and scale.

Throughout these milestones, feedback from users (network engineers) will guide adjustments. Perhaps after MVP we do a pilot with a small network to gather usability input. The roadmap is flexible to adapt to what features users demand most (for example, if plugin interest is low but web dashboard is high, we might shift focus accordingly).

12. Risks and Open Questions

Developing a complex network automation platform in Rust comes with some uncertainties. We identify key risks and how we plan to mitigate or validate them:
	•	Rust SSH Library Maturity: The ecosystem for network-specific protocols in Rust is young. While crates like ssh2 and russh exist, there might be edge cases (certain key exchange algos, concurrency issues when handling many sessions, etc.). Risk: Hitting a bug or performance issue in the SSH library when scaling to thousands of devices. Mitigation: We will test early with a large number of parallel SSH tasks (perhaps using containerized dummy SSH servers) to see how libraries hold up. We’ll contribute fixes or use alternative strategies if needed (e.g., if pure Rust lib is slow, consider spawning the ssh command as a subprocess for some operations or using a pool of persistent connections). Additionally, keep an eye on projects like rustmiko for any known issues or contributions in this area.
	•	NETCONF/API Variability: Each vendor’s API can have quirks. The netconf-rs crate is not widely used/up-to-date (as noted, it hasn’t been updated in ~2 years and few users ￼). Risk: The NETCONF implementation might be incomplete or buggy, affecting Juniper or others. Mitigation: If netconf-rs fails, we can implement a simple NETCONF client using an XML library and our SSH connection (NETCONF is essentially sending XML over SSH subsystem). This is more work but gives full control. We can test NETCONF against a Junos VM early to ensure commit operations and rollbacks work. Alternatively, Juniper also supports a REST API (with JSON RPC called Junos REST or gNMI); we could pivot to that if NETCONF is problematic.
	•	Cross-Platform GUI Issues: Using Tauri (or any GUI framework) means dealing with OS differences. E.g., on Linux, the user must have a webkit installed for WebView; on Windows, older versions might not have WebView2 by default. Risk: GUI might not work out-of-the-box on some systems, causing user frustration. Mitigation: Clearly document prerequisites (like “for GUI on Windows, the installer will ensure WebView2 runtime is installed”). Possibly bundle the WebView runtime if license allows. As a fallback, ensure the TUI/CLI is fully capable so GUI is optional. Also test the GUI on all three platforms extensively (especially for things like file dialogs or permissions, which can differ).
	•	Performance at Scale: While Rust and Tokio are known for performance, we must validate memory usage and throughput when managing tens of thousands of devices. Potential bottlenecks: memory per task (Tokio tasks are lightweight, but storing large configs or outputs in memory for thousands of devices could add up), or hitting OS limits (like number of open TCP connections or ephemeral ports). Risk: The tool might run out of file descriptors or memory when hitting upper scale. Mitigation: Use streaming approaches – e.g., don’t hold entire config diffs in memory if not needed; write results to disk incrementally for very large jobs. For OS limits, we can guide users to tune ulimit on Linux or the registry on Windows for many sockets. Also implement backpressure: if output from devices is huge, maybe process and drop what’s not needed rather than buffering everything. We will profile the app with large dummy data to spot memory hotspots.
	•	Error Handling and Partial Changes: One nightmare scenario in network automation is leaving devices in unintended states (half-configured) due to an error. While we have plans for rollback, it’s tricky on platforms without native rollback. Risk: A bug in our engine might not rollback when it should, or a network glitch mid-config leaves device misconfigured. Mitigation: Emphasize dry-run and small batches in documentation for high-risk changes. Possibly implement a safety feature: if more than N failures occur, auto-stop the job (to limit damage). Also could implement “canary” feature: apply to 1 device, check success, then apply to rest, to avoid blasting all devices with a bad config at once.
	•	Security of Plugins: If we allow native plugins before fully moving to WASM, a bad plugin could compromise security. Risk: A third-party plugin crashes the app or worse, performs malicious actions. Mitigation: Only load plugins placed intentionally by user. Possibly have a whitelist mechanism or require a flag to enable plugin loading. Encourage using WASM plugins with sandbox. We could start with only WASM plugins support to avoid this risk altogether.
	•	Adoption/Compatibility: Network engineers are used to tools like Python/Ansible. Convincing them to use a Rust tool means it must integrate well. Risk: Users might find the learning curve for our tool high if it’s not as flexible as, say, writing a quick Python script. Mitigation: Make common tasks very straightforward (lots of examples, sensible defaults). Provide importers or converters (maybe ingest an Ansible inventory or even run certain Ansible playbooks). And highlight the benefits (speed, safety, single binary). Over time, as they see reliability at scale, adoption should grow. But initial feedback might say “does it support X vendor or Y feature that my existing scripts have?” If something is missing, be responsive in adding or guiding them to plugin.
	•	Maintaining Multi-Platform Code: Ensuring everything works on Windows, Mac, Linux can be challenging (especially things like terminal handling and keychain). Risk: A feature might break on one OS due to differences (e.g., Linux keyring might require DBus session). Mitigation: Use well-tested crates (like keyring which abstracts platform specifics). Set up CI tests on all OSes for core functionality (maybe spinning up ephemeral VMs/containers).
	•	Unknown Unknowns: Interacting with real network gear can throw surprises – e.g., some devices might output ANSI color codes or have weird prompt patterns, or an SNMP agent that hangs. Mitigation: Beta testing with actual network devices of different types to catch these. Provide a debug mode where the tool logs raw device I/O to help troubleshoot when a device doesn’t behave as expected.
	•	Open Questions:
	•	How to handle device dependencies? E.g., if configuring a core router and edge switches, does order matter? Our design treats each device independently in a job. Perhaps in future, we need a way to orchestrate an ordered sequence (first do core, then edge). This is not in MVP, but if needed, we could implement job dependencies or a simple ordering in the job definition.
	•	How to deal with interactive prompts? Some device commands prompt (like “This will reboot, continue? [y/N]”). Our automation should either detect and auto-respond (with a setting), or fail safely. We need to include some expect-like capability for these cases. Possibly allow job definitions to include expected prompts and responses.
	•	APIs and streaming telemetry: Should we integrate gRPC libraries for things like gNMI (streaming telemetry)? Possibly down the road. If needed, can use tonic to implement a gNMI client.
	•	Testing Environment: Setting up tests for network automation is non-trivial. We might use container labs (e.g., containerized Arista cEOS, Cisco CML images, etc.) in CI to run real integration tests. Or use simulators (like stub SSH servers). We should plan for an automated test environment for at least some common scenarios.

Prototype validation: We will create small prototypes for risky components early:
	•	A quick Rust program to spawn 1000 tokio tasks that open an SSH session to a test server (perhaps OpenSSH on localhost) to see how memory/CPU behave.
	•	A minimal Tauri app with a single button calling a Rust command to verify our idea for GUI communication.
	•	A sample WASM plugin loaded by wasmtime with a dummy function to ensure we can call into it and back.

By tackling these in prototypes, we can uncover issues while the design is still flexible. For example, if we find Tokio with a million tasks is fine but our chosen SSH crate chokes at 10k, we adjust plans (maybe use a connection pooling or chunking strategy).

In conclusion, while there are risks in using new tech for such a broad tool, our plan incorporates careful selection of libraries, testing at scale, and phased feature rollout. The benefits of Rust (performance, safety) give us confidence that we can overcome these challenges. The open questions will be addressed as we iterate, always keeping the end-user (network engineer) in focus – the tool must ultimately simplify their work without introducing new headaches, and that principle will guide every technical decision.

Sources:
	1.	Mathieu Poussin, Scaleway – on choosing Rust for single-binary deployment ￼ ￼
	2.	rustmiko crate documentation – Rust library inspired by Netmiko for device automation ￼
	3.	async-ssh2-tokio crate – asynchronous SSH client for Rust/Tokio ￼
	4.	snmp2 crate – async SNMP v1/v2/v3 client library details ￼
	5.	Tokio runtime performance – handling thousands of connections with minimal overhead ￼
	6.	Tracing crate docs – structured, async-aware logging in Rust ￼
	7.	Tauri vs Electron – Tauri yields very small app bundles and lower RAM by using OS webview ￼
	8.	NullDeref on plugin interfaces – importance of a simple plugin interface for ABI stability ￼
	9.	Sy Brand (tartanllama) on WebAssembly plugins – sandboxed plugins solve security/interface issues ￼
	10.	keyring crate docs – cross-platform secure password storage (Windows, macOS, Linux) ￼
	11.	GTER conference notes – netconf in Rust crates not widely used, highlighting potential maintenance issues ￼
	12.	similar crate usage – example of generating unified diff output from two texts ￼