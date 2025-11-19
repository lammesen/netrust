# Netrust

## Project Overview
Netrust is a cross-platform network automation toolkit built in Rust. It is designed to provide a secure, efficient, and extensible platform for managing network infrastructure.

**Key Features:**
*   **Async Job Engine:** High-concurrency execution engine for running jobs across thousands of devices.
*   **Multi-Vendor Drivers:** Support for Cisco IOS/NX-OS, Juniper Junos, Arista EOS, Meraki Cloud, and generic SSH.
*   **Interfaces:**
    *   **CLI/TUI:** Powerful command-line interface with terminal UI capabilities.
    *   **Web UI:** Modern React-based dashboard for visualization and job management.
*   **Compliance & Telemetry:** Built-in modules for configuration compliance auditing and telemetry collection.
*   **Extensibility:** Plugin system (future WASM support) and marketplace for extensions.

**Architecture:**
The project is organized as a Rust workspace with a modular architecture:
*   `apps/nauto_cli`: The main CLI application entry point.
*   `apps/web-ui`: The React-based frontend application.
*   `crates/`: Core libraries containing business logic:
    *   `nauto_model`: Domain models (Devices, Jobs, Credentials).
    *   `nauto_drivers`: Vendor-specific driver implementations.
    *   `nauto_engine`: Orchestration logic for executing jobs.
    *   `nauto_security`: Credential management and security features.

## Building and Running

### Rust Backend & CLI

**Prerequisites:** Stable Rust toolchain.

*   **Format Code:**
    ```bash
    cargo fmt --all
    ```

*   **Lint Code:**
    ```bash
    cargo clippy --all-targets --all-features
    ```

*   **Run Tests:**
    ```bash
    cargo test
    ```

*   **Run CLI:**
    You can run the CLI directly via Cargo:
    ```bash
    cargo run -p nauto_cli -- <command> <args>
    ```
    *Example:*
    ```bash
    cargo run -p nauto_cli -- run --job examples/jobs/show_version.yaml --inventory examples/inventory.yaml
    ```

### Web Frontend

**Prerequisites:** Node.js.

Navigate to the web UI directory:
```bash
cd apps/web-ui
```

*   **Install Dependencies:**
    ```bash
    npm install
    ```

*   **Start Development Server:**
    ```bash
    npm run dev
    ```

*   **Build for Production:**
    ```bash
    npm run build
    ```

*   **Lint:**
    ```bash
    npm run lint
    ```

## Development Conventions

*   **Language & Style:**
    *   **Rust:** Follow standard Rust idioms. Use `cargo fmt` for formatting and `cargo clippy` for linting.
    *   **TypeScript/React:** Uses Vite for building. Follow ESLint configuration in `apps/web-ui`.
*   **Testing:**
    *   Unit and integration tests are standard in Rust crates (`cargo test`).
    *   CI pipeline enforces formatting, linting, and testing.
*   **Architecture Patterns:**
    *   **Async/Await:** Heavy usage of Tokio for asynchronous operations, especially in drivers and the job engine.
    *   **Traits:** Core functionality is defined via traits (e.g., `DeviceDriver`, `CredentialStore`) to ensure modularity and testability.
    *   **Observability:** Uses the `tracing` crate for instrumentation.
