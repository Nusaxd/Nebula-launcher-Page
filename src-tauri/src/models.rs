//! Data structures persisted on disk and exchanged with the frontend.

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    /// Default max memory for instances that do not override it (MB).
    pub default_memory_mb: u32,
    /// Custom Java executable (empty = auto).
    pub java_path: String,
    /// Extra JVM arguments applied to every instance.
    pub jvm_args: String,
    /// Download and use Mojang's bundled Java runtimes when needed.
    pub managed_java: bool,
    /// Hide the launcher window while the game is running.
    pub hide_on_launch: bool,
    /// Parallel downloads.
    pub concurrency: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            default_memory_mb: 2048,
            java_path: String::new(),
            jvm_args: String::new(),
            managed_java: true,
            hide_on_launch: false,
            concurrency: 12,
        }
    }
}

/// Account as stored on disk (contains secrets, never sent to the frontend).
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct StoredAccount {
    pub id: String,
    /// "offline" or "elyby"
    pub kind: String,
    pub username: String,
    /// UUID without dashes
    pub uuid: String,
    pub access_token: String,
    pub client_token: String,
}

/// Account as shown in the UI.
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AccountView {
    pub id: String,
    pub kind: String,
    pub username: String,
    pub uuid: String,
}

impl From<&StoredAccount> for AccountView {
    fn from(a: &StoredAccount) -> Self {
        AccountView {
            id: a.id.clone(),
            kind: a.kind.clone(),
            username: a.username.clone(),
            uuid: a.uuid.clone(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Instance {
    pub id: String,
    pub name: String,
    pub version_id: String,
    pub memory_mb: Option<u32>,
    pub java_path: String,
    pub jvm_args: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub fullscreen: bool,
    /// Optional server to join automatically (host or host:port).
    pub server: String,
    pub color: String,
    pub created_at: u64,
    pub last_played: Option<u64>,
    pub play_time_secs: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Store {
    pub settings: Settings,
    pub accounts: Vec<StoredAccount>,
    pub instances: Vec<Instance>,
    pub selected_account: Option<String>,
    pub selected_instance: Option<String>,
}

/// Everything the frontend needs on start-up.
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Bootstrap {
    pub settings: Settings,
    pub accounts: Vec<AccountView>,
    pub instances: Vec<Instance>,
    pub selected_account: Option<String>,
    pub selected_instance: Option<String>,
    pub data_dir: String,
    pub os: String,
    pub app_version: String,
}

// ----------------------------------------------------------------------------
// Events
// ----------------------------------------------------------------------------

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub id: String,
    pub stage: String,
    pub done_bytes: u64,
    pub total_bytes: u64,
    pub done_files: u64,
    pub total_files: u64,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GameState {
    pub instance_id: String,
    /// preparing | running | stopped | crashed | error
    pub state: String,
    pub message: Option<String>,
    pub exit_code: Option<i32>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LogLine {
    pub instance_id: String,
    /// stdout | stderr | launcher
    pub stream: String,
    pub line: String,
}
