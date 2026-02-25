use crate::client::KeycloakClient;
use crate::utils::to_sorted_yaml;
use anyhow::{Context, Result};
use sanitize_filename::sanitize;
use std::path::PathBuf;
use tokio::fs;

pub async fn run(client: &KeycloakClient, output_dir: PathBuf) -> Result<()> {
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
    let realm_yaml = to_sorted_yaml(&realm).context("Failed to serialize realm")?;
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
    for client_rep in clients {
        let name = client_rep.client_id.as_deref().unwrap_or("unknown");
        let filename = format!("{}.yaml", sanitize(name));
        let path = clients_dir.join(filename);
        let yaml = to_sorted_yaml(&client_rep).context("Failed to serialize client")?;
        fs::write(&path, yaml)
            .await
            .context(format!("Failed to write client {}", name))?;
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
    for role in roles {
        let name = &role.name;
        let filename = format!("{}.yaml", sanitize(name));
        let path = roles_dir.join(filename);
        let yaml = to_sorted_yaml(&role).context("Failed to serialize role")?;
        fs::write(&path, yaml)
            .await
            .context(format!("Failed to write role {}", name))?;
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
    for scope in client_scopes {
        let name = scope.name.as_deref().unwrap_or("unknown");
        let filename = format!("{}.yaml", sanitize(name));
        let path = scopes_dir.join(filename);
        let yaml = to_sorted_yaml(&scope).context("Failed to serialize client scope")?;
        fs::write(&path, yaml)
            .await
            .context(format!("Failed to write client scope {}", name))?;
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
    for group in groups {
        let name = group.name.as_deref().unwrap_or("unknown");
        let filename = format!("{}.yaml", sanitize(name));
        let path = groups_dir.join(filename);
        let yaml = to_sorted_yaml(&group).context("Failed to serialize group")?;
        fs::write(&path, yaml)
            .await
            .context(format!("Failed to write group {}", name))?;
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
    for user in users {
        let username = user.username.as_deref().unwrap_or("unknown");
        let filename = format!("{}.yaml", sanitize(username));
        let path = users_dir.join(filename);
        let yaml = to_sorted_yaml(&user).context("Failed to serialize user")?;
        fs::write(&path, yaml)
            .await
            .context(format!("Failed to write user {}", username))?;
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
    for flow in flows {
        let alias = flow.alias.as_deref().unwrap_or("unknown");
        let filename = format!("{}.yaml", sanitize(alias));
        let path = flows_dir.join(filename);
        let yaml = to_sorted_yaml(&flow).context("Failed to serialize authentication flow")?;
        fs::write(&path, yaml)
            .await
            .context(format!("Failed to write authentication flow {}", alias))?;
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
    for action in actions {
        let alias = action.alias.as_deref().unwrap_or("unknown");
        let filename = format!("{}.yaml", sanitize(alias));
        let path = actions_dir.join(filename);
        let yaml = to_sorted_yaml(&action).context("Failed to serialize required action")?;
        fs::write(&path, yaml)
            .await
            .context(format!("Failed to write required action {}", alias))?;
    }
    println!("Exported required actions to required-actions/");

    // Fetch components
    let components = client
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
    for component in components {
        let name = component.name.as_deref().unwrap_or("unknown");
        let filename = format!("{}.yaml", sanitize(name));
        let path = components_dir.join(filename);
        let yaml = to_sorted_yaml(&component).context("Failed to serialize component")?;
        fs::write(&path, yaml)
            .await
            .context(format!("Failed to write component {}", name))?;
    }
    println!("Exported components to components/");

    Ok(())
}
