use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Keycloak Server URL
    #[arg(long, env = "KEYCLOAK_URL")]
    pub server: String,

    /// Keycloak Realm
    #[arg(long, env = "KEYCLOAK_REALM")]
    pub realm: String,

    /// Keycloak Admin User
    #[arg(long, env = "KEYCLOAK_USER")]
    pub user: Option<String>,

    /// Keycloak Admin Password
    #[arg(skip)]
    pub password: Option<String>,

    /// Keycloak Client ID (for client credentials grant)
    #[arg(long, env = "KEYCLOAK_CLIENT_ID", default_value = "admin-cli")]
    pub client_id: String,

    /// Keycloak Client Secret (for client credentials grant)
    #[arg(skip)]
    pub client_secret: Option<String>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Inspect the current Keycloak configuration and dump to files
    Inspect {
        /// Output directory for configuration files
        #[arg(long, short, default_value = "config")]
        output: PathBuf,
    },
    /// Validate the local Keycloak configuration files
    Validate {
        /// Input directory containing configuration files
        #[arg(long, short, default_value = "config")]
        input: PathBuf,
    },
    /// Apply the local Keycloak configuration to the server
    Apply {
        /// Input directory containing configuration files
        #[arg(long, short, default_value = "config")]
        input: PathBuf,
    },
    /// Plan the application of the local Keycloak configuration
    Plan {
        /// Input directory containing configuration files
        #[arg(long, short, default_value = "config")]
        input: PathBuf,

        /// Show only changes, suppressing "No changes" messages
        #[arg(long, short = 'c')]
        changes_only: bool,
    },
    /// Check for drift between local configuration and server
    Drift {
        /// Input directory containing configuration files
        #[arg(long, short, default_value = "config")]
        input: PathBuf,
    },
}
