# GEMINI.md - Keycloak Continuous Delivery (kcd)

This document serves as the internal developer guide for `kcd`. It explains the architecture, design decisions, and workflows for extending the tool.

## 🏛️ Architecture Overview

`kcd` follows a **Reconciliation Loop** pattern, similar to Kubernetes controllers.

1.  **Desired State**: Defined in local YAML files within the workspace.
2.  **Current State**: Fetched from the Keycloak Admin API.
3.  **Diff Engine (`plan.rs`)**: Compares the two states to identify what needs to be Created, Updated, or Deleted.
4.  **Reconciler (`apply.rs`)**: Executes the necessary API calls to bring the Current State in line with the Desired State.

### Core Modules

-   `src/client.rs`: Low-level wrapper for the Keycloak Admin REST API. Handles authentication (Token/Password/Client Credentials) and HTTP requests using `reqwest`.
-   `src/models.rs`: Serde-based representations of Keycloak resources. We use `#[serde(flatten)]` with a `HashMap<String, Value>` to maintain forward/backward compatibility with unknown Keycloak fields.
-   `src/inspect.rs`: Deep-scans the remote Keycloak server and serializes resources into local files.
-   `src/utils/secrets.rs`: Uses heuristics to find and mask sensitive fields in configuration objects.

---

## 🛠️ Adding a New Resource Support

To support a new Keycloak resource (e.g., "Event Listeners"):

1.  **Update `models.rs`**: Add the `struct` for the resource and register it in the corresponding realm or parent resource.
2.  **Update `client.rs`**: Add CRUD methods for the new resource.
3.  **Update `inspect.rs`**: Add a function to fetch and save the resource to disk.
4.  **Update `plan.rs`**: Logic to compare the local and remote versions of the resource.
5.  **Update `apply.rs`**: Hook the resource into the reconciliation loop.
6.  **Update `validate.rs`**: (Optional) Add specific validation rules.
7.  **Update `cli.rs`**: (Optional) Add interactive scaffolding for the new resource.

---

## 🧪 Testing Strategy

`kcd` employs a multi-layered testing strategy:

### Unit Tests
Located within the modules themselves (e.g., `src/utils/secrets.rs`). Focused on pure logic like secret masking, path resolution, and YAML parsing.

### Integration Tests
Located in `tests/`.
-   **Common**: Shared utilities for setting up temporary workspaces.
-   **Mocked Tests**: Use `mockito` or similar (if implemented) to simulate Keycloak responses.
-   **Real Integration**: Requires a live Keycloak instance (configured via environment variables). See `tests/real_integration_test.rs`.

### Benchmarks
Located in `benches/`. Used to monitor performance for large workspaces with thousands of files.

---

## 🔐 Secret Management Logic

The masking heuristic looks for keys matching these patterns:
-   Contains `secret` (case-insensitive)
-   Contains `password`
-   Matches exactly `value` (for certain component configurations)
-   Matches exactly `hashedValue`

When detected, the value is replaced by `${KEYCLOAK_<RESOURCE_TYPE>_<RESOURCE_NAME>_<FIELD_NAME>}` and written to the `.secrets` file.

---

## 📜 Coding Conventions

1.  **Asynchronous by Default**: All I/O and API operations must use `tokio`.
2.  **Error Handling**: Use `anyhow::Context` for descriptive error chains.
3.  **Formatting**: Run `cargo fmt` before every commit.
4.  **Clippy**: Ensure `cargo clippy` passes without warnings.
5.  **Serialization**: Prefer `serde_yaml_ng` for YAML operations to ensure compatibility with modern YAML features.

---

## 🚀 Future Roadmap

-   [ ] Support for custom SPIs and provider configurations.
-   [ ] Parallel reconciliation (apply changes concurrently for different realms).
-   [ ] Support for multiple environment profiles (e.g., `prod.yaml`, `staging.yaml`).
-   [ ] Integration with HashiCorp Vault for secret resolution.
