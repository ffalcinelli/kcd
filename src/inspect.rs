use crate::client::KeycloakClient;
use crate::utils::to_sorted_yaml_with_secrets;
use anyhow::{Context, Result};
use sanitize_filename::sanitize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;
use tokio::sync::Mutex;

pub async fn run(
    client: &KeycloakClient,
    output_dir: PathBuf,
    realms_to_inspect: &[String],
) -> Result<()> {
    if !fs::try_exists(&output_dir)
        .await
        .context("Failed to check output directory")?
    {
        fs::create_dir_all(&output_dir)
            .await
            .context("Failed to create output directory")?;
    }

    let realms = if realms_to_inspect.is_empty() {
        let all_realms = client
            .get_realms()
            .await
            .context("Failed to fetch realms")?;
        all_realms.into_iter().map(|r| r.realm).collect()
    } else {
        realms_to_inspect.to_vec()
    };

    let all_secrets = Arc::new(Mutex::new(HashMap::new()));

    for realm_name in realms {
        let mut realm_client = client.clone();
        realm_client.set_target_realm(realm_name.clone());
        let realm_dir = output_dir.join(&realm_name);
        println!("Inspecting realm: {}", realm_name);
        inspect_realm(&realm_client, realm_dir, Arc::clone(&all_secrets)).await?;
    }

    let secrets_lock = all_secrets.lock().await;
    if !secrets_lock.is_empty() {
        let env_path = output_dir.join(".env");
        let mut env_content = String::new();
        let mut keys: Vec<&String> = secrets_lock.keys().collect();
        keys.sort();
        for key in keys {
            env_content.push_str(&format!("{}={}\n", key, secrets_lock[key]));
        }

        let mut existing_env = String::new();
        if fs::try_exists(&env_path).await.unwrap_or(false) {
            #[allow(clippy::collapsible_if)]
            if let Ok(content) = fs::read_to_string(&env_path).await {
                existing_env = content;
                if !existing_env.ends_with('\n') && !existing_env.is_empty() {
                    existing_env.push('\n');
                }
            }
        }

        fs::write(&env_path, format!("{}{}", existing_env, env_content))
            .await
            .context("Failed to write .env file")?;
        println!("Exported secrets to .env");
    }

    Ok(())
}

async fn inspect_realm(
    client: &KeycloakClient,
    output_dir: PathBuf,
    all_secrets: Arc<Mutex<HashMap<String, String>>>,
) -> Result<()> {
    if !fs::try_exists(&output_dir)
        .await
        .context("Failed to check output directory")?
    {
        fs::create_dir_all(&output_dir)
            .await
            .context("Failed to create output directory")?;
    }

    // Fetch realm
    let realm = client.get_realm().await.context("Failed to fetch realm")?;
    let mut local_secrets = HashMap::new();
    let realm_yaml = to_sorted_yaml_with_secrets(&realm, "realm", &mut local_secrets)
        .context("Failed to serialize realm")?;
    all_secrets.lock().await.extend(local_secrets);
    fs::write(output_dir.join("realm.yaml"), realm_yaml)
        .await
        .context("Failed to write realm.yaml")?;
    println!("Exported realm configuration to realm.yaml");

    // Fetch clients
    let clients = client
        .get_clients()
        .await
        .context("Failed to fetch clients")?;
    let clients_dir = output_dir.join("clients");
    if !fs::try_exists(&clients_dir)
        .await
        .context("Failed to check clients directory")?
    {
        fs::create_dir_all(&clients_dir)
            .await
            .context("Failed to create clients directory")?;
    }
    let mut set = tokio::task::JoinSet::new();
    for client_rep in clients {
        let clients_dir = clients_dir.clone();
        let all_secrets = Arc::clone(&all_secrets);
        set.spawn(async move {
            let name = client_rep
                .client_id
                .as_deref()
                .unwrap_or("unknown")
                .to_string();
            let filename = format!("{}.yaml", sanitize(&name));
            let path = clients_dir.join(filename);
            let mut local_secrets = HashMap::new();
            let yaml = to_sorted_yaml_with_secrets(&client_rep, "client", &mut local_secrets)
                .context("Failed to serialize client")?;
            all_secrets.lock().await.extend(local_secrets);
            fs::write(&path, yaml)
                .await
                .context(format!("Failed to write client {}", name))
        });
    }
    while let Some(res) = set.join_next().await {
        res.context("Task panicked")??;
    }
    println!("Exported clients to clients/");

    // Fetch roles
    let roles = client.get_roles().await.context("Failed to fetch roles")?;
    let roles_dir = output_dir.join("roles");
    if !fs::try_exists(&roles_dir)
        .await
        .context("Failed to check roles directory")?
    {
        fs::create_dir_all(&roles_dir)
            .await
            .context("Failed to create roles directory")?;
    }
    let mut set = tokio::task::JoinSet::new();
    for role in roles {
        let roles_dir = roles_dir.clone();
        let all_secrets = Arc::clone(&all_secrets);
        set.spawn(async move {
            let name = &role.name;
            let filename = format!("{}.yaml", sanitize(name));
            let path = roles_dir.join(filename);
            let mut local_secrets = HashMap::new();
            let yaml = to_sorted_yaml_with_secrets(&role, "role", &mut local_secrets)
                .context("Failed to serialize role")?;
            all_secrets.lock().await.extend(local_secrets);
            fs::write(&path, yaml)
                .await
                .context(format!("Failed to write role {}", name))
        });
    }
    while let Some(res) = set.join_next().await {
        res.context("Task panicked")??;
    }
    println!("Exported roles to roles/");

    // Fetch client scopes
    let client_scopes = client
        .get_client_scopes()
        .await
        .context("Failed to fetch client scopes")?;
    let scopes_dir = output_dir.join("client-scopes");
    if !fs::try_exists(&scopes_dir)
        .await
        .context("Failed to check client-scopes directory")?
    {
        fs::create_dir_all(&scopes_dir)
            .await
            .context("Failed to create client-scopes directory")?;
    }
    let mut set = tokio::task::JoinSet::new();
    for scope in client_scopes {
        let scopes_dir = scopes_dir.clone();
        let all_secrets = Arc::clone(&all_secrets);
        set.spawn(async move {
            let name = scope.name.as_deref().unwrap_or("unknown").to_string();
            let filename = format!("{}.yaml", sanitize(&name));
            let path = scopes_dir.join(filename);
            let mut local_secrets = HashMap::new();
            let yaml = to_sorted_yaml_with_secrets(&scope, "client_scope", &mut local_secrets)
                .context("Failed to serialize client scope")?;
            all_secrets.lock().await.extend(local_secrets);
            fs::write(&path, yaml)
                .await
                .context(format!("Failed to write client scope {}", name))
        });
    }
    while let Some(res) = set.join_next().await {
        res.context("Task panicked")??;
    }
    println!("Exported client scopes to client-scopes/");

    // Fetch groups
    let groups = client
        .get_groups()
        .await
        .context("Failed to fetch groups")?;
    let groups_dir = output_dir.join("groups");
    if !fs::try_exists(&groups_dir)
        .await
        .context("Failed to check groups directory")?
    {
        fs::create_dir_all(&groups_dir)
            .await
            .context("Failed to create groups directory")?;
    }
    let mut set = tokio::task::JoinSet::new();
    for group in groups {
        let groups_dir = groups_dir.clone();
        let all_secrets = Arc::clone(&all_secrets);
        set.spawn(async move {
            let name = group.name.as_deref().unwrap_or("unknown").to_string();
            let filename = format!("{}.yaml", sanitize(&name));
            let path = groups_dir.join(filename);
            let mut local_secrets = HashMap::new();
            let yaml = to_sorted_yaml_with_secrets(&group, "group", &mut local_secrets)
                .context("Failed to serialize group")?;
            all_secrets.lock().await.extend(local_secrets);
            fs::write(&path, yaml)
                .await
                .context(format!("Failed to write group {}", name))
        });
    }
    while let Some(res) = set.join_next().await {
        res.context("Task panicked")??;
    }
    println!("Exported groups to groups/");

    // Fetch users
    let users = client.get_users().await.context("Failed to fetch users")?;
    let users_dir = output_dir.join("users");
    if !fs::try_exists(&users_dir)
        .await
        .context("Failed to check users directory")?
    {
        fs::create_dir_all(&users_dir)
            .await
            .context("Failed to create users directory")?;
    }
    let mut set = tokio::task::JoinSet::new();
    for user in users {
        let users_dir = users_dir.clone();
        let all_secrets = Arc::clone(&all_secrets);
        set.spawn(async move {
            let username = user.username.as_deref().unwrap_or("unknown").to_string();
            let filename = format!("{}.yaml", sanitize(&username));
            let path = users_dir.join(filename);
            let mut local_secrets = HashMap::new();
            let yaml = to_sorted_yaml_with_secrets(&user, "user", &mut local_secrets)
                .context("Failed to serialize user")?;
            all_secrets.lock().await.extend(local_secrets);
            fs::write(&path, yaml)
                .await
                .context(format!("Failed to write user {}", username))
        });
    }
    while let Some(res) = set.join_next().await {
        res.context("Task panicked")??;
    }
    println!("Exported users to users/");

    // Fetch authentication flows
    let flows = client
        .get_authentication_flows()
        .await
        .context("Failed to fetch authentication flows")?;
    let flows_dir = output_dir.join("authentication-flows");
    if !fs::try_exists(&flows_dir)
        .await
        .context("Failed to check authentication-flows directory")?
    {
        fs::create_dir_all(&flows_dir)
            .await
            .context("Failed to create authentication-flows directory")?;
    }
    let mut set = tokio::task::JoinSet::new();
    for flow in flows {
        let flows_dir = flows_dir.clone();
        let all_secrets = Arc::clone(&all_secrets);
        set.spawn(async move {
            let alias = flow.alias.as_deref().unwrap_or("unknown").to_string();
            let filename = format!("{}.yaml", sanitize(&alias));
            let path = flows_dir.join(filename);
            let mut local_secrets = HashMap::new();
            let yaml = to_sorted_yaml_with_secrets(&flow, "flow", &mut local_secrets)
                .context("Failed to serialize authentication flow")?;
            all_secrets.lock().await.extend(local_secrets);
            fs::write(&path, yaml)
                .await
                .context(format!("Failed to write authentication flow {}", alias))
        });
    }
    while let Some(res) = set.join_next().await {
        res.context("Task panicked")??;
    }
    println!("Exported authentication flows to authentication-flows/");

    // Fetch required actions
    let actions = client
        .get_required_actions()
        .await
        .context("Failed to fetch required actions")?;
    let actions_dir = output_dir.join("required-actions");
    if !fs::try_exists(&actions_dir)
        .await
        .context("Failed to check required-actions directory")?
    {
        fs::create_dir_all(&actions_dir)
            .await
            .context("Failed to create required-actions directory")?;
    }
    let mut set = tokio::task::JoinSet::new();
    for action in actions {
        let actions_dir = actions_dir.clone();
        let all_secrets = Arc::clone(&all_secrets);
        set.spawn(async move {
            let alias = action.alias.as_deref().unwrap_or("unknown").to_string();
            let filename = format!("{}.yaml", sanitize(&alias));
            let path = actions_dir.join(filename);
            let mut local_secrets = HashMap::new();
            let yaml = to_sorted_yaml_with_secrets(&action, "action", &mut local_secrets)
                .context("Failed to serialize required action")?;
            all_secrets.lock().await.extend(local_secrets);
            fs::write(&path, yaml)
                .await
                .context(format!("Failed to write required action {}", alias))
        });
    }
    while let Some(res) = set.join_next().await {
        res.context("Task panicked")??;
    }
    println!("Exported required actions to required-actions/");

    // Fetch components and keys
    let all_components = client
        .get_components()
        .await
        .context("Failed to fetch components")?;

    let components_dir = output_dir.join("components");
    if !fs::try_exists(&components_dir)
        .await
        .context("Failed to check components directory")?
    {
        fs::create_dir_all(&components_dir)
            .await
            .context("Failed to create components directory")?;
    }

    let keys_dir = output_dir.join("keys");
    if !fs::try_exists(&keys_dir)
        .await
        .context("Failed to check keys directory")?
    {
        fs::create_dir_all(&keys_dir)
            .await
            .context("Failed to create keys directory")?;
    }

    let mut set = tokio::task::JoinSet::new();
    for component in all_components {
        let is_key = component
            .provider_type
            .as_deref()
            .is_some_and(|pt| pt == "org.keycloak.keys.KeyProvider");
        let target_dir = if is_key {
            keys_dir.clone()
        } else {
            components_dir.clone()
        };

        let all_secrets = Arc::clone(&all_secrets);
        set.spawn(async move {
            let name = component.name.as_deref().unwrap_or("unknown").to_string();
            let filename = format!("{}.yaml", sanitize(&name));
            let path = target_dir.join(filename);
            let mut local_secrets = HashMap::new();
            let prefix = if is_key { "key" } else { "component" };
            let yaml = to_sorted_yaml_with_secrets(&component, prefix, &mut local_secrets)
                .context("Failed to serialize component")?;
            all_secrets.lock().await.extend(local_secrets);
            fs::write(&path, yaml)
                .await
                .context(format!("Failed to write component {}", name))
        });
    }
    while let Some(res) = set.join_next().await {
        res.context("Task panicked")??;
    }
    println!("Exported components to components/ and keys to keys/");

    Ok(())
}
