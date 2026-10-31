use std::fs::{self, File};
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};

use serde::Serialize;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

const BACKUP_DIR: &str = "plainblock-backups";
const MAX_BACKUPS_PER_WORLD: usize = 10;

#[derive(Serialize, Debug, Clone)]
pub struct WorldBackup {
    pub world: String,
    pub file_name: String,
    pub created: u64,
    pub size: u64,
}

fn backup_error(message: impl Into<String>) -> crate::Error {
    crate::ErrorKind::LauncherError(message.into()).into()
}

fn validate_component(value: &str) -> crate::Result<()> {
    if value.is_empty()
        || value.contains(['/', '\\'])
        || value == "."
        || value == ".."
        || value.contains("..")
    {
        return Err(backup_error("Invalid backup name"));
    }
    Ok(())
}

fn timestamp_name() -> String {
    chrono::Local::now().format("%Y-%m-%d_%H-%M-%S").to_string()
}

fn add_directory(
    archive: &mut ZipWriter<BufWriter<File>>,
    root: &Path,
    current: &Path,
    options: SimpleFileOptions,
) -> crate::Result<()> {
    for entry in fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();
        let relative = path
            .strip_prefix(root)
            .map_err(|_| backup_error("Invalid world path"))?
            .to_string_lossy()
            .replace('\\', "/");

        if entry.file_type()?.is_dir() {
            archive
                .add_directory(format!("{relative}/"), options)
                .map_err(std::io::Error::from)?;
            add_directory(archive, root, &path, options)?;
        } else {
            if relative == "session.lock" {
                continue;
            }
            let Ok(mut input) = File::open(&path) else {
                continue;
            };
            archive
                .start_file(&relative, options.large_file(true))
                .map_err(std::io::Error::from)?;
            std::io::copy(&mut input, archive)?;
        }
    }
    Ok(())
}

fn create_backup_blocking(
    instance_dir: &Path,
    world: &str,
    label: Option<&str>,
) -> crate::Result<WorldBackup> {
    validate_component(world)?;
    let world_dir = instance_dir.join("saves").join(world);
    if !world_dir.join("level.dat").exists() {
        return Err(backup_error(format!("\"{world}\" is not a Minecraft world")));
    }

    let target_dir = instance_dir.join(BACKUP_DIR).join(world);
    fs::create_dir_all(&target_dir)?;
    let file_name = match label {
        Some(label) => format!("{}_{label}.zip", timestamp_name()),
        None => format!("{}.zip", timestamp_name()),
    };
    let target = target_dir.join(&file_name);
    let partial = target_dir.join(format!("{file_name}.partial"));

    let result = (|| -> crate::Result<()> {
        let mut archive =
            ZipWriter::new(BufWriter::new(File::create(&partial)?));
        let options = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Deflated);
        add_directory(&mut archive, &world_dir, &world_dir, options)?;
        archive.finish().map_err(std::io::Error::from)?;
        Ok(())
    })();

    if let Err(error) = result {
        let _ = fs::remove_file(&partial);
        return Err(error);
    }
    fs::rename(&partial, &target)?;
    prune(&target_dir);

    let metadata = fs::metadata(&target)?;
    Ok(WorldBackup {
        world: world.to_string(),
        file_name,
        created: modified_secs(&metadata),
        size: metadata.len(),
    })
}

fn modified_secs(metadata: &fs::Metadata) -> u64 {
    metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn prune(world_backup_dir: &Path) {
    let Ok(read) = fs::read_dir(world_backup_dir) else {
        return;
    };
    let mut automatic: Vec<PathBuf> = read
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension().is_some_and(|ext| ext == "zip")
                && !path
                    .file_name()
                    .is_some_and(|name| name.to_string_lossy().contains("_keep"))
        })
        .collect();
    automatic.sort();
    while automatic.len() > MAX_BACKUPS_PER_WORLD {
        let oldest = automatic.remove(0);
        let _ = fs::remove_file(oldest);
    }
}

fn list_worlds(instance_dir: &Path) -> Vec<String> {
    let Ok(read) = fs::read_dir(instance_dir.join("saves")) else {
        return Vec::new();
    };
    read.filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().join("level.dat").exists())
        .map(|entry| entry.file_name().to_string_lossy().to_string())
        .collect()
}

fn list_backups_blocking(instance_dir: &Path) -> Vec<WorldBackup> {
    let Ok(worlds) = fs::read_dir(instance_dir.join(BACKUP_DIR)) else {
        return Vec::new();
    };
    let mut backups = Vec::new();
    for world in worlds.filter_map(|entry| entry.ok()) {
        let world_name = world.file_name().to_string_lossy().to_string();
        let Ok(files) = fs::read_dir(world.path()) else {
            continue;
        };
        for file in files.filter_map(|entry| entry.ok()) {
            let path = file.path();
            if !path.extension().is_some_and(|ext| ext == "zip") {
                continue;
            }
            let Ok(metadata) = file.metadata() else {
                continue;
            };
            backups.push(WorldBackup {
                world: world_name.clone(),
                file_name: file.file_name().to_string_lossy().to_string(),
                created: modified_secs(&metadata),
                size: metadata.len(),
            });
        }
    }
    backups.sort_by(|a, b| b.created.cmp(&a.created));
    backups
}

fn is_unchanged_since_last_backup(instance_dir: &Path, world: &str) -> bool {
    let level_modified = fs::metadata(
        instance_dir.join("saves").join(world).join("level.dat"),
    )
    .map(|metadata| modified_secs(&metadata))
    .unwrap_or(u64::MAX);
    let newest_backup = list_backups_blocking(instance_dir)
        .into_iter()
        .filter(|backup| backup.world == world)
        .map(|backup| backup.created)
        .max()
        .unwrap_or(0);
    newest_backup >= level_modified
}

fn restore_backup_blocking(
    instance_dir: &Path,
    world: &str,
    file_name: &str,
) -> crate::Result<()> {
    validate_component(world)?;
    validate_component(file_name)?;
    let backup_path = instance_dir.join(BACKUP_DIR).join(world).join(file_name);
    if !backup_path.exists() {
        return Err(backup_error("This backup no longer exists"));
    }

    let saves = instance_dir.join("saves");
    let target = saves.join(world);
    let staging = saves.join(format!(".{world}.restoring"));
    if staging.exists() {
        fs::remove_dir_all(&staging)?;
    }
    fs::create_dir_all(&staging)?;

    let extract = (|| -> crate::Result<()> {
        let mut archive = ZipArchive::new(BufReader::new(File::open(
            &backup_path,
        )?))
        .map_err(|e| backup_error(format!("Backup is damaged: {e}")))?;
        for index in 0..archive.len() {
            let mut entry = archive
                .by_index(index)
                .map_err(|e| backup_error(format!("Backup is damaged: {e}")))?;
            let Some(relative) = entry.enclosed_name() else {
                continue;
            };
            let out_path = staging.join(relative);
            if entry.is_dir() {
                fs::create_dir_all(&out_path)?;
            } else {
                if let Some(parent) = out_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                let mut output = File::create(&out_path)?;
                std::io::copy(&mut entry, &mut output)?;
            }
        }
        if !staging.join("level.dat").exists() {
            return Err(backup_error("Backup does not contain a world"));
        }
        Ok(())
    })();

    if let Err(error) = extract {
        let _ = fs::remove_dir_all(&staging);
        return Err(error);
    }

    if target.exists() {
        let _ = create_backup_blocking(instance_dir, world, Some("before-restore_keep"));
        fs::remove_dir_all(&target)?;
    }
    fs::rename(&staging, &target)?;
    Ok(())
}

#[tracing::instrument]
pub async fn list(instance_id: &str) -> crate::Result<Vec<WorldBackup>> {
    let instance_dir = super::instance::get_full_path(instance_id).await?;
    tokio::task::spawn_blocking(move || list_backups_blocking(&instance_dir))
        .await
        .map_err(|e| backup_error(e.to_string()))
}

#[tracing::instrument]
pub async fn create(
    instance_id: &str,
    world: &str,
) -> crate::Result<WorldBackup> {
    let instance_dir = super::instance::get_full_path(instance_id).await?;
    let world = world.to_string();
    tokio::task::spawn_blocking(move || {
        create_backup_blocking(&instance_dir, &world, Some("manual_keep"))
    })
    .await
    .map_err(|e| backup_error(e.to_string()))?
}

#[tracing::instrument]
pub async fn restore(
    instance_id: &str,
    world: &str,
    file_name: &str,
) -> crate::Result<()> {
    let instance_dir = super::instance::get_full_path(instance_id).await?;
    let world = world.to_string();
    let file_name = file_name.to_string();
    tokio::task::spawn_blocking(move || {
        restore_backup_blocking(&instance_dir, &world, &file_name)
    })
    .await
    .map_err(|e| backup_error(e.to_string()))?
}

#[tracing::instrument]
pub async fn delete(
    instance_id: &str,
    world: &str,
    file_name: &str,
) -> crate::Result<()> {
    validate_component(world)?;
    validate_component(file_name)?;
    let instance_dir = super::instance::get_full_path(instance_id).await?;
    let path = instance_dir.join(BACKUP_DIR).join(world).join(file_name);
    tokio::task::spawn_blocking(move || fs::remove_file(path))
        .await
        .map_err(|e| backup_error(e.to_string()))??;
    Ok(())
}

/// Backs up every world of an instance. Called right before the game starts so
/// that a bad session can always be rolled back.
pub async fn backup_all_worlds(instance_dir: PathBuf) {
    let _ = tokio::task::spawn_blocking(move || {
        for world in list_worlds(&instance_dir) {
            if is_unchanged_since_last_backup(&instance_dir, &world) {
                continue;
            }
            if let Err(error) =
                create_backup_blocking(&instance_dir, &world, None)
            {
                tracing::warn!("World backup of {world} failed: {error}");
            }
        }
    })
    .await;
}
