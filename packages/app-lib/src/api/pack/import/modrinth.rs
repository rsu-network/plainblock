use std::path::{Path, PathBuf};

use sqlx::{
    Row,
    sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions},
};

use crate::{
    State,
    install::{InstallPhaseDetails, InstallProgressReporter},
    prelude::ModLoader,
    state::{AppliedContentSetPatch, EditInstance, InstanceInstallStage},
};

use super::{finish_import, recache_icon};

struct ModrinthInstanceMeta {
    name: String,
    icon_path: Option<String>,
    game_version: String,
    loader: ModLoader,
    loader_version: Option<String>,
}

/// Splits `<base>/profiles/<instance>` into the app directory and instance folder name.
fn split_instance_path(instance_path: &Path) -> Option<(PathBuf, String)> {
    let folder = instance_path.file_name()?.to_string_lossy().to_string();
    let base = instance_path.parent()?.parent()?.to_path_buf();
    Some((base, folder))
}

async fn open_pool(base_path: &Path) -> Option<SqlitePool> {
    let db = base_path.join("app.db");
    if !db.exists() {
        return None;
    }

    let options = SqliteConnectOptions::new().filename(db).read_only(true);
    SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .ok()
}

async fn read_meta(
    base_path: &Path,
    folder: &str,
) -> Option<ModrinthInstanceMeta> {
    let pool = open_pool(base_path).await?;

    let current = sqlx::query(
        "SELECT i.name, i.icon_path, c.game_version, c.loader, c.loader_version \
         FROM instances i \
         JOIN instance_content_sets c ON c.id = i.applied_content_set_id \
         WHERE i.path = ?1",
    )
    .bind(folder)
    .fetch_optional(&pool)
    .await;

    let legacy = if matches!(current, Ok(Some(_))) {
        None
    } else {
        sqlx::query(
            "SELECT name, icon_path, game_version, mod_loader AS loader, \
             mod_loader_version AS loader_version \
             FROM profiles WHERE path = ?1",
        )
        .bind(folder)
        .fetch_optional(&pool)
        .await
        .ok()
        .flatten()
    };

    let row = match current {
        Ok(Some(row)) => row,
        _ => legacy?,
    };

    let loader: String = row.try_get("loader").ok()?;
    let meta = ModrinthInstanceMeta {
        name: row.try_get("name").ok()?,
        icon_path: row.try_get("icon_path").ok().flatten(),
        game_version: row.try_get("game_version").ok()?,
        loader: ModLoader::from_string(&loader),
        loader_version: row.try_get("loader_version").ok().flatten(),
    };
    pool.close().await;
    Some(meta)
}

pub async fn is_valid_modrinth(instance_folder: PathBuf) -> bool {
    let Some((base, folder)) = split_instance_path(&instance_folder) else {
        return false;
    };
    read_meta(&base, &folder).await.is_some()
}

pub async fn import_modrinth(
    base_path: PathBuf,
    instance_folder: String,
    instance_id: &str,
    reporter: InstallProgressReporter,
    details: InstallPhaseDetails,
) -> crate::Result<()> {
    let meta = read_meta(&base_path, &instance_folder)
        .await
        .ok_or_else(|| {
            crate::ErrorKind::InputError(format!(
                "could not read instance '{instance_folder}' from the Modrinth App database"
            ))
        })?;

    let instance_dir = base_path.join("profiles").join(&instance_folder);

    let icon = match meta.icon_path.as_deref() {
        Some(icon) => recache_icon(PathBuf::from(icon)).await?,
        None => None,
    };

    let state = State::get().await?;

    let loader_version = if meta.loader != ModLoader::Vanilla {
        crate::launcher::get_loader_version_from_profile(
            &meta.game_version,
            meta.loader,
            meta.loader_version.as_deref(),
            &state.directories,
        )
        .await?
    } else {
        None
    };

    crate::api::instance::edit(
        instance_id,
        EditInstance {
            install_stage: Some(InstanceInstallStage::PackInstalling),
            name: Some(meta.name),
            icon_path: Some(
                icon.clone().map(|x| x.to_string_lossy().to_string()),
            ),
            content_set_patch: Some(AppliedContentSetPatch {
                source_kind: None,
                game_version: Some(meta.game_version.clone()),
                protocol_version: Some(None),
                loader: Some(meta.loader),
                loader_version: Some(loader_version.map(|x| x.id)),
            }),
            ..EditInstance::default()
        },
    )
    .await?;

    finish_import(instance_id, instance_dir, &state, reporter, details)
        .await?;

    Ok(())
}
