# GEMINI.md - Keycloak Continuous Delivery (kcd)

## Project Overview
`kcd` is a Rust-based command-line tool designed for the declarative management of Keycloak configurations. It allows administrators to treat Keycloak settings as code by using local YAML files to represent the desired state of a Keycloak server.

### Core Technologies
- **Language**: Rust (Edition 2024)
- **Runtime**: `tokio` (Asynchronous I/O)
- **HTTP Client**: `reqwest`
- **Serialization**: `serde`, `serde_json`, `serde_yaml_ng`
- **CLI Framework**: `clap` (with derive and env features)
- **Secret Management**: Custom heuristic-based masking and environment variable substitution.

### Architecture
The project follows a modular structure where each major CLI command has its own module:
- `src/main.rs`: Application entry point and command dispatching.
- `src/client.rs`: A robust wrapper around the Keycloak Admin REST API.
- `src/models.rs`: Defines Keycloak resource representations (Realms, Clients, Users, etc.) using `serde`.
- `src/inspect.rs`: Logic for fetching remote state and exporting to YAML.
- `src/apply.rs`: Logic for reconciling local YAML state with the remote server (Create/Update/Delete).
- `src/plan.rs`: Logic for calculating and displaying differences between local and remote states.
- `src/validate.rs`: Local configuration structure and syntax validation.
- `src/utils/secrets.rs`: Core logic for identifying and masking sensitive data in configuration files.

## Building and Running

### Prerequisites
- Rust and Cargo (latest stable recommended)

### Build Commands
- **Debug Build**: `cargo build`
- **Release Build**: `cargo build --release`
- **Run Locally**: `cargo run -- <COMMAND> [ARGS]` (e.g., `cargo run -- plan --input config`)

### Testing
- **Run All Tests**: `cargo test`
- **Run Benchmarks**: `cargo bench`

### Environment Configuration
The tool requires several environment variables to connect to Keycloak:
- `KEYCLOAK_URL`: Base URL of the Keycloak server.
- `KEYCLOAK_USER` / `KEYCLOAK_PASSWORD`: Admin credentials.
- `KEYCLOAK_CLIENT_ID` / `KEYCLOAK_CLIENT_SECRET`: Client credentials for authentication.

## Development Conventions

### Coding Style
- Follow standard Rust idioms and `rustfmt` conventions.
- Use `anyhow::Result` for error handling in high-level logic.
- Utilize `serde(flatten)` and `HashMap<String, Value>` in models to maintain compatibility with unknown or extra Keycloak fields.

### Secret Management
- **Never commit plain-text secrets.**
- Use `kcd inspect` to automatically generate placeholders like `${KEYCLOAK_..._SECRET}` in YAML files.
- The tool looks for keys containing "secret", "password", or "value" to identify sensitive data.
- Ensure any new sensitive fields are added to the heuristics in `src/utils/secrets.rs` if they aren't caught by the current logic.

### Configuration Structure
Local configuration is organized by realm and then by resource type:
```
config/
└── <realm-name>/
    ├── realm.yaml
    ├── clients/
    ├── users/
    ├── roles/
    └── ...
```

### Contribution Guidelines
- When adding new Keycloak resources, update `src/models.rs`, `src/client.rs`, and the corresponding logic in `inspect.rs`, `plan.rs`, and `apply.rs`.
- Ensure new features are accompanied by unit tests in the relevant module or integration tests in `tests/`.
- Validate changes using `cargo test` before submitting.
