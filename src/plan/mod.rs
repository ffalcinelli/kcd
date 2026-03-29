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
use crate::utils::secrets::obfuscate_secrets;
use crate::utils::ui::{ACTION, CHECK, MEMO, WARN};

use anyhow::Result;
use console::{Style, style};
use serde::Serialize;
use similar::{ChangeTag, TextDiff};
use std::collections::HashMap;
use std::env;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs as async_fs;

#[derive(Debug, Clone, Copy)]
pub struct PlanOptions {
    pub changes_only: bool,
    pub interactive: bool,
}

pub async fn run(
    client: &KeycloakClient,
    workspace_dir: PathBuf,
    changes_only: bool,
    interactive: bool,
    realms_to_plan: &[String],
) -> Result<()> {
    if !workspace_dir.exists() {
        anyhow::bail!("Input directory {:?} does not exist", workspace_dir);
    }

    // Load .secrets from input directory if it exists
    let env_path = workspace_dir.join(".secrets");
    if env_path.exists() {
        dotenvy::from_path(&env_path).ok();
    }

    let env_vars = Arc::new(env::vars().collect::<HashMap<String, String>>());

    let realms = if realms_to_plan.is_empty() {
        let mut dirs = Vec::new();
        let mut entries = async_fs::read_dir(&workspace_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            if entry.file_type().await?.is_dir() {
                dirs.push(entry.file_name().to_string_lossy().to_string());
            }
        }
        dirs
    } else {
        realms_to_plan.to_vec()
    };

    if realms.is_empty() {
        println!(
            "{} {}",
            WARN,
            style(format!("No realms found to plan in {:?}", workspace_dir)).yellow()
        );
        return Ok(());
    }

    let mut set = tokio::task::JoinSet::new();

    for realm_name in realms {
        let mut realm_client = client.clone();
        realm_client.set_target_realm(realm_name.clone());
        let realm_dir = workspace_dir.join(&realm_name);
        let env_vars = Arc::clone(&env_vars);

        set.spawn(async move {
            println!(
                "\n{} {}",
                ACTION,
                style(format!("Planning changes for realm: {}", realm_name))
                    .cyan()
                    .bold()
            );

            let mut changed_files = Vec::new();
            plan_single_realm(
                &realm_client,
                realm_dir,
                changes_only,
                interactive,
                env_vars,
                &mut changed_files,
                &realm_name,
            )
            .await?;

            Ok::<Vec<PathBuf>, anyhow::Error>(changed_files)
        });
    }

    let mut changed_files = Vec::new();
    while let Some(res) = set.join_next().await {
        changed_files.extend(res??);
    }
    changed_files.sort();

    let plan_file = workspace_dir.join(".kcdplan");
    if changed_files.is_empty() {
        if async_fs::try_exists(&plan_file).await? {
            async_fs::remove_file(&plan_file).await?;
        }
    } else {
        let content = serde_json::to_string_pretty(&changed_files)?;
        async_fs::write(&plan_file, content).await?;
    }

    Ok(())
}

async fn plan_single_realm(
    client: &KeycloakClient,
    workspace_dir: PathBuf,
    changes_only: bool,
    interactive: bool,
    env_vars: Arc<HashMap<String, String>>,
    changed_files: &mut Vec<PathBuf>,
    realm_name: &str,
) -> Result<()> {
    realm::plan_realm(
        client,
        &workspace_dir,
        changes_only,
        interactive,
        Arc::clone(&env_vars),
        changed_files,
        realm_name,
    )
    .await?;

    roles::plan_roles(
        client,
        &workspace_dir,
        changes_only,
        interactive,
        Arc::clone(&env_vars),
        changed_files,
        realm_name,
    )
    .await?;

    clients::plan_clients(
        client,
        &workspace_dir,
        changes_only,
        interactive,
        Arc::clone(&env_vars),
        changed_files,
        realm_name,
    )
    .await?;

    idps::plan_identity_providers(
        client,
        &workspace_dir,
        changes_only,
        interactive,
        Arc::clone(&env_vars),
        changed_files,
        realm_name,
    )
    .await?;

    scopes::plan_client_scopes(
        client,
        &workspace_dir,
        changes_only,
        interactive,
        Arc::clone(&env_vars),
        changed_files,
        realm_name,
    )
    .await?;

    groups::plan_groups(
        client,
        &workspace_dir,
        changes_only,
        interactive,
        Arc::clone(&env_vars),
        changed_files,
        realm_name,
    )
    .await?;

    users::plan_users(
        client,
        &workspace_dir,
        changes_only,
        interactive,
        Arc::clone(&env_vars),
        changed_files,
        realm_name,
    )
    .await?;

    flows::plan_authentication_flows(
        client,
        &workspace_dir,
        changes_only,
        interactive,
        Arc::clone(&env_vars),
        changed_files,
        realm_name,
    )
    .await?;

    actions::plan_required_actions(
        client,
        &workspace_dir,
        changes_only,
        interactive,
        Arc::clone(&env_vars),
        changed_files,
        realm_name,
    )
    .await?;

    let options = PlanOptions {
        changes_only,
        interactive,
    };

    components::plan_components_or_keys(
        client,
        &workspace_dir,
        options,
        "components",
        Arc::clone(&env_vars),
        changed_files,
        realm_name,
    )
    .await?;
    components::plan_components_or_keys(
        client,
        &workspace_dir,
        options,
        "keys",
        Arc::clone(&env_vars),
        changed_files,
        realm_name,
    )
    .await?;
    components::check_keys_drift(client, options, realm_name).await?;

    Ok(())
}

pub fn print_diff<T: Serialize>(
    name: &str,
    old: Option<&T>,
    new: &T,
    changes_only: bool,
    prefix: &str,
) -> Result<bool> {
    let old_yaml = if let Some(o) = old {
        let mut val = serde_json::to_value(o)?;
        obfuscate_secrets(&mut val, prefix);
        crate::utils::to_sorted_yaml(&val)?
    } else {
        String::new()
    };

    let mut new_val = serde_json::to_value(new)?;
    obfuscate_secrets(&mut new_val, prefix);
    let new_yaml = crate::utils::to_sorted_yaml(&new_val)?;

    let diff = TextDiff::from_lines(&old_yaml, &new_yaml);
    let changed = diff.ratio() < 1.0;

    if changed {
        println!("\n{} Changes for {}:", MEMO, name);
        for change in diff.iter_all_changes() {
            let (sign, style) = match change.tag() {
                ChangeTag::Delete => ("-", Style::new().red()),
                ChangeTag::Insert => ("+", Style::new().green()),
                ChangeTag::Equal => (" ", Style::new().dim()),
            };
            print!("{}{}", style.apply_to(sign).bold(), style.apply_to(change));
        }
    } else if !changes_only {
        println!("{} No changes for {}", CHECK, name);
    }
    Ok(changed)
}
