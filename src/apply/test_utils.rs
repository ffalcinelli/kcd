use crate::models::*;
use anyhow::Result;
use axum::{
    Json, Router,
    http::StatusCode,
    routing::{get, post, put},
};
use std::sync::Arc;
use tokio::net::TcpListener;

pub async fn start_mock_server() -> Result<(String, Arc<std::sync::atomic::AtomicUsize>)> {
    let call_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let count_clone = Arc::clone(&call_count);

    let app = Router::new()
        .route(
            "/admin/realms/test/client-scopes",
            get(|| async {
                Json(vec![ClientScopeRepresentation {
                    id: Some("existing-id".to_string()),
                    name: Some("existing-scope".to_string()),
                    description: None,
                    protocol: None,
                    attributes: None,
                    extra: Default::default(),
                }])
            }),
        )
        .route(
            "/admin/realms/test/client-scopes/existing-id",
            put({
                let count = Arc::clone(&count_clone);
                move || {
                    let c = count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    async move {
                        if c == 0 {
                            StatusCode::INTERNAL_SERVER_ERROR
                        } else {
                            StatusCode::OK
                        }
                    }
                }
            }),
        )
        .route(
            "/admin/realms/test/client-scopes",
            post({
                let count = Arc::clone(&count_clone);
                move || {
                    let c = count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    async move {
                        if c == 0 {
                            StatusCode::INTERNAL_SERVER_ERROR
                        } else {
                            StatusCode::CREATED
                        }
                    }
                }
            }),
        )
        .route(
            "/admin/realms/test/groups",
            get(|| async {
                Json(vec![GroupRepresentation {
                    id: Some("existing-id".to_string()),
                    name: Some("Existing Group".to_string()),
                    path: Some("/existing-group".to_string()),
                    sub_groups: None,
                    extra: Default::default(),
                }])
            }),
        )
        .route(
            "/admin/realms/test/groups/existing-id",
            put({
                let count = Arc::clone(&count_clone);
                move || {
                    let c = count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    async move {
                        if c == 0 {
                            StatusCode::INTERNAL_SERVER_ERROR
                        } else {
                            StatusCode::OK
                        }
                    }
                }
            }),
        )
        .route(
            "/admin/realms/test/groups",
            post({
                let count = Arc::clone(&count_clone);
                move || {
                    let c = count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    async move {
                        if c == 0 {
                            StatusCode::INTERNAL_SERVER_ERROR
                        } else {
                            StatusCode::CREATED
                        }
                    }
                }
            }),
        )
        .route(
            "/admin/realms/test/roles",
            get(|| async {
                Json(vec![RoleRepresentation {
                    id: Some("existing-id".to_string()),
                    name: "existing-role".to_string(),
                    description: None,
                    container_id: None,
                    composite: false,
                    client_role: false,
                    extra: Default::default(),
                }])
            }),
        )
        .route(
            "/admin/realms/test/roles-by-id/existing-id",
            put({
                let count = Arc::clone(&count_clone);
                move || {
                    let c = count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    async move {
                        if c == 0 {
                            StatusCode::INTERNAL_SERVER_ERROR
                        } else {
                            StatusCode::OK
                        }
                    }
                }
            }),
        )
        .route(
            "/admin/realms/test/roles",
            post({
                let count = Arc::clone(&count_clone);
                move || {
                    let c = count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    async move {
                        if c == 0 {
                            StatusCode::INTERNAL_SERVER_ERROR
                        } else {
                            StatusCode::CREATED
                        }
                    }
                }
            }),
        )
        .route(
            "/admin/realms/test/authentication/required-actions",
            get(|| async {
                Json(vec![RequiredActionProviderRepresentation {
                    alias: Some("existing-action".to_string()),
                    name: Some("Existing Action".to_string()),
                    provider_id: Some("existing-provider".to_string()),
                    enabled: Some(true),
                    default_action: Some(false),
                    priority: Some(0),
                    config: None,
                    extra: Default::default(),
                }])
            }),
        )
        .route(
            "/admin/realms/test/authentication/required-actions/existing-action",
            put({
                let count = Arc::clone(&count_clone);
                move || {
                    count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    async { StatusCode::INTERNAL_SERVER_ERROR }
                }
            }),
        )
        .route(
            "/admin/realms/test/authentication/register-required-action",
            post({
                let count = Arc::clone(&count_clone);
                move || {
                    let c = count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    async move {
                        if c == 0 {
                            StatusCode::INTERNAL_SERVER_ERROR
                        } else {
                            StatusCode::OK
                        }
                    }
                }
            }),
        )
        .route(
            "/admin/realms/test/authentication/required-actions/new-action",
            put({
                let count = Arc::clone(&count_clone);
                move || {
                    count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    async { StatusCode::INTERNAL_SERVER_ERROR }
                }
            }),
        )
        .route(
            "/admin/realms/test/authentication/flows",
            get(|| async {
                Json(vec![AuthenticationFlowRepresentation {
                    id: Some("existing-id".to_string()),
                    alias: Some("existing-flow".to_string()),
                    description: Some("Existing Flow".to_string()),
                    provider_id: None,
                    top_level: Some(true),
                    built_in: Some(false),
                    authentication_executions: None,
                    extra: Default::default(),
                }])
            }),
        )
        .route(
            "/admin/realms/test/authentication/flows/existing-id",
            put({
                let count = Arc::clone(&count_clone);
                move || {
                    let c = count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    async move {
                        if c == 0 {
                            StatusCode::INTERNAL_SERVER_ERROR
                        } else {
                            StatusCode::OK
                        }
                    }
                }
            }),
        )
        .route(
            "/admin/realms/test/authentication/flows",
            post({
                let count = Arc::clone(&count_clone);
                move || {
                    let c = count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    async move {
                        if c == 0 {
                            StatusCode::INTERNAL_SERVER_ERROR
                        } else {
                            StatusCode::CREATED
                        }
                    }
                }
            }),
        )
        .route(
            "/admin/realms/test/users",
            get(|| async {
                Json(vec![UserRepresentation {
                    id: Some("existing-id".to_string()),
                    username: Some("existing-user".to_string()),
                    enabled: Some(true),
                    first_name: None,
                    last_name: None,
                    email: None,
                    email_verified: None,
                    credentials: None,
                    extra: Default::default(),
                }])
            }),
        )
        .route(
            "/admin/realms/test/users/existing-id",
            put({
                let count = Arc::clone(&count_clone);
                move || {
                    let c = count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    async move {
                        if c == 0 {
                            StatusCode::INTERNAL_SERVER_ERROR
                        } else {
                            StatusCode::OK
                        }
                    }
                }
            }),
        )
        .route(
            "/admin/realms/test/users",
            post({
                let count = Arc::clone(&count_clone);
                move || {
                    let c = count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    async move {
                        if c == 0 {
                            StatusCode::INTERNAL_SERVER_ERROR
                        } else {
                            StatusCode::CREATED
                        }
                    }
                }
            }),
        )
        .route(
            "/admin/realms/test/components",
            get(|| async {
                Json(vec![ComponentRepresentation {
                    id: Some("existing-id".to_string()),
                    name: Some("Existing Component".to_string()),
                    provider_id: Some("existing-provider".to_string()),
                    provider_type: Some("existing-type".to_string()),
                    parent_id: Some("test".to_string()),
                    sub_type: None,
                    config: None,
                    extra: Default::default(),
                }])
            }),
        )
        .route(
            "/admin/realms/test/components/existing-id",
            put({
                let count = Arc::clone(&count_clone);
                move || {
                    let c = count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    async move {
                        if c == 0 {
                            StatusCode::INTERNAL_SERVER_ERROR
                        } else {
                            StatusCode::OK
                        }
                    }
                }
            }),
        )
        .route(
            "/admin/realms/test/components",
            post({
                let count = Arc::clone(&count_clone);
                move || {
                    let c = count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    async move {
                        if c == 0 {
                            StatusCode::INTERNAL_SERVER_ERROR
                        } else {
                            StatusCode::CREATED
                        }
                    }
                }
            }),
        )
        .route(
            "/admin/realms/test/clients",
            get(|| async {
                Json(vec![ClientRepresentation {
                    id: Some("existing-id".to_string()),
                    client_id: Some("existing-client".to_string()),
                    name: Some("Existing Client".to_string()),
                    description: None,
                    enabled: Some(true),
                    protocol: None,
                    redirect_uris: None,
                    web_origins: None,
                    public_client: None,
                    bearer_only: None,
                    service_accounts_enabled: None,
                    extra: Default::default(),
                }])
            })
            .post({
                let count = Arc::clone(&count_clone);
                move || {
                    let c = count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    async move {
                        if c == 0 {
                            StatusCode::INTERNAL_SERVER_ERROR
                        } else {
                            StatusCode::CREATED
                        }
                    }
                }
            }),
        )
        .route(
            "/admin/realms/test/clients/existing-id",
            put({
                let count = Arc::clone(&count_clone);
                move || {
                    let c = count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    async move {
                        if c == 0 {
                            StatusCode::INTERNAL_SERVER_ERROR
                        } else {
                            StatusCode::OK
                        }
                    }
                }
            }),
        );

    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    Ok((format!("http://{}", addr), call_count))
}
