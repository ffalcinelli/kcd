use crate::client::KeycloakClient;
use crate::models::{
    AuthenticationFlowRepresentation, ClientRepresentation, ClientScopeRepresentation,
    ComponentRepresentation, GroupRepresentation, IdentityProviderRepresentation, KeycloakResource,
    RequiredActionProviderRepresentation, ResourceMeta, RoleRepresentation, UserRepresentation,
};
use crate::utils::to_sorted_yaml_with_secrets;
use crate::utils::ui::{CHECK, SEARCH, SUCCESS, WARN};
use anyhow::{Context, Result};
use console::style;
use dialoguer::{Confirm, theme::ColorfulTheme};
use sanitize_filename::sanitize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;
use tokio::sync::Mutex;

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
            SEARCH,
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
            CHECK,
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

async fn inspect_resources<T>(
    client: &KeycloakClient,
    realm_name: &str,
    target_dir: Arc<PathBuf>,
    all_secrets: Arc<Mutex<HashMap<String, String>>>,
    yes: bool,
    prompt_mutex: Arc<Mutex<()>>,
) -> Result<()>
where
    T: KeycloakResource
        + ResourceMeta
        + serde::Serialize
        + for<'de> serde::Deserialize<'de>
        + Send
        + Sync
        + 'static,
{
    let resources = client
        .get_resources::<T>()
        .await
        .with_context(|| format!("Failed to fetch {} for realm '{}'", T::label(), realm_name))?;

    if !fs::try_exists(&*target_dir)
        .await
        .context(format!("Failed to check {} directory", T::label()))?
    {
        fs::create_dir_all(&*target_dir)
            .await
            .context(format!("Failed to create {} directory", T::label()))?;
    }

    let mut set = tokio::task::JoinSet::new();
    for res in resources {
        let target_dir = Arc::clone(&target_dir);
        let all_secrets = Arc::clone(&all_secrets);
        let realm_name = realm_name.to_string();
        let prompt_mutex = Arc::clone(&prompt_mutex);
        set.spawn(async move {
            let filename = format!("{}.yaml", sanitize(res.get_filename()));
            let path = target_dir.join(filename);
            let mut local_secrets = HashMap::new();
            let prefix = format!("realm_{}_{}", realm_name, T::secret_prefix());
            let yaml = to_sorted_yaml_with_secrets(&res, &prefix, &mut local_secrets).context(
                format!("Failed to serialize {} {}", T::label(), res.get_name()),
            )?;
            all_secrets.lock().await.extend(local_secrets);
            write_if_changed_with_mutex(&path, &yaml, yes, prompt_mutex).await
        });
    }
    while let Some(res) = set.join_next().await {
        res.context("Task panicked")??;
    }
    {
        let _lock = prompt_mutex.lock().await;
        println!(
            "  {} {}",
            SUCCESS,
            style(format!(
                "Exported {} to {}/",
                T::label(),
                target_dir
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or_default()
            ))
            .green()
        );
    }

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

    let mut set = tokio::task::JoinSet::new();
    let workspace_dir = Arc::new(workspace_dir);

    // Fetch realm configuration in parallel
    {
        let client = client.clone();
        let realm_name = realm_name.to_string();
        let workspace_dir = Arc::clone(&workspace_dir);
        let all_secrets = Arc::clone(&all_secrets);
        let prompt_mutex = Arc::clone(&prompt_mutex);
        set.spawn(async move {
            let realm = client.get_realm().await.context("Failed to fetch realm")?;
            let mut local_secrets = HashMap::new();
            let realm_prefix = format!("realm_{}", realm_name);
            let realm_yaml = to_sorted_yaml_with_secrets(&realm, &realm_prefix, &mut local_secrets)
                .context("Failed to serialize realm")?;
            all_secrets.lock().await.extend(local_secrets);

            let realm_path = workspace_dir.join("realm.yaml");
            write_if_changed_with_mutex(&realm_path, &realm_yaml, yes, Arc::clone(&prompt_mutex))
                .await?;
            {
                let _lock = prompt_mutex.lock().await;
                println!(
                    "  {} {}",
                    SUCCESS,
                    style("Exported realm configuration to realm.yaml").green()
                );
            }
            Ok::<(), anyhow::Error>(())
        });
    }

    // Fetch resources in parallel
    spawn_inspect::<ClientRepresentation>(
        &mut set,
        client,
        realm_name,
        &workspace_dir,
        &all_secrets,
        yes,
        &prompt_mutex,
    );
    spawn_inspect::<RoleRepresentation>(
        &mut set,
        client,
        realm_name,
        &workspace_dir,
        &all_secrets,
        yes,
        &prompt_mutex,
    );
    spawn_inspect::<ClientScopeRepresentation>(
        &mut set,
        client,
        realm_name,
        &workspace_dir,
        &all_secrets,
        yes,
        &prompt_mutex,
    );
    spawn_inspect::<IdentityProviderRepresentation>(
        &mut set,
        client,
        realm_name,
        &workspace_dir,
        &all_secrets,
        yes,
        &prompt_mutex,
    );
    spawn_inspect::<GroupRepresentation>(
        &mut set,
        client,
        realm_name,
        &workspace_dir,
        &all_secrets,
        yes,
        &prompt_mutex,
    );
    spawn_inspect::<UserRepresentation>(
        &mut set,
        client,
        realm_name,
        &workspace_dir,
        &all_secrets,
        yes,
        &prompt_mutex,
    );
    spawn_inspect::<AuthenticationFlowRepresentation>(
        &mut set,
        client,
        realm_name,
        &workspace_dir,
        &all_secrets,
        yes,
        &prompt_mutex,
    );
    spawn_inspect::<RequiredActionProviderRepresentation>(
        &mut set,
        client,
        realm_name,
        &workspace_dir,
        &all_secrets,
        yes,
        &prompt_mutex,
    );
    spawn_inspect::<ComponentRepresentation>(
        &mut set,
        client,
        realm_name,
        &workspace_dir,
        &all_secrets,
        yes,
        &prompt_mutex,
    );

    while let Some(res) = set.join_next().await {
        res.context("Task panicked")??;
    }

    Ok(())
}

fn spawn_inspect<T>(
    set: &mut tokio::task::JoinSet<Result<()>>,
    client: &KeycloakClient,
    realm_name: &str,
    workspace_dir: &Arc<PathBuf>,
    all_secrets: &Arc<Mutex<HashMap<String, String>>>,
    yes: bool,
    prompt_mutex: &Arc<Mutex<()>>,
) where
    T: KeycloakResource
        + ResourceMeta
        + serde::Serialize
        + for<'de> serde::Deserialize<'de>
        + Send
        + Sync
        + 'static,
{
    let client = client.clone();
    let realm_name = realm_name.to_string();
    let target_dir = Arc::new(workspace_dir.join(T::dir_name()));
    let all_secrets = Arc::clone(all_secrets);
    let prompt_mutex = Arc::clone(prompt_mutex);

    set.spawn(async move {
        inspect_resources::<T>(
            &client,
            &realm_name,
            target_dir,
            all_secrets,
            yes,
            prompt_mutex,
        )
        .await
    });
}
