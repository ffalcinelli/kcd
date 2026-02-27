use app::client::KeycloakClient;
use app::plan;
use std::path::PathBuf;
use tokio::runtime::Runtime;

#[path = "../tests/common/mod.rs"]
mod common;

fn main() {
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        let server_url = common::start_mock_server().await;
        let mut client = KeycloakClient::new(server_url, "test-realm".to_string());
        // For password grant: login(client_id, client_secret, username, password)
        client.login("admin-cli", None, Some("admin"), Some("admin")).await.unwrap();

        let start = std::time::Instant::now();
        for _ in 0..50 {
            plan::run(&client, PathBuf::from("/tmp/perf_test"), true).await.unwrap();
        }
        let elapsed = start.elapsed();
        println!("Elapsed time: {:?}", elapsed);
    });
}
