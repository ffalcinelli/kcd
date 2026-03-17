use crate::client::KeycloakClient;
use crate::models::{
    AuthenticationFlowRepresentation, ClientRepresentation, ClientScopeRepresentation,
    ComponentRepresentation, GroupRepresentation, IdentityProviderRepresentation,
    RealmRepresentation, RequiredActionProviderRepresentation, RoleRepresentation,
    UserRepresentation,
};

use anyhow::Result;
use console::{Emoji, Style};
use serde::Serialize;
use similar::{ChangeTag, TextDiff};
use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs as async_fs;

pub async fn run(
    client: &KeycloakClient,
    input_dir: PathBuf,
    changes_only: bool,
    realms_to_plan: &[String],
) -> Result<()> {
    let env_vars = Arc::new(env::vars().collect::<HashMap<String, String>>());
    if !input_dir.exists() {
        anyhow::bail!("Input directory {:?} does not exist", input_dir);
    }

    let realms = if realms_to_plan.is_empty() {
        let mut dirs = Vec::new();
        let mut entries = async_fs::read_dir(&input_dir).await?;
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
        println!("No realms found to plan in {:?}", input_dir);
        return Ok(());
    }

    for realm_name in realms {
        let mut realm_client = client.clone();
        realm_client.set_target_realm(realm_name.clone());
        let realm_dir = input_dir.join(&realm_name);
        println!(
            "{} Planning changes for realm: {}",
            Emoji("🔮", ""),
            realm_name
        );
        plan_single_realm(
            &realm_client,
            realm_dir,
            changes_only,
            Arc::clone(&env_vars),
        )
        .await?;
    }
    Ok(())
}

async fn plan_single_realm(
    client: &KeycloakClient,
    input_dir: PathBuf,
    changes_only: bool,
    env_vars: Arc<HashMap<String, String>>,
) -> Result<()> {
    // 1. Plan Realm
    plan_realm(client, &input_dir, changes_only, Arc::clone(&env_vars)).await?;

    // 2. Plan Roles
    plan_roles(client, &input_dir, changes_only, Arc::clone(&env_vars)).await?;

    // 3. Plan Clients
    plan_clients(client, &input_dir, changes_only, Arc::clone(&env_vars)).await?;

    // 4. Plan Identity Providers
    plan_identity_providers(client, &input_dir, changes_only, Arc::clone(&env_vars)).await?;

    // 5. Plan Client Scopes
    plan_client_scopes(client, &input_dir, changes_only, Arc::clone(&env_vars)).await?;

    // 6. Plan Groups
    plan_groups(client, &input_dir, changes_only, Arc::clone(&env_vars)).await?;

    // 7. Plan Users
    plan_users(client, &input_dir, changes_only, Arc::clone(&env_vars)).await?;

    // 8. Plan Authentication Flows
    plan_authentication_flows(client, &input_dir, changes_only, Arc::clone(&env_vars)).await?;

    // 9. Plan Required Actions
    plan_required_actions(client, &input_dir, changes_only, Arc::clone(&env_vars)).await?;

    // 10. Plan Components
    plan_components_or_keys(
        client,
        &input_dir,
        changes_only,
        "components",
        Arc::clone(&env_vars),
    )
    .await?;
    plan_components_or_keys(
        client,
        &input_dir,
        changes_only,
        "keys",
        Arc::clone(&env_vars),
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
    env_vars: Arc<HashMap<String, String>>,
) -> Result<()> {
    let old_yaml = if let Some(o) = old {
        let mut val = serde_json::to_value(o)?;
        obfuscate_secrets(&mut val);
        crate::utils::to_sorted_yaml(&val)?
    } else {
        String::new()
    };

    let mut new_val = serde_json::to_value(new)?;
    substitute_secrets(&mut new_val, &env_vars).map_err(|e| anyhow::anyhow!(e))?;
    obfuscate_secrets(&mut new_val);
    let new_yaml = crate::utils::to_sorted_yaml(&new_val)?;

    let diff = TextDiff::from_lines(&old_yaml, &new_yaml);

    if diff.ratio() < 1.0 {
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
    Ok(())
}

async fn plan_client_scopes(
    client: &KeycloakClient,
    input_dir: &Path,
    changes_only: bool,
    env_vars: Arc<HashMap<String, String>>,
) -> Result<()> {
    let scopes_dir = input_dir.join("client-scopes");
    if async_fs::try_exists(&scopes_dir).await? {
        let existing_scopes = client.get_client_scopes().await?;
        let existing_scopes_map: HashMap<String, ClientScopeRepresentation> = existing_scopes
            .into_iter()
            .filter_map(|mut s| s.name.take().map(|n| (n, s)))
            .collect();

        let mut entries = async_fs::read_dir(&scopes_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "yaml") {
                let content = async_fs::read_to_string(&path).await?;
                let local_scope: ClientScopeRepresentation = serde_yaml::from_str(&content)?;
                let name = local_scope.name.as_deref().unwrap_or("");

                if name.is_empty() {
                    continue;
                }

                if let Some(remote) = existing_scopes_map.get(name) {
                    let mut remote_clone = remote.clone();
                    remote_clone.name = Some(name.to_string());
                    if local_scope.id.is_none() {
                        remote_clone.id = None;
                    }
                    print_diff(
                        &format!("ClientScope {}", name),
                        Some(&remote_clone),
                        &local_scope,
                        changes_only,
                        Arc::clone(&env_vars),
                    )?;
                } else {
                    println!("\n{} Will create ClientScope: {}", Emoji("✨", ""), name);
                    print_diff(
                        &format!("ClientScope {}", name),
                        None::<&ClientScopeRepresentation>,
                        &local_scope,
                        changes_only,
                        Arc::clone(&env_vars),
                    )?;
                }
            }
        }
    }
    Ok(())
}

async fn plan_groups(
    client: &KeycloakClient,
    input_dir: &Path,
    changes_only: bool,
    env_vars: Arc<HashMap<String, String>>,
) -> Result<()> {
    let groups_dir = input_dir.join("groups");
    if async_fs::try_exists(&groups_dir).await? {
        let existing_groups = client.get_groups().await?;
        let existing_groups_map: HashMap<String, GroupRepresentation> = existing_groups
            .into_iter()
            .filter_map(|mut g| g.name.take().map(|n| (n, g)))
            .collect();

        let mut entries = async_fs::read_dir(&groups_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "yaml") {
                let content = async_fs::read_to_string(&path).await?;
                let local_group: GroupRepresentation = serde_yaml::from_str(&content)?;
                let name = local_group.name.as_deref().unwrap_or("");

                if name.is_empty() {
                    continue;
                }

                if let Some(remote) = existing_groups_map.get(name) {
                    let mut remote_clone = remote.clone();
                    remote_clone.name = Some(name.to_string());
                    if local_group.id.is_none() {
                        remote_clone.id = None;
                    }
                    print_diff(
                        &format!("Group {}", name),
                        Some(&remote_clone),
                        &local_group,
                        changes_only,
                        Arc::clone(&env_vars),
                    )?;
                } else {
                    println!("\n{} Will create Group: {}", Emoji("✨", ""), name);
                    print_diff(
                        &format!("Group {}", name),
                        None::<&GroupRepresentation>,
                        &local_group,
                        changes_only,
                        Arc::clone(&env_vars),
                    )?;
                }
            }
        }
    }
    Ok(())
}

async fn plan_users(
    client: &KeycloakClient,
    input_dir: &Path,
    changes_only: bool,
    env_vars: Arc<HashMap<String, String>>,
) -> Result<()> {
    let users_dir = input_dir.join("users");
    if async_fs::try_exists(&users_dir).await? {
        let existing_users = client.get_users().await?;
        let existing_users_map: HashMap<String, UserRepresentation> = existing_users
            .into_iter()
            .filter_map(|mut u| u.username.take().map(|n| (n, u)))
            .collect();

        let mut entries = async_fs::read_dir(&users_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "yaml") {
                let content = async_fs::read_to_string(&path).await?;
                let local_user: UserRepresentation = serde_yaml::from_str(&content)?;
                let username = local_user.username.as_deref().unwrap_or("");

                if username.is_empty() {
                    continue;
                }

                if let Some(remote) = existing_users_map.get(username) {
                    let mut remote_clone = remote.clone();
                    remote_clone.username = Some(username.to_string());
                    if local_user.id.is_none() {
                        remote_clone.id = None;
                    }
                    print_diff(
                        &format!("User {}", username),
                        Some(&remote_clone),
                        &local_user,
                        changes_only,
                        Arc::clone(&env_vars),
                    )?;
                } else {
                    println!("\n{} Will create User: {}", Emoji("✨", ""), username);
                    print_diff(
                        &format!("User {}", username),
                        None::<&UserRepresentation>,
                        &local_user,
                        changes_only,
                        Arc::clone(&env_vars),
                    )?;
                }
            }
        }
    }
    Ok(())
}

async fn plan_authentication_flows(
    client: &KeycloakClient,
    input_dir: &Path,
    changes_only: bool,
    env_vars: Arc<HashMap<String, String>>,
) -> Result<()> {
    let flows_dir = input_dir.join("authentication-flows");
    if async_fs::try_exists(&flows_dir).await? {
        let existing_flows = client.get_authentication_flows().await?;
        let existing_flows_map: HashMap<String, AuthenticationFlowRepresentation> = existing_flows
            .into_iter()
            .filter_map(|mut f| f.alias.take().map(|a| (a, f)))
            .collect();

        let mut entries = async_fs::read_dir(&flows_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "yaml") {
                let content = async_fs::read_to_string(&path).await?;
                let local_flow: AuthenticationFlowRepresentation = serde_yaml::from_str(&content)?;
                let alias = local_flow.alias.as_deref().unwrap_or("");

                if alias.is_empty() {
                    continue;
                }

                if let Some(remote) = existing_flows_map.get(alias) {
                    let mut remote_clone = remote.clone();
                    remote_clone.alias = Some(alias.to_string());
                    if local_flow.id.is_none() {
                        remote_clone.id = None;
                    }
                    print_diff(
                        &format!("AuthenticationFlow {}", alias),
                        Some(&remote_clone),
                        &local_flow,
                        changes_only,
                        Arc::clone(&env_vars),
                    )?;
                } else {
                    println!(
                        "\n{} Will create AuthenticationFlow: {}",
                        Emoji("✨", ""),
                        alias
                    );
                    print_diff(
                        &format!("AuthenticationFlow {}", alias),
                        None::<&AuthenticationFlowRepresentation>,
                        &local_flow,
                        changes_only,
                        Arc::clone(&env_vars),
                    )?;
                }
            }
        }
    }
    Ok(())
}

async fn plan_required_actions(
    client: &KeycloakClient,
    input_dir: &Path,
    changes_only: bool,
    env_vars: Arc<HashMap<String, String>>,
) -> Result<()> {
    let actions_dir = input_dir.join("required-actions");
    if async_fs::try_exists(&actions_dir).await? {
        let existing_actions = client.get_required_actions().await?;
        let existing_actions_map: HashMap<String, RequiredActionProviderRepresentation> =
            existing_actions
                .into_iter()
                .filter_map(|mut a| a.alias.take().map(|n| (n, a)))
                .collect();

        let mut entries = async_fs::read_dir(&actions_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "yaml") {
                let content = async_fs::read_to_string(&path).await?;
                let local_action: RequiredActionProviderRepresentation =
                    serde_yaml::from_str(&content)?;
                let alias = local_action.alias.as_deref().unwrap_or("");

                if alias.is_empty() {
                    continue;
                }

                if let Some(remote) = existing_actions_map.get(alias) {
                    let mut remote_clone = remote.clone();
                    remote_clone.alias = Some(alias.to_string());
                    print_diff(
                        &format!("RequiredAction {}", alias),
                        Some(&remote_clone),
                        &local_action,
                        changes_only,
                        Arc::clone(&env_vars),
                    )?;
                } else {
                    println!(
                        "\n{} Will register RequiredAction: {}",
                        Emoji("✨", ""),
                        alias
                    );
                    print_diff(
                        &format!("RequiredAction {}", alias),
                        None::<&RequiredActionProviderRepresentation>,
                        &local_action,
                        changes_only,
                        Arc::clone(&env_vars),
                    )?;
                }
            }
        }
    }
    Ok(())
}

async fn plan_components_or_keys(
    client: &KeycloakClient,
    input_dir: &Path,
    changes_only: bool,
    dir_name: &str,
    env_vars: Arc<HashMap<String, String>>,
) -> Result<()> {
    let components_dir = input_dir.join(dir_name);
    if async_fs::try_exists(&components_dir).await? {
        let existing_components = client.get_components().await?;
        let existing_components_map: HashMap<String, ComponentRepresentation> = existing_components
            .into_iter()
            .filter_map(|mut c| c.name.take().map(|n| (n, c)))
            .collect();

        let mut entries = async_fs::read_dir(&components_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "yaml") {
                let content = async_fs::read_to_string(&path).await?;
                let local_component: ComponentRepresentation = serde_yaml::from_str(&content)?;
                let name = local_component.name.as_deref().unwrap_or("");

                if name.is_empty() {
                    continue;
                }

                if let Some(remote) = existing_components_map.get(name) {
                    let mut remote_clone = remote.clone();
                    remote_clone.name = Some(name.to_string());
                    if local_component.id.is_none() {
                        remote_clone.id = None;
                    }
                    print_diff(
                        &format!("Component {}", name),
                        Some(&remote_clone),
                        &local_component,
                        changes_only,
                        Arc::clone(&env_vars),
                    )?;
                } else {
                    println!("\n{} Will create Component: {}", Emoji("✨", ""), name);
                    print_diff(
                        &format!("Component {}", name),
                        None::<&ComponentRepresentation>,
                        &local_component,
                        changes_only,
                        Arc::clone(&env_vars),
                    )?;
                }
            }
        }
    }
    Ok(())
}

async fn plan_realm(
    client: &KeycloakClient,
    input_dir: &Path,
    changes_only: bool,
    env_vars: Arc<HashMap<String, String>>,
) -> Result<()> {
    let realm_path = input_dir.join("realm.yaml");
    if async_fs::try_exists(&realm_path).await? {
        let content = async_fs::read_to_string(&realm_path).await?;
        let local_realm: RealmRepresentation = serde_yaml::from_str(&content)?;

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

        print_diff(
            "Realm",
            remote_realm.as_ref(),
            &local_realm,
            changes_only,
            Arc::clone(&env_vars),
        )?;
    }
    Ok(())
}

async fn plan_roles(
    client: &KeycloakClient,
    input_dir: &Path,
    changes_only: bool,
    env_vars: Arc<HashMap<String, String>>,
) -> Result<()> {
    let roles_dir = input_dir.join("roles");
    if async_fs::try_exists(&roles_dir).await? {
        let existing_roles = client.get_roles().await?;
        let existing_roles_map: HashMap<String, RoleRepresentation> = existing_roles
            .into_iter()
            .map(|mut r| (std::mem::take(&mut r.name), r))
            .collect();

        let mut entries = async_fs::read_dir(&roles_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "yaml") {
                let content = async_fs::read_to_string(&path).await?;
                let local_role: RoleRepresentation = serde_yaml::from_str(&content)?;

                let remote_role = existing_roles_map.get(&local_role.name);

                if let Some(remote) = remote_role {
                    let mut remote_clone = remote.clone();
                    remote_clone.name = local_role.name.clone();
                    // Ignore ID differences if local doesn't specify it
                    if local_role.id.is_none() {
                        remote_clone.id = None;
                        remote_clone.container_id = None;
                    }
                    print_diff(
                        &format!("Role {}", local_role.name),
                        Some(&remote_clone),
                        &local_role,
                        changes_only,
                        Arc::clone(&env_vars),
                    )?;
                } else {
                    println!(
                        "\n{} Will create Role: {}",
                        Emoji("✨", ""),
                        local_role.name
                    );
                    print_diff(
                        &format!("Role {}", local_role.name),
                        None::<&RoleRepresentation>,
                        &local_role,
                        changes_only,
                        Arc::clone(&env_vars),
                    )?;
                }
            }
        }
    }
    Ok(())
}

async fn plan_clients(
    client: &KeycloakClient,
    input_dir: &Path,
    changes_only: bool,
    env_vars: Arc<HashMap<String, String>>,
) -> Result<()> {
    let clients_dir = input_dir.join("clients");
    if async_fs::try_exists(&clients_dir).await? {
        let existing_clients = client.get_clients().await?;
        let existing_clients_map: HashMap<String, ClientRepresentation> = existing_clients
            .into_iter()
            .filter_map(|mut c| c.client_id.take().map(|id| (id, c)))
            .collect();

        let mut entries = async_fs::read_dir(&clients_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "yaml") {
                let content = async_fs::read_to_string(&path).await?;
                let local_client: ClientRepresentation = serde_yaml::from_str(&content)?;
                let client_id = local_client.client_id.as_deref().unwrap_or("");

                if client_id.is_empty() {
                    continue;
                }

                if let Some(remote) = existing_clients_map.get(client_id) {
                    let mut remote_clone = remote.clone();
                    remote_clone.client_id = Some(client_id.to_string());
                    if local_client.id.is_none() {
                        remote_clone.id = None;
                    }
                    print_diff(
                        &format!("Client {}", client_id),
                        Some(&remote_clone),
                        &local_client,
                        changes_only,
                        Arc::clone(&env_vars),
                    )?;
                } else {
                    println!("\n{} Will create Client: {}", Emoji("✨", ""), client_id);
                    print_diff(
                        &format!("Client {}", client_id),
                        None::<&ClientRepresentation>,
                        &local_client,
                        changes_only,
                        Arc::clone(&env_vars),
                    )?;
                }
            }
        }
    }
    Ok(())
}

async fn plan_identity_providers(
    client: &KeycloakClient,
    input_dir: &Path,
    changes_only: bool,
    env_vars: Arc<HashMap<String, String>>,
) -> Result<()> {
    let idps_dir = input_dir.join("identity-providers");
    if async_fs::try_exists(&idps_dir).await? {
        let existing_idps = client.get_identity_providers().await?;
        let existing_idps_map: HashMap<String, IdentityProviderRepresentation> = existing_idps
            .into_iter()
            .filter_map(|mut i| i.alias.take().map(|alias| (alias, i)))
            .collect();

        let mut entries = async_fs::read_dir(&idps_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "yaml") {
                let content = async_fs::read_to_string(&path).await?;
                let local_idp: IdentityProviderRepresentation = serde_yaml::from_str(&content)?;
                let alias = local_idp.alias.as_deref().unwrap_or("");

                if alias.is_empty() {
                    continue;
                }

                if let Some(remote) = existing_idps_map.get(alias) {
                    let mut remote_clone = remote.clone();
                    remote_clone.alias = Some(alias.to_string());
                    if local_idp.internal_id.is_none() {
                        remote_clone.internal_id = None;
                    }
                    print_diff(
                        &format!("IdentityProvider {}", alias),
                        Some(&remote_clone),
                        &local_idp,
                        changes_only,
                        Arc::clone(&env_vars),
                    )?;
                } else {
                    println!(
                        "\n{} Will create IdentityProvider: {}",
                        Emoji("✨", ""),
                        alias
                    );
                    print_diff(
                        &format!("IdentityProvider {}", alias),
                        None::<&IdentityProviderRepresentation>,
                        &local_idp,
                        changes_only,
                        Arc::clone(&env_vars),
                    )?;
                }
            }
        }
    }
    Ok(())
}

use console::style;
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
