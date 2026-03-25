pub mod client;
pub mod group;
pub mod idp;
pub mod keys;
pub mod role;
pub mod user;

use crate::utils::ui::{ERROR, INFO};
use anyhow::Result;
use console::style;
use dialoguer::{Select, theme::ColorfulTheme};
use std::path::PathBuf;

pub async fn run(workspace_dir: PathBuf) -> Result<()> {
    println!(
        "{} {}",
        INFO,
        style("Welcome to kcd interactive CLI!").cyan().bold()
    );
    let theme = ColorfulTheme::default();
    let selections = &[
        "Create User",
        "Change User Password",
        "Create Client",
        "Create Role",
        "Create Group",
        "Create Identity Provider",
        "Create Client Scope",
        "Rotate Keys",
        "Exit",
    ];

    loop {
        let selection = Select::with_theme(&theme)
            .with_prompt("What would you like to do?")
            .default(0)
            .items(&selections[..])
            .interact()?;

        match selection {
            0 => {
                if let Err(e) = user::create_user_interactive(&workspace_dir).await {
                    println!(
                        "{} {}",
                        ERROR,
                        style(format!("Error creating user: {}", e)).red()
                    );
                }
            }
            1 => {
                if let Err(e) = user::change_user_password_interactive(&workspace_dir).await {
                    println!(
                        "{} {}",
                        ERROR,
                        style(format!("Error changing password: {}", e)).red()
                    );
                }
            }
            2 => {
                if let Err(e) = client::create_client_interactive(&workspace_dir).await {
                    println!(
                        "{} {}",
                        ERROR,
                        style(format!("Error creating client: {}", e)).red()
                    );
                }
            }
            3 => {
                if let Err(e) = role::create_role_interactive(&workspace_dir).await {
                    println!(
                        "{} {}",
                        ERROR,
                        style(format!("Error creating role: {}", e)).red()
                    );
                }
            }
            4 => {
                if let Err(e) = group::create_group_interactive(&workspace_dir).await {
                    println!(
                        "{} {}",
                        ERROR,
                        style(format!("Error creating group: {}", e)).red()
                    );
                }
            }
            5 => {
                if let Err(e) = idp::create_idp_interactive(&workspace_dir).await {
                    println!(
                        "{} {}",
                        ERROR,
                        style(format!("Error creating IDP: {}", e)).red()
                    );
                }
            }
            6 => {
                if let Err(e) = client::create_client_scope_interactive(&workspace_dir).await {
                    println!(
                        "{} {}",
                        ERROR,
                        style(format!("Error creating client scope: {}", e)).red()
                    );
                }
            }
            7 => {
                if let Err(e) = keys::rotate_keys_interactive(&workspace_dir).await {
                    println!(
                        "{} {}",
                        ERROR,
                        style(format!("Error rotating keys: {}", e)).red()
                    );
                }
            }
            8 => {
                println!("{} {}", INFO, style("Exiting...").cyan());
                break;
            }
            _ => unreachable!(),
        }
    }

    Ok(())
}
