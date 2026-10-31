use crate::api::Result;
use theseus::curseforge::{
    self, CurseForgeCategory, CurseForgeFile, CurseForgeMod,
    CurseForgeSearchResponse, CurseForgeUpdate,
};
use theseus::data::ModLoader;

pub fn init<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("curseforge")
        .invoke_handler(tauri::generate_handler![
            curseforge_search,
            curseforge_get_mod,
            curseforge_get_mod_description,
            curseforge_get_files,
            curseforge_get_categories,
            curseforge_install_content,
            curseforge_install_modpack,
            curseforge_check_updates,
            curseforge_update_file,
        ])
        .build()
}

/// Search CurseForge for mods, modpacks, resource packs, or shader packs.
#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn curseforge_search(
    query: String,
    class_id: u32,
    game_version: Option<String>,
    mod_loader: Option<ModLoader>,
    category_id: Option<u32>,
    sort_field: Option<u32>,
    sort_order: Option<String>,
    index: u32,
    page_size: u32,
) -> Result<CurseForgeSearchResponse> {
    Ok(curseforge::search(
        &query,
        class_id,
        game_version.as_deref(),
        mod_loader,
        category_id,
        sort_field,
        sort_order.as_deref(),
        index,
        page_size,
    )
    .await?)
}

#[tauri::command]
pub async fn curseforge_get_categories(
    class_id: u32,
) -> Result<Vec<CurseForgeCategory>> {
    Ok(curseforge::get_categories(class_id).await?)
}

#[tauri::command]
pub async fn curseforge_get_mod(mod_id: u32) -> Result<CurseForgeMod> {
    Ok(curseforge::get_mod(mod_id).await?)
}

#[tauri::command]
pub async fn curseforge_get_mod_description(mod_id: u32) -> Result<String> {
    Ok(curseforge::get_mod_description(mod_id).await?)
}

#[tauri::command]
pub async fn curseforge_get_files(
    mod_id: u32,
    game_version: Option<String>,
    mod_loader: Option<ModLoader>,
) -> Result<Vec<CurseForgeFile>> {
    Ok(curseforge::get_files(
        mod_id,
        game_version.as_deref(),
        mod_loader,
    )
    .await?)
}

/// Install a single mod, resource pack, or shader pack into an existing instance.
#[tauri::command]
pub async fn curseforge_install_content(
    instance_id: String,
    mod_id: u32,
    file_id: u32,
    class_id: u32,
) -> Result<()> {
    Ok(curseforge::install_content_to_instance(
        &instance_id,
        mod_id,
        file_id,
        class_id,
    )
    .await?)
}

/// Install a CurseForge modpack as a brand-new instance. Returns the new instance's id.
#[tauri::command]
pub async fn curseforge_install_modpack(
    mod_id: u32,
    file_id: u32,
) -> Result<String> {
    Ok(curseforge::install_modpack(mod_id, file_id).await?)
}

/// List installed CurseForge content in an instance that has a newer file available.
#[tauri::command]
pub async fn curseforge_check_updates(
    instance_id: String,
) -> Result<Vec<CurseForgeUpdate>> {
    Ok(curseforge::check_updates(&instance_id).await?)
}

/// Replace an installed CurseForge file with a newer one.
#[tauri::command]
pub async fn curseforge_update_file(
    instance_id: String,
    project_path: String,
    new_file_id: u32,
) -> Result<()> {
    Ok(curseforge::update_file(&instance_id, &project_path, new_file_id)
        .await?)
}
