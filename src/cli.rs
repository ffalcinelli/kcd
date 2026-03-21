use crate::models::{
    ClientRepresentation, ClientScopeRepresentation, ComponentRepresentation,
    CredentialRepresentation, GroupRepresentation, IdentityProviderRepresentation,
    RoleRepresentation, UserRepresentation,
};
use anyhow::{Context, Result};
use console::{Emoji, style};
use dialoguer::{Confirm, Input, Password, Select, theme::ColorfulTheme};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::fs;

static SUCCESS: Emoji<'_, '_> = Emoji("✨ ", "* ");
static ERROR: Emoji<'_, '_> = Emoji("❌ ", "x ");
static WARN: Emoji<'_, '_> = Emoji("⚠️ ", "! ");
static INFO: Emoji<'_, '_> = Emoji("💡 ", "i ");

pub async fn run(workspace_dir: PathBuf) -> Result<()> {
    println!(
        "{} {}",
        INFO,
        style("Welcome to kcd interactive CLI!").cyan().bold()
    );
    let theme = ColorfulTheme::default();
    let selections = &[
        "Create User",
        "Change User Password",
        "Create Client",
        "Create Role",
        "Create Group",
        "Create Identity Provider",
        "Create Client Scope",
        "Rotate Keys",
        "Exit",
    ];

    loop {
        let selection = Select::with_theme(&theme)
            .with_prompt("What would you like to do?")
            .default(0)
            .items(&selections[..])
            .interact()?;

        match selection {
            0 => {
                if let Err(e) = create_user_interactive(&workspace_dir).await {
                    println!(
                        "{} {}",
                        ERROR,
                        style(format!("Error creating user: {}", e)).red()
                    );
                }
            }
            1 => {
                if let Err(e) = change_user_password_interactive(&workspace_dir).await {
                    println!(
                        "{} {}",
                        ERROR,
                        style(format!("Error changing password: {}", e)).red()
                    );
                }
            }
            2 => {
                if let Err(e) = create_client_interactive(&workspace_dir).await {
                    println!(
                        "{} {}",
                        ERROR,
                        style(format!("Error creating client: {}", e)).red()
                    );
                }
            }
            3 => {
                if let Err(e) = create_role_interactive(&workspace_dir).await {
                    println!(
                        "{} {}",
                        ERROR,
                        style(format!("Error creating role: {}", e)).red()
                    );
                }
            }
            4 => {
                if let Err(e) = create_group_interactive(&workspace_dir).await {
                    println!(
                        "{} {}",
                        ERROR,
                        style(format!("Error creating group: {}", e)).red()
                    );
                }
            }
            5 => {
                if let Err(e) = create_idp_interactive(&workspace_dir).await {
                    println!(
                        "{} {}",
                        ERROR,
                        style(format!("Error creating IDP: {}", e)).red()
                    );
                }
            }
            6 => {
                if let Err(e) = create_client_scope_interactive(&workspace_dir).await {
                    println!(
                        "{} {}",
                        ERROR,
                        style(format!("Error creating client scope: {}", e)).red()
                    );
                }
            }
            7 => {
                if let Err(e) = rotate_keys_interactive(&workspace_dir).await {
                    println!(
                        "{} {}",
                        ERROR,
                        style(format!("Error rotating keys: {}", e)).red()
                    );
                }
            }
            8 => {
                println!("{} {}", INFO, style("Exiting...").cyan());
                break;
            }
            _ => unreachable!(),
        }
    }

    Ok(())
}

async fn create_role_interactive(workspace_dir: &Path) -> Result<()> {
    let theme = ColorfulTheme::default();

    let realm: String = Input::with_theme(&theme)
        .with_prompt("Target Realm")
        .interact_text()?;

    let name: String = Input::with_theme(&theme)
        .with_prompt("Role Name")
        .interact_text()?;

    let description: String = Input::with_theme(&theme)
        .with_prompt("Description")
        .allow_empty(true)
        .interact_text()?;

    let is_client_role = Confirm::with_theme(&theme)
        .with_prompt("Is this a client role?")
        .default(false)
        .interact()?;

    let client_id = if is_client_role {
        let id: String = Input::with_theme(&theme)
            .with_prompt("Client ID")
            .interact_text()?;
        Some(id)
    } else {
        None
    };

    let description_opt = if description.is_empty() {
        None
    } else {
        Some(description)
    };

    create_role_yaml(workspace_dir, &realm, &name, description_opt, client_id).await?;

    println!(
        "{} {}",
        SUCCESS,
        style(format!(
            "Successfully generated YAML for role '{}' in realm '{}'.",
            name, realm
        ))
        .green()
    );
    Ok(())
}

pub async fn create_role_yaml(
    workspace_dir: &Path,
    realm: &str,
    name: &str,
    description: Option<String>,
    client_id: Option<String>,
) -> Result<()> {
    let role = RoleRepresentation {
        id: None,
        name: name.to_string(),
        description,
        container_id: None,
        composite: false,
        client_role: client_id.is_some(),
        extra: HashMap::new(),
    };

    let realm_dir = workspace_dir.join(realm);
    let roles_dir = if let Some(cid) = &client_id {
        realm_dir.join("clients").join(cid).join("roles")
    } else {
        realm_dir.join("roles")
    };

    fs::create_dir_all(&roles_dir)
        .await
        .context("Failed to create roles directory")?;

    let file_path = roles_dir.join(format!("{}.yaml", name));
    let yaml = serde_yaml::to_string(&role).context("Failed to serialize role to YAML")?;

    fs::write(&file_path, yaml)
        .await
        .context("Failed to write role YAML file")?;

    Ok(())
}

async fn create_group_interactive(workspace_dir: &Path) -> Result<()> {
    let theme = ColorfulTheme::default();

    let realm: String = Input::with_theme(&theme)
        .with_prompt("Target Realm")
        .interact_text()?;

    let name: String = Input::with_theme(&theme)
        .with_prompt("Group Name")
        .interact_text()?;

    create_group_yaml(workspace_dir, &realm, &name).await?;

    println!(
        "{} {}",
        SUCCESS,
        style(format!(
            "Successfully generated YAML for group '{}' in realm '{}'.",
            name, realm
        ))
        .green()
    );
    Ok(())
}

pub async fn create_group_yaml(workspace_dir: &Path, realm: &str, name: &str) -> Result<()> {
    let group = GroupRepresentation {
        id: None,
        name: Some(name.to_string()),
        path: None,
        sub_groups: None,
        extra: HashMap::new(),
    };

    let groups_dir = workspace_dir.join(realm).join("groups");
    fs::create_dir_all(&groups_dir)
        .await
        .context("Failed to create groups directory")?;

    let file_path = groups_dir.join(format!("{}.yaml", name));
    let yaml = serde_yaml::to_string(&group).context("Failed to serialize group to YAML")?;

    fs::write(&file_path, yaml)
        .await
        .context("Failed to write group YAML file")?;

    Ok(())
}

async fn create_idp_interactive(workspace_dir: &Path) -> Result<()> {
    let theme = ColorfulTheme::default();

    let realm: String = Input::with_theme(&theme)
        .with_prompt("Target Realm")
        .interact_text()?;

    let alias: String = Input::with_theme(&theme)
        .with_prompt("Alias (e.g., google)")
        .interact_text()?;

    let provider_id: String = Input::with_theme(&theme)
        .with_prompt("Provider ID (e.g., google, github, oidc)")
        .default(alias.clone())
        .interact_text()?;

    create_idp_yaml(workspace_dir, &realm, &alias, &provider_id).await?;

    println!(
        "{} {}",
        SUCCESS,
        style(format!(
            "Successfully generated YAML for Identity Provider '{}' in realm '{}'.",
            alias, realm
        ))
        .green()
    );
    Ok(())
}

pub async fn create_idp_yaml(
    workspace_dir: &Path,
    realm: &str,
    alias: &str,
    provider_id: &str,
) -> Result<()> {
    let idp = IdentityProviderRepresentation {
        internal_id: None,
        alias: Some(alias.to_string()),
        provider_id: Some(provider_id.to_string()),
        enabled: Some(true),
        update_profile_first_login_mode: Some("on".to_string()),
        trust_email: Some(false),
        store_token: Some(false),
        add_read_token_role_on_create: Some(false),
        authenticate_by_default: Some(false),
        link_only: Some(false),
        first_broker_login_flow_alias: Some("first broker login".to_string()),
        post_broker_login_flow_alias: None,
        display_name: Some(alias.to_string()),
        config: Some(HashMap::new()),
        extra: HashMap::new(),
    };

    let idp_dir = workspace_dir.join(realm).join("identity-providers");
    fs::create_dir_all(&idp_dir)
        .await
        .context("Failed to create identity-providers directory")?;

    let file_path = idp_dir.join(format!("{}.yaml", alias));
    let yaml = serde_yaml::to_string(&idp).context("Failed to serialize IDP to YAML")?;

    fs::write(&file_path, yaml)
        .await
        .context("Failed to write IDP YAML file")?;

    Ok(())
}

async fn create_client_scope_interactive(workspace_dir: &Path) -> Result<()> {
    let theme = ColorfulTheme::default();

    let realm: String = Input::with_theme(&theme)
        .with_prompt("Target Realm")
        .interact_text()?;

    let name: String = Input::with_theme(&theme)
        .with_prompt("Scope Name")
        .interact_text()?;

    let protocol: String = Input::with_theme(&theme)
        .with_prompt("Protocol")
        .default("openid-connect".to_string())
        .interact_text()?;

    create_client_scope_yaml(workspace_dir, &realm, &name, &protocol).await?;

    println!(
        "{} {}",
        SUCCESS,
        style(format!(
            "Successfully generated YAML for client scope '{}' in realm '{}'.",
            name, realm
        ))
        .green()
    );
    Ok(())
}

pub async fn create_client_scope_yaml(
    workspace_dir: &Path,
    realm: &str,
    name: &str,
    protocol: &str,
) -> Result<()> {
    let scope = ClientScopeRepresentation {
        id: None,
        name: Some(name.to_string()),
        description: None,
        protocol: Some(protocol.to_string()),
        attributes: Some(HashMap::new()),
        extra: HashMap::new(),
    };

    let scopes_dir = workspace_dir.join(realm).join("client-scopes");
    fs::create_dir_all(&scopes_dir)
        .await
        .context("Failed to create client-scopes directory")?;

    let file_path = scopes_dir.join(format!("{}.yaml", name));
    let yaml = serde_yaml::to_string(&scope).context("Failed to serialize client scope to YAML")?;

    fs::write(&file_path, yaml)
        .await
        .context("Failed to write client scope YAML file")?;

    Ok(())
}

async fn rotate_keys_interactive(workspace_dir: &Path) -> Result<()> {
    let theme = ColorfulTheme::default();

    let realm: String = Input::with_theme(&theme)
        .with_prompt("Target Realm")
        .interact_text()?;

    let rotated_count = rotate_keys_yaml(workspace_dir, &realm).await?;

    if rotated_count > 0 {
        println!(
            "{} {}",
            SUCCESS,
            style(format!(
                "Successfully generated {} rotated key component(s) for realm '{}'.",
                rotated_count, realm
            ))
            .green()
        );
    } else {
        println!(
            "{} {}",
            INFO,
            style(format!(
                "No key providers found to rotate for realm '{}'.",
                realm
            ))
            .cyan()
        );
    }

    Ok(())
}

pub async fn rotate_keys_yaml(workspace_dir: &Path, realm: &str) -> Result<usize> {
    let keys_dir = workspace_dir.join(realm).join("components");

    if !keys_dir.exists() {
        return Ok(0);
    }

    let mut rotated_count = 0;
    let mut entries = fs::read_dir(&keys_dir)
        .await
        .context("Failed to read components directory")?;

    while let Some(entry) = entries
        .next_entry()
        .await
        .context("Failed to read directory entry")?
    {
        let path = entry.path();
        if path.is_file()
            && path
                .extension()
                .is_some_and(|ext| ext == "yaml" || ext == "yml")
        {
            let yaml_content = fs::read_to_string(&path)
                .await
                .context("Failed to read key YAML file")?;

            #[allow(clippy::collapsible_if)]
            if let Ok(component) = serde_yaml::from_str::<ComponentRepresentation>(&yaml_content) {
                if component.provider_type.as_deref() == Some("org.keycloak.keys.KeyProvider") {
                    let mut new_component = component.clone();
                    new_component.id = None;

                    let old_name = new_component
                        .name
                        .clone()
                        .unwrap_or_else(|| "key".to_string());
                    let timestamp = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .context("System clock is before UNIX EPOCH")?
                        .as_secs();
                    new_component.name = Some(format!("{}-rotated-{}", old_name, timestamp));

                    #[allow(clippy::collapsible_if)]
                    if let Some(config) = &mut new_component.config {
                        if let Some(priority_vals) = config.get_mut("priority") {
                            if let Some(arr) = priority_vals.as_array_mut() {
                                if let Some(first) = arr.first_mut() {
                                    if let Some(p_str) = first.as_str() {
                                        if let Ok(p_num) = p_str.parse::<i64>() {
                                            *first =
                                                serde_json::Value::String((p_num + 10).to_string());
                                        }
                                    }
                                }
                            }
                        }
                    }

                    let new_filename = format!("{}.yaml", new_component.name.as_deref().unwrap());
                    let new_file_path = keys_dir.join(new_filename);

                    let yaml = serde_yaml::to_string(&new_component)
                        .context("Failed to serialize rotated key to YAML")?;
                    fs::write(&new_file_path, yaml)
                        .await
                        .context("Failed to write rotated key YAML file")?;

                    rotated_count += 1;
                }
            }
        }
    }

    Ok(rotated_count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_create_user_yaml() {
        let dir = tempdir().unwrap();
        let workspace_dir = dir.path();

        create_user_yaml(
            workspace_dir,
            "master",
            "testuser",
            Some("test@example.com".to_string()),
            Some("Test".to_string()),
            Some("User".to_string()),
        )
        .await
        .unwrap();

        let file_path = workspace_dir
            .join("master")
            .join("users")
            .join("testuser.yaml");
        assert!(file_path.exists());

        let content = fs::read_to_string(&file_path).await.unwrap();
        let user: UserRepresentation = serde_yaml::from_str(&content).unwrap();

        assert_eq!(user.username.as_deref(), Some("testuser"));
        assert_eq!(user.email.as_deref(), Some("test@example.com"));
        assert_eq!(user.first_name.as_deref(), Some("Test"));
        assert_eq!(user.last_name.as_deref(), Some("User"));
    }

    #[tokio::test]
    async fn test_change_user_password_yaml() {
        let dir = tempdir().unwrap();
        let workspace_dir = dir.path();

        create_user_yaml(workspace_dir, "master", "testuser", None, None, None)
            .await
            .unwrap();

        change_user_password_yaml(workspace_dir, "master", "testuser", "newpass123")
            .await
            .unwrap();

        let file_path = workspace_dir
            .join("master")
            .join("users")
            .join("testuser.yaml");
        let content = fs::read_to_string(&file_path).await.unwrap();
        let user: UserRepresentation = serde_yaml::from_str(&content).unwrap();

        let credentials = user.credentials.expect("Credentials should not be None");
        assert_eq!(credentials.len(), 1);
        assert_eq!(credentials[0].type_.as_deref(), Some("password"));
        assert_eq!(credentials[0].value.as_deref(), Some("newpass123"));
    }

    #[tokio::test]
    async fn test_create_client_yaml() {
        let dir = tempdir().unwrap();
        let workspace_dir = dir.path();

        create_client_yaml(workspace_dir, "master", "testclient", true)
            .await
            .unwrap();

        let file_path = workspace_dir
            .join("master")
            .join("clients")
            .join("testclient.yaml");
        assert!(file_path.exists());

        let content = fs::read_to_string(&file_path).await.unwrap();
        let client: ClientRepresentation = serde_yaml::from_str(&content).unwrap();

        assert_eq!(client.client_id.as_deref(), Some("testclient"));
        assert_eq!(client.public_client, Some(true));
        assert_eq!(client.service_accounts_enabled, Some(false));
    }

    #[tokio::test]
    async fn test_create_role_yaml() {
        let dir = tempdir().unwrap();
        let workspace_dir = dir.path();

        // Realm role
        create_role_yaml(
            workspace_dir,
            "master",
            "admin",
            Some("desc".to_string()),
            None,
        )
        .await
        .unwrap();
        let realm_role_path = workspace_dir
            .join("master")
            .join("roles")
            .join("admin.yaml");
        assert!(realm_role_path.exists());
        let content = fs::read_to_string(&realm_role_path).await.unwrap();
        let role: RoleRepresentation = serde_yaml::from_str(&content).unwrap();
        assert_eq!(role.name, "admin");
        assert_eq!(role.client_role, false);

        // Client role
        create_role_yaml(
            workspace_dir,
            "master",
            "editor",
            None,
            Some("my-client".to_string()),
        )
        .await
        .unwrap();
        let client_role_path = workspace_dir
            .join("master")
            .join("clients")
            .join("my-client")
            .join("roles")
            .join("editor.yaml");
        assert!(client_role_path.exists());
    }

    #[tokio::test]
    async fn test_create_group_yaml() {
        let dir = tempdir().unwrap();
        let workspace_dir = dir.path();

        create_group_yaml(workspace_dir, "master", "my-group")
            .await
            .unwrap();
        let file_path = workspace_dir
            .join("master")
            .join("groups")
            .join("my-group.yaml");
        assert!(file_path.exists());
    }

    #[tokio::test]
    async fn test_create_idp_yaml() {
        let dir = tempdir().unwrap();
        let workspace_dir = dir.path();

        create_idp_yaml(workspace_dir, "master", "google", "google")
            .await
            .unwrap();
        let file_path = workspace_dir
            .join("master")
            .join("identity-providers")
            .join("google.yaml");
        assert!(file_path.exists());
    }

    #[tokio::test]
    async fn test_create_client_scope_yaml() {
        let dir = tempdir().unwrap();
        let workspace_dir = dir.path();

        create_client_scope_yaml(workspace_dir, "master", "my-scope", "openid-connect")
            .await
            .unwrap();
        let file_path = workspace_dir
            .join("master")
            .join("client-scopes")
            .join("my-scope.yaml");
        assert!(file_path.exists());
    }

    #[tokio::test]
    async fn test_rotate_keys_yaml() {
        let dir = tempdir().unwrap();
        let workspace_dir = dir.path();

        let keys_dir = workspace_dir.join("master").join("components");
        fs::create_dir_all(&keys_dir).await.unwrap();

        let original_component = ComponentRepresentation {
            id: None,
            name: Some("rsa-generated".to_string()),
            provider_id: Some("rsa-generated".to_string()),
            provider_type: Some("org.keycloak.keys.KeyProvider".to_string()),
            parent_id: Some("master".to_string()),
            sub_type: None,
            config: Some({
                let mut map = HashMap::new();
                map.insert("priority".to_string(), serde_json::json!(["100"]));
                map
            }),
            extra: HashMap::new(),
        };

        let original_yaml = serde_yaml::to_string(&original_component).unwrap();
        fs::write(keys_dir.join("rsa-generated.yaml"), original_yaml)
            .await
            .unwrap();

        let count = rotate_keys_yaml(workspace_dir, "master").await.unwrap();
        assert_eq!(count, 1);

        let mut entries = fs::read_dir(&keys_dir).await.unwrap();
        let mut found_rotated = false;

        while let Some(entry) = entries.next_entry().await.unwrap() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("rsa-generated-rotated-") {
                found_rotated = true;
                let content = fs::read_to_string(entry.path()).await.unwrap();
                let rotated: ComponentRepresentation = serde_yaml::from_str(&content).unwrap();

                let config = rotated.config.unwrap();
                let priority_array = config.get("priority").unwrap().as_array().unwrap();
                assert_eq!(priority_array[0].as_str().unwrap(), "110");
            }
        }

        assert!(found_rotated, "Did not find a rotated key component file");
    }
}

async fn create_client_interactive(workspace_dir: &Path) -> Result<()> {
    let theme = ColorfulTheme::default();

    let realm: String = Input::with_theme(&theme)
        .with_prompt("Target Realm")
        .interact_text()?;

    let client_id: String = Input::with_theme(&theme)
        .with_prompt("Client ID")
        .interact_text()?;

    let is_public = Confirm::with_theme(&theme)
        .with_prompt("Is this a public client? (No for confidential)")
        .default(true)
        .interact()?;

    create_client_yaml(workspace_dir, &realm, &client_id, is_public).await?;

    println!(
        "{} {}",
        SUCCESS,
        style(format!(
            "Successfully generated YAML for client '{}' in realm '{}'.",
            client_id, realm
        ))
        .green()
    );
    Ok(())
}

pub async fn create_client_yaml(
    workspace_dir: &Path,
    realm: &str,
    client_id: &str,
    is_public: bool,
) -> Result<()> {
    let client = ClientRepresentation {
        id: None,
        client_id: Some(client_id.to_string()),
        name: None,
        description: None,
        enabled: Some(true),
        protocol: Some("openid-connect".to_string()),
        redirect_uris: Some(vec!["/*".to_string()]),
        web_origins: Some(vec!["+".to_string()]),
        public_client: Some(is_public),
        bearer_only: Some(false),
        service_accounts_enabled: Some(!is_public),
        extra: HashMap::new(),
    };

    let realm_dir = workspace_dir.join(realm).join("clients");
    fs::create_dir_all(&realm_dir)
        .await
        .context("Failed to create clients directory")?;

    let file_path = realm_dir.join(format!("{}.yaml", client_id));
    let yaml = serde_yaml::to_string(&client).context("Failed to serialize client to YAML")?;

    fs::write(&file_path, yaml)
        .await
        .context("Failed to write client YAML file")?;

    Ok(())
}

async fn change_user_password_interactive(workspace_dir: &Path) -> Result<()> {
    let theme = ColorfulTheme::default();

    let realm: String = Input::with_theme(&theme)
        .with_prompt("Target Realm")
        .interact_text()?;

    let username: String = Input::with_theme(&theme)
        .with_prompt("Username")
        .interact_text()?;

    let new_password = Password::with_theme(&theme)
        .with_prompt("New Password")
        .with_confirmation("Confirm Password", "Passwords mismatching")
        .interact()?;

    change_user_password_yaml(workspace_dir, &realm, &username, &new_password).await?;

    println!(
        "{} {}",
        SUCCESS,
        style(format!(
            "Successfully updated YAML for user '{}' in realm '{}' with new password.",
            username, realm
        ))
        .green()
    );
    Ok(())
}

pub async fn change_user_password_yaml(
    workspace_dir: &Path,
    realm: &str,
    username: &str,
    new_password: &str,
) -> Result<()> {
    let file_path = workspace_dir
        .join(realm)
        .join("users")
        .join(format!("{}.yaml", username));

    if !file_path.exists() {
        println!(
            "{} {}",
            WARN,
            style(format!(
                "Warning: User file {:?} does not exist. Creating a new one.",
                file_path
            ))
            .yellow()
        );
        create_user_yaml(workspace_dir, realm, username, None, None, None).await?;
    }

    let yaml_content = fs::read_to_string(&file_path)
        .await
        .context("Failed to read user YAML file")?;
    let mut user: UserRepresentation =
        serde_yaml::from_str(&yaml_content).context("Failed to parse user YAML file")?;

    let new_cred = CredentialRepresentation {
        id: None,
        type_: Some("password".to_string()),
        value: Some(new_password.to_string()),
        temporary: Some(false),
        extra: HashMap::new(),
    };

    if let Some(credentials) = &mut user.credentials {
        if let Some(existing) = credentials
            .iter_mut()
            .find(|c| c.type_.as_deref() == Some("password"))
        {
            existing.value = Some(new_password.to_string());
        } else {
            credentials.push(new_cred);
        }
    } else {
        user.credentials = Some(vec![new_cred]);
    }

    let yaml = serde_yaml::to_string(&user).context("Failed to serialize user to YAML")?;
    fs::write(&file_path, yaml)
        .await
        .context("Failed to write updated user YAML file")?;

    Ok(())
}

async fn create_user_interactive(workspace_dir: &Path) -> Result<()> {
    let theme = ColorfulTheme::default();

    let realm: String = Input::with_theme(&theme)
        .with_prompt("Target Realm")
        .interact_text()?;

    let username: String = Input::with_theme(&theme)
        .with_prompt("Username")
        .interact_text()?;

    let email: String = Input::with_theme(&theme)
        .with_prompt("Email")
        .allow_empty(true)
        .interact_text()?;

    let first_name: String = Input::with_theme(&theme)
        .with_prompt("First Name")
        .allow_empty(true)
        .interact_text()?;

    let last_name: String = Input::with_theme(&theme)
        .with_prompt("Last Name")
        .allow_empty(true)
        .interact_text()?;

    let email_opt = if email.is_empty() { None } else { Some(email) };
    let first_name_opt = if first_name.is_empty() {
        None
    } else {
        Some(first_name)
    };
    let last_name_opt = if last_name.is_empty() {
        None
    } else {
        Some(last_name)
    };

    create_user_yaml(
        workspace_dir,
        &realm,
        &username,
        email_opt,
        first_name_opt,
        last_name_opt,
    )
    .await?;

    println!(
        "{} {}",
        SUCCESS,
        style(format!(
            "Successfully generated YAML for user '{}' in realm '{}'.",
            username, realm
        ))
        .green()
    );
    Ok(())
}

pub async fn create_user_yaml(
    workspace_dir: &Path,
    realm: &str,
    username: &str,
    email: Option<String>,
    first_name: Option<String>,
    last_name: Option<String>,
) -> Result<()> {
    let user = UserRepresentation {
        id: None,
        username: Some(username.to_string()),
        enabled: Some(true),
        first_name,
        last_name,
        email,
        email_verified: Some(false),
        credentials: None,
        extra: HashMap::new(),
    };

    let realm_dir = workspace_dir.join(realm).join("users");
    fs::create_dir_all(&realm_dir)
        .await
        .context("Failed to create users directory")?;

    let file_path = realm_dir.join(format!("{}.yaml", username));
    let yaml = serde_yaml::to_string(&user).context("Failed to serialize user to YAML")?;

    fs::write(&file_path, yaml)
        .await
        .context("Failed to write user YAML file")?;

    Ok(())
}
