pub mod components;
pub mod generic;
pub mod realm;

use crate::client::KeycloakClient;
use crate::utils::secrets::obfuscate_secrets;
use crate::utils::ui::{ACTION, CHECK, MEMO, Ui, WARN};

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

pub struct PlanContext<'a> {
    pub client: &'a KeycloakClient,
    pub workspace_dir: &'a std::path::Path,
    pub options: PlanOptions,
    pub env_vars: Arc<HashMap<String, String>>,
    pub realm_name: &'a str,
    pub ui: &'a dyn Ui,
}

pub async fn run(
    client: &KeycloakClient,
    workspace_dir: PathBuf,
    changes_only: bool,
    interactive: bool,
    realms_to_plan: &[String],
    ui: Arc<dyn Ui>,
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
        let ui = Arc::clone(&ui);

        set.spawn(async move {
            println!(
                "\n{} {}",
                ACTION,
                style(format!("Planning changes for realm: {}", realm_name))
                    .cyan()
                    .bold()
            );

            let mut changed_files = Vec::new();
            let options = PlanOptions {
                changes_only,
                interactive,
            };
            let ctx = PlanContext {
                client: &realm_client,
                workspace_dir: &realm_dir,
                options,
                env_vars,
                realm_name: &realm_name,
                ui: ui.as_ref(),
            };
            plan_single_realm(ctx, &mut changed_files).await?;

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

use crate::models::{
    AuthenticationFlowRepresentation, ClientRepresentation, ClientScopeRepresentation,
    GroupRepresentation, IdentityProviderRepresentation, RequiredActionProviderRepresentation,
    RoleRepresentation, UserRepresentation,
};

async fn plan_single_realm(
    ctx: PlanContext<'_>,
    changed_files: &mut Vec<PathBuf>,
) -> Result<()> {
    realm::plan_realm(&ctx, changed_files).await?;

    generic::plan_resources::<RoleRepresentation>(&ctx, changed_files).await?;

    generic::plan_resources::<ClientRepresentation>(&ctx, changed_files).await?;

    generic::plan_resources::<IdentityProviderRepresentation>(&ctx, changed_files).await?;

    generic::plan_resources::<ClientScopeRepresentation>(&ctx, changed_files).await?;

    generic::plan_resources::<GroupRepresentation>(&ctx, changed_files).await?;

    generic::plan_resources::<UserRepresentation>(&ctx, changed_files).await?;

    generic::plan_resources::<AuthenticationFlowRepresentation>(&ctx, changed_files).await?;

    generic::plan_resources::<RequiredActionProviderRepresentation>(&ctx, changed_files).await?;

    components::plan_components_or_keys(&ctx, "components", changed_files).await?;
    components::plan_components_or_keys(&ctx, "keys", changed_files).await?;
    components::check_keys_drift(ctx.client, ctx.options, ctx.realm_name).await?;

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
