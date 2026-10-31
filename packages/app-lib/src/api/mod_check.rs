use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{BufReader, Read};
use std::path::Path;

use serde::Serialize;
use zip::ZipArchive;

use crate::state::ModLoader;

#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ModIssueKind {
    MissingDependency,
    Duplicate,
    WrongLoader,
    Unreadable,
}

#[derive(Serialize, Debug, Clone)]
pub struct ModIssue {
    pub kind: ModIssueKind,
    pub file: String,
    pub message: String,
}

#[derive(Default)]
struct ModInfo {
    file: String,
    ids: Vec<String>,
    depends: Vec<String>,
    is_fabric_style: bool,
    is_forge_style: bool,
    is_neoforge_style: bool,
}

const BUILTIN_IDS: &[&str] = &[
    "minecraft",
    "java",
    "fabricloader",
    "fabric-loader",
    "quilt_loader",
    "quilt-loader",
    "forge",
    "neoforge",
    "fml",
    "javafml",
    "lowcodefml",
    "mclanguage",
];

fn read_entry(
    archive: &mut ZipArchive<impl Read + std::io::Seek>,
    name: &str,
) -> Option<Vec<u8>> {
    let mut entry = archive.by_name(name).ok()?;
    let mut buffer = Vec::new();
    entry.read_to_end(&mut buffer).ok()?;
    Some(buffer)
}

fn parse_fabric_json(bytes: &[u8], info: &mut ModInfo) {
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(bytes) else {
        return;
    };
    info.is_fabric_style = true;
    if let Some(id) = value.get("id").and_then(|v| v.as_str()) {
        info.ids.push(id.to_string());
    }
    if let Some(provides) = value.get("provides").and_then(|v| v.as_array()) {
        for provided in provides {
            if let Some(id) = provided.as_str() {
                info.ids.push(id.to_string());
            }
        }
    }
    if let Some(depends) = value.get("depends").and_then(|v| v.as_object()) {
        info.depends.extend(depends.keys().cloned());
    }
}

fn parse_quilt_json(bytes: &[u8], info: &mut ModInfo) {
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(bytes) else {
        return;
    };
    info.is_fabric_style = true;
    let Some(loader) = value.get("quilt_loader") else {
        return;
    };
    if let Some(id) = loader.get("id").and_then(|v| v.as_str()) {
        info.ids.push(id.to_string());
    }
    if let Some(provides) = loader.get("provides").and_then(|v| v.as_array()) {
        for provided in provides {
            let id = provided
                .as_str()
                .or_else(|| provided.get("id").and_then(|v| v.as_str()));
            if let Some(id) = id {
                info.ids.push(id.to_string());
            }
        }
    }
    if let Some(depends) = loader.get("depends").and_then(|v| v.as_array()) {
        for dependency in depends {
            let optional = dependency
                .get("optional")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let id = dependency
                .as_str()
                .or_else(|| dependency.get("id").and_then(|v| v.as_str()));
            if let (Some(id), false) = (id, optional) {
                info.depends.push(id.to_string());
            }
        }
    }
}

fn parse_forge_toml(bytes: &[u8], info: &mut ModInfo) {
    let Ok(text) = std::str::from_utf8(bytes) else {
        return;
    };
    let Ok(value) = text.parse::<toml::Table>() else {
        return;
    };
    if let Some(mods) = value.get("mods").and_then(|v| v.as_array()) {
        for entry in mods {
            if let Some(id) = entry.get("modId").and_then(|v| v.as_str()) {
                info.ids.push(id.to_string());
            }
        }
    }
    let Some(dependencies) =
        value.get("dependencies").and_then(|v| v.as_table())
    else {
        return;
    };
    for entries in dependencies.values() {
        let Some(entries) = entries.as_array() else {
            continue;
        };
        for entry in entries {
            let Some(id) = entry.get("modId").and_then(|v| v.as_str()) else {
                continue;
            };
            let required = match entry.get("type").and_then(|v| v.as_str()) {
                Some(kind) => kind.eq_ignore_ascii_case("required"),
                None => entry
                    .get("mandatory")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false),
            };
            let side_ok = !matches!(
                entry.get("side").and_then(|v| v.as_str()),
                Some("SERVER")
            );
            if required && side_ok {
                info.depends.push(id.to_string());
            }
        }
    }
}

fn read_jar(path: &Path, depth: u8) -> Option<ModInfo> {
    let file = File::open(path).ok()?;
    let mut archive = ZipArchive::new(BufReader::new(file)).ok()?;
    let mut info = ModInfo {
        file: path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_default(),
        ..Default::default()
    };

    let mut nested_files = Vec::new();
    if let Some(bytes) = read_entry(&mut archive, "fabric.mod.json") {
        if let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes)
            && let Some(jars) = value.get("jars").and_then(|v| v.as_array())
        {
            for jar in jars {
                if let Some(name) = jar.get("file").and_then(|v| v.as_str()) {
                    nested_files.push(name.to_string());
                }
            }
        }
        parse_fabric_json(&bytes, &mut info);
    }
    if let Some(bytes) = read_entry(&mut archive, "quilt.mod.json") {
        parse_quilt_json(&bytes, &mut info);
    }
    if let Some(bytes) = read_entry(&mut archive, "META-INF/neoforge.mods.toml")
    {
        info.is_neoforge_style = true;
        parse_forge_toml(&bytes, &mut info);
    }
    if let Some(bytes) = read_entry(&mut archive, "META-INF/mods.toml") {
        info.is_forge_style = true;
        parse_forge_toml(&bytes, &mut info);
    }

    if depth == 0 {
        for nested in nested_files {
            let Some(bytes) = read_entry(&mut archive, &nested) else {
                continue;
            };
            let Ok(mut inner) =
                ZipArchive::new(std::io::Cursor::new(bytes))
            else {
                continue;
            };
            if let Some(inner_json) =
                read_entry(&mut inner, "fabric.mod.json")
            {
                let mut nested_info = ModInfo::default();
                parse_fabric_json(&inner_json, &mut nested_info);
                info.ids.extend(nested_info.ids);
            }
        }
    }

    Some(info)
}

fn scan_blocking(mods_dir: &Path, loader: ModLoader) -> Vec<ModIssue> {
    let Ok(read) = fs::read_dir(mods_dir) else {
        return Vec::new();
    };

    let mut issues = Vec::new();
    let mut mods: Vec<ModInfo> = Vec::new();
    let mut paths: Vec<_> = read
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "jar"))
        .collect();
    paths.sort();

    for path in paths {
        match read_jar(&path, 0) {
            Some(info) => mods.push(info),
            None => issues.push(ModIssue {
                kind: ModIssueKind::Unreadable,
                file: path
                    .file_name()
                    .map(|name| name.to_string_lossy().to_string())
                    .unwrap_or_default(),
                message: "This file is damaged or not a valid mod.".into(),
            }),
        }
    }

    let mut provided: HashSet<String> =
        BUILTIN_IDS.iter().map(|id| id.to_string()).collect();
    let mut owners: HashMap<String, Vec<String>> = HashMap::new();
    for info in &mods {
        let unique_ids: HashSet<&String> = info.ids.iter().collect();
        for id in unique_ids {
            provided.insert(id.clone());
        }
        if let Some(main_id) = info.ids.first() {
            owners
                .entry(main_id.clone())
                .or_default()
                .push(info.file.clone());
        }
    }

    for (id, files) in &owners {
        if files.len() > 1 {
            for file in files {
                issues.push(ModIssue {
                    kind: ModIssueKind::Duplicate,
                    file: file.clone(),
                    message: format!(
                        "The mod \"{id}\" is installed more than once: {}",
                        files.join(", ")
                    ),
                });
            }
        }
    }

    for info in &mods {
        let has_loader_metadata = info.is_fabric_style
            || info.is_forge_style
            || info.is_neoforge_style;
        let supported = match loader {
            ModLoader::Fabric | ModLoader::Quilt => info.is_fabric_style,
            ModLoader::Forge => info.is_forge_style,
            ModLoader::NeoForge => info.is_neoforge_style || info.is_forge_style,
            ModLoader::Vanilla => true,
        };
        if has_loader_metadata && !supported {
            issues.push(ModIssue {
                kind: ModIssueKind::WrongLoader,
                file: info.file.clone(),
                message: format!(
                    "This mod is not made for {}.",
                    loader.as_str()
                ),
            });
            continue;
        }

        let mut seen = HashSet::new();
        for dependency in &info.depends {
            if provided.contains(dependency) || !seen.insert(dependency) {
                continue;
            }
            issues.push(ModIssue {
                kind: ModIssueKind::MissingDependency,
                file: info.file.clone(),
                message: format!("Requires \"{dependency}\", which is not installed."),
            });
        }
    }

    issues.sort_by(|a, b| a.file.cmp(&b.file));
    issues
}

/// Scans the enabled mods of an instance for problems that would crash the game.
#[tracing::instrument]
pub async fn check(instance_id: &str) -> crate::Result<Vec<ModIssue>> {
    let metadata =
        crate::api::instance::get(instance_id).await?.ok_or_else(|| {
            crate::Error::from(crate::ErrorKind::LauncherError(format!(
                "Unknown instance id: {instance_id}"
            )))
        })?;
    let loader = metadata.applied_content_set.loader;
    if loader == ModLoader::Vanilla {
        return Ok(Vec::new());
    }
    let mods_dir = crate::api::instance::get_full_path(instance_id)
        .await?
        .join("mods");

    tokio::task::spawn_blocking(move || scan_blocking(&mods_dir, loader))
        .await
        .map_err(|e| {
            crate::ErrorKind::LauncherError(e.to_string()).into()
        })
}
