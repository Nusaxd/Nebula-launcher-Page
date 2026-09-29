//! Java runtime handling: Mojang's bundled runtimes with a fallback to the system Java.

use crate::download::{download_all, emit_stage, get_json, Job};
use crate::error::Result;
use crate::state::Shared;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tauri::AppHandle;

const RUNTIME_INDEX_URL: &str =
    "https://launchermeta.mojang.com/v1/products/java-runtime/2ec0cc96c44e5a76b9c8b7c39df7210883d12871/all.json";

#[derive(Deserialize, Debug, Default, Clone)]
struct RtDownload {
    #[serde(default)]
    sha1: String,
    #[serde(default)]
    size: u64,
    #[serde(default)]
    url: String,
}

#[derive(Deserialize, Debug)]
struct RtEntry {
    manifest: RtDownload,
}

#[derive(Deserialize, Debug, Default)]
struct RtFileDownloads {
    raw: Option<RtDownload>,
}

#[derive(Deserialize, Debug)]
struct RtFile {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    executable: bool,
    downloads: Option<RtFileDownloads>,
    target: Option<String>,
}

#[derive(Deserialize, Debug)]
struct RtManifest {
    files: HashMap<String, RtFile>,
}

/// Platform key used by Mojang's runtime index.
pub fn platform_key() -> Option<&'static str> {
    let arch = std::env::consts::ARCH;
    if cfg!(target_os = "windows") {
        match arch {
            "x86_64" => Some("windows-x64"),
            "aarch64" => Some("windows-arm64"),
            "x86" => Some("windows-x86"),
            _ => None,
        }
    } else if cfg!(target_os = "macos") {
        match arch {
            "aarch64" => Some("mac-os-arm64"),
            _ => Some("mac-os"),
        }
    } else {
        match arch {
            "x86_64" => Some("linux"),
            "x86" => Some("linux-i386"),
            _ => None,
        }
    }
}

fn exe_name() -> &'static str {
    if cfg!(windows) {
        "java.exe"
    } else {
        "java"
    }
}

/// Find the java executable inside an extracted runtime directory.
pub fn java_bin_in(dir: &Path) -> Option<PathBuf> {
    let candidates = [
        dir.join("bin").join(exe_name()),
        dir.join("jre.bundle").join("Contents").join("Home").join("bin").join(exe_name()),
        dir.join("Contents").join("Home").join("bin").join(exe_name()),
    ];
    candidates.into_iter().find(|p| p.is_file())
}

fn runtime_dir(st: &Shared, component: &str) -> PathBuf {
    st.dirs.runtimes().join(component)
}

/// Returns the path of an installed managed runtime, if any.
pub fn installed_runtime(st: &Shared, component: &str) -> Option<PathBuf> {
    let dir = runtime_dir(st, component);
    if dir.join(".nebula-installed").is_file() {
        java_bin_in(&dir)
    } else {
        None
    }
}

/// Download (if needed) the Mojang runtime `component`. Returns `None` when Mojang
/// does not offer this runtime for the current platform.
pub async fn ensure_runtime(app: &AppHandle, st: &Shared, id: &str, component: &str) -> Result<Option<PathBuf>> {
    if let Some(p) = installed_runtime(st, component) {
        return Ok(Some(p));
    }
    let Some(platform) = platform_key() else {
        return Ok(None);
    };

    emit_stage(app, id, &format!("Looking up Java runtime ({component})"));
    let index: HashMap<String, HashMap<String, Vec<RtEntry>>> = get_json(&st.http, RUNTIME_INDEX_URL).await?;
    let Some(entry) = index
        .get(platform)
        .and_then(|p| p.get(component))
        .and_then(|list| list.first())
    else {
        return Ok(None);
    };

    let manifest: RtManifest = get_json(&st.http, &entry.manifest.url).await?;
    let dir = runtime_dir(st, component);
    tokio::fs::create_dir_all(&dir).await?;

    let mut jobs = Vec::new();
    let mut executables = Vec::new();
    let mut links = Vec::new();
    let mut dirs = Vec::new();
    for (rel, f) in manifest.files.iter() {
        if rel.contains("..") {
            continue;
        }
        let path = dir.join(rel);
        match f.kind.as_str() {
            "directory" => dirs.push(path),
            "link" => {
                if let Some(t) = &f.target {
                    links.push((path, t.clone()));
                }
            }
            _ => {
                if let Some(raw) = f.downloads.as_ref().and_then(|d| d.raw.as_ref()) {
                    if f.executable {
                        executables.push(path.clone());
                    }
                    jobs.push(Job {
                        url: raw.url.clone(),
                        path,
                        sha1: if raw.sha1.is_empty() { None } else { Some(raw.sha1.clone()) },
                        size: raw.size,
                    });
                }
            }
        }
    }

    let conc = st.read(|s| s.settings.concurrency) as usize;
    download_all(app, &st.http, id, &format!("Downloading Java runtime ({component})"), jobs, conc).await?;

    let dir2 = dir.clone();
    tokio::task::spawn_blocking(move || -> Result<()> {
        for d in dirs {
            std::fs::create_dir_all(d)?;
        }
        for (path, target) in links {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            if std::fs::symlink_metadata(&path).is_ok() {
                continue;
            }
            #[cfg(unix)]
            {
                let _ = std::os::unix::fs::symlink(&target, &path);
            }
            #[cfg(not(unix))]
            {
                // Symlinks are not used by the Windows runtimes.
                let _ = target;
            }
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            for p in executables {
                if let Ok(meta) = std::fs::metadata(&p) {
                    let mut perms = meta.permissions();
                    perms.set_mode(0o755);
                    let _ = std::fs::set_permissions(&p, perms);
                }
            }
        }
        #[cfg(not(unix))]
        {
            let _ = executables;
        }
        std::fs::write(dir2.join(".nebula-installed"), b"ok")?;
        Ok(())
    })
    .await
    .map_err(|e| crate::error::Error::msg(e.to_string()))??;

    Ok(java_bin_in(&dir))
}

/// Best effort lookup of a Java on the system.
pub fn system_java() -> String {
    if let Ok(home) = std::env::var("JAVA_HOME") {
        let p = Path::new(&home).join("bin").join(exe_name());
        if p.is_file() {
            return p.to_string_lossy().to_string();
        }
    }
    "java".to_string()
}

/// Runs `java -version` and returns the first line of its output.
pub async fn probe(path: &str) -> Result<String> {
    let mut cmd = tokio::process::Command::new(path);
    cmd.arg("-version");
    #[cfg(windows)]
    {
        cmd.creation_flags(0x0800_0000);
    }
    let out = cmd.output().await?;
    let text = String::from_utf8_lossy(&out.stderr).to_string() + &String::from_utf8_lossy(&out.stdout);
    let first = text.lines().next().unwrap_or("").trim().to_string();
    if first.is_empty() {
        crate::bail!("`{}` did not print a Java version", path);
    }
    Ok(first)
}
