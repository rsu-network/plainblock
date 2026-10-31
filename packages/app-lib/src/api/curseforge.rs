use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::State;
use crate::state::{InstanceLink, ModLoader};
use crate::util::io::{self, IOError};

// A dedicated client, kept separate from the shared REQWEST_CLIENT used for
// Modrinth so a flaky third-party proxy can't affect Modrinth requests (or
// vice versa).
static CF_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .user_agent(crate::launcher_user_agent())
        .build()
        .expect("Failed to build CurseForge HTTP client")
});

// Uses GDLauncher's public CurseForge proxy by default, which mirrors
// CurseForge's own API 1:1 without requiring an API key (CurseForge's own
// API requires manual approval for third-party launchers, see
// https://docs.curseforge.com/rest-api/). Can be overridden at build time to
// point at the official https://api.curseforge.com/v1/ once an approved key
// is available, by setting CURSEFORGE_API_URL and CURSEFORGE_API_KEY.
const CF_API_BASE: &str = match option_env!("CURSEFORGE_API_URL") {
    Some(url) => url,
    None => "https://api.curse.tools/v1/",
};
const CF_GAME_ID_MINECRAFT: u32 = 432;
const REQUIRED_DEPENDENCY: u32 = 3;

pub const CLASS_ID_MODS: u32 = 6;
pub const CLASS_ID_RESOURCE_PACKS: u32 = 12;
pub const CLASS_ID_MODPACKS: u32 = 4471;
pub const CLASS_ID_SHADER_PACKS: u32 = 6552;

fn api_key() -> Option<&'static str> {
    option_env!("CURSEFORGE_API_KEY").filter(|key| !key.is_empty())
}

fn cf_mod_loader_type(loader: ModLoader) -> u32 {
    match loader {
        ModLoader::Vanilla => 0,
        ModLoader::Forge => 1,
        ModLoader::Fabric => 4,
        ModLoader::Quilt => 5,
        ModLoader::NeoForge => 6,
    }
}

fn parse_cf_loader(id: &str) -> ModLoader {
    if id.starts_with("forge-") {
        ModLoader::Forge
    } else if id.starts_with("fabric-") {
        ModLoader::Fabric
    } else if id.starts_with("quilt-") {
        ModLoader::Quilt
    } else if id.starts_with("neoforge-") {
        ModLoader::NeoForge
    } else {
        ModLoader::Vanilla
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CurseForgeModLinks {
    #[serde(rename = "websiteUrl")]
    pub website_url: Option<String>,
    #[serde(rename = "wikiUrl")]
    pub wiki_url: Option<String>,
    #[serde(rename = "issuesUrl")]
    pub issues_url: Option<String>,
    #[serde(rename = "sourceUrl")]
    pub source_url: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CurseForgeLogo {
    #[serde(rename = "thumbnailUrl")]
    pub thumbnail_url: Option<String>,
    pub url: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CurseForgeAuthor {
    pub name: String,
    pub url: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CurseForgeCategory {
    pub id: u32,
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CurseForgeScreenshot {
    pub url: String,
    #[serde(rename = "thumbnailUrl")]
    pub thumbnail_url: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CurseForgeMod {
    pub id: u32,
    pub name: String,
    #[serde(default)]
    pub summary: String,
    pub slug: String,
    #[serde(rename = "classId")]
    pub class_id: Option<u32>,
    #[serde(rename = "mainFileId")]
    pub main_file_id: u32,
    #[serde(rename = "downloadCount")]
    pub download_count: f64,
    #[serde(rename = "dateModified")]
    pub date_modified: String,
    pub logo: Option<CurseForgeLogo>,
    #[serde(default)]
    pub screenshots: Vec<CurseForgeScreenshot>,
    #[serde(default)]
    pub authors: Vec<CurseForgeAuthor>,
    #[serde(default)]
    pub categories: Vec<CurseForgeCategory>,
    pub links: Option<CurseForgeModLinks>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CurseForgePagination {
    pub index: u32,
    #[serde(rename = "pageSize")]
    pub page_size: u32,
    #[serde(rename = "resultCount")]
    pub result_count: u32,
    #[serde(rename = "totalCount")]
    pub total_count: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CurseForgeSearchResponse {
    pub data: Vec<CurseForgeMod>,
    pub pagination: CurseForgePagination,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CurseForgeFileDependency {
    #[serde(rename = "modId")]
    pub mod_id: u32,
    #[serde(rename = "relationType")]
    pub relation_type: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CurseForgeFile {
    pub id: u32,
    #[serde(rename = "modId")]
    pub mod_id: u32,
    #[serde(rename = "fileName")]
    pub file_name: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
    #[serde(rename = "downloadUrl")]
    pub download_url: Option<String>,
    #[serde(rename = "gameVersions")]
    pub game_versions: Vec<String>,
    #[serde(default)]
    pub dependencies: Vec<CurseForgeFileDependency>,
    #[serde(rename = "fileDate")]
    pub file_date: String,
    #[serde(rename = "releaseType", default)]
    pub release_type: u32,
    #[serde(rename = "downloadCount", default)]
    pub download_count: f64,
    #[serde(rename = "fileLength", default)]
    pub file_length: f64,
}

#[derive(Deserialize)]
struct CurseForgeDataWrapper<T> {
    data: T,
}

fn cf_error(message: String) -> crate::Error {
    crate::ErrorKind::LauncherError(message).into()
}

const CF_MAX_RETRIES: u32 = 2;

async fn cf_request(url: &str) -> crate::Result<reqwest::Response> {
    let mut attempt = 0;
    loop {
        attempt += 1;
        let mut request = CF_CLIENT
            .get(url)
            .header("Accept", "application/json")
            .timeout(Duration::from_secs(15));
        if let Some(key) = api_key() {
            request = request
                .header("x-api-key", key)
                .header("Authorization", format!("Bearer {key}"));
        }

        let result = request.send().await;

        match result {
            Ok(response) if response.status().is_success() => {
                return Ok(response);
            }
            Ok(response) => {
                let status = response.status();
                // Transient upstream/mirror hiccups (5xx) are worth a couple
                // of quick retries; anything else (403/404/etc.) is a real
                // error and should surface immediately.
                if status.is_server_error() && attempt <= CF_MAX_RETRIES {
                    tokio::time::sleep(Duration::from_millis(
                        400 * attempt as u64,
                    ))
                    .await;
                    continue;
                }
                return Err(cf_error(format!(
                        "CurseForge API returned HTTP {status} for {url}"
                    )));
            }
            Err(e) => {
                if attempt <= CF_MAX_RETRIES {
                    tokio::time::sleep(Duration::from_millis(
                        400 * attempt as u64,
                    ))
                    .await;
                    continue;
                }
                return Err(cf_error(format!("CurseForge request failed: {e}")));
            }
        }
    }
}

async fn cf_get<T: serde::de::DeserializeOwned>(
    path: &str,
) -> crate::Result<T> {
    let response = cf_request(&format!("{CF_API_BASE}{path}")).await?;
    let wrapper: CurseForgeDataWrapper<T> =
        response.json().await.map_err(|e| {
            cf_error(format!("Failed to parse CurseForge response: {e}"))
        })?;
    Ok(wrapper.data)
}

#[allow(clippy::too_many_arguments)]
#[tracing::instrument]
pub async fn search(
    query: &str,
    class_id: u32,
    game_version: Option<&str>,
    mod_loader: Option<ModLoader>,
    category_id: Option<u32>,
    sort_field: Option<u32>,
    sort_order: Option<&str>,
    index: u32,
    page_size: u32,
) -> crate::Result<CurseForgeSearchResponse> {
    let mut url = format!(
        "{CF_API_BASE}mods/search?gameId={CF_GAME_ID_MINECRAFT}&classId={class_id}&index={index}&pageSize={page_size}&sortField={}&sortOrder={}",
        sort_field.unwrap_or(2),
        sort_order.unwrap_or("desc"),
    );
    if !query.trim().is_empty() {
        url.push_str(&format!(
            "&searchFilter={}",
            urlencoding::encode(query.trim())
        ));
    }
    if let Some(version) = game_version {
        url.push_str(&format!(
            "&gameVersion={}",
            urlencoding::encode(version)
        ));
    }
    if let Some(loader) = mod_loader {
        url.push_str(&format!(
            "&modLoaderType={}",
            cf_mod_loader_type(loader)
        ));
    }
    if let Some(category_id) = category_id {
        url.push_str(&format!("&categoryId={category_id}"));
    }

    let response = cf_request(&url).await?;
    response.json::<CurseForgeSearchResponse>().await.map_err(|e| {
        cf_error(format!(
                "Failed to parse CurseForge search response: {e}"
            ))
    })
}

#[tracing::instrument]
pub async fn get_mod(mod_id: u32) -> crate::Result<CurseForgeMod> {
    cf_get(&format!("mods/{mod_id}")).await
}

#[tracing::instrument]
pub async fn get_categories(
    class_id: u32,
) -> crate::Result<Vec<CurseForgeCategory>> {
    cf_get(&format!(
        "categories?gameId={CF_GAME_ID_MINECRAFT}&classId={class_id}"
    ))
    .await
}

#[tracing::instrument]
pub async fn get_mod_description(mod_id: u32) -> crate::Result<String> {
    cf_get(&format!("mods/{mod_id}/description")).await
}

#[tracing::instrument]
pub async fn get_file(
    mod_id: u32,
    file_id: u32,
) -> crate::Result<CurseForgeFile> {
    cf_get(&format!("mods/{mod_id}/files/{file_id}")).await
}

#[tracing::instrument]
pub async fn get_files(
    mod_id: u32,
    game_version: Option<&str>,
    mod_loader: Option<ModLoader>,
) -> crate::Result<Vec<CurseForgeFile>> {
    let mut url =
        format!("{CF_API_BASE}mods/{mod_id}/files?pageSize=50");
    if let Some(version) = game_version {
        url.push_str(&format!(
            "&gameVersion={}",
            urlencoding::encode(version)
        ));
    }
    if let Some(loader) = mod_loader {
        url.push_str(&format!(
            "&modLoaderType={}",
            cf_mod_loader_type(loader)
        ));
    }

    let response = cf_request(&url).await?;
    let wrapper: CurseForgeDataWrapper<Vec<CurseForgeFile>> =
        response.json().await.map_err(|e| {
            cf_error(format!(
                    "Failed to parse CurseForge files response: {e}"
                ))
        })?;
    Ok(wrapper.data)
}

fn class_id_project_type(class_id: u32) -> Option<crate::state::ProjectType> {
    match class_id {
        CLASS_ID_MODS => Some(crate::state::ProjectType::Mod),
        CLASS_ID_RESOURCE_PACKS => {
            Some(crate::state::ProjectType::ResourcePack)
        }
        CLASS_ID_SHADER_PACKS => Some(crate::state::ProjectType::ShaderPack),
        _ => None,
    }
}

async fn download_cf_file_bytes(
    file: &CurseForgeFile,
    timeout: Duration,
) -> crate::Result<bytes::Bytes> {
    let url = file.download_url.clone().ok_or_else(|| {
        cf_error(format!(
            "\"{}\" does not allow third-party downloads (the author disabled distribution on CurseForge)",
            file.display_name
        ))
    })?;

    let response = CF_CLIENT
        .get(&url)
        .timeout(timeout)
        .send()
        .await
        .map_err(|e| {
            cf_error(format!("Failed to download {}: {e}", file.file_name))
        })?;

    if !response.status().is_success() {
        return Err(cf_error(format!(
                "Failed to download {}: HTTP {}",
                file.file_name,
                response.status()
            )));
    }

    response.bytes().await.map_err(|e| {
        cf_error(format!(
                "Failed to read download for {}: {e}",
                file.file_name
            ))
    })
}

/// Downloads a CurseForge file into a temporary directory and hands it to the
/// instance so that it is tracked like any other piece of content.
async fn download_and_add_to_instance(
    instance_id: &str,
    file: &CurseForgeFile,
    project_type: Option<crate::state::ProjectType>,
) -> crate::Result<String> {
    let bytes =
        download_cf_file_bytes(file, Duration::from_secs(180)).await?;

    let temp_dir = tempfile::tempdir()?;
    let file_name = Path::new(&file.file_name)
        .file_name()
        .map(|name| name.to_os_string())
        .unwrap_or_else(|| std::ffi::OsString::from("download.jar"));
    let temp_path: PathBuf = temp_dir.path().join(file_name);
    tokio::fs::write(&temp_path, &bytes)
        .await
        .map_err(|e| IOError::with_path(e, &temp_path))?;

    crate::api::instance::add_project_from_path(
        instance_id,
        &temp_path,
        project_type,
    )
    .await
}

const TRACKING_FILE: &str = "plainblock-curseforge.json";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CurseForgeTrackedFile {
    pub project_path: String,
    pub mod_id: u32,
    pub file_id: u32,
    pub file_date: String,
    pub class_id: u32,
    pub name: String,
}

#[derive(Serialize, Debug, Clone)]
pub struct CurseForgeUpdate {
    pub project_path: String,
    pub mod_id: u32,
    pub name: String,
    pub current_file_name: String,
    pub new_file_id: u32,
    pub new_file_name: String,
}

async fn tracking_path(instance_id: &str) -> crate::Result<PathBuf> {
    Ok(crate::api::instance::get_full_path(instance_id)
        .await?
        .join(TRACKING_FILE))
}

async fn read_tracking(
    instance_id: &str,
) -> crate::Result<Vec<CurseForgeTrackedFile>> {
    let path = tracking_path(instance_id).await?;
    match tokio::fs::read(&path).await {
        Ok(bytes) => Ok(serde_json::from_slice(&bytes).unwrap_or_default()),
        Err(_) => Ok(Vec::new()),
    }
}

async fn write_tracking(
    instance_id: &str,
    entries: &[CurseForgeTrackedFile],
) -> crate::Result<()> {
    let path = tracking_path(instance_id).await?;
    let bytes = serde_json::to_vec_pretty(entries)
        .map_err(|e| cf_error(format!("Failed to save tracking data: {e}")))?;
    tokio::fs::write(&path, bytes)
        .await
        .map_err(|e| IOError::with_path(e, &path))?;
    Ok(())
}

async fn track_file(
    instance_id: &str,
    project_path: String,
    file: &CurseForgeFile,
    class_id: u32,
    name: String,
) -> crate::Result<()> {
    let mut entries = read_tracking(instance_id).await?;
    entries.retain(|entry| {
        entry.project_path != project_path && entry.mod_id != file.mod_id
    });
    entries.push(CurseForgeTrackedFile {
        project_path,
        mod_id: file.mod_id,
        file_id: file.id,
        file_date: file.file_date.clone(),
        class_id,
        name,
    });
    write_tracking(instance_id, &entries).await
}

fn strip_disabled(path: &str) -> &str {
    path.strip_suffix(".disabled").unwrap_or(path)
}

/// Returns the CurseForge-installed content of an instance that has a newer
/// compatible file available.
#[tracing::instrument]
pub async fn check_updates(
    instance_id: &str,
) -> crate::Result<Vec<CurseForgeUpdate>> {
    let entries = read_tracking(instance_id).await?;
    if entries.is_empty() {
        return Ok(Vec::new());
    }

    let metadata =
        crate::api::instance::get(instance_id).await?.ok_or_else(|| {
            cf_error(format!("Unknown instance id: {instance_id}"))
        })?;
    let game_version = metadata.applied_content_set.game_version.clone();
    let loader = metadata.applied_content_set.loader;
    let root = crate::api::instance::get_full_path(instance_id).await?;

    let mut updates = Vec::new();
    for entry in entries {
        let on_disk = root.join(&entry.project_path).exists()
            || root
                .join(format!("{}.disabled", strip_disabled(&entry.project_path)))
                .exists();
        if !on_disk {
            continue;
        }

        let loader_filter = (entry.class_id == CLASS_ID_MODS).then_some(loader);
        let Ok(files) =
            get_files(entry.mod_id, Some(&game_version), loader_filter).await
        else {
            continue;
        };
        let Some(newest) = files
            .into_iter()
            .filter(|file| file.download_url.is_some())
            .max_by_key(|file| file.file_date.clone())
        else {
            continue;
        };

        if newest.file_date > entry.file_date && newest.id != entry.file_id {
            let current_file_name = Path::new(&entry.project_path)
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_default();
            updates.push(CurseForgeUpdate {
                project_path: entry.project_path,
                mod_id: entry.mod_id,
                name: entry.name,
                current_file_name,
                new_file_id: newest.id,
                new_file_name: newest.file_name,
            });
        }
    }

    Ok(updates)
}

/// Replaces an installed CurseForge file with a newer one.
#[tracing::instrument]
pub async fn update_file(
    instance_id: &str,
    project_path: &str,
    new_file_id: u32,
) -> crate::Result<()> {
    let entries = read_tracking(instance_id).await?;
    let entry = entries
        .into_iter()
        .find(|entry| entry.project_path == project_path)
        .ok_or_else(|| {
            cf_error("This file is not tracked by CurseForge".to_string())
        })?;

    let file = get_file(entry.mod_id, new_file_id).await?;
    let project_type = class_id_project_type(entry.class_id);
    let new_path =
        download_and_add_to_instance(instance_id, &file, project_type)
            .await?;

    if new_path != project_path {
        crate::api::instance::remove_project(instance_id, project_path)
            .await?;
    }
    track_file(instance_id, new_path, &file, entry.class_id, entry.name)
        .await
}

/// Installs a single mod, resource pack, or shader pack (and its required
/// dependencies) from CurseForge into an existing instance.
#[tracing::instrument]
pub async fn install_content_to_instance(
    instance_id: &str,
    mod_id: u32,
    file_id: u32,
    class_id: u32,
) -> crate::Result<()> {
    let project_type = class_id_project_type(class_id).ok_or_else(|| {
        cf_error(
            "Unsupported CurseForge content type for direct install"
                .to_string(),
        )
    })?;

    let metadata =
        crate::api::instance::get(instance_id).await?.ok_or_else(|| {
            cf_error(format!(
                "Unknown instance id: {instance_id}"
            ))
        })?;
    let game_version = metadata.applied_content_set.game_version.clone();
    let loader = metadata.applied_content_set.loader;

    let mut to_install = vec![(mod_id, file_id, project_type)];
    let mut installed = HashSet::new();
    while let Some((current_mod_id, current_file_id, current_type)) =
        to_install.pop()
    {
        if !installed.insert((current_mod_id, current_file_id)) {
            continue;
        }
        let file = get_file(current_mod_id, current_file_id).await?;
        let project_path = download_and_add_to_instance(
            instance_id,
            &file,
            Some(current_type),
        )
        .await?;
        let class_id = match current_type {
            crate::state::ProjectType::ResourcePack => CLASS_ID_RESOURCE_PACKS,
            crate::state::ProjectType::ShaderPack => CLASS_ID_SHADER_PACKS,
            _ => CLASS_ID_MODS,
        };
        let name = file
            .display_name
            .trim_end_matches(".jar")
            .trim_end_matches(".zip")
            .to_string();
        let _ = track_file(instance_id, project_path, &file, class_id, name)
            .await;

        for dependency in &file.dependencies {
            if dependency.relation_type == REQUIRED_DEPENDENCY
                && let Ok(dep_files) = get_files(
                    dependency.mod_id,
                    Some(&game_version),
                    Some(loader),
                )
                .await
                && let Some(dep_file) =
                    dep_files.into_iter().max_by_key(|f| f.file_date.clone())
            {
                to_install.push((
                    dependency.mod_id,
                    dep_file.id,
                    crate::state::ProjectType::Mod,
                ));
            }
        }
    }

    Ok(())
}

#[derive(Deserialize)]
struct CfManifestMinecraft {
    version: String,
    #[serde(rename = "modLoaders")]
    mod_loaders: Vec<CfManifestModLoader>,
}

#[derive(Deserialize)]
struct CfManifestModLoader {
    id: String,
    #[serde(default)]
    primary: bool,
}

#[derive(Deserialize)]
struct CfManifestFile {
    #[serde(rename = "projectID")]
    project_id: u32,
    #[serde(rename = "fileID")]
    file_id: u32,
    #[serde(default)]
    required: bool,
}

#[derive(Deserialize)]
struct CfManifest {
    minecraft: CfManifestMinecraft,
    name: String,
    files: Vec<CfManifestFile>,
    #[serde(default = "default_overrides")]
    overrides: String,
}

fn default_overrides() -> String {
    "overrides".to_string()
}

/// Installs a CurseForge modpack as a brand-new instance.
///
/// Returns the id of the newly created instance.
#[tracing::instrument]
pub async fn install_modpack(
    mod_id: u32,
    file_id: u32,
) -> crate::Result<String> {
    let file = get_file(mod_id, file_id).await?;
    let archive_bytes =
        download_cf_file_bytes(&file, Duration::from_secs(300)).await?;

    let manifest_bytes = archive_bytes.clone();
    let manifest: CfManifest = tokio::task::spawn_blocking(move || {
        let cursor = std::io::Cursor::new(manifest_bytes);
        let mut archive = zip::ZipArchive::new(cursor).map_err(|e| {
            cf_error(format!(
                "Failed to read modpack archive: {e}"
            ))
        })?;
        let manifest_entry =
            archive.by_name("manifest.json").map_err(|e| {
                cf_error(format!(
                    "Modpack archive is missing manifest.json: {e}"
                ))
            })?;
        serde_json::from_reader(manifest_entry).map_err(|e| {
            cf_error(format!("Failed to parse modpack manifest: {e}"))
        })
    })
    .await
    .map_err(|e| {
        cf_error(format!(
            "Modpack extraction task failed: {e}"
        ))
    })??;

    let loader = manifest
        .minecraft
        .mod_loaders
        .iter()
        .find(|l| l.primary)
        .or_else(|| manifest.minecraft.mod_loaders.first())
        .map(|l| parse_cf_loader(&l.id))
        .unwrap_or(ModLoader::Vanilla);

    let metadata = crate::api::instance::create(
        manifest.name.clone(),
        manifest.minecraft.version.clone(),
        loader,
        None,
        None,
        None,
        InstanceLink::Unmanaged,
    )
    .await?;
    let instance_id = metadata.instance.id.clone();

    let install_result = install_modpack_contents(
        &instance_id,
        &metadata.instance.path,
        &manifest,
        archive_bytes,
    )
    .await;

    if let Err(e) = install_result {
        let _ = crate::api::instance::remove(&instance_id).await;
        return Err(e);
    }

    crate::event::emit::emit_instance(
        &instance_id,
        crate::event::InstancePayloadType::Edited,
    )
    .await?;

    Ok(instance_id)
}

async fn install_modpack_contents(
    instance_id: &str,
    instance_relative_path: &str,
    manifest: &CfManifest,
    archive_bytes: bytes::Bytes,
) -> crate::Result<()> {
    let state = State::get().await?;
    let instance_path =
        state.directories.instances_dir().join(instance_relative_path);
    io::create_dir_all(&instance_path).await?;

    for entry in &manifest.files {
        let result: crate::Result<()> =
            match get_file(entry.project_id, entry.file_id).await {
                Ok(file) => {
                    download_and_add_to_instance(instance_id, &file, None)
                        .await
                        .map(|_| ())
                }
                Err(e) => Err(e),
            };

        if let Err(e) = result {
            if entry.required {
                return Err(e);
            }
            tracing::warn!(
                "Skipping optional modpack file {} (project {}): {e}",
                entry.file_id,
                entry.project_id
            );
        }
    }

    let overrides_prefix = format!("{}/", manifest.overrides);
    let instance_path_for_overrides = instance_path.clone();
    tokio::task::spawn_blocking(move || -> crate::Result<()> {
        let cursor = std::io::Cursor::new(archive_bytes);
        let mut archive = zip::ZipArchive::new(cursor).map_err(|e| {
            cf_error(format!(
                "Failed to reopen modpack archive: {e}"
            ))
        })?;

        for i in 0..archive.len() {
            let mut entry = archive.by_index(i).map_err(|e| {
                cf_error(format!(
                    "Failed to read archive entry: {e}"
                ))
            })?;
            let Ok(name) = entry.name().map(|name| name.to_string()) else {
                continue;
            };
            let Some(relative) = name.strip_prefix(&overrides_prefix) else {
                continue;
            };
            if relative.is_empty()
                || relative.split(['/', '\\']).any(|part| part == "..")
            {
                continue;
            }

            let out_path = instance_path_for_overrides.join(relative);
            if entry.is_dir() {
                std::fs::create_dir_all(&out_path)
                    .map_err(|e| IOError::with_path(e, &out_path))?;
            } else {
                if let Some(parent) = out_path.parent() {
                    std::fs::create_dir_all(parent)
                        .map_err(|e| IOError::with_path(e, parent))?;
                }
                let mut out_file = std::fs::File::create(&out_path)
                    .map_err(|e| IOError::with_path(e, &out_path))?;
                std::io::copy(&mut entry, &mut out_file)
                    .map_err(|e| IOError::with_path(e, &out_path))?;
            }
        }

        Ok(())
    })
    .await
    .map_err(|e| {
        cf_error(format!(
            "Overrides extraction task failed: {e}"
        ))
    })??;

    crate::launcher::install_minecraft_for_instance_id_with_reporter(
        instance_id,
        false,
        None,
        &state,
    )
    .await?;

    Ok(())
}
