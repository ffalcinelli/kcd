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
        Commands::Inspect { output, yes } => {
            let client = init_client(&cli).await?;
            println!("Inspecting Keycloak configuration into {:?}", output);
            inspect::run(&client, output.clone(), &cli.realms, *yes).await?;
        }
        Commands::Validate { input } => {
            println!("Validating Keycloak configuration from {:?}", input);
            validate::run(input.clone(), &cli.realms)?;
        }
        Commands::Apply { input } => {
            let client = init_client(&cli).await?;
            println!("Applying Keycloak configuration from {:?}", input);
            apply::run(&client, input.clone(), &cli.realms).await?;
        }
        Commands::Plan {
            input,
            changes_only,
        } => {
            let client = init_client(&cli).await?;
            println!("Planning Keycloak configuration from {:?}", input);
            plan::run(&client, input.clone(), *changes_only, &cli.realms).await?;
        }
        Commands::Drift { input } => {
            let client = init_client(&cli).await?;
            println!("Checking drift for Keycloak configuration from {:?}", input);
            plan::run(&client, input.clone(), true, &cli.realms).await?;
        }
        Commands::Cli { config_dir } => {
            interactive_cli::run(config_dir.clone()).await?;
        }
        Commands::Clean { output, yes } => {
            clean::run(output.clone(), *yes, &cli.realms).await?;
        }
    }

    Ok(())
}
