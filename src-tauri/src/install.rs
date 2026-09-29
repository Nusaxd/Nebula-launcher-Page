//! Installing a Minecraft version: client jar, libraries, assets, logging config.

use crate::bail;
use crate::download::{download_all, download_file, emit_stage, read_json_file, Job};
use crate::error::{Error, Result};
use crate::state::Shared;
use crate::versions::{current_os, load_manifest, rules_allow, Artifact, Library, VersionJson};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicU64;
use tauri::AppHandle;

const LIBRARIES_BASE: &str = "https://libraries.minecraft.net/";
const ASSETS_BASE: &str = "https://resources.download.minecraft.net/";

#[derive(Deserialize, Debug, Default)]
pub struct AssetObject {
    pub hash: String,
    #[serde(default)]
    pub size: u64,
}

#[derive(Deserialize, Debug, Default)]
pub struct AssetIndex {
    #[serde(default)]
    pub objects: HashMap<String, AssetObject>,
    #[serde(rename = "virtual", default)]
    pub is_virtual: bool,
    #[serde(default)]
    pub map_to_resources: bool,
}

#[derive(Default, Debug)]
pub struct ResolvedLib {
    pub artifact: Option<Job>,
    pub native: Option<Job>,
    pub exclude: Vec<String>,
}

pub fn version_dir(st: &Shared, version_id: &str) -> PathBuf {
    st.dirs.versions().join(version_id)
}

pub fn client_jar(st: &Shared, version_id: &str) -> PathBuf {
    version_dir(st, version_id).join(format!("{version_id}.jar"))
}

fn marker(st: &Shared, version_id: &str) -> PathBuf {
    version_dir(st, version_id).join(".nebula-installed")
}

pub fn is_installed(st: &Shared, version_id: &str) -> bool {
    marker(st, version_id).is_file() && client_jar(st, version_id).is_file()
}

pub fn installed_versions(st: &Shared) -> Vec<String> {
    let mut out = Vec::new();
    if let Ok(rd) = std::fs::read_dir(st.dirs.versions()) {
        for e in rd.flatten() {
            let id = e.file_name().to_string_lossy().to_string();
            if is_installed(st, &id) {
                out.push(id);
            }
        }
    }
    out
}

/// `group:artifact:version[:classifier]` -> maven repository relative path.
pub fn maven_path(name: &str) -> Option<String> {
    let parts: Vec<&str> = name.split(':').collect();
    if parts.len() < 3 {
        return None;
    }
    let group = parts[0].replace('.', "/");
    let artifact = parts[1];
    let (version, ext) = match parts[2].split_once('@') {
        Some((v, e)) => (v, e),
        None => (parts[2], "jar"),
    };
    let classifier = parts
        .get(3)
        .map(|c| format!("-{}", c.split('@').next().unwrap_or(c)))
        .unwrap_or_default();
    Some(format!("{group}/{artifact}/{version}/{artifact}-{version}{classifier}.{ext}"))
}

fn artifact_job(a: &Artifact, fallback_name: &str, libs_dir: &Path, base: &str) -> Option<Job> {
    let rel = if a.path.is_empty() {
        maven_path(fallback_name)?
    } else {
        a.path.clone()
    };
    let url = if a.url.is_empty() {
        format!("{base}{rel}")
    } else {
        a.url.clone()
    };
    Some(Job {
        url,
        path: libs_dir.join(&rel),
        sha1: if a.sha1.is_empty() { None } else { Some(a.sha1.clone()) },
        size: a.size,
    })
}

/// Work out which files a library needs on this platform (None = not applicable).
pub fn resolve_library(lib: &Library, libs_dir: &Path) -> Option<ResolvedLib> {
    let features = HashMap::new();
    if let Some(rules) = &lib.rules {
        if !rules_allow(rules, &features) {
            return None;
        }
    }
    let base = lib.url.clone().unwrap_or_else(|| LIBRARIES_BASE.to_string());
    let base = if base.ends_with('/') { base } else { format!("{base}/") };
    let dl = lib.downloads.as_ref();
    let mut out = ResolvedLib::default();

    if let Some(a) = dl.and_then(|d| d.artifact.as_ref()) {
        out.artifact = artifact_job(a, &lib.name, libs_dir, &base);
    } else if dl.is_none() && lib.natives.is_none() {
        // Very old / hand written profiles: plain maven coordinates.
        let empty = Artifact::default();
        out.artifact = artifact_job(&empty, &lib.name, libs_dir, &base);
    }

    if let Some(natives) = &lib.natives {
        if let Some(key) = natives.get(current_os()) {
            let arch = if cfg!(target_pointer_width = "64") { "64" } else { "32" };
            let key = key.replace("${arch}", arch);
            let classifier = dl.and_then(|d| d.classifiers.as_ref()).and_then(|c| c.get(&key));
            match classifier {
                Some(a) => {
                    let name = format!("{}:{}", lib.name, key);
                    out.native = artifact_job(a, &name, libs_dir, &base);
                }
                None if dl.is_none() => {
                    let name = format!("{}:{}", lib.name, key);
                    out.native = artifact_job(&Artifact::default(), &name, libs_dir, &base);
                }
                None => {}
            }
            out.exclude = lib.extract.as_ref().map(|e| e.exclude.clone()).unwrap_or_default();
        }
    }

    if out.artifact.is_none() && out.native.is_none() {
        return None;
    }
    Some(out)
}

pub fn resolve_all(vj: &VersionJson, libs_dir: &Path) -> Vec<ResolvedLib> {
    vj.libraries
        .iter()
        .filter_map(|l| resolve_library(l, libs_dir))
        .collect()
}

/// Fetches (and caches) the per-version metadata JSON.
pub async fn fetch_version_json(st: &Shared, version_id: &str) -> Result<VersionJson> {
    let file = version_dir(st, version_id).join(format!("{version_id}.json"));
    if let Ok(v) = read_json_file::<VersionJson>(&file).await {
        if !v.main_class.is_empty() {
            return Ok(v);
        }
    }
    let (manifest, _) = load_manifest(st, false).await?;
    let entry = manifest
        .versions
        .iter()
        .find(|e| e.id == version_id)
        .ok_or_else(|| Error::msg(format!("Unknown Minecraft version: {version_id}")))?;
    let job = Job {
        url: entry.url.clone(),
        path: file.clone(),
        sha1: if entry.sha1.is_empty() { None } else { Some(entry.sha1.clone()) },
        size: 0,
    };
    download_file(&st.http, &job, &AtomicU64::new(0)).await?;
    let v: VersionJson = read_json_file(&file).await?;
    Ok(v)
}

pub fn asset_index_path(st: &Shared, index_id: &str) -> PathBuf {
    st.dirs.assets().join("indexes").join(format!("{index_id}.json"))
}

pub fn log_config_path(st: &Shared, file_id: &str) -> PathBuf {
    st.dirs.assets().join("log_configs").join(file_id)
}

/// Make sure every file required to run `version_id` is present and verified.
/// `id` is the identifier used for progress events (instance id or version id).
pub async fn ensure_version(app: &AppHandle, st: &Shared, id: &str, version_id: &str) -> Result<VersionJson> {
    emit_stage(app, id, "Fetching version info");
    let vj = fetch_version_json(st, version_id).await?;
    let conc = st.read(|s| s.settings.concurrency) as usize;

    // 1. Client jar (the real Mojang client .jar)
    let client = vj
        .downloads
        .as_ref()
        .and_then(|d| d.client.clone())
        .ok_or_else(|| Error::msg(format!("Version {version_id} has no client download")))?;
    let jar_job = Job {
        url: client.url,
        path: client_jar(st, version_id),
        sha1: if client.sha1.is_empty() { None } else { Some(client.sha1) },
        size: client.size,
    };
    download_all(app, &st.http, id, "Downloading Minecraft client", vec![jar_job], conc).await?;

    // 2. Libraries (+ native classifiers)
    let libs_dir = st.dirs.libraries();
    let mut jobs = Vec::new();
    for r in resolve_all(&vj, &libs_dir) {
        if let Some(j) = r.artifact {
            jobs.push(j);
        }
        if let Some(j) = r.native {
            jobs.push(j);
        }
    }
    download_all(app, &st.http, id, "Downloading libraries", jobs, conc).await?;

    // 3. Logging configuration
    if let Some(file) = vj
        .logging
        .as_ref()
        .and_then(|l| l.client.as_ref())
        .and_then(|c| c.file.as_ref())
    {
        if !file.id.is_empty() && !file.url.is_empty() {
            let job = Job {
                url: file.url.clone(),
                path: log_config_path(st, &file.id),
                sha1: if file.sha1.is_empty() { None } else { Some(file.sha1.clone()) },
                size: file.size,
            };
            download_all(app, &st.http, id, "Downloading logging config", vec![job], conc).await?;
        }
    }

    // 4. Assets
    if let Some(ai) = &vj.asset_index {
        let index_id = if ai.id.is_empty() { "legacy".to_string() } else { ai.id.clone() };
        let index_job = Job {
            url: ai.url.clone(),
            path: asset_index_path(st, &index_id),
            sha1: if ai.sha1.is_empty() { None } else { Some(ai.sha1.clone()) },
            size: ai.size,
        };
        download_all(app, &st.http, id, "Downloading asset index", vec![index_job], conc).await?;
        let index: AssetIndex = read_json_file(&asset_index_path(st, &index_id)).await?;
        let objects_dir = st.dirs.assets().join("objects");
        let jobs: Vec<Job> = index
            .objects
            .values()
            .filter(|o| o.hash.len() >= 2)
            .map(|o| Job {
                url: format!("{ASSETS_BASE}{}/{}", &o.hash[..2], o.hash),
                path: objects_dir.join(&o.hash[..2]).join(&o.hash),
                sha1: Some(o.hash.clone()),
                size: o.size,
            })
            .collect();
        download_all(app, &st.http, id, "Downloading assets", jobs, conc).await?;
    }

    tokio::fs::write(marker(st, version_id), b"ok").await?;
    Ok(vj)
}

pub async fn uninstall_version(st: &Shared, version_id: &str) -> Result<()> {
    if version_id.is_empty() || version_id.contains('/') || version_id.contains('\\') || version_id.contains("..") {
        bail!("Invalid version id");
    }
    let dir = version_dir(st, version_id);
    if dir.exists() {
        tokio::fs::remove_dir_all(dir).await?;
    }
    Ok(())
}

/// Extract the native libraries (.dll/.so/.dylib) of a jar into `dest`.
pub fn extract_natives(jar: &Path, dest: &Path, exclude: &[String]) -> Result<()> {
    let file = std::fs::File::open(jar)?;
    let mut archive = zip::ZipArchive::new(file)?;
    std::fs::create_dir_all(dest)?;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        if entry.is_dir() {
            continue;
        }
        let name = entry.name().to_string();
        if exclude.iter().any(|ex| name.starts_with(ex.as_str())) {
            continue;
        }
        let Some(file_name) = Path::new(&name).file_name() else {
            continue;
        };
        let out_path = dest.join(file_name);
        let mut out = std::fs::File::create(&out_path)?;
        std::io::copy(&mut entry, &mut out)?;
    }
    Ok(())
}

/// Legacy (pre 1.7) versions read assets from a plain directory tree instead of the hashed store.
/// Returns the directory to use for `${game_assets}`.
pub async fn prepare_legacy_assets(st: &Shared, index_id: &str, game_dir: &Path) -> Result<Option<PathBuf>> {
    let index: AssetIndex = match read_json_file(&asset_index_path(st, index_id)).await {
        Ok(i) => i,
        Err(_) => return Ok(None),
    };
    if !index.is_virtual && !index.map_to_resources {
        return Ok(None);
    }
    let target = if index.map_to_resources {
        game_dir.join("resources")
    } else {
        st.dirs.assets().join("virtual").join(index_id)
    };
    let objects_dir = st.dirs.assets().join("objects");
    let target2 = target.clone();
    tokio::task::spawn_blocking(move || -> Result<()> {
        for (name, obj) in index.objects.iter() {
            if obj.hash.len() < 2 || name.contains("..") {
                continue;
            }
            let dest = target2.join(name);
            if dest.is_file() {
                continue;
            }
            let src = objects_dir.join(&obj.hash[..2]).join(&obj.hash);
            if !src.is_file() {
                continue;
            }
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(&src, &dest)?;
        }
        Ok(())
    })
    .await
    .map_err(|e| Error::msg(e.to_string()))??;
    Ok(Some(target))
}
