use crate::models::{
    AuthenticationFlowRepresentation, ClientRepresentation, ClientScopeRepresentation,
    ComponentRepresentation, GroupRepresentation, IdentityProviderRepresentation,
    RealmRepresentation, RequiredActionProviderRepresentation, RoleRepresentation,
    UserRepresentation,
};
use anyhow::{Context, Result};
use console::{Emoji, style};
use serde::de::DeserializeOwned;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

static CHECK: Emoji<'_, '_> = Emoji("✅ ", "√ ");
static SEARCH: Emoji<'_, '_> = Emoji("🔍 ", "> ");
static SUCCESS: Emoji<'_, '_> = Emoji("🎉 ", "* ");
static WARN: Emoji<'_, '_> = Emoji("⚠️ ", "! ");

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

pub fn run(workspace_dir: PathBuf, realms_to_validate: &[String]) -> Result<()> {
    if !workspace_dir.exists() {
        anyhow::bail!("Input directory {:?} does not exist", workspace_dir);
    }

    let realms = if realms_to_validate.is_empty() {
        let mut dirs = Vec::new();
        for entry in fs::read_dir(&workspace_dir)? {
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
        println!(
            "{} {}",
            WARN,
            style(format!(
                "No realms found to validate in {:?}",
                workspace_dir
            ))
            .yellow()
        );
        return Ok(());
    }

    for realm_name in &realms {
        println!(
            "\n{} {}",
            SEARCH,
            style(format!("Validating realm: {}", realm_name))
                .cyan()
                .bold()
        );
        let realm_dir = workspace_dir.join(realm_name);
        validate_realm(realm_dir)?;
        println!(
            "  {} {}",
            SUCCESS,
            style(format!("Successfully validated realm: {}", realm_name))
                .green()
                .bold()
        );
    }
    Ok(())
}

fn validate_realm(workspace_dir: PathBuf) -> Result<()> {
    // 1. Validate Realm
    let realm_path = workspace_dir.join("realm.yaml");
    if !realm_path.exists() {
        anyhow::bail!("realm.yaml not found in {:?}", workspace_dir);
    }
    let realm_content = fs::read_to_string(&realm_path).context("Failed to read realm.yaml")?;
    let realm: RealmRepresentation =
        serde_yaml::from_str(&realm_content).context("Failed to parse realm.yaml")?;

    if realm.realm.is_empty() {
        anyhow::bail!("Realm name is empty in realm.yaml");
    }
    println!(
        "  {} {} {}",
        CHECK,
        style("Realm configuration is valid:").dim(),
        style(&realm.realm).green()
    );

    // 2. Validate Roles
    let roles_dir = workspace_dir.join("roles");
    let mut role_names = HashSet::new();
    let roles: Vec<(PathBuf, RoleRepresentation)> = read_yaml_files(&roles_dir, "role")?;

    for (path, role) in &roles {
        if role.name.is_empty() {
            anyhow::bail!("Role name is empty in {:?}", path);
        }
        if role_names.contains(&role.name) {
            anyhow::bail!("Duplicate role name: {}", role.name);
        }
        role_names.insert(role.name.clone());
    }
    println!(
        "  {} {} {}",
        CHECK,
        style("Validated roles:").dim(),
        style(roles.len()).green()
    );

    // 3. Validate Clients
    let clients_dir = workspace_dir.join("clients");
    let clients: Vec<(PathBuf, ClientRepresentation)> = read_yaml_files(&clients_dir, "client")?;

    for (path, client) in &clients {
        if client.client_id.is_none() || client.client_id.as_deref().unwrap_or("").is_empty() {
            anyhow::bail!("Client ID is missing or empty in {:?}", path);
        }
    }
    println!(
        "  {} {} {}",
        CHECK,
        style("Validated clients:").dim(),
        style(clients.len()).green()
    );

    // 4. Validate Identity Providers
    let idps_dir = workspace_dir.join("identity-providers");
    let idps: Vec<(PathBuf, IdentityProviderRepresentation)> = read_yaml_files(&idps_dir, "idp")?;

    for (path, idp) in &idps {
        if idp.alias.is_none() || idp.alias.as_deref().unwrap_or("").is_empty() {
            anyhow::bail!("Identity Provider alias is missing or empty in {:?}", path);
        }
        if idp.provider_id.is_none() || idp.provider_id.as_deref().unwrap_or("").is_empty() {
            anyhow::bail!(
                "Identity Provider providerId is missing or empty in {:?}",
                path
            );
        }
    }
    println!(
        "  {} {} {}",
        CHECK,
        style("Validated Identity Providers:").dim(),
        style(idps.len()).green()
    );

    // 5. Validate Client Scopes
    let scopes_dir = workspace_dir.join("client-scopes");
    let scopes: Vec<(PathBuf, ClientScopeRepresentation)> =
        read_yaml_files(&scopes_dir, "client-scope")?;
    for (path, scope) in &scopes {
        if scope.name.as_deref().unwrap_or("").is_empty() {
            anyhow::bail!("Client Scope name is missing or empty in {:?}", path);
        }
    }
    println!(
        "  {} {} {}",
        CHECK,
        style("Validated client scopes:").dim(),
        style(scopes.len()).green()
    );

    // 6. Validate Groups
    let groups_dir = workspace_dir.join("groups");
    let groups: Vec<(PathBuf, GroupRepresentation)> = read_yaml_files(&groups_dir, "group")?;
    for (path, group) in &groups {
        if group.name.as_deref().unwrap_or("").is_empty() {
            anyhow::bail!("Group name is missing or empty in {:?}", path);
        }
    }
    println!(
        "  {} {} {}",
        CHECK,
        style("Validated groups:").dim(),
        style(groups.len()).green()
    );

    // 7. Validate Users
    let users_dir = workspace_dir.join("users");
    let users: Vec<(PathBuf, UserRepresentation)> = read_yaml_files(&users_dir, "user")?;
    for (path, user) in &users {
        if user.username.as_deref().unwrap_or("").is_empty() {
            anyhow::bail!("User username is missing or empty in {:?}", path);
        }
    }
    println!(
        "  {} {} {}",
        CHECK,
        style("Validated users:").dim(),
        style(users.len()).green()
    );

    // 8. Validate Authentication Flows
    let flows_dir = workspace_dir.join("authentication-flows");
    let flows: Vec<(PathBuf, AuthenticationFlowRepresentation)> =
        read_yaml_files(&flows_dir, "authentication-flow")?;
    for (path, flow) in &flows {
        if flow.alias.as_deref().unwrap_or("").is_empty() {
            anyhow::bail!(
                "Authentication Flow alias is missing or empty in {:?}",
                path
            );
        }
    }
    println!(
        "  {} {} {}",
        CHECK,
        style("Validated authentication flows:").dim(),
        style(flows.len()).green()
    );

    // 9. Validate Required Actions
    let actions_dir = workspace_dir.join("required-actions");
    let actions: Vec<(PathBuf, RequiredActionProviderRepresentation)> =
        read_yaml_files(&actions_dir, "required-action")?;
    for (path, action) in &actions {
        if action.alias.as_deref().unwrap_or("").is_empty() {
            anyhow::bail!("Required Action alias is missing or empty in {:?}", path);
        }
        if action.provider_id.as_deref().unwrap_or("").is_empty() {
            anyhow::bail!(
                "Required Action providerId is missing or empty in {:?}",
                path
            );
        }
    }
    println!(
        "  {} {} {}",
        CHECK,
        style("Validated required actions:").dim(),
        style(actions.len()).green()
    );

    // 10. Validate Components and Keys
    for dir_name in ["components", "keys"].iter() {
        let dir = workspace_dir.join(dir_name);
        if fs::exists(&dir)? {
            let components: Vec<(PathBuf, ComponentRepresentation)> =
                read_yaml_files(&dir, dir_name)?;
            for (path, component) in &components {
                if let Some(name) = &component.name
                    && name.is_empty()
                {
                    anyhow::bail!("Component name is empty in {:?}", path);
                }
                if component.provider_id.as_deref().unwrap_or("").is_empty() {
                    anyhow::bail!("Component providerId is missing or empty in {:?}", path);
                }
            }
            println!(
                "  {} {} {}",
                CHECK,
                style(format!("Validated {}:", dir_name)).dim(),
                style(components.len()).green()
            );
        }
    }

    Ok(())
}
