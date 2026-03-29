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
use crate::utils::ui::{ACTION, SUCCESS_CREATE, SUCCESS_UPDATE, WARN};
use anyhow::Result;
use console::style;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs as async_fs;
use tokio::task::JoinSet;

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

    let mut set = tokio::task::JoinSet::new();

    for realm_name in realms {
        let mut realm_client = client.clone();
        realm_client.set_target_realm(realm_name.clone());
        let realm_dir = workspace_dir.join(&realm_name);
        let env_vars = Arc::clone(&env_vars);
        let planned_files = Arc::clone(&planned_files);

        set.spawn(async move {
            println!(
                "\n{} {}",
                ACTION,
                style(format!("Applying realm: {}", realm_name))
                    .cyan()
                    .bold()
            );

            apply_single_realm(
                &realm_client,
                realm_dir,
                env_vars,
                planned_files,
                &realm_name,
            )
            .await
        });
    }

    while let Some(res) = set.join_next().await {
        res??;
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
    realm_name: &str,
) -> Result<()> {
    realm::apply_realm(
        client,
        &workspace_dir,
        Arc::clone(&env_vars),
        Arc::clone(&planned_files),
        realm_name,
    )
    .await?;

    let mut set = JoinSet::new();

    // Roles
    {
        let client = client.clone();
        let workspace_dir = workspace_dir.clone();
        let env_vars = Arc::clone(&env_vars);
        let planned_files = Arc::clone(&planned_files);
        let realm_name = realm_name.to_string();
        set.spawn(async move {
            roles::apply_roles(
                &client,
                &workspace_dir,
                env_vars,
                planned_files,
                &realm_name,
            )
            .await
        });
    }

    // Identity Providers
    {
        let client = client.clone();
        let workspace_dir = workspace_dir.clone();
        let env_vars = Arc::clone(&env_vars);
        let planned_files = Arc::clone(&planned_files);
        let realm_name = realm_name.to_string();
        set.spawn(async move {
            idps::apply_identity_providers(
                &client,
                &workspace_dir,
                env_vars,
                planned_files,
                &realm_name,
            )
            .await
        });
    }

    // Clients
    {
        let client = client.clone();
        let workspace_dir = workspace_dir.clone();
        let env_vars = Arc::clone(&env_vars);
        let planned_files = Arc::clone(&planned_files);
        let realm_name = realm_name.to_string();
        set.spawn(async move {
            clients::apply_clients(
                &client,
                &workspace_dir,
                env_vars,
                planned_files,
                &realm_name,
            )
            .await
        });
    }

    // Client Scopes
    {
        let client = client.clone();
        let workspace_dir = workspace_dir.clone();
        let env_vars = Arc::clone(&env_vars);
        let planned_files = Arc::clone(&planned_files);
        let realm_name = realm_name.to_string();
        set.spawn(async move {
            scopes::apply_client_scopes(
                &client,
                &workspace_dir,
                env_vars,
                planned_files,
                &realm_name,
            )
            .await
        });
    }

    // Groups
    {
        let client = client.clone();
        let workspace_dir = workspace_dir.clone();
        let env_vars = Arc::clone(&env_vars);
        let planned_files = Arc::clone(&planned_files);
        let realm_name = realm_name.to_string();
        set.spawn(async move {
            groups::apply_groups(
                &client,
                &workspace_dir,
                env_vars,
                planned_files,
                &realm_name,
            )
            .await
        });
    }

    // Users
    {
        let client = client.clone();
        let workspace_dir = workspace_dir.clone();
        let env_vars = Arc::clone(&env_vars);
        let planned_files = Arc::clone(&planned_files);
        let realm_name = realm_name.to_string();
        set.spawn(async move {
            users::apply_users(
                &client,
                &workspace_dir,
                env_vars,
                planned_files,
                &realm_name,
            )
            .await
        });
    }

    // Authentication Flows
    {
        let client = client.clone();
        let workspace_dir = workspace_dir.clone();
        let env_vars = Arc::clone(&env_vars);
        let planned_files = Arc::clone(&planned_files);
        let realm_name = realm_name.to_string();
        set.spawn(async move {
            flows::apply_authentication_flows(
                &client,
                &workspace_dir,
                env_vars,
                planned_files,
                &realm_name,
            )
            .await
        });
    }

    // Required Actions
    {
        let client = client.clone();
        let workspace_dir = workspace_dir.clone();
        let env_vars = Arc::clone(&env_vars);
        let planned_files = Arc::clone(&planned_files);
        let realm_name = realm_name.to_string();
        set.spawn(async move {
            actions::apply_required_actions(
                &client,
                &workspace_dir,
                env_vars,
                planned_files,
                &realm_name,
            )
            .await
        });
    }

    // Components
    {
        let client = client.clone();
        let workspace_dir = workspace_dir.clone();
        let env_vars = Arc::clone(&env_vars);
        let planned_files = Arc::clone(&planned_files);
        let realm_name = realm_name.to_string();
        set.spawn(async move {
            components::apply_components_or_keys(
                &client,
                &workspace_dir,
                "components",
                env_vars,
                planned_files,
                &realm_name,
            )
            .await
        });
    }

    // Keys
    {
        let client = client.clone();
        let workspace_dir = workspace_dir.clone();
        let env_vars = Arc::clone(&env_vars);
        let planned_files = Arc::clone(&planned_files);
        let realm_name = realm_name.to_string();
        set.spawn(async move {
            components::apply_components_or_keys(
                &client,
                &workspace_dir,
                "keys",
                env_vars,
                planned_files,
                &realm_name,
            )
            .await
        });
    }

    while let Some(res) = set.join_next().await {
        res??;
    }

    Ok(())
}
