pub mod apply;
pub mod args;
pub mod clean;
pub mod cli;
pub mod client;
pub mod inspect;
pub mod models;
pub mod plan;
pub mod utils;
pub mod validate;

use anyhow::{Context, Result};
use args::{Cli, Commands};
use client::KeycloakClient;
use console::{Emoji, style};
use std::collections::HashMap;
use std::sync::Arc;
use utils::secrets::vault::VaultResolver;
use utils::secrets::{CompositeResolver, EnvResolver, SecretResolver};

static ACTION: Emoji<'_, '_> = Emoji("🚀 ", ">> ");
static SEARCH: Emoji<'_, '_> = Emoji("🔍 ", "> ");

#[derive(serde::Deserialize, Debug, Clone)]
pub struct Profile {
    pub server_url: String,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub user: Option<String>,
    pub password: Option<String>,
    pub secrets_file: Option<String>,
    pub vault_addr: Option<String>,
    pub vault_token: Option<String>,
}

pub async fn load_profile(workspace: &std::path::Path, name: &str) -> Result<Profile> {
    let profile_path = workspace.join("profiles").join(format!("{}.yaml", name));
    let content = std::fs::read_to_string(&profile_path)
        .with_context(|| format!("Failed to read profile file: {:?}", profile_path))?;
    let profile: Profile = serde_yaml::from_str(&content)
        .with_context(|| format!("Failed to parse profile file: {:?}", profile_path))?;
    Ok(profile)
}

pub async fn init_client(cli: &Cli, profile: Option<&Profile>) -> Result<KeycloakClient> {
    let server = profile
        .map(|p| p.server_url.clone())
        .or_else(|| cli.server.clone())
        .context("Keycloak server URL not provided (neither via --server nor --profile)")?;

    let client_id = profile
        .and_then(|p| p.client_id.clone())
        .unwrap_or_else(|| cli.client_id.clone());

    let client_secret = profile
        .and_then(|p| p.client_secret.clone())
        .or_else(|| cli.client_secret.clone());

    let user = profile
        .and_then(|p| p.user.clone())
        .or_else(|| cli.user.clone());

    let password = profile
        .and_then(|p| p.password.clone())
        .or_else(|| cli.password.clone());

    let mut client = KeycloakClient::new(server);
    client
        .login(
            &client_id,
            client_secret.as_deref(),
            user.as_deref(),
            password.as_deref(),
        )
        .await
        .context("Login failed")?;
    Ok(client)
}

pub async fn init_secrets(
    cli: &Cli,
    workspace: &std::path::Path,
    profile: Option<&Profile>,
) -> Result<Arc<dyn SecretResolver>> {
    // Load secrets from profile-specific secrets file or default .secrets
    let secrets_file = profile
        .and_then(|p| p.secrets_file.as_deref())
        .unwrap_or(".secrets");

    let env_path = workspace.join(secrets_file);
    if env_path.exists() {
        dotenvy::from_path(&env_path).ok();
    }

    let mut resolvers: Vec<Box<dyn SecretResolver>> = Vec::new();

    let vault_addr = profile
        .and_then(|p| p.vault_addr.clone())
        .or_else(|| cli.vault_addr.clone());

    let vault_token = profile
        .and_then(|p| p.vault_token.clone())
        .or_else(|| cli.vault_token.clone());

    if let (Some(addr), Some(token)) = (vault_addr, vault_token) {
        resolvers.push(Box::new(VaultResolver::new(&addr, &token)?));
    }

    resolvers.push(Box::new(EnvResolver::new(
        std::env::vars().collect::<HashMap<String, String>>(),
    )));

    Ok(Arc::new(CompositeResolver::new(resolvers)))
}

pub async fn run_app(cli: Cli) -> Result<()> {
    let workspace = match &cli.command {
        Commands::Inspect { workspace, .. } => workspace,
        Commands::Validate { workspace } => workspace,
        Commands::Apply { workspace, .. } => workspace,
        Commands::Plan { workspace, .. } => workspace,
        Commands::Drift { workspace } => workspace,
        Commands::Cli { workspace } => workspace,
        Commands::Clean { workspace, .. } => workspace,
    };

    let profile = if let Some(p) = &cli.profile {
        Some(load_profile(workspace, p).await?)
    } else {
        None
    };

    match &cli.command {
        Commands::Inspect { workspace, yes } => {
            let client = init_client(&cli, profile.as_ref()).await?;
            println!(
                "{} {}",
                SEARCH,
                style(format!(
                    "Inspecting Keycloak configuration into {:?}",
                    workspace
                ))
                .cyan()
                .bold()
            );
            inspect::run(&client, workspace.clone(), &cli.realms, *yes).await?;
        }
        Commands::Validate { workspace } => {
            println!(
                "{} {}",
                SEARCH,
                style(format!(
                    "Validating Keycloak configuration from {:?}",
                    workspace
                ))
                .cyan()
                .bold()
            );
            validate::run(workspace.clone(), &cli.realms).await?;
        }
        Commands::Apply {
            workspace,
            yes,
            review,
        } => {
            let client = init_client(&cli, profile.as_ref()).await?;
            let resolver = init_secrets(&cli, workspace, profile.as_ref()).await?;
            println!(
                "{} {}",
                ACTION,
                style(format!(
                    "Applying Keycloak configuration from {:?}",
                    workspace
                ))
                .cyan()
                .bold()
            );
            apply::run(
                &client,
                workspace.clone(),
                &cli.realms,
                *yes,
                *review,
                Arc::new(crate::utils::ui::DialoguerUi::new()),
                resolver,
                cli.profile.clone(),
            )
            .await?;
        }
        Commands::Plan {
            workspace,
            changes_only,
            interactive,
        } => {
            let client = init_client(&cli, profile.as_ref()).await?;
            let resolver = init_secrets(&cli, workspace, profile.as_ref()).await?;
            println!(
                "{} {}",
                SEARCH,
                style(format!(
                    "Planning Keycloak configuration from {:?}",
                    workspace
                ))
                .cyan()
                .bold()
            );
            plan::run(
                &client,
                workspace.clone(),
                *changes_only,
                *interactive,
                &cli.realms,
                Arc::new(crate::utils::ui::DialoguerUi::new()),
                resolver,
                cli.profile.clone(),
            )
            .await?;
        }
        Commands::Drift { workspace } => {
            let client = init_client(&cli, profile.as_ref()).await?;
            let resolver = init_secrets(&cli, workspace, profile.as_ref()).await?;
            println!(
                "{} {}",
                SEARCH,
                style(format!(
                    "Checking drift for Keycloak configuration from {:?}",
                    workspace
                ))
                .cyan()
                .bold()
            );
            plan::run(
                &client,
                workspace.clone(),
                true,
                false,
                &cli.realms,
                Arc::new(crate::utils::ui::DialoguerUi::new()),
                resolver,
                cli.profile.clone(),
            )
            .await?;
        }
        Commands::Cli { workspace } => {
            cli::run(workspace.clone(), &crate::utils::ui::DialoguerUi::new()).await?;
        }
        Commands::Clean { workspace, yes } => {
            println!(
                "{} {}",
                ACTION,
                style(format!(
                    "Cleaning up Keycloak configuration in {:?}",
                    workspace
                ))
                .cyan()
                .bold()
            );
            clean::run(workspace.clone(), *yes, &cli.realms).await?;
        }
    }

    Ok(())
}
