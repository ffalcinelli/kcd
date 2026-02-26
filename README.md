# Keycloak Configuration Deployer (kcd)

A CLI tool to manage Keycloak configurations using local YAML files. It allows you to inspect the current state, validate local files, plan changes, and apply them to a Keycloak server.

## Features

- **Inspect**: Fetches current Keycloak configuration (Realm, Clients, Roles, Identity Providers) and dumps it to local YAML files.
- **Validate**: Validates the structure and content of local configuration files.
- **Plan**: Shows a detailed diff between local configuration and the server's state, previewing changes before applying them.
- **Apply**: Applies local configuration changes to the Keycloak server (Create, Update, Delete).
- **Drift**: Checks for drift between the local configuration and the server's state, showing only the differences.

## Installation

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) and Cargo

### Building from Source

1. Clone the repository:
   ```bash
   git clone <repository-url>
   cd kcd
   ```

2. Build the project:
   ```bash
   cargo build --release
   ```

3. The binary will be available at `target/release/app`. You can also install it directly:
   ```bash
   cargo install --path .
   ```

## Configuration

The tool uses environment variables for authentication and connection details. You can set these in your shell or use a `.env` file in the project root.

| Variable | Description | Default |
| :--- | :--- | :--- |
| `KEYCLOAK_URL` | The base URL of your Keycloak server (e.g., `http://localhost:8080`). | |
| `KEYCLOAK_REALM` | The name of the realm to manage. | |
| `KEYCLOAK_USER` | Admin username for authentication. | |
| `KEYCLOAK_PASSWORD` | Admin password for authentication. | |
| `KEYCLOAK_CLIENT_ID` | Client ID for authentication. | `admin-cli` |
| `KEYCLOAK_CLIENT_SECRET` | Client Secret for authentication (if using client credentials). | |

## Project Structure

The configuration files are organized in a specific directory structure. By default, the tool looks for a `config` directory, but you can specify a custom path using the `--input` or `--output` arguments.

```
config/
├── realm.yaml             # Realm configuration
├── clients/               # Client configurations
│   ├── client-1.yaml
│   ├── client-2.yaml
│   └── ...
├── roles/                 # Role configurations
│   ├── role-1.yaml
│   ├── role-2.yaml
│   └── ...
└── identity-providers/    # Identity Provider configurations
    ├── google.yaml
    ├── github.yaml
    └── ...
```

## Usage

### Inspect
Download the current configuration from the Keycloak server and save it to the local filesystem.
```bash
kcd inspect --output config/
```

### Validate
Validate the local configuration files for syntax and structure errors.
```bash
kcd validate --input config/
```

### Plan
Show the differences between the local configuration and the remote Keycloak server. This is useful to preview changes before applying them.
```bash
kcd plan --input config/
```

You can also use the `--changes-only` flag to show only the differences and suppress "No changes" messages.
```bash
kcd plan --input config/ --changes-only
```

### Drift
Check for drift between local configuration and server. This command is equivalent to `plan --changes-only`.
```bash
kcd drift --input config/
```

### Apply
Apply the local configuration to the Keycloak server. This will create new resources, update existing ones, and delete resources that are not present in the local configuration.
```bash
kcd apply --input config/
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
