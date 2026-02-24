use axum::{Json, Router, http::StatusCode, response::IntoResponse, routing::post};
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct TokenRequest {
    pub grant_type: String,
    pub client_id: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub client_secret: Option<String>,
}

#[derive(Serialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub expires_in: i32,
}

pub async fn start_mock_server() -> String {
    let app = Router::new()
        .route(
            "/realms/master/protocol/openid-connect/token",
            post(token_handler),
        )
        .route(
            "/admin/realms/{realm}",
            axum::routing::get(get_realm_handler).put(generic_handler),
        )
        .route(
            "/admin/realms/{realm}/clients",
            axum::routing::get(get_clients_handler).post(generic_handler),
        )
        .route(
            "/admin/realms/{realm}/roles",
            axum::routing::get(get_roles_handler).post(generic_handler),
        )
        .route(
            "/admin/realms/{realm}/clients/{id}",
            axum::routing::put(generic_handler).delete(generic_handler),
        )
        .route(
            "/admin/realms/{realm}/roles-by-id/{id}",
            axum::routing::put(generic_handler).delete(generic_handler),
        );

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let port = addr.port();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    format!("http://127.0.0.1:{}", port)
}

async fn token_handler(axum::Form(payload): axum::Form<TokenRequest>) -> impl IntoResponse {
    if payload.grant_type == "password"
        && payload.username.as_deref() == Some("admin")
        && payload.password.as_deref() == Some("admin")
    {
        (
            StatusCode::OK,
            Json(TokenResponse {
                access_token: "mock_token".to_string(),
                expires_in: 300,
            }),
        )
    } else if payload.grant_type == "client_credentials"
        && payload.client_id == "admin-cli"
        && payload.client_secret.as_deref() == Some("secret")
    {
        (
            StatusCode::OK,
            Json(TokenResponse {
                access_token: "mock_token".to_string(),
                expires_in: 300,
            }),
        )
    } else {
        (
            StatusCode::UNAUTHORIZED,
            Json(TokenResponse {
                access_token: "invalid".to_string(),
                expires_in: 0,
            }),
        )
    }
}

async fn get_realm_handler(
    axum::extract::Path(realm): axum::extract::Path<String>,
) -> impl IntoResponse {
    if realm == "test-realm" {
        (
            StatusCode::OK,
            Json(serde_json::json!({
                "realm": "test-realm",
                "enabled": true,
                "displayName": "Test Realm"
            })),
        )
    } else {
        (StatusCode::NOT_FOUND, Json(serde_json::json!({})))
    }
}

async fn get_clients_handler(
    axum::extract::Path(realm): axum::extract::Path<String>,
) -> impl IntoResponse {
    if realm == "test-realm" {
        (
            StatusCode::OK,
            Json(serde_json::json!([
                {
                    "id": "1",
                    "clientId": "client-1",
                    "name": "Client 1",
                    "enabled": true
                },
                {
                    "id": "2",
                    "clientId": "client-2",
                    "enabled": false
                }
            ])),
        )
    } else {
        (StatusCode::NOT_FOUND, Json(serde_json::json!([])))
    }
}

async fn generic_handler() -> impl IntoResponse {
    StatusCode::NO_CONTENT
}

async fn get_roles_handler(
    axum::extract::Path(realm): axum::extract::Path<String>,
) -> impl IntoResponse {
    if realm == "test-realm" {
        (
            StatusCode::OK,
            Json(serde_json::json!([
                {
                    "id": "r1",
                    "name": "role-1",
                    "description": "Role 1"
                },
                {
                    "id": "r2",
                    "name": "role-2"
                }
            ])),
        )
    } else {
        (StatusCode::NOT_FOUND, Json(serde_json::json!([])))
    }
}
