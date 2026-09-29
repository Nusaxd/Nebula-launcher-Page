use crate::error::Result;
use crate::models::{LogLine, Store};
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::oneshot;

pub const MAX_LOG_LINES: usize = 3000;

#[derive(Clone, Debug)]
pub struct Dirs {
    pub root: PathBuf,
}

impl Dirs {
    /// Shared game files (versions, libraries, assets).
    pub fn game(&self) -> PathBuf {
        self.root.join("game")
    }
    pub fn versions(&self) -> PathBuf {
        self.game().join("versions")
    }
    pub fn libraries(&self) -> PathBuf {
        self.game().join("libraries")
    }
    pub fn assets(&self) -> PathBuf {
        self.game().join("assets")
    }
    pub fn runtimes(&self) -> PathBuf {
        self.root.join("runtimes")
    }
    pub fn instances(&self) -> PathBuf {
        self.root.join("instances")
    }
    pub fn instance(&self, id: &str) -> PathBuf {
        self.instances().join(id)
    }
    pub fn natives(&self, id: &str) -> PathBuf {
        self.root.join("natives").join(id)
    }
    pub fn tools(&self) -> PathBuf {
        self.root.join("tools")
    }
    pub fn meta(&self) -> PathBuf {
        self.root.join("meta")
    }
    pub fn store_file(&self) -> PathBuf {
        self.root.join("launcher.json")
    }
}

pub struct AppState {
    pub dirs: Dirs,
    pub http: reqwest::Client,
    store: Mutex<Store>,
    /// instance id -> kill switch of the running game
    pub running: Mutex<HashMap<String, oneshot::Sender<()>>>,
    /// instances / versions currently being installed or launched
    pub busy: Mutex<HashSet<String>>,
    pub logs: Mutex<HashMap<String, VecDeque<LogLine>>>,
}

pub type Shared = Arc<AppState>;

impl AppState {
    pub fn new(root: PathBuf) -> Result<AppState> {
        std::fs::create_dir_all(&root)?;
        let dirs = Dirs { root };
        let store: Store = std::fs::read_to_string(dirs.store_file())
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        let http = reqwest::Client::builder()
            .user_agent(concat!("NebulaLauncher/", env!("CARGO_PKG_VERSION")))
            .connect_timeout(Duration::from_secs(15))
            .build()?;

        Ok(AppState {
            dirs,
            http,
            store: Mutex::new(store),
            running: Mutex::new(HashMap::new()),
            busy: Mutex::new(HashSet::new()),
            logs: Mutex::new(HashMap::new()),
        })
    }

    /// Read-only access to the store.
    pub fn read<R>(&self, f: impl FnOnce(&Store) -> R) -> R {
        let guard = self.store.lock().unwrap();
        f(&guard)
    }

    /// Mutate the store and persist it to disk.
    pub fn mutate<R>(&self, f: impl FnOnce(&mut Store) -> R) -> Result<R> {
        let mut guard = self.store.lock().unwrap();
        let out = f(&mut guard);
        let json = serde_json::to_string_pretty(&*guard)?;
        let path = self.dirs.store_file();
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, json)?;
        std::fs::rename(&tmp, &path)?;
        Ok(out)
    }

    pub fn push_log(&self, line: LogLine) {
        let mut logs = self.logs.lock().unwrap();
        let buf = logs.entry(line.instance_id.clone()).or_default();
        if buf.len() >= MAX_LOG_LINES {
            buf.pop_front();
        }
        buf.push_back(line);
    }
}

pub fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Marks an id as busy for as long as the guard lives.
pub struct BusyGuard {
    state: Shared,
    id: String,
}

impl BusyGuard {
    /// Returns `None` if the id is already busy.
    pub fn acquire(state: &Shared, id: &str) -> Option<BusyGuard> {
        let mut busy = state.busy.lock().unwrap();
        if !busy.insert(id.to_string()) {
            return None;
        }
        Some(BusyGuard {
            state: state.clone(),
            id: id.to_string(),
        })
    }
}

impl Drop for BusyGuard {
    fn drop(&mut self) {
        self.state.busy.lock().unwrap().remove(&self.id);
    }
}
