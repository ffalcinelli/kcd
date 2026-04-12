use console::Emoji;

pub static ACTION: Emoji<'_, '_> = Emoji("🚀 ", ">> ");
pub static SEARCH: Emoji<'_, '_> = Emoji("🔍 ", "> ");
pub static CHECK: Emoji<'_, '_> = Emoji("✅ ", "√ ");
pub static SUCCESS: Emoji<'_, '_> = Emoji("🎉 ", "* ");
pub static SUCCESS_CREATE: Emoji<'_, '_> = Emoji("✨ ", "+ ");
pub static SUCCESS_UPDATE: Emoji<'_, '_> = Emoji("🔄 ", "~ ");
pub static WARN: Emoji<'_, '_> = Emoji("⚠️ ", "! ");
pub static ERROR: Emoji<'_, '_> = Emoji("❌ ", "x ");
pub static INFO: Emoji<'_, '_> = Emoji("💡 ", "i ");
pub static SPARKLE: Emoji<'_, '_> = Emoji("✨", "");
pub static MEMO: Emoji<'_, '_> = Emoji("📝", "");

use anyhow::Result;

pub trait Ui: Send + Sync {
    fn input(&self, prompt: &str, default: Option<String>, allow_empty: bool) -> Result<String>;
    fn confirm(&self, prompt: &str, default: bool) -> Result<bool>;
    fn password(&self, prompt: &str, confirm: Option<&str>) -> Result<String>;
    fn select(&self, prompt: &str, items: &[&str], default: usize) -> Result<usize>;
    fn print_info(&self, msg: &str);
    fn print_success(&self, msg: &str);
    fn print_error(&self, msg: &str);
    fn print_warn(&self, msg: &str);
}

pub struct DialoguerUi;

impl Ui for DialoguerUi {
    fn input(&self, prompt: &str, default: Option<String>, allow_empty: bool) -> Result<String> {
        let input = dialoguer::Input::<String>::new()
            .with_prompt(prompt)
            .allow_empty(allow_empty);
        let input = if let Some(d) = default {
            input.default(d)
        } else {
            input
        };
        Ok(input.interact_text()?)
    }

    fn confirm(&self, prompt: &str, default: bool) -> Result<bool> {
        Ok(dialoguer::Confirm::new()
            .with_prompt(prompt)
            .default(default)
            .interact()?)
    }

    fn password(&self, prompt: &str, confirm: Option<&str>) -> Result<String> {
        let p = dialoguer::Password::new().with_prompt(prompt);
        let p = if let Some(c) = confirm {
            p.with_confirmation(c, "Passwords mismatching")
        } else {
            p
        };
        Ok(p.interact()?)
    }

    fn select(&self, prompt: &str, items: &[&str], default: usize) -> Result<usize> {
        Ok(dialoguer::Select::new()
            .with_prompt(prompt)
            .items(items)
            .default(default)
            .interact()?)
    }

    fn print_info(&self, msg: &str) {
        println!("{} {}", INFO, msg);
    }

    fn print_success(&self, msg: &str) {
        println!("{} {}", SUCCESS, msg);
    }

    fn print_error(&self, msg: &str) {
        println!("{} {}", ERROR, msg);
    }

    fn print_warn(&self, msg: &str) {
        println!("{} {}", WARN, msg);
    }
}

pub struct MockUi {
    pub inputs: std::sync::Mutex<Vec<String>>,
    pub confirms: std::sync::Mutex<Vec<bool>>,
    pub selects: std::sync::Mutex<Vec<usize>>,
    pub passwords: std::sync::Mutex<Vec<String>>,
}

impl Ui for MockUi {
    fn input(&self, _prompt: &str, _default: Option<String>, _allow_empty: bool) -> Result<String> {
        let mut inputs = self.inputs.lock().unwrap();
        if inputs.is_empty() {
            anyhow::bail!("No more mock inputs");
        }
        Ok(inputs.remove(0))
    }
    fn confirm(&self, _prompt: &str, _default: bool) -> Result<bool> {
        let mut confirms = self.confirms.lock().unwrap();
        if confirms.is_empty() {
            anyhow::bail!("No more mock confirms");
        }
        Ok(confirms.remove(0))
    }
    fn password(&self, _prompt: &str, _confirm: Option<&str>) -> Result<String> {
        let mut p = self.passwords.lock().unwrap();
        if p.is_empty() {
            return Err(anyhow::anyhow!("Mock passwords missing"));
        }
        // Minimize secret copying/retention in memory.
        Ok(p.swap_remove(0))
    }
    fn select(&self, _prompt: &str, _items: &[&str], _default: usize) -> Result<usize> {
        let mut selects = self.selects.lock().unwrap();
        if selects.is_empty() {
            anyhow::bail!("No more mock selects");
        }
        Ok(selects.remove(0))
    }
    fn print_info(&self, _msg: &str) {}
    fn print_success(&self, _msg: &str) {}
    fn print_error(&self, _msg: &str) {}
    fn print_warn(&self, _msg: &str) {}
}
