use crate::client::KeycloakClient;
use crate::models::{
    AuthenticationFlowRepresentation, ClientRepresentation, ClientScopeRepresentation,
    ComponentRepresentation, GroupRepresentation, IdentityProviderRepresentation, KeycloakResource,
    RealmRepresentation, RequiredActionProviderRepresentation, RoleRepresentation,
    UserRepresentation,
};

use anyhow::{Context, Result};
use console::{Emoji, Style, style};
use serde::Serialize;
use similar::{ChangeTag, TextDiff};
use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs as async_fs;

static WARN: Emoji<'_, '_> = Emoji("⚠️ ", "! ");
static ACTION: Emoji<'_, '_> = Emoji("🔍 ", "> ");

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

    let mut changed_files = Vec::new();
    for realm_name in realms {
        let mut realm_client = client.clone();
        realm_client.set_target_realm(realm_name.clone());
        let realm_dir = workspace_dir.join(&realm_name);
        println!(
            "\n{} {}",
            ACTION,
            style(format!("Planning changes for realm: {}", realm_name))
                .cyan()
                .bold()
        );
        plan_single_realm(
            &realm_client,
            realm_dir,
            changes_only,
            interactive,
            Arc::clone(&env_vars),
            &mut changed_files,
        )
        .await?;
    }

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
) -> Result<()> {
    // 1. Plan Realm
    plan_realm(
        client,
        &workspace_dir,
        changes_only,
        interactive,
        Arc::clone(&env_vars),
        changed_files,
    )
    .await?;

    // 2. Plan Roles
    plan_roles(
        client,
        &workspace_dir,
        changes_only,
        interactive,
        Arc::clone(&env_vars),
        changed_files,
    )
    .await?;

    // 3. Plan Clients
    plan_clients(
        client,
        &workspace_dir,
        changes_only,
        interactive,
        Arc::clone(&env_vars),
        changed_files,
    )
    .await?;

    // 4. Plan Identity Providers
    plan_identity_providers(
        client,
        &workspace_dir,
        changes_only,
        interactive,
        Arc::clone(&env_vars),
        changed_files,
    )
    .await?;

    // 5. Plan Client Scopes
    plan_client_scopes(
        client,
        &workspace_dir,
        changes_only,
        interactive,
        Arc::clone(&env_vars),
        changed_files,
    )
    .await?;

    // 6. Plan Groups
    plan_groups(
        client,
        &workspace_dir,
        changes_only,
        interactive,
        Arc::clone(&env_vars),
        changed_files,
    )
    .await?;

    // 7. Plan Users
    plan_users(
        client,
        &workspace_dir,
        changes_only,
        interactive,
        Arc::clone(&env_vars),
        changed_files,
    )
    .await?;

    // 8. Plan Authentication Flows
    plan_authentication_flows(
        client,
        &workspace_dir,
        changes_only,
        interactive,
        Arc::clone(&env_vars),
        changed_files,
    )
    .await?;

    // 9. Plan Required Actions
    plan_required_actions(
        client,
        &workspace_dir,
        changes_only,
        interactive,
        Arc::clone(&env_vars),
        changed_files,
    )
    .await?;

    // 10. Plan Components
    plan_components_or_keys(
        client,
        &workspace_dir,
        changes_only,
        interactive,
        "components",
        Arc::clone(&env_vars),
        changed_files,
    )
    .await?;
    plan_components_or_keys(
        client,
        &workspace_dir,
        changes_only,
        interactive,
        "keys",
        Arc::clone(&env_vars),
        changed_files,
    )
    .await?;
    check_keys_drift(client, changes_only).await?;

    Ok(())
}

use crate::utils::secrets::{obfuscate_secrets, substitute_secrets};

fn print_diff<T: Serialize>(
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
        println!("\n{} Changes for {}:", Emoji("📝", ""), name);
        for change in diff.iter_all_changes() {
            let (sign, style) = match change.tag() {
                ChangeTag::Delete => ("-", Style::new().red()),
                ChangeTag::Insert => ("+", Style::new().green()),
                ChangeTag::Equal => (" ", Style::new().dim()),
            };
            print!("{}{}", style.apply_to(sign).bold(), style.apply_to(change));
        }
    } else if !changes_only {
        println!("{} No changes for {}", Emoji("✅", ""), name);
    }
    Ok(changed)
}

async fn plan_client_scopes(
    client: &KeycloakClient,
    workspace_dir: &Path,
    changes_only: bool,
    interactive: bool,
    env_vars: Arc<HashMap<String, String>>,
    changed_files: &mut Vec<PathBuf>,
) -> Result<()> {
    let scopes_dir = workspace_dir.join("client-scopes");
    if async_fs::try_exists(&scopes_dir).await? {
        let existing_scopes = client.get_client_scopes().await?;
        let existing_scopes_map: HashMap<String, ClientScopeRepresentation> = existing_scopes
            .into_iter()
            .filter_map(|s| s.get_identity().map(|id| (id, s)))
            .collect();

        let mut entries = async_fs::read_dir(&scopes_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "yaml") {
                let content = async_fs::read_to_string(&path).await?;
                let mut val: serde_json::Value = serde_yaml::from_str(&content)
                    .with_context(|| format!("Failed to parse YAML file: {:?}", path))?;
                substitute_secrets(&mut val, &env_vars).map_err(|e| anyhow::anyhow!(e))?;
                let local_scope: ClientScopeRepresentation = serde_json::from_value(val)
                    .with_context(|| format!("Failed to deserialize YAML file: {:?}", path))?;

                let identity = local_scope
                    .get_identity()
                    .context(format!("Failed to get identity for scope in {:?}", path))?;
                let remote = existing_scopes_map.get(&identity);

                let changed = if let Some(remote) = remote {
                    let mut remote_clone = remote.clone();
                    if local_scope.id.is_none() {
                        remote_clone.id = None;
                    }
                    print_diff(
                        &format!("ClientScope {}", local_scope.get_name()),
                        Some(&remote_clone),
                        &local_scope,
                        changes_only,
                        "client_scope",
                    )?
                } else {
                    println!(
                        "\n{} Will create ClientScope: {}",
                        Emoji("✨", ""),
                        local_scope.get_name()
                    );
                    print_diff(
                        &format!("ClientScope {}", local_scope.get_name()),
                        None::<&ClientScopeRepresentation>,
                        &local_scope,
                        changes_only,
                        "client_scope",
                    )?
                };

                if changed {
                    let mut include = true;
                    if interactive {
                        include = dialoguer::Confirm::with_theme(
                            &dialoguer::theme::ColorfulTheme::default(),
                        )
                        .with_prompt("Include this change in the plan?")
                        .default(true)
                        .interact()?;
                    }
                    if include {
                        changed_files.push(path);
                    }
                }
            }
        }
    }
    Ok(())
}

async fn plan_groups(
    client: &KeycloakClient,
    workspace_dir: &Path,
    changes_only: bool,
    interactive: bool,
    env_vars: Arc<HashMap<String, String>>,
    changed_files: &mut Vec<PathBuf>,
) -> Result<()> {
    let groups_dir = workspace_dir.join("groups");
    if async_fs::try_exists(&groups_dir).await? {
        let existing_groups = client.get_groups().await?;
        let existing_groups_map: HashMap<String, GroupRepresentation> = existing_groups
            .into_iter()
            .filter_map(|g| g.get_identity().map(|id| (id, g)))
            .collect();

        let mut entries = async_fs::read_dir(&groups_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "yaml") {
                let content = async_fs::read_to_string(&path).await?;
                let mut val: serde_json::Value = serde_yaml::from_str(&content)
                    .with_context(|| format!("Failed to parse YAML file: {:?}", path))?;
                substitute_secrets(&mut val, &env_vars).map_err(|e| anyhow::anyhow!(e))?;
                let local_group: GroupRepresentation = serde_json::from_value(val)
                    .with_context(|| format!("Failed to deserialize YAML file: {:?}", path))?;

                let identity = local_group
                    .get_identity()
                    .context(format!("Failed to get identity for group in {:?}", path))?;
                let remote = existing_groups_map.get(&identity);

                let changed = if let Some(remote) = remote {
                    let mut remote_clone = remote.clone();
                    if local_group.id.is_none() {
                        remote_clone.id = None;
                    }
                    print_diff(
                        &format!("Group {}", local_group.get_name()),
                        Some(&remote_clone),
                        &local_group,
                        changes_only,
                        "group",
                    )?
                } else {
                    println!(
                        "\n{} Will create Group: {}",
                        Emoji("✨", ""),
                        local_group.get_name()
                    );
                    print_diff(
                        &format!("Group {}", local_group.get_name()),
                        None::<&GroupRepresentation>,
                        &local_group,
                        changes_only,
                        "group",
                    )?
                };

                if changed {
                    let mut include = true;
                    if interactive {
                        include = dialoguer::Confirm::with_theme(
                            &dialoguer::theme::ColorfulTheme::default(),
                        )
                        .with_prompt("Include this change in the plan?")
                        .default(true)
                        .interact()?;
                    }
                    if include {
                        changed_files.push(path);
                    }
                }
            }
        }
    }
    Ok(())
}

async fn plan_users(
    client: &KeycloakClient,
    workspace_dir: &Path,
    changes_only: bool,
    interactive: bool,
    env_vars: Arc<HashMap<String, String>>,
    changed_files: &mut Vec<PathBuf>,
) -> Result<()> {
    let users_dir = workspace_dir.join("users");
    if async_fs::try_exists(&users_dir).await? {
        let existing_users = client.get_users().await?;
        let existing_users_map: HashMap<String, UserRepresentation> = existing_users
            .into_iter()
            .filter_map(|u| u.get_identity().map(|id| (id, u)))
            .collect();

        let mut entries = async_fs::read_dir(&users_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "yaml") {
                let content = async_fs::read_to_string(&path).await?;
                let mut val: serde_json::Value = serde_yaml::from_str(&content)
                    .with_context(|| format!("Failed to parse YAML file: {:?}", path))?;
                substitute_secrets(&mut val, &env_vars).map_err(|e| anyhow::anyhow!(e))?;
                let local_user: UserRepresentation = serde_json::from_value(val)
                    .with_context(|| format!("Failed to deserialize YAML file: {:?}", path))?;

                let identity = local_user
                    .get_identity()
                    .context(format!("Failed to get identity for user in {:?}", path))?;
                let remote = existing_users_map.get(&identity);

                let changed = if let Some(remote) = remote {
                    let mut remote_clone = remote.clone();
                    if local_user.id.is_none() {
                        remote_clone.id = None;
                    }
                    print_diff(
                        &format!("User {}", local_user.get_name()),
                        Some(&remote_clone),
                        &local_user,
                        changes_only,
                        "user",
                    )?
                } else {
                    println!(
                        "\n{} Will create User: {}",
                        Emoji("✨", ""),
                        local_user.get_name()
                    );
                    print_diff(
                        &format!("User {}", local_user.get_name()),
                        None::<&UserRepresentation>,
                        &local_user,
                        changes_only,
                        "user",
                    )?
                };

                if changed {
                    let mut include = true;
                    if interactive {
                        include = dialoguer::Confirm::with_theme(
                            &dialoguer::theme::ColorfulTheme::default(),
                        )
                        .with_prompt("Include this change in the plan?")
                        .default(true)
                        .interact()?;
                    }
                    if include {
                        changed_files.push(path);
                    }
                }
            }
        }
    }
    Ok(())
}

async fn plan_authentication_flows(
    client: &KeycloakClient,
    workspace_dir: &Path,
    changes_only: bool,
    interactive: bool,
    env_vars: Arc<HashMap<String, String>>,
    changed_files: &mut Vec<PathBuf>,
) -> Result<()> {
    let flows_dir = workspace_dir.join("authentication-flows");
    if async_fs::try_exists(&flows_dir).await? {
        let existing_flows = client.get_authentication_flows().await?;
        let existing_flows_map: HashMap<String, AuthenticationFlowRepresentation> = existing_flows
            .into_iter()
            .filter_map(|f| f.get_identity().map(|id| (id, f)))
            .collect();

        let mut entries = async_fs::read_dir(&flows_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "yaml") {
                let content = async_fs::read_to_string(&path).await?;
                let mut val: serde_json::Value = serde_yaml::from_str(&content)
                    .with_context(|| format!("Failed to parse YAML file: {:?}", path))?;
                substitute_secrets(&mut val, &env_vars).map_err(|e| anyhow::anyhow!(e))?;
                let local_flow: AuthenticationFlowRepresentation = serde_json::from_value(val)
                    .with_context(|| format!("Failed to deserialize YAML file: {:?}", path))?;

                let identity = local_flow
                    .get_identity()
                    .context(format!("Failed to get identity for flow in {:?}", path))?;
                let remote = existing_flows_map.get(&identity);

                let changed = if let Some(remote) = remote {
                    let mut remote_clone = remote.clone();
                    if local_flow.id.is_none() {
                        remote_clone.id = None;
                    }
                    print_diff(
                        &format!("AuthenticationFlow {}", local_flow.get_name()),
                        Some(&remote_clone),
                        &local_flow,
                        changes_only,
                        "flow",
                    )?
                } else {
                    println!(
                        "\n{} Will create AuthenticationFlow: {}",
                        Emoji("✨", ""),
                        local_flow.get_name()
                    );
                    print_diff(
                        &format!("AuthenticationFlow {}", local_flow.get_name()),
                        None::<&AuthenticationFlowRepresentation>,
                        &local_flow,
                        changes_only,
                        "flow",
                    )?
                };

                if changed {
                    let mut include = true;
                    if interactive {
                        include = dialoguer::Confirm::with_theme(
                            &dialoguer::theme::ColorfulTheme::default(),
                        )
                        .with_prompt("Include this change in the plan?")
                        .default(true)
                        .interact()?;
                    }
                    if include {
                        changed_files.push(path);
                    }
                }
            }
        }
    }
    Ok(())
}

async fn plan_required_actions(
    client: &KeycloakClient,
    workspace_dir: &Path,
    changes_only: bool,
    interactive: bool,
    env_vars: Arc<HashMap<String, String>>,
    changed_files: &mut Vec<PathBuf>,
) -> Result<()> {
    let actions_dir = workspace_dir.join("required-actions");
    if async_fs::try_exists(&actions_dir).await? {
        let existing_actions = client.get_required_actions().await?;
        let existing_actions_map: HashMap<String, RequiredActionProviderRepresentation> =
            existing_actions
                .into_iter()
                .filter_map(|a| a.get_identity().map(|id| (id, a)))
                .collect();

        let mut entries = async_fs::read_dir(&actions_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "yaml") {
                let content = async_fs::read_to_string(&path).await?;
                let mut val: serde_json::Value = serde_yaml::from_str(&content)
                    .with_context(|| format!("Failed to parse YAML file: {:?}", path))?;
                substitute_secrets(&mut val, &env_vars).map_err(|e| anyhow::anyhow!(e))?;
                let local_action: RequiredActionProviderRepresentation =
                    serde_json::from_value(val)
                        .with_context(|| format!("Failed to deserialize YAML file: {:?}", path))?;

                let identity = local_action
                    .get_identity()
                    .context(format!("Failed to get identity for action in {:?}", path))?;
                let remote = existing_actions_map.get(&identity);

                let changed = if let Some(remote) = remote {
                    let remote_clone = remote.clone();
                    print_diff(
                        &format!("RequiredAction {}", local_action.get_name()),
                        Some(&remote_clone),
                        &local_action,
                        changes_only,
                        "action",
                    )?
                } else {
                    println!(
                        "\n{} Will create RequiredAction: {}",
                        Emoji("✨", ""),
                        local_action.get_name()
                    );
                    print_diff(
                        &format!("RequiredAction {}", local_action.get_name()),
                        None::<&RequiredActionProviderRepresentation>,
                        &local_action,
                        changes_only,
                        "action",
                    )?
                };

                if changed {
                    let mut include = true;
                    if interactive {
                        include = dialoguer::Confirm::with_theme(
                            &dialoguer::theme::ColorfulTheme::default(),
                        )
                        .with_prompt("Include this change in the plan?")
                        .default(true)
                        .interact()?;
                    }
                    if include {
                        changed_files.push(path);
                    }
                }
            }
        }
    }
    Ok(())
}

async fn plan_components_or_keys(
    client: &KeycloakClient,
    workspace_dir: &Path,
    changes_only: bool,
    interactive: bool,
    dir_name: &str,
    env_vars: Arc<HashMap<String, String>>,
    changed_files: &mut Vec<PathBuf>,
) -> Result<()> {
    let components_dir = workspace_dir.join(dir_name);
    if async_fs::try_exists(&components_dir).await? {
        let existing_components = client.get_components().await?;
        let mut by_identity: HashMap<String, ComponentRepresentation> = HashMap::new();
        type ComponentKey = (
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
        );
        let mut by_details: HashMap<ComponentKey, ComponentRepresentation> = HashMap::new();

        for c in existing_components {
            if let Some(id) = c.get_identity() {
                by_identity.insert(id, c.clone());
            }
            let key = (
                c.name.clone(),
                c.sub_type.clone(),
                c.provider_id.clone(),
                c.parent_id.clone(),
            );
            by_details.insert(key, c);
        }

        let mut entries = async_fs::read_dir(&components_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "yaml") {
                let content = async_fs::read_to_string(&path).await?;
                let mut val: serde_json::Value = serde_yaml::from_str(&content)
                    .with_context(|| format!("Failed to parse YAML file: {:?}", path))?;
                substitute_secrets(&mut val, &env_vars).map_err(|e| anyhow::anyhow!(e))?;
                let local_component: ComponentRepresentation = serde_json::from_value(val)
                    .with_context(|| format!("Failed to deserialize YAML file: {:?}", path))?;

                let remote = if let Some(identity) = local_component.get_identity() {
                    by_identity.get(&identity).or_else(|| {
                        let key = (
                            local_component.name.clone(),
                            local_component.sub_type.clone(),
                            local_component.provider_id.clone(),
                            local_component.parent_id.clone(),
                        );
                        by_details.get(&key)
                    })
                } else {
                    let key = (
                        local_component.name.clone(),
                        local_component.sub_type.clone(),
                        local_component.provider_id.clone(),
                        local_component.parent_id.clone(),
                    );
                    by_details.get(&key)
                };

                let changed = if let Some(remote) = remote {
                    let mut remote_clone = remote.clone();
                    if local_component.id.is_none() {
                        remote_clone.id = None;
                    }
                    let prefix = if dir_name == "keys" {
                        "key"
                    } else {
                        "component"
                    };
                    print_diff(
                        &format!("Component {}", local_component.get_name()),
                        Some(&remote_clone),
                        &local_component,
                        changes_only,
                        prefix,
                    )?
                } else {
                    println!(
                        "\n{} Will create Component: {}",
                        Emoji("✨", ""),
                        local_component.get_name()
                    );
                    let prefix = if dir_name == "keys" {
                        "key"
                    } else {
                        "component"
                    };
                    print_diff(
                        &format!("Component {}", local_component.get_name()),
                        None::<&ComponentRepresentation>,
                        &local_component,
                        changes_only,
                        prefix,
                    )?
                };

                if changed {
                    let mut include = true;
                    if interactive {
                        include = dialoguer::Confirm::with_theme(
                            &dialoguer::theme::ColorfulTheme::default(),
                        )
                        .with_prompt("Include this change in the plan?")
                        .default(true)
                        .interact()?;
                    }
                    if include {
                        changed_files.push(path);
                    }
                }
            }
        }
    }
    Ok(())
}

async fn plan_realm(
    client: &KeycloakClient,
    workspace_dir: &Path,
    changes_only: bool,
    interactive: bool,
    env_vars: Arc<HashMap<String, String>>,
    changed_files: &mut Vec<PathBuf>,
) -> Result<()> {
    let realm_path = workspace_dir.join("realm.yaml");
    if async_fs::try_exists(&realm_path).await? {
        let content = async_fs::read_to_string(&realm_path).await?;
        let mut val: serde_json::Value = serde_yaml::from_str(&content)
            .with_context(|| format!("Failed to parse YAML file: {:?}", realm_path))?;
        substitute_secrets(&mut val, &env_vars).map_err(|e| anyhow::anyhow!(e))?;
        let local_realm: RealmRepresentation = serde_json::from_value(val)
            .with_context(|| format!("Failed to deserialize YAML file: {:?}", realm_path))?;

        // We handle the case where remote realm fetch might fail (e.g. if we are creating it)
        // by treating it as None (creation). However, usually plan is run against existing realm.
        let remote_realm = match client.get_realm().await {
            Ok(r) => Some(r),
            Err(e) => {
                // Check if it's a 404 (Not Found)
                if e.to_string().contains("404") {
                    None
                } else {
                    return Err(e);
                }
            }
        };

        if print_diff(
            "Realm",
            remote_realm.as_ref(),
            &local_realm,
            changes_only,
            "realm",
        )? {
            let mut include = true;
            if interactive {
                include =
                    dialoguer::Confirm::with_theme(&dialoguer::theme::ColorfulTheme::default())
                        .with_prompt("Include this change in the plan?")
                        .default(true)
                        .interact()?;
            }
            if include {
                changed_files.push(realm_path);
            }
        }
    }
    Ok(())
}

async fn plan_roles(
    client: &KeycloakClient,
    workspace_dir: &Path,
    changes_only: bool,
    interactive: bool,
    env_vars: Arc<HashMap<String, String>>,
    changed_files: &mut Vec<PathBuf>,
) -> Result<()> {
    let roles_dir = workspace_dir.join("roles");
    if async_fs::try_exists(&roles_dir).await? {
        let existing_roles = client.get_roles().await?;
        let existing_roles_map: HashMap<String, RoleRepresentation> = existing_roles
            .into_iter()
            .filter_map(|r| r.get_identity().map(|id| (id, r)))
            .collect();

        let mut entries = async_fs::read_dir(&roles_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "yaml") {
                let content = async_fs::read_to_string(&path).await?;
                let mut val: serde_json::Value = serde_yaml::from_str(&content)
                    .with_context(|| format!("Failed to parse YAML file: {:?}", path))?;
                substitute_secrets(&mut val, &env_vars).map_err(|e| anyhow::anyhow!(e))?;
                let local_role: RoleRepresentation = serde_json::from_value(val)
                    .with_context(|| format!("Failed to deserialize YAML file: {:?}", path))?;

                let identity = local_role
                    .get_identity()
                    .context(format!("Failed to get identity for role in {:?}", path))?;
                let remote_role = existing_roles_map.get(&identity);

                let changed = if let Some(remote) = remote_role {
                    let mut remote_clone = remote.clone();
                    // Ignore ID differences if local doesn't specify it
                    if local_role.id.is_none() {
                        remote_clone.id = None;
                        remote_clone.container_id = None;
                    }
                    print_diff(
                        &format!("Role {}", local_role.get_name()),
                        Some(&remote_clone),
                        &local_role,
                        changes_only,
                        "role",
                    )?
                } else {
                    println!(
                        "\n{} Will create Role: {}",
                        Emoji("✨", ""),
                        local_role.get_name()
                    );
                    print_diff(
                        &format!("Role {}", local_role.get_name()),
                        None::<&RoleRepresentation>,
                        &local_role,
                        changes_only,
                        "role",
                    )?
                };

                if changed {
                    let mut include = true;
                    if interactive {
                        include = dialoguer::Confirm::with_theme(
                            &dialoguer::theme::ColorfulTheme::default(),
                        )
                        .with_prompt("Include this change in the plan?")
                        .default(true)
                        .interact()?;
                    }
                    if include {
                        changed_files.push(path);
                    }
                }
            }
        }
    }
    Ok(())
}

async fn plan_clients(
    client: &KeycloakClient,
    workspace_dir: &Path,
    changes_only: bool,
    interactive: bool,
    env_vars: Arc<HashMap<String, String>>,
    changed_files: &mut Vec<PathBuf>,
) -> Result<()> {
    let clients_dir = workspace_dir.join("clients");
    if async_fs::try_exists(&clients_dir).await? {
        let existing_clients = client.get_clients().await?;
        let existing_clients_map: HashMap<String, ClientRepresentation> = existing_clients
            .into_iter()
            .filter_map(|c| c.get_identity().map(|id| (id, c)))
            .collect();

        let mut entries = async_fs::read_dir(&clients_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "yaml") {
                let content = async_fs::read_to_string(&path).await?;
                let mut val: serde_json::Value = serde_yaml::from_str(&content)
                    .with_context(|| format!("Failed to parse YAML file: {:?}", path))?;
                substitute_secrets(&mut val, &env_vars).map_err(|e| anyhow::anyhow!(e))?;
                let local_client: ClientRepresentation = serde_json::from_value(val)
                    .with_context(|| format!("Failed to deserialize YAML file: {:?}", path))?;

                let identity = local_client
                    .get_identity()
                    .context(format!("Failed to get identity for client in {:?}", path))?;
                let remote = existing_clients_map.get(&identity);

                let changed = if let Some(remote) = remote {
                    let mut remote_clone = remote.clone();
                    if local_client.id.is_none() {
                        remote_clone.id = None;
                    }
                    print_diff(
                        &format!("Client {}", local_client.get_name()),
                        Some(&remote_clone),
                        &local_client,
                        changes_only,
                        "client",
                    )?
                } else {
                    println!(
                        "\n{} Will create Client: {}",
                        Emoji("✨", ""),
                        local_client.get_name()
                    );
                    print_diff(
                        &format!("Client {}", local_client.get_name()),
                        None::<&ClientRepresentation>,
                        &local_client,
                        changes_only,
                        "client",
                    )?
                };

                if changed {
                    let mut include = true;
                    if interactive {
                        include = dialoguer::Confirm::with_theme(
                            &dialoguer::theme::ColorfulTheme::default(),
                        )
                        .with_prompt("Include this change in the plan?")
                        .default(true)
                        .interact()?;
                    }
                    if include {
                        changed_files.push(path);
                    }
                }
            }
        }
    }
    Ok(())
}

async fn plan_identity_providers(
    client: &KeycloakClient,
    workspace_dir: &Path,
    changes_only: bool,
    interactive: bool,
    env_vars: Arc<HashMap<String, String>>,
    changed_files: &mut Vec<PathBuf>,
) -> Result<()> {
    let idps_dir = workspace_dir.join("identity-providers");
    if async_fs::try_exists(&idps_dir).await? {
        let existing_idps = client.get_identity_providers().await?;
        let existing_idps_map: HashMap<String, IdentityProviderRepresentation> = existing_idps
            .into_iter()
            .filter_map(|i| i.get_identity().map(|id| (id, i)))
            .collect();

        let mut entries = async_fs::read_dir(&idps_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "yaml") {
                let content = async_fs::read_to_string(&path).await?;
                let mut val: serde_json::Value = serde_yaml::from_str(&content)
                    .with_context(|| format!("Failed to parse YAML file: {:?}", path))?;
                substitute_secrets(&mut val, &env_vars).map_err(|e| anyhow::anyhow!(e))?;
                let local_idp: IdentityProviderRepresentation = serde_json::from_value(val)
                    .with_context(|| format!("Failed to deserialize YAML file: {:?}", path))?;

                let identity = local_idp
                    .get_identity()
                    .context(format!("Failed to get identity for IDP in {:?}", path))?;
                let remote = existing_idps_map.get(&identity);

                let changed = if let Some(remote) = remote {
                    let mut remote_clone = remote.clone();
                    if local_idp.internal_id.is_none() {
                        remote_clone.internal_id = None;
                    }
                    print_diff(
                        &format!("IdentityProvider {}", local_idp.get_name()),
                        Some(&remote_clone),
                        &local_idp,
                        changes_only,
                        "idp",
                    )?
                } else {
                    println!(
                        "\n{} Will create IdentityProvider: {}",
                        Emoji("✨", ""),
                        local_idp.get_name()
                    );
                    print_diff(
                        &format!("IdentityProvider {}", local_idp.get_name()),
                        None::<&IdentityProviderRepresentation>,
                        &local_idp,
                        changes_only,
                        "idp",
                    )?
                };

                if changed {
                    let mut include = true;
                    if interactive {
                        include = dialoguer::Confirm::with_theme(
                            &dialoguer::theme::ColorfulTheme::default(),
                        )
                        .with_prompt("Include this change in the plan?")
                        .default(true)
                        .interact()?;
                    }
                    if include {
                        changed_files.push(path);
                    }
                }
            }
        }
    }
    Ok(())
}

use std::time::{SystemTime, UNIX_EPOCH};

async fn check_keys_drift(client: &KeycloakClient, changes_only: bool) -> Result<()> {
    if !changes_only {
        return Ok(());
    }

    let keys_metadata = match client.get_keys().await {
        Ok(km) => km,
        Err(_) => return Ok(()), // Ignore if not available
    };

    if let Some(keys) = keys_metadata.keys {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;
        let thirty_days = 30 * 24 * 60 * 60 * 1000; // 30 days in ms

        for key in keys {
            #[allow(clippy::collapsible_if)]
            if key.status.as_deref() == Some("ACTIVE") {
                if let Some(valid_to) = key.valid_to {
                    #[allow(clippy::collapsible_if)]
                    if valid_to > 0 && valid_to - now < thirty_days {
                        let provider_id = key.provider_id.as_deref().unwrap_or("unknown");
                        println!(
                            "{} Warning: Active key (providerId: {}) is near expiration or expired! Consider rotating keys.",
                            Emoji("⚠️", ""),
                            style(provider_id).yellow()
                        );
                    }
                }
            }
        }
    }

    Ok(())
}
