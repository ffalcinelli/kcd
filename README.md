# Keycloak Continuous Delivery (kcd)

**Disclaimer**: This project is experimentally written almost entirely by AI, so any usage should keep this in mind and that the execution of this software is at your own risk.

A CLI tool to manage Keycloak configurations using local YAML files. It allows you to inspect the current state, validate local files, plan changes, and apply them to a Keycloak server.

## Features

- **Inspect**: Fetches current Keycloak configuration and dumps it to local YAML files.
- **Validate**: Validates the structure and content of local configuration files.
- **Plan**: Shows a detailed diff between local configuration and the server's state, previewing changes before applying them.
- **Apply**: Applies local configuration changes to the Keycloak server (Create, Update, Delete).
- **Drift**: Checks for drift between the local configuration and the server's state, showing only the differences.
- **Rotate Keys**: Rotates realm keys by creating new key provider components with incremented priority.
- **Supported Resources**: Realm, Roles, Identity Providers, Clients, Client Scopes, Groups, Users, Authentication Flows, Required Actions, and Components.

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

3. The binary will be available at `target/release/kcd`. You can also install it directly:
   ```bash
   cargo install --path .
   ```

## Configuration

The tool uses environment variables for authentication and connection details. You can set these in your shell or use a `.secrets` file in the project root.

| Variable | Description | Default |
| :--- | :--- | :--- |
| `KEYCLOAK_URL` | The base URL of your Keycloak server (e.g., `http://localhost:8080`). | |
| `KEYCLOAK_USER` | Admin username for authentication. | |
| `KEYCLOAK_PASSWORD` | Admin password for authentication. | |
| `KEYCLOAK_CLIENT_ID` | Client ID for authentication. | `admin-cli` |
| `KEYCLOAK_CLIENT_SECRET` | Client Secret for authentication (if using client credentials). | |

**Note:** The target realms are specified using the `--realms` CLI argument (comma-separated). If omitted, the tool auto-detects realms based on existing directories in the input/output path or queries the server for all realms during inspection.

## Project Structure

The configuration files are organized in a specific directory structure. By default, the tool looks for a `config` directory, but you can specify a custom path using the `--input` or `--output` arguments.

```
config/
└── my-realm/                  # Target realm directory
    ├── realm.yaml             # Realm configuration
    ├── clients/               # Client configurations
    │   ├── client-1.yaml
    │   └── ...
    ├── roles/                 # Role configurations
    │   ├── role-1.yaml
    │   └── ...
    ├── identity-providers/    # Identity Provider configurations
    │   ├── google.yaml
    │   └── ...
    ├── client-scopes/         # Client Scopes configurations
    ├── groups/                # Group configurations
    ├── users/                 # User configurations
    ├── authentication-flows/  # Authentication Flow configurations
    ├── required-actions/      # Required Action configurations
    └── components/            # Component configurations
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

### Interactive CLI
Launch an interactive terminal menu where you can easily perform common Keycloak management tasks. The CLI strictly honors the declarative approach and modifies local YAML configuration files (acting as scaffolding generators), preparing them for `kcd apply`.
```bash
kcd cli --config-dir config/
```

Available Actions:
- **Create User**: Prompts for user details and generates a `UserRepresentation` YAML file in `config/<realm>/users/`.
- **Change User Password**: Appends or updates a password credential within a user's local YAML file.
- **Create Client**: Prompts for Client ID and Public vs. Confidential type, generating the YAML in `config/<realm>/clients/`.
- **Rotate Keys**: Locally reads Keycloak key components in `config/<realm>/components/`, increments the priority of active key providers, and writes the new YAML files back to disk.


## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Secret Management

kcd helps securely manage Keycloak secrets (such as client secrets, passwords, or SMTP bind credentials) so they are never stored in plain text in your version-controlled YAML files.

When you run `kcd inspect`, the tool will automatically detect known secret fields using heuristics (`clientSecret`, `password`, `value`, etc.) and replace their plain-text values in the resulting YAML files with environment variable placeholders like `${KEYCLOAK_CLIENT_CLIENTSECRET}`.

Simultaneously, `kcd inspect` aggregates the actual secret values into a single `.secrets` file located in the output directory. It's recommended to add `.secrets` to your `.gitignore`.

When executing `kcd plan` or `kcd apply`, kcd parses these placeholders from your YAML files and resolves them by reading your local environment variables. If a required environment variable is missing during execution, the command fails gracefully with a descriptive error.

### Example Secret Workflow

1. Inspect the realm to export the configuration:
   ```bash
   kcd inspect --output config/
   ```
   This generates your configuration files in `config/` along with a `config/.secrets` file containing your real secrets.

2. Source the `.secrets` file to load secrets into your environment:
   ```bash
   set -a; source config/.secrets; set +a
   ```

3. Make your desired changes to the YAML files. Since the secrets are masked with placeholders (e.g., `${KEYCLOAK_IDP_GOOGLE_CLIENTSECRET}`), it is safe to commit your `config/` directory to source control.

4. Plan and apply your changes (with the secrets loaded in your shell):
   ```bash
   kcd plan --input config/
   kcd apply --input config/
   ```
