use reqwest::Client;
use serde::{Deserialize, Serialize};
use anyhow::{Result, Context};
use log::{info, debug};
use crate::models::{RealmRepresentation, ClientRepresentation, RoleRepresentation};

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
    pub fn new(base_url: String, target_realm: String) -> Self {
        let base_url = base_url.trim_end_matches('/').to_string();
        Self {
            client: Client::new(),
            base_url,
            target_realm,
            token: None,
        }
    }

    pub async fn get_realm(&self) -> Result<RealmRepresentation> {
        let url = format!("{}/admin/realms/{}", self.base_url, self.target_realm);
        self.get(&url).await
    }

    pub async fn get_clients(&self) -> Result<Vec<ClientRepresentation>> {
        let url = format!("{}/admin/realms/{}/clients", self.base_url, self.target_realm);
        self.get(&url).await
    }

    pub async fn get_roles(&self) -> Result<Vec<RoleRepresentation>> {
         let url = format!("{}/admin/realms/{}/roles", self.base_url, self.target_realm);
         self.get(&url).await
    }

    pub async fn update_realm(&self, realm_rep: &RealmRepresentation) -> Result<()> {
        let url = format!("{}/admin/realms/{}", self.base_url, self.target_realm);
        self.put(&url, realm_rep).await
    }

    pub async fn create_client(&self, client_rep: &ClientRepresentation) -> Result<()> {
        let url = format!("{}/admin/realms/{}/clients", self.base_url, self.target_realm);
        self.post(&url, client_rep).await
    }

    pub async fn update_client(&self, id: &str, client_rep: &ClientRepresentation) -> Result<()> {
        let url = format!("{}/admin/realms/{}/clients/{}", self.base_url, self.target_realm, id);
        self.put(&url, client_rep).await
    }

    pub async fn delete_client(&self, id: &str) -> Result<()> {
        let url = format!("{}/admin/realms/{}/clients/{}", self.base_url, self.target_realm, id);
        self.delete(&url).await
    }

    pub async fn create_role(&self, role_rep: &RoleRepresentation) -> Result<()> {
        let url = format!("{}/admin/realms/{}/roles", self.base_url, self.target_realm);
        self.post(&url, role_rep).await
    }

    pub async fn update_role(&self, id: &str, role_rep: &RoleRepresentation) -> Result<()> {
         // Keycloak API for updating role by ID: PUT /admin/realms/{realm}/roles-by-id/{role-id}
         let url = format!("{}/admin/realms/{}/roles-by-id/{}", self.base_url, self.target_realm, id);
         self.put(&url, role_rep).await
    }

    pub async fn delete_role(&self, id: &str) -> Result<()> {
         // Keycloak API for deleting role by ID: DELETE /admin/realms/{realm}/roles-by-id/{role-id}
         let url = format!("{}/admin/realms/{}/roles-by-id/{}", self.base_url, self.target_realm, id);
         self.delete(&url).await
    }

    async fn get<T: for<'a> Deserialize<'a>>(&self, url: &str) -> Result<T> {
        let token = self.get_token()?;
        debug!("GET {}", redact_url(url));
        let response = self.client.get(url)
            .bearer_auth(token)
            .send()
            .await
            .context(format!("Failed to send GET request to {}", redact_url(url)))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
             anyhow::bail!("GET request failed: {} - {}", status, text);
        }

        response.json().await.context("Failed to parse response")
    }

    async fn post<T: Serialize>(&self, url: &str, body: &T) -> Result<()> {
        let token = self.get_token()?;
        debug!("POST {}", redact_url(url));
        let response = self.client.post(url)
            .bearer_auth(token)
            .json(body)
            .send()
            .await
            .context(format!("Failed to send POST request to {}", redact_url(url)))?;

        if !response.status().is_success() {
             let status = response.status();
             let text = response.text().await.unwrap_or_default();
             anyhow::bail!("POST request failed: {} - {}", status, text);
        }
        Ok(())
    }

    async fn put<T: Serialize>(&self, url: &str, body: &T) -> Result<()> {
        let token = self.get_token()?;
        debug!("PUT {}", redact_url(url));
        let response = self.client.put(url)
            .bearer_auth(token)
            .json(body)
            .send()
            .await
            .context(format!("Failed to send PUT request to {}", redact_url(url)))?;

        if !response.status().is_success() {
             let status = response.status();
             let text = response.text().await.unwrap_or_default();
             anyhow::bail!("PUT request failed: {} - {}", status, text);
        }
        Ok(())
    }

    async fn delete(&self, url: &str) -> Result<()> {
        let token = self.get_token()?;
        debug!("DELETE {}", redact_url(url));
        let response = self.client.delete(url)
            .bearer_auth(token)
            .send()
            .await
            .context(format!("Failed to send DELETE request to {}", redact_url(url)))?;

        if !response.status().is_success() {
             let status = response.status();
             let text = response.text().await.unwrap_or_default();
             anyhow::bail!("DELETE request failed: {} - {}", status, text);
        }
        Ok(())
    }

    pub async fn login(&mut self, client_id: &str, client_secret: Option<&str>, username: Option<&str>, password: Option<&str>) -> Result<()> {
        // We auth against the master realm usually for admin tasks, or the specific realm if using client credentials for a client in that realm.
        // Assuming admin-cli in master realm for now as default.
        let auth_realm = "master";
        let url = format!("{}/realms/{}/protocol/openid-connect/token", self.base_url, auth_realm);

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

        let response = self.client.post(&url)
            .form(&params)
            .send()
            .await
            .context("Failed to send login request")?;

        if !response.status().is_success() {
             let status = response.status();
             let text = response.text().await.unwrap_or_default();
             anyhow::bail!("Login failed: {} - {}", status, text);
        }

        let token_response: TokenResponse = response.json().await.context("Failed to parse token response")?;
        self.token = Some(token_response.access_token);

        info!("Successfully logged in to Keycloak");
        Ok(())
    }

    pub fn get_token(&self) -> Result<&str> {
        self.token.as_deref().context("Not authenticated")
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
        Err(_) => url_str.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redact_url() {
        assert_eq!(redact_url("http://localhost:8080"), "http://localhost:8080/");
        assert_eq!(redact_url("http://user:pass@localhost:8080/path"), "http://localhost:8080/path");
        assert_eq!(redact_url("http://user@localhost:8080/path"), "http://localhost:8080/path");
        assert_eq!(redact_url("invalid-url"), "invalid-url");
    }
}
