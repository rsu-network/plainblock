use crate::api::Result;
use theseus::mod_check::{self, ModIssue};

pub fn init<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("mod-check")
        .invoke_handler(tauri::generate_handler![mod_check_scan])
        .build()
}

/// Find missing dependencies, duplicates and wrong-loader mods in an instance.
#[tauri::command]
pub async fn mod_check_scan(instance_id: String) -> Result<Vec<ModIssue>> {
    Ok(mod_check::check(&instance_id).await?)
}
