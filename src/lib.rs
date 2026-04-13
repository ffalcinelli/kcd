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
use std::sync::Arc;

static ACTION: Emoji<'_, '_> = Emoji("🚀 ", ">> ");
static SEARCH: Emoji<'_, '_> = Emoji("🔍 ", "> ");

pub async fn init_client(cli: &Cli) -> Result<KeycloakClient> {
    let mut client = KeycloakClient::new(cli.server.clone());
    client
        .login(
            &cli.client_id,
            cli.client_secret.as_deref(),
            cli.user.as_deref(),
            cli.password.as_deref(),
        )
        .await
        .context("Login failed")?;
    Ok(client)
}

pub async fn run_app(cli: Cli) -> Result<()> {
    match &cli.command {
        Commands::Inspect { workspace, yes } => {
            let client = init_client(&cli).await?;
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
        Commands::Apply { workspace, yes } => {
            let client = init_client(&cli).await?;
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
            apply::run(&client, workspace.clone(), &cli.realms, *yes).await?;
        }
        Commands::Plan {
            workspace,
            changes_only,
            interactive,
        } => {
            let client = init_client(&cli).await?;
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
            )
            .await?;
        }
        Commands::Drift { workspace } => {
            let client = init_client(&cli).await?;
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
