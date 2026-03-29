use app::client::KeycloakClient;
use app::apply;
use std::path::PathBuf;
use tokio::runtime::Runtime;
use std::fs;

#[path = "../tests/common/mod.rs"]
mod common;

fn main() {
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        let server_url = common::start_mock_server().await;
        let mut client = KeycloakClient::new(server_url, "test-realm".to_string());
        client
            .login("admin-cli", None, Some("admin"), Some("admin"))
            .await
            .unwrap();

        let temp_dir = tempfile::tempdir().unwrap();
        let groups_dir = temp_dir.path().join("groups");
        fs::create_dir_all(&groups_dir).unwrap();

        for i in 0..100 {
            let group_name = format!("group-{}", i);
            let group_file = groups_dir.join(format!("{}.yaml", group_name));
            let content = format!("name: {}\n", group_name);
            fs::write(group_file, content).unwrap();
        }

        // Warm up
        let _ = apply::run(&client, temp_dir.path().to_path_buf()).await;

        let start = std::time::Instant::now();
        for _ in 0..10 {
            apply::run(&client, temp_dir.path().to_path_buf())
                .await
                .unwrap();
        }
        let elapsed = start.elapsed();
        println!("Elapsed time for 10 iterations of 100 groups: {:?}", elapsed);
    });
}
