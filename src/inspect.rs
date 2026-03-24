use crate::client::KeycloakClient;
use crate::models::KeycloakResource;
use crate::utils::to_sorted_yaml_with_secrets;
use anyhow::{Context, Result};
use console::{Emoji, style};
use dialoguer::{Confirm, theme::ColorfulTheme};
use sanitize_filename::sanitize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;
use tokio::sync::Mutex;

static ACTION: Emoji<'_, '_> = Emoji("🔍 ", "> ");
static SUCCESS: Emoji<'_, '_> = Emoji("✅ ", "√ ");
static WARN: Emoji<'_, '_> = Emoji("⚠️ ", "! ");

pub async fn run(
    client: &KeycloakClient,
    workspace_dir: PathBuf,
    realms_to_inspect: &[String],
    yes: bool,
) -> Result<()> {
    if !fs::try_exists(&workspace_dir)
        .await
        .context("Failed to check output directory")?
    {
        fs::create_dir_all(&workspace_dir)
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
    let prompt_mutex = Arc::new(Mutex::new(()));

    for realm_name in realms {
        let mut realm_client = client.clone();
        realm_client.set_target_realm(realm_name.clone());
        let realm_dir = workspace_dir.join(&realm_name);
        println!(
            "\n{} {}",
            ACTION,
            style(format!("Inspecting realm: {}", realm_name))
                .cyan()
                .bold()
        );
        inspect_realm(
            &realm_client,
            &realm_name,
            realm_dir,
            Arc::clone(&all_secrets),
            yes,
            Arc::clone(&prompt_mutex),
        )
        .await?;
    }

    let secrets_lock = all_secrets.lock().await;
    if !secrets_lock.is_empty() {
        let env_path = workspace_dir.join(".secrets");
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

        let new_content = format!("{}{}", existing_env, env_content);
        write_if_changed_with_mutex(&env_path, &new_content, yes, Arc::clone(&prompt_mutex))
            .await?;
        println!(
            "{} {}",
            SUCCESS,
            style("Exported secrets to .secrets").green()
        );
    }

    Ok(())
}

async fn write_if_changed_with_mutex(
    path: &Path,
    content: &str,
    yes: bool,
    prompt_mutex: Arc<Mutex<()>>,
) -> Result<()> {
    if fs::try_exists(path).await.unwrap_or(false) {
        let existing = fs::read_to_string(path).await.unwrap_or_default();
        if existing == content {
            return Ok(());
        }

        if !yes {
            let _lock = prompt_mutex.lock().await;
            if !Confirm::with_theme(&ColorfulTheme::default())
                .with_prompt(format!(
                    "File {:?} already exists with different content. Overwrite?",
                    path
                ))
                .default(false)
                .interact()?
            {
                println!(
                    "{} {}",
                    WARN,
                    style(format!("Skipping {:?}", path)).yellow()
                );
                return Ok(());
            }
        }
    }
    fs::write(path, content)
        .await
        .context(format!("Failed to write {:?}", path))?;
    Ok(())
}

async fn inspect_realm(
    client: &KeycloakClient,
    realm_name: &str,
    workspace_dir: PathBuf,
    all_secrets: Arc<Mutex<HashMap<String, String>>>,
    yes: bool,
    prompt_mutex: Arc<Mutex<()>>,
) -> Result<()> {
    if !fs::try_exists(&workspace_dir)
        .await
        .context("Failed to check output directory")?
    {
        fs::create_dir_all(&workspace_dir)
            .await
            .context("Failed to create output directory")?;
    }

    let mut master_set = tokio::task::JoinSet::new();

    // Fetch realm configuration
    {
        let client = client.clone();
        let realm_name = realm_name.to_string();
        let workspace_dir = workspace_dir.clone();
        let all_secrets = Arc::clone(&all_secrets);
        let prompt_mutex = Arc::clone(&prompt_mutex);
        master_set.spawn(async move {
            let realm = client.get_realm().await.context("Failed to fetch realm")?;
            let mut local_secrets = HashMap::new();
            let realm_prefix = format!("realm_{}", realm_name);
            let realm_yaml = to_sorted_yaml_with_secrets(&realm, &realm_prefix, &mut local_secrets)
                .context("Failed to serialize realm")?;
            all_secrets.lock().await.extend(local_secrets);

            let realm_path = workspace_dir.join("realm.yaml");
            write_if_changed_with_mutex(&realm_path, &realm_yaml, yes, prompt_mutex).await?;
            println!(
                "  {} {}",
                SUCCESS,
                style("Exported realm configuration to realm.yaml").green()
            );
            Ok::<(), anyhow::Error>(())
        });
    }

    // Fetch clients
    {
        let client = client.clone();
        let realm_name = realm_name.to_string();
        let workspace_dir = workspace_dir.clone();
        let all_secrets = Arc::clone(&all_secrets);
        let prompt_mutex = Arc::clone(&prompt_mutex);
        master_set.spawn(async move {
            let clients = client
                .get_clients()
                .await
                .context("Failed to fetch clients")?;
            let clients_dir = workspace_dir.join("clients");
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
                let realm_name = realm_name.clone();
                let prompt_mutex = Arc::clone(&prompt_mutex);
                set.spawn(async move {
                    let name = client_rep.get_name();
                    let filename = format!("{}.yaml", sanitize(&name));
                    let path = clients_dir.join(filename);
                    let mut local_secrets = HashMap::new();
                    let prefix = format!("realm_{}_client", realm_name);
                    let yaml =
                        to_sorted_yaml_with_secrets(&client_rep, &prefix, &mut local_secrets)
                            .context("Failed to serialize client")?;
                    all_secrets.lock().await.extend(local_secrets);
                    write_if_changed_with_mutex(&path, &yaml, yes, prompt_mutex).await
                });
            }
            while let Some(res) = set.join_next().await {
                res.context("Task panicked")??;
            }
            println!(
                "  {} {}",
                SUCCESS,
                style("Exported clients to clients/").green()
            );
            Ok(())
        });
    }

    // Fetch roles
    {
        let client = client.clone();
        let realm_name = realm_name.to_string();
        let workspace_dir = workspace_dir.clone();
        let all_secrets = Arc::clone(&all_secrets);
        let prompt_mutex = Arc::clone(&prompt_mutex);
        master_set.spawn(async move {
            let roles = client.get_roles().await.context("Failed to fetch roles")?;
            let roles_dir = workspace_dir.join("roles");
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
                let realm_name = realm_name.clone();
                let prompt_mutex = Arc::clone(&prompt_mutex);
                set.spawn(async move {
                    let name = role.get_name();
                    let filename = format!("{}.yaml", sanitize(&name));
                    let path = roles_dir.join(filename);
                    let mut local_secrets = HashMap::new();
                    let prefix = format!("realm_{}_role", realm_name);
                    let yaml = to_sorted_yaml_with_secrets(&role, &prefix, &mut local_secrets)
                        .context("Failed to serialize role")?;
                    all_secrets.lock().await.extend(local_secrets);
                    write_if_changed_with_mutex(&path, &yaml, yes, prompt_mutex).await
                });
            }
            while let Some(res) = set.join_next().await {
                res.context("Task panicked")??;
            }
            println!(
                "  {} {}",
                SUCCESS,
                style("Exported roles to roles/").green()
            );
            Ok(())
        });
    }

    // Fetch client scopes
    {
        let client = client.clone();
        let realm_name = realm_name.to_string();
        let workspace_dir = workspace_dir.clone();
        let all_secrets = Arc::clone(&all_secrets);
        let prompt_mutex = Arc::clone(&prompt_mutex);
        master_set.spawn(async move {
            let client_scopes = client
                .get_client_scopes()
                .await
                .context("Failed to fetch client scopes")?;
            let scopes_dir = workspace_dir.join("client-scopes");
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
                let realm_name = realm_name.clone();
                let prompt_mutex = Arc::clone(&prompt_mutex);
                set.spawn(async move {
                    let name = scope.get_name();
                    let filename = format!("{}.yaml", sanitize(&name));
                    let path = scopes_dir.join(filename);
                    let mut local_secrets = HashMap::new();
                    let prefix = format!("realm_{}_client_scope", realm_name);
                    let yaml = to_sorted_yaml_with_secrets(&scope, &prefix, &mut local_secrets)
                        .context("Failed to serialize client scope")?;
                    all_secrets.lock().await.extend(local_secrets);
                    write_if_changed_with_mutex(&path, &yaml, yes, prompt_mutex).await
                });
            }
            while let Some(res) = set.join_next().await {
                res.context("Task panicked")??;
            }
            println!(
                "  {} {}",
                SUCCESS,
                style("Exported client scopes to client-scopes/").green()
            );
            Ok(())
        });
    }

    // Fetch identity providers
    {
        let client = client.clone();
        let realm_name = realm_name.to_string();
        let workspace_dir = workspace_dir.clone();
        let all_secrets = Arc::clone(&all_secrets);
        let prompt_mutex = Arc::clone(&prompt_mutex);
        master_set.spawn(async move {
            let idps = client
                .get_identity_providers()
                .await
                .context("Failed to fetch identity providers")?;
            let idps_dir = workspace_dir.join("identity-providers");
            if !fs::try_exists(&idps_dir)
                .await
                .context("Failed to check identity-providers directory")?
            {
                fs::create_dir_all(&idps_dir)
                    .await
                    .context("Failed to create identity-providers directory")?;
            }
            let mut set = tokio::task::JoinSet::new();
            for idp in idps {
                let idps_dir = idps_dir.clone();
                let all_secrets = Arc::clone(&all_secrets);
                let realm_name = realm_name.clone();
                let prompt_mutex = Arc::clone(&prompt_mutex);
                set.spawn(async move {
                    let name = idp.get_name();
                    let filename = format!("{}.yaml", sanitize(&name));
                    let path = idps_dir.join(filename);
                    let mut local_secrets = HashMap::new();
                    let prefix = format!("realm_{}_idp", realm_name);
                    let yaml = to_sorted_yaml_with_secrets(&idp, &prefix, &mut local_secrets)
                        .context("Failed to serialize identity provider")?;
                    all_secrets.lock().await.extend(local_secrets);
                    write_if_changed_with_mutex(&path, &yaml, yes, prompt_mutex).await
                });
            }
            while let Some(res) = set.join_next().await {
                res.context("Task panicked")??;
            }
            println!(
                "  {} {}",
                SUCCESS,
                style("Exported identity providers to identity-providers/").green()
            );
            Ok(())
        });
    }

    // Fetch groups
    {
        let client = client.clone();
        let realm_name = realm_name.to_string();
        let workspace_dir = workspace_dir.clone();
        let all_secrets = Arc::clone(&all_secrets);
        let prompt_mutex = Arc::clone(&prompt_mutex);
        master_set.spawn(async move {
            let groups = client
                .get_groups()
                .await
                .context("Failed to fetch groups")?;
            let groups_dir = workspace_dir.join("groups");
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
                let realm_name = realm_name.clone();
                let prompt_mutex = Arc::clone(&prompt_mutex);
                set.spawn(async move {
                    let name = group.get_name();
                    let id = group.id.as_deref().unwrap_or("unknown");
                    let filename = format!("{}-{}.yaml", sanitize(&name), id);
                    let path = groups_dir.join(filename);
                    let mut local_secrets = HashMap::new();
                    let prefix = format!("realm_{}_group", realm_name);
                    let yaml = to_sorted_yaml_with_secrets(&group, &prefix, &mut local_secrets)
                        .context("Failed to serialize group")?;
                    all_secrets.lock().await.extend(local_secrets);
                    write_if_changed_with_mutex(&path, &yaml, yes, prompt_mutex).await
                });
            }
            while let Some(res) = set.join_next().await {
                res.context("Task panicked")??;
            }
            println!(
                "  {} {}",
                SUCCESS,
                style("Exported groups to groups/").green()
            );
            Ok(())
        });
    }

    // Fetch users
    {
        let client = client.clone();
        let realm_name = realm_name.to_string();
        let workspace_dir = workspace_dir.clone();
        let all_secrets = Arc::clone(&all_secrets);
        let prompt_mutex = Arc::clone(&prompt_mutex);
        master_set.spawn(async move {
            let users = client.get_users().await.context("Failed to fetch users")?;
            let users_dir = workspace_dir.join("users");
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
                let realm_name = realm_name.clone();
                let prompt_mutex = Arc::clone(&prompt_mutex);
                set.spawn(async move {
                    let name = user.get_name();
                    let filename = format!("{}.yaml", sanitize(&name));
                    let path = users_dir.join(filename);
                    let mut local_secrets = HashMap::new();
                    let prefix = format!("realm_{}_user", realm_name);
                    let yaml = to_sorted_yaml_with_secrets(&user, &prefix, &mut local_secrets)
                        .context("Failed to serialize user")?;
                    all_secrets.lock().await.extend(local_secrets);
                    write_if_changed_with_mutex(&path, &yaml, yes, prompt_mutex).await
                });
            }
            while let Some(res) = set.join_next().await {
                res.context("Task panicked")??;
            }
            println!(
                "  {} {}",
                SUCCESS,
                style("Exported users to users/").green()
            );
            Ok(())
        });
    }

    // Fetch authentication flows
    {
        let client = client.clone();
        let realm_name = realm_name.to_string();
        let workspace_dir = workspace_dir.clone();
        let all_secrets = Arc::clone(&all_secrets);
        let prompt_mutex = Arc::clone(&prompt_mutex);
        master_set.spawn(async move {
            let flows = client
                .get_authentication_flows()
                .await
                .context("Failed to fetch authentication flows")?;
            let flows_dir = workspace_dir.join("authentication-flows");
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
                let realm_name = realm_name.clone();
                let prompt_mutex = Arc::clone(&prompt_mutex);
                set.spawn(async move {
                    let name = flow.get_name();
                    let filename = format!("{}.yaml", sanitize(&name));
                    let path = flows_dir.join(filename);
                    let mut local_secrets = HashMap::new();
                    let prefix = format!("realm_{}_flow", realm_name);
                    let yaml = to_sorted_yaml_with_secrets(&flow, &prefix, &mut local_secrets)
                        .context("Failed to serialize authentication flow")?;
                    all_secrets.lock().await.extend(local_secrets);
                    write_if_changed_with_mutex(&path, &yaml, yes, prompt_mutex).await
                });
            }
            while let Some(res) = set.join_next().await {
                res.context("Task panicked")??;
            }
            println!(
                "  {} {}",
                SUCCESS,
                style("Exported authentication flows to authentication-flows/").green()
            );
            Ok(())
        });
    }

    // Fetch required actions
    {
        let client = client.clone();
        let realm_name = realm_name.to_string();
        let workspace_dir = workspace_dir.clone();
        let all_secrets = Arc::clone(&all_secrets);
        let prompt_mutex = Arc::clone(&prompt_mutex);
        master_set.spawn(async move {
            let actions = client
                .get_required_actions()
                .await
                .context("Failed to fetch required actions")?;
            let actions_dir = workspace_dir.join("required-actions");
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
                let realm_name = realm_name.clone();
                let prompt_mutex = Arc::clone(&prompt_mutex);
                set.spawn(async move {
                    let name = action.get_name();
                    let filename = format!("{}.yaml", sanitize(&name));
                    let path = actions_dir.join(filename);
                    let mut local_secrets = HashMap::new();
                    let prefix = format!("realm_{}_action", realm_name);
                    let yaml = to_sorted_yaml_with_secrets(&action, &prefix, &mut local_secrets)
                        .context("Failed to serialize required action")?;
                    all_secrets.lock().await.extend(local_secrets);
                    write_if_changed_with_mutex(&path, &yaml, yes, prompt_mutex).await
                });
            }
            while let Some(res) = set.join_next().await {
                res.context("Task panicked")??;
            }
            println!(
                "  {} {}",
                SUCCESS,
                style("Exported required actions to required-actions/").green()
            );
            Ok(())
        });
    }

    // Fetch components and keys
    {
        let client = client.clone();
        let realm_name = realm_name.to_string();
        let workspace_dir = workspace_dir.clone();
        let all_secrets = Arc::clone(&all_secrets);
        let prompt_mutex = Arc::clone(&prompt_mutex);
        master_set.spawn(async move {
            let all_components = client
                .get_components()
                .await
                .context("Failed to fetch components")?;

            let components_dir = workspace_dir.join("components");
            if !fs::try_exists(&components_dir)
                .await
                .context("Failed to check components directory")?
            {
                fs::create_dir_all(&components_dir)
                    .await
                    .context("Failed to create components directory")?;
            }

            let keys_dir = workspace_dir.join("keys");
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
                let realm_name = realm_name.clone();
                let prompt_mutex = Arc::clone(&prompt_mutex);
                set.spawn(async move {
                    let name = component.get_name();
                    let id = component.id.as_deref().unwrap_or("unknown");
                    let filename = format!("{}-{}.yaml", sanitize(&name), id);
                    let path = target_dir.join(filename);
                    let mut local_secrets = HashMap::new();
                    let sub_prefix = if is_key { "key" } else { "component" };
                    let prefix = format!("realm_{}_{}", realm_name, sub_prefix);
                    let yaml = to_sorted_yaml_with_secrets(&component, &prefix, &mut local_secrets)
                        .context("Failed to serialize component")?;
                    all_secrets.lock().await.extend(local_secrets);
                    write_if_changed_with_mutex(&path, &yaml, yes, prompt_mutex).await
                });
            }
            while let Some(res) = set.join_next().await {
                res.context("Task panicked")??;
            }
            println!(
                "  {} {}",
                SUCCESS,
                style("Exported components to components/ and keys to keys/").green()
            );
            Ok(())
        });
    }

    while let Some(res) = master_set.join_next().await {
        res.context("Master task panicked")??;
    }

    Ok(())
}
