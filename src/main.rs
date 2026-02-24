use clap::Parser;
use app::args::{Cli, Commands};
use app::client::KeycloakClient;
use app::inspect;
use app::validate;
use app::apply;
use app::plan;
use anyhow::{Result, Context};

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    env_logger::init();

    let mut cli = Cli::parse();

    if cli.password.is_none() {
        cli.password = std::env::var("KEYCLOAK_PASSWORD").ok();
    }
    if cli.client_secret.is_none() {
        cli.client_secret = std::env::var("KEYCLOAK_CLIENT_SECRET").ok();
    }

    match &cli.command {
        Commands::Inspect { output } => {
            let mut client = KeycloakClient::new(cli.server.clone(), cli.realm.clone());
            client.login(&cli.client_id, cli.client_secret.as_deref(), cli.user.as_deref(), cli.password.as_deref()).await.context("Login failed")?;
            println!("Inspecting Keycloak configuration into {:?}", output);
            inspect::run(&client, output.clone()).await?;
        }
        Commands::Validate { input } => {
            println!("Validating Keycloak configuration from {:?}", input);
            validate::run(input.clone())?;
        }
        Commands::Apply { input } => {
            let mut client = KeycloakClient::new(cli.server.clone(), cli.realm.clone());
            client.login(&cli.client_id, cli.client_secret.as_deref(), cli.user.as_deref(), cli.password.as_deref()).await.context("Login failed")?;
            println!("Applying Keycloak configuration from {:?}", input);
            apply::run(&client, input.clone()).await?;
        }
        Commands::Plan { input } => {
            let mut client = KeycloakClient::new(cli.server.clone(), cli.realm.clone());
            client.login(&cli.client_id, cli.client_secret.as_deref(), cli.user.as_deref(), cli.password.as_deref()).await.context("Login failed")?;
            println!("Planning Keycloak configuration from {:?}", input);
            plan::run(&client, input.clone()).await?;
        }
    }

    Ok(())
}
