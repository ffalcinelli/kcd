use crate::models::{
    AuthenticationFlowRepresentation, ClientRepresentation, ClientScopeRepresentation,
    ComponentRepresentation, GroupRepresentation, IdentityProviderRepresentation,
    RealmRepresentation, RequiredActionProviderRepresentation, RoleRepresentation,
    UserRepresentation,
};
use anyhow::{Context, Result};
use serde::de::DeserializeOwned;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

pub trait RequiredField {
    fn is_missing_or_empty(&self) -> bool;
}

impl RequiredField for String {
    fn is_missing_or_empty(&self) -> bool {
        self.is_empty()
    }
}

impl RequiredField for Option<String> {
    fn is_missing_or_empty(&self) -> bool {
        self.as_deref().unwrap_or("").is_empty()
    }
}

fn read_yaml_files<T: DeserializeOwned>(dir: &Path, file_type: &str) -> Result<Vec<(PathBuf, T)>> {
    let mut results = Vec::new();
    if dir.exists() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "yaml") {
                let content = fs::read_to_string(&path)
                    .context(format!("Failed to read {} file {:?}", file_type, path))?;
                let item: T = serde_yaml::from_str(&content)
                    .context(format!("Failed to parse {} file {:?}", file_type, path))?;
                results.push((path, item));
            }
        }
    }
    Ok(results)
}

pub fn run(input_dir: PathBuf, realms_to_validate: &[String]) -> Result<()> {
    if !input_dir.exists() {
        anyhow::bail!("Input directory {:?} does not exist", input_dir);
    }

    let realms = if realms_to_validate.is_empty() {
        let mut dirs = Vec::new();
        for entry in fs::read_dir(&input_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                dirs.push(entry.file_name().to_string_lossy().to_string());
            }
        }
        dirs
    } else {
        realms_to_validate.to_vec()
    };

    if realms.is_empty() {
        println!("No realms found to validate in {:?}", input_dir);
        return Ok(());
    }

    for realm_name in realms {
        println!("Validating realm: {}", realm_name);
        let realm_dir = input_dir.join(&realm_name);
        validate_realm(realm_dir)?;
    }
    Ok(())
}

fn validate_resources<T: DeserializeOwned>(
    input_dir: &Path,
    dir_name: &str,
    file_type: &str,
    success_msg: &str,
    validate_fn: impl Fn(&PathBuf, &T) -> Result<()>,
) -> Result<()> {
    let dir = input_dir.join(dir_name);
    let items: Vec<(PathBuf, T)> = read_yaml_files(&dir, file_type)?;
    for (path, item) in &items {
        validate_fn(path, item)?;
    }
    println!("{}", success_msg);
    Ok(())
}

fn validate_realm(input_dir: PathBuf) -> Result<()> {
    // 1. Validate Realm
    let realm_path = input_dir.join("realm.yaml");
    if !realm_path.exists() {
        anyhow::bail!("realm.yaml not found in {:?}", input_dir);
    }
    let realm_content = fs::read_to_string(&realm_path).context("Failed to read realm.yaml")?;
    let realm: RealmRepresentation =
        serde_yaml::from_str(&realm_content).context("Failed to parse realm.yaml")?;

    if realm.realm.is_missing_or_empty() {
        anyhow::bail!("Realm name is empty in realm.yaml");
    }
    println!("Realm configuration is valid: {}", realm.realm);

    // 2. Validate Roles
    let roles_dir = input_dir.join("roles");
    let mut role_names = HashSet::new();
    let roles: Vec<(PathBuf, RoleRepresentation)> = read_yaml_files(&roles_dir, "role")?;

    for (path, role) in roles {
        if role.name.is_missing_or_empty() {
            anyhow::bail!("Role name is empty in {:?}", path);
        }
        if role_names.contains(&role.name) {
            anyhow::bail!("Duplicate role name: {}", role.name);
        }
        role_names.insert(role.name.clone());
    }
    println!("Validated {} roles", role_names.len());

    // 3. Validate Clients
    validate_resources(
        &input_dir,
        "clients",
        "client",
        "Validated clients",
        |path, client: &ClientRepresentation| {
            if client.client_id.is_missing_or_empty() {
                anyhow::bail!("Client ID is missing or empty in {:?}", path);
            }
            Ok(())
        },
    )?;

    // 4. Validate Identity Providers
    validate_resources(
        &input_dir,
        "identity-providers",
        "idp",
        "Validated Identity Providers",
        |path, idp: &IdentityProviderRepresentation| {
            if idp.alias.is_missing_or_empty() {
                anyhow::bail!("Identity Provider alias is missing or empty in {:?}", path);
            }
            if idp.provider_id.is_missing_or_empty() {
                anyhow::bail!(
                    "Identity Provider providerId is missing or empty in {:?}",
                    path
                );
            }
            Ok(())
        },
    )?;

    // 5. Validate Client Scopes
    validate_resources(
        &input_dir,
        "client-scopes",
        "client-scope",
        "Validated client scopes",
        |path, scope: &ClientScopeRepresentation| {
            if scope.name.is_missing_or_empty() {
                anyhow::bail!("Client Scope name is missing or empty in {:?}", path);
            }
            Ok(())
        },
    )?;

    // 6. Validate Groups
    validate_resources(
        &input_dir,
        "groups",
        "group",
        "Validated groups",
        |path, group: &GroupRepresentation| {
            if group.name.is_missing_or_empty() {
                anyhow::bail!("Group name is missing or empty in {:?}", path);
            }
            Ok(())
        },
    )?;

    // 7. Validate Users
    validate_resources(
        &input_dir,
        "users",
        "user",
        "Validated users",
        |path, user: &UserRepresentation| {
            if user.username.is_missing_or_empty() {
                anyhow::bail!("User username is missing or empty in {:?}", path);
            }
            Ok(())
        },
    )?;

    // 8. Validate Authentication Flows
    validate_resources(
        &input_dir,
        "authentication-flows",
        "authentication-flow",
        "Validated authentication flows",
        |path, flow: &AuthenticationFlowRepresentation| {
            if flow.alias.is_missing_or_empty() {
                anyhow::bail!(
                    "Authentication Flow alias is missing or empty in {:?}",
                    path
                );
            }
            Ok(())
        },
    )?;

    // 9. Validate Required Actions
    validate_resources(
        &input_dir,
        "required-actions",
        "required-action",
        "Validated required actions",
        |path, action: &RequiredActionProviderRepresentation| {
            if action.alias.is_missing_or_empty() {
                anyhow::bail!("Required Action alias is missing or empty in {:?}", path);
            }
            if action.provider_id.is_missing_or_empty() {
                anyhow::bail!(
                    "Required Action providerId is missing or empty in {:?}",
                    path
                );
            }
            Ok(())
        },
    )?;

    // 10. Validate Components and Keys
    for dir_name in ["components", "keys"].iter() {
        if fs::exists(input_dir.join(dir_name))? {
            let singular = if *dir_name == "keys" { "key" } else { "component" };
            validate_resources(
                &input_dir,
                dir_name,
                singular,
                &format!("Validated {}", dir_name),
                |path, component: &ComponentRepresentation| {
                    if component.name.is_missing_or_empty() {
                        anyhow::bail!("Component name is missing or empty in {:?}", path);
                    }
                    if component.provider_id.is_missing_or_empty() {
                        anyhow::bail!("Component providerId is missing or empty in {:?}", path);
                    }
                    Ok(())
                },
            )?;
        }
    }

    Ok(())
}
