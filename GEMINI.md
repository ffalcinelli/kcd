# GEMINI.md - Keycloak Configuration Drive (kcd)

This document serves as the internal developer guide for `kcd`. It explains the architecture, design decisions, and workflows for extending the tool.

## 🏛️ Architecture Overview

`kcd` follows a **Reconciliation Loop** pattern, similar to Kubernetes controllers.

1.  **Desired State**: Defined in local YAML files within the workspace.
2.  **Current State**: Fetched from the Keycloak Admin API.
3.  **Diff Engine (`plan.rs`)**: Compares the two states to identify what needs to be Created, Updated, or Deleted. It generates a `.kcdplan` file in the workspace containing the list of files that have pending changes.
4.  **Reconciler (`apply.rs`)**: Executes the necessary API calls to bring the Current State in line with the Desired State. It optionally uses the `.kcdplan` file to only apply changes that were previously planned.

### Core Modules

-   `src/client.rs`: Low-level wrapper for the Keycloak Admin REST API. Handles authentication and provides a **generic CRUD interface** for Keycloak resources.
-   `src/models.rs`: Serde-based representations of Keycloak resources. Defines the `KeycloakResource` and `ResourceMeta` traits for generic resource management.
-   `src/inspect.rs`: Deep-scans the remote Keycloak server and serializes resources into local files using a **generic, parallelized inspection pipeline**.
-   `src/plan/`: Contains the logic for calculating diffs for each resource type. It's structured into submodules for better maintainability.
-   `src/apply/`: Contains the logic for applying changes. It uses **parallelized reconciliation** at the resource type level within each realm (using `tokio::task::JoinSet`).
-   `src/utils/secrets.rs`: Uses heuristics to find and mask sensitive fields in configuration objects.
-   `src/utils/ui.rs`: Centralized module for CLI output formatting, emoji management, and consistent styling across the tool.
-   `src/clean.rs`: Removes local workspace representations of Keycloak realms and resources using **parallel I/O**.
-   `src/validate.rs`: Performs local validation of YAML configurations before they are applied using **async I/O**.
-   `src/cli/`: Logic for the interactive scaffolding of new resources.

---

## 🛠️ Adding a New Resource Support

To support a new Keycloak resource (e.g., "Event Listeners"):

1.  **Update `models.rs`**: 
    - Add the `struct` for the resource.
    - Implement `KeycloakResource` (for name/ID handling and API paths).
    - Implement `ResourceMeta` (to define labels and secret prefixes).
2.  **Update `inspect.rs`**: Add a `spawn_inspect::<NewResourceRepresentation>(...)` call in the `inspect_realm` function.
3.  **Update `plan/`**: Create a new submodule (if needed) and add logic to compare the local and remote versions of the resource. Hook it into `src/plan/mod.rs`.
4.  **Update `apply/`**: Create a new submodule (if needed) and add logic to execute API calls for Creating/Updating the resource. Hook it into `src/apply/mod.rs` (ensure it's added to the parallelized `JoinSet`).
5.  **Update `validate.rs`**: (Optional) Add specific validation rules.
6.  **Update `cli/`**: (Optional) Add interactive scaffolding for the new resource.

---

## 🧪 Testing Strategy

`kcd` employs a multi-layered testing strategy:

### Unit Tests
Located within the modules themselves (e.g., `src/utils/secrets.rs`). Focused on pure logic like secret masking, path resolution, and YAML parsing.

### Integration Tests
Located in `tests/`.
-   **Common**: Shared utilities for setting up temporary workspaces and environment variables.
-   **Mocked Tests**: Use `mockito` to simulate Keycloak responses for various scenarios.
-   **Real Integration**: Requires a live Keycloak instance (configured via environment variables). See `tests/real_integration_test.rs`.
-   **Ultimate Coverage**: `tests/ultimate_coverage_test.rs` and `tests/models_coverage_test.rs` provide comprehensive checks for resource handling.

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
2.  **Concurrency**: Use `tokio::task::JoinSet` to parallelize independent resource operations.
3.  **Generic Abstractions**: Prefer using the generic CRUD methods in `KeycloakClient` and the `KeycloakResource`/`ResourceMeta` traits to avoid boilerplate.
4.  **Error Handling**: Use `anyhow::Context` for descriptive error chains, including specific resource identifiers (e.g., realm name).
5.  **Formatting**: Run `cargo fmt --all -- --check` before every commit and ensure all formatting issues are resolved.
6.  **Clippy**: Ensure `cargo clippy` passes without warnings.
7.  **Serialization**: Prefer `serde_yaml_ng` for YAML operations to ensure compatibility with modern YAML features.

---

## 🚀 Future Roadmap

-   [x] Parallel reconciliation (apply changes concurrently for resources within a realm).
-   [x] Generic refactor for `inspect.rs`.
-   [ ] Support for custom SPIs and provider configurations.
-   [ ] Support for multiple environment profiles (e.g., `prod.yaml`, `staging.yaml`).
-   [ ] Integration with HashiCorp Vault for secret resolution.
-   [ ] Generic refactor for `plan.rs` and `apply.rs` (similar to `inspect.rs`).
