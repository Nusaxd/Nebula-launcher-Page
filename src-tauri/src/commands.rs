//! Tauri commands exposed to the Svelte frontend.

use crate::auth;
use crate::bail;
use crate::download::emit_stage;
use crate::error::{Error, Result};
use crate::install;
use crate::java;
use crate::launch;
use crate::models::*;
use crate::state::{now_secs, BusyGuard, Shared};
use crate::versions::{self, ManifestEntry};
use serde::Serialize;
use std::path::Path;
use tauri::{AppHandle, State};

// ----------------------------------------------------------------------------
// Bootstrap / settings
// ----------------------------------------------------------------------------

fn bootstrap_data(st: &Shared) -> Bootstrap {
    st.read(|s| Bootstrap {
        settings: s.settings.clone(),
        accounts: s.accounts.iter().map(AccountView::from).collect(),
        instances: s.instances.clone(),
        selected_account: s.selected_account.clone(),
        selected_instance: s.selected_instance.clone(),
        data_dir: st.dirs.root.to_string_lossy().to_string(),
        os: std::env::consts::OS.to_string(),
        app_version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

#[tauri::command]
pub fn bootstrap(state: State<'_, Shared>) -> Bootstrap {
    bootstrap_data(state.inner())
}

#[tauri::command]
pub fn update_settings(state: State<'_, Shared>, settings: Settings) -> Result<Settings> {
    let mut settings = settings;
    settings.default_memory_mb = settings.default_memory_mb.clamp(512, 65536);
    settings.concurrency = settings.concurrency.clamp(1, 64);
    let s2 = settings.clone();
    state.mutate(|s| s.settings = s2)?;
    Ok(settings)
}

#[tauri::command]
pub async fn test_java(state: State<'_, Shared>, path: String) -> Result<String> {
    let p = if path.trim().is_empty() {
        java::system_java()
    } else {
        path.trim().to_string()
    };
    let _ = &state;
    java::probe(&p).await
}

#[tauri::command]
pub fn running_instances(state: State<'_, Shared>) -> Vec<String> {
    state.running.lock().unwrap().keys().cloned().collect()
}

// ----------------------------------------------------------------------------
// Versions
// ----------------------------------------------------------------------------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionList {
    pub latest_release: String,
    pub latest_snapshot: String,
    pub versions: Vec<ManifestEntry>,
    pub installed: Vec<String>,
    pub offline: bool,
}

#[tauri::command]
pub async fn list_versions(state: State<'_, Shared>, refresh: bool) -> Result<VersionList> {
    let st = state.inner().clone();
    let (manifest, offline) = versions::load_manifest(&st, refresh).await?;
    Ok(VersionList {
        latest_release: manifest.latest.release,
        latest_snapshot: manifest.latest.snapshot,
        versions: manifest.versions,
        installed: install::installed_versions(&st),
        offline,
    })
}

#[tauri::command]
pub async fn install_version(app: AppHandle, state: State<'_, Shared>, version_id: String) -> Result<()> {
    let st = state.inner().clone();
    let key = format!("version:{version_id}");
    let Some(_guard) = BusyGuard::acquire(&st, &key) else {
        bail!("{version_id} is already being installed");
    };
    let vj = install::ensure_version(&app, &st, &version_id, &version_id).await?;
    if st.read(|s| s.settings.managed_java) {
        let component = vj.java_component();
        // Not fatal: the game can still run with a system Java.
        let _ = java::ensure_runtime(&app, &st, &version_id, &component).await;
    }
    emit_stage(&app, &version_id, "Done");
    Ok(())
}

#[tauri::command]
pub async fn uninstall_version(state: State<'_, Shared>, version_id: String) -> Result<()> {
    let st = state.inner().clone();
    install::uninstall_version(&st, &version_id).await
}

// ----------------------------------------------------------------------------
// Instances
// ----------------------------------------------------------------------------

fn sanitize_instance(mut inst: Instance) -> Result<Instance> {
    inst.name = inst.name.trim().to_string();
    inst.version_id = inst.version_id.trim().to_string();
    if inst.name.is_empty() {
        bail!("Please give the instance a name");
    }
    if inst.version_id.is_empty() {
        bail!("Please choose a Minecraft version");
    }
    inst.memory_mb = inst.memory_mb.filter(|m| *m > 0).map(|m| m.clamp(256, 65536));
    inst.width = inst.width.filter(|v| *v > 0);
    inst.height = inst.height.filter(|v| *v > 0);
    inst.java_path = inst.java_path.trim().to_string();
    inst.server = inst.server.trim().to_string();
    if inst.color.is_empty() {
        inst.color = "violet".into();
    }
    Ok(inst)
}

fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

#[tauri::command]
pub fn save_instance(state: State<'_, Shared>, instance: Instance) -> Result<Instance> {
    let mut inst = sanitize_instance(instance)?;
    if inst.id.is_empty() {
        inst.id = auth::new_id();
        inst.created_at = now_secs();
    } else if !valid_id(&inst.id) {
        bail!("Invalid instance id");
    }
    let saved = inst.clone();
    state.mutate(|s| {
        match s.instances.iter_mut().find(|i| i.id == inst.id) {
            Some(existing) => {
                // keep the statistics the frontend does not edit
                inst.created_at = existing.created_at;
                inst.last_played = existing.last_played;
                inst.play_time_secs = existing.play_time_secs;
                *existing = inst;
            }
            None => {
                s.instances.push(inst);
                s.selected_instance = Some(saved.id.clone());
            }
        }
    })?;
    // return what is actually stored
    let id = saved.id.clone();
    Ok(state
        .read(|s| s.instances.iter().find(|i| i.id == id).cloned())
        .unwrap_or(saved))
}

#[tauri::command]
pub async fn duplicate_instance(state: State<'_, Shared>, id: String) -> Result<Instance> {
    let st = state.inner().clone();
    if !valid_id(&id) {
        bail!("Invalid instance id");
    }
    let mut copy = st
        .read(|s| s.instances.iter().find(|i| i.id == id).cloned())
        .ok_or_else(|| Error::msg("Instance not found"))?;
    copy.id = auth::new_id();
    copy.name = format!("{} (copy)", copy.name);
    copy.created_at = now_secs();
    copy.last_played = None;
    copy.play_time_secs = 0;

    let src = st.dirs.instance(&id);
    let dst = st.dirs.instance(&copy.id);
    if src.is_dir() {
        tokio::task::spawn_blocking(move || copy_dir(&src, &dst))
            .await
            .map_err(|e| Error::msg(e.to_string()))??;
    }
    let c2 = copy.clone();
    st.mutate(|s| s.instances.push(c2))?;
    Ok(copy)
}

fn copy_dir(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir(&from, &to)?;
        } else if ty.is_file() {
            std::fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn delete_instance(state: State<'_, Shared>, id: String, delete_files: bool) -> Result<()> {
    let st = state.inner().clone();
    if !valid_id(&id) {
        bail!("Invalid instance id");
    }
    if st.running.lock().unwrap().contains_key(&id) {
        bail!("Stop the game before deleting this instance");
    }
    let id2 = id.clone();
    st.mutate(|s| {
        s.instances.retain(|i| i.id != id2);
        if s.selected_instance.as_deref() == Some(id2.as_str()) {
            s.selected_instance = s.instances.first().map(|i| i.id.clone());
        }
    })?;
    if delete_files {
        let dir = st.dirs.instance(&id);
        if dir.is_dir() {
            tokio::fs::remove_dir_all(dir).await?;
        }
    }
    Ok(())
}

#[tauri::command]
pub fn select_instance(state: State<'_, Shared>, id: Option<String>) -> Result<()> {
    state.mutate(|s| s.selected_instance = id)
}

#[tauri::command]
pub fn open_instance_folder(state: State<'_, Shared>, id: String) -> Result<()> {
    if !valid_id(&id) {
        bail!("Invalid instance id");
    }
    let dir = state.dirs.instance(&id);
    std::fs::create_dir_all(&dir)?;
    open_target(&dir.to_string_lossy())
}

#[tauri::command]
pub fn open_data_folder(state: State<'_, Shared>) -> Result<()> {
    std::fs::create_dir_all(&state.dirs.root)?;
    open_target(&state.dirs.root.to_string_lossy())
}

#[tauri::command]
pub fn open_url(url: String) -> Result<()> {
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        bail!("Only http(s) links can be opened");
    }
    open_target(&url)
}

fn open_target(target: &str) -> Result<()> {
    #[cfg(target_os = "windows")]
    let program = "explorer";
    #[cfg(target_os = "macos")]
    let program = "open";
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let program = "xdg-open";

    std::process::Command::new(program)
        .arg(target)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| Error::msg(format!("Could not open {target}: {e}")))?;
    Ok(())
}

// ----------------------------------------------------------------------------
// Accounts
// ----------------------------------------------------------------------------

#[tauri::command]
pub fn add_offline_account(state: State<'_, Shared>, username: String) -> Result<AccountView> {
    let username = username.trim().to_string();
    auth::validate_username(&username)?;
    let exists = state.read(|s| {
        s.accounts
            .iter()
            .any(|a| a.kind == "offline" && a.username.eq_ignore_ascii_case(&username))
    });
    if exists {
        bail!("An offline account named {username} already exists");
    }
    let acc = StoredAccount {
        id: auth::new_id(),
        kind: "offline".into(),
        uuid: auth::offline_uuid(&username),
        username,
        access_token: String::new(),
        client_token: String::new(),
    };
    let view = AccountView::from(&acc);
    let id = acc.id.clone();
    state.mutate(|s| {
        s.accounts.push(acc);
        s.selected_account = Some(id);
    })?;
    Ok(view)
}

#[tauri::command]
pub async fn login_elyby(
    state: State<'_, Shared>,
    login: String,
    password: String,
    totp: Option<String>,
) -> Result<AccountView> {
    let st = state.inner().clone();
    if login.trim().is_empty() || password.is_empty() {
        bail!("Enter your Ely.by username/e-mail and password");
    }
    let mut acc = auth::elyby_login(&st, login.trim(), &password, totp.as_deref()).await?;
    // Re-logging into the same account replaces the old entry.
    let existing = st.read(|s| {
        s.accounts
            .iter()
            .find(|a| a.kind == "elyby" && a.uuid == acc.uuid)
            .map(|a| a.id.clone())
    });
    if let Some(id) = existing {
        acc.id = id;
    }
    let view = AccountView::from(&acc);
    let a2 = acc.clone();
    st.mutate(|s| {
        match s.accounts.iter_mut().find(|a| a.id == a2.id) {
            Some(slot) => *slot = a2.clone(),
            None => s.accounts.push(a2.clone()),
        }
        s.selected_account = Some(a2.id.clone());
    })?;
    Ok(view)
}

#[tauri::command]
pub async fn remove_account(state: State<'_, Shared>, id: String) -> Result<()> {
    let st = state.inner().clone();
    let acc = st.read(|s| s.accounts.iter().find(|a| a.id == id).cloned());
    if let Some(acc) = &acc {
        if acc.kind == "elyby" {
            auth::elyby_sign_out(&st, acc).await;
        }
    }
    st.mutate(|s| {
        s.accounts.retain(|a| a.id != id);
        if s.selected_account.as_deref() == Some(id.as_str()) {
            s.selected_account = s.accounts.first().map(|a| a.id.clone());
        }
    })
}

#[tauri::command]
pub fn select_account(state: State<'_, Shared>, id: String) -> Result<()> {
    state.mutate(|s| {
        if s.accounts.iter().any(|a| a.id == id) {
            s.selected_account = Some(id);
        }
    })
}

// ----------------------------------------------------------------------------
// Launching
// ----------------------------------------------------------------------------

#[tauri::command]
pub fn launch_instance(app: AppHandle, state: State<'_, Shared>, id: String) -> Result<()> {
    launch::start(app, state.inner().clone(), id)
}

#[tauri::command]
pub fn kill_instance(state: State<'_, Shared>, id: String) -> bool {
    launch::kill(state.inner(), &id)
}

#[tauri::command]
pub fn get_logs(state: State<'_, Shared>, id: String) -> Vec<LogLine> {
    state
        .logs
        .lock()
        .unwrap()
        .get(&id)
        .map(|b| b.iter().cloned().collect())
        .unwrap_or_default()
}

#[tauri::command]
pub fn clear_logs(state: State<'_, Shared>, id: String) {
    state.logs.lock().unwrap().remove(&id);
}
