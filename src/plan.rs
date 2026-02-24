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
use std::fs;
use std::path::PathBuf;

pub async fn run(client: &KeycloakClient, input_dir: PathBuf) -> Result<()> {
    println!(
        "{} Planning changes for realm: {}",
        Emoji("🔮", ""),
        client.target_realm
    );

    // 1. Plan Realm
    plan_realm(client, &input_dir).await?;

    // 2. Plan Roles
    plan_roles(client, &input_dir).await?;

    // 3. Plan Clients
    plan_clients(client, &input_dir).await?;

    // 4. Plan Identity Providers
    plan_identity_providers(client, &input_dir).await?;

    // 5. Plan Client Scopes
    plan_client_scopes(client, &input_dir).await?;

    // 6. Plan Groups
    plan_groups(client, &input_dir).await?;

    // 7. Plan Users
    plan_users(client, &input_dir).await?;

    // 8. Plan Authentication Flows
    plan_authentication_flows(client, &input_dir).await?;

    // 9. Plan Required Actions
    plan_required_actions(client, &input_dir).await?;

    // 10. Plan Components
    plan_components(client, &input_dir).await?;

    Ok(())
}

fn print_diff<T: Serialize>(name: &str, old: Option<&T>, new: &T) -> Result<()> {
    let old_yaml = if let Some(o) = old {
        serde_yaml::to_string(o)?
    } else {
        String::new()
    };
    let new_yaml = serde_yaml::to_string(new)?;

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
    } else {
        println!("{} No changes for {}", Emoji("✅", ""), name);
    }
    Ok(())
}

async fn plan_client_scopes(client: &KeycloakClient, input_dir: &std::path::Path) -> Result<()> {
    let scopes_dir = input_dir.join("client-scopes");
    if scopes_dir.exists() {
        let existing_scopes = client.get_client_scopes().await.unwrap_or_default();
        let existing_scopes_map: HashMap<String, ClientScopeRepresentation> = existing_scopes
            .into_iter()
            .filter_map(|s| s.name.clone().map(|n| (n, s)))
            .collect();

        for entry in fs::read_dir(&scopes_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "yaml") {
                let content = fs::read_to_string(&path)?;
                let local_scope: ClientScopeRepresentation = serde_yaml::from_str(&content)?;
                let name = local_scope.name.as_deref().unwrap_or("");

                if name.is_empty() {
                    continue;
                }

                if let Some(remote) = existing_scopes_map.get(name) {
                    let mut remote_clone = remote.clone();
                    if local_scope.id.is_none() {
                        remote_clone.id = None;
                    }
                    print_diff(
                        &format!("ClientScope {}", name),
                        Some(&remote_clone),
                        &local_scope,
                    )?;
                } else {
                    println!("\n{} Will create ClientScope: {}", Emoji("✨", ""), name);
                    print_diff(
                        &format!("ClientScope {}", name),
                        None::<&ClientScopeRepresentation>,
                        &local_scope,
                    )?;
                }
            }
        }
    }
    Ok(())
}

async fn plan_groups(client: &KeycloakClient, input_dir: &std::path::Path) -> Result<()> {
    let groups_dir = input_dir.join("groups");
    if groups_dir.exists() {
        let existing_groups = client.get_groups().await.unwrap_or_default();
        let existing_groups_map: HashMap<String, GroupRepresentation> = existing_groups
            .into_iter()
            .filter_map(|g| g.name.clone().map(|n| (n, g)))
            .collect();

        for entry in fs::read_dir(&groups_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "yaml") {
                let content = fs::read_to_string(&path)?;
                let local_group: GroupRepresentation = serde_yaml::from_str(&content)?;
                let name = local_group.name.as_deref().unwrap_or("");

                if name.is_empty() {
                    continue;
                }

                if let Some(remote) = existing_groups_map.get(name) {
                    let mut remote_clone = remote.clone();
                    if local_group.id.is_none() {
                        remote_clone.id = None;
                    }
                    print_diff(
                        &format!("Group {}", name),
                        Some(&remote_clone),
                        &local_group,
                    )?;
                } else {
                    println!("\n{} Will create Group: {}", Emoji("✨", ""), name);
                    print_diff(
                        &format!("Group {}", name),
                        None::<&GroupRepresentation>,
                        &local_group,
                    )?;
                }
            }
        }
    }
    Ok(())
}

async fn plan_users(client: &KeycloakClient, input_dir: &std::path::Path) -> Result<()> {
    let users_dir = input_dir.join("users");
    if users_dir.exists() {
        let existing_users = client.get_users().await.unwrap_or_default();
        let existing_users_map: HashMap<String, UserRepresentation> = existing_users
            .into_iter()
            .filter_map(|u| u.username.clone().map(|n| (n, u)))
            .collect();

        for entry in fs::read_dir(&users_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "yaml") {
                let content = fs::read_to_string(&path)?;
                let local_user: UserRepresentation = serde_yaml::from_str(&content)?;
                let username = local_user.username.as_deref().unwrap_or("");

                if username.is_empty() {
                    continue;
                }

                if let Some(remote) = existing_users_map.get(username) {
                    let mut remote_clone = remote.clone();
                    if local_user.id.is_none() {
                        remote_clone.id = None;
                    }
                    print_diff(
                        &format!("User {}", username),
                        Some(&remote_clone),
                        &local_user,
                    )?;
                } else {
                    println!("\n{} Will create User: {}", Emoji("✨", ""), username);
                    print_diff(
                        &format!("User {}", username),
                        None::<&UserRepresentation>,
                        &local_user,
                    )?;
                }
            }
        }
    }
    Ok(())
}

async fn plan_authentication_flows(client: &KeycloakClient, input_dir: &std::path::Path) -> Result<()> {
    let flows_dir = input_dir.join("authentication-flows");
    if flows_dir.exists() {
        let existing_flows = client.get_authentication_flows().await.unwrap_or_default();
        let existing_flows_map: HashMap<String, AuthenticationFlowRepresentation> = existing_flows
            .into_iter()
            .filter_map(|f| f.alias.clone().map(|a| (a, f)))
            .collect();

        for entry in fs::read_dir(&flows_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "yaml") {
                let content = fs::read_to_string(&path)?;
                let local_flow: AuthenticationFlowRepresentation = serde_yaml::from_str(&content)?;
                let alias = local_flow.alias.as_deref().unwrap_or("");

                if alias.is_empty() {
                    continue;
                }

                if let Some(remote) = existing_flows_map.get(alias) {
                    let mut remote_clone = remote.clone();
                    if local_flow.id.is_none() {
                        remote_clone.id = None;
                    }
                    print_diff(
                        &format!("AuthenticationFlow {}", alias),
                        Some(&remote_clone),
                        &local_flow,
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
                    )?;
                }
            }
        }
    }
    Ok(())
}

async fn plan_required_actions(client: &KeycloakClient, input_dir: &std::path::Path) -> Result<()> {
    let actions_dir = input_dir.join("required-actions");
    if actions_dir.exists() {
        let existing_actions = client.get_required_actions().await.unwrap_or_default();
        let existing_actions_map: HashMap<String, RequiredActionProviderRepresentation> =
            existing_actions
                .into_iter()
                .filter_map(|a| a.alias.clone().map(|n| (n, a)))
                .collect();

        for entry in fs::read_dir(&actions_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "yaml") {
                let content = fs::read_to_string(&path)?;
                let local_action: RequiredActionProviderRepresentation =
                    serde_yaml::from_str(&content)?;
                let alias = local_action.alias.as_deref().unwrap_or("");

                if alias.is_empty() {
                    continue;
                }

                if let Some(remote) = existing_actions_map.get(alias) {
                    print_diff(
                        &format!("RequiredAction {}", alias),
                        Some(remote),
                        &local_action,
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
                    )?;
                }
            }
        }
    }
    Ok(())
}

async fn plan_components(client: &KeycloakClient, input_dir: &std::path::Path) -> Result<()> {
    let components_dir = input_dir.join("components");
    if components_dir.exists() {
        let existing_components = client.get_components().await.unwrap_or_default();
        let existing_components_map: HashMap<String, ComponentRepresentation> = existing_components
            .into_iter()
            .filter_map(|c| c.name.clone().map(|n| (n, c)))
            .collect();

        for entry in fs::read_dir(&components_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "yaml") {
                let content = fs::read_to_string(&path)?;
                let local_component: ComponentRepresentation = serde_yaml::from_str(&content)?;
                let name = local_component.name.as_deref().unwrap_or("");

                if name.is_empty() {
                    continue;
                }

                if let Some(remote) = existing_components_map.get(name) {
                    let mut remote_clone = remote.clone();
                    if local_component.id.is_none() {
                        remote_clone.id = None;
                    }
                    print_diff(
                        &format!("Component {}", name),
                        Some(&remote_clone),
                        &local_component,
                    )?;
                } else {
                    println!("\n{} Will create Component: {}", Emoji("✨", ""), name);
                    print_diff(
                        &format!("Component {}", name),
                        None::<&ComponentRepresentation>,
                        &local_component,
                    )?;
                }
            }
        }
    }
    Ok(())
}

async fn plan_realm(client: &KeycloakClient, input_dir: &std::path::Path) -> Result<()> {
    let realm_path = input_dir.join("realm.yaml");
    if realm_path.exists() {
        let content = fs::read_to_string(&realm_path)?;
        let local_realm: RealmRepresentation = serde_yaml::from_str(&content)?;

        // We handle the case where remote realm fetch might fail (e.g. if we are creating it)
        // by treating it as None (creation). However, usually plan is run against existing realm.
        // If get_realm fails, it might be an error or not exist.
        // For plan, we assume if it fails, it might not exist or we can't access it.
        // Let's try to fetch it.
        let remote_realm = client.get_realm().await.ok();

        print_diff("Realm", remote_realm.as_ref(), &local_realm)?;
    }
    Ok(())
}

async fn plan_roles(client: &KeycloakClient, input_dir: &std::path::Path) -> Result<()> {
    let roles_dir = input_dir.join("roles");
    if roles_dir.exists() {
        let existing_roles = client.get_roles().await.unwrap_or_default();
        let existing_roles_map: HashMap<String, RoleRepresentation> = existing_roles
            .into_iter()
            .map(|r| (r.name.clone(), r))
            .collect();

        for entry in fs::read_dir(&roles_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "yaml") {
                let content = fs::read_to_string(&path)?;
                let local_role: RoleRepresentation = serde_yaml::from_str(&content)?;

                let remote_role = existing_roles_map.get(&local_role.name);

                if let Some(remote) = remote_role {
                    let mut remote_clone = remote.clone();
                    // Ignore ID differences if local doesn't specify it
                    if local_role.id.is_none() {
                        remote_clone.id = None;
                        remote_clone.container_id = None;
                    }
                    print_diff(
                        &format!("Role {}", local_role.name),
                        Some(&remote_clone),
                        &local_role,
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
                    )?;
                }
            }
        }
    }
    Ok(())
}

async fn plan_clients(client: &KeycloakClient, input_dir: &std::path::Path) -> Result<()> {
    let clients_dir = input_dir.join("clients");
    if clients_dir.exists() {
        let existing_clients = client.get_clients().await.unwrap_or_default();
        let existing_clients_map: HashMap<String, ClientRepresentation> = existing_clients
            .into_iter()
            .filter_map(|c| c.client_id.clone().map(|id| (id, c)))
            .collect();

        for entry in fs::read_dir(&clients_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "yaml") {
                let content = fs::read_to_string(&path)?;
                let local_client: ClientRepresentation = serde_yaml::from_str(&content)?;
                let client_id = local_client.client_id.as_deref().unwrap_or("");

                if client_id.is_empty() {
                    continue;
                }

                if let Some(remote) = existing_clients_map.get(client_id) {
                    let mut remote_clone = remote.clone();
                    if local_client.id.is_none() {
                        remote_clone.id = None;
                    }
                    print_diff(
                        &format!("Client {}", client_id),
                        Some(&remote_clone),
                        &local_client,
                    )?;
                } else {
                    println!("\n{} Will create Client: {}", Emoji("✨", ""), client_id);
                    print_diff(
                        &format!("Client {}", client_id),
                        None::<&ClientRepresentation>,
                        &local_client,
                    )?;
                }
            }
        }
    }
    Ok(())
}

async fn plan_identity_providers(client: &KeycloakClient, input_dir: &std::path::Path) -> Result<()> {
    let idps_dir = input_dir.join("identity-providers");
    if idps_dir.exists() {
        let existing_idps = client.get_identity_providers().await.unwrap_or_default();
        let existing_idps_map: HashMap<String, IdentityProviderRepresentation> = existing_idps
            .into_iter()
            .filter_map(|i| i.alias.clone().map(|alias| (alias, i)))
            .collect();

        for entry in fs::read_dir(&idps_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "yaml") {
                let content = fs::read_to_string(&path)?;
                let local_idp: IdentityProviderRepresentation = serde_yaml::from_str(&content)?;
                let alias = local_idp.alias.as_deref().unwrap_or("");

                if alias.is_empty() {
                    continue;
                }

                if let Some(remote) = existing_idps_map.get(alias) {
                    let mut remote_clone = remote.clone();
                    if local_idp.internal_id.is_none() {
                        remote_clone.internal_id = None;
                    }
                    print_diff(
                        &format!("IdentityProvider {}", alias),
                        Some(&remote_clone),
                        &local_idp,
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
                    )?;
                }
            }
        }
    }
    Ok(())
}
