use crate::models::{
    ClientRepresentation, ComponentRepresentation, CredentialRepresentation, UserRepresentation,
};
use anyhow::{Context, Result};
use dialoguer::{Confirm, Input, Password, Select, theme::ColorfulTheme};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::fs;

pub async fn run(config_dir: PathBuf) -> Result<()> {
    println!("Welcome to kcd interactive CLI!");
    let theme = ColorfulTheme::default();
    let selections = &[
        "Create User",
        "Change User Password",
        "Create Client",
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
                if let Err(e) = create_user_interactive(&config_dir).await {
                    println!("Error creating user: {}", e);
                }
            }
            1 => {
                if let Err(e) = change_user_password_interactive(&config_dir).await {
                    println!("Error changing password: {}", e);
                }
            }
            2 => {
                if let Err(e) = create_client_interactive(&config_dir).await {
                    println!("Error creating client: {}", e);
                }
            }
            3 => {
                if let Err(e) = rotate_keys_interactive(&config_dir).await {
                    println!("Error rotating keys: {}", e);
                }
            }
            4 => {
                println!("Exiting...");
                break;
            }
            _ => unreachable!(),
        }
    }

    Ok(())
}

async fn rotate_keys_interactive(config_dir: &Path) -> Result<()> {
    let theme = ColorfulTheme::default();

    let realm: String = Input::with_theme(&theme)
        .with_prompt("Target Realm")
        .interact_text()?;

    let rotated_count = rotate_keys_yaml(config_dir, &realm).await?;

    if rotated_count > 0 {
        println!(
            "Successfully generated {} rotated key component(s) for realm '{}'.",
            rotated_count, realm
        );
    } else {
        println!("No key providers found to rotate for realm '{}'.", realm);
    }

    Ok(())
}

pub async fn rotate_keys_yaml(config_dir: &Path, realm: &str) -> Result<usize> {
    let keys_dir = config_dir.join(realm).join("components");

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
        let config_dir = dir.path();

        create_user_yaml(
            config_dir,
            "master",
            "testuser",
            Some("test@example.com".to_string()),
            Some("Test".to_string()),
            Some("User".to_string()),
        )
        .await
        .unwrap();

        let file_path = config_dir
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
        let config_dir = dir.path();

        create_user_yaml(config_dir, "master", "testuser", None, None, None)
            .await
            .unwrap();

        change_user_password_yaml(config_dir, "master", "testuser", "newpass123")
            .await
            .unwrap();

        let file_path = config_dir
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
        let config_dir = dir.path();

        create_client_yaml(config_dir, "master", "testclient", true)
            .await
            .unwrap();

        let file_path = config_dir
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
    async fn test_rotate_keys_yaml() {
        let dir = tempdir().unwrap();
        let config_dir = dir.path();

        let keys_dir = config_dir.join("master").join("components");
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

        let count = rotate_keys_yaml(config_dir, "master").await.unwrap();
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

async fn create_client_interactive(config_dir: &Path) -> Result<()> {
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

    create_client_yaml(config_dir, &realm, &client_id, is_public).await?;

    println!(
        "Successfully generated YAML for client '{}' in realm '{}'.",
        client_id, realm
    );
    Ok(())
}

pub async fn create_client_yaml(
    config_dir: &Path,
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

    let realm_dir = config_dir.join(realm).join("clients");
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

async fn change_user_password_interactive(config_dir: &Path) -> Result<()> {
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

    change_user_password_yaml(config_dir, &realm, &username, &new_password).await?;

    println!(
        "Successfully updated YAML for user '{}' in realm '{}' with new password.",
        username, realm
    );
    Ok(())
}

pub async fn change_user_password_yaml(
    config_dir: &Path,
    realm: &str,
    username: &str,
    new_password: &str,
) -> Result<()> {
    let file_path = config_dir
        .join(realm)
        .join("users")
        .join(format!("{}.yaml", username));

    if !file_path.exists() {
        println!(
            "Warning: User file {:?} does not exist. Creating a new one.",
            file_path
        );
        create_user_yaml(config_dir, realm, username, None, None, None).await?;
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
        // Find existing password credential to replace, or add new
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

async fn create_user_interactive(config_dir: &Path) -> Result<()> {
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
        config_dir,
        &realm,
        &username,
        email_opt,
        first_name_opt,
        last_name_opt,
    )
    .await?;

    println!(
        "Successfully generated YAML for user '{}' in realm '{}'.",
        username, realm
    );
    Ok(())
}

pub async fn create_user_yaml(
    config_dir: &Path,
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

    let realm_dir = config_dir.join(realm).join("users");
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
