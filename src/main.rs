use anyhow::{Context, Result};
use app::apply;
use app::args::{Cli, Commands};
use app::clean;
use app::cli as interactive_cli;
use app::client::KeycloakClient;
use app::inspect;
use app::plan;
use app::validate;
use clap::Parser;
use console::{Emoji, style};

static ACTION: Emoji<'_, '_> = Emoji("🚀 ", ">> ");
static SEARCH: Emoji<'_, '_> = Emoji("🔍 ", "> ");

async fn init_client(cli: &Cli) -> Result<KeycloakClient> {
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

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    dotenvy::from_filename(".secrets").ok();
    env_logger::init();

    let mut cli = Cli::parse();

    if cli.password.is_none() {
        cli.password = std::env::var("KEYCLOAK_PASSWORD").ok();
    }
    if cli.client_secret.is_none() {
        cli.client_secret = std::env::var("KEYCLOAK_CLIENT_SECRET").ok();
    }

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
            validate::run(workspace.clone(), &cli.realms)?;
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
            plan::run(&client, workspace.clone(), true, false, &cli.realms).await?;
        }
        Commands::Cli { workspace } => {
            interactive_cli::run(workspace.clone()).await?;
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
