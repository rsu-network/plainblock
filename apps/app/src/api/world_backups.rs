use crate::api::Result;
use theseus::world_backups::{self, WorldBackup};

pub fn init<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("world-backups")
        .invoke_handler(tauri::generate_handler![
            world_backups_list,
            world_backups_create,
            world_backups_restore,
            world_backups_delete,
        ])
        .build()
}

/// List all world backups of an instance, newest first.
#[tauri::command]
pub async fn world_backups_list(
    instance_id: String,
) -> Result<Vec<WorldBackup>> {
    Ok(world_backups::list(&instance_id).await?)
}

/// Create a backup that is never removed automatically.
#[tauri::command]
pub async fn world_backups_create(
    instance_id: String,
    world: String,
) -> Result<WorldBackup> {
    Ok(world_backups::create(&instance_id, &world).await?)
}

/// Roll a world back to a backup. The current state is kept as a backup first.
#[tauri::command]
pub async fn world_backups_restore(
    instance_id: String,
    world: String,
    file_name: String,
) -> Result<()> {
    Ok(world_backups::restore(&instance_id, &world, &file_name).await?)
}

#[tauri::command]
pub async fn world_backups_delete(
    instance_id: String,
    world: String,
    file_name: String,
) -> Result<()> {
    Ok(world_backups::delete(&instance_id, &world, &file_name).await?)
}
