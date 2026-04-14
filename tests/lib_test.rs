use kcd::args::{Cli, Commands};
use kcd::init_client;
use kcd::run_app;
use std::path::PathBuf;

#[tokio::test]
async fn test_init_client_fail() {
    let cli = Cli {
        server: "http://invalid".to_string(),
        client_id: "admin-cli".to_string(),
        client_secret: None,
        user: Some("admin".to_string()),
        password: Some("password".to_string()),
        realms: vec![],
        command: Commands::Validate {
            workspace: PathBuf::from("."),
        },
        vault_addr: None,
        vault_token: None,
    };

    let res = init_client(&cli).await;
    assert!(res.is_err());
}

#[tokio::test]
async fn test_run_app_validate_non_existent() {
    let cli = Cli {
        server: "http://localhost:8080".to_string(),
        client_id: "admin-cli".to_string(),
        client_secret: None,
        user: None,
        password: None,
        realms: vec![],
        command: Commands::Validate {
            workspace: PathBuf::from("non-existent-dir-123"),
        },
        vault_addr: None,
        vault_token: None,
    };

    let res = run_app(cli).await;
    assert!(res.is_err());
}
