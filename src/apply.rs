use crate::client::KeycloakClient;
use crate::models::{
    AuthenticationFlowRepresentation, ClientRepresentation, ClientScopeRepresentation,
    ComponentRepresentation, GroupRepresentation, IdentityProviderRepresentation, KeycloakResource,
    RealmRepresentation, RequiredActionProviderRepresentation, RoleRepresentation,
    UserRepresentation,
};
use crate::utils::secrets::substitute_secrets;
use anyhow::{Context, Result};
use console::{style, Emoji};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs as async_fs;
use tokio::task::JoinSet;

static ACTION: Emoji<'_, '_> = Emoji("🚀 ", ">> ");
static SUCCESS_CREATE: Emoji<'_, '_> = Emoji("✨ ", "+ ");
static SUCCESS_UPDATE: Emoji<'_, '_> = Emoji("🔄 ", "~ ");
static WARN: Emoji<'_, '_> = Emoji("⚠️ ", "! ");

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
                let proceed = dialoguer::Confirm::with_theme(&dialoguer::theme::ColorfulTheme::default())
                    .with_prompt("No planned changes found. Send everything to Keycloak anyway?")
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
            let proceed = dialoguer::Confirm::with_theme(&dialoguer::theme::ColorfulTheme::default())
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
        println!("{} {}", WARN, style(format!("No realms found to apply in {:?}", workspace_dir)).yellow());
        return Ok(());
    }

    for realm_name in realms {
        println!("\n{} {}", ACTION, style(format!("Applying realm: {}", realm_name)).cyan().bold());
        let mut realm_client = client.clone();
        realm_client.set_target_realm(realm_name.clone());
        let realm_dir = workspace_dir.join(&realm_name);
        apply_single_realm(&realm_client, realm_dir, Arc::clone(&env_vars), Arc::clone(&planned_files)).await?;
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
    apply_realm(client, &workspace_dir, Arc::clone(&env_vars), Arc::clone(&planned_files)).await?;
    apply_roles(client, &workspace_dir, Arc::clone(&env_vars), Arc::clone(&planned_files)).await?;
    apply_identity_providers(client, &workspace_dir, Arc::clone(&env_vars), Arc::clone(&planned_files)).await?;
    apply_clients(client, &workspace_dir, Arc::clone(&env_vars), Arc::clone(&planned_files)).await?;
    apply_client_scopes(client, &workspace_dir, Arc::clone(&env_vars), Arc::clone(&planned_files)).await?;
    apply_groups(client, &workspace_dir, Arc::clone(&env_vars), Arc::clone(&planned_files)).await?;
    apply_users(client, &workspace_dir, Arc::clone(&env_vars), Arc::clone(&planned_files)).await?;
    apply_authentication_flows(client, &workspace_dir, Arc::clone(&env_vars), Arc::clone(&planned_files)).await?;
    apply_required_actions(client, &workspace_dir, Arc::clone(&env_vars), Arc::clone(&planned_files)).await?;
    apply_components_or_keys(client, &workspace_dir, "components", Arc::clone(&env_vars), Arc::clone(&planned_files)).await?;
    apply_components_or_keys(client, &workspace_dir, "keys", Arc::clone(&env_vars), Arc::clone(&planned_files)).await?;

    Ok(())
}

async fn apply_realm(
    client: &KeycloakClient,
    workspace_dir: &std::path::Path,
    env_vars: Arc<HashMap<String, String>>,
    planned_files: Arc<Option<HashSet<PathBuf>>>,
) -> Result<()> {
    // 1. Apply Realm
    let realm_path = workspace_dir.join("realm.yaml");
    if let Some(plan) = &*planned_files {
        if !plan.contains(&realm_path) {
            return Ok(());
        }
    }
    if async_fs::try_exists(&realm_path).await? {
        let content = async_fs::read_to_string(&realm_path).await?;
        let mut val: serde_json::Value = serde_yaml::from_str(&content)
            .with_context(|| format!("Failed to parse YAML file: {:?}", realm_path))?;
        substitute_secrets(&mut val, &env_vars).map_err(|e| anyhow::anyhow!(e))?;
        let realm_rep: RealmRepresentation = serde_json::from_value(val)?;
        client
            .update_realm(&realm_rep)
            .await
            .context("Failed to update realm")?;
        println!("  {} {}", SUCCESS_UPDATE, style("Updated realm configuration").cyan());
    }
    Ok(())
}

async fn apply_roles(
    client: &KeycloakClient,
    workspace_dir: &std::path::Path,
    env_vars: Arc<HashMap<String, String>>,
    planned_files: Arc<Option<HashSet<PathBuf>>>,
) -> Result<()> {
    // 2. Apply Roles
    let roles_dir = workspace_dir.join("roles");
    if async_fs::try_exists(&roles_dir).await? {
        let existing_roles = client.get_roles().await?;
        let existing_roles_map: HashMap<String, String> = existing_roles
            .into_iter()
            .filter_map(|r| {
                let identity = r.get_identity();
                let id = r.id.clone();
                match (identity, id) {
                    (Some(identity), Some(id)) => Some((identity, id)),
                    _ => None,
                }
            })
            .collect();
        let existing_roles_map = std::sync::Arc::new(existing_roles_map);

        let mut entries = async_fs::read_dir(&roles_dir).await?;
        let mut set = JoinSet::new();

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if let Some(plan) = &*planned_files {
                if !plan.contains(&path) {
                    continue;
                }
            }
            if path.extension().is_some_and(|ext| ext == "yaml") {
                let client = client.clone();
                let existing_roles_map = existing_roles_map.clone();
                let env_vars = Arc::clone(&env_vars);
                set.spawn(async move {
                    let content = async_fs::read_to_string(&path).await?;
                    let mut val: serde_json::Value = serde_yaml::from_str(&content)
                        .with_context(|| format!("Failed to parse YAML file: {:?}", path))?;
                    substitute_secrets(&mut val, &env_vars).map_err(|e| anyhow::anyhow!(e))?;
                    let mut role_rep: RoleRepresentation = serde_json::from_value(val)?;

                    let identity = role_rep
                        .get_identity()
                        .context(format!("Failed to get identity for role in {:?}", path))?;

                    if let Some(id) = existing_roles_map.get(&identity) {
                        role_rep.id = Some(id.clone()); // Use remote ID
                        client
                            .update_role(id, &role_rep)
                            .await
                            .context(format!("Failed to update role {}", role_rep.get_name()))?;
                        println!("  {} {}", SUCCESS_UPDATE, style(format!("Updated role {}", role_rep.get_name())).cyan());
                    } else {
                        role_rep.id = None; // Don't send ID on create
                        client
                            .create_role(&role_rep)
                            .await
                            .context(format!("Failed to create role {}", role_rep.get_name()))?;
                        println!("  {} {}", SUCCESS_CREATE, style(format!("Created role {}", role_rep.get_name())).green());
                    }
                    Ok::<(), anyhow::Error>(())
                });
            }
        }
        while let Some(res) = set.join_next().await {
            res??;
        }
    }
    Ok(())
}

async fn apply_identity_providers(
    client: &KeycloakClient,
    workspace_dir: &std::path::Path,
    env_vars: Arc<HashMap<String, String>>,
    planned_files: Arc<Option<HashSet<PathBuf>>>,
) -> Result<()> {
    // 4. Apply Identity Providers
    let idps_dir = workspace_dir.join("identity-providers");
    if async_fs::try_exists(&idps_dir).await? {
        let existing_idps = client.get_identity_providers().await?;
        let existing_idps_map: HashMap<String, IdentityProviderRepresentation> = existing_idps
            .into_iter()
            .filter_map(|i| i.get_identity().map(|id| (id, i)))
            .collect();
        let existing_idps_map = std::sync::Arc::new(existing_idps_map);

        let mut entries = async_fs::read_dir(&idps_dir).await?;
        let mut set = JoinSet::new();

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if let Some(plan) = &*planned_files {
                if !plan.contains(&path) {
                    continue;
                }
            }
            if path.extension().is_some_and(|ext| ext == "yaml") {
                let client = client.clone();
                let existing_idps_map = existing_idps_map.clone();
                let env_vars = Arc::clone(&env_vars);
                set.spawn(async move {
                    let content = async_fs::read_to_string(&path).await?;
                    let mut val: serde_json::Value = serde_yaml::from_str(&content)
                        .with_context(|| format!("Failed to parse YAML file: {:?}", path))?;
                    substitute_secrets(&mut val, &env_vars).map_err(|e| anyhow::anyhow!(e))?;
                    let mut idp_rep: IdentityProviderRepresentation = serde_json::from_value(val)?;

                    let identity = idp_rep
                        .get_identity()
                        .context(format!("Failed to get identity for IDP in {:?}", path))?;

                    if let Some(existing) = existing_idps_map.get(&identity) {
                        if let Some(internal_id) = &existing.internal_id {
                            idp_rep.internal_id = Some(internal_id.clone());
                            client
                                .update_identity_provider(&identity, &idp_rep)
                                .await
                                .context(format!("Failed to update identity provider {}", idp_rep.get_name()))?;
                            println!("  {} {}", SUCCESS_UPDATE, style(format!("Updated identity provider {}", idp_rep.get_name())).cyan());
                        }
                    } else {
                        idp_rep.internal_id = None;
                        client
                            .create_identity_provider(&idp_rep)
                            .await
                            .context(format!("Failed to create identity provider {}", idp_rep.get_name()))?;
                        println!("  {} {}", SUCCESS_CREATE, style(format!("Created identity provider {}", idp_rep.get_name())).green());
                    }
                    Ok::<(), anyhow::Error>(())
                });
            }
        }
        while let Some(res) = set.join_next().await {
            res??;
        }
    }
    Ok(())
}

async fn apply_clients(
    client: &KeycloakClient,
    workspace_dir: &std::path::Path,
    env_vars: Arc<HashMap<String, String>>,
    planned_files: Arc<Option<HashSet<PathBuf>>>,
) -> Result<()> {
    // 3. Apply Clients
    let clients_dir = workspace_dir.join("clients");
    if async_fs::try_exists(&clients_dir).await? {
        let existing_clients = client.get_clients().await?;
        let existing_clients_map: HashMap<String, ClientRepresentation> = existing_clients
            .into_iter()
            .filter_map(|c| c.get_identity().map(|id| (id, c)))
            .collect();
        let existing_clients_map = std::sync::Arc::new(existing_clients_map);

        let mut entries = async_fs::read_dir(&clients_dir).await?;
        let mut set = JoinSet::new();

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if let Some(plan) = &*planned_files {
                if !plan.contains(&path) {
                    continue;
                }
            }
            if path.extension().is_some_and(|ext| ext == "yaml") {
                let client = client.clone();
                let existing_clients_map = existing_clients_map.clone();
                let env_vars = Arc::clone(&env_vars);
                set.spawn(async move {
                    let content = async_fs::read_to_string(&path).await?;
                    let mut val: serde_json::Value = serde_yaml::from_str(&content)
                        .with_context(|| format!("Failed to parse YAML file: {:?}", path))?;
                    substitute_secrets(&mut val, &env_vars).map_err(|e| anyhow::anyhow!(e))?;
                    let mut client_rep: ClientRepresentation = serde_json::from_value(val)?;

                    let identity = client_rep
                        .get_identity()
                        .context(format!("Failed to get identity for client in {:?}", path))?;

                    if let Some(existing) = existing_clients_map.get(&identity) {
                        if let Some(id) = &existing.id {
                            client_rep.id = Some(id.clone()); // Use remote ID
                            client
                                .update_client(id, &client_rep)
                                .await
                                .context(format!("Failed to update client {}", client_rep.get_name()))?;
                            println!("  {} {}", SUCCESS_UPDATE, style(format!("Updated client {}", client_rep.get_name())).cyan());
                        }
                    } else {
                        client_rep.id = None; // Don't send ID on create
                        client
                            .create_client(&client_rep)
                            .await
                            .context(format!("Failed to create client {}", client_rep.get_name()))?;
                        println!("  {} {}", SUCCESS_CREATE, style(format!("Created client {}", client_rep.get_name())).green());
                    }
                    Ok::<(), anyhow::Error>(())
                });
            }
        }
        while let Some(res) = set.join_next().await {
            res??;
        }
    }
    Ok(())
}

async fn apply_client_scopes(
    client: &KeycloakClient,
    workspace_dir: &std::path::Path,
    env_vars: Arc<HashMap<String, String>>,
    planned_files: Arc<Option<HashSet<PathBuf>>>,
) -> Result<()> {
    // 5. Apply Client Scopes
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
            if let Some(plan) = &*planned_files {
                if !plan.contains(&path) {
                    continue;
                }
            }
            if path.extension().is_some_and(|ext| ext == "yaml") {
                let content = async_fs::read_to_string(&path).await?;
                let mut val: serde_json::Value = serde_yaml::from_str(&content)
                    .with_context(|| format!("Failed to parse YAML file: {:?}", path))?;
                substitute_secrets(&mut val, &env_vars).map_err(|e| anyhow::anyhow!(e))?;
                let mut scope_rep: ClientScopeRepresentation = serde_json::from_value(val)?;

                let identity = scope_rep
                    .get_identity()
                    .context(format!("Failed to get identity for client scope in {:?}", path))?;

                if let Some(existing) = existing_scopes_map.get(&identity) {
                    if let Some(id) = &existing.id {
                        scope_rep.id = Some(id.clone());
                        client
                            .update_client_scope(id, &scope_rep)
                            .await
                            .context(format!("Failed to update client scope {}", scope_rep.get_name()))?;
                        println!("  {} {}", SUCCESS_UPDATE, style(format!("Updated client scope {}", scope_rep.get_name())).cyan());
                    }
                } else {
                    scope_rep.id = None;
                    client
                        .create_client_scope(&scope_rep)
                        .await
                        .context(format!("Failed to create client scope {}", scope_rep.get_name()))?;
                    println!("  {} {}", SUCCESS_CREATE, style(format!("Created client scope {}", scope_rep.get_name())).green());
                }
            }
        }
    }
    Ok(())
}

async fn apply_groups(
    client: &KeycloakClient,
    workspace_dir: &std::path::Path,
    env_vars: Arc<HashMap<String, String>>,
    planned_files: Arc<Option<HashSet<PathBuf>>>,
) -> Result<()> {
    // 6. Apply Groups
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
            if let Some(plan) = &*planned_files {
                if !plan.contains(&path) {
                    continue;
                }
            }
            if path.extension().is_some_and(|ext| ext == "yaml") {
                let content = async_fs::read_to_string(&path).await?;
                let mut val: serde_json::Value = serde_yaml::from_str(&content)
                    .with_context(|| format!("Failed to parse YAML file: {:?}", path))?;
                substitute_secrets(&mut val, &env_vars).map_err(|e| anyhow::anyhow!(e))?;
                let mut group_rep: GroupRepresentation = serde_json::from_value(val)?;

                let identity = group_rep
                    .get_identity()
                    .context(format!("Failed to get identity for group in {:?}", path))?;

                if let Some(existing) = existing_groups_map.get(&identity) {
                    if let Some(id) = &existing.id {
                        group_rep.id = Some(id.clone());
                        client
                            .update_group(id, &group_rep)
                            .await
                            .context(format!("Failed to update group {}", group_rep.get_name()))?;
                        println!("  {} {}", SUCCESS_UPDATE, style(format!("Updated group {}", group_rep.get_name())).cyan());
                    }
                } else {
                    group_rep.id = None;
                    client
                        .create_group(&group_rep)
                        .await
                        .context(format!("Failed to create group {}", group_rep.get_name()))?;
                    println!("  {} {}", SUCCESS_CREATE, style(format!("Created group {}", group_rep.get_name())).green());
                }
            }
        }
    }
    Ok(())
}

async fn apply_users(
    client: &KeycloakClient,
    workspace_dir: &std::path::Path,
    env_vars: Arc<HashMap<String, String>>,
    planned_files: Arc<Option<HashSet<PathBuf>>>,
) -> Result<()> {
    // 7. Apply Users
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
            if let Some(plan) = &*planned_files {
                if !plan.contains(&path) {
                    continue;
                }
            }
            if path.extension().is_some_and(|ext| ext == "yaml") {
                let content = async_fs::read_to_string(&path).await?;
                let mut val: serde_json::Value = serde_yaml::from_str(&content)
                    .with_context(|| format!("Failed to parse YAML file: {:?}", path))?;
                substitute_secrets(&mut val, &env_vars).map_err(|e| anyhow::anyhow!(e))?;
                let mut user_rep: UserRepresentation = serde_json::from_value(val)?;

                let identity = user_rep
                    .get_identity()
                    .context(format!("Failed to get identity for user in {:?}", path))?;

                if let Some(existing) = existing_users_map.get(&identity) {
                    if let Some(id) = &existing.id {
                        user_rep.id = Some(id.clone());
                        client
                            .update_user(id, &user_rep)
                            .await
                            .context(format!("Failed to update user {}", user_rep.get_name()))?;
                        println!("  {} {}", SUCCESS_UPDATE, style(format!("Updated user {}", user_rep.get_name())).cyan());
                    }
                } else {
                    user_rep.id = None;
                    client
                        .create_user(&user_rep)
                        .await
                        .context(format!("Failed to create user {}", user_rep.get_name()))?;
                    println!("  {} {}", SUCCESS_CREATE, style(format!("Created user {}", user_rep.get_name())).green());
                }
            }
        }
    }
    Ok(())
}

async fn apply_authentication_flows(
    client: &KeycloakClient,
    workspace_dir: &std::path::Path,
    env_vars: Arc<HashMap<String, String>>,
    planned_files: Arc<Option<HashSet<PathBuf>>>,
) -> Result<()> {
    // 8. Apply Authentication Flows
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
            if let Some(plan) = &*planned_files {
                if !plan.contains(&path) {
                    continue;
                }
            }
            if path.extension().is_some_and(|ext| ext == "yaml") {
                let content = async_fs::read_to_string(&path).await?;
                let mut val: serde_json::Value = serde_yaml::from_str(&content)
                    .with_context(|| format!("Failed to parse YAML file: {:?}", path))?;
                substitute_secrets(&mut val, &env_vars).map_err(|e| anyhow::anyhow!(e))?;
                let mut flow_rep: AuthenticationFlowRepresentation = serde_json::from_value(val)?;

                let identity = flow_rep
                    .get_identity()
                    .context(format!("Failed to get identity for flow in {:?}", path))?;

                if let Some(existing) = existing_flows_map.get(&identity) {
                    if let Some(id) = &existing.id {
                        flow_rep.id = Some(id.clone());
                        client
                            .update_authentication_flow(id, &flow_rep)
                            .await
                            .context(format!("Failed to update authentication flow {}", flow_rep.get_name()))?;
                        println!("  {} {}", SUCCESS_UPDATE, style(format!("Updated authentication flow {}", flow_rep.get_name())).cyan());
                    }
                } else {
                    flow_rep.id = None;
                    client
                        .create_authentication_flow(&flow_rep)
                        .await
                        .context(format!("Failed to create authentication flow {}", flow_rep.get_name()))?;
                    println!("  {} {}", SUCCESS_CREATE, style(format!("Created authentication flow {}", flow_rep.get_name())).green());
                }
            }
        }
    }
    Ok(())
}

async fn apply_required_actions(
    client: &KeycloakClient,
    workspace_dir: &std::path::Path,
    env_vars: Arc<HashMap<String, String>>,
    planned_files: Arc<Option<HashSet<PathBuf>>>,
) -> Result<()> {
    // 9. Apply Required Actions
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
            if let Some(plan) = &*planned_files {
                if !plan.contains(&path) {
                    continue;
                }
            }
            if path.extension().is_some_and(|ext| ext == "yaml") {
                let content = async_fs::read_to_string(&path).await?;
                let mut val: serde_json::Value = serde_yaml::from_str(&content)
                    .with_context(|| format!("Failed to parse YAML file: {:?}", path))?;
                substitute_secrets(&mut val, &env_vars).map_err(|e| anyhow::anyhow!(e))?;
                let action_rep: RequiredActionProviderRepresentation = serde_json::from_value(val)?;

                let identity = action_rep
                    .get_identity()
                    .context(format!("Failed to get identity for required action in {:?}", path))?;

                if existing_actions_map.contains_key(&identity) {
                    client
                        .update_required_action(&identity, &action_rep)
                        .await
                        .context(format!("Failed to update required action {}", action_rep.get_name()))?;
                    println!("  {} {}", SUCCESS_UPDATE, style(format!("Updated required action {}", action_rep.get_name())).cyan());
                } else {
                    // Register
                    client
                        .register_required_action(&action_rep)
                        .await
                        .context(format!("Failed to register required action {}", action_rep.get_name()))?;
                    client
                        .update_required_action(&identity, &action_rep)
                        .await
                        .context(format!(
                            "Failed to configure registered required action {}",
                            action_rep.get_name()
                        ))?;
                    println!("  {} {}", SUCCESS_CREATE, style(format!("Registered required action {}", action_rep.get_name())).green());
                }
            }
        }
    }
    Ok(())
}

async fn apply_components_or_keys(
    client: &KeycloakClient,
    workspace_dir: &std::path::Path,
    dir_name: &str,
    env_vars: Arc<HashMap<String, String>>,
    planned_files: Arc<Option<HashSet<PathBuf>>>,
) -> Result<()> {
    let components_dir = workspace_dir.join(dir_name);
    if async_fs::try_exists(&components_dir).await? {
        let existing_components = client.get_components().await?;
        let mut by_identity: HashMap<String, ComponentRepresentation> = HashMap::new();
        let mut by_details: HashMap<(Option<String>, Option<String>, Option<String>, Option<String>), ComponentRepresentation> =
            HashMap::new();

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
            if let Some(plan) = &*planned_files {
                if !plan.contains(&path) {
                    continue;
                }
            }
            if path.extension().is_some_and(|ext| ext == "yaml") {
                let content = async_fs::read_to_string(&path).await?;
                let mut val: serde_json::Value = serde_yaml::from_str(&content)
                    .with_context(|| format!("Failed to parse YAML file: {:?}", path))?;
                substitute_secrets(&mut val, &env_vars).map_err(|e| anyhow::anyhow!(e))?;
                let mut component_rep: ComponentRepresentation = serde_json::from_value(val)?;

                let existing = if let Some(identity) = component_rep.get_identity() {
                    by_identity.get(&identity).or_else(|| {
                        let key = (
                            component_rep.name.clone(),
                            component_rep.sub_type.clone(),
                            component_rep.provider_id.clone(),
                            component_rep.parent_id.clone(),
                        );
                        by_details.get(&key)
                    })
                } else {
                    let key = (
                        component_rep.name.clone(),
                        component_rep.sub_type.clone(),
                        component_rep.provider_id.clone(),
                        component_rep.parent_id.clone(),
                    );
                    by_details.get(&key)
                };

                if let Some(existing) = existing {
                    if let Some(id) = &existing.id {
                        component_rep.id = Some(id.clone());
                        client
                            .update_component(id, &component_rep)
                            .await
                            .context(format!("Failed to update component {}", component_rep.get_name()))?;
                        println!("  {} {}", SUCCESS_UPDATE, style(format!("Updated component {}", component_rep.get_name())).cyan());
                    }
                } else {
                    component_rep.id = None;
                    client
                        .create_component(&component_rep)
                        .await
                        .context(format!("Failed to create component {}", component_rep.get_name()))?;
                    println!("  {} {}", SUCCESS_CREATE, style(format!("Created component {}", component_rep.get_name())).green());
                }
            }
        }
    }
    Ok(())
}
