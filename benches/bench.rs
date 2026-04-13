use kcd::client::KeycloakClient;
use kcd::plan;
use kcd::utils::ui::DialoguerUi;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::runtime::Runtime;

#[path = "../tests/common/mod.rs"]
mod common;

fn main() {
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        let server_url = common::start_mock_server().await;
        let mut client = KeycloakClient::new(server_url);
        client.set_target_realm("test-realm".to_string());
        // For password grant: login(client_id, client_secret, username, password)
        client
            .login("admin-cli", None, Some("admin"), Some("admin"))
            .await
            .unwrap();

        let start = std::time::Instant::now();
        let ui = Arc::new(DialoguerUi);
        for _ in 0..50 {
            plan::run(
                &client,
                PathBuf::from("/tmp/perf_test"),
                true,
                false,
                &[],
                ui.clone(),
            )
            .await
            .unwrap();
        }
        let elapsed = start.elapsed();
        println!("Elapsed time: {:?}", elapsed);
    });
}
