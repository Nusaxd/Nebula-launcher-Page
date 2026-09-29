//! Mojang version manifest and version JSON models.

use crate::download::{get_json, read_json_file};
use crate::error::Result;
use crate::state::Shared;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const MANIFEST_URL: &str = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Latest {
    #[serde(default)]
    pub release: String,
    #[serde(default)]
    pub snapshot: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ManifestEntry {
    pub id: String,
    /// release | snapshot | old_beta | old_alpha
    #[serde(rename = "type")]
    pub kind: String,
    pub url: String,
    #[serde(rename = "releaseTime", default)]
    pub release_time: String,
    #[serde(default)]
    pub sha1: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Manifest {
    #[serde(default)]
    pub latest: Latest,
    pub versions: Vec<ManifestEntry>,
}

/// Loads the manifest from the network, falling back to the on-disk cache.
/// The bool is `true` when the cached copy was used (offline).
pub async fn load_manifest(st: &Shared, force_refresh: bool) -> Result<(Manifest, bool)> {
    let cache = st.dirs.meta().join("version_manifest_v2.json");
    let _ = force_refresh;
    match get_json::<Manifest>(&st.http, MANIFEST_URL).await {
        Ok(m) => {
            if let Ok(json) = serde_json::to_vec(&m) {
                let _ = tokio::fs::create_dir_all(st.dirs.meta()).await;
                let _ = tokio::fs::write(&cache, json).await;
            }
            Ok((m, false))
        }
        Err(e) => match read_json_file::<Manifest>(&cache).await {
            Ok(m) => Ok((m, true)),
            Err(_) => Err(e),
        },
    }
}

// ----------------------------------------------------------------------------
// Version JSON (per version metadata)
// ----------------------------------------------------------------------------

#[derive(Deserialize, Clone, Debug, Default)]
pub struct Download {
    #[serde(default)]
    pub sha1: String,
    #[serde(default)]
    pub size: u64,
    #[serde(default)]
    pub url: String,
}

#[derive(Deserialize, Clone, Debug, Default)]
pub struct Artifact {
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub sha1: String,
    #[serde(default)]
    pub size: u64,
    #[serde(default)]
    pub url: String,
}

#[derive(Deserialize, Clone, Debug, Default)]
pub struct OsRule {
    pub name: Option<String>,
    pub arch: Option<String>,
    pub version: Option<String>,
}

#[derive(Deserialize, Clone, Debug, Default)]
pub struct Rule {
    #[serde(default)]
    pub action: String,
    pub os: Option<OsRule>,
    pub features: Option<HashMap<String, bool>>,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(untagged)]
pub enum StringOrList {
    One(String),
    Many(Vec<String>),
}

impl StringOrList {
    pub fn to_vec(&self) -> Vec<String> {
        match self {
            StringOrList::One(s) => vec![s.clone()],
            StringOrList::Many(v) => v.clone(),
        }
    }
}

#[derive(Deserialize, Clone, Debug)]
#[serde(untagged)]
pub enum Argument {
    Plain(String),
    Conditional {
        #[serde(default)]
        rules: Vec<Rule>,
        value: StringOrList,
    },
}

#[derive(Deserialize, Clone, Debug, Default)]
pub struct Arguments {
    #[serde(default)]
    pub game: Vec<Argument>,
    #[serde(default)]
    pub jvm: Vec<Argument>,
}

#[derive(Deserialize, Clone, Debug, Default)]
pub struct AssetIndexRef {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub sha1: String,
    #[serde(default)]
    pub size: u64,
    #[serde(default)]
    pub url: String,
}

#[derive(Deserialize, Clone, Debug, Default)]
pub struct LibDownloads {
    pub artifact: Option<Artifact>,
    pub classifiers: Option<HashMap<String, Artifact>>,
}

#[derive(Deserialize, Clone, Debug, Default)]
pub struct Extract {
    #[serde(default)]
    pub exclude: Vec<String>,
}

#[derive(Deserialize, Clone, Debug, Default)]
pub struct Library {
    pub name: String,
    pub downloads: Option<LibDownloads>,
    pub rules: Option<Vec<Rule>>,
    pub natives: Option<HashMap<String, String>>,
    pub extract: Option<Extract>,
    pub url: Option<String>,
}

#[derive(Deserialize, Clone, Debug, Default)]
pub struct Downloads {
    pub client: Option<Download>,
}

#[derive(Deserialize, Clone, Debug, Default)]
pub struct LogFile {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub sha1: String,
    #[serde(default)]
    pub size: u64,
    #[serde(default)]
    pub url: String,
}

#[derive(Deserialize, Clone, Debug, Default)]
pub struct LogClient {
    #[serde(default)]
    pub argument: String,
    pub file: Option<LogFile>,
}

#[derive(Deserialize, Clone, Debug, Default)]
pub struct Logging {
    pub client: Option<LogClient>,
}

#[derive(Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct JavaVersion {
    #[serde(default)]
    pub component: String,
    #[serde(default)]
    pub major_version: u32,
}

#[derive(Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct VersionJson {
    #[serde(default)]
    pub id: String,
    #[serde(rename = "type", default)]
    pub kind: String,
    #[serde(default)]
    pub main_class: String,
    pub minecraft_arguments: Option<String>,
    pub arguments: Option<Arguments>,
    pub asset_index: Option<AssetIndexRef>,
    pub assets: Option<String>,
    pub downloads: Option<Downloads>,
    #[serde(default)]
    pub libraries: Vec<Library>,
    pub logging: Option<Logging>,
    pub java_version: Option<JavaVersion>,
}

impl VersionJson {
    pub fn java_component(&self) -> String {
        match &self.java_version {
            Some(j) if !j.component.is_empty() => j.component.clone(),
            // Versions without the field predate bundled runtimes and use Java 8.
            _ => "jre-legacy".to_string(),
        }
    }
}

// ----------------------------------------------------------------------------
// Rules
// ----------------------------------------------------------------------------

pub fn current_os() -> &'static str {
    if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "osx"
    } else {
        "linux"
    }
}

fn arch_matches(rule_arch: &str) -> bool {
    match std::env::consts::ARCH {
        "x86_64" => rule_arch == "x86_64" || rule_arch == "amd64",
        "x86" => rule_arch == "x86" || rule_arch == "i386",
        "aarch64" => rule_arch == "arm64" || rule_arch == "aarch64",
        other => rule_arch == other,
    }
}

fn rule_matches(rule: &Rule, features: &HashMap<String, bool>) -> bool {
    if let Some(os) = &rule.os {
        if let Some(name) = &os.name {
            if name != current_os() {
                return false;
            }
        }
        if let Some(arch) = &os.arch {
            if !arch_matches(arch) {
                return false;
            }
        }
    }
    if let Some(req) = &rule.features {
        for (k, v) in req {
            if features.get(k).copied().unwrap_or(false) != *v {
                return false;
            }
        }
    }
    true
}

/// Evaluate Mojang style rules: no rules = allowed; otherwise the last matching rule wins.
pub fn rules_allow(rules: &[Rule], features: &HashMap<String, bool>) -> bool {
    if rules.is_empty() {
        return true;
    }
    let mut allowed = false;
    for r in rules {
        if rule_matches(r, features) {
            allowed = r.action == "allow";
        }
    }
    allowed
}
