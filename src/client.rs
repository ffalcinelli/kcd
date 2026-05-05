use crate::models::{
    AuthenticationFlowRepresentation, ClientRepresentation, ClientScopeRepresentation,
    ComponentRepresentation, GroupRepresentation, IdentityProviderRepresentation, KeycloakResource,
    RealmRepresentation, RequiredActionProviderRepresentation, RoleRepresentation,
    UserRepresentation,
};
use anyhow::{Context, Result};
use log::{debug, info};
use reqwest::{Client, Response};
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct KeycloakClient {
    client: Client,
    base_url: String,
    pub target_realm: String, // The realm we are managing
    token: Option<String>,
}

#[derive(Deserialize, Debug)]
struct TokenResponse {
    access_token: String,
}

impl KeycloakClient {
    pub fn new(base_url: String) -> Self {
        let target_realm = "".to_string();
        let base_url = base_url.trim_end_matches('/').to_string();
        Self {
            client: Client::new(),
            base_url,
            target_realm,
            token: None,
        }
    }

    pub fn set_target_realm(&mut self, target_realm: String) {
        self.target_realm = target_realm;
    }

    fn realm_admin_url(&self) -> String {
        format!("{}/admin/realms/{}", self.base_url, self.target_realm)
    }

    fn resource_url<T: KeycloakResource>(&self) -> String {
        if T::api_path() == "realms" {
            format!("{}/admin/realms", self.base_url)
        } else {
            format!("{}/{}", self.realm_admin_url(), T::api_path())
        }
    }

    fn object_url<T: KeycloakResource>(&self, id: &str) -> String {
        if T::api_path() == "realms" {
            format!("{}/admin/realms/{}", self.base_url, id)
        } else {
            format!("{}/{}", self.realm_admin_url(), T::object_path(id))
        }
    }

    pub async fn get_resources<T: KeycloakResource + for<'a> Deserialize<'a>>(
        &self,
    ) -> Result<Vec<T>> {
        self.get(&self.resource_url::<T>()).await
    }

    pub async fn get_resource<T: KeycloakResource + for<'a> Deserialize<'a>>(
        &self,
        id: &str,
    ) -> Result<T> {
        self.get(&self.object_url::<T>(id)).await
    }

    pub async fn create_resource<T: KeycloakResource + Serialize>(&self, res: &T) -> Result<()> {
        self.post(&self.resource_url::<T>(), res).await
    }

    pub async fn update_resource<T: KeycloakResource + Serialize>(
        &self,
        id: &str,
        res: &T,
    ) -> Result<()> {
        self.put(&self.object_url::<T>(id), res).await
    }

    pub async fn delete_resource<T: KeycloakResource>(&self, id: &str) -> Result<()> {
        self.delete(&self.object_url::<T>(id)).await
    }

    pub async fn get_realms(&self) -> Result<Vec<RealmRepresentation>> {
        self.get_resources().await
    }

    pub async fn get_realm(&self) -> Result<RealmRepresentation> {
        self.get_resource(&self.target_realm).await
    }

    pub async fn get_clients(&self) -> Result<Vec<ClientRepresentation>> {
        self.get_resources().await
    }

    pub async fn get_roles(&self) -> Result<Vec<RoleRepresentation>> {
        self.get_resources().await
    }

    pub async fn get_identity_providers(&self) -> Result<Vec<IdentityProviderRepresentation>> {
        self.get_resources().await
    }

    /// Updates the target realm representation, passing the realm string by reference to avoid allocations.
    pub async fn update_realm(&self, realm_rep: &RealmRepresentation) -> Result<()> {
        self.update_resource(&self.target_realm, realm_rep).await
    }

    pub async fn create_client(&self, client_rep: &ClientRepresentation) -> Result<()> {
        self.create_resource(client_rep).await
    }

    pub async fn update_client(&self, id: &str, client_rep: &ClientRepresentation) -> Result<()> {
        self.update_resource(id, client_rep).await
    }

    pub async fn delete_client(&self, id: &str) -> Result<()> {
        self.delete_resource::<ClientRepresentation>(id).await
    }

    pub async fn create_role(&self, role_rep: &RoleRepresentation) -> Result<()> {
        self.create_resource(role_rep).await
    }

    pub async fn update_role(&self, id: &str, role_rep: &RoleRepresentation) -> Result<()> {
        self.update_resource(id, role_rep).await
    }

    pub async fn delete_role(&self, id: &str) -> Result<()> {
        self.delete_resource::<RoleRepresentation>(id).await
    }

    pub async fn create_identity_provider(
        &self,
        idp_rep: &IdentityProviderRepresentation,
    ) -> Result<()> {
        self.create_resource(idp_rep).await
    }

    pub async fn update_identity_provider(
        &self,
        alias: &str,
        idp_rep: &IdentityProviderRepresentation,
    ) -> Result<()> {
        self.update_resource(alias, idp_rep).await
    }

    pub async fn delete_identity_provider(&self, alias: &str) -> Result<()> {
        self.delete_resource::<IdentityProviderRepresentation>(alias)
            .await
    }

    pub async fn get_client_scopes(&self) -> Result<Vec<ClientScopeRepresentation>> {
        self.get_resources().await
    }

    pub async fn create_client_scope(&self, scope_rep: &ClientScopeRepresentation) -> Result<()> {
        self.create_resource(scope_rep).await
    }

    pub async fn update_client_scope(
        &self,
        id: &str,
        scope_rep: &ClientScopeRepresentation,
    ) -> Result<()> {
        self.update_resource(id, scope_rep).await
    }

    pub async fn delete_client_scope(&self, id: &str) -> Result<()> {
        self.delete_resource::<ClientScopeRepresentation>(id).await
    }

    pub async fn get_groups(&self) -> Result<Vec<GroupRepresentation>> {
        self.get_resources().await
    }

    pub async fn create_group(&self, group_rep: &GroupRepresentation) -> Result<()> {
        self.create_resource(group_rep).await
    }

    pub async fn update_group(&self, id: &str, group_rep: &GroupRepresentation) -> Result<()> {
        self.update_resource(id, group_rep).await
    }

    pub async fn delete_group(&self, id: &str) -> Result<()> {
        self.delete_resource::<GroupRepresentation>(id).await
    }

    pub async fn get_users(&self) -> Result<Vec<UserRepresentation>> {
        self.get_resources().await
    }

    pub async fn create_user(&self, user_rep: &UserRepresentation) -> Result<()> {
        self.create_resource(user_rep).await
    }

    pub async fn update_user(&self, id: &str, user_rep: &UserRepresentation) -> Result<()> {
        self.update_resource(id, user_rep).await
    }

    pub async fn delete_user(&self, id: &str) -> Result<()> {
        self.delete_resource::<UserRepresentation>(id).await
    }

    pub async fn get_authentication_flows(&self) -> Result<Vec<AuthenticationFlowRepresentation>> {
        self.get_resources().await
    }

    pub async fn create_authentication_flow(
        &self,
        flow_rep: &AuthenticationFlowRepresentation,
    ) -> Result<()> {
        self.create_resource(flow_rep).await
    }

    pub async fn update_authentication_flow(
        &self,
        id: &str,
        flow_rep: &AuthenticationFlowRepresentation,
    ) -> Result<()> {
        self.update_resource(id, flow_rep).await
    }

    pub async fn delete_authentication_flow(&self, id: &str) -> Result<()> {
        self.delete_resource::<AuthenticationFlowRepresentation>(id)
            .await
    }

    pub async fn get_required_actions(&self) -> Result<Vec<RequiredActionProviderRepresentation>> {
        self.get_resources().await
    }

    pub async fn update_required_action(
        &self,
        alias: &str,
        action_rep: &RequiredActionProviderRepresentation,
    ) -> Result<()> {
        self.update_resource(alias, action_rep).await
    }

    pub async fn register_required_action(
        &self,
        action_rep: &RequiredActionProviderRepresentation,
    ) -> Result<()> {
        let url = self.realm_admin_url() + "/authentication/register-required-action";

        #[derive(Serialize)]
        struct RegisterActionBody<'a> {
            #[serde(rename = "providerId")]
            provider_id: &'a str,
            name: &'a str,
        }

        let provider_id = action_rep
            .provider_id
            .as_deref()
            .context("Provider ID required for registration")?;
        let name = action_rep.name.as_deref().unwrap_or(provider_id);

        let body = RegisterActionBody { provider_id, name };
        self.post(&url, &body).await
    }

    pub async fn delete_required_action(&self, alias: &str) -> Result<()> {
        self.delete_resource::<RequiredActionProviderRepresentation>(alias)
            .await
    }

    pub async fn get_components(&self) -> Result<Vec<ComponentRepresentation>> {
        self.get_resources().await
    }

    pub async fn create_component(&self, component_rep: &ComponentRepresentation) -> Result<()> {
        self.create_resource(component_rep).await
    }

    pub async fn update_component(
        &self,
        id: &str,
        component_rep: &ComponentRepresentation,
    ) -> Result<()> {
        self.update_resource(id, component_rep).await
    }

    pub async fn delete_component(&self, id: &str) -> Result<()> {
        self.delete_resource::<ComponentRepresentation>(id).await
    }

    async fn get<T: for<'a> Deserialize<'a>>(&self, url: &str) -> Result<T> {
        let token = self.get_token()?;
        debug!("GET {}", redact_url(url));
        let response = self
            .client
            .get(url)
            .bearer_auth(token)
            .send()
            .await
            .context(format!("Failed to send GET request to {}", redact_url(url)))?;

        let response = Self::check_response(response, "GET request failed").await?;

        response.json().await.context("Failed to parse response")
    }

    async fn post<T: Serialize>(&self, url: &str, body: &T) -> Result<()> {
        let token = self.get_token()?;
        debug!("POST {}", redact_url(url));
        let response = self
            .client
            .post(url)
            .bearer_auth(token)
            .json(body)
            .send()
            .await
            .context(format!(
                "Failed to send POST request to {}",
                redact_url(url)
            ))?;

        Self::check_response(response, "POST request failed").await?;
        Ok(())
    }

    async fn put<T: Serialize>(&self, url: &str, body: &T) -> Result<()> {
        let token = self.get_token()?;
        debug!("PUT {}", redact_url(url));
        let response = self
            .client
            .put(url)
            .bearer_auth(token)
            .json(body)
            .send()
            .await
            .context(format!("Failed to send PUT request to {}", redact_url(url)))?;

        Self::check_response(response, "PUT request failed").await?;
        Ok(())
    }

    async fn delete(&self, url: &str) -> Result<()> {
        let token = self.get_token()?;
        debug!("DELETE {}", redact_url(url));
        let response = self
            .client
            .delete(url)
            .bearer_auth(token)
            .send()
            .await
            .context(format!(
                "Failed to send DELETE request to {}",
                redact_url(url)
            ))?;

        Self::check_response(response, "DELETE request failed").await?;
        Ok(())
    }

    pub async fn login(
        &mut self,
        client_id: &str,
        client_secret: Option<&str>,
        username: Option<&str>,
        password: Option<&str>,
    ) -> Result<()> {
        // We auth against the master realm usually for admin tasks, or the specific realm if using client credentials for a client in that realm.
        // Assuming admin-cli in master realm for now as default.
        let auth_realm = "master";
        let url = format!(
            "{}/realms/{}/protocol/openid-connect/token",
            self.base_url, auth_realm
        );

        let mut params = Vec::new();
        params.push(("client_id", client_id));

        if let (Some(u), Some(p)) = (username, password) {
            params.push(("username", u));
            params.push(("password", p));
            params.push(("grant_type", "password"));
        } else if let Some(s) = client_secret {
            params.push(("client_secret", s));
            params.push(("grant_type", "client_credentials"));
        } else {
            anyhow::bail!("Either username/password or client_secret must be provided");
        }

        debug!("Logging in to {}", redact_url(&url));

        let response = self
            .client
            .post(&url)
            .form(&params)
            .send()
            .await
            .context("Failed to send login request")?;

        let response = Self::check_response(response, "Login failed").await?;

        let token_response: TokenResponse = response
            .json()
            .await
            .context("Failed to parse token response")?;
        self.token = Some(token_response.access_token);

        info!("Successfully logged in to Keycloak");
        Ok(())
    }

    pub fn get_token(&self) -> Result<&str> {
        self.token.as_deref().context("Not authenticated")
    }

    pub fn set_token(&mut self, token: String) {
        self.token = Some(token);
    }

    async fn check_response(response: Response, context_msg: &str) -> Result<Response> {
        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            anyhow::bail!("{}: {} - {}", context_msg, status, text);
        }
        Ok(response)
    }
}

fn redact_url(url_str: &str) -> String {
    match reqwest::Url::parse(url_str) {
        Ok(mut url) => {
            if !url.username().is_empty() || url.password().is_some() {
                let _ = url.set_username("");
                let _ = url.set_password(None);
            }
            url.to_string()
        }
        Err(_) => {
            if let Some(pos) = url_str.rfind('@') {
                format!("<redacted>@{}", &url_str[pos + 1..])
            } else {
                url_str.to_string()
            }
        }
    }
}

impl KeycloakClient {
    pub async fn get_keys(&self) -> Result<crate::models::KeysMetadataRepresentation> {
        let url = self.realm_admin_url() + "/keys";
        self.get(&url).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_token() {
        let mut client = KeycloakClient::new("http://127.0.0.1:1".to_string());

        // Initially, there's no token
        let result = client.get_token();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "Not authenticated");

        // Set token
        client.set_token("mock_token".to_string());

        // After setting token, we can get it
        let result = client.get_token();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "mock_token");
    }

    #[test]
    fn test_redact_url() {
        assert_eq!(
            redact_url("http://localhost:8080"),
            "http://localhost:8080/"
        );
        assert_eq!(
            redact_url("http://user:pass@localhost:8080/path"),
            "http://localhost:8080/path"
        );
        assert_eq!(
            redact_url("http://user@localhost:8080/path"),
            "http://localhost:8080/path"
        );
        assert_eq!(redact_url("invalid-url"), "invalid-url");
        assert_eq!(
            redact_url("https://user:password@example.com:99999"),
            "<redacted>@example.com:99999"
        );
    }

    #[tokio::test]
    async fn test_post_send_failure() {
        let mut client = KeycloakClient::new("http://127.0.0.1:1".to_string());
        client.token = Some("mock_token".to_string());
        let result = client.post("http://127.0.0.1:1", &"body").await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Failed to send POST request")
        );
    }

    #[tokio::test]
    async fn test_delete_send_failure() {
        let mut client = KeycloakClient::new("http://127.0.0.1:1".to_string());
        client.token = Some("mock_token".to_string());
        let result = client.delete("http://127.0.0.1:1").await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Failed to send DELETE request")
        );
    }

    #[tokio::test]
    async fn test_get_send_failure() {
        let mut client = KeycloakClient::new("http://127.0.0.1:1".to_string());
        client.token = Some("mock_token".to_string());
        let result = client.get::<serde_json::Value>("http://127.0.0.1:1").await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Failed to send GET request")
        );
    }

    #[tokio::test]
    async fn test_put_send_failure() {
        let mut client = KeycloakClient::new("http://127.0.0.1:1".to_string());
        client.token = Some("mock_token".to_string());
        let result = client.put("http://127.0.0.1:1", &"body").await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Failed to send PUT request")
        );
    }
}
