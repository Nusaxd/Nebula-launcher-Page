//! Offline accounts and Ely.by (Yggdrasil compatible) authentication.

use crate::bail;
use crate::download::{download_file, emit_stage};
use crate::error::{Error, Result};
use crate::models::StoredAccount;
use crate::state::Shared;
use md5::{Digest, Md5};
use serde::Deserialize;
use serde_json::json;
use std::path::PathBuf;
use std::sync::atomic::AtomicU64;
use tauri::AppHandle;

pub const ELY_AUTH_URL: &str = "https://authserver.ely.by/auth";
/// Value passed to authlib-injector so that the game talks to Ely.by.
pub const ELY_INJECTOR_TARGET: &str = "ely.by";

/// Vanilla compatible offline UUID: `UUID.nameUUIDFromBytes("OfflinePlayer:<name>")`.
pub fn offline_uuid(name: &str) -> String {
    let mut hasher = Md5::new();
    hasher.update(format!("OfflinePlayer:{name}").as_bytes());
    let digest = hasher.finalize();
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&digest);
    bytes[6] = (bytes[6] & 0x0f) | 0x30;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    hex::encode(bytes)
}

pub fn validate_username(name: &str) -> Result<()> {
    if name.len() < 3 || name.len() > 16 {
        bail!("Usernames must be between 3 and 16 characters long");
    }
    if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        bail!("Usernames may only contain letters, numbers and underscores");
    }
    Ok(())
}

pub fn new_id() -> String {
    uuid::Uuid::new_v4().simple().to_string()[..12].to_string()
}

#[derive(Deserialize, Debug)]
struct Profile {
    id: String,
    name: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct AuthResponse {
    access_token: String,
    #[serde(default)]
    client_token: String,
    selected_profile: Option<Profile>,
    #[serde(default)]
    available_profiles: Vec<Profile>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ErrorResponse {
    #[serde(default)]
    error: String,
    #[serde(default)]
    error_message: String,
}

async fn parse_error(resp: reqwest::Response) -> Error {
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    if let Ok(e) = serde_json::from_str::<ErrorResponse>(&text) {
        if !e.error_message.is_empty() {
            return Error::msg(e.error_message);
        }
        if !e.error.is_empty() {
            return Error::msg(e.error);
        }
    }
    Error::msg(format!("Ely.by returned HTTP {}", status.as_u16()))
}

/// Sign in with an Ely.by account. `totp` is the two factor code, if required.
pub async fn elyby_login(st: &Shared, login: &str, password: &str, totp: Option<&str>) -> Result<StoredAccount> {
    let client_token = uuid::Uuid::new_v4().simple().to_string();
    let password = match totp {
        Some(code) if !code.trim().is_empty() => format!("{}:{}", password, code.trim()),
        _ => password.to_string(),
    };
    let resp = st
        .http
        .post(format!("{ELY_AUTH_URL}/authenticate"))
        .json(&json!({
            "username": login,
            "password": password,
            "clientToken": client_token,
            "requestUser": true,
        }))
        .send()
        .await?;
    if !resp.status().is_success() {
        return Err(parse_error(resp).await);
    }
    let auth: AuthResponse = resp.json().await?;
    let profile = auth
        .selected_profile
        .or_else(|| auth.available_profiles.into_iter().next())
        .ok_or_else(|| Error::msg("This Ely.by account has no Minecraft profile"))?;
    Ok(StoredAccount {
        id: new_id(),
        kind: "elyby".into(),
        username: profile.name,
        uuid: profile.id.replace('-', ""),
        access_token: auth.access_token,
        client_token: if auth.client_token.is_empty() { client_token } else { auth.client_token },
    })
}

/// Makes sure the Ely.by token is usable, refreshing it if necessary.
/// Returns the (possibly updated) account.
pub async fn elyby_ensure_valid(st: &Shared, acc: &StoredAccount) -> Result<StoredAccount> {
    let valid = st
        .http
        .post(format!("{ELY_AUTH_URL}/validate"))
        .json(&json!({ "accessToken": acc.access_token }))
        .send()
        .await;
    match valid {
        Ok(r) if r.status().is_success() => return Ok(acc.clone()),
        // Network problems: try to play with the token we have.
        Err(_) => return Ok(acc.clone()),
        _ => {}
    }

    let resp = st
        .http
        .post(format!("{ELY_AUTH_URL}/refresh"))
        .json(&json!({
            "accessToken": acc.access_token,
            "clientToken": acc.client_token,
            "requestUser": false,
        }))
        .send()
        .await?;
    if !resp.status().is_success() {
        bail!("Your Ely.by session has expired. Please remove the account and sign in again.");
    }
    let auth: AuthResponse = resp.json().await?;
    let mut updated = acc.clone();
    updated.access_token = auth.access_token;
    if !auth.client_token.is_empty() {
        updated.client_token = auth.client_token;
    }
    if let Some(p) = auth.selected_profile {
        updated.username = p.name;
        updated.uuid = p.id.replace('-', "");
    }
    Ok(updated)
}

pub async fn elyby_sign_out(st: &Shared, acc: &StoredAccount) {
    let _ = st
        .http
        .post(format!("{ELY_AUTH_URL}/invalidate"))
        .json(&json!({ "accessToken": acc.access_token, "clientToken": acc.client_token }))
        .send()
        .await;
}

// ----------------------------------------------------------------------------
// authlib-injector
// ----------------------------------------------------------------------------

#[derive(Deserialize)]
struct GhAsset {
    name: String,
    browser_download_url: String,
}

#[derive(Deserialize)]
struct GhRelease {
    #[serde(default)]
    assets: Vec<GhAsset>,
}

pub fn injector_path(st: &Shared) -> PathBuf {
    st.dirs.tools().join("authlib-injector.jar")
}

/// Downloads authlib-injector (needed to use Ely.by skins / servers in-game).
pub async fn ensure_authlib_injector(app: &AppHandle, st: &Shared, id: &str) -> Result<PathBuf> {
    let path = injector_path(st);
    if path.is_file() {
        return Ok(path);
    }
    emit_stage(app, id, "Downloading authlib-injector");

    let mut url: Option<String> = None;
    if let Ok(resp) = st
        .http
        .get("https://api.github.com/repos/yushijinhun/authlib-injector/releases/latest")
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
    {
        if resp.status().is_success() {
            if let Ok(rel) = resp.json::<GhRelease>().await {
                url = rel
                    .assets
                    .into_iter()
                    .find(|a| a.name.starts_with("authlib-injector") && a.name.ends_with(".jar"))
                    .map(|a| a.browser_download_url);
            }
        }
    }
    let url = url.unwrap_or_else(|| {
        "https://github.com/yushijinhun/authlib-injector/releases/download/v1.2.8/authlib-injector-1.2.8.jar".to_string()
    });

    let job = crate::download::Job {
        url,
        path: path.clone(),
        sha1: None,
        size: 0,
    };
    download_file(&st.http, &job, &AtomicU64::new(0)).await?;
    Ok(path)
}
