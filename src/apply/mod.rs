pub mod actions;
pub mod clients;
pub mod components;
pub mod flows;
pub mod groups;
pub mod idps;
pub mod realm;
pub mod roles;
pub mod scopes;
pub mod users;

use crate::client::KeycloakClient;
use anyhow::Result;
use console::{Emoji, style};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs as async_fs;

pub static ACTION: Emoji<'_, '_> = Emoji("🚀 ", ">> ");
pub static SUCCESS_CREATE: Emoji<'_, '_> = Emoji("✨ ", "+ ");
pub static SUCCESS_UPDATE: Emoji<'_, '_> = Emoji("🔄 ", "~ ");
pub static WARN: Emoji<'_, '_> = Emoji("⚠️ ", "! ");

pub async fn run(
    client: &KeycloakClient,
    workspace_dir: PathBuf,
    realms_to_apply: &[String],
    yes: bool,
) -> Result<()> {
    if !workspace_dir.exists() {
        anyhow::bail!("Input directory {:?} does not exist", workspace_dir);
    }

    // Load .secrets from input directory if it exists
    let env_path = workspace_dir.join(".secrets");
    if env_path.exists() {
        dotenvy::from_path(&env_path).ok();
    }

    let env_vars = Arc::new(std::env::vars().collect::<HashMap<String, String>>());

    // Check for .kcdplan
    let plan_path = workspace_dir.join(".kcdplan");
    let planned_files = if plan_path.exists() {
        let content = async_fs::read_to_string(&plan_path).await?;
        let items: Vec<PathBuf> = serde_json::from_str(&content)?;
        if items.is_empty() {
            if !yes {
                let proceed =
                    dialoguer::Confirm::with_theme(&dialoguer::theme::ColorfulTheme::default())
                        .with_prompt(
                            "No planned changes found. Send everything to Keycloak anyway?",
                        )
                        .default(false)
                        .interact()?;
                if !proceed {
                    println!("Aborted.");
                    return Ok(());
                }
            }
            Arc::new(None)
        } else {
            let hashset: HashSet<PathBuf> = items.into_iter().collect();
            Arc::new(Some(hashset))
        }
    } else {
        if !yes {
            let proceed =
                dialoguer::Confirm::with_theme(&dialoguer::theme::ColorfulTheme::default())
                    .with_prompt("No planned changes found. Send everything to Keycloak anyway?")
                    .default(false)
                    .interact()?;
            if !proceed {
                println!("Aborted.");
                return Ok(());
            }
        }
        Arc::new(None)
    };

    let realms = if realms_to_apply.is_empty() {
        let mut dirs = Vec::new();
        let mut entries = async_fs::read_dir(&workspace_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            if entry.file_type().await?.is_dir() {
                dirs.push(entry.file_name().to_string_lossy().to_string());
            }
        }
        dirs
    } else {
        realms_to_apply.to_vec()
    };

    if realms.is_empty() {
        println!(
            "{} {}",
            WARN,
            style(format!("No realms found to apply in {:?}", workspace_dir)).yellow()
        );
        return Ok(());
    }

    for realm_name in realms {
        println!(
            "\n{} {}",
            ACTION,
            style(format!("Applying realm: {}", realm_name))
                .cyan()
                .bold()
        );
        let mut realm_client = client.clone();
        realm_client.set_target_realm(realm_name.clone());
        let realm_dir = workspace_dir.join(&realm_name);
        apply_single_realm(
            &realm_client,
            realm_dir,
            Arc::clone(&env_vars),
            Arc::clone(&planned_files),
        )
        .await?;
    }

    // Success - remove plan
    if plan_path.exists() {
        let _ = async_fs::remove_file(plan_path).await;
    }

    Ok(())
}

async fn apply_single_realm(
    client: &KeycloakClient,
    workspace_dir: PathBuf,
    env_vars: Arc<HashMap<String, String>>,
    planned_files: Arc<Option<HashSet<PathBuf>>>,
) -> Result<()> {
    realm::apply_realm(
        client,
        &workspace_dir,
        Arc::clone(&env_vars),
        Arc::clone(&planned_files),
    )
    .await?;
    roles::apply_roles(
        client,
        &workspace_dir,
        Arc::clone(&env_vars),
        Arc::clone(&planned_files),
    )
    .await?;
    idps::apply_identity_providers(
        client,
        &workspace_dir,
        Arc::clone(&env_vars),
        Arc::clone(&planned_files),
    )
    .await?;
    clients::apply_clients(
        client,
        &workspace_dir,
        Arc::clone(&env_vars),
        Arc::clone(&planned_files),
    )
    .await?;
    scopes::apply_client_scopes(
        client,
        &workspace_dir,
        Arc::clone(&env_vars),
        Arc::clone(&planned_files),
    )
    .await?;
    groups::apply_groups(
        client,
        &workspace_dir,
        Arc::clone(&env_vars),
        Arc::clone(&planned_files),
    )
    .await?;
    users::apply_users(
        client,
        &workspace_dir,
        Arc::clone(&env_vars),
        Arc::clone(&planned_files),
    )
    .await?;
    flows::apply_authentication_flows(
        client,
        &workspace_dir,
        Arc::clone(&env_vars),
        Arc::clone(&planned_files),
    )
    .await?;
    actions::apply_required_actions(
        client,
        &workspace_dir,
        Arc::clone(&env_vars),
        Arc::clone(&planned_files),
    )
    .await?;
    components::apply_components_or_keys(
        client,
        &workspace_dir,
        "components",
        Arc::clone(&env_vars),
        Arc::clone(&planned_files),
    )
    .await?;
    components::apply_components_or_keys(
        client,
        &workspace_dir,
        "keys",
        Arc::clone(&env_vars),
        Arc::clone(&planned_files),
    )
    .await?;

    Ok(())
}
