use anyhow::{Context, Result};
use dialoguer::{theme::ColorfulTheme, Confirm};
use std::path::PathBuf;
use tokio::fs;

pub async fn run(output_dir: PathBuf, yes: bool, realms_to_clean: &[String]) -> Result<()> {
    if !output_dir.exists() {
        println!("Output directory {:?} does not exist, nothing to clean.", output_dir);
        return Ok(());
    }

    let targets = if realms_to_clean.is_empty() {
        vec![output_dir.clone()]
    } else {
        realms_to_clean
            .iter()
            .map(|r| output_dir.join(r))
            .filter(|p| p.exists())
            .collect()
    };

    if targets.is_empty() {
        println!("No targets found to clean.");
        return Ok(());
    }

    if !yes {
        let msg = if realms_to_clean.is_empty() {
            format!("Are you sure you want to delete everything in {:?}?", output_dir)
        } else {
            format!("Are you sure you want to delete the following realms in {:?}: {}?", output_dir, realms_to_clean.join(", "))
        };

        if !Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt(msg)
            .default(false)
            .interact()?
        {
            println!("Aborted.");
            return Ok(());
        }
    }

    for target in targets {
        if target == output_dir && realms_to_clean.is_empty() {
            // Special case: cleaning the whole directory. 
            // We might want to keep the directory itself but empty it, 
            // or just delete it and let inspect recreate it.
            // Let's delete everything inside it but keep .secrets if it exists?
            // Actually, usually "clean" means "wipe out everything".
            
            println!("Cleaning all configuration in {:?}", output_dir);
            let mut entries = fs::read_dir(&output_dir).await?;
            while let Some(entry) = entries.next_entry().await? {
                let path = entry.path();
                if path.is_dir() {
                    fs::remove_dir_all(&path).await.context(format!("Failed to remove dir {:?}", path))?;
                } else {
                    // Maybe we should keep .secrets? 
                    // User said "wipe out data from the output folder". 
                    // Usually that means everything.
                    fs::remove_file(&path).await.context(format!("Failed to remove file {:?}", path))?;
                }
            }
        } else {
            println!("Cleaning realm directory {:?}", target);
            if target.is_dir() {
                fs::remove_dir_all(&target).await.context(format!("Failed to remove dir {:?}", target))?;
            } else {
                fs::remove_file(&target).await.context(format!("Failed to remove file {:?}", target))?;
            }
        }
    }

    println!("Clean completed successfully.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_clean_all() {
        let dir = tempdir().unwrap();
        let path = dir.path().to_path_buf();

        fs::create_dir(path.join("realm1")).await.unwrap();
        fs::write(path.join("realm1").join("realm.yaml"), "test").await.unwrap();
        fs::write(path.join(".secrets"), "test").await.unwrap();

        run(path.clone(), true, &[]).await.unwrap();

        assert!(path.exists());
        let mut entries = fs::read_dir(&path).await.unwrap();
        assert!(entries.next_entry().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_clean_subset() {
        let dir = tempdir().unwrap();
        let path = dir.path().to_path_buf();

        fs::create_dir(path.join("realm1")).await.unwrap();
        fs::create_dir(path.join("realm2")).await.unwrap();

        run(path.clone(), true, &["realm1".to_string()]).await.unwrap();

        assert!(path.exists());
        assert!(!path.join("realm1").exists());
        assert!(path.join("realm2").exists());
    }
}
