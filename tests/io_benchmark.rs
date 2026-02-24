use std::path::PathBuf;
use std::time::{Duration, Instant};
use tokio::task::JoinSet;

// Mock work: read file, parse, sleep
async fn process_file_async(path: PathBuf) -> anyhow::Result<()> {
    let _content = tokio::fs::read_to_string(path).await?;
    // Simulate network latency (e.g. Keycloak API call)
    tokio::time::sleep(Duration::from_millis(50)).await;
    Ok(())
}

fn process_file_blocking(path: PathBuf) -> anyhow::Result<()> {
    let _content = std::fs::read_to_string(path)?;
    // Simulate network latency (e.g. Keycloak API call)
    std::thread::sleep(Duration::from_millis(50));
    Ok(())
}

#[tokio::test]
async fn benchmark_io() -> anyhow::Result<()> {
    let dir = tempfile::tempdir()?;
    let roles_dir = dir.path().join("roles");
    tokio::fs::create_dir(&roles_dir).await?;

    // Create 50 dummy files
    println!("Creating 50 dummy role files...");
    for i in 0..50 {
        tokio::fs::write(roles_dir.join(format!("role_{}.yaml", i)), "name: test").await?;
    }

    // Baseline: Blocking
    println!("Running blocking implementation...");
    let start = Instant::now();
    let entries = std::fs::read_dir(&roles_dir)?;
    for entry in entries {
        let entry = entry?;
        process_file_blocking(entry.path())?;
    }
    let duration_blocking = start.elapsed();
    println!("Blocking implementation took: {:?}", duration_blocking);

    // Optimized: Async + Concurrent
    println!("Running async implementation...");
    let start = Instant::now();
    let mut entries = tokio::fs::read_dir(&roles_dir).await?;
    let mut set = JoinSet::new();
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        set.spawn(async move {
            process_file_async(path).await
        });
    }
    while let Some(res) = set.join_next().await {
        res??;
    }
    let duration_async = start.elapsed();
    println!("Async implementation took: {:?}", duration_async);

    if duration_async < duration_blocking {
        println!("Improvement: {:.2}x faster", duration_blocking.as_secs_f64() / duration_async.as_secs_f64());
    } else {
        println!("No improvement (or slower).");
    }

    Ok(())
}
